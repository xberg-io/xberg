/**
 * Result type definitions for Kreuzberg document extraction.
 *
 * These types represent the output of extraction operations,
 * including extracted content, metadata, tables, chunks, images, and keywords.
 */

import type { ExtractedKeyword } from "./config.js";
import type { Metadata } from "./metadata.js";

// ============================================================================

export interface ProcessingWarning {
	/** Pipeline stage name that generated this warning */
	source: string;
	/** Warning description */
	message: string;
}

export interface Table {
	cells: string[][];
	markdown: string;
	pageNumber: number;
	/** Bounding box of the table on the page (PDF coordinates). */
	boundingBox?: BoundingBox | null;
}

export interface HeadingLevel {
	/** Heading depth (1 = h1, 2 = h2, etc.) */
	level: number;
	/** Text content of the heading */
	text: string;
}

export interface HeadingContext {
	/** Heading hierarchy from document root to this chunk's section */
	headings: HeadingLevel[];
}

export interface ChunkMetadata {
	byteStart: number;
	byteEnd: number;
	tokenCount?: number | null;
	chunkIndex: number;
	totalChunks: number;
	firstPage?: number | null;
	lastPage?: number | null;
	headingContext?: HeadingContext | null;
}

export interface Chunk {
	content: string;
	embedding?: number[] | null;
	metadata: ChunkMetadata;
}

export interface ExtractedImage {
	data: Uint8Array;
	format: string;
	imageIndex: number;
	pageNumber?: number | null;
	width?: number | null;
	height?: number | null;
	colorspace?: string | null;
	bitsPerComponent?: number | null;
	isMask: boolean;
	description?: string | null;
	ocrResult?: ExtractionResult;
	/** Bounding box of the image on the page (PDF coordinates). */
	boundingBox?: BoundingBox | null;
}

// ============================================================================
// Element-based output types (compatible with Unstructured.io format)

/**
 * Semantic element type classification.
 *
 * Categorizes text content into semantic units for downstream processing.
 * Supports element types commonly found in document analysis.
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
 * Represents a rectangular region in a document with normalized coordinates.
 */
export interface BoundingBox {
	/** Left x-coordinate (0.0 to 1.0 or page-width normalized) */
	x0: number;
	/** Bottom y-coordinate (0.0 to 1.0 or page-height normalized) */
	y0: number;
	/** Right x-coordinate (0.0 to 1.0 or page-width normalized) */
	x1: number;
	/** Top y-coordinate (0.0 to 1.0 or page-height normalized) */
	y1: number;
}

/**
 * Metadata for a semantic element.
 *
 * Provides contextual information about an extracted element including
 * page location, document filename, spatial coordinates, and custom metadata.
 */
export interface ElementMetadata {
	/** Page number (1-indexed), or null if not available */
	pageNumber?: number | null;
	/** Source filename or document name, or null if not available */
	filename?: string | null;
	/** Bounding box coordinates if available, or null */
	coordinates?: BoundingBox | null;
	/** Position index in the element sequence, or null if not available */
	elementIndex?: number | null;
	/** Additional custom metadata fields */
	additional?: Record<string, string>;
}

/**
 * Semantic element extracted from document.
 *
 * Represents a logical unit of content with semantic classification,
 * unique identifier, and metadata for tracking origin and position.
 * Compatible with Unstructured.io element format.
 */
export interface Element {
	/** Unique element identifier (deterministic hash-based ID) */
	elementId: string;
	/** Semantic type of this element */
	elementType: ElementType;
	/** Text content of the element */
	text: string;
	/** Metadata about the element including page number, coordinates, etc. */
	metadata: ElementMetadata;
}

// ============================================================================
// Djot structured content types

/**
 * Structured Djot document representation.
 *
 * Provides rich, block-level document structure with formatting,
 * tables, images, links, and metadata extracted from source documents.
 * Available when Djot output format is enabled.
 */
export interface DjotContent {
	/** Plain text representation for backward compatibility */
	plainText: string;
	/** Structured block-level content (headings, paragraphs, lists, etc.) */
	blocks: FormattedBlock[];
	/** Metadata from YAML frontmatter */
	metadata: Metadata;
	/** Extracted tables as structured data */
	tables: Table[];
	/** Extracted images with metadata */
	images: DjotImage[];
	/** Extracted links with URLs and titles */
	links: DjotLink[];
	/** Footnote definitions */
	footnotes: Footnote[];
	/** Attributes mapped by element identifier */
	attributes?: Record<string, Attributes>;
}

/**
 * Block-level element in a Djot document.
 *
 * Represents structural elements like headings, paragraphs, lists, code blocks, etc.
 */
export interface FormattedBlock {
	/** Type of block element */
	blockType: BlockType;
	/** Heading level (1-6) for headings, or nesting level for lists */
	level?: number | null;
	/** Text content for inline elements */
	content?: string | null;
	/** Child blocks for list items and containers */
	children?: FormattedBlock[] | null;
	/** Attributes (id, class, etc.) */
	attributes?: Attributes | null;
}

/**
 * Block element type classification.
 */
export type BlockType =
	| "paragraph"
	| "heading"
	| "list_item"
	| "code_block"
	| "block_quote"
	| "thematic_break"
	| "table"
	| "image"
	| "footnote_definition"
	| "raw_block";

/**
 * HTML/CSS attributes for an element.
 */
export interface Attributes {
	[key: string]: string | number | boolean | string[] | null;
}

/**
 * Image with metadata in Djot document.
 */
export interface DjotImage {
	/** Image URL or reference */
	url: string;
	/** Alternative text */
	alt?: string | null;
	/** Image title/caption */
	title?: string | null;
	/** Image attributes */
	attributes?: Attributes | null;
}

/**
 * Link in Djot document.
 */
