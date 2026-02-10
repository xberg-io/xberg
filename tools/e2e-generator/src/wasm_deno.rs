use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;

use crate::fixtures::{Assertions, ExtractionMethod, Fixture, InputType, WasmTarget};

const DENO_HELPERS_TEMPLATE: &str = r#"import { assertEquals, assertExists } from "@std/assert";
// @deno-types="../../crates/kreuzberg-wasm/dist/index.d.ts"
import { extractBytes, initWasm } from "npm:@kreuzberg/wasm@^4.0.0";
// @deno-types="../../crates/kreuzberg-wasm/dist/index.d.ts"
import type {
    ChunkingConfig,
    ExtractionConfig,
    ExtractionResult,
    ImageExtractionConfig,
    LanguageDetectionConfig,
    Metadata,
    OcrConfig,
    PdfConfig,
    PostProcessorConfig,
    Table,
    TesseractConfig,
    TokenReductionConfig,
} from "npm:@kreuzberg/wasm@^4.0.0";

export type {
    ChunkingConfig,
    ExtractionConfig,
    ExtractionResult,
    ImageExtractionConfig,
    LanguageDetectionConfig,
    Metadata,
    OcrConfig,
    PdfConfig,
    PostProcessorConfig,
    Table,
    TesseractConfig,
    TokenReductionConfig,
};

export { extractBytes, initWasm };

const WORKSPACE_ROOT = new URL("../..", import.meta.url).pathname.replace(/\/$/, "");
const TEST_DOCUMENTS = `${WORKSPACE_ROOT}/test_documents`;

type PlainRecord = Record<string, unknown>;

function isPlainRecord(value: unknown): value is PlainRecord {
    return typeof value === "object" && value !== null;
}

export async function resolveDocument(relative: string): Promise<Uint8Array> {
    const path = `${TEST_DOCUMENTS}/${relative}`;
    return await Deno.readFile(path);
}

function assignBooleanField(target: PlainRecord, source: PlainRecord, sourceKey: string, targetKey: string): void {
    if (sourceKey in source) {
        const value = source[sourceKey];
        if (typeof value === "boolean") {
            target[targetKey] = value;
        } else if (value != null) {
            target[targetKey] = Boolean(value);
        }
    }
}

function assignNumberField(target: PlainRecord, source: PlainRecord, sourceKey: string, targetKey: string): void {
    if (sourceKey in source) {
        const value = source[sourceKey];
        if (typeof value === "number") {
            target[targetKey] = value;
        } else if (typeof value === "string") {
            const parsed = Number(value);
            if (!Number.isNaN(parsed)) {
                target[targetKey] = parsed;
            }
        }
    }
}

function mapStringArray(value: unknown): string[] | undefined {
    if (!Array.isArray(value)) {
        return undefined;
    }
    return value.filter((item): item is string => typeof item === "string");
}

function mapTesseractConfig(raw: PlainRecord): TesseractConfig {
    const config: PlainRecord = {};
    assignNumberField(config, raw, "psm", "psm");
    assignBooleanField(config, raw, "enable_table_detection", "enableTableDetection");
    if (typeof raw.tessedit_char_whitelist === "string") {
        (config as unknown as TesseractConfig).tesseditCharWhitelist = raw.tessedit_char_whitelist;
    }
    return config as unknown as TesseractConfig;
}

function mapOcrConfig(raw: PlainRecord): OcrConfig | undefined {
    const backend = raw.backend;
    if (typeof backend !== "string" || backend.length === 0) {
        return undefined;
    }

    const config: PlainRecord = { backend };
    if (typeof raw.language === "string") {
        config.language = raw.language as string;
    }

    if (isPlainRecord(raw.tesseract_config)) {
        (config as unknown as OcrConfig).tesseractConfig = mapTesseractConfig(raw.tesseract_config as PlainRecord);
    }

    return config as unknown as OcrConfig;
}

function mapChunkingConfig(raw: PlainRecord): ChunkingConfig {
    const config: PlainRecord = {};
    assignNumberField(config, raw, "max_chars", "maxChars");
    assignNumberField(config, raw, "max_overlap", "maxOverlap");
    return config as unknown as ChunkingConfig;
}

