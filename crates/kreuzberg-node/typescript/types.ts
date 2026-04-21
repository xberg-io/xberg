/**
 * Type definitions for Kreuzberg extraction results.
 *
 * These types mirror the strongly-typed Rust metadata structures,
 * providing type safety for TypeScript users.
 */

// ============================================================================
// ============================================================================

/**
 * Tesseract OCR engine configuration options.
 *
 * @example
 * ```typescript
 * const config: TesseractConfig = {
 *   psm: 6,
 *   enableTableDetection: true,
 *   tesseditCharWhitelist: '0123456789'
 * };
 * ```
 */
export interface TesseractConfig {
	/**
	 * Page Segmentation Mode (0-13). Controls how Tesseract segments and recognizes text.
	 * Common values: 3 (auto), 6 (single uniform block), 11 (sparse text).
	 * Default: 3 (auto layout analysis).
	 */
	psm?: number;

	/**
	 * Enable table detection during OCR processing.
	 * When true, Tesseract attempts to preserve table structure in the output.
	 * Default: false.
	 */
	enableTableDetection?: boolean;

	/**
	 * Whitelist of characters Tesseract should recognize.
	 * Only these characters will be returned by the OCR engine.
	 * Use empty string to allow all characters. Useful for constraining output to digits,
	 * specific alphabets, or other character sets.
	 * Default: null (recognize all).
	 */
	tesseditCharWhitelist?: string;
}

/**
 * OCR element hierarchy level.
 *
 * Defines the granularity of OCR element extraction.
 */
export type OcrElementLevel = "word" | "line" | "block" | "page";

/**
 * Bounding geometry for OCR elements using rectangle coordinates.
 *
 * Represents rectangular coordinates with position and dimensions.
 */
export interface OcrBoundingGeometryRectangle {
	type: "rectangle";
	left: number;
	top: number;
	width: number;
	height: number;
}

/**
 * Bounding geometry for OCR elements using quadrilateral points.
 *
 * Represents irregular quadrilateral shapes with four corner points.
 */
export interface OcrBoundingGeometryQuadrilateral {
	type: "quadrilateral";
	points: number[][];
}

/**
 * Bounding geometry for OCR elements.
 *
 * Can be either rectangular or quadrilateral based on the OCR engine's detection capability.
 */
export type OcrBoundingGeometry = OcrBoundingGeometryRectangle | OcrBoundingGeometryQuadrilateral;

/**
 * Confidence scores for OCR operations.
 *
 * Tracks confidence levels for different aspects of OCR processing.
 */
export interface OcrConfidence {
	/** Confidence score (0.0-1.0) for text detection. */
	detection?: number;

	/** Confidence score (0.0-1.0) for text recognition. */
	recognition?: number;
}

/**
 * Rotation information for OCR elements.
 *
 * Tracks detected text rotation and associated confidence.
 */
export interface OcrRotation {
	/** Angle of rotation in degrees. */
	angleDegrees?: number;

	/** Confidence score (0.0-1.0) for rotation detection. */
	confidence?: number;
}

/**
 * Individual OCR element (word, line, block, or page).
 *
 * Represents a granular unit of text extracted by OCR with geometric and confidence information.
 */
export interface OcrElement {
	/** Extracted text content */
	text: string;

	/** Bounding geometry of the element in the image */
	geometry?: OcrBoundingGeometry;

	/** Confidence scores for detection and recognition */
	confidence?: OcrConfidence;

	/** Hierarchy level of this element */
	level?: OcrElementLevel;

	/** Rotation information if text is rotated */
	rotation?: OcrRotation;

	/** Page number where this element was found (1-indexed) */
	pageNumber?: number;

	/** Parent element ID for hierarchical relationships */
	parentId?: string;

	/** Backend-specific metadata that doesn't fit standard fields */
	backendMetadata?: Record<string, unknown>;
}

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

	/** Batch size for text recognition. Default: 30. */
	recBatchNum?: number;
}

/**
 * OCR (Optical Character Recognition) configuration.
 *
 * Controls which OCR engine to use and how it processes images.
 */
export interface OcrConfig {
	/** Whether OCR is enabled. Setting to false disables OCR entirely (equivalent to disable_ocr: true on ExtractionConfig). Default: true. */
	enabled?: boolean;

	/** OCR backend name (e.g., 'tesseract', 'paddleocr', 'easyocr'). Required. */
	backend: string;

	/** ISO 639-1/3 language code(s) for OCR (e.g., 'eng', 'fra', 'deu'). Default: 'eng'. */
	language?: string;

	/** Tesseract engine-specific configuration options. Only used when backend is 'tesseract'. */
	tesseractConfig?: TesseractConfig;

	/** PaddleOCR engine-specific configuration options. Only used when backend is 'paddleocr'. */
	paddleOcrConfig?: PaddleOcrConfig;

	/** OCR element extraction configuration. */
	elementConfig?: OcrElementConfig;
}

/**
 * Document chunking configuration for splitting large documents.
 *
 * Breaks large documents into smaller, manageable chunks while preserving context.
 * Useful for RAG (Retrieval Augmented Generation) and vector database indexing.
 */

/**
 * Embedding model type selector.
 *
 * Use `modelType: "preset"` with a named preset (recommended), or `modelType: "custom"` with
 * a HuggingFace model ID and the expected vector dimensions.
 */
export interface EmbeddingModelType {
	/** Selector: `"preset"`, `"custom"`, or `"llm"` */
	modelType: "preset" | "custom" | "llm";
	/** For preset: preset name (e.g., "fast", "balanced", "quality", "multilingual"); for custom: HuggingFace model ID; for llm: model identifier */
	value: string;
	/** Expected embedding dimensions — required for `modelType: "custom"` */
	dimensions?: number;
}

/**
 * Configuration for the standalone embedding API and for chunk-level embeddings.
 */
export interface EmbeddingConfig {
	/** Embedding model selection (defaults to the "balanced" preset). */
	model?: EmbeddingModelType;
	/** Normalize embedding vectors to unit length (recommended for cosine similarity). Default: true. */
	normalize?: boolean;
	/** Number of texts processed per inference batch. Default: 32. */
	batchSize?: number;
	/** Show download progress when the model is downloaded for the first time. Default: false. */
	showDownloadProgress?: boolean;
	/** Custom directory for caching downloaded model files. */
	cacheDir?: string;
}

