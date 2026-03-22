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

/**
 * OCR element hierarchy level.
 *
 * Defines the granularity of OCR element extraction.
 */
export type OcrElementLevel = "word" | "line" | "block" | "page";

/**
 * Configuration for OCR element extraction.
 *
 * Controls how granular OCR elements are extracted and organized.
 */
export interface OcrElementConfig {
	/** Enable extraction of granular OCR elements. Default: false. */
	includeElements?: boolean;

	/** Minimum hierarchy level to extract. Default: 'word'. */
	minLevel?: OcrElementLevel;

	/** Minimum confidence threshold (0.0-1.0) for including elements. Default: 0.0. */
	minConfidence?: number;

	/** Build hierarchical relationships between elements. Default: false. */
	buildHierarchy?: boolean;
}

/**
 * PaddleOCR engine configuration options.
 *
 * Specific configuration for the PaddleOCR backend.
 */
export interface PaddleOcrConfig {
	/** Language code(s) for OCR (e.g., 'en', 'zh', 'multi'). */
	language?: string;

	/** Directory to cache downloaded OCR models. */
	cacheDir?: string;

	/** Enable angle classification for rotated text detection. Default: false. */
	useAngleCls?: boolean;

	/** Enable table structure detection. Default: false. */
	enableTableDetection?: boolean;

	/** Database threshold for text detection (0.0-1.0). Default: 0.3. */
	detDbThresh?: number;

	/** Box threshold for text detection (0.0-1.0). Default: 0.5. */
	detDbBoxThresh?: number;

	/** Unclip ratio for expanding detected text regions. Default: 1.5. */
	detDbUnclipRatio?: number;

	/** Maximum side length for detection preprocessing. Default: 960. */
	detLimitSideLen?: number;

	/** Batch size for text recognition. Default: 6. */
	recBatchNum?: number;

	/** Padding in pixels added around image before detection (0-100). Default: 10. */
	padding?: number;

	/** Model tier: "server" (default, high accuracy) or "mobile" (lightweight, faster). */
	modelTier?: string;
}

export interface OcrConfig {
	backend: string;
	language?: string;
	tesseractConfig?: TesseractConfig;
	paddleOcrConfig?: PaddleOcrConfig;
	elementConfig?: OcrElementConfig;
}

export interface EmbeddingModelType {
	/** Type of model: "preset", "fastembed", or "custom" */
	modelType: string;
	/** For preset: preset name; for fastembed/custom: model ID */
	value: string;
	/** Number of dimensions (only for fastembed/custom) */
	dimensions?: number;
}

export interface EmbeddingConfig {
	/** Embedding model configuration */
	model?: EmbeddingModelType;
	/** Whether to normalize embeddings (L2 normalization) */
	normalize?: boolean;
	/** Batch size for embedding generation */
	batchSize?: number;
	/** Whether to show download progress for models */
	showDownloadProgress?: boolean;
	/** Custom cache directory for model storage */
	cacheDir?: string;
}

export interface ChunkingConfig {
	maxChars?: number;
	maxOverlap?: number;
	preset?: string;
	embedding?: EmbeddingConfig;
	/** Chunker type: "text" (default) or "markdown" */
	chunkerType?: string;
	/** Sizing type: "characters" (default) or "tokenizer" */
	sizingType?: "characters" | "tokenizer";
	/** HuggingFace model ID for tokenizer sizing (e.g., "Xenova/gpt-4o") */
	sizingModel?: string;
	/** Optional cache directory for tokenizer files */
	sizingCacheDir?: string;
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

export interface HierarchyConfig {
	/** Enable hierarchical document structure extraction */
	enabled?: boolean;
	/** Number of clusters for hierarchy (default: 6) */
	kClusters?: number;
	/** Include bounding box information */
	includeBbox?: boolean;
	/** OCR coverage threshold for hierarchy (0.0-1.0) */
	ocrCoverageThreshold?: number;
}

export interface PdfConfig {
	extractImages?: boolean;
	passwords?: string[];
	extractMetadata?: boolean;
	hierarchy?: HierarchyConfig;
	extractAnnotations?: boolean;
	topMarginFraction?: number;
	bottomMarginFraction?: number;
	/** Allow single-column pseudo tables in extraction results. Default: false */
	allowSingleColumnTables?: boolean;
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

/**
 * Keyword extraction algorithm type.
 *
 * Supported algorithms:
 * - "yake": YAKE (Yet Another Keyword Extractor) - statistical approach
 * - "rake": RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based
 */
export type KeywordAlgorithm = "yake" | "rake";

/**
 * YAKE algorithm-specific parameters.
 */
export interface YakeParams {
	/** Window size for co-occurrence analysis (default: 2) */
	windowSize?: number;
}

/**
 * RAKE algorithm-specific parameters.
 */
export interface RakeParams {
	/** Minimum word length to consider (default: 1) */
	minWordLength?: number;

	/** Maximum words in a keyword phrase (default: 3) */
	maxWordsPerPhrase?: number;
}

/**
 * Keyword extraction configuration.
 *
 * Controls how keywords are extracted from text, including algorithm selection,
 * scoring thresholds, n-gram ranges, and language-specific settings.
 */
export interface KeywordConfig {
	/** Algorithm to use for extraction (default: "yake") */
	algorithm?: KeywordAlgorithm;

