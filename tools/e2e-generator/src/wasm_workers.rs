use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;

use crate::fixtures::{Assertions, ExtractionMethod, Fixture, InputType, WasmTarget};

const WORKERS_HELPERS_TEMPLATE: &str = r#"import { expect } from "vitest";
import type {
    ChunkingConfig,
    ExtractionConfig,
    ExtractionResult,
    ImageExtractionConfig,
    LanguageDetectionConfig,
    OcrConfig,
    PdfConfig,
    PostProcessorConfig,
    TesseractConfig,
    TokenReductionConfig,
} from "@kreuzberg/wasm";

// CRITICAL: Cloudflare Workers cannot access the filesystem
// All fixture-based tests are skipped in this environment
export function getFixture(fixturePath: string): Uint8Array | null {
    console.warn(
        `[SKIP] Cloudflare Workers cannot load fixtures from disk. Fixture: ${fixturePath}`,
    );
    console.warn(
        "[SKIP] These tests require filesystem access which is not available in the Workers sandbox.",
    );
    return null;
}

type PlainRecord = Record<string, unknown>;

function isPlainRecord(value: unknown): value is PlainRecord {
    return typeof value === "object" && value !== null;
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
    const config: TesseractConfig = {};
    assignNumberField(config as PlainRecord, raw, "psm", "psm");
    assignBooleanField(config as PlainRecord, raw, "enable_table_detection", "enableTableDetection");
    if (typeof raw.tessedit_char_whitelist === "string") {
        config.tesseditCharWhitelist = raw.tessedit_char_whitelist as string;
    }
    return config;
}

function mapOcrConfig(raw: PlainRecord): OcrConfig | undefined {
    const backend = raw.backend;
    if (typeof backend !== "string" || backend.length === 0) {
        return undefined;
    }

    const config: OcrConfig = { backend };
    if (typeof raw.language === "string") {
        config.language = raw.language as string;
    }

    if (isPlainRecord(raw.tesseract_config)) {
        config.tesseractConfig = mapTesseractConfig(raw.tesseract_config as PlainRecord);
    }

    return config;
}

function mapChunkingConfig(raw: PlainRecord): ChunkingConfig {
    const config: ChunkingConfig = {};
    assignNumberField(config as PlainRecord, raw, "max_chars", "maxChars");
    assignNumberField(config as PlainRecord, raw, "max_overlap", "maxOverlap");
    return config;
}

function mapImageExtractionConfig(raw: PlainRecord): ImageExtractionConfig {
    const config: ImageExtractionConfig = {};
    assignBooleanField(config as PlainRecord, raw, "extract_images", "extractImages");
    assignNumberField(config as PlainRecord, raw, "target_dpi", "targetDpi");
    assignNumberField(config as PlainRecord, raw, "max_image_dimension", "maxImageDimension");
    assignBooleanField(config as PlainRecord, raw, "auto_adjust_dpi", "autoAdjustDpi");
    assignNumberField(config as PlainRecord, raw, "min_dpi", "minDpi");
    assignNumberField(config as PlainRecord, raw, "max_dpi", "maxDpi");
    return config;
}

function mapPdfConfig(raw: PlainRecord): PdfConfig {
    const config: PdfConfig = {};
    assignBooleanField(config as PlainRecord, raw, "extract_images", "extractImages");
    if (Array.isArray(raw.passwords)) {
        config.passwords = raw.passwords.filter((item: unknown): item is string => typeof item === "string");
    }
    assignBooleanField(config as PlainRecord, raw, "extract_metadata", "extractMetadata");
    return config;
}

function mapTokenReductionConfig(raw: PlainRecord): TokenReductionConfig {
    const config: TokenReductionConfig = {};
    if (typeof raw.mode === "string") {
        config.mode = raw.mode as string;
    }
    assignBooleanField(config as PlainRecord, raw, "preserve_important_words", "preserveImportantWords");
    return config;
}