export interface ChunkingConfig {
	/** Maximum characters per chunk. Default: 4096. */
	maxChars?: number;

	/** Maximum overlapping characters between consecutive chunks for context preservation. Default: 512. */
	maxOverlap?: number;

	/**
	 * Alternative to maxChars: chunk size using different unit.
	 * Mutually exclusive with maxChars.
	 */
	chunkSize?: number;

	/**
	 * Alternative to maxOverlap: overlap amount using different unit.
	 * Mutually exclusive with maxOverlap.
	 */
	chunkOverlap?: number;

	/**
	 * Named preset configuration (e.g., 'default', 'aggressive', 'minimal').
	 * Uses preset values if neither maxChars nor chunkSize is specified.
	 */
	preset?: string;

	/** Embedding configuration for generating vector embeddings for each chunk. */
	embedding?: EmbeddingConfig;

	/** Enable or disable chunking. Default: true when chunking config is provided. */
	enabled?: boolean;

	/** Sizing type: "characters" (default) or "tokenizer". */
	sizingType?: "characters" | "tokenizer";

	/** HuggingFace model ID for tokenizer sizing (e.g., "Xenova/gpt-4o"). */
	sizingModel?: string;

	/** Optional cache directory for tokenizer files. */
	sizingCacheDir?: string;
}

/**
 * Language detection configuration.
 *
 * Automatically detects the language(s) of extracted content.
 */
export interface LanguageDetectionConfig {
	/** Enable automatic language detection. Default: true. */
	enabled?: boolean;

	/** Minimum confidence score (0.0-1.0) for language detection. Default: 0.5. */
	minConfidence?: number;

	/** Detect multiple languages in the same document. Default: false. */
	detectMultiple?: boolean;
}

/**
 * Token reduction configuration for optimizing token usage.
 *
 * Reduces the number of tokens in extracted content while preserving meaning.
 * Useful for reducing costs in LLM pipelines.
 */
export interface TokenReductionConfig {
	/** Reduction mode: 'aggressive' or 'conservative'. Default: 'conservative'. */
	mode?: string;

	/** Preserve tokens for semantically important words even in aggressive mode. Default: true. */
	preserveImportantWords?: boolean;
}

/**
 * Hierarchy extraction configuration.
 *
 * Controls document hierarchy detection based on font size clustering.
 */
export interface HierarchyConfig {
	/** Enable hierarchy extraction. Default: true. */
	enabled?: boolean;

	/** Number of font size clusters (2-10). Default: 6. */
	kClusters?: number;

	/** Include bounding box information. Default: true. */
	includeBbox?: boolean;

	/** OCR coverage threshold (0.0-1.0). Default: null. */
	ocrCoverageThreshold?: number | null;
}

/**
 * PDF-specific extraction configuration.
 *
 * Controls how PDF documents are processed.
 */
export interface PdfConfig {
	/** Extract images from PDF pages. Default: true. */
	extractImages?: boolean;

	/** List of passwords to try for password-protected PDFs. */
	passwords?: string[];

	/** Extract document metadata (title, author, creation date, etc.). Default: true. */
	extractMetadata?: boolean;

	/** Hierarchy extraction configuration. */
	hierarchy?: HierarchyConfig;

	/** Extract annotations from PDF pages. Default: false. */
	extractAnnotations?: boolean;

	/** Top margin fraction (0.0-0.5) for filtering header content. */
	topMarginFraction?: number;

	/** Bottom margin fraction (0.0-0.5) for filtering footer content. */
	bottomMarginFraction?: number;
}

/**
 * Image extraction and processing configuration.
 *
 * Controls how images are extracted and optimized from documents.
 */
export interface ImageExtractionConfig {
	/** Enable image extraction from documents. Default: true. */
	extractImages?: boolean;

	/** Target DPI (dots per inch) for extracted images. Higher DPI = better quality but larger files. Default: 150. */
	targetDpi?: number;

	/** Maximum image dimension (width or height) in pixels. Images larger than this are downscaled. Default: 2000. */
	maxImageDimension?: number;

	/** Automatically adjust DPI based on image content and quality. Default: true. */
	autoAdjustDpi?: boolean;

	/** Minimum DPI to maintain for image quality. Default: 72. */
	minDpi?: number;

	/** Maximum DPI to avoid excessive file sizes. Default: 300. */
	maxDpi?: number;
}

/**
 * Post-processor configuration for modifying extracted content.
 *
 * Post-processors allow customization and cleanup of extraction results
 * without failing the extraction if they encounter errors.
 */
export interface PostProcessorConfig {
	/** Enable or disable post-processing entirely. Default: true. */
	enabled?: boolean;

	/** List of processor names to enable (allowlist). When set, only these are used. */
	enabledProcessors?: string[];

	/** List of processor names to disable (denylist). These are skipped. */
	disabledProcessors?: string[];
}

/**
 * HTML preprocessing options.
 *
 * Cleans HTML content before conversion to Markdown.
 */
export interface HtmlPreprocessingOptions {
	/** Enable HTML preprocessing. Default: true. */
	enabled?: boolean;

	/** Preset cleanup level: 'minimal' (light), 'standard' (balanced), 'aggressive' (heavy). Default: 'standard'. */
	preset?: "minimal" | "standard" | "aggressive";

	/** Remove navigation menus and headers. Default: true. */
	removeNavigation?: boolean;

	/** Remove form elements. Default: true. */
	removeForms?: boolean;
}

/**
 * HTML to Markdown conversion configuration options.
 *
 * Controls how HTML content is converted to Markdown format, including formatting,
 * escaping, and special handling for various HTML elements.
 */
export interface HtmlConversionOptions {
	/** Heading style conversion: "atx" (# style), "underlined" (underline style), or "atx_closed" (# style closed). Default: "atx". */
	headingStyle?: "atx" | "underlined" | "atx_closed";

	/** List indentation type: "spaces" or "tabs". Default: "spaces". */
	listIndentType?: "spaces" | "tabs";

	/** Number of spaces/tabs per list indent level. Default: 4. */
	listIndentWidth?: number;

	/** Bullet characters for unordered lists (e.g., '*', '-', '+'). Default: '*'. */
	bullets?: string;

	/** Markdown symbol for strong/bold emphasis: '**' or '__'. Default: '**'. */
	strongEmSymbol?: string;

	/** Escape asterisks (*) in text to prevent accidental formatting. Default: false. */
	escapeAsterisks?: boolean;

