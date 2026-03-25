import { assignBooleanField, assignNumberField, assignStringArrayField, assignStringField } from "./field-mappers.js";
import { isPlainRecord, type PlainRecord } from "./types.js";

/**
 * Config types - these should match the types from @kreuzberg/node
 * but are defined here to avoid circular dependencies
 */
export interface TesseractConfig {
	psm?: number;
	enableTableDetection?: boolean;
	tesseditCharWhitelist?: string;
}

export interface OcrConfig {
	backend: string;
	language?: string;
	tesseractConfig?: TesseractConfig;
}

export interface ChunkingConfig {
	maxChars?: number;
	maxOverlap?: number;
}

export interface ImageExtractionConfig {
	extractImages?: boolean;
	targetDpi?: number;
	maxImageDimension?: number;
	autoAdjustDpi?: boolean;
	minDpi?: number;
	maxDpi?: number;
}

export interface PdfConfig {
	extractImages?: boolean;
	passwords?: string[];
	extractMetadata?: boolean;
	extractAnnotations?: boolean;
	topMarginFraction?: number;
	bottomMarginFraction?: number;
}

export interface TokenReductionConfig {
	mode?: string;
	preserveImportantWords?: boolean;
}

export interface LanguageDetectionConfig {
	enabled?: boolean;
	minConfidence?: number;
	detectMultiple?: boolean;
}

export interface PostProcessorConfig {
	enabled?: boolean;
	enabledProcessors?: string[];
	disabledProcessors?: string[];
}

export interface ExtractionConfig {
	useCache?: boolean;
	enableQualityProcessing?: boolean;
	forceOcr?: boolean;
	forceOcrPages?: number[];
	maxConcurrentExtractions?: number;
	extractionTimeoutSecs?: number;
	ocr?: OcrConfig;
	chunking?: ChunkingConfig;
	images?: ImageExtractionConfig;
	pdfOptions?: PdfConfig;
	tokenReduction?: TokenReductionConfig;
	languageDetection?: LanguageDetectionConfig;
	postprocessor?: PostProcessorConfig;
}

/**
 * Maps a plain object to TesseractConfig
 */
function mapTesseractConfig(raw: PlainRecord): TesseractConfig {
	const config: TesseractConfig = {};
	assignNumberField(config as PlainRecord, raw, "psm", "psm");
	assignBooleanField(config as PlainRecord, raw, "enable_table_detection", "enableTableDetection");
	assignStringField(config as PlainRecord, raw, "tessedit_char_whitelist", "tesseditCharWhitelist");
	return config;
}

/**
 * Maps a plain object to OcrConfig
 */
function mapOcrConfig(raw: PlainRecord): OcrConfig | undefined {
	const backend = raw["backend"];
	if (typeof backend !== "string" || backend.length === 0) {
		return undefined;
	}

	const config: OcrConfig = { backend };
	assignStringField(config as unknown as PlainRecord, raw, "language", "language");

	if (isPlainRecord(raw["tesseract_config"])) {
		config.tesseractConfig = mapTesseractConfig(raw["tesseract_config"]);
	}

	return config;
}

/**
 * Maps a plain object to ChunkingConfig
 */
function mapChunkingConfig(raw: PlainRecord): ChunkingConfig {
	const config: ChunkingConfig = {};
	assignNumberField(config as PlainRecord, raw, "max_chars", "maxChars");
	assignNumberField(config as PlainRecord, raw, "max_overlap", "maxOverlap");
	return config;
}

/**
 * Maps a plain object to ImageExtractionConfig
 */
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

/**
 * Maps a plain object to PdfConfig
 */
function mapPdfConfig(raw: PlainRecord): PdfConfig {
	const config: PdfConfig = {};
	assignBooleanField(config as PlainRecord, raw, "extract_images", "extractImages");
	assignStringArrayField(config as PlainRecord, raw, "passwords", "passwords");
	assignBooleanField(config as PlainRecord, raw, "extract_metadata", "extractMetadata");
	assignBooleanField(config as PlainRecord, raw, "extract_annotations", "extractAnnotations");
	assignNumberField(config as PlainRecord, raw, "top_margin_fraction", "topMarginFraction");
	assignNumberField(config as PlainRecord, raw, "bottom_margin_fraction", "bottomMarginFraction");
	return config;
}

/**
 * Maps a plain object to TokenReductionConfig
 */
function mapTokenReductionConfig(raw: PlainRecord): TokenReductionConfig {
	const config: TokenReductionConfig = {};
	assignStringField(config as PlainRecord, raw, "mode", "mode");
	assignBooleanField(config as PlainRecord, raw, "preserve_important_words", "preserveImportantWords");
	return config;
}

/**
 * Maps a plain object to LanguageDetectionConfig
 */
function mapLanguageDetectionConfig(raw: PlainRecord): LanguageDetectionConfig {
	const config: LanguageDetectionConfig = {};
	assignBooleanField(config as PlainRecord, raw, "enabled", "enabled");
	assignNumberField(config as PlainRecord, raw, "min_confidence", "minConfidence");
	assignBooleanField(config as PlainRecord, raw, "detect_multiple", "detectMultiple");
	return config;
}

/**
 * Maps a plain object to PostProcessorConfig
 */
function mapPostProcessorConfig(raw: PlainRecord): PostProcessorConfig {
	const config: PostProcessorConfig = {};
	assignBooleanField(config as PlainRecord, raw, "enabled", "enabled");
	assignStringArrayField(config as PlainRecord, raw, "enabled_processors", "enabledProcessors");
	assignStringArrayField(config as PlainRecord, raw, "disabled_processors", "disabledProcessors");
	return config;
}

/**
 * Builds an ExtractionConfig from a plain object, converting snake_case to camelCase
 * and handling nested config objects
 */
export function buildConfig(raw: unknown): ExtractionConfig {
	if (!isPlainRecord(raw)) {
		return {};
	}

	const source = raw;
	const result: ExtractionConfig = {};
	const target = result as PlainRecord;

	assignBooleanField(target, source, "use_cache", "useCache");
	assignBooleanField(target, source, "enable_quality_processing", "enableQualityProcessing");
	assignBooleanField(target, source, "force_ocr", "forceOcr");

	const forceOcrPages = source["force_ocr_pages"];
	if (Array.isArray(forceOcrPages)) {
		result.forceOcrPages = forceOcrPages.filter((v): v is number => typeof v === "number");
	}

	assignNumberField(target, source, "max_concurrent_extractions", "maxConcurrentExtractions");
	assignNumberField(target, source, "extraction_timeout_secs", "extractionTimeoutSecs");

	if (isPlainRecord(source["ocr"])) {
		const mapped = mapOcrConfig(source["ocr"]);
		if (mapped) {
			result.ocr = mapped;
		}
	}

	if (isPlainRecord(source["chunking"])) {
		result.chunking = mapChunkingConfig(source["chunking"]);
	}

	if (isPlainRecord(source["images"])) {
		result.images = mapImageExtractionConfig(source["images"]);
	}

	if (isPlainRecord(source["pdf_options"])) {
		result.pdfOptions = mapPdfConfig(source["pdf_options"]);
	}

	if (isPlainRecord(source["token_reduction"])) {
		result.tokenReduction = mapTokenReductionConfig(source["token_reduction"]);
	}

	if (isPlainRecord(source["language_detection"])) {
		result.languageDetection = mapLanguageDetectionConfig(source["language_detection"]);
	}

	if (isPlainRecord(source["postprocessor"])) {
		result.postprocessor = mapPostProcessorConfig(source["postprocessor"]);
	}

	return result;
}
