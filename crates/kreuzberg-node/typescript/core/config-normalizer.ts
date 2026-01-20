/**
 * Configuration normalization utilities for converting TypeScript config types
 * to native binding format.
 *
 * This module handles transformation of all extraction configuration options
 * from user-friendly TypeScript types to the format expected by the native
 * Rust binding.
 *
 * @internal This module is part of the core infrastructure layer (Layer 1).
 */

import type {
	ChunkingConfig,
	ExtractionConfig,
	HtmlConversionOptions,
	HtmlPreprocessingOptions,
	ImageExtractionConfig,
	KeywordConfig,
	LanguageDetectionConfig,
	OcrConfig,
	PageExtractionConfig,
	PdfConfig,
	PostProcessorConfig,
	TesseractConfig,
	TokenReductionConfig,
} from "../types.js";

/**
 * Native extraction config type used internally for native binding calls.
 * @internal
 */
type NativeExtractionConfig = Record<string, unknown>;

/**
 * Selectively set a property on the target object if the value is defined.
 * Used to avoid explicitly setting undefined values in config objects.
 *
 * @param target - Target object to set property on
 * @param key - Property key
 * @param value - Value to set (if defined)
 * @internal
 */
function setIfDefined<T>(target: NativeExtractionConfig, key: string, value: T | undefined): void {
	if (value !== undefined) {
		target[key] = value;
	}
}