function mapLanguageDetectionConfig(raw: PlainRecord): LanguageDetectionConfig {
    const config: LanguageDetectionConfig = {};
    assignBooleanField(config as PlainRecord, raw, "enabled", "enabled");
    assignNumberField(config as PlainRecord, raw, "min_confidence", "minConfidence");
    assignBooleanField(config as PlainRecord, raw, "detect_multiple", "detectMultiple");
    return config;
}

function mapPostProcessorConfig(raw: PlainRecord): PostProcessorConfig {
    const config: PostProcessorConfig = {};
    assignBooleanField(config as PlainRecord, raw, "enabled", "enabled");
    const enabled = mapStringArray(raw.enabled_processors);
    if (enabled) {
        config.enabledProcessors = enabled;
    }
    const disabled = mapStringArray(raw.disabled_processors);
    if (disabled) {
        config.disabledProcessors = disabled;
    }
    return config;
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
        expect(
            expected.some((token) => result.mimeType.includes(token)),
        ).toBe(true);
    },

    assertMinContentLength(result: ExtractionResult, minimum: number): void {
        expect(result.content.length >= minimum).toBe(true);
    },

    assertMaxContentLength(result: ExtractionResult, maximum: number): void {
        expect(result.content.length <= maximum).toBe(true);
    },

    assertContentContainsAny(result: ExtractionResult, snippets: string[]): void {
        if (!snippets.length) {
            return;
        }
        const lowered = result.content.toLowerCase();
        expect(
            snippets.some((snippet) => lowered.includes(snippet.toLowerCase())),
        ).toBe(true);
    },

    assertContentContainsAll(result: ExtractionResult, snippets: string[]): void {
        if (!snippets.length) {
            return;
        }
        const lowered = result.content.toLowerCase();
        expect(
            snippets.every((snippet) => lowered.includes(snippet.toLowerCase())),
        ).toBe(true);
    },

    assertTableCount(result: ExtractionResult, minimum?: number | null, maximum?: number | null): void {
        const tables = Array.isArray(result.tables) ? result.tables : [];
        if (typeof minimum === "number") {
            expect(tables.length >= minimum).toBe(true);
        }
        if (typeof maximum === "number") {
            expect(tables.length <= maximum).toBe(true);
        }
    },

    assertDetectedLanguages(result: ExtractionResult, expected: string[], minConfidence?: number | null): void {
        if (!expected.length) {
            return;
        }
        expect(result.detectedLanguages).toBeDefined();
        const languages = result.detectedLanguages ?? [];
        expect(expected.every((lang) => languages.includes(lang))).toBe(true);

        if (typeof minConfidence === "number" && isPlainRecord(result.metadata)) {
            const confidence = (result.metadata as PlainRecord).confidence;
            if (typeof confidence === "number") {
                expect(confidence >= minConfidence).toBe(true);
            }
        }
    },

    assertMetadataExpectation(
        result: ExtractionResult,
        path: string,
        expectation: PlainRecord | string | number | boolean,
    ): void {
        if (!isPlainRecord(result.metadata)) {
            throw new Error(`Metadata is not a record for path ${path}`);
        }

        const value = getMetadataPath(result.metadata as PlainRecord, path);
        if (value === undefined || value === null) {
            throw new Error(`Metadata path '${path}' missing in ${JSON.stringify(result.metadata)}`);
        }

        if (!isPlainRecord(expectation)) {
            expect(valuesEqual(value, expectation)).toBe(true);
            return;
        }

        if ("eq" in expectation) {
            expect(valuesEqual(value, expectation.eq)).toBe(true);
        }

        if ("gte" in expectation) {
            expect(Number(value) >= Number(expectation.gte)).toBe(true);
        }

        if ("lte" in expectation) {
            expect(Number(value) <= Number(expectation.lte)).toBe(true);
        }

        if ("contains" in expectation) {
            const contains = expectation.contains;
            if (typeof value === "string" && typeof contains === "string") {
                expect(value.includes(contains)).toBe(true);
            } else if (Array.isArray(value) && Array.isArray(contains)) {
                expect(contains.every((item: unknown) => (value as unknown[]).includes(item))).toBe(true);
            } else {
                throw new Error(`Unsupported contains expectation for path '${path}'`);
            }
        }
    },

    assertChunks(
        result: ExtractionResult,
        minCount: number | null,
        maxCount: number | null,
        eachHasContent: boolean | null,
        eachHasEmbedding: boolean | null,
    ): void {
        const chunks = Array.isArray(result.chunks) ? result.chunks : [];
        if (typeof minCount === "number") {
            expect(chunks.length >= minCount).toBe(true);
        }
        if (typeof maxCount === "number") {
            expect(chunks.length <= maxCount).toBe(true);
        }
        if (eachHasContent === true) {
            for (const chunk of chunks) {
                expect(typeof chunk.content === "string" && chunk.content.length > 0).toBe(true);
            }
        }
        if (eachHasEmbedding === true) {
            for (const chunk of chunks) {
                expect(chunk.embedding !== undefined && chunk.embedding !== null).toBe(true);
            }
        }
    },

    assertImages(
        result: ExtractionResult,
        minCount: number | null,
        maxCount: number | null,
        formatsInclude: string[] | null,
    ): void {
        const images = Array.isArray(result.images) ? result.images : [];
        if (typeof minCount === "number") {
            expect(images.length >= minCount).toBe(true);
        }
        if (typeof maxCount === "number") {
            expect(images.length <= maxCount).toBe(true);
        }
        if (formatsInclude && formatsInclude.length > 0) {
            const formats = images.map((img) => img.format ?? img.mimeType ?? "").filter(Boolean);
            for (const expected of formatsInclude) {
                expect(formats.some((f) => f.toLowerCase().includes(expected.toLowerCase()))).toBe(true);
            }
        }
    },

    assertPages(result: ExtractionResult, minCount: number | null, exactCount: number | null): void {
        const pages = Array.isArray(result.pages) ? result.pages : [];
        if (typeof minCount === "number") {
            expect(pages.length >= minCount).toBe(true);
        }
        if (typeof exactCount === "number") {
            expect(pages.length).toBe(exactCount);
        }
    },

    assertElements(result: ExtractionResult, minCount: number | null, typesInclude: string[] | null): void {
        const elements = Array.isArray(result.elements) ? result.elements : [];
        if (typeof minCount === "number") {
            expect(elements.length >= minCount).toBe(true);
        }
        if (typesInclude && typesInclude.length > 0) {
            const types = elements.map((el) => el.element_type ?? "").filter(Boolean);
            for (const expected of typesInclude) {
                expect(types.some((t) => t.toLowerCase().includes(expected.toLowerCase()))).toBe(true);
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
            expect(doc).toBeDefined();
            expect(doc).not.toBeNull();
            const nodes = (doc as PlainRecord).nodes as unknown[];
            expect(nodes).toBeDefined();
            if (typeof minNodeCount === "number") {
                expect(nodes.length).toBeGreaterThanOrEqual(minNodeCount);
            }
            if (nodeTypesInclude && nodeTypesInclude.length > 0) {
                const foundTypes = new Set(nodes.map((n) => ((n as PlainRecord).content as PlainRecord)?.node_type ?? (n as PlainRecord).node_type));
                for (const expected of nodeTypesInclude) {
                    const found = [...foundTypes].some((t) => typeof t === "string" && t.toLowerCase() === expected.toLowerCase());
                    expect(found).toBe(true);
                }
            }
            if (typeof hasGroups === "boolean") {
                const hasGroupNodes = nodes.some((n) => ((n as PlainRecord).content as PlainRecord)?.node_type === "group" || (n as PlainRecord).node_type === "group");
                expect(hasGroupNodes).toBe(hasGroups);
            }
        } else {
            expect(doc).toBeUndefined();
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

/// Generate Cloudflare Workers/WASM test suite from fixtures.
pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let output_dir = output_root.join("wasm-workers");
    let tests_dir = output_dir.join("tests");

    fs::create_dir_all(&tests_dir).context("Failed to create Workers tests directory")?;

    clean_test_files(&tests_dir)?;

    let doc_fixtures: Vec<_> = fixtures
        .iter()
        .filter(|f| f.is_document_extraction() && crate::fixtures::should_include_for_wasm(f, WasmTarget::Workers))
        .collect();

    let plugin_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_plugin_api()).collect();

    write_helpers(&tests_dir)?;

    let mut grouped = doc_fixtures
        .into_iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let file_name = format!("{}.spec.ts", to_snake_case(&category));
        let content = render_category(&category, &fixtures)?;
        let path = tests_dir.join(&file_name);
        fs::write(&path, content).with_context(|| format!("Writing {}", path))?;
    }

    if !plugin_fixtures.is_empty() {
        generate_plugin_api_tests(&plugin_fixtures, &tests_dir)?;
    }

    write_vitest_config(&output_dir)?;

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

        if file_name.ends_with(".spec.ts") {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

fn write_helpers(output_dir: &Utf8Path) -> Result<()> {
    let helpers_path = output_dir.join("helpers.ts");
    fs::write(&helpers_path, WORKERS_HELPERS_TEMPLATE).context("Failed to write helpers.ts")?;
    Ok(())
}

fn render_category(category: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "// Auto-generated tests for {category} fixtures.")?;
    writeln!(buffer, "// Designed for Cloudflare Workers with Vitest + Miniflare")?;
    writeln!(buffer)?;
    writeln!(buffer, "import {{ describe, it, expect }} from \"vitest\";")?;

    // Collect required imports based on extraction methods used
    // WASM exports: extractBytes (async), extractBytesSync, batchExtractBytes, batchExtractBytesSync
    let mut needs_extract_bytes = false;
    let needs_extract_bytes_sync = false;
    let mut needs_batch_extract_bytes = false;
    let mut needs_batch_extract_bytes_sync = false;

    for fixture in fixtures {
        let extraction = fixture.extraction();
        match extraction.method {
            ExtractionMethod::Sync => needs_extract_bytes = true,
            ExtractionMethod::Async => needs_extract_bytes = true,
            ExtractionMethod::BatchSync => needs_batch_extract_bytes_sync = true,
            ExtractionMethod::BatchAsync => needs_batch_extract_bytes = true,
        }
    }

    let mut imports = Vec::new();
    if needs_extract_bytes {
        imports.push("extractBytes");
    }
    if needs_extract_bytes_sync {
        imports.push("extractBytesSync");
    }
    if needs_batch_extract_bytes {
        imports.push("batchExtractBytes");
    }
    if needs_batch_extract_bytes_sync {
        imports.push("batchExtractBytesSync");
    }

    // Default to extractBytes if no specific methods found
    if imports.is_empty() {
        imports.push("extractBytes");
    }

    writeln!(buffer, "import {{ {} }} from \"@kreuzberg/wasm\";", imports.join(", "))?;
    writeln!(
        buffer,
        "import {{ assertions, buildConfig, getFixture, shouldSkipFixture }} from \"./helpers.js\";"
    )?;
    writeln!(buffer, "import type {{ ExtractionResult }} from \"@kreuzberg/wasm\";\n")?;
    writeln!(buffer, "describe(\"{category}\", () => {{")?;

    for fixture in fixtures {
        buffer.write_str(&render_test(fixture)?)?;
    }

    writeln!(buffer, "}});")?;

    Ok(buffer)
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut body = String::new();

    let test_name = &fixture.id;
    let extraction = fixture.extraction();
    let method = extraction.method;
    let input_type = extraction.input_type;

    writeln!(body, "    it(\"{test_name}\", async () => {{")?;

    writeln!(
        body,
        "        const documentBytes = getFixture(\"{}\");",
        escape_ts_string(&fixture.document().path)
    )?;
    writeln!(body, "        if (documentBytes === null) {{")?;
    writeln!(
        body,
        "            console.warn(\"[SKIP] Test skipped: fixture not available in Cloudflare Workers environment\");"
    )?;
    writeln!(body, "            return;")?;
    writeln!(body, "        }}\n")?;

    match render_config_expression(&extraction.config)? {
        None => writeln!(body, "        const config = buildConfig(undefined);")?,
        Some(config_expr) => writeln!(body, "        const config = buildConfig({config_expr});")?,
    }

    let requirements = collect_requirements(fixture);
    let mime_type = fixture
        .document()
        .media_type
        .as_deref()
        .unwrap_or("application/octet-stream");

    // Generate extraction call based on method and input_type
    let extraction_call = render_extraction_call(method, input_type, mime_type);

    writeln!(body, "        let result: ExtractionResult | null = null;")?;
    writeln!(body, "        try {{")?;
    writeln!(body, "{extraction_call}")?;
    writeln!(body, "        }} catch (error) {{")?;
    if !requirements.is_empty()
        || fixture.skip().notes.is_some()
        || !fixture.document().requires_external_tool.is_empty()
    {
        writeln!(
            body,
            "            if (shouldSkipFixture(error, \"{}\", {}, {})) {{",
            escape_ts_string(&fixture.id),
            render_string_array(&requirements),
            render_optional_string(fixture.skip().notes.as_ref())
        )?;
    } else {
        writeln!(
            body,
            "            if (shouldSkipFixture(error, \"{}\", [], undefined)) {{",
            escape_ts_string(&fixture.id)
        )?;
    }
    writeln!(
        body,
        "                return;\n            }}\n            throw error;\n        }}"
    )?;
    writeln!(body, "        if (result === null) {{\n            return;\n        }}")?;

    body.push_str(&render_assertions(&fixture.assertions()));

    writeln!(body, "    }});\n")?;

    Ok(body)
}

fn render_extraction_call(method: ExtractionMethod, input_type: InputType, mime_type: &str) -> String {
    let escaped_mime = escape_ts_string(mime_type);

    // In Workers environment, all operations use bytes (no filesystem access)
    // WASM exports: extractBytes (async), extractBytesSync, batchExtractBytes, batchExtractBytesSync
    match (method, input_type) {
        // Sync/async single extraction - use extractBytes (which is async)
        (ExtractionMethod::Sync, _) | (ExtractionMethod::Async, _) => {
            format!("            result = await extractBytes(documentBytes, \"{escaped_mime}\", config);")
        }
        // Batch sync: use batchExtractBytesSync
        (ExtractionMethod::BatchSync, _) => {
            format!(
                "            const results = await batchExtractBytesSync([{{ data: documentBytes, mimeType: \"{escaped_mime}\" }}], config);\n            result = results[0] ?? null;"
            )
        }
        // Batch async: use batchExtractBytes
        (ExtractionMethod::BatchAsync, _) => {
            format!(
                "            const results = await batchExtractBytes([{{ data: documentBytes, mimeType: \"{escaped_mime}\" }}], config);\n            result = results[0] ?? null;"
            )
        }
    }
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
            "        assertions.assertExpectedMime(result, {});\n",
            render_string_array(&assertions.expected_mime)
        ));
    }

    if let Some(min) = assertions.min_content_length {
        buffer.push_str(&format!("        assertions.assertMinContentLength(result, {min});\n"));
    }

    if let Some(max) = assertions.max_content_length {
        buffer.push_str(&format!("        assertions.assertMaxContentLength(result, {max});\n"));
    }

    if !assertions.content_contains_any.is_empty() {
        buffer.push_str(&format!(
            "        assertions.assertContentContainsAny(result, {});\n",
            render_string_array(&assertions.content_contains_any)
        ));
    }

    if !assertions.content_contains_all.is_empty() {
        buffer.push_str(&format!(
            "        assertions.assertContentContainsAll(result, {});\n",
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
        buffer.push_str(&format!("        assertions.assertTableCount(result, {min}, {max});\n"));
    }

    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_array(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "        assertions.assertDetectedLanguages(result, {expected}, {min_conf});\n"
        ));
    }

    if !assertions.metadata.is_empty() {
        for (path, expectation) in &assertions.metadata {
            buffer.push_str(&format!(
                "        assertions.assertMetadataExpectation(result, \"{}\", {});\n",
                escape_ts_string(path),
                render_json_literal(expectation)
            ));
        }
    }

    if let Some(chunks) = assertions.chunks.as_ref() {
        let min_count = chunks.min_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let max_count = chunks.max_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let each_has_content = chunks
            .each_has_content
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let each_has_embedding = chunks
            .each_has_embedding
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "        assertions.assertChunks(result, {min_count}, {max_count}, {each_has_content}, {each_has_embedding});\n"
        ));
    }

    if let Some(images) = assertions.images.as_ref() {
        let min_count = images.min_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let max_count = images.max_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let formats_include = images
            .formats_include
            .as_ref()
            .map(|f| render_string_array(f))
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "        assertions.assertImages(result, {min_count}, {max_count}, {formats_include});\n"
        ));
    }

    if let Some(pages) = assertions.pages.as_ref() {
        let min_count = pages.min_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let exact_count = pages
            .exact_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "        assertions.assertPages(result, {min_count}, {exact_count});\n"
        ));
    }

    if let Some(elements) = assertions.elements.as_ref() {
        let min_count = elements
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let types_include = elements
            .types_include
            .as_ref()
            .map(|t| render_string_array(t))
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "        assertions.assertElements(result, {min_count}, {types_include});\n"
        ));
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
        let min_count = ocr.min_count.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "        assertions.assertOcrElements(result, {has_elements}, {has_geometry}, {has_confidence}, {min_count});\n"
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
            "        assertions.assertDocument(result, {has_document}, {min_node_count}, {node_types}, {has_groups});\n"
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

