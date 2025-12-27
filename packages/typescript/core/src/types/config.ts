/**
 * Configuration interfaces for Kreuzberg extraction options.
 *
 * These types define all configurable parameters for document extraction,
 * including OCR, chunking, image processing, and post-processing options.
 */

// ============================================================================
// ============================================================================

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
	chunkSize?: number;
	chunkOverlap?: number;
	preset?: string;
	embedding?: Record<string, unknown>;
	enabled?: boolean;
}

export interface LanguageDetectionConfig {
	enabled?: boolean;
	minConfidence?: number;
	detectMultiple?: boolean;
}

export interface TokenReductionConfig {
	mode?: string;
	preserveImportantWords?: boolean;
}

export interface FontConfig {
	enabled?: boolean;
	customFontDirs?: string[];
}

export interface PdfConfig {
	extractImages?: boolean;
	passwords?: string[];
	extractMetadata?: boolean;
	fontConfig?: FontConfig;
}

export interface ImageExtractionConfig {
	extractImages?: boolean;
	targetDpi?: number;
	maxImageDimension?: number;
	autoAdjustDpi?: boolean;
	minDpi?: number;
	maxDpi?: number;
}

export interface PostProcessorConfig {
	enabled?: boolean;
	enabledProcessors?: string[];
	disabledProcessors?: string[];
}

export interface HtmlPreprocessingOptions {
	enabled?: boolean;
	preset?: "minimal" | "standard" | "aggressive";
	removeNavigation?: boolean;
	removeForms?: boolean;
}

export interface HtmlConversionOptions {
	headingStyle?: "atx" | "underlined" | "atx_closed";
	listIndentType?: "spaces" | "tabs";
	listIndentWidth?: number;
	bullets?: string;
	strongEmSymbol?: string;
	escapeAsterisks?: boolean;
	escapeUnderscores?: boolean;
	escapeMisc?: boolean;
	escapeAscii?: boolean;
	codeLanguage?: string;
	autolinks?: boolean;
	defaultTitle?: boolean;
	brInTables?: boolean;
	hocrSpatialTables?: boolean;
	highlightStyle?: "double_equal" | "html" | "bold" | "none";
	extractMetadata?: boolean;
	whitespaceMode?: "normalized" | "strict";
	stripNewlines?: boolean;
	wrap?: boolean;
	wrapWidth?: number;
	convertAsInline?: boolean;
	subSymbol?: string;
	supSymbol?: string;
	newlineStyle?: "spaces" | "backslash";
	codeBlockStyle?: "indented" | "backticks" | "tildes";
	keepInlineImagesIn?: string[];
	encoding?: string;
	debug?: boolean;
	stripTags?: string[];
	preserveTags?: string[];
	preprocessing?: HtmlPreprocessingOptions;
}

export type KeywordAlgorithm = "yake" | "rake";

export interface YakeParams {
	windowSize?: number;
}

export interface RakeParams {
	minWordLength?: number;
	maxWordsPerPhrase?: number;
}

export interface KeywordConfig {
	algorithm?: KeywordAlgorithm;
	maxKeywords?: number;
	minScore?: number;
	ngramRange?: [number, number];
	language?: string;
	yakeParams?: YakeParams;
	rakeParams?: RakeParams;
}

export interface ExtractionConfig {
	useCache?: boolean;
	enableQualityProcessing?: boolean;
	ocr?: OcrConfig;
	forceOcr?: boolean;
	chunking?: ChunkingConfig;
	images?: ImageExtractionConfig;
	pdfOptions?: PdfConfig;
	tokenReduction?: TokenReductionConfig;
	languageDetection?: LanguageDetectionConfig;
	postprocessor?: PostProcessorConfig;
	htmlOptions?: HtmlConversionOptions;
	keywords?: KeywordConfig;
	maxConcurrentExtractions?: number;

	/**
	 * Serialize the configuration to a JSON string.
	 *
	 * Converts this configuration object to its JSON representation.
	 * The JSON can be used to create a new config via fromJson() or
	 * passed to extraction functions that accept JSON configs.
	 *
	 * @returns JSON string representation of the configuration
	 *
	 * @example
	 * ```typescript
	 * const config: ExtractionConfig = { useCache: true };
	 * const json = config.toJson();
	 * console.log(json); // '{"useCache":true,...}'
	 * ```
	 */
	toJson(): string;

	/**
	 * Get a configuration field by name (dot notation supported).
	 *
	 * Retrieves a nested configuration field using dot notation
	 * (e.g., "ocr.backend", "images.targetDpi").
	 *
	 * @param fieldName - The field path to retrieve
	 * @returns The field value as a JSON string, or null if not found
	 *
	 * @example
	 * ```typescript
	 * const config: ExtractionConfig = {
	 *   ocr: { backend: 'tesseract' }
	 * };
	 * const backend = config.getField('ocr.backend');
	 * console.log(backend); // '"tesseract"'
	 *
	 * const missing = config.getField('nonexistent');
	 * console.log(missing); // null
	 * ```
	 */
	getField(fieldName: string): string | null;

	/**
	 * Merge another configuration into this one.
	 *
	 * Performs a shallow merge where fields from the other config
	 * take precedence over this config's fields. Modifies this config
	 * in-place.
	 *
	 * @param other - Configuration to merge in (takes precedence)
	 *
	 * @example
	 * ```typescript
	 * const base: ExtractionConfig = { useCache: true, forceOcr: false };
	 * const override: ExtractionConfig = { forceOcr: true };
	 * base.merge(override);
	 * console.log(base.useCache); // true
	 * console.log(base.forceOcr); // true
	 * ```
	 */
	merge(other: ExtractionConfig): void;
}