	/** Escape underscores (_) in text to prevent accidental formatting. Default: false. */
	escapeUnderscores?: boolean;

	/** Escape miscellaneous special characters. Default: false. */
	escapeMisc?: boolean;

	/** Escape ASCII control characters. Default: false. */
	escapeAscii?: boolean;

	/** Default code language for syntax highlighting in code blocks (e.g., 'javascript'). Default: null. */
	codeLanguage?: string;

	/** Convert HTML links to Markdown autolinks format ([text](url)). Default: true. */
	autolinks?: boolean;

	/** Use the HTML title element as default for links when no text is available. Default: false. */
	defaultTitle?: boolean;

	/** Insert <br> tags in Markdown tables. Default: false. */
	brInTables?: boolean;

	/** Use HOCR spatial table format for better table structure preservation. Default: false. */
	hocrSpatialTables?: boolean;

	/** Highlight style for marked/highlighted text: "double_equal" (==text==), "html" (<mark>), "bold" (**text**), or "none". Default: "none". */
	highlightStyle?: "double_equal" | "html" | "bold" | "none";

	/** Extract metadata from HTML (title, meta tags, etc.). Default: false. */
	extractMetadata?: boolean;

	/** Whitespace handling: "normalized" (collapse whitespace) or "strict" (preserve all whitespace). Default: "normalized". */
	whitespaceMode?: "normalized" | "strict";

	/** Remove newlines from output (convert to single line). Default: false. */
	stripNewlines?: boolean;

	/** Enable line wrapping at specified width. Default: true. */
	wrap?: boolean;

	/** Maximum line width when wrapping is enabled. Default: 80. */
	wrapWidth?: number;

	/** Convert as inline Markdown instead of block elements. Default: false. */
	convertAsInline?: boolean;

	/** Markdown symbol for subscript text (e.g., '~' for ~text~). Default: '~'. */
	subSymbol?: string;

	/** Markdown symbol for superscript text (e.g., '^' for ^text^). Default: '^'. */
	supSymbol?: string;

	/** Newline style in output: "spaces" (two spaces + newline) or "backslash" (backslash + newline). Default: "spaces". */
	newlineStyle?: "spaces" | "backslash";

	/** Code block style: "indented" (4-space indent), "backticks" (```), or "tildes" (~~~). Default: "backticks". */
	codeBlockStyle?: "indented" | "backticks" | "tildes";

	/** List of HTML tag names to keep as inline images (don't convert). Default: []. */
	keepInlineImagesIn?: string[];

	/** Character encoding for output (e.g., 'utf-8', 'ascii'). Default: 'utf-8'. */
	encoding?: string;

	/** Enable debug mode for detailed conversion logging. Default: false. */
	debug?: boolean;

	/** List of HTML tag names to remove entirely from output. Default: []. */
	stripTags?: string[];

	/** List of HTML tag names to preserve in output (don't convert to Markdown). Default: []. */
	preserveTags?: string[];

	/** HTML preprocessing options for cleaning HTML before conversion. */
	preprocessing?: HtmlPreprocessingOptions;
}

/** Keyword extraction algorithm type. */
export type KeywordAlgorithm = "yake" | "rake";

/**
 * YAKE (Yet Another Keyword Extractor) algorithm configuration.
 *
 * YAKE is an unsupervised keyword extraction method that doesn't require training data.
 */
export interface YakeParams {
	/** Window size for co-occurrence analysis (number of words to consider). Default: 3. */
	windowSize?: number;
}

/**
 * RAKE (Rapid Automatic Keyword Extraction) algorithm configuration.
 *
 * RAKE extracts keywords based on word co-occurrence and statistical measures.
 */
export interface RakeParams {
	/** Minimum word length to consider as keyword. Default: 3. */
	minWordLength?: number;

	/** Maximum number of words per keyword phrase. Default: 3. */
	maxWordsPerPhrase?: number;
}

/**
 * Keyword extraction configuration.
 *
 * Extracts important keywords/phrases from document content using YAKE or RAKE algorithms.
 */
export interface KeywordConfig {
	/** Extraction algorithm: "yake" or "rake". Default: "yake". */
	algorithm?: KeywordAlgorithm;

	/** Maximum number of keywords to extract. Default: 10. */
	maxKeywords?: number;

	/** Minimum relevance score (0.0-1.0) for keywords. Keywords below this are filtered out. Default: 0.1. */
	minScore?: number;

	/** N-gram range: [min_length, max_length] for phrase keywords (e.g., [1, 3] for 1-3 word phrases). Default: [1, 3]. */
	ngramRange?: [number, number];

	/** Language for keyword extraction (e.g., 'en', 'de', 'fr'). Default: 'en'. */
	language?: string;

	/** YAKE algorithm-specific parameters. Only used when algorithm is "yake". */
	yakeParams?: YakeParams;

	/** RAKE algorithm-specific parameters. Only used when algorithm is "rake". */
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
 * Warning from a post-processor during extraction.
 */
export interface ProcessingWarning {
	/** Name of the post-processor that produced the warning */
	source: string;

	/** Warning message */
	message: string;
}

/**
 * LLM usage statistics for an extraction operation.
 *
 * Tracks token usage and cost information when LLM-based processing is applied.
 */
export interface LlmUsage {
	/** LLM model identifier (e.g., 'gpt-4', 'claude-3-opus') */
	model: string;

	/** Source of this LLM usage (e.g., 'extraction', 'post-processing') */
	source: string;

	/** Number of input tokens consumed (if available) */
	inputTokens?: number | null;

	/** Number of output tokens generated (if available) */
	outputTokens?: number | null;

	/** Total tokens used (sum of input + output tokens, if available) */
	totalTokens?: number | null;

	/** Estimated cost in USD (if available) */
	estimatedCost?: number | null;

	/** Finish reason or status (e.g., 'stop', 'max_tokens', 'tool_use') */
	finishReason?: string | null;
}

/**
 * Page tracking and extraction configuration.
 *
 * Controls how pages/slides/sheets are extracted and tracked in the document.
 * Page range information in chunk metadata (first_page/last_page) is automatically
 * enabled when page boundaries are available and chunking is configured.
 */
export interface PageExtractionConfig {
	/** Extract pages as separate array (ExtractionResult.pages) */
	extractPages?: boolean;
	/** Insert page markers in main content string */
	insertPageMarkers?: boolean;
	/** Page marker format (use {page_num} placeholder) */
	markerFormat?: string;
}

/**
 * Layout detection configuration for PDF extraction.
 *
 * Controls layout detection using ONNX-based document layout models (YOLO or RT-DETR)
 * to detect document structure elements like tables, figures, headers, and code blocks.
 * Requires the `layout-detection` feature to be compiled.
 */
export interface LayoutDetectionConfig {
	/** Model preset: "fast" (YOLO, 11 classes) or "accurate" (RT-DETR, 17 classes). Default: "fast". */
	preset?: string;