/**
 * Normalize Tesseract OCR configuration.
 *
 * @param config - Tesseract configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizeTesseractConfig(config?: TesseractConfig) {
	if (!config) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "psm", config.psm);
	setIfDefined(normalized, "enableTableDetection", config.enableTableDetection);
	setIfDefined(normalized, "tesseditCharWhitelist", config.tesseditCharWhitelist);
	return normalized;
}

/**
 * Normalize OCR backend configuration.
 *
 * @param ocr - OCR configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizeOcrConfig(ocr?: OcrConfig): NativeExtractionConfig | undefined {
	if (!ocr) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {
		backend: ocr.backend,
	};
	setIfDefined(normalized, "language", ocr.language);

	const tesseract = normalizeTesseractConfig(ocr.tesseractConfig);
	if (tesseract) {
		setIfDefined(normalized, "tesseractConfig", tesseract);
	}

	return normalized;
}

/**
 * Normalize chunking configuration.
 *
 * @param chunking - Chunking configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizeChunkingConfig(chunking?: ChunkingConfig): NativeExtractionConfig | undefined {
	if (!chunking) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "maxChars", chunking.maxChars);
	setIfDefined(normalized, "maxOverlap", chunking.maxOverlap);
	setIfDefined(normalized, "preset", chunking.preset);
	setIfDefined(normalized, "embedding", chunking.embedding);
	setIfDefined(normalized, "enabled", chunking.enabled);
	return normalized;
}

/**
 * Normalize image extraction configuration.
 *
 * @param images - Image extraction configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizeImageExtractionConfig(images?: ImageExtractionConfig): NativeExtractionConfig | undefined {
	if (!images) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "extractImages", images.extractImages);
	setIfDefined(normalized, "targetDpi", images.targetDpi);
	setIfDefined(normalized, "maxImageDimension", images.maxImageDimension);
	setIfDefined(normalized, "autoAdjustDpi", images.autoAdjustDpi);
	setIfDefined(normalized, "minDpi", images.minDpi);
	setIfDefined(normalized, "maxDpi", images.maxDpi);
	return normalized;
}

/**
 * Normalize PDF-specific configuration.
 *
 * @param pdf - PDF configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizePdfConfig(pdf?: PdfConfig): NativeExtractionConfig | undefined {
	if (!pdf) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "extractImages", pdf.extractImages);
	setIfDefined(normalized, "passwords", pdf.passwords);
	setIfDefined(normalized, "extractMetadata", pdf.extractMetadata);
	return normalized;
}

/**
 * Normalize token reduction configuration.
 *
 * @param tokenReduction - Token reduction configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizeTokenReductionConfig(tokenReduction?: TokenReductionConfig): NativeExtractionConfig | undefined {
	if (!tokenReduction) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "mode", tokenReduction.mode);
	setIfDefined(normalized, "preserveImportantWords", tokenReduction.preserveImportantWords);
	return normalized;
}

/**
 * Normalize language detection configuration.
 *
 * @param languageDetection - Language detection configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizeLanguageDetectionConfig(
	languageDetection?: LanguageDetectionConfig,
): NativeExtractionConfig | undefined {
	if (!languageDetection) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "enabled", languageDetection.enabled);
	setIfDefined(normalized, "minConfidence", languageDetection.minConfidence);
	setIfDefined(normalized, "detectMultiple", languageDetection.detectMultiple);
	return normalized;
}

/**
 * Normalize post-processor configuration.
 *
 * @param postprocessor - Post-processor configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizePostProcessorConfig(postprocessor?: PostProcessorConfig): NativeExtractionConfig | undefined {
	if (!postprocessor) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "enabled", postprocessor.enabled);
	setIfDefined(normalized, "enabledProcessors", postprocessor.enabledProcessors);
	setIfDefined(normalized, "disabledProcessors", postprocessor.disabledProcessors);
	return normalized;
}

/**
 * Normalize HTML preprocessing options.
 *
 * @param options - HTML preprocessing options
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizeHtmlPreprocessing(options?: HtmlPreprocessingOptions): NativeExtractionConfig | undefined {
	if (!options) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "enabled", options.enabled);
	setIfDefined(normalized, "preset", options.preset);
	setIfDefined(normalized, "removeNavigation", options.removeNavigation);
	setIfDefined(normalized, "removeForms", options.removeForms);
	return normalized;
}

/**
 * Normalize HTML conversion options.
 * Includes nested preprocessing configuration.
 *
 * @param options - HTML conversion options
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizeHtmlOptions(options?: HtmlConversionOptions): NativeExtractionConfig | undefined {
	if (!options) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "headingStyle", options.headingStyle);
	setIfDefined(normalized, "listIndentType", options.listIndentType);
	setIfDefined(normalized, "listIndentWidth", options.listIndentWidth);
	setIfDefined(normalized, "bullets", options.bullets);
	setIfDefined(normalized, "strongEmSymbol", options.strongEmSymbol);
	setIfDefined(normalized, "escapeAsterisks", options.escapeAsterisks);
	setIfDefined(normalized, "escapeUnderscores", options.escapeUnderscores);
	setIfDefined(normalized, "escapeMisc", options.escapeMisc);
	setIfDefined(normalized, "escapeAscii", options.escapeAscii);
	setIfDefined(normalized, "codeLanguage", options.codeLanguage);
	setIfDefined(normalized, "autolinks", options.autolinks);
	setIfDefined(normalized, "defaultTitle", options.defaultTitle);
	setIfDefined(normalized, "brInTables", options.brInTables);
	setIfDefined(normalized, "hocrSpatialTables", options.hocrSpatialTables);
	setIfDefined(normalized, "highlightStyle", options.highlightStyle);
	setIfDefined(normalized, "extractMetadata", options.extractMetadata);
	setIfDefined(normalized, "whitespaceMode", options.whitespaceMode);
	setIfDefined(normalized, "stripNewlines", options.stripNewlines);
	setIfDefined(normalized, "wrap", options.wrap);
	setIfDefined(normalized, "wrapWidth", options.wrapWidth);
	setIfDefined(normalized, "convertAsInline", options.convertAsInline);
	setIfDefined(normalized, "subSymbol", options.subSymbol);
	setIfDefined(normalized, "supSymbol", options.supSymbol);
	setIfDefined(normalized, "newlineStyle", options.newlineStyle);
	setIfDefined(normalized, "codeBlockStyle", options.codeBlockStyle);
	setIfDefined(normalized, "keepInlineImagesIn", options.keepInlineImagesIn);
	setIfDefined(normalized, "encoding", options.encoding);
	setIfDefined(normalized, "debug", options.debug);
	setIfDefined(normalized, "stripTags", options.stripTags);
	setIfDefined(normalized, "preserveTags", options.preserveTags);

	const preprocessing = normalizeHtmlPreprocessing(options.preprocessing);
	setIfDefined(normalized, "preprocessing", preprocessing);

	return normalized;
}

/**
 * Normalize keyword extraction configuration.
 *
 * @param config - Keyword configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizeKeywordConfig(config?: KeywordConfig): NativeExtractionConfig | undefined {
	if (!config) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "algorithm", config.algorithm);
	setIfDefined(normalized, "maxKeywords", config.maxKeywords);
	setIfDefined(normalized, "minScore", config.minScore);
	setIfDefined(normalized, "ngramRange", config.ngramRange);
	setIfDefined(normalized, "language", config.language);
	setIfDefined(normalized, "yakeParams", config.yakeParams);
	setIfDefined(normalized, "rakeParams", config.rakeParams);
	return normalized;
}

/**
 * Normalize page extraction configuration.
 *
 * @param pages - Page extraction configuration
 * @returns Normalized config object or undefined
 * @internal
 */