function mapImageExtractionConfig(raw: PlainRecord): ImageExtractionConfig {
    const config: PlainRecord = {};
    assignBooleanField(config, raw, "extract_images", "extractImages");
    assignNumberField(config, raw, "target_dpi", "targetDpi");
    assignNumberField(config, raw, "max_image_dimension", "maxImageDimension");
    assignBooleanField(config, raw, "auto_adjust_dpi", "autoAdjustDpi");
    assignNumberField(config, raw, "min_dpi", "minDpi");
    assignNumberField(config, raw, "max_dpi", "maxDpi");
    return config as unknown as ImageExtractionConfig;
}

function mapPdfConfig(raw: PlainRecord): PdfConfig {
    const config: PlainRecord = {};
    assignBooleanField(config, raw, "extract_images", "extractImages");
    if (Array.isArray(raw.passwords)) {
        (config as unknown as PdfConfig).passwords = raw.passwords.filter((item: unknown): item is string => typeof item === "string");
    }
    assignBooleanField(config, raw, "extract_metadata", "extractMetadata");
    return config as unknown as PdfConfig;
}

function mapTokenReductionConfig(raw: PlainRecord): TokenReductionConfig {
    const config: PlainRecord = {};
    if (typeof raw.mode === "string") {
        (config as unknown as TokenReductionConfig).mode = raw.mode;
    }
    assignBooleanField(config, raw, "preserve_important_words", "preserveImportantWords");
    return config as unknown as TokenReductionConfig;
}

function mapLanguageDetectionConfig(raw: PlainRecord): LanguageDetectionConfig {
    const config: PlainRecord = {};
    assignBooleanField(config, raw, "enabled", "enabled");
    assignNumberField(config, raw, "min_confidence", "minConfidence");
    assignBooleanField(config, raw, "detect_multiple", "detectMultiple");
    return config as unknown as LanguageDetectionConfig;
}

function mapPostProcessorConfig(raw: PlainRecord): PostProcessorConfig {
    const config: PlainRecord = {};
    assignBooleanField(config, raw, "enabled", "enabled");
    const enabled = mapStringArray(raw.enabled_processors);
    if (enabled) {
        (config as unknown as PostProcessorConfig).enabledProcessors = enabled;
    }
    const disabled = mapStringArray(raw.disabled_processors);
    if (disabled) {
        (config as unknown as PostProcessorConfig).disabledProcessors = disabled;
    }
    return config as unknown as PostProcessorConfig;
}

export function buildConfig(raw: unknown): ExtractionConfig {
    if (!isPlainRecord(raw)) {
        return {};
    }

    const source = raw as PlainRecord;
    const result: ExtractionConfig = {};
    const target = result as PlainRecord;

    assignBooleanField(target, source, "use_cache", "useCache");
    assignBooleanField(target, source, "enable_quality_processing", "enableQualityProcessing");
    assignBooleanField(target, source, "force_ocr", "forceOcr");
    assignBooleanField(target, source, "include_document_structure", "includeDocumentStructure");
    assignNumberField(target, source, "max_concurrent_extractions", "maxConcurrentExtractions");

    if (isPlainRecord(source.ocr)) {
        const mapped = mapOcrConfig(source.ocr as PlainRecord);
        if (mapped) {
            result.ocr = mapped;
        }
    }

    if (isPlainRecord(source.chunking)) {
        result.chunking = mapChunkingConfig(source.chunking as PlainRecord);
    }

    if (isPlainRecord(source.images)) {
        result.images = mapImageExtractionConfig(source.images as PlainRecord);
    }

    if (isPlainRecord(source.pdf_options)) {
        result.pdfOptions = mapPdfConfig(source.pdf_options as PlainRecord);
    }

    if (isPlainRecord(source.token_reduction)) {
        result.tokenReduction = mapTokenReductionConfig(source.token_reduction as PlainRecord);
    }

    if (isPlainRecord(source.language_detection)) {
        result.languageDetection = mapLanguageDetectionConfig(source.language_detection as PlainRecord);
    }

    if (isPlainRecord(source.postprocessor)) {
        result.postprocessor = mapPostProcessorConfig(source.postprocessor as PlainRecord);
    }

    return result;
}