	/** Override the model's default confidence threshold for detections. Default: null (use model default). */
	confidenceThreshold?: number;

	/** Apply postprocessing heuristics to improve detection quality. Default: true. */
	applyHeuristics?: boolean;

	/**
	 * Table structure recognition model. Controls which model is used for table cell detection.
	 * Options: "tatr" (default), "slanet_wired", "slanet_wireless", "slanet_plus", "slanet_auto".
	 */
	tableModel?: string;
}

/**
 * HTML output configuration for styled HTML rendering.
 *
 * Controls how `outputFormat: "html"` renders documents when styled output
 * is desired. Supports built-in themes and custom CSS injection.
 */
export interface HtmlOutputConfig {
	/** Inline CSS string injected after the theme stylesheet. */
	css?: string;

	/** Path to a CSS file loaded at renderer construction time. */
	cssFile?: string;

	/**
	 * Built-in colour/typography theme.
	 * Default: "unstyled".
	 */
	theme?: "default" | "github" | "dark" | "light" | "unstyled";

	/**
	 * CSS class prefix applied to every emitted class name.
	 * Default: "kb-".
	 */
	classPrefix?: string;

	/**
	 * When true, embed resolved CSS in a `<style>` block in the output.
	 * Set to false to emit only structural markup.
	 * Default: true.
	 */
	embedCss?: boolean;
}

/**
 * Main extraction configuration interface.
 *
 * Combines all sub-configurations for document extraction, OCR, chunking, post-processing, etc.
 * All fields are optional and use sensible defaults.
 */
export interface ExtractionConfig {
	/** Enable caching of extraction results for identical inputs. Default: true. */
	useCache?: boolean;

	/** Enable quality processing filters to improve extraction reliability. Default: false. */
	enableQualityProcessing?: boolean;

	/** OCR configuration for text extraction from images. Only used when document contains images or forceOcr is true. */
	ocr?: OcrConfig;

	/** Force OCR processing even for documents with selectable text. Useful for scanned documents. Default: false. */
	forceOcr?: boolean;

	/** Include structured document tree in the extraction result. Default: false. */
	includeDocumentStructure?: boolean;

	/** Chunking configuration for splitting documents into smaller pieces for RAG or vector DB. */
	chunking?: ChunkingConfig;

	/** Image extraction and optimization configuration. */
	images?: ImageExtractionConfig;

	/** PDF-specific extraction options (passwords, metadata, etc.). */
	pdfOptions?: PdfConfig;

	/** Token reduction configuration for optimizing token usage in LLM pipelines. */
	tokenReduction?: TokenReductionConfig;

	/** Language detection configuration for automatic language identification. */
	languageDetection?: LanguageDetectionConfig;

	/** Post-processor configuration for customizing extraction results. */
	postprocessor?: PostProcessorConfig;

	/** HTML to Markdown conversion options for HTML content. */
	htmlOptions?: HtmlConversionOptions;

	/** Keyword extraction configuration for extracting important phrases. */
	keywords?: KeywordConfig;

	/** Page tracking and extraction configuration for multi-page documents. */
	pages?: PageExtractionConfig;

	/** Maximum number of concurrent extractions in batch operations. Default: 4. */
	maxConcurrentExtractions?: number;

	/**
	 * Output text format for extracted content. Default: "plain".
	 *
	 * - "plain": Raw extracted text
	 * - "markdown": Markdown formatted output
	 * - "djot": Djot markup format
	 * - "html": HTML formatted output
	 * - "json": JSON tree format with heading-driven sections
	 * - "structured": Structured JSON format with full OCR element metadata
	 */
	outputFormat?: "plain" | "markdown" | "djot" | "html" | "json" | "structured";

	/**
	 * Result structure format. Default: "unified".
	 *
	 * - "unified": All content in the `content` field (default)
	 * - "element_based": Semantic element extraction (Unstructured-compatible)
	 */
	resultFormat?: "unified" | "element_based";

	/** Layout detection configuration for detecting document structure in PDFs. */
	layout?: LayoutDetectionConfig;

	/**
	 * HTML output configuration for styled HTML rendering.
	 *
	 * When set alongside `outputFormat: "html"`, the pipeline uses a styled HTML renderer
	 * with theme support and optional custom CSS instead of the plain renderer.
	 */
	htmlOutput?: HtmlOutputConfig;
}

/**
 * Extracted table data from document.
 *
 * Contains both cell data and Markdown representation for easy display and processing.
 */
export interface Table {
	/** 2D array of cell contents (rows × columns) */
	cells: string[][];

	/** Markdown representation of the table for display or parsing */
	markdown: string;

	/** Page number where this table was found (1-indexed) */
	pageNumber: number;

