/**
 * Type definitions for Kreuzberg WASM bindings
 *
 * These types are generated from the Rust core library and define
 * the interface for extraction, configuration, and results.
 */

/**
 * Token reduction configuration
 */
export interface TokenReductionConfig {
	/** Token reduction mode */
	mode?: string;
	/** Preserve important words during reduction */
	preserveImportantWords?: boolean;
}

/**
 * Post-processor configuration
 */
export interface PostProcessorConfig {
	/** Whether post-processing is enabled */
	enabled?: boolean;
	/** List of enabled processors */
	enabledProcessors?: string[];
	/** List of disabled processors */
	disabledProcessors?: string[];
}

/**
 * Keyword extraction algorithm type
 *
 * Supported algorithms:
 * - "yake": YAKE (Yet Another Keyword Extractor) - statistical approach
 * - "rake": RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based
 */
export type KeywordAlgorithm = "yake" | "rake";

/**
 * YAKE algorithm-specific parameters
 */
export interface YakeParams {
	/** Window size for co-occurrence analysis (default: 2) */
	windowSize?: number;
}

/**
 * RAKE algorithm-specific parameters
 */
export interface RakeParams {
	/** Minimum word length to consider (default: 1) */
	minWordLength?: number;
	/** Maximum words in a keyword phrase (default: 3) */
	maxWordsPerPhrase?: number;
}

/**
 * Keyword extraction configuration
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
 * Extracted keyword with relevance metadata
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
 * HTML preprocessing options
 */
export interface HtmlPreprocessingOptions {
	/** Whether preprocessing is enabled */
	enabled?: boolean;
	/** Preset configuration */
	preset?: "minimal" | "standard" | "aggressive";
	/** Remove navigation elements */
	removeNavigation?: boolean;
	/** Remove form elements */
	removeForms?: boolean;
}

/**
 * HTML conversion options for HTML documents
 */
export interface HtmlConversionOptions {
	/** Heading style for markdown output */
	headingStyle?: "atx" | "underlined" | "atx_closed";
	/** List indentation type */
	listIndentType?: "spaces" | "tabs";
	/** List indentation width */
	listIndentWidth?: number;
	/** Bullet character for lists */
	bullets?: string;
	/** Symbol for strong/emphasis */
	strongEmSymbol?: string;
	/** Escape asterisks in output */
	escapeAsterisks?: boolean;
	/** Escape underscores in output */
	escapeUnderscores?: boolean;
	/** Escape miscellaneous characters */
	escapeMisc?: boolean;
	/** Escape ASCII control characters */
	escapeAscii?: boolean;
	/** Language for code blocks */
	codeLanguage?: string;
	/** Auto-convert URLs to links */
	autolinks?: boolean;
	/** Default document title */
	defaultTitle?: boolean;
	/** Use HTML line breaks in tables */
	brInTables?: boolean;
	/** Use hOCR spatial tables */
	hocrSpatialTables?: boolean;
	/** Highlighting style */
	highlightStyle?: "double_equal" | "html" | "bold" | "none";
	/** Extract metadata from HTML */
	extractMetadata?: boolean;
	/** Whitespace handling mode */
	whitespaceMode?: "normalized" | "strict";
	/** Strip newlines from output */
	stripNewlines?: boolean;
	/** Wrap text output */
	wrap?: boolean;
	/** Text wrap width */
	wrapWidth?: number;
	/** Convert as inline content */
	convertAsInline?: boolean;
	/** Subscript symbol */
	subSymbol?: string;
	/** Superscript symbol */
	supSymbol?: string;
	/** Newline style */
	newlineStyle?: "spaces" | "backslash";
	/** Code block style */
	codeBlockStyle?: "indented" | "backticks" | "tildes";
	/** Elements to keep inline images in */
	keepInlineImagesIn?: string[];
	/** Output encoding */
	encoding?: string;
	/** Enable debug output */
	debug?: boolean;
	/** Tags to strip from output */
	stripTags?: string[];
	/** Tags to preserve in output */
	preserveTags?: string[];
	/** HTML preprocessing options */
	preprocessing?: HtmlPreprocessingOptions;
}

/**
 * Configuration for document extraction
 */