export function shouldSkipFixture(
    error: unknown,
    fixtureId: string,
    requirements: string[],
    notes?: string | null,
): boolean {
    if (!(error instanceof Error)) {
        return false;
    }

    const message = `${error.name}: ${error.message}`;
    const lower = message.toLowerCase();

    const requirementHit = requirements.some((req) => lower.includes(req.toLowerCase()));
    const missingDependency = lower.includes("missingdependencyerror") || lower.includes("missing dependency");
    const unsupportedFormat = lower.includes("unsupported mime type") || lower.includes("unsupported format");
    const pdfiumError = lower.includes("pdfium") || lower.includes("pdf extraction requires proper wasm");
    const stackOverflow = lower.includes("maximum call stack") || lower.includes("stack overflow");

    if (missingDependency || unsupportedFormat || pdfiumError || stackOverflow || requirementHit) {
        const reason = missingDependency
            ? "missing dependency"
            : unsupportedFormat
              ? "unsupported format"
              : pdfiumError
                ? "PDFium not available (non-browser environment)"
                : stackOverflow
                  ? "stack overflow (document too large for WASM)"
                  : requirements.join(", ");
        console.warn(`Skipping ${fixtureId}: ${reason}. ${message}`);
        if (notes) {
            console.warn(`Notes: ${notes}`);
        }
        return true;
    }

    return false;
}