	/** Bounding box of the table on the page (PDF coordinates). */
	boundingBox?: BoundingBox | null;
}

export interface ExcelMetadata {
	sheetCount?: number;
	sheetNames?: string[];
}

export interface EmailMetadata {
	fromEmail?: string | null;
	fromName?: string | null;
	toEmails?: string[];
	ccEmails?: string[];
	bccEmails?: string[];
	messageId?: string | null;
	attachments?: string[];
}

export interface ArchiveMetadata {
	format?: string;
	fileCount?: number;
	fileList?: string[];
	totalSize?: number;
	compressedSize?: number | null;
}

export interface ImageMetadata {
	width?: number;
	height?: number;
	format?: string;
	exif?: Record<string, string>;
}

export interface XmlMetadata {
	elementCount?: number;
	uniqueElements?: string[];
}

export interface TextMetadata {
	lineCount?: number;
	wordCount?: number;
	characterCount?: number;
	headers?: string[] | null;
	links?: [string, string][] | null;
	codeBlocks?: [string, string][] | null;
}

export interface HeaderMetadata {
	level: number;
	text: string;
	id?: string | null;
	depth: number;
	htmlOffset: number;
}

export interface LinkMetadata {
	href: string;
	text: string;
	title?: string | null;
	linkType: "anchor" | "internal" | "external" | "email" | "phone" | "other";
	rel: string[];
	attributes: Record<string, string>;
}

export interface HtmlImageMetadata {
	src: string;
	alt?: string | null;
	title?: string | null;
	dimensions?: [number, number] | null;
	imageType: "data_uri" | "inline_svg" | "external" | "relative";
	attributes: Record<string, string>;
}

export interface StructuredData {
	dataType: "json_ld" | "microdata" | "rdfa";
	rawJson: string;
	schemaType?: string | null;
}

export interface HtmlMetadata {
	title?: string | null;
	description?: string | null;
	keywords: string[];
	author?: string | null;
	canonicalUrl?: string | null;
	baseHref?: string | null;
	language?: string | null;
	textDirection?: "ltr" | "rtl" | "auto" | null;
	openGraph: Record<string, string>;
	twitterCard: Record<string, string>;
	metaTags: Record<string, string>;
	htmlHeaders: HeaderMetadata[];
	htmlLinks: LinkMetadata[];
	htmlImages: HtmlImageMetadata[];
	structuredData: StructuredData[];
}

export interface PdfMetadata {
	title?: string | null;
	author?: string | null;
	subject?: string | null;
	keywords?: string | null;
	creator?: string | null;
	producer?: string | null;
	creationDate?: string | null;
	modificationDate?: string | null;
	pageCount?: number;
}

export interface PptxMetadata {
	title?: string | null;
	author?: string | null;
	description?: string | null;
	summary?: string | null;
	fonts?: string[];
}

export interface OcrMetadata {
	language?: string;
	psm?: number;
	outputFormat?: string;
	tableCount?: number;
	tableRows?: number | null;
	tableCols?: number | null;
}

export interface ImagePreprocessingMetadata {
	originalDimensions?: [number, number];
	originalDpi?: [number, number];
	targetDpi?: number;
	scaleFactor?: number;
	autoAdjusted?: boolean;
	finalDpi?: number;
	newDimensions?: [number, number] | null;
	resampleMethod?: string;
	dimensionClamped?: boolean;
	calculatedDpi?: number | null;
	skippedResize?: boolean;
	resizeError?: string | null;
}

export interface ErrorMetadata {
	errorType?: string;
	message?: string;
}

/**
 * Page boundary information for chunk metadata.
 *
 * Tracks where a specific page's content starts and ends in the main content string,
 * enabling mapping from byte positions to page numbers. All offsets are guaranteed to be
 * at valid UTF-8 character boundaries.
 */
export interface PageBoundary {
	/** Byte offset where this page starts in the content string (UTF-8 valid boundary, inclusive) */
	byteStart: number;
	/** Byte offset where this page ends in the content string (UTF-8 valid boundary, exclusive) */
	byteEnd: number;
	/** Page number (1-indexed) */
	pageNumber: number;
}

/**
 * Type of paginated unit in a document.
 *
 * Distinguishes between different types of "pages":
 * - "page": Standard document pages (PDF, DOCX, images)
 * - "slide": Presentation slides (PPTX, ODP)
 * - "sheet": Spreadsheet sheets (XLSX, ODS)
 */
export type PageUnitType = "page" | "slide" | "sheet";

/**
 * Detailed per-page metadata.
 *
 * Captures information about a single page/slide/sheet including dimensions,
 * content counts, and visibility state.
 */
export interface PageInfo {
	/** Page number (1-indexed) */
	number: number;
	/** Page title (usually for presentations) */
	title?: string | null;
	/** Dimensions in points (PDF) or pixels (images): [width, height] */
	dimensions?: [number, number] | null;
	/** Number of images on this page */
	imageCount?: number | null;
	/** Number of tables on this page */
	tableCount?: number | null;
	/** Whether this page is hidden (e.g., in presentations) */
	hidden?: boolean | null;
	/** Whether this page is blank (contains no meaningful content) */
	isBlank?: boolean | null;
}

/**
 * Page structure metadata.
 *
 * Contains information about pages/slides/sheets in a document, including
 * boundaries for mapping chunks to pages and detailed per-page metadata.
 */
export interface PageStructure {
	/** Total number of pages/slides/sheets */
	totalCount: number;
	/** Type of paginated unit (page, slide, or sheet) */
	unitType: PageUnitType;
	/** Byte offset boundaries for each page */
	boundaries?: PageBoundary[] | null;
	/** Detailed per-page metadata (optional, only when needed) */
	pages?: PageInfo[] | null;
}

/**
 * Metadata about a chunk's position and properties in the document.
 *
 * Tracks where a chunk appears in the original document, including byte offsets
 * and page ranges when page tracking is enabled.
 */
/** Heading depth and text for markdown heading context. */
export interface HeadingLevel {
	/** Heading depth (1 = h1, 2 = h2, etc.) */
	level: number;
	/** Text content of the heading */
	text: string;
}

/** Heading hierarchy context for a markdown chunk. */
export interface HeadingContext {
	/** Heading hierarchy from document root to this chunk's section */
	headings: HeadingLevel[];
}

export interface ChunkMetadata {
	/** Byte offset where this chunk starts in the original text (UTF-8 valid boundary) */
	byteStart: number;
	/** Byte offset where this chunk ends in the original text (UTF-8 valid boundary) */
	byteEnd: number;
	/** Number of tokens in this chunk (if available from embedding model) */
	tokenCount?: number | null;
	/** Zero-based index of this chunk in the document */
	chunkIndex: number;
	/** Total number of chunks in the document */
	totalChunks: number;
	/** First page number this chunk spans (1-indexed, only when page tracking enabled) */
	firstPage?: number | null;
	/** Last page number this chunk spans (1-indexed, only when page tracking enabled) */
	lastPage?: number | null;
	/** Heading context when using markdown chunker */
	headingContext?: HeadingContext | null;
}

/**
 * Text chunk with optional embedding.
 *
 * Represents a segment of a document created by the chunking algorithm, useful for RAG and vector databases.
 */
export interface Chunk {
	/** Text content of this chunk */
	content: string;

	/** Semantic type classification of this chunk (e.g. 'heading', 'code_block', 'table_like', 'unknown') */
	chunkType?: string | null;

	/** Vector embedding for this chunk (if embedding model was used) */
	embedding?: number[] | null;

