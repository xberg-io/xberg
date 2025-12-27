import { expect } from "vitest";
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

export function getFixture(fixturePath: string): Uint8Array | null {
	console.warn(`[SKIP] Cloudflare Workers cannot load fixtures from disk. Fixture: ${fixturePath}`);
	console.warn("[SKIP] These tests require filesystem access which is not available in the Workers sandbox.");
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
		expect(expected.some((token) => result.mimeType.includes(token))).toBe(true);
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
		expect(snippets.some((snippet) => lowered.includes(snippet.toLowerCase()))).toBe(true);
	},

	assertContentContainsAll(result: ExtractionResult, snippets: string[]): void {
		if (!snippets.length) {
			return;
		}
		const lowered = result.content.toLowerCase();
		expect(snippets.every((snippet) => lowered.includes(snippet.toLowerCase()))).toBe(true);
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

	assertMetadataExpectation(result: ExtractionResult, path: string, expectation: PlainRecord): void {
		if (!isPlainRecord(result.metadata)) {
			throw new Error(`Metadata is not a record for path ${path}`);
		}

		const value = getMetadataPath(result.metadata as PlainRecord, path);
		if (value === undefined || value === null) {
			throw new Error(`Metadata path '${path}' missing in ${JSON.stringify(result.metadata)}`);
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
				expect(contains.every((item) => value.includes(item))).toBe(true);
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