	/** Maximum number of keywords to extract (default: 10) */
	maxKeywords?: number;

	/** Minimum score threshold 0.0-1.0 (default: 0.0) */
	minScore?: number;

	/** N-gram range [min, max] for keyword extraction (default: [1, 3]) */
	ngramRange?: [number, number];

	/** Language code for stopword filtering (e.g., "en", "de", "fr") */
	language?: string;

	/** YAKE-specific tuning parameters */
	yakeParams?: YakeParams;

	/** RAKE-specific tuning parameters */
	rakeParams?: RakeParams;
}

/**
 * Extracted keyword with relevance metadata.
 *
 * Represents a single keyword extracted from text along with its relevance score,
 * the algorithm that extracted it, and optional position information.
 */
export interface ExtractedKeyword {
	/** The keyword text */
	text: string;

	/** Relevance score (higher is better, algorithm-specific range) */
	score: number;

	/** Algorithm that extracted this keyword */
	algorithm: KeywordAlgorithm;

	/** Optional positions where keyword appears in text (character offsets) */
	positions?: number[];
}

/**
 * Page extraction and tracking configuration.
 *
 * Controls whether Kreuzberg tracks page boundaries and optionally inserts page markers
 * into the extracted content.
 *
 * @example
 * ```typescript
 * // Basic page tracking
 * const config: PageConfig = {
 *   extractPages: true,
 *   insertPageMarkers: false
 * };
 *
 * // With custom page marker format
 * const config: PageConfig = {
 *   extractPages: true,
 *   insertPageMarkers: true,
 *   markerFormat: '\\n--- Page {page_num} ---\\n'
 * };
 * ```
 */
export interface PageConfig {
	/**
	 * Enable page tracking and per-page extraction.
	 * Default: false
	 */
	extractPages?: boolean;

	/**
	 * Insert page markers into the main content string.
	 * Default: false
	 */
	insertPageMarkers?: boolean;

	/**
	 * Template for page markers containing {page_num} placeholder.
	 * Default: "\n\n<!-- PAGE {page_num} -->\n\n"
	 */
	markerFormat?: string;
}

/**
 * Execution provider type for model acceleration.
 *
 * Supported providers:
 * - "auto": Automatically select best available provider (default)
 * - "cpu": Use CPU for inference
 * - "coreml": Use CoreML (Apple) acceleration
 * - "cuda": Use NVIDIA CUDA acceleration
 * - "tensorrt": Use NVIDIA TensorRT acceleration
 */
export type ExecutionProviderType = "auto" | "cpu" | "coreml" | "cuda" | "tensorrt";

/**
 * Model acceleration configuration.
 *
 * Controls hardware acceleration settings for layout inference and other model operations.
 */
export interface AccelerationConfig {
	/** Execution provider type. Default: 'auto' */
	provider?: ExecutionProviderType;

	/** GPU device ID (for CUDA/TensorRT). Default: 0 */
	deviceId?: number;
}

/**
 * Email extraction configuration.
 *
 * Controls behavior of MSG file extraction, specifically the fallback codepage
 * used when an MSG file contains no codepage property.
 */
export interface EmailConfig {
	/** Windows codepage number to use when an MSG file contains no codepage property.
	 * Defaults to undefined (falls back to windows-1252).
	 * Common values: 1250 (Central European), 1251 (Cyrillic), 1253 (Greek), 932 (Japanese). */
	msgFallbackCodepage?: number;
}

/**
 * Concurrency configuration for controlling thread usage.
 *
 * Caps all internal thread pools (Rayon, ONNX Runtime intra-op) and batch
 * concurrency to a single limit.
 */
export interface ConcurrencyConfig {
	/** Maximum number of threads for all internal thread pools. undefined = system defaults. */
	maxThreads?: number;
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
	pages?: PageConfig;
	acceleration?: AccelerationConfig;
	email?: EmailConfig;
	securityLimits?: Record<string, number>;
	maxConcurrentExtractions?: number;
	/** Concurrency configuration for thread pool control */
	concurrency?: ConcurrencyConfig;
	/**
	 * Content text format (default: Plain).
	 * Controls the format of the extracted content:
	 * - "plain": Raw extracted text (default)
	 * - "markdown": Markdown formatted output
	 * - "djot": Djot markup format
	 * - "html": HTML formatted output
	 */
	outputFormat?: "plain" | "markdown" | "djot" | "html";
	/**
	 * Result structure format (default: Unified).
	 * Controls whether results are returned in unified format with all
	 * content in the content field, or element-based format with semantic
	 * elements (for Unstructured-compatible output).
	 *
	 * - "unified": All content in the content field with metadata at result level (default)
	 * - "element_based": Semantic elements (headings, paragraphs, tables, etc.)
	 */
	resultFormat?: "unified" | "element_based";
	/**
	 * Include hierarchical document structure in extraction result.
	 * Default: false
	 *
	 * When enabled, the result will include a DocumentStructure with a flat array
	 * of nodes representing the document tree structure with semantic content types.
	 */
	includeDocumentStructure?: boolean;
	/** Cache namespace for tenant isolation. Alphanumeric, hyphens, underscores only. */
	cacheNamespace?: string;
	/** Per-request cache TTL in seconds. 0 = skip cache entirely. */
	cacheTtlSecs?: number;
}