	/** Metadata about chunk position and properties in the document */
	metadata: ChunkMetadata;
}

/**
 * Extracted image from document with optional OCR result.
 *
 * Contains image data and metadata about position, dimensions, and properties.
 */
export interface ExtractedImage {
	/** Raw image bytes as Uint8Array */
	data: Uint8Array;

	/** Image format (e.g., 'png', 'jpeg', 'tiff') */
	format: string;

	/** Sequential index of this image in the document (0-indexed) */
	imageIndex: number;

	/** Page number where this image was found (1-indexed), null if unknown */
	pageNumber?: number | null;

	/** Image width in pixels, null if unknown */
	width?: number | null;

	/** Image height in pixels, null if unknown */
	height?: number | null;

	/** Color space (e.g., 'RGB', 'CMYK', 'Grayscale'), null if unknown */
	colorspace?: string | null;

	/** Bits per color component (e.g., 8 for 8-bit), null if unknown */
	bitsPerComponent?: number | null;

	/** Whether this is a mask image (used internally by PDF) */
	isMask: boolean;

	/** Image description or caption if available */
	description?: string | null;

	/** OCR extraction result if OCR was run on this image, null otherwise */
	ocrResult?: ExtractionResult | null;

	/** Bounding box of the image on the page (PDF coordinates). */
	boundingBox?: BoundingBox | null;
}

/**
 * Content for a single page/slide/sheet.
 *
 * When page extraction is enabled, documents are split into per-page content
 * with associated tables and images mapped to each page. This allows for page-specific processing.
 */
export interface PageContent {
	/** Page number (1-indexed) starting from 1 */
	pageNumber: number;

	/** Text content extracted from this page */
	content: string;

	/** Tables found and extracted from this page */
	tables: Table[];

	/** Images found and extracted from this page */
	images: ExtractedImage[];

	/** Whether this page is blank (contains no meaningful content) */
	isBlank?: boolean | null;
}

/**
 * Extraction result metadata.
 *
 * Uses a flattened discriminated union approach with format_type as the discriminator.
 * When format_type is set (e.g., "archive"), the corresponding format-specific fields
 * are available at the root level of the metadata object.
 *
 * This structure matches the Rust serialization with serde's tagged enum flattening.
 */
export interface Metadata {
	language?: string | null;
	createdAt?: string | null;
	modifiedAt?: string | null;
	subject?: string | null;

	formatType?: "pdf" | "excel" | "email" | "pptx" | "archive" | "image" | "xml" | "text" | "html" | "ocr";

	title?: string | null;
	authors?: string[] | null;
	keywords?: string[] | null;
	creator?: string | null;
	producer?: string | null;
	creationDate?: string | null;
	modificationDate?: string | null;
	pageCount?: number;

	sheetCount?: number;
	sheetNames?: string[];

	fromEmail?: string | null;
	fromName?: string | null;
	toEmails?: string[];
	ccEmails?: string[];
	bccEmails?: string[];
	messageId?: string | null;
	attachments?: string[];

	description?: string | null;
	summary?: string | null;
	fonts?: string[];

	format?: string;
	fileCount?: number;
	fileList?: string[];
	totalSize?: number;
	compressedSize?: number | null;

	width?: number;
	height?: number;
	exif?: Record<string, string>;

	elementCount?: number;
	uniqueElements?: string[];

	line_count?: number;
	word_count?: number;
	character_count?: number;
	headers?: string[] | null;
	links?: [string, string][] | null;
	code_blocks?: [string, string][] | null;

	canonical_url?: string | null;
	base_href?: string | null;
	open_graph?: Record<string, string>;
	twitter_card?: Record<string, string>;
	meta_tags?: Record<string, string>;
	html_language?: string | null;
	text_direction?: "ltr" | "rtl" | "auto" | null;
	html_headers?: HeaderMetadata[];
	html_links?: LinkMetadata[];
	html_images?: HtmlImageMetadata[];
	structured_data?: StructuredData[];

	psm?: number;
	output_format?: string;
	table_count?: number;
	table_rows?: number | null;
	table_cols?: number | null;

	image_preprocessing?: ImagePreprocessingMetadata | null;

	json_schema?: Record<string, unknown> | null;

	page_structure?: PageStructure | null;

	error?: ErrorMetadata | null;

	/**
	 * Additional fields may be added at runtime by postprocessors.
	 * Use bracket notation to safely access unexpected properties.
	 */
	[key: string]: unknown;
}

/**
 * Semantic element type classification.
 *
 * Categorizes text content into semantic units for downstream processing.
 * Supports the element types commonly found in structured documents.
 */
export type ElementType =
	| "title"
	| "narrative_text"
	| "heading"
	| "list_item"
	| "table"
	| "image"
	| "page_break"
	| "code_block"
	| "block_quote"
	| "footer"
	| "header";

/**
 * Bounding box coordinates for element positioning.
 *
 * Represents rectangular coordinates in the document space.
 */
export interface BoundingBox {
	/** Left x-coordinate */
	x0: number;
	/** Bottom y-coordinate */
	y0: number;
	/** Right x-coordinate */
	x1: number;
	/** Top y-coordinate */
	y1: number;
}

/**
 * A PDF annotation extracted from a document page.
 */
export interface PdfAnnotation {
	/** Type of annotation (e.g., "text", "highlight", "link", "underline") */
	annotationType: string;
	/** Text content of the annotation, if available */
	content?: string | null;
	/** Page number (1-indexed) where the annotation appears */
	pageNumber: number;
	/** Bounding box of the annotation on the page */
	boundingBox?: BoundingBox | null;
}

/**
 * Metadata for a semantic element.
 *
 * Contains structural and positioning information about an extracted element.
 */
export interface ElementMetadata {
	/** Page number (1-indexed) */
	pageNumber?: number | null;
	/** Source filename or document name */
	filename?: string | null;
	/** Bounding box coordinates if available */
	coordinates?: BoundingBox | null;
	/** Position index in the element sequence */
	elementIndex?: number | null;
	/** Additional custom metadata */
	additional?: Record<string, string>;
}

/**
 * Semantic element extracted from document.
 *
 * Represents a logical unit of content with semantic classification,
 * unique identifier, and metadata for tracking origin and position.
 */
export interface Element {
	/** Unique element identifier */
	elementId: string;
	/** Semantic type of this element */
	elementType: ElementType;
	/** Text content of the element */
	text: string;
	/** Metadata about the element */
	metadata: ElementMetadata;
}

/**
 * Complete extraction result from document processing.
 *
 * Contains all extracted content, metadata, and optional processed data like chunks and images.
 * This is the primary return value from extraction functions.
 */
export interface ExtractionResult {
	/** Extracted text content from the document (main content) */
	content: string;

