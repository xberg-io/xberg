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
	/** Hardware acceleration configuration for ONNX Runtime inference */
	acceleration?: AccelerationConfig;
}

export interface ChunkingConfig {
	maxChars?: number;
	maxOverlap?: number;
	preset?: string;
	embedding?: EmbeddingConfig;
	/** Chunker type: "text" (default), "markdown", "yaml", or "semantic".
	 * Set to "semantic" for topic-aware chunking that works out of the box
	 * with sensible defaults. No other parameters needed. */
	chunkerType?: "text" | "markdown" | "yaml" | "semantic";
	/** Sizing type: "characters" (default) or "tokenizer" */
	sizingType?: "characters" | "tokenizer";
	/** HuggingFace model ID for tokenizer sizing (e.g., "Xenova/gpt-4o") */
	sizingModel?: string;
	/** Optional cache directory for tokenizer files */
	sizingCacheDir?: string;
	/** Prepend heading context to each chunk when using markdown chunker. Default: false */
	prependHeadingContext?: boolean;
	/** Cosine similarity threshold for semantic topic detection (0.0-1.0).
	 * Optional, defaults to 0.75. Rarely needs tuning. */
	topicThreshold?: number;
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
 * Layout detection configuration.
 *
 * Controls document layout analysis, including semantic zone detection and table structure recognition.
 */
export interface LayoutDetectionConfig {
	/** Model preset: "fast" (YOLO, 11 classes) or "accurate" (RT-DETR, 17 classes). Default: "fast". */
	preset?: string;

	/** Override the model's default confidence threshold for detections. Default: null (use model default). */
	confidenceThreshold?: number;

	/** Apply postprocessing heuristics to improve detection quality. Default: true. */
	applyHeuristics?: boolean;

	/** Table structure recognition model. Controls which model is used for table cell detection.
	 * Options: "tatr" (default), "slanet_wired", "slanet_wireless", "slanet_plus", "slanet_auto". */
	tableModel?: string;

	/** Hardware acceleration configuration for ONNX Runtime inference */
	acceleration?: AccelerationConfig;
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

/**
 * Configuration for tree-sitter code analysis processing options.
 */
export interface TreeSitterProcessConfig {
	/** Extract structural items (functions, classes, etc.). Default: true. */
	structure?: boolean;

	/** Extract import statements. Default: true. */
	imports?: boolean;

	/** Extract export statements. Default: true. */
	exports?: boolean;

	/** Extract comments. Default: false. */
	comments?: boolean;

	/** Extract docstrings. Default: false. */
	docstrings?: boolean;

	/** Extract symbol definitions. Default: false. */
	symbols?: boolean;

	/** Include parse diagnostics. Default: false. */
	diagnostics?: boolean;

	/** Maximum chunk size in bytes. Undefined disables chunking. */
	chunkMaxSize?: number;

	/** Content rendering mode: "chunks" (default), "raw", or "structure". */
	contentMode?: "chunks" | "raw" | "structure";
}

/**
 * Configuration for tree-sitter language pack integration.
 */
export interface TreeSitterConfig {
	/** Enable code intelligence processing. Default: true. */
	enabled?: boolean;

	/** Custom cache directory for downloaded grammars. */
	cacheDir?: string;

	/** Languages to pre-download on init. */
	languages?: string[];

	/** Language groups to pre-download ("web", "systems", "scripting", etc.). */
	groups?: string[];