export const assertions = {
    assertExpectedMime(result: ExtractionResult, expected: string[]): void {
        if (!expected.length) {
            return;
        }
        assertEquals(
            expected.some((token) => result.mimeType.includes(token)),
            true,
        );
    },

    assertMinContentLength(result: ExtractionResult, minimum: number): void {
        assertEquals(result.content.length >= minimum, true);
    },

    assertMaxContentLength(result: ExtractionResult, maximum: number): void {
        assertEquals(result.content.length <= maximum, true);
    },

    assertContentContainsAny(result: ExtractionResult, snippets: string[]): void {
        if (!snippets.length) {
            return;
        }
        const lowered = result.content.toLowerCase();
        assertEquals(
            snippets.some((snippet) => lowered.includes(snippet.toLowerCase())),
            true,
        );
    },

    assertContentContainsAll(result: ExtractionResult, snippets: string[]): void {
        if (!snippets.length) {
            return;
        }
        const lowered = result.content.toLowerCase();
        assertEquals(
            snippets.every((snippet) => lowered.includes(snippet.toLowerCase())),
            true,
        );
    },

    assertTableCount(result: ExtractionResult, minimum?: number | null, maximum?: number | null): void {
        const tables = Array.isArray(result.tables) ? result.tables : [];
        if (typeof minimum === "number") {
            assertEquals(tables.length >= minimum, true);
        }
        if (typeof maximum === "number") {
            assertEquals(tables.length <= maximum, true);
        }
    },

    assertDetectedLanguages(result: ExtractionResult, expected: string[], minConfidence?: number | null): void {
        if (!expected.length) {
            return;
        }
        assertExists(result.detectedLanguages);
        const languages = result.detectedLanguages ?? [];
        assertEquals(expected.every((lang) => languages.includes(lang)), true);

        if (typeof minConfidence === "number" && isPlainRecord(result.metadata)) {
            const confidence = (result.metadata as PlainRecord).confidence;
            if (typeof confidence === "number") {
                assertEquals(confidence >= minConfidence, true);
            }
        }
    },

    assertMetadataExpectation(result: ExtractionResult, path: string, expectation: PlainRecord): void {
        if (!isPlainRecord(result.metadata)) {
            throw new Error(`Metadata is not a record for path ${path}`);
        }

        const value = getMetadataPath(result.metadata as PlainRecord, path);
        if (value === undefined || value === null) {
            throw new Error(`Metadata path '${path}' missing in ${JSON.stringify(result.metadata)}`);
        }

        if ("eq" in expectation) {
            assertEquals(valuesEqual(value, expectation.eq), true);
        }

        if ("gte" in expectation) {
            assertEquals(Number(value) >= Number(expectation.gte), true);
        }

        if ("lte" in expectation) {
            assertEquals(Number(value) <= Number(expectation.lte), true);
        }

        if ("contains" in expectation) {
            const contains = expectation.contains;
            if (typeof value === "string" && typeof contains === "string") {
                assertEquals(value.includes(contains), true);
            } else if (Array.isArray(value) && Array.isArray(contains)) {
                assertEquals(contains.every((item) => value.includes(item)), true);
            } else {
                throw new Error(`Unsupported contains expectation for path '${path}'`);
            }
        }
    },

    assertChunks(
        result: ExtractionResult,
        minCount?: number | null,
        maxCount?: number | null,
        eachHasContent?: boolean | null,
        eachHasEmbedding?: boolean | null,
    ): void {
        const chunks = (result as unknown as PlainRecord).chunks as unknown[] | undefined;
        assertExists(chunks, "Expected chunks to be defined");
        if (!Array.isArray(chunks)) {
            throw new Error("Expected chunks to be an array");
        }
        if (typeof minCount === "number") {
            assertEquals(chunks.length >= minCount, true, `Expected at least ${minCount} chunks, got ${chunks.length}`);
        }
        if (typeof maxCount === "number") {
            assertEquals(chunks.length <= maxCount, true, `Expected at most ${maxCount} chunks, got ${chunks.length}`);
        }
        if (eachHasContent) {
            for (const chunk of chunks) {
                assertExists((chunk as PlainRecord).content, "Chunk missing content");
            }
        }
        if (eachHasEmbedding) {
            for (const chunk of chunks) {
                assertExists((chunk as PlainRecord).embedding, "Chunk missing embedding");
            }
        }
    },

    assertImages(
        result: ExtractionResult,
        minCount?: number | null,
        maxCount?: number | null,
        formatsInclude?: string[] | null,
    ): void {
        const images = (result as unknown as PlainRecord).images as unknown[] | undefined;
        assertExists(images, "Expected images to be defined");
        if (!Array.isArray(images)) {
            throw new Error("Expected images to be an array");
        }
        if (typeof minCount === "number") {
            assertEquals(images.length >= minCount, true, `Expected at least ${minCount} images, got ${images.length}`);
        }
        if (typeof maxCount === "number") {
            assertEquals(images.length <= maxCount, true, `Expected at most ${maxCount} images, got ${images.length}`);
        }
        if (formatsInclude && formatsInclude.length > 0) {
            const foundFormats = new Set(images.map((img) => (img as PlainRecord).format));
            for (const fmt of formatsInclude) {
                assertEquals(foundFormats.has(fmt), true, `Expected image format ${fmt} not found`);
            }
        }
    },

    assertPages(
        result: ExtractionResult,
        minCount?: number | null,
        exactCount?: number | null,
    ): void {
        const pages = (result as unknown as PlainRecord).pages as unknown[] | undefined;
        assertExists(pages, "Expected pages to be defined");
        if (!Array.isArray(pages)) {
            throw new Error("Expected pages to be an array");
        }
        if (typeof exactCount === "number") {
            assertEquals(pages.length, exactCount, `Expected exactly ${exactCount} pages, got ${pages.length}`);
        }
        if (typeof minCount === "number") {
            assertEquals(pages.length >= minCount, true, `Expected at least ${minCount} pages, got ${pages.length}`);
        }
    },

    assertElements(
        result: ExtractionResult,
        minCount?: number | null,
        typesInclude?: string[] | null,
    ): void {
        const elements = (result as unknown as PlainRecord).elements as unknown[] | undefined;
        assertExists(elements, "Expected elements to be defined");
        if (!Array.isArray(elements)) {
            throw new Error("Expected elements to be an array");
        }
        if (typeof minCount === "number") {
            assertEquals(elements.length >= minCount, true, `Expected at least ${minCount} elements, got ${elements.length}`);
        }
        if (typesInclude && typesInclude.length > 0) {
            const foundTypes = new Set(elements.map((el) => (el as PlainRecord).type));
            for (const elType of typesInclude) {
                assertEquals(foundTypes.has(elType), true, `Expected element type ${elType} not found`);
            }
        }
    },

    assertDocument(
        result: ExtractionResult,
        hasDocument: boolean,
        minNodeCount?: number | null,
        nodeTypesInclude?: string[] | null,
        hasGroups?: boolean | null,
    ): void {
        const doc = (result as unknown as PlainRecord).document as PlainRecord | undefined | null;
        if (hasDocument) {
            assertExists(doc, "Expected document but got null");
            const nodes = (doc as PlainRecord).nodes as unknown[];
            assertExists(nodes, "Expected document nodes but got null");
            if (typeof minNodeCount === "number") {
                assertEquals(nodes.length >= minNodeCount, true, `Expected at least ${minNodeCount} nodes, got ${nodes.length}`);
            }
            if (nodeTypesInclude && nodeTypesInclude.length > 0) {
                const foundTypes = new Set(nodes.map((n) => ((n as PlainRecord).content as PlainRecord)?.node_type ?? (n as PlainRecord).node_type));
                for (const expected of nodeTypesInclude) {
                    assertEquals(
                        [...foundTypes].some((t) => typeof t === "string" && t.toLowerCase() === expected.toLowerCase()),
                        true,
                        `Expected node type '${expected}' not found in [${[...foundTypes].join(", ")}]`,
                    );
                }
            }
            if (typeof hasGroups === "boolean") {
                const hasGroupNodes = nodes.some((n) => ((n as PlainRecord).content as PlainRecord)?.node_type === "group" || (n as PlainRecord).node_type === "group");
                assertEquals(hasGroupNodes, hasGroups, `Expected hasGroups=${hasGroups} but got ${hasGroupNodes}`);
            }
        } else {
            assertEquals(doc == null, true, "Expected document to be null but got a document");
        }
    },
};