	/** MIME type of the input document (e.g., 'application/pdf', 'text/html') */
	mimeType: string;

	/** Document metadata including title, author, creation date, language, and format-specific fields */
	metadata: Metadata;

	/** Tables extracted from the document (2D cell arrays with Markdown representation) */
	tables: Table[];

	/** Detected languages in the document (ISO 639-1 codes, e.g., ['en', 'de']), null if detection disabled */
	detectedLanguages: string[] | null;

	/** Document chunks for RAG/vector databases (if chunking was enabled), null otherwise */
	chunks: Chunk[] | null;

	/** Images extracted from document with metadata (if image extraction was enabled), null otherwise */
	images: ExtractedImage[] | null;

	/** Semantic elements extracted from document with type classification and metadata (if element extraction was enabled), null otherwise */
	elements?: Element[] | null;

	/** Per-page content when page extraction is enabled, null otherwise. Each item contains page number, content, tables, and images. */
	pages?: PageContent[] | null;

	/** Extracted keywords when keyword extraction is enabled, undefined otherwise */
	extractedKeywords?: ExtractedKeyword[];

	/** Quality score when quality processing is enabled, undefined otherwise */
	qualityScore?: number;

	/** Processing warnings from post-processors */
	processingWarnings?: ProcessingWarning[];

	/** Granular OCR elements (words, lines, blocks) when OCR element extraction is enabled, null otherwise */
	ocrElements?: OcrElement[] | null;

	/** Structured document tree when include_document_structure is enabled, null otherwise */
	document?: Record<string, unknown> | null;

	/** PDF annotations when extract_annotations is enabled, null otherwise */
	annotations?: PdfAnnotation[] | null;

	/** LLM usage statistics when LLM-based processing is applied, null otherwise */
	llmUsage?: LlmUsage[] | null;
}

/** Post-processor execution stage in the extraction pipeline. */
export type ProcessingStage = "early" | "middle" | "late";

/**
 * Protocol for custom post-processors that modify extraction results.
 *
 * Post-processors enrich or transform extraction results without failing the extraction.
 * If a post-processor throws an error, it's logged but extraction continues.
 * Only works with async extraction functions (`extractFile`, `extractBytes`, etc.).
 */
export interface PostProcessorProtocol {
	/**
	 * Return the unique name of this postprocessor.
	 *
	 * @returns Unique processor name (case-sensitive, alphanumeric + underscores recommended)
	 */
	name(): string;

	/**
	 * Process and enrich an extraction result.
	 *
	 * Modify the result to add new metadata, transform content, or perform other enrichment.
	 * If this throws an error, it's logged but extraction continues.
	 *
	 * @param result - ExtractionResult with extracted content, metadata, and tables
	 * @returns Modified result with enriched data. Can be async or sync.
	 */
	process(result: ExtractionResult): ExtractionResult | Promise<ExtractionResult>;

	/**
	 * Return the processing stage for this processor.
	 *
	 * Determines when this processor runs relative to others:
	 * - "early": Runs first, before other processors (good for cleanup/normalization)
	 * - "middle": Runs with other middle-stage processors (default)
	 * - "late": Runs last, after others (good for final enrichment)
	 *
	 * @returns One of "early", "middle", or "late" (default: "middle")
	 */
	processingStage?(): ProcessingStage;

	/**
	 * Initialize the processor (e.g., load ML models, setup resources).
	 *
	 * Called once when the processor is first registered. Use for expensive operations.
	 */
	initialize?(): void | Promise<void>;

	/**
	 * Shutdown the processor and release resources.
	 *
	 * Called when the processor is unregistered. Use for cleanup (closing connections, freeing memory).
	 */
	shutdown?(): void | Promise<void>;
}

/**
 * Protocol for custom validators that check extraction results.
 *
 * Validators perform quality checks and fail the extraction if validation fails.
 * Unlike post-processors, validator errors cause the entire extraction to fail.
 * Useful for enforcing quality standards on extracted content.
 */
export interface ValidatorProtocol {
	/**
	 * Return the unique name of this validator.
	 *
	 * @returns Unique validator name (case-sensitive, alphanumeric + underscores recommended)
	 */
	name(): string;

	/**
	 * Validate an extraction result.
	 *
	 * Throw an error if validation fails. The error message will be used as the extraction error.
	 * If validation passes, return without throwing (return value is ignored).
	 *
	 * @param result - ExtractionResult to validate
	 * @throws {Error} If validation fails (extraction will fail with this error)
	 */
	validate(result: ExtractionResult): void | Promise<void>;

	/**
	 * Return the validation priority.
	 *
	 * Higher priority validators run first. Useful for running cheap validations (e.g., length checks)
	 * before expensive ones (e.g., AI-based quality checks) to fail fast.
	 *
	 * @returns Priority value (higher = runs earlier, default: 50). Range: 0-1000.
	 */
	priority?(): number;

	/**
	 * Check if this validator should run for a given result.
	 *
	 * Allows conditional validation based on MIME type, metadata, or content.
	 * This is evaluated before validation, so expensive checks can be skipped for irrelevant documents.
	 *
	 * @param result - ExtractionResult to check
	 * @returns true if validator should run, false to skip (default: true)
	 */
	shouldValidate?(result: ExtractionResult): boolean;

	/**
	 * Initialize the validator (e.g., load ML models, setup resources).
	 *
	 * Called once when the validator is first registered. Use for expensive operations.
	 */
	initialize?(): void | Promise<void>;