	/** Processing options for code analysis. */
	process?: TreeSitterProcessConfig;
}

// ============================================================================
// Format-specific metadata interfaces (serialized from Rust via serde)
// ============================================================================

export interface CsvMetadata {
	format_type: "csv";
	row_count: number;
	column_count: number;
	delimiter?: string;
	has_header: boolean;
	column_types?: string[];
}

export interface YearRange {
	min?: number;
	max?: number;
	years: number[];
}

export interface BibtexMetadata {
	format_type: "bibtex";
	entry_count: number;
	citation_keys: string[];
	authors: string[];
	year_range?: YearRange;
	entry_types?: Record<string, number>;
}

export interface CitationMetadata {
	format_type: "citation";
	citation_count: number;
	format?: string;
	authors: string[];
	year_range?: YearRange;
	dois: string[];
	keywords: string[];
}

export interface FictionBookMetadata {
	format_type: "fiction_book";
	genres: string[];
	sequences: string[];
	annotation?: string;
}

export interface DbfFieldInfo {
	name: string;
	field_type: string;
}

export interface DbfMetadata {
	format_type: "dbf";
	record_count: number;
	field_count: number;
	fields: DbfFieldInfo[];
}

export interface ContributorRole {
	name: string;
	role?: string;
}

export interface JatsMetadata {
	format_type: "jats";
	copyright?: string;
	license?: string;
	history_dates: Record<string, string>;
	contributor_roles: ContributorRole[];
}

export interface EpubMetadata {
	format_type: "epub";
	coverage?: string;
	dc_format?: string;
	relation?: string;
	source?: string;
	dc_type?: string;
	cover_image?: string;
}

export interface PstMetadata {
	format_type: "pst";
	message_count: number;
}

// ============================================================================
// Tree-sitter ProcessResult types (serialized from Rust via serde)
// ============================================================================

export interface CodeSpan {
	start_byte: number;
	end_byte: number;
	start_line: number;
	start_column: number;
	end_line: number;
	end_column: number;
}

export interface CodeFileMetrics {
	total_lines: number;
	code_lines: number;
	comment_lines: number;
	blank_lines: number;
	total_bytes: number;
	node_count: number;
	error_count: number;
	max_depth: number;
}

export interface CodeStructureItem {
	kind: string;
	name?: string;
	visibility?: string;
	span: CodeSpan;
	children: CodeStructureItem[];
	decorators: string[];
	doc_comment?: string;
	signature?: string;
	body_span?: CodeSpan;
}

export interface CodeImportInfo {
	source: string;
	items: string[];
	alias?: string;
	is_wildcard: boolean;
	span: CodeSpan;
}

export interface CodeExportInfo {
	name: string;
	kind: string;
	span: CodeSpan;
}

export interface CodeSymbolInfo {
	name: string;
	kind: string;
	type_annotation?: string;
	span: CodeSpan;
}

export interface CodeCommentInfo {
	text: string;
	kind: string;
	span: CodeSpan;
}

export interface CodeDocSection {
	kind: string;
	name?: string;
	content: string;
}

export interface CodeDocstringInfo {
	text: string;
	format: string;
	associated_item?: string;
	span: CodeSpan;
	sections: CodeDocSection[];
}

export interface CodeDiagnostic {
	message: string;
	severity: string;
	span: CodeSpan;
}

export interface CodeChunkContext {
	parent_name?: string;
	parent_kind?: string;
}

export interface CodeChunk {
	content: string;
	language: string;
	span: CodeSpan;
	context?: CodeChunkContext;
}

export interface CodeProcessResult {
	language: string;
	metrics: CodeFileMetrics;
	structure: CodeStructureItem[];
	imports: CodeImportInfo[];
	exports: CodeExportInfo[];
	comments: CodeCommentInfo[];
	docstrings: CodeDocstringInfo[];
	symbols: CodeSymbolInfo[];
	diagnostics: CodeDiagnostic[];
	chunks: CodeChunk[];
}

/**
 * Content filtering configuration.
 *
 * Controls which content elements are included or excluded during extraction,
 * such as headers, footers, watermarks, and repeating text.
 */
export interface ContentFilterConfig {
	/** Include page headers in extracted content. Default: false. */
	includeHeaders?: boolean;

	/** Include page footers in extracted content. Default: false. */
	includeFooters?: boolean;

	/** Strip repeating text (e.g., running headers/footers) from output. Default: true. */
	stripRepeatingText?: boolean;

	/** Include watermark text in extracted content. Default: false. */
	includeWatermarks?: boolean;
}

export interface ExtractionConfig {
	useCache?: boolean;
	enableQualityProcessing?: boolean;
	ocr?: OcrConfig;
	forceOcr?: boolean;
	/** Disable OCR entirely — image files return empty content instead of errors */
	disableOcr?: boolean;
	forceOcrPages?: number[];
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
	layout?: LayoutDetectionConfig;
	email?: EmailConfig;
	securityLimits?: Record<string, number>;
	maxConcurrentExtractions?: number;
	/** Concurrency configuration for thread pool control */
	concurrency?: ConcurrencyConfig;
	/** Tree-sitter language pack and code analysis configuration */
	treeSitter?: TreeSitterConfig;
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
	/** Default per-file extraction timeout in seconds for batch operations. None = no timeout. */
	extractionTimeoutSecs?: number;
	/** Maximum recursion depth for archive extraction (ZIP, TAR, 7Z, GZIP). Default: 3. */
	maxArchiveDepth?: number;
	/** Content filtering configuration for headers, footers, watermarks, and repeating text */
	contentFilter?: ContentFilterConfig;
}
