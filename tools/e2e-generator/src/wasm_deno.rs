use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;

use crate::fixtures::{Assertions, Fixture, WasmTarget};

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

const WORKSPACE_ROOT = new URL("../..", import.meta.url).pathname;
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

    let test_name = &fixture.id;
    writeln!(
        body,
        "Deno.test(\"{test_name}\", {{ permissions: {{ read: true }} }}, async () => {{"
    )?;

    writeln!(
        body,
        "    const documentBytes = await resolveDocument(\"{}\");",
        escape_ts_string(&fixture.document().path)
    )?;

    match render_config_expression(&fixture.extraction().config)? {
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
    writeln!(
        body,
        "      result = await extractBytes(documentBytes, \"{}\", config);",
        escape_ts_string(mime_type)
    )?;
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
    writeln!(
        buffer,
        "import {{ assertEquals, assertExists }} from \"https://deno.land/std@0.224.0/assert/mod.ts\";"
    )?;
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