	/**
	 * Shutdown the validator and release resources.
	 *
	 * Called when the validator is unregistered. Use for cleanup (closing connections, freeing memory).
	 */
	shutdown?(): void | Promise<void>;
}

/**
 * OCR backend protocol for implementing custom OCR engines.
 *
 * This interface defines the contract for OCR backends that can be registered
 * with Kreuzberg's extraction pipeline.
 *
 * ## Implementation Requirements
 *
 * OCR backends must implement:
 * - `name()`: Return a unique backend identifier
 * - `supportedLanguages()`: Return list of supported ISO 639-1/2/3 language codes
 * - `processImage()`: Process image bytes and return extraction result
 *
 * ## Optional Methods
 *
 * - `initialize()`: Called when backend is registered (load models, etc.)
 * - `shutdown()`: Called when backend is unregistered (cleanup resources)
 *
 * @example
 * ```typescript
 * import { registerOcrBackend, extractFile } from '@kreuzberg/node';
 *
 * // PaddleOCR is built into the native Rust core - just use the backend name
 * const result = await extractFile('scanned.pdf', null, {
 *   ocr: { backend: 'paddle-ocr', language: 'en' }
 * });
 * ```
 */
export interface OcrBackendProtocol {
	/**
	 * Return the unique name of this OCR backend.
	 *
	 * This name is used in ExtractionConfig to select the backend:
	 * ```typescript
	 * { ocr: { backend: 'paddle-ocr', language: 'en' } }
	 * ```
	 *
	 * @returns Unique backend identifier (e.g., "paddle-ocr", "tesseract")
	 */
	name(): string;

	/**
	 * Return list of supported language codes.
	 *
	 * Language codes should follow ISO 639-1 (2-letter) or ISO 639-2 (3-letter) standards.
	 * Common codes: "en", "eng" (English), "de", "deu" (German), "fr", "fra" (French).
	 *
	 * @returns Array of supported language codes
	 *
	 * @example
	 * ```typescript
	 * supportedLanguages(): string[] {
	 *   return ["en", "eng", "de", "deu", "fr", "fra"];
	 * }
	 * ```
	 */
	supportedLanguages(): string[];

	/**
	 * Process image bytes and extract text via OCR.
	 *
	 * This method receives raw image data and must return a result object with:
	 * - `content`: Extracted text content
	 * - `mime_type`: MIME type (usually "text/plain")
	 * - `metadata`: Additional information (confidence, dimensions, etc.)
	 * - `tables`: Optional array of detected tables
	 *
	 * @param imageBytes - Raw image data (Uint8Array) or Base64-encoded string (when called from Rust bindings)
	 * @param language - Language code from supportedLanguages()
	 * @returns Promise resolving to extraction result
	 *
	 * @example
	 * ```typescript
	 * async processImage(imageBytes: Uint8Array | string, language: string): Promise<{
	 *   content: string;
	 *   mime_type: string;
	 *   metadata: Record<string, unknown>;
	 *   tables: unknown[];
	 * }> {
	 *   const buffer = typeof imageBytes === "string" ? Buffer.from(imageBytes, "base64") : Buffer.from(imageBytes);
	 *   const text = await myOcrEngine.recognize(buffer, language);
	 *   return {
	 *     content: text,
	 *     mime_type: "text/plain",
	 *     metadata: { confidence: 0.95, language },
	 *     tables: []
	 *   };
	 * }
	 * ```
	 */
	processImage(
		imageBytes: Uint8Array | string,
		language: string,
	): Promise<{
		content: string;
		mime_type: string;
		metadata: Record<string, unknown>;
		tables: unknown[];
	}>;

	/**
	 * Initialize the OCR backend (optional).
	 *
	 * Called once when the backend is registered. Use this to:
	 * - Load ML models
	 * - Initialize libraries
	 * - Validate dependencies
	 *
	 * @example
	 * ```typescript
	 * async initialize(): Promise<void> {
	 *   this.model = await loadModel('./path/to/model');
	 * }
	 * ```
	 */
	initialize?(): void | Promise<void>;

	/**
	 * Shutdown the OCR backend and release resources (optional).
	 *
	 * Called when the backend is unregistered. Use this to:
	 * - Free model memory
	 * - Close file handles
	 * - Cleanup temporary files
	 *
	 * @example
	 * ```typescript
	 * async shutdown(): Promise<void> {
	 *   await this.model.dispose();
	 *   this.model = null;
	 * }
	 * ```
	 */
	shutdown?(): void | Promise<void>;
}

/**
 * Result of error message classification into error codes.
 *
 * Provides classification details including the error code, name,
 * description, and confidence score for the classification.
 *
 * @example
 * ```typescript
 * import { classifyError, ErrorCode } from '@kreuzberg/node';
 *
 * const result = classifyError("File not found in read operation");
 * if (result.code === ErrorCode.IoError) {
 *   console.error(`I/O Error: ${result.description}`);
 *   console.log(`Confidence: ${result.confidence}`);
 * }
 * ```
 */
export interface ErrorClassification {
	/**
	 * The numeric error code (0-7) representing the error type.
	 */
	code: number;

	/**
	 * The human-readable name of the error code (e.g., "validation", "ocr").
	 */
	name: string;

	/**
	 * A brief description of the error type.
	 */
	description: string;

	/**
	 * Confidence score (0.0-1.0) indicating how certain the classification is.
	 * Higher values indicate higher confidence in the classification.
	 */
	confidence: number;
}

// ============================================================================
// Worker Pool APIs
// ============================================================================

/**
 * Opaque handle to a worker pool for concurrent extraction operations.
 *
 * Worker pools enable parallel processing of CPU-bound document extraction
 * tasks by distributing work across multiple threads. This is especially
 * useful for batch processing large numbers of documents.
 *
 * @example
 * ```typescript
 * import { createWorkerPool, extractFileInWorker, closeWorkerPool } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4); // 4 concurrent workers
 * try {
 *   const result = await extractFileInWorker(pool, 'document.pdf');
 *   console.log(result.content);
 * } finally {
 *   await closeWorkerPool(pool);
 * }
 * ```
 */
export interface WorkerPool {
	/** Internal pool identifier (opaque) */
	readonly poolId: number;
}

/**
 * Worker pool statistics.
 *
 * Provides information about the current state of a worker pool including
 * pool size, number of active workers, and queued tasks.
 *
 * @example
 * ```typescript
 * import { createWorkerPool, getWorkerPoolStats } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4);
 * const stats = getWorkerPoolStats(pool);
 * console.log(`Active: ${stats.activeWorkers}/${stats.size}`);
 * console.log(`Queued: ${stats.queuedTasks}`);
 * ```
 */
export interface WorkerPoolStats {
	/**
	 * Maximum number of concurrent workers in the pool.
	 */
	size: number;

	/**
	 * Number of currently active (executing) workers.
	 */
	activeWorkers: number;

	/**
	 * Number of tasks waiting in the queue.
	 */
	queuedTasks: number;
}