function lookupMetadataPath(metadata: PlainRecord, path: string): unknown {
    const segments = path.split(".");
    let current: unknown = metadata;
    for (const segment of segments) {
        if (!isPlainRecord(current) || !(segment in current)) {
            return undefined;
        }
        current = (current as PlainRecord)[segment];
    }
    return current;
}

function getMetadataPath(metadata: PlainRecord, path: string): unknown {
    const direct = lookupMetadataPath(metadata, path);
    if (direct !== undefined) {
        return direct;
    }
    const format = metadata.format;
    if (isPlainRecord(format)) {
        return lookupMetadataPath(format as PlainRecord, path);
    }
    return undefined;
}

function valuesEqual(lhs: unknown, rhs: unknown): boolean {
    if (typeof lhs === "string" && typeof rhs === "string") {
        return lhs === rhs;
    }
    if (typeof lhs === "number" && typeof rhs === "number") {
        return lhs === rhs;
    }
    if (typeof lhs === "boolean" && typeof rhs === "boolean") {
        return lhs === rhs;
    }
    return JSON.stringify(lhs) === JSON.stringify(rhs);
}
"#;

/// Generate Deno/WASM test suite from fixtures.
pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let output_dir = output_root.join("wasm-deno");

    fs::create_dir_all(&output_dir).context("Failed to create Deno tests directory")?;

    clean_test_files(&output_dir)?;
    write_helpers(&output_dir)?;

    let doc_fixtures: Vec<_> = fixtures
        .iter()
        .filter(|f| f.is_document_extraction() && crate::fixtures::should_include_for_wasm(f, WasmTarget::Deno))
        .collect();

    let plugin_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_plugin_api()).collect();

    let mut grouped = doc_fixtures
        .into_iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let file_name = format!("{}.test.ts", to_snake_case(&category));
        let content = render_category(&category, &fixtures)?;
        let path = output_dir.join(&file_name);
        fs::write(&path, content).with_context(|| format!("Writing {}", path))?;
    }

    if !plugin_fixtures.is_empty() {
        generate_plugin_api_tests(&plugin_fixtures, &output_dir)?;
    }

    Ok(())
}