function normalizePageConfig(pages?: PageExtractionConfig): NativeExtractionConfig | undefined {
	if (!pages) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "extractPages", pages.extractPages);
	setIfDefined(normalized, "insertPageMarkers", pages.insertPageMarkers);
	setIfDefined(normalized, "markerFormat", pages.markerFormat);
	return normalized;
}

/**
 * Master orchestrator for normalizing the complete extraction configuration.
 * Calls all specific normalizers and aggregates results into a single config object
 * suitable for passing to the native binding.
 *
 * @param config - Complete extraction configuration
 * @returns Normalized config object or null if input is null
 * @internal
 */
function normalizeExtractionConfig(config: ExtractionConfig | null): NativeExtractionConfig | null {
	if (!config) {
		return null;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "useCache", config.useCache);
	setIfDefined(normalized, "enableQualityProcessing", config.enableQualityProcessing);
	setIfDefined(normalized, "forceOcr", config.forceOcr);
	setIfDefined(normalized, "maxConcurrentExtractions", config.maxConcurrentExtractions);

	const ocr = normalizeOcrConfig(config.ocr);
	setIfDefined(normalized, "ocr", ocr);

	const chunking = normalizeChunkingConfig(config.chunking);
	setIfDefined(normalized, "chunking", chunking);

	const images = normalizeImageExtractionConfig(config.images);
	setIfDefined(normalized, "images", images);

	const pdf = normalizePdfConfig(config.pdfOptions);
	setIfDefined(normalized, "pdfOptions", pdf);

	const tokenReduction = normalizeTokenReductionConfig(config.tokenReduction);
	setIfDefined(normalized, "tokenReduction", tokenReduction);

	const languageDetection = normalizeLanguageDetectionConfig(config.languageDetection);
	setIfDefined(normalized, "languageDetection", languageDetection);

	const postprocessor = normalizePostProcessorConfig(config.postprocessor);
	setIfDefined(normalized, "postprocessor", postprocessor);

	const keywords = normalizeKeywordConfig(config.keywords);
	setIfDefined(normalized, "keywords", keywords);

	const pages = normalizePageConfig(config.pages);
	setIfDefined(normalized, "pages", pages);

	const htmlOptions = normalizeHtmlOptions(config.htmlOptions);
	setIfDefined(normalized, "htmlOptions", htmlOptions);

	return normalized;
}

/**
 * Export public normalization functions for use by extraction modules.
 */
export {
	normalizeExtractionConfig,
	normalizeTesseractConfig,
	normalizeOcrConfig,
	normalizeChunkingConfig,
	normalizeImageExtractionConfig,
	normalizePdfConfig,
	normalizeTokenReductionConfig,
	normalizeLanguageDetectionConfig,
	normalizePostProcessorConfig,
	normalizeHtmlPreprocessing,
	normalizeHtmlOptions,
	normalizeKeywordConfig,
	normalizePageConfig,
	setIfDefined,
};
