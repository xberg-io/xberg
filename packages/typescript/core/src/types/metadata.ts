/**
 * Metadata type definitions for extracted documents.
 *
 * Includes format-specific metadata for various document types
 * (PDF, Excel, Email, Images, etc.) and the unified Metadata interface.
 */

// ============================================================================
// ============================================================================

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

export interface ImageMetadata {
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
	htmlImages: ImageMetadata[];
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
	date?: string | null;
	subject?: string | null;

	format_type?: "pdf" | "excel" | "email" | "pptx" | "archive" | "image" | "xml" | "text" | "html" | "ocr";

	title?: string | null;
	author?: string | null;
	keywords?: string | null;
	creator?: string | null;
	producer?: string | null;
	creation_date?: string | null;
	modification_date?: string | null;
	page_count?: number;

	sheet_count?: number;
	sheet_names?: string[];

	from_email?: string | null;
	from_name?: string | null;
	to_emails?: string[];
	cc_emails?: string[];
	bcc_emails?: string[];
	message_id?: string | null;
	attachments?: string[];

	description?: string | null;
	summary?: string | null;
	fonts?: string[];

	format?: string;
	file_count?: number;
	file_list?: string[];
	total_size?: number;
	compressed_size?: number | null;

	width?: number;
	height?: number;
	exif?: Record<string, string>;

	element_count?: number;
	unique_elements?: string[];

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
	html_images?: ImageMetadata[];
	structured_data?: StructuredData[];

	psm?: number;
	output_format?: string;
	table_count?: number;
	table_rows?: number | null;
	table_cols?: number | null;

	image_preprocessing?: ImagePreprocessingMetadata | null;

	json_schema?: Record<string, unknown> | null;

	error?: ErrorMetadata | null;

	// biome-ignore lint/suspicious/noExplicitAny: Postprocessors can add arbitrary metadata fields
	[key: string]: any;
}