export interface DjotLink {
	/** Link URL */
	url: string;
	/** Link text */
	text: string;
	/** Link title */
	title?: string | null;
	/** Link type */
	linkType?: "internal" | "external" | "email" | "phone" | "footnote";
}

/**
 * Footnote definition.
 */
export interface Footnote {
	/** Footnote identifier */
	label: string;
	/** Footnote content */
	content: string;
}

export interface HierarchicalBlock {
	text: string;
	fontSize: number;
	level: string;
	bbox?: [number, number, number, number] | null;
}

export interface PageHierarchy {
	blockCount: number;
	blocks: HierarchicalBlock[];
}

export interface PageContent {
	pageNumber: number;
	content: string;
	tables: Table[];
	images: ExtractedImage[];
	hierarchy?: PageHierarchy;
	isBlank?: boolean;
}

// ============================================================================
// OCR element types

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
	angle_degrees?: number;

	/** Confidence score (0.0-1.0) for rotation detection. */
	confidence?: number;
}

/**
 * OCR element hierarchy level.
 *
 * Defines the granularity of OCR element extraction.
 */
export type OcrElementLevel = "word" | "line" | "block" | "page";

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
	page_number?: number;

	/** Parent element ID for hierarchical relationships */
	parent_id?: string;

	/** Backend-specific metadata that doesn't fit standard fields */
	backend_metadata?: Record<string, unknown>;
}

// ============================================================================
// Document Structure types

/**
 * Semantic node type in document structure.
 */
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

/**
 * Content layer classification for document nodes.
 */
export type ContentLayer = "body" | "header" | "footer" | "footnote";

/**
 * Structured table grid with cell-level metadata.
 */
export interface TableGrid {
	rows: number;
	cols: number;
	cells: GridCell[];
}

/**
 * Individual grid cell with position and span metadata.
 */
export interface GridCell {
	content: string;
	row: number;
	col: number;
	row_span: number;
	col_span: number;
	is_header: boolean;
	bbox?: BoundingBox | null;
}

/**
 * Inline text annotation (formatting, links).
 */
export interface TextAnnotation {
	start: number;
	end: number;
	kind: AnnotationKind;
}

/**
 * Types of inline text annotations.
 */
export type AnnotationKind =
	| { annotation_type: "bold" }
	| { annotation_type: "italic" }
	| { annotation_type: "underline" }
	| { annotation_type: "strikethrough" }
	| { annotation_type: "code" }
	| { annotation_type: "subscript" }
	| { annotation_type: "superscript" }
	| { annotation_type: "link"; url: string; title?: string | null };

/**
 * Tagged union for node content. Each variant carries only type-specific data.
 */
export type NodeContent =
	| { node_type: "title"; text: string }
	| { node_type: "heading"; level: number; text: string }
	| { node_type: "paragraph"; text: string }
	| { node_type: "list"; ordered: boolean }
	| { node_type: "list_item"; text: string }
	| { node_type: "table"; grid: TableGrid }
	| {
			node_type: "image";
			description?: string | null;
			image_index?: number | null;
	  }
	| { node_type: "code"; text: string; language?: string | null }
	| { node_type: "quote" }
	| { node_type: "formula"; text: string }
	| { node_type: "footnote"; text: string }
	| {
			node_type: "group";
			label?: string | null;
			heading_level?: number | null;
			heading_text?: string | null;
	  }
	| { node_type: "page_break" };

/**
 * A single node in the document tree.
 *
 * Each node has deterministic id, typed content, optional parent/children
 * for tree structure, and metadata like page number, bounding box, and content layer.
 */
export interface DocumentNode {
	/** Deterministic identifier (hash of content + position) */
	id: string;
	/** Node content — tagged enum, type-specific data only */
	content: NodeContent;
	/** Parent node index (undefined = root-level node) */
	parent?: number | null;
	/** Child node indices in reading order */
	children?: number[] | null;
	/** Content layer classification */
	content_layer?: ContentLayer | null;
	/** Page number where this node starts (1-indexed) */
	page?: number | null;
	/** Page number where this node ends (for multi-page tables/sections) */
	page_end?: number | null;
	/** Bounding box in document coordinates */
	bbox?: BoundingBox | null;
	/** Inline annotations (formatting, links) on this node's text content */
	annotations?: TextAnnotation[] | null;
}

/**
 * Top-level structured document representation.
 *
 * A flat array of nodes with index-based parent/child references forming a tree.
 * Root-level nodes have parent undefined. Nodes are in document/reading order.
 */
export interface DocumentStructure {
	/** All nodes in document/reading order */
	nodes: DocumentNode[];
}

/**
 * Type of PDF annotation.
 */
export type PdfAnnotationType = "text" | "highlight" | "link" | "stamp" | "underline" | "strike_out" | "other";

/**
 * Bounding box for a PDF annotation.
 */
export interface PdfAnnotationBoundingBox {
	x0: number;
	y0: number;
	x1: number;
	y1: number;
}

/**
 * A PDF annotation extracted from a document.
 */
export interface PdfAnnotation {
	annotationType: PdfAnnotationType;
	content?: string | null;
	pageNumber: number;
	boundingBox?: PdfAnnotationBoundingBox | null;
}

export interface ExtractionResult {
	content: string;
	mimeType: string;
	metadata: Metadata;
	tables: Table[];
	detectedLanguages?: string[];
	chunks?: Chunk[];
	images?: ExtractedImage[];
	pages?: PageContent[];
	elements?: Element[];
	ocrElements?: OcrElement[];
	document?: DocumentStructure | null;
	extractedKeywords?: ExtractedKeyword[];
	qualityScore?: number;
	processingWarnings: ProcessingWarning[];
	annotations?: PdfAnnotation[];
}