fn generate_plugin_api_tests(fixtures: &[&Fixture], output_dir: &Utf8Path) -> Result<()> {
    let mut buffer = String::new();

    writeln!(buffer, "// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT")?;
    writeln!(buffer, "/**")?;
    writeln!(buffer, " * E2E tests for plugin/config/utility APIs.")?;
    writeln!(buffer, " *")?;
    writeln!(buffer, " * Generated from plugin API fixtures.")?;
    writeln!(
        buffer,
        " * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang wasm-workers"
    )?;
    writeln!(buffer, " */")?;
    writeln!(buffer)?;
    writeln!(buffer, "import {{ describe, it, expect }} from \"vitest\";")?;
    writeln!(buffer)?;

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
            writeln!(buffer, "describe(\"{}\", () => {{", escape_ts_string(test_name))?;
            writeln!(
                buffer,
                "    it(\"should test {}\", () => {{",
                escape_ts_string(&fixture.id)
            )?;
            writeln!(buffer, "        // Plugin API tests not yet implemented for Workers")?;
            writeln!(buffer, "    }});")?;
            writeln!(buffer, "}});")?;
            writeln!(buffer)?;
        }
    }

    let path = output_dir.join("plugin-apis.spec.ts");
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

fn write_vitest_config(output_dir: &Utf8Path) -> Result<()> {
    let vitest_config = r#"import { defineWorkersConfig } from "@cloudflare/vitest-pool-workers/config";

export default defineWorkersConfig({
    test: {
        globals: true,
        poolOptions: {
            workers: {
                main: "./tests/index.ts",
                wrangler: {
                    configPath: "./wrangler.toml",
                },
            },
        },
        testTimeout: 60000,
    },
});
"#;

    let config_path = output_dir.join("vitest.config.ts");
    fs::write(&config_path, vitest_config).context("Failed to write vitest.config.ts")?;
    Ok(())
}