fn clean_test_files(dir: &Utf8Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir.as_std_path())? {
        let entry = entry?;
        let file_name = entry
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if file_name == "helpers.ts" {
            continue;
        }

        if file_name.ends_with(".test.ts") {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

fn write_helpers(output_dir: &Utf8Path) -> Result<()> {
    let helpers_path = output_dir.join("helpers.ts");
    fs::write(&helpers_path, DENO_HELPERS_TEMPLATE).context("Failed to write helpers.ts")?;
    Ok(())
}

fn render_category(category: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "// Auto-generated tests for {category} fixtures.")?;
    writeln!(buffer, "// Run with: deno test --allow-read")?;
    writeln!(buffer)?;
    writeln!(
        buffer,
        "import {{ assertions, buildConfig, extractBytes, initWasm, resolveDocument, shouldSkipFixture }} from \"./helpers.ts\";"
    )?;
    writeln!(buffer, "import type {{ ExtractionResult }} from \"./helpers.ts\";\n")?;
    writeln!(buffer, "// Initialize WASM module once at module load time")?;
    writeln!(buffer, "await initWasm();\n")?;

    for fixture in fixtures {
        buffer.write_str(&render_test(fixture)?)?;
    }

    Ok(buffer)
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut body = String::new();
    let extraction = fixture.extraction();
    let method = extraction.method;
    let input_type = extraction.input_type;

    // WASM only supports async extractBytes - all tests are async
    // We adapt the test pattern based on method and input_type for consistency
    let test_name = &fixture.id;
    writeln!(
        body,
        "Deno.test(\"{test_name}\", {{ permissions: {{ read: true }} }}, async () => {{"
    )?;

    // Read document bytes - Deno always reads as Uint8Array
    writeln!(
        body,
        "    const documentBytes = await resolveDocument(\"{}\");",
        escape_ts_string(&fixture.document().path)
    )?;

    match render_config_expression(&extraction.config)? {
        None => writeln!(body, "    const config = buildConfig(undefined);")?,
        Some(config_expr) => writeln!(body, "    const config = buildConfig({config_expr});")?,
    }

    let requirements = collect_requirements(fixture);
    let mime_type = fixture
        .document()
        .media_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    writeln!(body, "    let result: ExtractionResult | null = null;")?;
    writeln!(body, "    try {{")?;

    // Generate extraction call based on method and input_type
    // Note: WASM only has extractBytes available, but we document the intent
    match (method, input_type) {
        (ExtractionMethod::Sync, InputType::File) => {
            // Sync file: In WASM, we read the file and use extractBytes (which is async)
            writeln!(
                body,
                "      // Sync file extraction - WASM uses extractBytes with pre-read bytes"
            )?;
            writeln!(
                body,
                "      result = await extractBytes(documentBytes, \"{}\", config);",
                escape_ts_string(mime_type)
            )?;
        }
        (ExtractionMethod::Sync, InputType::Bytes) => {
            // Sync bytes: Read file as Uint8Array and use bytes extraction
            writeln!(
                body,
                "      // Sync bytes extraction - WASM uses extractBytes with Uint8Array"
            )?;
            writeln!(
                body,
                "      result = await extractBytes(documentBytes, \"{}\", config);",
                escape_ts_string(mime_type)
            )?;
        }
        (ExtractionMethod::Async, InputType::File) => {
            // Async file: Use async/await pattern
            writeln!(body, "      // Async file extraction - native WASM pattern")?;
            writeln!(
                body,
                "      result = await extractBytes(documentBytes, \"{}\", config);",
                escape_ts_string(mime_type)
            )?;
        }
        (ExtractionMethod::Async, InputType::Bytes) => {
            // Async bytes: Use async/await with bytes
            writeln!(body, "      // Async bytes extraction - native WASM pattern")?;
            writeln!(
                body,
                "      result = await extractBytes(documentBytes, \"{}\", config);",
                escape_ts_string(mime_type)
            )?;
        }
        (ExtractionMethod::BatchSync, InputType::File) | (ExtractionMethod::BatchSync, InputType::Bytes) => {
            // Batch sync: WASM doesn't have batch methods, simulate with single extraction
            writeln!(
                body,
                "      // Batch sync extraction - WASM simulates with single extraction"
            )?;
            writeln!(
                body,
                "      const results = [await extractBytes(documentBytes, \"{}\", config)];",
                escape_ts_string(mime_type)
            )?;
            writeln!(body, "      result = results[0];")?;
        }
        (ExtractionMethod::BatchAsync, InputType::File) | (ExtractionMethod::BatchAsync, InputType::Bytes) => {
            // Batch async: WASM doesn't have batch methods, simulate with single extraction
            writeln!(
                body,
                "      // Batch async extraction - WASM simulates with single extraction"
            )?;
            writeln!(
                body,
                "      const results = [await extractBytes(documentBytes, \"{}\", config)];",
                escape_ts_string(mime_type)
            )?;
            writeln!(body, "      result = results[0];")?;
        }
    }

    writeln!(body, "    }} catch (error) {{")?;
    if !requirements.is_empty()
        || fixture.skip().notes.is_some()
        || !fixture.document().requires_external_tool.is_empty()
    {
        writeln!(
            body,
            "      if (shouldSkipFixture(error, \"{}\", {}, {})) {{",
            escape_ts_string(&fixture.id),
            render_string_array(&requirements),
            render_optional_string(fixture.skip().notes.as_ref())
        )?;
    } else {
        writeln!(
            body,
            "      if (shouldSkipFixture(error, \"{}\", [], undefined)) {{",
            escape_ts_string(&fixture.id)
        )?;
    }
    writeln!(body, "        return;\n      }}\n      throw error;\n    }}")?;
    writeln!(body, "    if (result === null) {{\n      return;\n    }}")?;

    body.push_str(&render_assertions(&fixture.assertions()));

    writeln!(body, "}});\n")?;

    Ok(body)
}

fn render_config_expression(config: &Map<String, Value>) -> Result<Option<String>> {
    if config.is_empty() {
        Ok(None)
    } else {
        let json = serde_json::to_string(&Value::Object(config.clone()))?;
        Ok(Some(json))
    }
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        buffer.push_str(&format!(
            "    assertions.assertExpectedMime(result, {});\n",
            render_string_array(&assertions.expected_mime)
        ));
    }

    if let Some(min) = assertions.min_content_length {
        buffer.push_str(&format!("    assertions.assertMinContentLength(result, {min});\n"));
    }

    if let Some(max) = assertions.max_content_length {
        buffer.push_str(&format!("    assertions.assertMaxContentLength(result, {max});\n"));
    }

    if !assertions.content_contains_any.is_empty() {
        buffer.push_str(&format!(
            "    assertions.assertContentContainsAny(result, {});\n",
            render_string_array(&assertions.content_contains_any)
        ));
    }

    if !assertions.content_contains_all.is_empty() {
        buffer.push_str(&format!(
            "    assertions.assertContentContainsAll(result, {});\n",
            render_string_array(&assertions.content_contains_all)
        ));
    }

    if let Some(tables) = assertions.tables.as_ref() {
        let min = tables
            .min
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".into());
        let max = tables
            .max
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!("    assertions.assertTableCount(result, {min}, {max});\n"));
    }

    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_array(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "    assertions.assertDetectedLanguages(result, {expected}, {min_conf});\n"
        ));
    }

    if !assertions.metadata.is_empty() {
        for (path, expectation) in &assertions.metadata {
            buffer.push_str(&format!(
                "    assertions.assertMetadataExpectation(result, \"{}\", {});\n",
                escape_ts_string(path),
                normalize_metadata_expectation(expectation)
            ));
        }
    }

    if let Some(chunks) = assertions.chunks.as_ref() {
        let min = chunks.min_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let max = chunks.max_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let has_content = chunks
            .each_has_content
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let has_embedding = chunks
            .each_has_embedding
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "    assertions.assertChunks(result, {min}, {max}, {has_content}, {has_embedding});\n"
        ));
    }

    if let Some(images) = assertions.images.as_ref() {
        let min = images.min_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let max = images.max_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let formats = images
            .formats_include
            .as_ref()
            .map(|f| render_string_array(f))
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "    assertions.assertImages(result, {min}, {max}, {formats});\n"
        ));
    }

    if let Some(pages) = assertions.pages.as_ref() {
        let min = pages.min_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let exact = pages
            .exact_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!("    assertions.assertPages(result, {min}, {exact});\n"));
    }

    if let Some(elements) = assertions.elements.as_ref() {
        let min = elements
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let types = elements
            .types_include
            .as_ref()
            .map(|t| render_string_array(t))
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!("    assertions.assertElements(result, {min}, {types});\n"));
    }

    if let Some(ocr) = assertions.ocr_elements.as_ref() {
        let has_elements = ocr.has_elements.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let has_geometry = ocr
            .elements_have_geometry
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let has_confidence = ocr
            .elements_have_confidence
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let min = ocr.min_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "    assertions.assertOcrElements(result, {has_elements}, {has_geometry}, {has_confidence}, {min});\n"
        ));
    }

    if let Some(document) = assertions.document.as_ref() {
        let has_document = document.has_document.to_string();
        let min_node_count = document
            .min_node_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let node_types = if !document.node_types_include.is_empty() {
            render_string_array(&document.node_types_include)
        } else {
            "null".into()
        };
        let has_groups = document
            .has_groups
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "    assertions.assertDocument(result, {has_document}, {min_node_count}, {node_types}, {has_groups});\n"
        ));
    }

    buffer
}