export interface ExtractionConfig {
	/** OCR configuration */
	ocr?: OcrConfig;
	/** Chunking configuration */
	chunking?: ChunkingConfig;
	/** Image extraction configuration */
	images?: ImageExtractionConfig;
	/** Page extraction configuration */
	pages?: PageExtractionConfig;
	/** Language detection configuration */
	languageDetection?: LanguageDetectionConfig;
	/** PDF extraction options */
	pdfOptions?: PdfConfig;
	/** Token reduction configuration */
	tokenReduction?: TokenReductionConfig;
	/** Post-processor configuration */
	postprocessor?: PostProcessorConfig;
	/** Keyword extraction configuration */
	keywords?: KeywordConfig;
	/** HTML conversion options */
	htmlOptions?: HtmlConversionOptions;
	/** Whether to use caching */
	useCache?: boolean;
	/** Enable quality processing */
	enableQualityProcessing?: boolean;
	/** Force OCR even if text is available */
	forceOcr?: boolean;
	/** Security limits for archive extraction */
	securityLimits?: Record<string, number>;
	/** Maximum concurrent extractions */
	maxConcurrentExtractions?: number;
	/**
	 * Content output format.
	 * Controls the format of the extracted content:
	 * - "plain": Raw extracted text (default)
	 * - "markdown": Markdown formatted output
	 * - "djot": Djot markup format
	 * - "html": HTML formatted output
	 */
	outputFormat?: "plain" | "markdown" | "djot" | "html";
	/**
	 * Result structure format.
	 * Controls whether results are returned in unified format or element-based format.
	 * - "unified": All content in the content field (default)
	 * - "element_based": Semantic elements for Unstructured compatibility
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
}

/**
 * Tesseract OCR configuration
 */
export interface TesseractConfig {
	/** Tesseract page segmentation mode */
	psm?: number;
	/** Enable table detection */
	enableTableDetection?: boolean;
	/** Character whitelist for recognition */
	tesseditCharWhitelist?: string;
}

/**
 * OCR configuration
 */
export interface OcrConfig {
	/** OCR backend to use */
	backend?: string;
	/** Language codes (ISO 639) */
	languages?: string[];
	/** Whether to perform OCR */
	enabled?: boolean;
	/** Tesseract-specific configuration */
	tesseractConfig?: TesseractConfig;
	/** Language code for OCR */
	language?: string;
}

/**
 * Chunking configuration
 */
export interface ChunkingConfig {
	/** Maximum characters per chunk */
	maxChars?: number;
	/** Overlap between chunks */
	maxOverlap?: number;
}

/**
 * Image extraction configuration
 */
export interface ImageExtractionConfig {
	/** Whether to extract images */
	enabled?: boolean;
	/** Target DPI for image extraction */
	targetDpi?: number;
	/** Maximum image dimension in pixels */
	maxImageDimension?: number;
	/** Automatically adjust DPI */
	autoAdjustDpi?: boolean;
	/** Minimum DPI threshold */
	minDpi?: number;
	/** Maximum DPI threshold */
	maxDpi?: number;
}

/**
 * PDF extraction configuration
 */
export interface PdfConfig {
	/** Whether to extract images from PDF */
	extractImages?: boolean;
	/** Passwords for encrypted PDFs */
	passwords?: string[];
	/** Whether to extract metadata */
	extractMetadata?: boolean;
	/** Whether to extract annotations from PDF */
	extractAnnotations?: boolean;
	/** Top margin fraction (0.0-0.5) for filtering header content */
	topMarginFraction?: number;
	/** Bottom margin fraction (0.0-0.5) for filtering footer content */
	bottomMarginFraction?: number;
}

/**
 * Page extraction configuration
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
 * Language detection configuration
 */
export interface LanguageDetectionConfig {
	/** Whether to detect languages */
	enabled?: boolean;
}

/**
 * Semantic element type classification.
 *
 * Categorizes text content extracted from documents into semantic units for downstream processing.
 * Supports element types commonly found in documents processed by Unstructured-compatible systems.
 *
 * WASM serialization note: This is serialized from Rust using serde with snake_case transformation.
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
 * Represents the spatial boundaries of an element on a page using normalized coordinates.
 * Coordinates are in document space (typically PDF or image coordinates).
 *
 * WASM serialization note: All fields are serialized as numbers (floats) by serde.
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
 * Metadata for a semantic element.
 *
 * Contains optional contextual information about the element including its page location,
 * source filename, bounding box coordinates, and custom metadata fields.
 *
 * WASM serialization note: Optional fields use snake_case from Rust with serde skip_serializing_if.
 */
export interface ElementMetadata {
	/** Page number (1-indexed) */
	page_number?: number | null;
	/** Source filename or document name */
	filename?: string | null;
	/** Bounding box coordinates if available */
	coordinates?: BoundingBox | null;
	/** Position index in the element sequence */
	element_index?: number | null;
	/** Additional custom metadata fields */
	additional?: Record<string, string>;
}

/**
 * Semantic element extracted from document.
 *
 * Represents a logical unit of content with semantic classification, unique identifier,
 * and metadata for tracking origin and position. Compatible with Unstructured.io element
 * format when using element-based output.
 *
 * This type is generated by serde serialization from the Rust Element struct and includes:
 * - A deterministic element ID based on content and location
 * - Semantic type classification for downstream processing
 * - Full text content
 * - Comprehensive metadata including page numbers and coordinates
 *
 * WASM serialization note: All fields are serialized directly from Rust types with snake_case
 * field name transformation applied by serde.
 */
export interface Element {
	/** Unique element identifier (deterministic hash-based ID) */
	element_id: string;
	/** Semantic type classification */
	element_type: ElementType;
	/** Text content of the element */
	text: string;
	/** Metadata about the element including page number, coordinates, etc. */
	metadata: ElementMetadata;
}

/**
 * Non-fatal warning from the extraction pipeline
 */
export interface ProcessingWarning {
	/** Pipeline stage name that generated this warning */
	source: string;
	/** Warning description */
	message: string;
}

/**
 * OCR bounding geometry using rectangle coordinates
 */
export interface OcrBoundingGeometryRectangle {
	type: "rectangle";
	left: number;
	top: number;
	width: number;
	height: number;
}

/**
 * OCR bounding geometry using quadrilateral points
 */
export interface OcrBoundingGeometryQuadrilateral {
	type: "quadrilateral";
	points: number[][];
}

export type OcrBoundingGeometry = OcrBoundingGeometryRectangle | OcrBoundingGeometryQuadrilateral;

export interface OcrConfidence {
	detection?: number;
	recognition?: number;
}

export type OcrElementLevel = "word" | "line" | "block" | "page";

/**
 * Individual OCR element with bounding box and confidence scores
 */
export interface OcrElement {
	text: string;
	geometry?: OcrBoundingGeometry;
	confidence?: OcrConfidence;
	level?: OcrElementLevel;
	pageNumber?: number;
	parentId?: string;
}

export type NodeContentType =
	| "title"
	| "heading"
	| "paragraph"
	| "list"
	| "list_item"
	| "table"
	| "image"
	| "code"
	| "quote"
	| "formula"
	| "footnote"
	| "group"
	| "page_break";

export type ContentLayer = "body" | "header" | "footer" | "footnote";

export interface DocumentNode {
	id: string;
	content: Record<string, unknown>;
	parent?: number | null;
	children?: number[] | null;
	contentLayer?: ContentLayer | null;
	page?: number | null;
	pageEnd?: number | null;
	bbox?: BoundingBox | null;
}

export interface DocumentStructure {
	nodes: DocumentNode[];
}

/** Type of PDF annotation */
export type PdfAnnotationType = "text" | "highlight" | "link" | "stamp" | "underline" | "strike_out" | "other";

/** A PDF annotation extracted from a document page */
export interface PdfAnnotation {
	/** The type of annotation */
	annotationType: PdfAnnotationType;
	/** Text content of the annotation (e.g., comment text, link URL) */
	content?: string | null;
	/** Page number where the annotation appears (1-indexed) */
	pageNumber: number;
	/** Bounding box of the annotation on the page */
	boundingBox?: BoundingBox | null;
}

/**
 * Result of document extraction
 */
export interface ExtractionResult {
	/** Extracted text content */
	content: string;
	/** MIME type of the document */
	mimeType: string;
	/** Document metadata */
	metadata: Metadata;
	/** Extracted tables */
	tables: Table[];
	/** Detected languages (ISO 639 codes) */
	detectedLanguages?: string[] | null;
	/** Text chunks when chunking is enabled */
	chunks?: Chunk[] | null;
	/** Extracted images */
	images?: ExtractedImage[] | null;
	/** Per-page content */
	pages?: PageContent[] | null;
	/** Extracted keywords when keyword extraction is enabled */
	extractedKeywords?: ExtractedKeyword[] | null;
	/** Quality score (0.0-1.0) when quality processing is enabled */
	qualityScore?: number | null;
	/** Non-fatal warnings from the extraction pipeline */
	processingWarnings?: ProcessingWarning[] | null;
	/** Semantic elements when element-based output format is used */
	elements?: Element[] | null;
	/** OCR elements with bounding boxes and confidence scores */
	ocrElements?: OcrElement[] | null;
	/** Hierarchical document structure */
	document?: DocumentStructure | null;
	/** PDF annotations (text notes, highlights, links, stamps) */
	annotations?: PdfAnnotation[] | null;
}

/**
 * Document metadata
 */
export interface Metadata {
	/** Document title */
	title?: string;
	/** Document subject or description */
	subject?: string;
	/** Document author(s) */
	authors?: string[];
	/** Keywords/tags */
	keywords?: string[];
	/** Primary language (ISO 639 code) */
	language?: string;
	/** Creation timestamp (ISO 8601 format) */
	createdAt?: string;
	/** Last modification timestamp (ISO 8601 format) */
	modifiedAt?: string;
	/** User who created the document */
	creator?: string;
	/** User who last modified the document */
	lastModifiedBy?: string;
	/** Number of pages/slides */
	pageCount?: number;
	/** Document category */
	category?: string | null;
	/** Document tags */
	tags?: string[];
	/** Document version */
	documentVersion?: string | null;
	/** Document abstract */
	abstractText?: string | null;
	/** Output format used (plain, markdown, djot, html, structured) */
	outputFormat?: string | null;
	/** Format-specific metadata */
	formatMetadata?: unknown;
	/**
	 * Additional fields may be added at runtime by postprocessors.
	 * Use bracket notation to safely access unexpected properties.
	 */
	[key: string]: unknown;
}

/**
 * Extracted table
 */
export interface Table {
	/** Table cells/rows */
	cells?: string[][];
	/** Table markdown representation */
	markdown?: string;
	/** Page number if available */
	pageNumber?: number;
	/** Table headers */
	headers?: string[];
	/** Table rows */
	rows?: string[][];
	/** Bounding box of the table on the page (PDF coordinates). */
	boundingBox?: BoundingBox | null;
}

/**
 * Chunk metadata
 */
export interface ChunkMetadata {
	/** Character start position in original content */
	charStart: number;
	/** Character end position in original content */
	charEnd: number;
	/** Token count if available */
	tokenCount: number | null;
	/** Index of this chunk */
	chunkIndex: number;
	/** Total number of chunks */
	totalChunks: number;
}

/**
 * Text chunk from chunked content
 */
export interface Chunk {
	/** Chunk text content */
	content: string;
	/** Chunk metadata */
	metadata?: ChunkMetadata;
	/** Character position in original content (legacy) */
	charIndex?: number;
	/** Token count if available (legacy) */
	tokenCount?: number;
	/** Embedding vector if computed */
	embedding?: number[] | null;
}

/**
 * Extracted image from document
 */
export interface ExtractedImage {
	/** Image data as Uint8Array or base64 string */
	data: Uint8Array | string;
	/** Image format/MIME type */
	format?: string;
	/** MIME type of the image */
	mimeType?: string;
	/** Image index in document */
	imageIndex?: number;
	/** Page number if available */
	pageNumber?: number | null;
	/** Image width in pixels */
	width?: number | null;
	/** Image height in pixels */
	height?: number | null;
	/** Color space of the image */
	colorspace?: string | null;
	/** Bits per color component */
	bitsPerComponent?: number | null;
	/** Whether this is a mask image */
	isMask?: boolean;
	/** Image description */
	description?: string | null;
	/** Optional OCR result from the image */
	ocrResult?: ExtractionResult | string | null;
	/** Bounding box of the image on the page (PDF coordinates). */
	boundingBox?: BoundingBox | null;
}

/**
 * A text block with hierarchy level assignment.
 */
export interface HierarchicalBlock {
	/** The text content of this block */
	text: string;
	/** The font size of the text in this block */
	font_size: number;
	/** The hierarchy level (h1-h6 or body) */
	level: string;
	/** Bounding box as (left, top, right, bottom) in PDF units */
	bbox?: [number, number, number, number] | null;
}

/**
 * Page hierarchy structure containing heading levels and block information.
 */
export interface PageHierarchy {
	/** Number of hierarchy blocks on this page */
	block_count: number;
	/** Hierarchical blocks with heading levels */
	blocks: HierarchicalBlock[];
}

/**
 * Per-page content
 */
export interface PageContent {
	/** Page number (1-indexed) */
	pageNumber: number;
	/** Text content of the page */
	content: string;
	/** Tables on this page */
	tables?: Table[];
	/** Images on this page */
	images?: ExtractedImage[];
	/** Hierarchy information for the page */
	hierarchy?: PageHierarchy | null;
	/** Whether this page is blank (contains no meaningful content) */
	isBlank?: boolean;
}

/**
 * OCR backend protocol/interface
 */
export interface OcrBackendProtocol {
	/** Get the backend name */
	name(): string;
	/** Get supported language codes */
	supportedLanguages?(): string[];
	/** Initialize the backend */
	initialize(options?: Record<string, unknown>): void | Promise<void>;
	/** Shutdown the backend */
	shutdown?(): void | Promise<void>;
	/** Process an image with OCR */
	processImage(
		imageData: Uint8Array | string,
		language?: string,
	): Promise<
		| {
				content: string;
				mime_type: string;
				metadata?: Record<string, unknown>;
				tables?: unknown[];
		  }
		| string
	>;
}