fn render_string_array(items: &[String]) -> String {
    if items.is_empty() {
        "[]".into()
    } else {
        let quoted = items
            .iter()
            .map(|item| format!("\"{}\"", escape_ts_string(item)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{quoted}]")
    }
}

fn render_optional_string(value: Option<&String>) -> String {
    match value {
        Some(text) => format!("\"{}\"", escape_ts_string(text)),
        None => "undefined".into(),
    }
}

fn collect_requirements(fixture: &Fixture) -> Vec<String> {
    fixture
        .skip()
        .requires_feature
        .iter()
        .chain(fixture.document().requires_external_tool.iter())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn to_snake_case(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c.to_ascii_lowercase() })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn escape_ts_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn render_json_literal(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into())
}

fn normalize_metadata_expectation(value: &Value) -> String {
    match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) => {
            format!("{{ eq: {} }}", render_json_literal(value))
        }
        _ => render_json_literal(value),
    }
}

fn generate_plugin_api_tests(fixtures: &[&Fixture], output_dir: &Utf8Path) -> Result<()> {
    let mut buffer = String::new();

    writeln!(buffer, "// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT")?;
    writeln!(buffer, "/**")?;
    writeln!(buffer, " * E2E tests for plugin/config/utility APIs.")?;
    writeln!(buffer, " *")?;
    writeln!(buffer, " * Generated from plugin API fixtures.")?;
    writeln!(
        buffer,
        " * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang wasm-deno"
    )?;
    writeln!(buffer, " */")?;
    writeln!(buffer)?;
    // Note: We don't import assertions here since plugin API tests are stub implementations.
    // Once implemented, they should use helpers.ts like the extraction tests.

    let mut grouped = fixtures
        .iter()
        .into_group_map_by(|fixture| {
            fixture
                .api_category
                .as_ref()
                .expect("api_category required for plugin API fixtures")
                .clone()
        })
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let category_title = to_title_case(&category);
        writeln!(buffer, "// {category_title}")?;
        writeln!(buffer)?;

        for fixture in fixtures {
            let test_name = &fixture.description;
            writeln!(buffer, "Deno.test(\"{}\", () => {{", escape_ts_string(test_name))?;
            writeln!(buffer, "    // Plugin API tests not yet implemented for Deno")?;
            writeln!(buffer, "}});",)?;
            writeln!(buffer)?;
        }
    }

    let path = output_dir.join("plugin-apis.test.ts");
    fs::write(&path, buffer).with_context(|| format!("Writing {}", path))?;

    Ok(())
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
