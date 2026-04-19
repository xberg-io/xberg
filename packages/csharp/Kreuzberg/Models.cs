using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace Kreuzberg;

/// <summary>
/// Semantic element type classification for extracted document content.
///
/// Categorizes text content into semantic units for downstream processing.
/// Compatible with Unstructured.io element types.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<ElementType>))]
public enum ElementType
{
    /// <summary>
    /// Document title element.
    /// </summary>
    [JsonStringEnumMemberName("title")]
    Title,

    /// <summary>
    /// Main narrative text body element.
    /// </summary>
    [JsonStringEnumMemberName("narrative_text")]
    NarrativeText,

    /// <summary>
    /// Section heading element.
    /// </summary>
    [JsonStringEnumMemberName("heading")]
    Heading,

    /// <summary>
    /// List item element (bullet, numbered, etc.).
    /// </summary>
    [JsonStringEnumMemberName("list_item")]
    ListItem,

    /// <summary>
    /// Table element.
    /// </summary>
    [JsonStringEnumMemberName("table")]
    Table,

    /// <summary>
    /// Image element.
    /// </summary>
    [JsonStringEnumMemberName("image")]
    Image,

    /// <summary>
    /// Page break marker element.
    /// </summary>
    [JsonStringEnumMemberName("page_break")]
    PageBreak,

    /// <summary>
    /// Code block element.
    /// </summary>
    [JsonStringEnumMemberName("code_block")]
    CodeBlock,

    /// <summary>
    /// Block quote element.
    /// </summary>
    [JsonStringEnumMemberName("block_quote")]
    BlockQuote,

    /// <summary>
    /// Footer text element.
    /// </summary>
    [JsonStringEnumMemberName("footer")]
    Footer,

    /// <summary>
    /// Header text element.
    /// </summary>
    [JsonStringEnumMemberName("header")]
    Header,
}

/// <summary>
/// Bounding box coordinates for element positioning on a page.
///
/// Defines the rectangular region occupied by an element using normalized or
/// absolute coordinates depending on the document format.
/// </summary>
public sealed class BoundingBox
{
    /// <summary>
    /// Left x-coordinate (origin from left edge).
    /// </summary>
    [JsonPropertyName("x0")]
    public double X0 { get; init; }

    /// <summary>
    /// Bottom y-coordinate (origin from bottom edge).
    /// </summary>
    [JsonPropertyName("y0")]
    public double Y0 { get; init; }

    /// <summary>
    /// Right x-coordinate (origin from left edge).
    /// </summary>
    [JsonPropertyName("x1")]
    public double X1 { get; init; }

    /// <summary>
    /// Top y-coordinate (origin from bottom edge).
    /// </summary>
    [JsonPropertyName("y1")]
    public double Y1 { get; init; }
}

/// <summary>
/// Metadata for a semantic element extracted from a document.
///
/// Includes positioning information, page references, and custom metadata
/// for tracking element origin and context within the source document.
/// </summary>
public sealed class ElementMetadata
{
    /// <summary>
    /// Page number (1-indexed) where this element appears in the document, if available.
    /// </summary>
    [JsonPropertyName("page_number")]
    public int? PageNumber { get; init; }

    /// <summary>
    /// Source filename or document name from which this element was extracted, if available.
    /// </summary>
    [JsonPropertyName("filename")]
    public string? Filename { get; init; }

    /// <summary>
    /// Bounding box coordinates for this element on the page, if available.
    /// </summary>
    [JsonPropertyName("coordinates")]
    public BoundingBox? Coordinates { get; init; }

    /// <summary>
    /// Position index of this element in the document's element sequence, if available.
    /// </summary>
    [JsonPropertyName("element_index")]
    public int? ElementIndex { get; init; }

    /// <summary>
    /// Additional custom metadata fields for this element.
    /// </summary>
    [JsonPropertyName("additional")]
    public Dictionary<string, string>? Additional { get; init; }
}

/// <summary>
/// A semantic element extracted from a document.
///
/// Represents a logical unit of content with semantic classification, unique identifier,
/// and metadata for tracking origin, position, and context within the source document.
/// Compatible with Unstructured.io element format when output_format='element_based'.
/// </summary>
public sealed class Element
{
    /// <summary>
    /// Unique identifier for this element (deterministic hash-based ID).
    /// </summary>
    [JsonPropertyName("element_id")]
    public string ElementId { get; init; } = string.Empty;

    /// <summary>
    /// Semantic type classification for this element.
    /// </summary>
    [JsonPropertyName("element_type")]
    public ElementType ElementType { get; init; }

    /// <summary>
    /// Text content of this element.
    /// </summary>
    [JsonPropertyName("text")]
    public string Text { get; init; } = string.Empty;

    /// <summary>
    /// Metadata about this element including page number, coordinates, and custom fields.
    /// </summary>
    [JsonPropertyName("metadata")]
    public ElementMetadata Metadata { get; init; } = new();
}

/// <summary>
/// Bounding geometry information for OCR elements with coordinates and point data.
/// </summary>
public sealed class OcrBoundingGeometry
{
    /// <summary>
    /// Type of geometry representation (e.g., "bbox", "polygon").
    /// </summary>
    [JsonPropertyName("type")]
    public string? Type { get; init; }

    /// <summary>
    /// Left x-coordinate of the bounding box.
    /// </summary>
    [JsonPropertyName("left")]
    public double? Left { get; init; }

    /// <summary>
    /// Top y-coordinate of the bounding box.
    /// </summary>
    [JsonPropertyName("top")]
    public double? Top { get; init; }

    /// <summary>
    /// Width of the bounding box.
    /// </summary>
    [JsonPropertyName("width")]
    public double? Width { get; init; }

    /// <summary>
    /// Height of the bounding box.
    /// </summary>
    [JsonPropertyName("height")]
    public double? Height { get; init; }

    /// <summary>
    /// Points defining the geometry polygon (array of [x, y] coordinate pairs).
    /// </summary>
    [JsonPropertyName("points")]
    public List<List<double>>? Points { get; init; }
}

/// <summary>
/// Confidence scores for OCR recognition and detection.
/// </summary>
public sealed class OcrConfidence
{
    /// <summary>
    /// Confidence score for text detection (0.0-1.0).
    /// </summary>
    [JsonPropertyName("detection")]
    public double? Detection { get; init; }

    /// <summary>
    /// Confidence score for character recognition (0.0-1.0).
    /// </summary>
    [JsonPropertyName("recognition")]
    public double? Recognition { get; init; }
}

/// <summary>
/// Rotation information for OCR elements.
/// </summary>
public sealed class OcrRotation
{
    /// <summary>
    /// Rotation angle in degrees.
    /// </summary>
    [JsonPropertyName("angle_degrees")]
    public double? AngleDegrees { get; init; }

    /// <summary>
    /// Confidence score for rotation detection.
    /// </summary>
    [JsonPropertyName("confidence")]
    public double? Confidence { get; init; }
}

/// <summary>
/// OCR element level classification.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<OcrElementLevel>))]
public enum OcrElementLevel
{
    /// <summary>
    /// Individual word level.
    /// </summary>
    [JsonStringEnumMemberName("word")]
    Word,

    /// <summary>
    /// Line level (sequence of words).
    /// </summary>
    [JsonStringEnumMemberName("line")]
    Line,

    /// <summary>
    /// Paragraph/block level.
    /// </summary>
    [JsonStringEnumMemberName("block")]
    Block,

    /// <summary>
    /// Page level.
    /// </summary>
    [JsonStringEnumMemberName("page")]
    Page,
}

/// <summary>
/// An OCR element extracted from a document containing recognized text and geometric information.
/// </summary>
public sealed class OcrElement
{
    /// <summary>
    /// Recognized text content.
    /// </summary>
    [JsonPropertyName("text")]
    public string? Text { get; init; }

    /// <summary>
    /// Bounding geometry information.
    /// </summary>
    [JsonPropertyName("geometry")]
    public OcrBoundingGeometry? Geometry { get; init; }

    /// <summary>
    /// Confidence scores for detection and recognition.
    /// </summary>
    [JsonPropertyName("confidence")]
    public OcrConfidence? Confidence { get; init; }

    /// <summary>
    /// Hierarchical level of this element (word, line, block, page).
    /// </summary>
    [JsonPropertyName("level")]
    public string? Level { get; init; }

    /// <summary>
    /// Rotation information if the element is rotated.
    /// </summary>
    [JsonPropertyName("rotation")]
    public OcrRotation? Rotation { get; init; }

    /// <summary>
    /// Page number where this element appears (1-indexed).
    /// </summary>
    [JsonPropertyName("page_number")]
    public int? PageNumber { get; init; }

    /// <summary>
    /// Parent element ID for hierarchical relationships.
    /// </summary>
    [JsonPropertyName("parent_id")]
    public string? ParentId { get; init; }

    /// <summary>
    /// Backend-specific metadata.
    /// </summary>
    [JsonPropertyName("backend_metadata")]
    public Dictionary<string, object>? BackendMetadata { get; init; }
}

/// <summary>
/// Configuration for OCR element extraction behavior.
/// </summary>
public sealed class OcrElementConfig
{
    /// <summary>
    /// Whether to include OCR elements in the output.
    /// </summary>
    [JsonPropertyName("include_elements")]
    public bool IncludeElements { get; init; }

    /// <summary>
    /// Minimum hierarchical level to include (word, line, block, page).
    /// </summary>
    [JsonPropertyName("min_level")]
    public string? MinLevel { get; init; }

    /// <summary>
    /// Minimum confidence threshold for including elements (0.0-1.0).
    /// </summary>
    [JsonPropertyName("min_confidence")]
    public double? MinConfidence { get; init; }

    /// <summary>
    /// Whether to build a hierarchical structure from elements.
    /// </summary>
    [JsonPropertyName("build_hierarchy")]
    public bool BuildHierarchy { get; init; }
}

/// <summary>
/// PaddleOCR-specific configuration options.
/// </summary>
public sealed class PaddleOcrConfig
{
    /// <summary>
    /// Languages to recognize (e.g., "en", "ch" for Chinese, "en,ch" for mixed).
    /// </summary>
    [JsonPropertyName("language")]
    public string? Language { get; init; }

    /// <summary>
    /// Cache directory for model files.
    /// </summary>
    [JsonPropertyName("cache_dir")]
    public string? CacheDir { get; init; }

    /// <summary>
    /// Whether to use angle classification for rotated text.
    /// </summary>
    [JsonPropertyName("use_angle_cls")]
    public bool? UseAngleCls { get; init; }

    /// <summary>
    /// Whether to enable table detection.
    /// </summary>
    [JsonPropertyName("enable_table_detection")]
    public bool? EnableTableDetection { get; init; }

    /// <summary>
    /// Detection database threshold for text detection.
    /// </summary>
    [JsonPropertyName("det_db_thresh")]
    public double? DetDbThresh { get; init; }

    /// <summary>
    /// Detection database box threshold.
    /// </summary>
    [JsonPropertyName("det_db_box_thresh")]
    public double? DetDbBoxThresh { get; init; }

    /// <summary>
    /// Detection database unclip ratio.
    /// </summary>
    [JsonPropertyName("det_db_unclip_ratio")]
    public double? DetDbUnclipRatio { get; init; }

    /// <summary>
    /// Maximum limit for detection side length.
    /// </summary>
    [JsonPropertyName("det_limit_side_len")]
    public int? DetLimitSideLen { get; init; }

    /// <summary>
    /// Batch size for recognition.
    /// </summary>
    [JsonPropertyName("rec_batch_num")]
    public int? RecBatchNum { get; init; }
}

/// <summary>
/// The main result of document extraction containing extracted content, metadata, and structured data.
/// </summary>
public sealed class ExtractionResult
{
    /// <summary>
    /// The extracted text content from the document.
    /// </summary>
    [JsonPropertyName("content")]
    public string Content { get; set; } = string.Empty;

    /// <summary>
    /// The detected MIME type of the document (e.g., "application/pdf").
    /// </summary>
    [JsonPropertyName("mime_type")]
    public string MimeType { get; set; } = string.Empty;

    /// <summary>
    /// Document metadata including language, format-specific info, and other attributes.
    /// </summary>
    [JsonPropertyName("metadata")]
    public Metadata Metadata { get; set; } = new();

    /// <summary>
    /// Extracted tables from the document, if any.
    /// </summary>
    [JsonPropertyName("tables")]
    public List<Table> Tables { get; set; } = new();

    /// <summary>
    /// Detected languages in the document, if language detection was enabled.
    /// </summary>
    [JsonPropertyName("detected_languages")]
    public List<string>? DetectedLanguages { get; set; }

    /// <summary>
    /// Text chunks if chunking was enabled, each with metadata and optional embedding vector.
    /// </summary>
    [JsonPropertyName("chunks")]
    public List<Chunk>? Chunks { get; set; }

    /// <summary>
    /// Images extracted from the document, if image extraction was enabled.
    /// </summary>
    [JsonPropertyName("images")]
    public List<ExtractedImage>? Images { get; set; }

    /// <summary>
    /// Per-page extracted content when page extraction is enabled.
    /// </summary>
    [JsonPropertyName("pages")]
    public List<PageContent>? Pages { get; set; }

    /// <summary>
    /// Semantic elements extracted from the document when output_format='element_based'.
    /// Each element represents a logical unit of content with semantic classification and metadata.
    /// </summary>
    [JsonPropertyName("elements")]
    public List<Element>? Elements { get; set; }

    /// <summary>
    /// OCR elements extracted from documents when OCR processing is applied.
    /// Contains detailed information about recognized text, geometry, confidence, and hierarchical structure.
    /// </summary>
    [JsonPropertyName("ocr_elements")]
    public List<OcrElement>? OcrElements { get; set; }

    /// <summary>
    /// Rich Djot content structure when extracting Djot documents.
    /// </summary>
    [JsonPropertyName("djot_content")]
    public DjotContent? DjotContent { get; set; }

    /// <summary>
    /// Structured document representation with hierarchical node tree.
    /// Available when document structure extraction is enabled.
    /// </summary>
    [JsonPropertyName("document")]
    public DocumentStructure? Document { get; set; }

    /// <summary>
    /// Extracted keywords when keyword extraction is enabled.
    /// </summary>
    [JsonPropertyName("extracted_keywords")]
    public List<ExtractedKeyword>? ExtractedKeywords { get; set; }

    /// <summary>
    /// Document quality score (0.0-1.0) from quality analysis.
    /// </summary>
    [JsonPropertyName("quality_score")]
    public double? QualityScore { get; set; }

    /// <summary>
    /// Non-fatal warnings collected during processing pipeline stages.
    /// </summary>
    [JsonPropertyName("processing_warnings")]
    public List<ProcessingWarning>? ProcessingWarnings { get; set; }

    /// <summary>
    /// LLM usage metrics from structured extraction and other LLM-based processing.
    /// Contains token counts, costs, and other metrics from LLM calls.
    /// </summary>
    [JsonPropertyName("llm_usage")]
    public List<LlmUsage>? LlmUsage { get; set; }

    /// <summary>
    /// PDF annotations extracted from the document, if any.
    /// Available when the document is a PDF containing annotations such as
    /// comments, highlights, links, stamps, underlines, or strikeouts.
    /// </summary>
    [JsonPropertyName("annotations")]
    public List<PdfAnnotation>? Annotations { get; set; }

    /// <summary>
    /// URIs/links discovered during document extraction.
    /// Contains hyperlinks, image references, citations, email addresses, and other URI-like references.
    /// </summary>
    [JsonPropertyName("uris")]
    public List<ExtractedUri>? Uris { get; set; }

    /// <summary>
    /// Nested extraction results from archive contents.
    /// When extracting archives (ZIP, TAR, 7Z, GZIP), each processable file inside
    /// produces its own full extraction result.
    /// </summary>
    [JsonPropertyName("children")]
    public List<ArchiveEntry>? Children { get; set; }

    /// <summary>
    /// Code intelligence results from tree-sitter processing when the document is a code file.
    /// </summary>
    [JsonPropertyName("code_intelligence")]
    public CodeProcessResult? CodeIntelligence { get; set; }

    /// <summary>
    /// Structured output from structured extraction mode.
    /// Available when the output format is set to "structured".
    /// </summary>
    [JsonPropertyName("structured_output")]
    public Dictionary<string, object>? StructuredOutput { get; set; }
}

/// <summary>
/// Represents an annotation extracted from a PDF document.
/// </summary>
public sealed class PdfAnnotation
{
    /// <summary>
    /// The type of annotation (e.g., "text", "highlight", "link", "stamp", "underline", "strike_out", "other").
    /// </summary>
    [JsonPropertyName("annotation_type")]
    public string AnnotationType { get; init; } = string.Empty;

    /// <summary>
    /// The text content of the annotation, if available.
    /// </summary>
    [JsonPropertyName("content")]
    public string? Content { get; init; }

    /// <summary>
    /// The page number where the annotation appears (1-indexed).
    /// </summary>
    [JsonPropertyName("page_number")]
    public int PageNumber { get; init; }

    /// <summary>
    /// The bounding box coordinates of the annotation on the page, if available.
    /// </summary>
    [JsonPropertyName("bounding_box")]
    public PdfAnnotationBoundingBox? BoundingBox { get; init; }
}

/// <summary>
/// Bounding box for a PDF annotation (PDF coordinates).
/// </summary>
public sealed class PdfAnnotationBoundingBox
{
    /// <summary>
    /// Left x-coordinate.
    /// </summary>
    [JsonPropertyName("x0")]
    public double X0 { get; init; }

    /// <summary>
    /// Bottom y-coordinate.
    /// </summary>
    [JsonPropertyName("y0")]
    public double Y0 { get; init; }

    /// <summary>
    /// Right x-coordinate.
    /// </summary>
    [JsonPropertyName("x1")]
    public double X1 { get; init; }

    /// <summary>
    /// Top y-coordinate.
    /// </summary>
    [JsonPropertyName("y1")]
    public double Y1 { get; init; }
}

/// <summary>
/// A URI extracted from a document.
/// Represents any link, reference, or resource pointer found during extraction.
/// </summary>
public sealed class ExtractedUri
{
    /// <summary>
    /// The URL or path string.
    /// </summary>
    [JsonPropertyName("url")]
    public string Url { get; init; } = string.Empty;

    /// <summary>
    /// Optional display text / label for the link.
    /// </summary>
    [JsonPropertyName("label")]
    public string? Label { get; init; }

    /// <summary>
    /// Optional page number where the URI was found (1-indexed).
    /// </summary>
    [JsonPropertyName("page")]
    public uint? Page { get; init; }

    /// <summary>
    /// Semantic classification of the URI (hyperlink, image, anchor, citation, reference, email).
    /// </summary>
    [JsonPropertyName("kind")]
    public string Kind { get; init; } = string.Empty;
}

/// <summary>
/// A text block with hierarchy level assignment.
/// </summary>
public sealed class HierarchicalBlock
{
    /// <summary>
    /// The text content of this block.
    /// </summary>
    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;

    /// <summary>
    /// The font size of the text in this block.
    /// </summary>
    [JsonPropertyName("font_size")]
    public float FontSize { get; set; }

    /// <summary>
    /// The hierarchy level (h1-h6 or body).
    /// </summary>
    [JsonPropertyName("level")]
    public string Level { get; set; } = string.Empty;

    /// <summary>
    /// Bounding box as [left, top, right, bottom] in PDF units, if available.
    /// </summary>
    [JsonPropertyName("bbox")]
    public float[]? Bbox { get; set; }
}

/// <summary>
/// Page hierarchy structure containing heading levels and block information.
/// </summary>
public sealed class PageHierarchy
{
    /// <summary>
    /// Number of hierarchy blocks on this page.
    /// </summary>
    [JsonPropertyName("block_count")]
    public int BlockCount { get; set; }

    /// <summary>
    /// Hierarchical blocks with heading levels.
    /// </summary>
    [JsonPropertyName("blocks")]
    public List<HierarchicalBlock> Blocks { get; set; } = new();
}

/// <summary>
/// A detected layout region on a page.
/// </summary>
/// <remarks>
/// When layout detection is enabled, each page may have layout regions
/// identifying different content types (text, pictures, tables, etc.)
/// with confidence scores and spatial positions.
/// </remarks>
public sealed class LayoutRegion
{
    /// <summary>
    /// Layout class name (e.g. "picture", "table", "text", "section_header").
    /// </summary>
    [JsonPropertyName("class")]
    public string Class { get; set; } = string.Empty;

    /// <summary>
    /// Detection confidence score (0.0 to 1.0).
    /// </summary>
    [JsonPropertyName("confidence")]
    public double Confidence { get; set; }

    /// <summary>
    /// Bounding box in document coordinate space.
    /// </summary>
    [JsonPropertyName("bounding_box")]
    public BoundingBox BoundingBox { get; set; } = new();

    /// <summary>
    /// Fraction of the page area covered by this region (0.0 to 1.0).
    /// </summary>
    [JsonPropertyName("area_fraction")]
    public double AreaFraction { get; set; }
}

/// <summary>
/// Extracted content for a single page when page extraction is enabled.
/// </summary>
public sealed class PageContent
{
    /// <summary>
    /// The page number (1-indexed).
    /// </summary>
    [JsonPropertyName("page_number")]
    public int PageNumber { get; set; }

    /// <summary>
    /// The extracted text content for this page.
    /// </summary>
    [JsonPropertyName("content")]
    public string Content { get; set; } = string.Empty;

    /// <summary>
    /// Tables extracted from this page, if any.
    /// </summary>
    [JsonPropertyName("tables")]
    public List<Table> Tables { get; set; } = new();

    /// <summary>
    /// Images extracted from this page.
    /// </summary>
    [JsonPropertyName("images")]
    public List<ExtractedImage> Images { get; set; } = new();

    /// <summary>
    /// Hierarchy information for the page, if available.
    /// </summary>
    [JsonPropertyName("hierarchy")]
    public PageHierarchy? Hierarchy { get; set; }

    /// <summary>
    /// Whether this page is blank (contains no meaningful content).
    /// </summary>
    [JsonPropertyName("is_blank")]
    public bool? IsBlank { get; set; }

    /// <summary>
    /// Layout detection regions for this page (when layout detection is enabled).
    /// </summary>
    [JsonPropertyName("layout_regions")]
    public List<LayoutRegion>? LayoutRegions { get; set; }
}

/// <summary>
/// Represents a table extracted from a document.
/// </summary>
public sealed class Table
{
    /// <summary>
    /// Table cells arranged as rows (outer list) and columns (inner lists).
    /// </summary>
    [JsonPropertyName("cells")]
    public List<List<string>> Cells { get; set; } = new();

    /// <summary>
    /// Table representation in Markdown format.
    /// </summary>
    [JsonPropertyName("markdown")]
    public string Markdown { get; set; } = string.Empty;

    /// <summary>
    /// The page number (1-indexed) where this table appears in the document.
    /// </summary>
    [JsonPropertyName("page_number")]
    public int PageNumber { get; set; }

    /// <summary>
    /// Bounding box coordinates for this table on the page, if available.
    /// </summary>
    [JsonPropertyName("bounding_box")]
    public BoundingBox? BoundingBox { get; set; }
}

/// <summary>
/// A chunk of text from a document, used for splitting large documents into smaller pieces.
/// </summary>
public sealed class Chunk
{
    /// <summary>
    /// The text content of this chunk.
    /// </summary>
    [JsonPropertyName("content")]
    public string Content { get; set; } = string.Empty;

    /// <summary>
    /// Optional embedding vector for the chunk, if embedding was enabled.
    /// </summary>
    [JsonPropertyName("embedding")]
    public float[]? Embedding { get; set; }

    /// <summary>
    /// Metadata about the chunk including position and token count.
    /// </summary>
    [JsonPropertyName("metadata")]
    public ChunkMetadata Metadata { get; set; } = new();

    /// <summary>
    /// Semantic type classification of this chunk.
    /// </summary>
    [JsonPropertyName("chunk_type")]
    public string ChunkType { get; set; } = "unknown";
}

/// <summary>
/// Metadata about a text chunk including position, token count, and page information.
/// </summary>
public sealed class ChunkMetadata
{
    /// <summary>
    /// Starting byte position of this chunk in the document.
    /// </summary>
    [JsonPropertyName("byte_start")]
    public long ByteStart { get; set; }

    /// <summary>
    /// Ending byte position of this chunk in the document.
    /// </summary>
    [JsonPropertyName("byte_end")]
    public long ByteEnd { get; set; }

    /// <summary>
    /// Token count for this chunk, if tokenization was enabled.
    /// </summary>
    [JsonPropertyName("token_count")]
    public int? TokenCount { get; set; }

    /// <summary>
    /// Zero-based index of this chunk among all chunks.
    /// </summary>
    [JsonPropertyName("chunk_index")]
    public int ChunkIndex { get; set; }

    /// <summary>
    /// Total number of chunks the document was split into.
    /// </summary>
    [JsonPropertyName("total_chunks")]
    public int TotalChunks { get; set; }

    /// <summary>
    /// Page number (1-indexed) of the first page this chunk starts on, if page tracking is enabled.
    /// </summary>
    [JsonPropertyName("first_page")]
    public int? FirstPage { get; set; }

    /// <summary>
    /// Page number (1-indexed) of the last page this chunk ends on, if page tracking is enabled.
    /// </summary>
    [JsonPropertyName("last_page")]
    public int? LastPage { get; set; }

    /// <summary>
    /// Heading hierarchy for this chunk's section, if available (markdown chunker only).
    /// </summary>
    [JsonPropertyName("heading_context")]
    public HeadingContext? HeadingContext { get; set; }
}

/// <summary>
/// Heading context for a chunk's section in the document.
/// </summary>
public sealed class HeadingContext
{
    /// <summary>
    /// Heading hierarchy from document root to this chunk's section.
    /// </summary>
    [JsonPropertyName("headings")]
    public List<HeadingLevel> Headings { get; set; } = new();
}

/// <summary>
/// A single heading in the document hierarchy.
/// </summary>
public sealed class HeadingLevel
{
    /// <summary>
    /// Heading depth (1 = h1, 2 = h2, etc.).
    /// </summary>
    [JsonPropertyName("level")]
    public int Level { get; set; }

    /// <summary>
    /// Text content of the heading.
    /// </summary>
    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;
}

/// <summary>
/// Represents an image extracted from a document with metadata and optional OCR results.
/// </summary>
public sealed class ExtractedImage
{
    /// <summary>
    /// Raw image data as bytes (PNG, JPEG, etc.).
    /// </summary>
    [JsonPropertyName("data")]
    public byte[] Data { get; set; } = Array.Empty<byte>();

    /// <summary>
    /// Image format (e.g., "PNG", "JPEG", "TIFF").
    /// </summary>
    [JsonPropertyName("format")]
    public string Format { get; set; } = string.Empty;

    /// <summary>
    /// Zero-based index of this image among all extracted images.
    /// </summary>
    [JsonPropertyName("image_index")]
    public int ImageIndex { get; set; }

    /// <summary>
    /// Page number (1-indexed) where this image appears, if page tracking is enabled.
    /// </summary>
    [JsonPropertyName("page_number")]
    public int? PageNumber { get; set; }

    /// <summary>
    /// Image width in pixels.
    /// </summary>
    [JsonPropertyName("width")]
    public uint? Width { get; set; }

    /// <summary>
    /// Image height in pixels.
    /// </summary>
    [JsonPropertyName("height")]
    public uint? Height { get; set; }

    /// <summary>
    /// Color space representation (e.g., "RGB", "CMYK", "DeviceGray").
    /// </summary>
    [JsonPropertyName("colorspace")]
    public string? Colorspace { get; set; }

    /// <summary>
    /// Bits per color component.
    /// </summary>
    [JsonPropertyName("bits_per_component")]
    public uint? BitsPerComponent { get; set; }

    /// <summary>
    /// Whether this image is a mask/transparency image.
    /// </summary>
    [JsonPropertyName("is_mask")]
    public bool IsMask { get; set; }

    /// <summary>
    /// Optional description or alternate text for the image.
    /// </summary>
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    /// <summary>
    /// OCR extraction result if OCR was applied to this image.
    /// </summary>
    [JsonPropertyName("ocr_result")]
    public ExtractionResult? OcrResult { get; set; }

    /// <summary>
    /// Bounding box coordinates for this image on the page, if available.
    /// </summary>
    [JsonPropertyName("bounding_box")]
    public BoundingBox? BoundingBox { get; set; }
}

/// <summary>
/// Document format type classification for metadata categorization.
/// </summary>
public enum FormatType
{
    /// <summary>
    /// Unknown or unrecognized document format.
    /// </summary>
    Unknown,
    /// <summary>
    /// PDF document format.
    /// </summary>
    [JsonStringEnumMemberName("pdf")]
    Pdf,
    /// <summary>
    /// Excel spreadsheet format.
    /// </summary>
    [JsonStringEnumMemberName("excel")]
    Excel,
    /// <summary>
    /// Email message format.
    /// </summary>
    [JsonStringEnumMemberName("email")]
    Email,
    /// <summary>
    /// PowerPoint presentation format.
    /// </summary>
    [JsonStringEnumMemberName("pptx")]
    Pptx,
    /// <summary>
    /// Archive format (ZIP, TAR, etc.).
    /// </summary>
    [JsonStringEnumMemberName("archive")]
    Archive,
    /// <summary>
    /// Image format (PNG, JPEG, TIFF, etc.).
    /// </summary>
    [JsonStringEnumMemberName("image")]
    Image,
    /// <summary>
    /// XML document format.
    /// </summary>
    [JsonStringEnumMemberName("xml")]
    Xml,
    /// <summary>
    /// Plain text format.
    /// </summary>
    [JsonStringEnumMemberName("text")]
    Text,
    /// <summary>
    /// HTML document format.
    /// </summary>
    [JsonStringEnumMemberName("html")]
    Html,
    /// <summary>
    /// OCR-processed document format.
    /// </summary>
    [JsonStringEnumMemberName("ocr")]
    Ocr,
    /// <summary>
    /// CSV/TSV document format.
    /// </summary>
    [JsonStringEnumMemberName("csv")]
    Csv,
    /// <summary>
    /// BibTeX bibliography format.
    /// </summary>
    [JsonStringEnumMemberName("bibtex")]
    Bibtex,
    /// <summary>
    /// Citation file format (RIS, PubMed, EndNote).
    /// </summary>
    [JsonStringEnumMemberName("citation")]
    Citation,
    /// <summary>
    /// FictionBook (FB2) format.
    /// </summary>
    [JsonStringEnumMemberName("fiction_book")]
    FictionBook,
    /// <summary>
    /// dBASE (DBF) format.
    /// </summary>
    [JsonStringEnumMemberName("dbf")]
    Dbf,
    /// <summary>
    /// JATS (Journal Article Tag Suite) format.
    /// </summary>
    [JsonStringEnumMemberName("jats")]
    Jats,
    /// <summary>
    /// EPUB format.
    /// </summary>
    [JsonStringEnumMemberName("epub")]
    Epub,
    /// <summary>
    /// Outlook PST archive format.
    /// </summary>
    [JsonStringEnumMemberName("pst")]
    Pst,
}

/// <summary>
/// Container for format-specific metadata based on the document type.
/// </summary>
public sealed class FormatMetadata
{
    /// <summary>
    /// The detected document format type.
    /// </summary>
    public FormatType Type { get; set; } = FormatType.Unknown;

    /// <summary>
    /// PDF-specific metadata (if Type is Pdf).
    /// </summary>
    public PdfMetadata? Pdf { get; set; }

    /// <summary>
    /// Excel-specific metadata (if Type is Excel).
    /// </summary>
    public ExcelMetadata? Excel { get; set; }

    /// <summary>
    /// Email-specific metadata (if Type is Email).
    /// </summary>
    public EmailMetadata? Email { get; set; }

    /// <summary>
    /// PowerPoint-specific metadata (if Type is Pptx).
    /// </summary>
    public PptxMetadata? Pptx { get; set; }

    /// <summary>
    /// Archive-specific metadata (if Type is Archive).
    /// </summary>
    public ArchiveMetadata? Archive { get; set; }

    /// <summary>
    /// Image-specific metadata (if Type is Image).
    /// </summary>
    public ImageMetadata? Image { get; set; }

    /// <summary>
    /// XML-specific metadata (if Type is Xml).
    /// </summary>
    public XmlMetadata? Xml { get; set; }

    /// <summary>
    /// Plain text-specific metadata (if Type is Text).
    /// </summary>
    public TextMetadata? Text { get; set; }

    /// <summary>
    /// HTML-specific metadata (if Type is Html).
    /// </summary>
    public HtmlMetadata? Html { get; set; }

    /// <summary>
    /// OCR-specific metadata (if Type is Ocr).
    /// </summary>
    public OcrMetadata? Ocr { get; set; }

    /// <summary>
    /// CSV-specific metadata (if Type is Csv).
    /// </summary>
    public CsvMetadata? Csv { get; set; }

    /// <summary>
    /// BibTeX-specific metadata (if Type is Bibtex).
    /// </summary>
    public BibtexMetadata? Bibtex { get; set; }

    /// <summary>
    /// Citation-specific metadata (if Type is Citation).
    /// </summary>
    public CitationMetadata? Citation { get; set; }

    /// <summary>
    /// FictionBook-specific metadata (if Type is FictionBook).
    /// </summary>
    public FictionBookMetadata? FictionBook { get; set; }

    /// <summary>
    /// dBASE-specific metadata (if Type is Dbf).
    /// </summary>
    public DbfMetadata? Dbf { get; set; }

    /// <summary>
    /// JATS-specific metadata (if Type is Jats).
    /// </summary>
    public JatsMetadata? Jats { get; set; }

    /// <summary>
    /// EPUB-specific metadata (if Type is Epub).
    /// </summary>
    public EpubMetadata? Epub { get; set; }

    /// <summary>
    /// PST-specific metadata (if Type is Pst).
    /// </summary>
    public PstMetadata? Pst { get; set; }
}

/// <summary>
/// Document-level metadata extracted during processing.
/// </summary>
public sealed class Metadata
{
    /// <summary>
    /// Detected or specified language of the document content.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    /// <summary>
    /// Document subject or description.
    /// </summary>
    [JsonPropertyName("subject")]
    public string? Subject { get; set; }

    /// <summary>
    /// Primary author(s).
    /// </summary>
    [JsonPropertyName("authors")]
    public List<string>? Authors { get; set; }

    /// <summary>
    /// Keywords/tags (simple string keywords from document metadata).
    /// </summary>
    [JsonPropertyName("keywords")]
    public List<string>? Keywords { get; set; }

    /// <summary>
    /// Primary language (ISO 639 code).
    /// </summary>
    [JsonPropertyName("language")]
    public string? Language { get; set; }

    /// <summary>
    /// Creation timestamp (ISO 8601 format).
    /// </summary>
    [JsonPropertyName("created_at")]
    public string? CreatedAt { get; set; }

    /// <summary>
    /// Last modification timestamp (ISO 8601 format).
    /// </summary>
    [JsonPropertyName("modified_at")]
    public string? ModifiedAt { get; set; }

    /// <summary>
    /// User who created the document.
    /// </summary>
    [JsonPropertyName("created_by")]
    public string? CreatedBy { get; set; }

    /// <summary>
    /// User who last modified the document.
    /// </summary>
    [JsonPropertyName("modified_by")]
    public string? ModifiedBy { get; set; }

    /// <summary>
    /// Format-specific metadata (discriminated union, flattened in JSON).
    /// </summary>
    [JsonPropertyName("format")]
    public FormatMetadata? Format { get; set; }

    /// <summary>
    /// Image preprocessing metadata, if image preprocessing was applied.
    /// </summary>
    [JsonPropertyName("image_preprocessing")]
    public ImagePreprocessingMetadata? ImagePreprocessing { get; set; }

    /// <summary>
    /// JSON schema associated with the document, if available.
    /// </summary>
    [JsonPropertyName("json_schema")]
    public JsonNode? JsonSchema { get; set; }

    /// <summary>
    /// Error metadata if an error occurred during extraction.
    /// </summary>
    [JsonPropertyName("error")]
    public ErrorMetadata? Error { get; set; }

    /// <summary>
    /// Page structure information for paginated documents.
    /// </summary>
    [JsonPropertyName("pages")]
    public PageStructure? Pages { get; set; }

    /// <summary>
    /// Document category (from frontmatter or classification).
    /// </summary>
    [JsonPropertyName("category")]
    public string? Category { get; set; }

    /// <summary>
    /// Document tags (from frontmatter).
    /// </summary>
    [JsonPropertyName("tags")]
    public List<string>? Tags { get; set; }

    /// <summary>
    /// Document version string (from frontmatter).
    /// </summary>
    [JsonPropertyName("document_version")]
    public string? DocumentVersion { get; set; }

    /// <summary>
    /// Abstract or summary text (from frontmatter).
    /// </summary>
    [JsonPropertyName("abstract_text")]
    public string? AbstractText { get; set; }

    /// <summary>
    /// Output format identifier.
    /// </summary>
    [JsonPropertyName("output_format")]
    public string? OutputFormat { get; set; }

    /// <summary>
    /// Extraction duration in milliseconds (for benchmarking).
    /// Populated by batch extraction to provide per-file timing information.
    /// </summary>
    [JsonPropertyName("extraction_duration_ms")]
    public long? ExtractionDurationMs { get; set; }

    /// <summary>
    /// Additional untyped metadata fields captured as extension data.
    /// </summary>
    /// <remarks>
    /// Deprecated: Prefer using typed fields on <see cref="ExtractionResult"/> and <see cref="Metadata"/>
    /// instead of inserting into this map. Typed fields provide better cross-language
    /// compatibility and type safety. This field will be removed in a future major version.
    /// </remarks>
    [Obsolete(
        "Use typed fields on ExtractionResult and Metadata instead of Additional. " +
        "This field will be removed in v2.0.0.",
        error: false
    )]
    [JsonExtensionData]
    public JsonObject? Additional { get; set; }
}

/// <summary>
/// Metadata about image preprocessing operations applied during extraction.
/// </summary>
public sealed class ImagePreprocessingMetadata
{
    /// <summary>
    /// Original image dimensions [width, height] in pixels.
    /// </summary>
    [JsonPropertyName("original_dimensions")]
    public int[]? OriginalDimensions { get; set; }

    /// <summary>
    /// Original image DPI [horizontal, vertical].
    /// </summary>
    [JsonPropertyName("original_dpi")]
    public double[]? OriginalDpi { get; set; }

    /// <summary>
    /// Target DPI used for preprocessing.
    /// </summary>
    [JsonPropertyName("target_dpi")]
    public int TargetDpi { get; set; }

    /// <summary>
    /// Scale factor applied to the image.
    /// </summary>
    [JsonPropertyName("scale_factor")]
    public double ScaleFactor { get; set; }

    /// <summary>
    /// Whether the DPI was automatically adjusted.
    /// </summary>
    [JsonPropertyName("auto_adjusted")]
    public bool AutoAdjusted { get; set; }

    /// <summary>
    /// Final DPI after preprocessing.
    /// </summary>
    [JsonPropertyName("final_dpi")]
    public int FinalDpi { get; set; }

    /// <summary>
    /// New image dimensions [width, height] after preprocessing.
    /// </summary>
    [JsonPropertyName("new_dimensions")]
    public int[]? NewDimensions { get; set; }

    /// <summary>
    /// Resampling method used for resizing (e.g., "lanczos", "bilinear").
    /// </summary>
    [JsonPropertyName("resample_method")]
    public string? ResampleMethod { get; set; }

    /// <summary>
    /// Whether the image dimensions were clamped to a maximum.
    /// </summary>
    [JsonPropertyName("dimension_clamped")]
    public bool DimensionClamped { get; set; }

    /// <summary>
    /// Calculated DPI from image metadata, if available.
    /// </summary>
    [JsonPropertyName("calculated_dpi")]
    public int? CalculatedDpi { get; set; }

    /// <summary>
    /// Whether resizing was skipped (e.g., image already at target size).
    /// </summary>
    [JsonPropertyName("skipped_resize")]
    public bool SkippedResize { get; set; }

    /// <summary>
    /// Error message if resizing failed, if any.
    /// </summary>
    [JsonPropertyName("resize_error")]
    public string? ResizeError { get; set; }
}

/// <summary>
/// Metadata about an error that occurred during document extraction.
/// </summary>
public sealed class ErrorMetadata
{
    /// <summary>
    /// The type or category of the error.
    /// </summary>
    [JsonPropertyName("error_type")]
    public string ErrorType { get; set; } = string.Empty;

    /// <summary>
    /// Human-readable error message describing what went wrong.
    /// </summary>
    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;
}

/// <summary>
/// Represents an extracted keyword from keyword extraction algorithms (YAKE, RAKE).
/// </summary>
public sealed class ExtractedKeyword
{
    /// <summary>
    /// The keyword text.
    /// </summary>
    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;

    /// <summary>
    /// Relevance score (higher is better, algorithm-specific range).
    /// </summary>
    [JsonPropertyName("score")]
    public float Score { get; set; }

    /// <summary>
    /// Algorithm that extracted this keyword (e.g., "yake", "rake").
    /// </summary>
    [JsonPropertyName("algorithm")]
    public string Algorithm { get; set; } = string.Empty;

    /// <summary>
    /// Optional positions where keyword appears in text (character offsets).
    /// </summary>
    [JsonPropertyName("positions")]
    public List<int>? Positions { get; set; }
}

/// <summary>
/// A non-fatal warning from a processing pipeline stage.
/// </summary>
public class ProcessingWarning
{
    /// <summary>
    /// The pipeline stage that produced this warning.
    /// </summary>
    [JsonPropertyName("source")]
    public string Source { get; set; } = string.Empty;

    /// <summary>
    /// Human-readable description of the warning.
    /// </summary>
    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;
}

/// <summary>
/// Represents LLM usage metrics from structured extraction and other LLM-based processing.
/// </summary>
public class LlmUsage
{
    /// <summary>
    /// The LLM model name used (e.g., "gpt-4", "claude-3").
    /// </summary>
    [JsonPropertyName("model")]
    public string? Model { get; set; }

    /// <summary>
    /// The source component using the LLM.
    /// </summary>
    [JsonPropertyName("source")]
    public string? Source { get; set; }

    /// <summary>
    /// Number of input tokens used.
    /// </summary>
    [JsonPropertyName("input_tokens")]
    public long? InputTokens { get; set; }

    /// <summary>
    /// Number of output tokens generated.
    /// </summary>
    [JsonPropertyName("output_tokens")]
    public long? OutputTokens { get; set; }

    /// <summary>
    /// Total tokens used (input + output).
    /// </summary>
    [JsonPropertyName("total_tokens")]
    public long? TotalTokens { get; set; }

    /// <summary>
    /// Estimated cost of the LLM call in USD.
    /// </summary>
    [JsonPropertyName("estimated_cost")]
    public double? EstimatedCost { get; set; }

    /// <summary>
    /// Finish reason from the LLM (e.g., "stop", "length").
    /// </summary>
    [JsonPropertyName("finish_reason")]
    public string? FinishReason { get; set; }
}

/// <summary>
/// Metadata specific to PDF documents.
/// </summary>
public sealed class PdfMetadata
{
    /// <summary>
    /// Document title from PDF metadata.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    /// <summary>
    /// Document subject from PDF metadata.
    /// </summary>
    [JsonPropertyName("subject")]
    public string? Subject { get; set; }

    /// <summary>
    /// Document author from PDF metadata.
    /// </summary>
    [JsonPropertyName("author")]
    public string? Author { get; set; }

    /// <summary>
    /// Keywords from PDF metadata.
    /// </summary>
    [JsonPropertyName("keywords")]
    public List<string>? Keywords { get; set; }

    /// <summary>
    /// Creator application from PDF metadata.
    /// </summary>
    [JsonPropertyName("creator")]
    public string? Creator { get; set; }

    /// <summary>
    /// PDF producer application from PDF metadata.
    /// </summary>
    [JsonPropertyName("producer")]
    public string? Producer { get; set; }

    /// <summary>
    /// Document creation date from PDF metadata.
    /// </summary>
    [JsonPropertyName("creation_date")]
    public string? CreationDate { get; set; }

    /// <summary>
    /// Document modification date from PDF metadata.
    /// </summary>
    [JsonPropertyName("modification_date")]
    public string? ModificationDate { get; set; }

    /// <summary>
    /// Total number of pages in the PDF document.
    /// </summary>
    [JsonPropertyName("page_count")]
    public int? PageCount { get; set; }
}

/// <summary>
/// Metadata specific to Excel spreadsheet documents.
/// </summary>
public sealed class ExcelMetadata
{
    /// <summary>
    /// Number of sheets in the workbook.
    /// </summary>
    [JsonPropertyName("sheet_count")]
    public int SheetCount { get; set; }

    /// <summary>
    /// Names of the sheets in the workbook.
    /// </summary>
    [JsonPropertyName("sheet_names")]
    public List<string> SheetNames { get; set; } = new();
}

/// <summary>
/// Metadata specific to email messages.
/// </summary>
public sealed class EmailMetadata
{
    /// <summary>
    /// Sender email address.
    /// </summary>
    [JsonPropertyName("from_email")]
    public string? FromEmail { get; set; }

    /// <summary>
    /// Sender display name.
    /// </summary>
    [JsonPropertyName("from_name")]
    public string? FromName { get; set; }

    /// <summary>
    /// List of recipient email addresses.
    /// </summary>
    [JsonPropertyName("to_emails")]
    public List<string> ToEmails { get; set; } = new();

    /// <summary>
    /// List of CC recipient email addresses.
    /// </summary>
    [JsonPropertyName("cc_emails")]
    public List<string> CcEmails { get; set; } = new();

    /// <summary>
    /// List of BCC recipient email addresses.
    /// </summary>
    [JsonPropertyName("bcc_emails")]
    public List<string> BccEmails { get; set; } = new();

    /// <summary>
    /// Unique message identifier from the email headers.
    /// </summary>
    [JsonPropertyName("message_id")]
    public string? MessageId { get; set; }

    /// <summary>
    /// List of attachment filenames in the email.
    /// </summary>
    [JsonPropertyName("attachments")]
    public List<string>? Attachments { get; set; }
}

/// <summary>
/// Metadata specific to archive files (ZIP, TAR, etc.).
/// </summary>
public sealed class ArchiveMetadata
{
    /// <summary>
    /// Archive format name (e.g., "zip", "tar", "gz").
    /// </summary>
    [JsonPropertyName("format")]
    public string Format { get; set; } = string.Empty;

    /// <summary>
    /// Number of files in the archive.
    /// </summary>
    [JsonPropertyName("file_count")]
    public int FileCount { get; set; }

    /// <summary>
    /// List of file paths within the archive.
    /// </summary>
    [JsonPropertyName("file_list")]
    public List<string> FileList { get; set; } = new();

    /// <summary>
    /// Total uncompressed size in bytes.
    /// </summary>
    [JsonPropertyName("total_size")]
    public long TotalSize { get; set; }

    /// <summary>
    /// Total compressed size in bytes.
    /// </summary>
    [JsonPropertyName("compressed_size")]
    public long? CompressedSize { get; set; }
}

/// <summary>
/// Metadata specific to image files.
/// </summary>
public sealed class ImageMetadata
{
    /// <summary>
    /// Image width in pixels.
    /// </summary>
    [JsonPropertyName("width")]
    public uint Width { get; set; }

    /// <summary>
    /// Image height in pixels.
    /// </summary>
    [JsonPropertyName("height")]
    public uint Height { get; set; }

    /// <summary>
    /// Image format name (e.g., "PNG", "JPEG", "TIFF").
    /// </summary>
    [JsonPropertyName("format")]
    public string Format { get; set; } = string.Empty;

    /// <summary>
    /// EXIF metadata key-value pairs, if available.
    /// </summary>
    [JsonPropertyName("exif")]
    public Dictionary<string, string> Exif { get; set; } = new();
}

/// <summary>
/// Metadata specific to XML documents.
/// </summary>
public sealed class XmlMetadata
{
    /// <summary>
    /// Total number of XML elements in the document.
    /// </summary>
    [JsonPropertyName("element_count")]
    public int ElementCount { get; set; }

    /// <summary>
    /// List of unique XML element names found in the document.
    /// </summary>
    [JsonPropertyName("unique_elements")]
    public List<string> UniqueElements { get; set; } = new();
}

/// <summary>
/// Metadata specific to plain text and Markdown documents.
/// </summary>
public sealed class TextMetadata
{
    /// <summary>
    /// Total number of lines in the document.
    /// </summary>
    [JsonPropertyName("line_count")]
    public int LineCount { get; set; }

    /// <summary>
    /// Total number of words in the document.
    /// </summary>
    [JsonPropertyName("word_count")]
    public int WordCount { get; set; }

    /// <summary>
    /// Total number of characters in the document.
    /// </summary>
    [JsonPropertyName("character_count")]
    public int CharacterCount { get; set; }

    /// <summary>
    /// Headers found in the text document.
    /// </summary>
    [JsonPropertyName("headers")]
    public List<string>? Headers { get; set; }

    /// <summary>
    /// Links found in the text document, each as [text, url].
    /// </summary>
    [JsonPropertyName("links")]
    public List<List<string>>? Links { get; set; }

    /// <summary>
    /// Code blocks found in the text document, each as [language, code].
    /// </summary>
    [JsonPropertyName("code_blocks")]
    public List<List<string>>? CodeBlocks { get; set; }
}

/// <summary>
/// Metadata specific to HTML documents.
/// </summary>
public sealed class HtmlMetadata
{
    /// <summary>
    /// Document title from the HTML title element.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    /// <summary>
    /// Meta description from the HTML document.
    /// </summary>
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    /// <summary>
    /// Meta keywords from the HTML document.
    /// </summary>
    [JsonPropertyName("keywords")]
    public List<string> Keywords { get; set; } = new();

    /// <summary>
    /// Author from the HTML meta tags.
    /// </summary>
    [JsonPropertyName("author")]
    public string? Author { get; set; }

    /// <summary>
    /// Canonical URL from the HTML link element.
    /// </summary>
    [JsonPropertyName("canonical_url")]
    public string? CanonicalUrl { get; set; }

    /// <summary>
    /// Base href from the HTML base element.
    /// </summary>
    [JsonPropertyName("base_href")]
    public string? BaseHref { get; set; }

    /// <summary>
    /// Document language from the HTML lang attribute.
    /// </summary>
    [JsonPropertyName("language")]
    public string? Language { get; set; }

    /// <summary>
    /// Text direction from the HTML dir attribute (e.g., "ltr", "rtl").
    /// </summary>
    [JsonPropertyName("text_direction")]
    public string? TextDirection { get; set; }

    /// <summary>
    /// Open Graph metadata key-value pairs.
    /// </summary>
    [JsonPropertyName("open_graph")]
    public Dictionary<string, string> OpenGraph { get; set; } = new();

    /// <summary>
    /// Twitter Card metadata key-value pairs.
    /// </summary>
    [JsonPropertyName("twitter_card")]
    public Dictionary<string, string> TwitterCard { get; set; } = new();

    /// <summary>
    /// Additional meta tag key-value pairs.
    /// </summary>
    [JsonPropertyName("meta_tags")]
    public Dictionary<string, string> MetaTags { get; set; } = new();

    /// <summary>
    /// Headers/headings found in the HTML document.
    /// </summary>
    [JsonPropertyName("headers")]
    public List<HeaderMetadata> Headers { get; set; } = new();

    /// <summary>
    /// Links found in the HTML document.
    /// </summary>
    [JsonPropertyName("links")]
    public List<LinkMetadata> Links { get; set; } = new();

    /// <summary>
    /// Images found in the HTML document.
    /// </summary>
    [JsonPropertyName("images")]
    public List<HtmlImageMetadata> Images { get; set; } = new();

    /// <summary>
    /// Structured data (JSON-LD, etc.) found in the HTML document.
    /// </summary>
    [JsonPropertyName("structured_data")]
    public List<StructuredData> StructuredData { get; set; } = new();
}

/// <summary>
/// Represents a header/heading in an HTML document.
/// </summary>
public sealed class HeaderMetadata
{
    /// <summary>
    /// Heading level (1-6 corresponding to h1-h6).
    /// </summary>
    [JsonPropertyName("level")]
    public byte Level { get; set; }

    /// <summary>
    /// Text content of the heading.
    /// </summary>
    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;

    /// <summary>
    /// HTML id attribute of the heading element, if present.
    /// </summary>
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    /// <summary>
    /// Nesting depth of the heading in the document structure.
    /// </summary>
    [JsonPropertyName("depth")]
    public int Depth { get; set; }

    /// <summary>
    /// Character offset of this heading in the original HTML source.
    /// </summary>
    [JsonPropertyName("html_offset")]
    public int HtmlOffset { get; set; }
}

/// <summary>
/// Represents a link in an HTML document.
/// </summary>
public sealed class LinkMetadata
{
    /// <summary>
    /// The URL target of the link.
    /// </summary>
    [JsonPropertyName("href")]
    public string Href { get; set; } = string.Empty;

    /// <summary>
    /// Display text of the link.
    /// </summary>
    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;

    /// <summary>
    /// Title attribute of the link, if present.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    /// <summary>
    /// Classified link type (e.g., "internal", "external", "mailto", "other").
    /// </summary>
    [JsonPropertyName("link_type")]
    public string LinkType { get; set; } = "other";

    /// <summary>
    /// Rel attribute values of the link (e.g., "nofollow", "noopener").
    /// </summary>
    [JsonPropertyName("rel")]
    public List<string> Rel { get; set; } = new();

    /// <summary>
    /// Additional HTML attributes on the link element.
    /// Handles both array-of-arrays format [["k","v"]] from Rust and object format {"k":"v"} from C#.
    /// </summary>
    [JsonPropertyName("attributes")]
    [JsonConverter(typeof(Kreuzberg.AttributesDictionaryConverter))]
    public Dictionary<string, string> Attributes { get; set; } = new();
}

/// <summary>
/// Represents an image in an HTML document.
/// </summary>
public sealed class HtmlImageMetadata
{
    /// <summary>
    /// Image source URL.
    /// </summary>
    [JsonPropertyName("src")]
    public string Src { get; set; } = string.Empty;

    /// <summary>
    /// Alt text of the image, if present.
    /// </summary>
    [JsonPropertyName("alt")]
    public string? Alt { get; set; }

    /// <summary>
    /// Title attribute of the image, if present.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    /// <summary>
    /// Image dimensions [width, height] in pixels, if specified.
    /// </summary>
    [JsonPropertyName("dimensions")]
    public int[]? Dimensions { get; set; }

    /// <summary>
    /// Image type classification (e.g., "external", "inline", "data_uri").
    /// </summary>
    [JsonPropertyName("image_type")]
    public string ImageType { get; set; } = "external";

    /// <summary>
    /// Additional HTML attributes on the image element.
    /// Handles both array-of-arrays format [["k","v"]] from Rust and object format {"k":"v"} from C#.
    /// </summary>
    [JsonPropertyName("attributes")]
    [JsonConverter(typeof(Kreuzberg.AttributesDictionaryConverter))]
    public Dictionary<string, string> Attributes { get; set; } = new();
}

/// <summary>
/// Represents structured data (JSON-LD, etc.) in an HTML document.
/// </summary>
public sealed class StructuredData
{
    /// <summary>
    /// Type of structured data (e.g., "json_ld", "microdata", "rdfa").
    /// </summary>
    [JsonPropertyName("data_type")]
    public string DataType { get; set; } = "json_ld";

    /// <summary>
    /// Raw JSON string of the structured data.
    /// </summary>
    [JsonPropertyName("raw_json")]
    public string RawJson { get; set; } = string.Empty;

    /// <summary>
    /// Schema.org type (e.g., "Article", "Product"), if detected.
    /// </summary>
    [JsonPropertyName("schema_type")]
    public string? SchemaType { get; set; }
}

/// <summary>
/// Metadata specific to PowerPoint presentation documents.
/// </summary>
public sealed class PptxMetadata
{
    /// <summary>
    /// Total number of slides in the presentation.
    /// </summary>
    [JsonPropertyName("slide_count")]
    public int SlideCount { get; set; }

    /// <summary>
    /// Names of slides (if available).
    /// </summary>
    [JsonPropertyName("slide_names")]
    public List<string> SlideNames { get; set; } = new();

    /// <summary>
    /// Number of embedded images.
    /// </summary>
    [JsonPropertyName("image_count")]
    public int? ImageCount { get; set; }

    /// <summary>
    /// Number of tables.
    /// </summary>
    [JsonPropertyName("table_count")]
    public int? TableCount { get; set; }
}

/// <summary>
/// Metadata specific to OCR-processed documents.
/// </summary>
public sealed class OcrMetadata
{
    /// <summary>
    /// Language used for OCR processing.
    /// </summary>
    [JsonPropertyName("language")]
    public string Language { get; set; } = string.Empty;

    /// <summary>
    /// Page Segmentation Mode (PSM) used by the OCR engine.
    /// </summary>
    [JsonPropertyName("psm")]
    public int Psm { get; set; }

    /// <summary>
    /// Output format of the OCR results.
    /// </summary>
    [JsonPropertyName("output_format")]
    public string OutputFormat { get; set; } = string.Empty;

    /// <summary>
    /// Number of tables detected by OCR.
    /// </summary>
    [JsonPropertyName("table_count")]
    public int TableCount { get; set; }

    /// <summary>
    /// Number of rows in detected tables.
    /// </summary>
    [JsonPropertyName("table_rows")]
    public int? TableRows { get; set; }

    /// <summary>
    /// Number of columns in detected tables.
    /// </summary>
    [JsonPropertyName("table_cols")]
    public int? TableCols { get; set; }
}

/// <summary>
/// Represents the page structure of a document with metadata about pagination.
/// </summary>
public sealed class PageStructure
{
    /// <summary>
    /// Total number of pages/slides/sheets in the document.
    /// </summary>
    [JsonPropertyName("total_count")]
    public int TotalCount { get; set; }

    /// <summary>
    /// Type of paginated unit (e.g., page, slide, sheet).
    /// </summary>
    [JsonPropertyName("unit_type")]
    public string UnitType { get; set; } = string.Empty;

    /// <summary>
    /// Character offset boundaries for each page. Maps character ranges in the extracted content to page numbers.
    /// </summary>
    [JsonPropertyName("boundaries")]
    public List<PageBoundary>? Boundaries { get; set; }

    /// <summary>
    /// Detailed per-page metadata (optional, only when needed).
    /// </summary>
    [JsonPropertyName("pages")]
    public List<PageInfo>? Pages { get; set; }
}

/// <summary>
/// Represents a character offset boundary for a page.
/// </summary>
public sealed class PageBoundary
{
    /// <summary>
    /// Page number (1-indexed).
    /// </summary>
    [JsonPropertyName("page_number")]
    public int PageNumber { get; set; }

    /// <summary>
    /// Starting character offset in the document.
    /// </summary>
    [JsonPropertyName("byte_start")]
    public long ByteStart { get; set; }

    /// <summary>
    /// Ending byte offset in the document.
    /// </summary>
    [JsonPropertyName("byte_end")]
    public long ByteEnd { get; set; }
}

/// <summary>
/// Represents per-page metadata in a document.
/// </summary>
public sealed class PageInfo
{
    /// <summary>
    /// Page number (1-indexed).
    /// </summary>
    [JsonPropertyName("number")]
    public int Number { get; set; }

    /// <summary>
    /// Page title (usually for presentations).
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    /// <summary>
    /// Dimensions in points (PDF) or pixels (images): [width, height].
    /// </summary>
    [JsonPropertyName("dimensions")]
    public double[]? Dimensions { get; set; }

    /// <summary>
    /// Number of images on this page.
    /// </summary>
    [JsonPropertyName("image_count")]
    public int? ImageCount { get; set; }

    /// <summary>
    /// Number of tables on this page.
    /// </summary>
    [JsonPropertyName("table_count")]
    public int? TableCount { get; set; }

    /// <summary>
    /// Whether this page is hidden (e.g., in presentations).
    /// </summary>
    [JsonPropertyName("hidden")]
    public bool? Hidden { get; set; }

    /// <summary>
    /// Whether this page is blank (contains no meaningful content).
    /// </summary>
    [JsonPropertyName("is_blank")]
    public bool? IsBlank { get; set; }
}

/// <summary>
/// Execution provider for ONNX Runtime model inference.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<ExecutionProviderType>))]
public enum ExecutionProviderType
{
    /// <summary>
    /// Automatic provider selection (default).
    /// </summary>
    [JsonStringEnumMemberName("auto")]
    Auto,

    /// <summary>
    /// CPU execution provider.
    /// </summary>
    [JsonStringEnumMemberName("cpu")]
    Cpu,

    /// <summary>
    /// CoreML execution provider (Apple devices).
    /// </summary>
    [JsonStringEnumMemberName("coreml")]
    CoreML,

    /// <summary>
    /// CUDA execution provider (NVIDIA GPUs).
    /// </summary>
    [JsonStringEnumMemberName("cuda")]
    Cuda,

    /// <summary>
    /// TensorRT execution provider (NVIDIA GPUs).
    /// </summary>
    [JsonStringEnumMemberName("tensorrt")]
    TensorRT,
}

/// <summary>
/// Email extraction configuration.
/// </summary>
public sealed class EmailConfig
{
    /// <summary>
    /// Fallback code page for MSG email body decoding.
    /// </summary>
    [JsonPropertyName("msg_fallback_codepage")]
    public int? MsgFallbackCodepage { get; init; }
}

/// <summary>
/// Thread and concurrency limits for constrained environments.
/// </summary>
public sealed class ConcurrencyConfig
{
    /// <summary>
    /// Maximum number of threads for all internal thread pools.
    /// </summary>
    [JsonPropertyName("max_threads")]
    public int? MaxThreads { get; init; }
}

/// <summary>
/// Hardware acceleration configuration for ONNX Runtime models.
/// </summary>
public sealed class AccelerationConfig
{
    /// <summary>
    /// Execution provider for model inference (default: auto).
    /// </summary>
    [JsonPropertyName("provider")]
    public string? Provider { get; init; }

    /// <summary>
    /// GPU device ID for CUDA/TensorRT providers (default: 0).
    /// </summary>
    [JsonPropertyName("device_id")]
    public int? DeviceId { get; init; }
}

/// <summary>
/// Configuration for tree-sitter code analysis processing options.
/// </summary>
public sealed class TreeSitterProcessConfig
{
    /// <summary>
    /// Whether to extract code structure information.
    /// </summary>
    [JsonPropertyName("structure")]
    public bool? Structure { get; init; }

    /// <summary>
    /// Whether to extract import statements.
    /// </summary>
    [JsonPropertyName("imports")]
    public bool? Imports { get; init; }

    /// <summary>
    /// Whether to extract export statements.
    /// </summary>
    [JsonPropertyName("exports")]
    public bool? Exports { get; init; }

    /// <summary>
    /// Whether to extract comments.
    /// </summary>
    [JsonPropertyName("comments")]
    public bool? Comments { get; init; }

    /// <summary>
    /// Whether to extract docstrings.
    /// </summary>
    [JsonPropertyName("docstrings")]
    public bool? Docstrings { get; init; }

    /// <summary>
    /// Whether to extract symbol definitions.
    /// </summary>
    [JsonPropertyName("symbols")]
    public bool? Symbols { get; init; }

    /// <summary>
    /// Whether to extract diagnostics information.
    /// </summary>
    [JsonPropertyName("diagnostics")]
    public bool? Diagnostics { get; init; }

    /// <summary>
    /// Maximum size of code chunks for processing.
    /// </summary>
    [JsonPropertyName("chunk_max_size")]
    public int? ChunkMaxSize { get; init; }

    /// <summary>
    /// Content rendering mode for code extraction: "chunks" (default), "raw", or "structure".
    /// </summary>
    [JsonPropertyName("content_mode")]
    public string? ContentMode { get; init; }
}

/// <summary>
/// Configuration for tree-sitter language pack integration.
/// </summary>
public sealed class TreeSitterConfig
{
    /// <summary>
    /// Enable code intelligence processing. Default: true.
    /// </summary>
    [JsonPropertyName("enabled")]
    public bool? Enabled { get; init; }

    /// <summary>
    /// Directory for caching tree-sitter language packs.
    /// </summary>
    [JsonPropertyName("cache_dir")]
    public string? CacheDir { get; init; }

    /// <summary>
    /// List of specific languages to enable for tree-sitter parsing.
    /// </summary>
    [JsonPropertyName("languages")]
    public string[]? Languages { get; init; }

    /// <summary>
    /// List of language groups to enable for tree-sitter parsing.
    /// </summary>
    [JsonPropertyName("groups")]
    public string[]? Groups { get; init; }

    /// <summary>
    /// Processing options for tree-sitter code analysis.
    /// </summary>
    [JsonPropertyName("process")]
    public TreeSitterProcessConfig? Process { get; init; }
}

/// <summary>
/// Configuration for document extraction, controlling extraction behavior and features.
/// </summary>
public sealed class ExtractionConfig
{
    /// <summary>
    /// Whether to use caching for extraction results. Default is null (use server default).
    /// </summary>
    [JsonPropertyName("use_cache")]
    public bool? UseCache { get; init; }

    /// <summary>
    /// Whether to enable quality processing to improve extraction quality. Default is null.
    /// </summary>
    [JsonPropertyName("enable_quality_processing")]
    public bool? EnableQualityProcessing { get; init; }

    /// <summary>
    /// OCR configuration for handling scanned documents and images. If null, OCR is disabled.
    /// </summary>
    [JsonPropertyName("ocr")]
    public OcrConfig? Ocr { get; init; }

    /// <summary>
    /// Whether to force OCR processing even for documents with text. Default is false.
    /// </summary>
    [JsonPropertyName("force_ocr")]
    public bool? ForceOcr { get; init; }

    /// <summary>
    /// Whether to disable OCR entirely. When enabled, image files that would normally
    /// require OCR return empty content instead of raising errors. Default is false.
    /// </summary>
    [JsonPropertyName("disable_ocr")]
    public bool? DisableOcr { get; init; }

    /// <summary>
    /// List of 1-indexed page numbers to force OCR on. If null, uses the ForceOcr setting.
    /// </summary>
    [JsonPropertyName("force_ocr_pages")]
    public IReadOnlyList<long>? ForceOcrPages { get; init; }

    /// <summary>
    /// Text chunking configuration for splitting long documents. If null, chunking is disabled.
    /// </summary>
    [JsonPropertyName("chunking")]
    public ChunkingConfig? Chunking { get; init; }

    /// <summary>
    /// Image extraction configuration. If null, image extraction is disabled.
    /// </summary>
    [JsonPropertyName("images")]
    public ImageExtractionConfig? Images { get; init; }

    /// <summary>
    /// PDF-specific extraction options (password protection, metadata, etc.).
    /// </summary>
    [JsonPropertyName("pdf_options")]
    public PdfConfig? PdfOptions { get; init; }

    /// <summary>
    /// Token reduction configuration for reducing token counts in results.
    /// </summary>
    [JsonPropertyName("token_reduction")]
    public TokenReductionConfig? TokenReduction { get; init; }

    /// <summary>
    /// Language detection configuration. If null, language detection is disabled.
    /// </summary>
    [JsonPropertyName("language_detection")]
    public LanguageDetectionConfig? LanguageDetection { get; init; }

    /// <summary>
    /// Post-processor configuration for controlling which processors are enabled/disabled.
    /// </summary>
    [JsonPropertyName("postprocessor")]
    public PostProcessorConfig? Postprocessor { get; init; }

    /// <summary>
    /// HTML conversion options for HTML documents.
    /// </summary>
    [JsonPropertyName("html_options")]
    public HtmlConversionOptions? HtmlOptions { get; init; }

    /// <summary>
    /// Keyword extraction configuration.
    /// </summary>
    [JsonPropertyName("keywords")]
    public KeywordConfig? Keywords { get; init; }

    /// <summary>
    /// Page extraction and tracking configuration.
    /// </summary>
    [JsonPropertyName("pages")]
    public PageConfig? Pages { get; init; }

    /// <summary>
    /// Maximum number of concurrent extractions in batch operations. Default is null.
    /// </summary>
    [JsonPropertyName("max_concurrent_extractions")]
    public int? MaxConcurrentExtractions { get; init; }

    /// <summary>
    /// Security limits for archive extraction (max archive size, compression ratio, etc.).
    /// </summary>
    [JsonPropertyName("security_limits")]
    public SecurityLimitsConfig? SecurityLimits { get; init; }

    /// <summary>
    /// Content output format (plain, markdown, djot, html).
    /// Default: plain
    /// </summary>
    [JsonPropertyName("output_format")]
    public string? OutputFormat { get; init; }

    /// <summary>
    /// Result structure format (unified, element_based).
    /// Default: unified
    /// </summary>
    [JsonPropertyName("result_format")]
    public string? ResultFormat { get; init; }

    /// <summary>
    /// Whether to include structured document representation in the extraction result.
    /// When enabled, the result will contain a hierarchical tree of document nodes.
    /// Default: false
    /// </summary>
    [JsonPropertyName("include_document_structure")]
    public bool IncludeDocumentStructure { get; init; }

    /// <summary>
    /// Layout detection configuration for document page analysis.
    /// If null, layout detection is disabled.
    /// </summary>
    [JsonPropertyName("layout")]
    public LayoutDetectionConfig? Layout { get; init; }

    /// <summary>
    /// Hardware acceleration configuration for ONNX Runtime models.
    /// If null, uses default acceleration settings.
    /// </summary>
    [JsonPropertyName("acceleration")]
    public AccelerationConfig? Acceleration { get; init; }

    /// <summary>
    /// Email extraction configuration.
    /// If null, uses default email extraction settings.
    /// </summary>
    [JsonPropertyName("email")]
    public EmailConfig? Email { get; init; }

    /// <summary>
    /// Thread and concurrency limits for constrained environments.
    /// If null, uses default concurrency settings.
    /// </summary>
    [JsonPropertyName("concurrency")]
    public ConcurrencyConfig? Concurrency { get; init; }

    /// <summary>Cache namespace for tenant isolation.</summary>
    [JsonPropertyName("cache_namespace")]
    public string? CacheNamespace { get; init; }

    /// <summary>Per-request cache TTL in seconds (0 = skip cache).</summary>
    [JsonPropertyName("cache_ttl_secs")]
    public ulong? CacheTtlSecs { get; init; }

    /// <summary>Per-request extraction timeout in seconds. When exceeded, extraction is cancelled and an error is returned.</summary>
    [JsonPropertyName("extraction_timeout_secs")]
    public ulong? ExtractionTimeoutSecs { get; init; }

    /// <summary>
    /// Maximum recursion depth for archive extraction (ZIP, TAR, 7Z, GZIP). Default: 3.
    /// </summary>
    [JsonPropertyName("max_archive_depth")]
    public int? MaxArchiveDepth { get; init; }

    /// <summary>
    /// Tree-sitter language pack integration configuration.
    /// If null, tree-sitter processing is disabled.
    /// </summary>
    [JsonPropertyName("tree_sitter")]
    public TreeSitterConfig? TreeSitter { get; init; }

    /// <summary>
    /// Content filtering configuration for controlling header, footer, watermark,
    /// and repeating text inclusion in extraction results.
    /// If null, each extractor uses its default behavior.
    /// </summary>
    [JsonPropertyName("content_filter")]
    public ContentFilterConfig? ContentFilter { get; init; }

    /// <summary>
    /// HTML output configuration for styled HTML rendering.
    /// Controls CSS themes, custom stylesheets, and class prefixes when using HTML output format.
    /// If null, the default plain HTML renderer is used.
    /// </summary>
    [JsonPropertyName("html_output")]
    public HtmlOutputConfig? HtmlOutput { get; init; }

}

/// <summary>
/// Configuration for OCR (Optical Character Recognition) processing.
/// </summary>
public sealed class OcrConfig
{
    /// <summary>
    /// OCR backend to use (e.g., "tesseract", "paddle").
    /// </summary>
    [JsonPropertyName("backend")]
    public string? Backend { get; init; }

    /// <summary>
    /// Language for OCR processing (e.g., "eng", "fra", "deu").
    /// </summary>
    [JsonPropertyName("language")]
    public string? Language { get; init; }

    /// <summary>
    /// Tesseract-specific configuration, if using Tesseract backend.
    /// </summary>
    [JsonPropertyName("tesseract_config")]
    public TesseractConfig? TesseractConfig { get; init; }

    /// <summary>
    /// PaddleOCR-specific configuration, if using PaddleOCR backend.
    /// </summary>
    [JsonPropertyName("paddle_ocr_config")]
    public PaddleOcrConfig? PaddleOcrConfig { get; init; }

    /// <summary>
    /// Configuration for OCR element extraction.
    /// </summary>
    [JsonPropertyName("element_config")]
    public OcrElementConfig? ElementConfig { get; init; }
}

/// <summary>
/// Tesseract OCR engine-specific configuration options.
/// </summary>
public sealed class TesseractConfig
{
    /// <summary>
    /// OCR language (e.g., "eng", "fra", "deu").
    /// </summary>
    [JsonPropertyName("language")]
    public string? Language { get; init; }

    /// <summary>
    /// Page Segmentation Mode (PSM) for Tesseract (0-13).
    /// </summary>
    [JsonPropertyName("psm")]
    public int? Psm { get; init; }

    /// <summary>
    /// Output format specification.
    /// </summary>
    [JsonPropertyName("output_format")]
    public string? OutputFormat { get; init; }

    /// <summary>
    /// OCR Engine Mode (OEM) for Tesseract (0-3).
    /// </summary>
    [JsonPropertyName("oem")]
    public int? Oem { get; init; }

    /// <summary>
    /// Minimum confidence threshold for character recognition (0.0-1.0).
    /// </summary>
    [JsonPropertyName("min_confidence")]
    public double? MinConfidence { get; init; }

    /// <summary>
    /// Image preprocessing configuration for OCR preparation.
    /// </summary>
    [JsonPropertyName("preprocessing")]
    public ImagePreprocessingConfig? Preprocessing { get; init; }

    /// <summary>
    /// Whether to enable table detection and extraction.
    /// </summary>
    [JsonPropertyName("enable_table_detection")]
    public bool? EnableTableDetection { get; init; }

    /// <summary>
    /// Minimum confidence for table detection.
    /// </summary>
    [JsonPropertyName("table_min_confidence")]
    public double? TableMinConfidence { get; init; }

    /// <summary>
    /// Threshold for detecting table columns.
    /// </summary>
    [JsonPropertyName("table_column_threshold")]
    public int? TableColumnThreshold { get; init; }

    /// <summary>
    /// Ratio threshold for detecting table rows.
    /// </summary>
    [JsonPropertyName("table_row_threshold_ratio")]
    public double? TableRowThresholdRatio { get; init; }

    /// <summary>
    /// Whether to use caching for OCR results.
    /// </summary>
    [JsonPropertyName("use_cache")]
    public bool? UseCache { get; init; }

    /// <summary>
    /// Whether to classify using pre-adapted templates.
    /// </summary>
    [JsonPropertyName("classify_use_pre_adapted_templates")]
    public bool? ClassifyUsePreAdaptedTemplates { get; init; }

    /// <summary>
    /// Whether to use language model n-gram.
    /// </summary>
    [JsonPropertyName("language_model_ngram_on")]
    public bool? LanguageModelNgramOn { get; init; }

    /// <summary>
    /// Tesseract parameter: don't reject good words in blocks.
    /// </summary>
    [JsonPropertyName("tessedit_dont_blkrej_good_wds")]
    public bool? TesseditDontBlkrejGoodWds { get; init; }

    /// <summary>
    /// Tesseract parameter: don't reject good words in rows.
    /// </summary>
    [JsonPropertyName("tessedit_dont_rowrej_good_wds")]
    public bool? TesseditDontRowrejGoodWds { get; init; }

    /// <summary>
    /// Tesseract parameter: enable dictionary correction.
    /// </summary>
    [JsonPropertyName("tessedit_enable_dict_correction")]
    public bool? TesseditEnableDictCorrection { get; init; }

    /// <summary>
    /// Whitelist of characters for OCR recognition.
    /// </summary>
    [JsonPropertyName("tessedit_char_whitelist")]
    public string? TesseditCharWhitelist { get; init; }

    /// <summary>
    /// Blacklist of characters to exclude from OCR recognition.
    /// </summary>
    [JsonPropertyName("tessedit_char_blacklist")]
    public string? TesseditCharBlacklist { get; init; }

    /// <summary>
    /// Tesseract parameter: use primary params model.
    /// </summary>
    [JsonPropertyName("tessedit_use_primary_params_model")]
    public bool? TesseditUsePrimaryParamsModel { get; init; }

    /// <summary>
    /// Tesseract parameter: space size is variable.
    /// </summary>
    [JsonPropertyName("textord_space_size_is_variable")]
    public bool? TextordSpaceSizeIsVariable { get; init; }

    /// <summary>
    /// Thresholding method for image binarization.
    /// </summary>
    [JsonPropertyName("thresholding_method")]
    public bool? ThresholdingMethod { get; init; }
}

/// <summary>
/// Configuration for image preprocessing operations (rotation, deskew, denoise, etc.).
/// </summary>
public sealed class ImagePreprocessingConfig
{
    /// <summary>
    /// Target dots-per-inch (DPI) for image upscaling/downscaling.
    /// </summary>
    [JsonPropertyName("target_dpi")]
    public int? TargetDpi { get; init; }

    /// <summary>
    /// Whether to automatically rotate images to correct orientation.
    /// </summary>
    [JsonPropertyName("auto_rotate")]
    public bool? AutoRotate { get; init; }

    /// <summary>
    /// Whether to deskew (rotate to correct skew) the image.
    /// </summary>
    [JsonPropertyName("deskew")]
    public bool? Deskew { get; init; }

    /// <summary>
    /// Whether to apply denoising to reduce image noise.
    /// </summary>
    [JsonPropertyName("denoise")]
    public bool? Denoise { get; init; }

    /// <summary>
    /// Whether to enhance image contrast.
    /// </summary>
    [JsonPropertyName("contrast_enhance")]
    public bool? ContrastEnhance { get; init; }

    /// <summary>
    /// Binarization method for converting to black and white.
    /// </summary>
    [JsonPropertyName("binarization_method")]
    public string? BinarizationMethod { get; init; }

    /// <summary>
    /// Whether to invert image colors.
    /// </summary>
    [JsonPropertyName("invert_colors")]
    public bool? InvertColors { get; init; }
}

/// <summary>
/// Configuration for embedding generation using ONNX Runtime models.
/// </summary>
public sealed class EmbeddingConfig
{
    /// <summary>
    /// Embedding model name or preset identifier (e.g., "default", "balanced", "compact").
    /// </summary>
    [JsonPropertyName("model")]
    public string? Model { get; init; }

    /// <summary>
    /// Number of texts to process simultaneously during embedding generation.
    /// Higher values use more memory but may be faster.
    /// </summary>
    [JsonPropertyName("batch_size")]
    public int? BatchSize { get; init; }

    /// <summary>
    /// Whether to normalize embedding vectors to unit length.
    /// Recommended for cosine similarity calculations.
    /// </summary>
    [JsonPropertyName("normalize")]
    public bool? Normalize { get; init; }

    /// <summary>
    /// Output dimensionality of the embedding vectors.
    /// Model-dependent; typically 384, 768, 1536, or higher.
    /// </summary>
    [JsonPropertyName("dimensions")]
    public int? Dimensions { get; init; }

    /// <summary>
    /// Whether to cache embeddings for identical text chunks.
    /// Improves performance when processing duplicate content.
    /// </summary>
    [JsonPropertyName("use_cache")]
    public bool? UseCache { get; init; }

    /// <summary>
    /// Hardware acceleration configuration for ONNX Runtime models.
    /// If null, uses default acceleration settings.
    /// </summary>
    [JsonPropertyName("acceleration")]
    public AccelerationConfig? Acceleration { get; init; }
}

/// <summary>
/// Configuration for text chunking (splitting long documents into smaller pieces).
/// </summary>
public sealed class ChunkingConfig
{
    /// <summary>
    /// Maximum number of characters per chunk.
    /// </summary>
    [JsonPropertyName("max_chars")]
    public int? MaxChars { get; init; }

    /// <summary>
    /// Maximum character overlap between consecutive chunks.
    /// </summary>
    [JsonPropertyName("max_overlap")]
    public int? MaxOverlap { get; init; }

    /// <summary>
    /// Chunk size (alternative to max_chars).
    /// </summary>
    [JsonPropertyName("chunk_size")]
    public int? ChunkSize { get; init; }

    /// <summary>
    /// Character overlap between chunks.
    /// </summary>
    [JsonPropertyName("chunk_overlap")]
    public int? ChunkOverlap { get; init; }

    /// <summary>
    /// Named preset for chunking strategy (e.g., "default", "semantic").
    /// </summary>
    [JsonPropertyName("preset")]
    public string? Preset { get; init; }

    /// <summary>
    /// Type of chunker to use: "text" (default), "markdown", "yaml", or "semantic".
    /// Set to "semantic" for topic-aware chunking that works out of the box with
    /// sensible defaults (max_chars=1000, overlap=200, topic_threshold=0.75).
    /// No other parameters needed.
    /// </summary>
    [JsonPropertyName("chunker_type")]
    public string? ChunkerType { get; init; }

    /// <summary>
    /// Cosine similarity threshold for semantic topic detection (0.0-1.0).
    /// Optional, defaults to 0.75. Only relevant when ChunkerType is "semantic"
    /// and embeddings are configured. Rarely needs tuning.
    /// </summary>
    [JsonPropertyName("topic_threshold")]
    public double? TopicThreshold { get; init; }

    /// <summary>
    /// Embedding configuration for vector generation.
    /// </summary>
    [JsonPropertyName("embedding")]
    public EmbeddingConfig? Embedding { get; init; }

    /// <summary>
    /// Whether chunking is enabled.
    /// </summary>
    [JsonPropertyName("enabled")]
    public bool? Enabled { get; init; }

    /// <summary>
    /// Chunk sizing configuration. Controls how chunk size is measured.
    /// Use <c>new { type = "characters" }</c> for character-based (default)
    /// or <c>new { type = "tokenizer", model = "Xenova/gpt-4o" }</c> for token-based.
    /// </summary>
    [JsonPropertyName("sizing")]
    public ChunkSizingConfig? Sizing { get; init; }

    /// <summary>
    /// When true, prepends the heading context to each chunk for improved retrieval.
    /// Default: false.
    /// </summary>
    [JsonPropertyName("prepend_heading_context")]
    public bool? PrependHeadingContext { get; init; }
}

/// <summary>
/// Configuration for how chunk size is measured.
/// </summary>
public sealed class ChunkSizingConfig
{
    /// <summary>
    /// Sizing type: "characters" (default) or "tokenizer".
    /// </summary>
    [JsonPropertyName("type")]
    public string? Type { get; init; }

    /// <summary>
    /// HuggingFace model ID for tokenizer sizing (e.g., "Xenova/gpt-4o").
    /// Only used when Type is "tokenizer".
    /// </summary>
    [JsonPropertyName("model")]
    public string? Model { get; init; }

    /// <summary>
    /// Optional cache directory for tokenizer files.
    /// </summary>
    [JsonPropertyName("cache_dir")]
    public string? CacheDir { get; init; }
}

/// <summary>
/// Configuration for image extraction from documents.
/// </summary>
public sealed class ImageExtractionConfig
{
    /// <summary>
    /// Whether to extract images from documents.
    /// </summary>
    [JsonPropertyName("extract_images")]
    public bool? ExtractImages { get; init; }

    /// <summary>
    /// Target DPI for extracted images.
    /// </summary>
    [JsonPropertyName("target_dpi")]
    public int? TargetDpi { get; init; }

    /// <summary>
    /// Maximum dimension (width or height) for extracted images.
    /// </summary>
    [JsonPropertyName("max_image_dimension")]
    public int? MaxImageDimension { get; init; }

    /// <summary>
    /// Whether to automatically adjust DPI based on image characteristics.
    /// </summary>
    [JsonPropertyName("auto_adjust_dpi")]
    public bool? AutoAdjustDpi { get; init; }

    /// <summary>
    /// Minimum allowed DPI for extracted images.
    /// </summary>
    [JsonPropertyName("min_dpi")]
    public int? MinDpi { get; init; }

    /// <summary>
    /// Maximum allowed DPI for extracted images.
    /// </summary>
    [JsonPropertyName("max_dpi")]
    public int? MaxDpi { get; init; }
}

/// <summary>
/// Font configuration for PDF processing.
/// </summary>
public sealed class FontConfig
{
    /// <summary>
    /// Whether font fallback is enabled for handling missing fonts.
    /// </summary>
    [JsonPropertyName("font_fallback_enabled")]
    public bool? FontFallbackEnabled { get; init; }

    /// <summary>
    /// Directory path for custom fonts.
    /// </summary>
    [JsonPropertyName("font_dir")]
    public string? FontDir { get; init; }
}

/// <summary>
/// Hierarchy detection configuration for PDF document structure.
/// </summary>
public sealed class HierarchyConfig
{
    /// <summary>
    /// Whether hierarchy detection is enabled.
    /// </summary>
    [JsonPropertyName("enabled")]
    public bool? Enabled { get; init; }

    /// <summary>
    /// Number of k clusters for hierarchy detection.
    /// </summary>
    [JsonPropertyName("k_clusters")]
    public int? KClusters { get; init; }

    /// <summary>
    /// Whether to include bounding box information in hierarchy output.
    /// </summary>
    [JsonPropertyName("include_bbox")]
    public bool? IncludeBbox { get; init; }

    /// <summary>
    /// OCR coverage threshold for hierarchy detection (0.0-1.0).
    /// </summary>
    [JsonPropertyName("ocr_coverage_threshold")]
    public float? OcrCoverageThreshold { get; init; }
}

/// <summary>
/// PDF-specific extraction configuration.
/// </summary>
public sealed class PdfConfig
{
    /// <summary>
    /// Whether to extract images from PDF documents.
    /// </summary>
    [JsonPropertyName("extract_images")]
    public bool? ExtractImages { get; init; }

    /// <summary>
    /// List of passwords to try for encrypted PDFs.
    /// </summary>
    [JsonPropertyName("passwords")]
    public List<string>? Passwords { get; init; }

    /// <summary>
    /// Whether to extract document metadata (title, author, etc.).
    /// </summary>
    [JsonPropertyName("extract_metadata")]
    public bool? ExtractMetadata { get; init; }

    /// <summary>
    /// Font configuration for PDF text extraction.
    /// </summary>
    [JsonPropertyName("font_config")]
    public FontConfig? FontConfig { get; init; }

    /// <summary>
    /// Hierarchy detection configuration for document structure analysis.
    /// </summary>
    [JsonPropertyName("hierarchy")]
    public HierarchyConfig? Hierarchy { get; init; }

    /// <summary>
    /// Whether to extract annotations from PDF documents.
    /// </summary>
    [JsonPropertyName("extract_annotations")]
    public bool? ExtractAnnotations { get; init; }

    /// <summary>
    /// Top margin fraction (0.0-0.5) for filtering header content.
    /// </summary>
    [JsonPropertyName("top_margin_fraction")]
    public float? TopMarginFraction { get; init; }

    /// <summary>
    /// Bottom margin fraction (0.0-0.5) for filtering footer content.
    /// </summary>
    [JsonPropertyName("bottom_margin_fraction")]
    public float? BottomMarginFraction { get; init; }

    /// <summary>
    /// Whether to allow single-column tables in PDF extraction output.
    /// Default: false
    /// </summary>
    [JsonPropertyName("allow_single_column_tables")]
    public bool? AllowSingleColumnTables { get; init; }
}

/// <summary>
/// Configuration for token reduction to minimize token counts in results.
/// </summary>
public sealed class TokenReductionConfig
{
    /// <summary>
    /// Token reduction mode (e.g., "none", "aggressive", "balanced").
    /// </summary>
    [JsonPropertyName("mode")]
    public string? Mode { get; init; }

    /// <summary>
    /// Whether to preserve important words during token reduction.
    /// </summary>
    [JsonPropertyName("preserve_important_words")]
    public bool? PreserveImportantWords { get; init; }
}

/// <summary>
/// Configuration for language detection in extracted text.
/// </summary>
public sealed class LanguageDetectionConfig
{
    /// <summary>
    /// Whether language detection is enabled.
    /// </summary>
    [JsonPropertyName("enabled")]
    public bool? Enabled { get; init; }

    /// <summary>
    /// Minimum confidence threshold for language detection (0.0-1.0).
    /// </summary>
    [JsonPropertyName("min_confidence")]
    public double? MinConfidence { get; init; }

    /// <summary>
    /// Whether to detect multiple languages in the document.
    /// </summary>
    [JsonPropertyName("detect_multiple")]
    public bool? DetectMultiple { get; init; }
}

/// <summary>
/// Configuration for custom post-processor execution.
/// </summary>
public sealed class PostProcessorConfig
{
    /// <summary>
    /// Whether post-processing is enabled.
    /// </summary>
    [JsonPropertyName("enabled")]
    public bool? Enabled { get; init; }

    /// <summary>
    /// List of post-processor names to enable.
    /// </summary>
    [JsonPropertyName("enabled_processors")]
    public List<string>? EnabledProcessors { get; init; }

    /// <summary>
    /// List of post-processor names to disable.
    /// </summary>
    [JsonPropertyName("disabled_processors")]
    public List<string>? DisabledProcessors { get; init; }
}

/// <summary>
/// Cross-extractor content filtering configuration.
///
/// Controls whether "furniture" content (headers, footers, page numbers,
/// watermarks, repeating text) is included in or stripped from extraction
/// results. Applies across all extractors (PDF, DOCX, RTF, ODT, HTML, etc.)
/// with format-specific implementation.
/// </summary>
public sealed class ContentFilterConfig
{
    /// <summary>
    /// Include running headers in extraction output.
    /// Default: false (headers are stripped or excluded).
    /// </summary>
    [JsonPropertyName("include_headers")]
    public bool? IncludeHeaders { get; init; }

    /// <summary>
    /// Include running footers in extraction output.
    /// Default: false (footers are stripped or excluded).
    /// </summary>
    [JsonPropertyName("include_footers")]
    public bool? IncludeFooters { get; init; }

    /// <summary>
    /// Enable cross-page repeating text detection and removal.
    /// When true (default), text that repeats verbatim across a supermajority
    /// of pages is stripped.
    /// Default: true.
    /// </summary>
    [JsonPropertyName("strip_repeating_text")]
    public bool? StripRepeatingText { get; init; }

    /// <summary>
    /// Include watermark text in extraction output.
    /// Default: false (watermarks are stripped).
    /// </summary>
    [JsonPropertyName("include_watermarks")]
    public bool? IncludeWatermarks { get; init; }
}

/// <summary>
/// Configuration for styled HTML output rendering.
///
/// Controls CSS themes, custom stylesheets, class prefixes, and whether CSS
/// is embedded in the output when using HTML output format.
/// </summary>
public sealed class HtmlOutputConfig
{
    /// <summary>
    /// Inline CSS string injected into the output after the theme stylesheet.
    /// </summary>
    [JsonPropertyName("css")]
    public string? Css { get; init; }

    /// <summary>
    /// Path to a CSS file loaded at renderer construction time.
    /// </summary>
    [JsonPropertyName("css_file")]
    public string? CssFile { get; init; }

    /// <summary>
    /// Built-in colour/typography theme (default, github, dark, light, unstyled).
    /// </summary>
    [JsonPropertyName("theme")]
    public string? Theme { get; init; }

    /// <summary>
    /// CSS class prefix applied to every emitted class name. Default: "kb-".
    /// </summary>
    [JsonPropertyName("class_prefix")]
    public string? ClassPrefix { get; init; }

    /// <summary>
    /// When true (default), write the resolved CSS into a style block in the output.
    /// Set to false to emit only structural markup.
    /// </summary>
    [JsonPropertyName("embed_css")]
    public bool? EmbedCss { get; init; }
}

/// <summary>
/// Configuration for HTML to text conversion with fine-grained formatting control.
/// </summary>
public sealed class HtmlConversionOptions
{
    /// <summary>
    /// Style for markdown headings (e.g., "setext", "atx").
    /// </summary>
    public string? HeadingStyle { get; init; }

    /// <summary>
    /// Type of indentation for lists (e.g., "space", "tab").
    /// </summary>
    public string? ListIndentType { get; init; }

    /// <summary>
    /// Width of list indentation.
    /// </summary>
    public int? ListIndentWidth { get; init; }

    /// <summary>
    /// Bullet style for unordered lists (e.g., "-", "*", "+").
    /// </summary>
    public string? Bullets { get; init; }

    /// <summary>
    /// Symbol for strong/emphasis text.
    /// </summary>
    public string? StrongEmSymbol { get; init; }

    /// <summary>
    /// Whether to escape asterisks in output.
    /// </summary>
    public bool? EscapeAsterisks { get; init; }

    /// <summary>
    /// Whether to escape underscores in output.
    /// </summary>
    public bool? EscapeUnderscores { get; init; }

    /// <summary>
    /// Whether to escape miscellaneous characters.
    /// </summary>
    public bool? EscapeMisc { get; init; }

    /// <summary>
    /// Whether to escape ASCII control characters.
    /// </summary>
    public bool? EscapeAscii { get; init; }

    /// <summary>
    /// Language for code blocks syntax highlighting.
    /// </summary>
    public string? CodeLanguage { get; init; }

    /// <summary>
    /// Whether to automatically convert URLs to hyperlinks.
    /// </summary>
    public bool? Autolinks { get; init; }

    /// <summary>
    /// Default title for documents without one.
    /// </summary>
    public string? DefaultTitle { get; init; }

    /// <summary>
    /// Whether to use HTML line breaks in tables.
    /// </summary>
    public bool? BrInTables { get; init; }

    /// <summary>
    /// Whether to use hOCR spatial tables.
    /// </summary>
    public bool? HocrSpatialTables { get; init; }

    /// <summary>
    /// Highlighting style for code blocks.
    /// </summary>
    public string? HighlightStyle { get; init; }

    /// <summary>
    /// Whether to extract and include document metadata.
    /// </summary>
    public bool? ExtractMetadata { get; init; }

    /// <summary>
    /// Whitespace handling mode (e.g., "preserve", "collapse").
    /// </summary>
    public string? WhitespaceMode { get; init; }

    /// <summary>
    /// Whether to strip newlines from output.
    /// </summary>
    public bool? StripNewlines { get; init; }

    /// <summary>
    /// Whether to wrap text output.
    /// </summary>
    public bool? Wrap { get; init; }

    /// <summary>
    /// Text wrapping width in characters.
    /// </summary>
    public int? WrapWidth { get; init; }

    /// <summary>
    /// Whether to convert HTML as inline content.
    /// </summary>
    public bool? ConvertAsInline { get; init; }

    /// <summary>
    /// Symbol for subscript text.
    /// </summary>
    public string? SubSymbol { get; init; }

    /// <summary>
    /// Symbol for superscript text.
    /// </summary>
    public string? SupSymbol { get; init; }

    /// <summary>
    /// Newline style for output (e.g., "lf", "crlf").
    /// </summary>
    public string? NewlineStyle { get; init; }

    /// <summary>
    /// Style for code blocks (e.g., "fenced", "indented").
    /// </summary>
    public string? CodeBlockStyle { get; init; }

    /// <summary>
    /// List of HTML elements to keep inline images in.
    /// </summary>
    public List<string>? KeepInlineImagesIn { get; init; }

    /// <summary>
    /// Character encoding for output.
    /// </summary>
    public string? Encoding { get; init; }

    /// <summary>
    /// Whether to include debug information in output.
    /// </summary>
    public bool? Debug { get; init; }

    /// <summary>
    /// HTML tags to strip from output.
    /// </summary>
    public List<string>? StripTags { get; init; }

    /// <summary>
    /// HTML tags to preserve in output.
    /// </summary>
    public List<string>? PreserveTags { get; init; }

    /// <summary>
    /// HTML preprocessing configuration.
    /// </summary>
    public HtmlPreprocessingOptions? Preprocessing { get; init; }
}

/// <summary>
/// Configuration for preprocessing HTML before conversion.
/// </summary>
public sealed class HtmlPreprocessingOptions
{
    /// <summary>
    /// Whether preprocessing is enabled.
    /// </summary>
    public bool? Enabled { get; init; }

    /// <summary>
    /// Named preset for preprocessing strategy.
    /// </summary>
    public string? Preset { get; init; }

    /// <summary>
    /// Whether to remove navigation elements.
    /// </summary>
    public bool? RemoveNavigation { get; init; }

    /// <summary>
    /// Whether to remove form elements.
    /// </summary>
    public bool? RemoveForms { get; init; }
}

/// <summary>
/// Keyword extraction algorithms supported by Kreuzberg.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<KeywordAlgorithm>))]
public enum KeywordAlgorithm
{
    /// <summary>
    /// YAKE (Yet Another Keyword Extractor) algorithm.
    /// </summary>
    [JsonStringEnumMemberName("yake")]
    Yake,

    /// <summary>
    /// RAKE (Rapid Automatic Keyword Extraction) algorithm.
    /// </summary>
    [JsonStringEnumMemberName("rake")]
    Rake,
}

/// <summary>
/// Configuration for keyword extraction from documents.
/// </summary>
public sealed class KeywordConfig
{
    /// <summary>
    /// Keyword extraction algorithm to use (e.g., "yake", "rake").
    /// </summary>
    [JsonPropertyName("algorithm")]
    public string? Algorithm { get; init; }

    /// <summary>
    /// Maximum number of keywords to extract.
    /// </summary>
    [JsonPropertyName("max_keywords")]
    public int? MaxKeywords { get; init; }

    /// <summary>
    /// Minimum relevance score threshold for keywords (0.0-1.0).
    /// </summary>
    [JsonPropertyName("min_score")]
    public double? MinScore { get; init; }

    /// <summary>
    /// N-gram range for keyword extraction [min, max].
    /// </summary>
    [JsonPropertyName("ngram_range")]
    public List<int>? NgramRange { get; init; }

    /// <summary>
    /// Language for keyword extraction (e.g., "en", "fr", "de").
    /// </summary>
    [JsonPropertyName("language")]
    public string? Language { get; init; }

    /// <summary>
    /// Algorithm-specific parameters for YAKE.
    /// </summary>
    [JsonPropertyName("yake_params")]
    public YakeParamsConfig? YakeParams { get; init; }

    /// <summary>
    /// Algorithm-specific parameters for RAKE.
    /// </summary>
    [JsonPropertyName("rake_params")]
    public RakeParamsConfig? RakeParams { get; init; }
}

/// <summary>
/// Algorithm-specific parameters for the YAKE keyword extraction algorithm.
/// </summary>
public sealed class YakeParamsConfig
{
    /// <summary>
    /// Window size for co-occurrence statistics.
    /// </summary>
    [JsonPropertyName("window_size")]
    public int? WindowSize { get; init; }
}

/// <summary>
/// Algorithm-specific parameters for the RAKE keyword extraction algorithm.
/// </summary>
public sealed class RakeParamsConfig
{
    /// <summary>
    /// Minimum word length for keyword candidates.
    /// </summary>
    [JsonPropertyName("min_word_length")]
    public int? MinWordLength { get; init; }

    /// <summary>
    /// Maximum number of words per extracted phrase.
    /// </summary>
    [JsonPropertyName("max_words_per_phrase")]
    public int? MaxWordsPerPhrase { get; init; }
}

/// <summary>
/// Security limits for archive and document extraction.
///
/// Controls thresholds to prevent resource exhaustion attacks such as
/// decompression bombs, deeply nested archives, and oversized content.
/// </summary>
public sealed class SecurityLimitsConfig
{
    /// <summary>
    /// Maximum allowed archive size in bytes.
    /// </summary>
    [JsonPropertyName("max_archive_size")]
    public long? MaxArchiveSize { get; init; }

    /// <summary>
    /// Maximum allowed compression ratio (uncompressed / compressed).
    /// </summary>
    [JsonPropertyName("max_compression_ratio")]
    public long? MaxCompressionRatio { get; init; }

    /// <summary>
    /// Maximum number of files allowed inside an archive.
    /// </summary>
    [JsonPropertyName("max_files_in_archive")]
    public long? MaxFilesInArchive { get; init; }

    /// <summary>
    /// Maximum nesting depth for recursive archive extraction.
    /// </summary>
    [JsonPropertyName("max_nesting_depth")]
    public long? MaxNestingDepth { get; init; }

    /// <summary>
    /// Maximum length of a single XML/HTML entity.
    /// </summary>
    [JsonPropertyName("max_entity_length")]
    public long? MaxEntityLength { get; init; }

    /// <summary>
    /// Maximum total content size in bytes after extraction.
    /// </summary>
    [JsonPropertyName("max_content_size")]
    public long? MaxContentSize { get; init; }

    /// <summary>
    /// Maximum number of processing iterations.
    /// </summary>
    [JsonPropertyName("max_iterations")]
    public long? MaxIterations { get; init; }

    /// <summary>
    /// Maximum XML document nesting depth.
    /// </summary>
    [JsonPropertyName("max_xml_depth")]
    public long? MaxXmlDepth { get; init; }

    /// <summary>
    /// Maximum number of cells in a single table.
    /// </summary>
    [JsonPropertyName("max_table_cells")]
    public long? MaxTableCells { get; init; }
}

/// <summary>
/// Configuration for page tracking and extraction during document processing.
/// </summary>
public sealed class PageConfig
{
    /// <summary>
    /// Whether to extract and track page information.
    /// </summary>
    [JsonPropertyName("extract_pages")]
    public bool? ExtractPages { get; init; }

    /// <summary>
    /// Whether to insert page markers in the extracted content.
    /// </summary>
    [JsonPropertyName("insert_page_markers")]
    public bool? InsertPageMarkers { get; init; }

    /// <summary>
    /// Format for page markers (e.g., "[PAGE_N]", "Page: N").
    /// </summary>
    [JsonPropertyName("marker_format")]
    public string? MarkerFormat { get; init; }
}

/// <summary>
/// Per-file extraction configuration overrides for batch processing.
/// All fields are nullable — null means "use the batch-level default."
/// Batch-level concerns (caching, concurrency, acceleration, security) are excluded.
/// </summary>
public sealed class FileExtractionConfig
{
    /// <summary>Override quality processing for this file.</summary>
    [JsonPropertyName("enable_quality_processing")]
    public bool? EnableQualityProcessing { get; init; }

    /// <summary>Override OCR configuration for this file.</summary>
    [JsonPropertyName("ocr")]
    public OcrConfig? Ocr { get; init; }

    /// <summary>Override force OCR for this file.</summary>
    [JsonPropertyName("force_ocr")]
    public bool? ForceOcr { get; init; }

    /// <summary>Override disable OCR for this file.</summary>
    [JsonPropertyName("disable_ocr")]
    public bool? DisableOcr { get; init; }

    /// <summary>List of 1-indexed page numbers to force OCR on for this file.</summary>
    [JsonPropertyName("force_ocr_pages")]
    public IReadOnlyList<long>? ForceOcrPages { get; init; }

    /// <summary>Override chunking configuration for this file.</summary>
    [JsonPropertyName("chunking")]
    public ChunkingConfig? Chunking { get; init; }

    /// <summary>Override image extraction configuration for this file.</summary>
    [JsonPropertyName("images")]
    public ImageExtractionConfig? Images { get; init; }

    /// <summary>Override PDF options for this file.</summary>
    [JsonPropertyName("pdf_options")]
    public PdfConfig? PdfOptions { get; init; }

    /// <summary>Override token reduction for this file.</summary>
    [JsonPropertyName("token_reduction")]
    public TokenReductionConfig? TokenReduction { get; init; }

    /// <summary>Override language detection for this file.</summary>
    [JsonPropertyName("language_detection")]
    public LanguageDetectionConfig? LanguageDetection { get; init; }

    /// <summary>Override page extraction for this file.</summary>
    [JsonPropertyName("pages")]
    public PageConfig? Pages { get; init; }

    /// <summary>Override keyword extraction for this file.</summary>
    [JsonPropertyName("keywords")]
    public KeywordConfig? Keywords { get; init; }

    /// <summary>Override post-processor for this file.</summary>
    [JsonPropertyName("postprocessor")]
    public PostProcessorConfig? Postprocessor { get; init; }

    /// <summary>Override HTML conversion options for this file.</summary>
    [JsonPropertyName("html_options")]
    public HtmlConversionOptions? HtmlOptions { get; init; }

    /// <summary>Override layout detection for this file.</summary>
    [JsonPropertyName("layout")]
    public LayoutDetectionConfig? Layout { get; init; }

    /// <summary>Override document structure output for this file.</summary>
    [JsonPropertyName("include_document_structure")]
    public bool? IncludeDocumentStructure { get; init; }

    /// <summary>Override content output format for this file.</summary>
    [JsonPropertyName("output_format")]
    public string? OutputFormat { get; init; }

    /// <summary>Override result format for this file.</summary>
    [JsonPropertyName("result_format")]
    public string? ResultFormat { get; init; }

    /// <summary>Per-file extraction timeout in seconds. When exceeded, extraction for this file is cancelled and an error is returned.</summary>
    [JsonPropertyName("timeout_secs")]
    public ulong? TimeoutSecs { get; init; }
}

/// <summary>
/// A file path paired with an optional per-file extraction config override.
/// </summary>
public sealed class FileItemWithConfig
{
    /// <summary>The file path to extract.</summary>
    public string Path { get; }

    /// <summary>Optional per-file config overrides (null = use batch default).</summary>
    public FileExtractionConfig? Config { get; }

    /// <summary>
    /// Initializes a new instance with a file path and optional config override.
    /// </summary>
    public FileItemWithConfig(string path, FileExtractionConfig? config = null)
    {
        Path = path ?? throw new ArgumentNullException(nameof(path));
        Config = config;
    }
}

/// <summary>
/// In-memory document data with MIME type and optional per-file config override.
/// </summary>
public sealed class BytesItemWithConfig
{
    /// <summary>The document bytes.</summary>
    public byte[] Data { get; }

    /// <summary>The MIME type of the document.</summary>
    public string MimeType { get; }

    /// <summary>Optional per-file config overrides (null = use batch default).</summary>
    public FileExtractionConfig? Config { get; }

    /// <summary>
    /// Initializes a new instance with document data, MIME type, and optional config override.
    /// </summary>
    public BytesItemWithConfig(byte[] data, string mimeType, FileExtractionConfig? config = null)
    {
        Data = data ?? throw new ArgumentNullException(nameof(data));
        MimeType = mimeType ?? throw new ArgumentNullException(nameof(mimeType));
        Config = config;
    }
}

/// <summary>
/// Represents a document as bytes with its MIME type, used for batch extraction from in-memory data.
/// </summary>
public sealed class BytesWithMime
{
    /// <summary>
    /// The document bytes.
    /// </summary>
    public byte[] Data { get; }

    /// <summary>
    /// The MIME type of the document (e.g., "application/pdf").
    /// </summary>
    public string MimeType { get; }

    /// <summary>
    /// Initializes a new instance with document bytes and MIME type.
    /// </summary>
    /// <param name="data">Document bytes. Must not be null.</param>
    /// <param name="mimeType">MIME type string. Must not be null.</param>
    /// <exception cref="ArgumentNullException">If data or mimeType is null</exception>
    public BytesWithMime(byte[] data, string mimeType)
    {
        Data = data ?? throw new ArgumentNullException(nameof(data));
        MimeType = mimeType ?? throw new ArgumentNullException(nameof(mimeType));
    }
}

/// <summary>
/// Interface for custom post-processors that can modify extraction results.
/// </summary>
public interface IPostProcessor
{
    /// <summary>
    /// The unique name of this post-processor.
    /// </summary>
    string Name { get; }

    /// <summary>
    /// The priority order (higher values run first). Useful for ordering multiple processors.
    /// </summary>
    int Priority { get; }

    /// <summary>
    /// Processes an extraction result, potentially modifying it.
    /// </summary>
    /// <param name="result">The extraction result to process. Can be modified in-place.</param>
    /// <returns>The (potentially modified) extraction result.</returns>
    ExtractionResult Process(ExtractionResult result);
}

/// <summary>
/// Interface for custom validators that can validate extraction results.
/// </summary>
public interface IValidator
{
    /// <summary>
    /// The unique name of this validator.
    /// </summary>
    string Name { get; }

    /// <summary>
    /// The priority order (higher values run first). Useful for ordering multiple validators.
    /// </summary>
    int Priority { get; }

    /// <summary>
    /// Validates an extraction result. Should throw an exception if validation fails.
    /// </summary>
    /// <param name="result">The extraction result to validate.</param>
    /// <exception cref="Exception">Thrown if validation fails. The exception message will be used as the error reason.</exception>
    void Validate(ExtractionResult result);
}

/// <summary>
/// Interface for custom OCR backends for document extraction.
/// </summary>
public interface IOcrBackend
{
    /// <summary>
    /// The unique name of this OCR backend (e.g., "tesseract", "custom_ocr").
    /// </summary>
    string Name { get; }

    /// <summary>
    /// Processes image bytes and returns OCR results as JSON.
    /// </summary>
    /// <param name="imageBytes">Raw image data (PNG, JPEG, etc.).</param>
    /// <param name="config">OCR configuration, if any.</param>
    /// <returns>JSON string with OCR results (structure depends on implementation).</returns>
    string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config);
}

/// <summary>
/// Represents an embedding preset with configuration for text embedding generation.
/// </summary>
public sealed class EmbeddingPreset
{
    /// <summary>
    /// The name/identifier of this embedding preset (e.g., "default", "openai").
    /// </summary>
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// The recommended chunk size (in tokens or characters) for this embedding model.
    /// </summary>
    [JsonPropertyName("chunk_size")]
    public int ChunkSize { get; set; }

    /// <summary>
    /// The recommended overlap between chunks when chunking text for this model.
    /// </summary>
    [JsonPropertyName("overlap")]
    public int Overlap { get; set; }

    /// <summary>
    /// The name of the embedding model (e.g., "text-embedding-ada-002").
    /// </summary>
    [JsonPropertyName("model_name")]
    public string ModelName { get; set; } = string.Empty;

    /// <summary>
    /// The output dimensionality of the embedding vectors from this model.
    /// </summary>
    [JsonPropertyName("dimensions")]
    public int Dimensions { get; set; }

    /// <summary>
    /// Human-readable description of this embedding preset.
    /// </summary>
    [JsonPropertyName("description")]
    public string Description { get; set; } = string.Empty;
}

/// <summary>
/// Comprehensive Djot document structure with semantic preservation.
/// </summary>
public sealed class DjotContent
{
    /// <summary>
    /// Plain text representation for backwards compatibility.
    /// </summary>
    [JsonPropertyName("plain_text")]
    public string PlainText { get; set; } = string.Empty;

    /// <summary>
    /// Structured block-level content.
    /// </summary>
    [JsonPropertyName("blocks")]
    public List<FormattedBlock> Blocks { get; set; } = new();

    /// <summary>
    /// Metadata from YAML frontmatter.
    /// </summary>
    [JsonPropertyName("metadata")]
    public Metadata Metadata { get; set; } = new();

    /// <summary>
    /// Extracted tables as structured data.
    /// </summary>
    [JsonPropertyName("tables")]
    public List<Table> Tables { get; set; } = new();

    /// <summary>
    /// Extracted images with metadata.
    /// </summary>
    [JsonPropertyName("images")]
    public List<DjotImage> Images { get; set; } = new();

    /// <summary>
    /// Extracted links with URLs.
    /// </summary>
    [JsonPropertyName("links")]
    public List<DjotLink> Links { get; set; } = new();

    /// <summary>
    /// Footnote definitions.
    /// </summary>
    [JsonPropertyName("footnotes")]
    public List<Footnote> Footnotes { get; set; } = new();

    /// <summary>
    /// Attributes mapped by element identifier.
    /// </summary>
    [JsonPropertyName("attributes")]
    public List<List<object>> Attributes { get; set; } = new();
}

/// <summary>
/// Block-level element in a Djot document.
/// </summary>
public sealed class FormattedBlock
{
    /// <summary>
    /// Type of block element.
    /// </summary>
    [JsonPropertyName("block_type")]
    public string BlockType { get; set; } = string.Empty;

    /// <summary>
    /// Heading level (1-6) for headings, or nesting level for lists.
    /// </summary>
    [JsonPropertyName("level")]
    public int? Level { get; set; }

    /// <summary>
    /// Inline content within the block.
    /// </summary>
    [JsonPropertyName("inline_content")]
    public List<InlineElement> InlineContent { get; set; } = new();

    /// <summary>
    /// Element attributes (classes, IDs, key-value pairs).
    /// </summary>
    [JsonPropertyName("attributes")]
    public DjotAttributes? Attributes { get; set; }

    /// <summary>
    /// Language identifier for code blocks.
    /// </summary>
    [JsonPropertyName("language")]
    public string? Language { get; set; }

    /// <summary>
    /// Raw code content for code blocks.
    /// </summary>
    [JsonPropertyName("code")]
    public string? Code { get; set; }

    /// <summary>
    /// Nested blocks for containers (blockquotes, list items, divs).
    /// </summary>
    [JsonPropertyName("children")]
    public List<FormattedBlock> Children { get; set; } = new();
}

/// <summary>
/// Inline element within a block.
/// </summary>
public sealed class InlineElement
{
    /// <summary>
    /// Type of inline element.
    /// </summary>
    [JsonPropertyName("element_type")]
    public string ElementType { get; set; } = string.Empty;

    /// <summary>
    /// Text content.
    /// </summary>
    [JsonPropertyName("content")]
    public string Content { get; set; } = string.Empty;

    /// <summary>
    /// Element attributes.
    /// </summary>
    [JsonPropertyName("attributes")]
    public DjotAttributes? Attributes { get; set; }

    /// <summary>
    /// Additional metadata (e.g., href for links, src/alt for images).
    /// </summary>
    [JsonPropertyName("metadata")]
    public Dictionary<string, string>? Metadata { get; set; }
}

/// <summary>
/// Element attributes in Djot.
/// </summary>
public sealed class DjotAttributes
{
    /// <summary>
    /// Element ID.
    /// </summary>
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    /// <summary>
    /// CSS classes.
    /// </summary>
    [JsonPropertyName("classes")]
    public List<string> Classes { get; set; } = new();

    /// <summary>
    /// Key-value pairs.
    /// </summary>
    [JsonPropertyName("key_values")]
    public List<List<string>> KeyValues { get; set; } = new();
}

/// <summary>
/// Image element in Djot.
/// </summary>
public sealed class DjotImage
{
    /// <summary>
    /// Image source URL or path.
    /// </summary>
    [JsonPropertyName("src")]
    public string Src { get; set; } = string.Empty;

    /// <summary>
    /// Alternative text.
    /// </summary>
    [JsonPropertyName("alt")]
    public string Alt { get; set; } = string.Empty;

    /// <summary>
    /// Optional title.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    /// <summary>
    /// Element attributes.
    /// </summary>
    [JsonPropertyName("attributes")]
    public DjotAttributes? Attributes { get; set; }
}

/// <summary>
/// Link element in Djot.
/// </summary>
public sealed class DjotLink
{
    /// <summary>
    /// Link URL.
    /// </summary>
    [JsonPropertyName("url")]
    public string Url { get; set; } = string.Empty;

    /// <summary>
    /// Link text content.
    /// </summary>
    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;

    /// <summary>
    /// Optional title.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    /// <summary>
    /// Element attributes.
    /// </summary>
    [JsonPropertyName("attributes")]
    public DjotAttributes? Attributes { get; set; }
}

/// <summary>
/// Footnote in Djot.
/// </summary>
public sealed class Footnote
{
    /// <summary>
    /// Footnote label.
    /// </summary>
    [JsonPropertyName("label")]
    public string Label { get; set; } = string.Empty;

    /// <summary>
    /// Footnote content blocks.
    /// </summary>
    [JsonPropertyName("content")]
    public List<FormattedBlock> Content { get; set; } = new();
}

/// <summary>
/// Top-level structured document representation with a flat array of nodes forming a tree.
/// Nodes are stored in document/reading order with index-based parent-child references.
/// </summary>
public sealed class DocumentStructure
{
    /// <summary>
    /// All nodes in the document, stored in document/reading order.
    /// </summary>
    [JsonPropertyName("nodes")]
    public List<DocumentNode> Nodes { get; set; } = new();
}

/// <summary>
/// A single node in the document tree structure.
/// Each node has a deterministic ID, typed content, optional parent/children references, and metadata.
/// </summary>
public sealed class DocumentNode
{
    /// <summary>
    /// Deterministic identifier generated from content hash and position.
    /// The same document always produces the same IDs, useful for diffing and caching.
    /// </summary>
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    /// <summary>
    /// Node content with type-specific data.
    /// Uses a discriminator field to identify the content type.
    /// </summary>
    [JsonPropertyName("content")]
    public NodeContent Content { get; set; } = new();

    /// <summary>
    /// Parent node index (None means this is a root-level node).
    /// </summary>
    [JsonPropertyName("parent")]
    public uint? Parent { get; set; }

    /// <summary>
    /// Child node indices in reading order.
    /// </summary>
    [JsonPropertyName("children")]
    public List<uint> Children { get; set; } = new();

    /// <summary>
    /// Content layer classification (Body, Header, Footer, Footnote).
    /// </summary>
    [JsonPropertyName("content_layer")]
    public string ContentLayer { get; set; } = "body";

    /// <summary>
    /// Page number where this node starts (1-indexed), if available.
    /// </summary>
    [JsonPropertyName("page")]
    public uint? Page { get; set; }

    /// <summary>
    /// Page number where this node ends (for multi-page tables/sections), if available.
    /// </summary>
    [JsonPropertyName("page_end")]
    public uint? PageEnd { get; set; }

    /// <summary>
    /// Bounding box in document coordinates, if available.
    /// </summary>
    [JsonPropertyName("bbox")]
    public BoundingBox? Bbox { get; set; }

    /// <summary>
    /// Inline annotations (formatting, links) on this node's text content.
    /// </summary>
    [JsonPropertyName("annotations")]
    public List<DocumentTextAnnotation> Annotations { get; set; } = new();
}

/// <summary>
/// Node content with type-specific data.
/// Uses node_type discriminator to identify the content variant.
/// </summary>
public sealed class NodeContent
{
    /// <summary>
    /// Node type discriminator (title, heading, paragraph, list, list_item, table, image, code, quote, formula, footnote, group, page_break).
    /// </summary>
    [JsonPropertyName("node_type")]
    public string NodeType { get; set; } = string.Empty;

    /// <summary>
    /// Text content (for title, heading, paragraph, list_item, code, formula, footnote).
    /// </summary>
    [JsonPropertyName("text")]
    public string? Text { get; set; }

    /// <summary>
    /// Heading level (1-6) for heading nodes.
    /// </summary>
    [JsonPropertyName("level")]
    public int? Level { get; set; }

    /// <summary>
    /// Whether a list is ordered (numbered) or unordered (bulleted).
    /// </summary>
    [JsonPropertyName("ordered")]
    public bool? Ordered { get; set; }

    /// <summary>
    /// Table grid structure with cells.
    /// </summary>
    [JsonPropertyName("grid")]
    public TableGrid? Grid { get; set; }

    /// <summary>
    /// Alternative text for images.
    /// </summary>
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    /// <summary>
    /// Image index reference for images.
    /// </summary>
    [JsonPropertyName("image_index")]
    public uint? ImageIndex { get; set; }

    /// <summary>
    /// Source URL or path for images.
    /// </summary>
    [JsonPropertyName("src")]
    public string? Src { get; set; }

    /// <summary>
    /// Programming language for code blocks.
    /// </summary>
    [JsonPropertyName("language")]
    public string? Language { get; set; }

    /// <summary>
    /// Label for group/section nodes.
    /// </summary>
    [JsonPropertyName("label")]
    public string? Label { get; set; }

    /// <summary>
    /// Heading level for group heading.
    /// </summary>
    [JsonPropertyName("heading_level")]
    public int? HeadingLevel { get; set; }

    /// <summary>
    /// Heading text for group nodes.
    /// </summary>
    [JsonPropertyName("heading_text")]
    public string? HeadingText { get; set; }
}

/// <summary>
/// Structured table grid with row and column dimensions and cell-level metadata.
/// </summary>
public sealed class TableGrid
{
    /// <summary>
    /// Number of rows in the table.
    /// </summary>
    [JsonPropertyName("rows")]
    public int Rows { get; set; }

    /// <summary>
    /// Number of columns in the table.
    /// </summary>
    [JsonPropertyName("cols")]
    public int Cols { get; set; }

    /// <summary>
    /// All cells in row-major order.
    /// </summary>
    [JsonPropertyName("cells")]
    public List<GridCell> Cells { get; set; } = new();
}

/// <summary>
/// Individual grid cell with content and position/span metadata.
/// </summary>
public sealed class GridCell
{
    /// <summary>
    /// Cell text content.
    /// </summary>
    [JsonPropertyName("content")]
    public string Content { get; set; } = string.Empty;

    /// <summary>
    /// Zero-indexed row position.
    /// </summary>
    [JsonPropertyName("row")]
    public uint Row { get; set; }

    /// <summary>
    /// Zero-indexed column position.
    /// </summary>
    [JsonPropertyName("col")]
    public uint Col { get; set; }

    /// <summary>
    /// Number of rows this cell spans.
    /// </summary>
    [JsonPropertyName("row_span")]
    public uint RowSpan { get; set; } = 1;

    /// <summary>
    /// Number of columns this cell spans.
    /// </summary>
    [JsonPropertyName("col_span")]
    public uint ColSpan { get; set; } = 1;

    /// <summary>
    /// Whether this is a header cell.
    /// </summary>
    [JsonPropertyName("is_header")]
    public bool IsHeader { get; set; }

    /// <summary>
    /// Bounding box for this cell, if available.
    /// </summary>
    [JsonPropertyName("bbox")]
    public BoundingBox? Bbox { get; set; }
}

/// <summary>
/// Inline text annotation with byte-range based formatting and link information.
/// Annotations reference byte offsets into the node's text content.
/// </summary>
public sealed class DocumentTextAnnotation
{
    /// <summary>
    /// Start byte offset in the node's text content (inclusive).
    /// </summary>
    [JsonPropertyName("start")]
    public uint Start { get; set; }

    /// <summary>
    /// End byte offset in the node's text content (exclusive).
    /// </summary>
    [JsonPropertyName("end")]
    public uint End { get; set; }

    /// <summary>
    /// Annotation type discriminator (bold, italic, underline, strikethrough, code, subscript, superscript, link).
    /// </summary>
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    /// <summary>
    /// URL for link annotations.
    /// </summary>
    [JsonPropertyName("url")]
    public string? Url { get; set; }

    /// <summary>
    /// Title for link annotations.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }
}

/// <summary>
/// Configuration for ONNX-based document layout detection.
///
/// Controls confidence filtering, heuristic post-processing,
/// and table model selection for page-level layout detection.
/// </summary>
public sealed class LayoutDetectionConfig
{
    /// <summary>
    /// Minimum confidence threshold for detected layout regions (0.0-1.0).
    /// Regions below this threshold are discarded.
    /// </summary>
    [JsonPropertyName("confidence_threshold")]
    public double? ConfidenceThreshold { get; init; }

    /// <summary>
    /// Whether to apply heuristic post-processing to refine layout regions.
    /// Default: true
    /// </summary>
    [JsonPropertyName("apply_heuristics")]
    public bool? ApplyHeuristics { get; init; }

    /// <summary>
    /// Table detection model to use.
    /// Supported values: "tatr", "slanet_wired", "slanet_wireless", "slanet_plus", "slanet_auto".
    /// </summary>
    [JsonPropertyName("table_model")]
    public string? TableModel { get; init; }

    /// <summary>
    /// Hardware acceleration configuration for ONNX Runtime models.
    /// If null, uses default acceleration settings.
    /// </summary>
    [JsonPropertyName("acceleration")]
    public AccelerationConfig? Acceleration { get; init; }
}

// ============================================================================
// Tree-sitter ProcessResult types (serialized from Rust via serde)
// ============================================================================

/// <summary>
/// Byte/line/column range in source code.
/// </summary>
public sealed class CodeSpan
{
    /// <summary>Start byte offset.</summary>
    [JsonPropertyName("start_byte")]
    public int StartByte { get; init; }

    /// <summary>End byte offset.</summary>
    [JsonPropertyName("end_byte")]
    public int EndByte { get; init; }

    /// <summary>Start line number (0-based).</summary>
    [JsonPropertyName("start_line")]
    public int StartLine { get; init; }

    /// <summary>Start column number (0-based).</summary>
    [JsonPropertyName("start_column")]
    public int StartColumn { get; init; }

    /// <summary>End line number (0-based).</summary>
    [JsonPropertyName("end_line")]
    public int EndLine { get; init; }

    /// <summary>End column number (0-based).</summary>
    [JsonPropertyName("end_column")]
    public int EndColumn { get; init; }
}

/// <summary>
/// Aggregate metrics for a parsed source file.
/// </summary>
public sealed class CodeFileMetrics
{
    /// <summary>Total number of lines.</summary>
    [JsonPropertyName("total_lines")]
    public int TotalLines { get; init; }

    /// <summary>Number of lines containing code.</summary>
    [JsonPropertyName("code_lines")]
    public int CodeLines { get; init; }

    /// <summary>Number of lines containing comments.</summary>
    [JsonPropertyName("comment_lines")]
    public int CommentLines { get; init; }

    /// <summary>Number of blank lines.</summary>
    [JsonPropertyName("blank_lines")]
    public int BlankLines { get; init; }

    /// <summary>Total byte size of the file.</summary>
    [JsonPropertyName("total_bytes")]
    public int TotalBytes { get; init; }

    /// <summary>Number of AST nodes.</summary>
    [JsonPropertyName("node_count")]
    public int NodeCount { get; init; }

    /// <summary>Number of parse errors.</summary>
    [JsonPropertyName("error_count")]
    public int ErrorCount { get; init; }

    /// <summary>Maximum AST depth.</summary>
    [JsonPropertyName("max_depth")]
    public int MaxDepth { get; init; }
}

/// <summary>
/// A structural element (function, class, etc.) in source code.
/// </summary>
public sealed class CodeStructureItem
{
    /// <summary>Kind of structure (e.g. "function", "class", "method").</summary>
    [JsonPropertyName("kind")]
    public string Kind { get; init; } = string.Empty;

    /// <summary>Name of the item, if available.</summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }

    /// <summary>Visibility modifier (e.g. "public", "private").</summary>
    [JsonPropertyName("visibility")]
    public string? Visibility { get; init; }

    /// <summary>Source span of the item.</summary>
    [JsonPropertyName("span")]
    public CodeSpan Span { get; init; } = new();

    /// <summary>Nested child structure items.</summary>
    [JsonPropertyName("children")]
    public CodeStructureItem[] Children { get; init; } = [];

    /// <summary>Decorators/attributes applied to the item.</summary>
    [JsonPropertyName("decorators")]
    public string[] Decorators { get; init; } = [];

    /// <summary>Associated documentation comment text.</summary>
    [JsonPropertyName("doc_comment")]
    public string? DocComment { get; init; }

    /// <summary>Full signature of the item.</summary>
    [JsonPropertyName("signature")]
    public string? Signature { get; init; }

    /// <summary>Source span of the item body, if applicable.</summary>
    [JsonPropertyName("body_span")]
    public CodeSpan? BodySpan { get; init; }
}

/// <summary>
/// An import/include/require statement.
/// </summary>
public sealed class CodeImportInfo
{
    /// <summary>Module or path being imported.</summary>
    [JsonPropertyName("source")]
    public string Source { get; init; } = string.Empty;

    /// <summary>Specific items imported from the source.</summary>
    [JsonPropertyName("items")]
    public string[] Items { get; init; } = [];

    /// <summary>Alias for the import, if any.</summary>
    [JsonPropertyName("alias")]
    public string? Alias { get; init; }

    /// <summary>Whether this is a wildcard import (e.g. import *).</summary>
    [JsonPropertyName("is_wildcard")]
    public bool IsWildcard { get; init; }

    /// <summary>Source span of the import statement.</summary>
    [JsonPropertyName("span")]
    public CodeSpan Span { get; init; } = new();
}

/// <summary>
/// An exported symbol from source code.
/// </summary>
public sealed class CodeExportInfo
{
    /// <summary>Name of the exported symbol.</summary>
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    /// <summary>Kind of export (e.g. "function", "class", "variable").</summary>
    [JsonPropertyName("kind")]
    public string Kind { get; init; } = string.Empty;

    /// <summary>Source span of the export.</summary>
    [JsonPropertyName("span")]
    public CodeSpan Span { get; init; } = new();
}

/// <summary>
/// A symbol (variable, constant, type alias, etc.) in source code.
/// </summary>
public sealed class CodeSymbolInfo
{
    /// <summary>Name of the symbol.</summary>
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    /// <summary>Kind of symbol (e.g. "variable", "constant", "type_alias").</summary>
    [JsonPropertyName("kind")]
    public string Kind { get; init; } = string.Empty;

    /// <summary>Type annotation, if present.</summary>
    [JsonPropertyName("type_annotation")]
    public string? TypeAnnotation { get; init; }

    /// <summary>Source span of the symbol.</summary>
    [JsonPropertyName("span")]
    public CodeSpan Span { get; init; } = new();
}

/// <summary>
/// A comment in source code.
/// </summary>
public sealed class CodeCommentInfo
{
    /// <summary>Text content of the comment.</summary>
    [JsonPropertyName("text")]
    public string Text { get; init; } = string.Empty;

    /// <summary>Kind of comment (e.g. "line", "block").</summary>
    [JsonPropertyName("kind")]
    public string Kind { get; init; } = string.Empty;

    /// <summary>Source span of the comment.</summary>
    [JsonPropertyName("span")]
    public CodeSpan Span { get; init; } = new();
}

/// <summary>
/// A section within a docstring (e.g. @param, @returns).
/// </summary>
public sealed class CodeDocSection
{
    /// <summary>Section kind (e.g. "param", "returns", "description").</summary>
    [JsonPropertyName("kind")]
    public string Kind { get; init; } = string.Empty;

    /// <summary>Parameter or section name, if applicable.</summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }

    /// <summary>Content of the section.</summary>
    [JsonPropertyName("content")]
    public string Content { get; init; } = string.Empty;
}

/// <summary>
/// A documentation comment/docstring.
/// </summary>
public sealed class CodeDocstringInfo
{
    /// <summary>Full text of the docstring.</summary>
    [JsonPropertyName("text")]
    public string Text { get; init; } = string.Empty;

    /// <summary>Docstring format (e.g. "javadoc", "restructuredtext", "google").</summary>
    [JsonPropertyName("format")]
    public string Format { get; init; } = string.Empty;

    /// <summary>Name of the associated code item, if any.</summary>
    [JsonPropertyName("associated_item")]
    public string? AssociatedItem { get; init; }

    /// <summary>Source span of the docstring.</summary>
    [JsonPropertyName("span")]
    public CodeSpan Span { get; init; } = new();

    /// <summary>Parsed sections of the docstring.</summary>
    [JsonPropertyName("sections")]
    public CodeDocSection[] Sections { get; init; } = [];
}

/// <summary>
/// A parse error or warning from tree-sitter.
/// </summary>
public sealed class CodeDiagnostic
{
    /// <summary>Diagnostic message.</summary>
    [JsonPropertyName("message")]
    public string Message { get; init; } = string.Empty;

    /// <summary>Severity level (e.g. "error", "warning").</summary>
    [JsonPropertyName("severity")]
    public string Severity { get; init; } = string.Empty;

    /// <summary>Source span of the diagnostic.</summary>
    [JsonPropertyName("span")]
    public CodeSpan Span { get; init; } = new();
}

/// <summary>
/// Parent context for a code chunk.
/// </summary>
public sealed class CodeChunkContext
{
    /// <summary>Name of the parent structure item.</summary>
    [JsonPropertyName("parent_name")]
    public string? ParentName { get; init; }

    /// <summary>Kind of the parent structure item.</summary>
    [JsonPropertyName("parent_kind")]
    public string? ParentKind { get; init; }
}

/// <summary>
/// A chunk of source code with optional context.
/// </summary>
public sealed class CodeChunk
{
    /// <summary>Source code content of the chunk.</summary>
    [JsonPropertyName("content")]
    public string Content { get; init; } = string.Empty;

    /// <summary>Programming language of the chunk.</summary>
    [JsonPropertyName("language")]
    public string Language { get; init; } = string.Empty;

    /// <summary>Source span of the chunk.</summary>
    [JsonPropertyName("span")]
    public CodeSpan Span { get; init; } = new();

    /// <summary>Optional parent context for the chunk.</summary>
    [JsonPropertyName("context")]
    public CodeChunkContext? Context { get; init; }
}

/// <summary>
/// Complete result of tree-sitter code analysis.
/// </summary>
public sealed class CodeProcessResult
{
    /// <summary>Detected programming language.</summary>
    [JsonPropertyName("language")]
    public string Language { get; init; } = string.Empty;

    /// <summary>File-level metrics.</summary>
    [JsonPropertyName("metrics")]
    public CodeFileMetrics Metrics { get; init; } = new();

    /// <summary>Structural items (functions, classes, etc.).</summary>
    [JsonPropertyName("structure")]
    public CodeStructureItem[] Structure { get; init; } = [];

    /// <summary>Import statements.</summary>
    [JsonPropertyName("imports")]
    public CodeImportInfo[] Imports { get; init; } = [];

    /// <summary>Export declarations.</summary>
    [JsonPropertyName("exports")]
    public CodeExportInfo[] Exports { get; init; } = [];

    /// <summary>Comments found in the source.</summary>
    [JsonPropertyName("comments")]
    public CodeCommentInfo[] Comments { get; init; } = [];

    /// <summary>Docstrings found in the source.</summary>
    [JsonPropertyName("docstrings")]
    public CodeDocstringInfo[] Docstrings { get; init; } = [];

    /// <summary>Symbols (variables, constants, etc.).</summary>
    [JsonPropertyName("symbols")]
    public CodeSymbolInfo[] Symbols { get; init; } = [];

    /// <summary>Parse diagnostics (errors, warnings).</summary>
    [JsonPropertyName("diagnostics")]
    public CodeDiagnostic[] Diagnostics { get; init; } = [];

    /// <summary>Code chunks for RAG/retrieval.</summary>
    [JsonPropertyName("chunks")]
    public CodeChunk[] Chunks { get; init; } = [];
}

/// <summary>
/// CSV/TSV file metadata.
/// </summary>
public sealed class CsvMetadata
{
    /// <summary>
    /// Number of rows.
    /// </summary>
    [JsonPropertyName("row_count")]
    public int RowCount { get; set; }

    /// <summary>
    /// Number of columns.
    /// </summary>
    [JsonPropertyName("column_count")]
    public int ColumnCount { get; set; }

    /// <summary>
    /// Detected delimiter character.
    /// </summary>
    [JsonPropertyName("delimiter")]
    public string? Delimiter { get; set; }

    /// <summary>
    /// Whether the file has a header row.
    /// </summary>
    [JsonPropertyName("has_header")]
    public bool HasHeader { get; set; }

    /// <summary>
    /// Detected column types.
    /// </summary>
    [JsonPropertyName("column_types")]
    public List<string>? ColumnTypes { get; set; }
}

/// <summary>
/// Year range for bibliographic metadata.
/// </summary>
public sealed class YearRange
{
    /// <summary>
    /// Minimum year.
    /// </summary>
    [JsonPropertyName("min")]
    public int? Min { get; set; }

    /// <summary>
    /// Maximum year.
    /// </summary>
    [JsonPropertyName("max")]
    public int? Max { get; set; }

    /// <summary>
    /// All years present.
    /// </summary>
    [JsonPropertyName("years")]
    public List<int> Years { get; set; } = new();
}

/// <summary>
/// BibTeX bibliography metadata.
/// </summary>
public sealed class BibtexMetadata
{
    /// <summary>
    /// Number of BibTeX entries.
    /// </summary>
    [JsonPropertyName("entry_count")]
    public int EntryCount { get; set; }

    /// <summary>
    /// Citation keys.
    /// </summary>
    [JsonPropertyName("citation_keys")]
    public List<string> CitationKeys { get; set; } = new();

    /// <summary>
    /// Authors.
    /// </summary>
    [JsonPropertyName("authors")]
    public List<string> Authors { get; set; } = new();

    /// <summary>
    /// Year range of entries.
    /// </summary>
    [JsonPropertyName("year_range")]
    public YearRange? YearRange { get; set; }

    /// <summary>
    /// Entry types with counts.
    /// </summary>
    [JsonPropertyName("entry_types")]
    public Dictionary<string, int>? EntryTypes { get; set; }
}

/// <summary>
/// Citation file metadata (RIS, PubMed, EndNote).
/// </summary>
public sealed class CitationMetadata
{
    /// <summary>
    /// Number of citations.
    /// </summary>
    [JsonPropertyName("citation_count")]
    public int CitationCount { get; set; }

    /// <summary>
    /// Citation format (e.g., "RIS", "PubMed").
    /// </summary>
    [JsonPropertyName("format")]
    public string? Format { get; set; }

    /// <summary>
    /// Authors.
    /// </summary>
    [JsonPropertyName("authors")]
    public List<string> Authors { get; set; } = new();

    /// <summary>
    /// Year range of citations.
    /// </summary>
    [JsonPropertyName("year_range")]
    public YearRange? YearRange { get; set; }

    /// <summary>
    /// DOIs.
    /// </summary>
    [JsonPropertyName("dois")]
    public List<string> Dois { get; set; } = new();

    /// <summary>
    /// Keywords.
    /// </summary>
    [JsonPropertyName("keywords")]
    public List<string> Keywords { get; set; } = new();
}

/// <summary>
/// FictionBook (FB2) metadata.
/// </summary>
public sealed class FictionBookMetadata
{
    /// <summary>
    /// Genres.
    /// </summary>
    [JsonPropertyName("genres")]
    public List<string> Genres { get; set; } = new();

    /// <summary>
    /// Sequences (series).
    /// </summary>
    [JsonPropertyName("sequences")]
    public List<string> Sequences { get; set; } = new();

    /// <summary>
    /// Annotation/summary.
    /// </summary>
    [JsonPropertyName("annotation")]
    public string? Annotation { get; set; }
}

/// <summary>
/// dBASE field information.
/// </summary>
public sealed class DbfFieldInfo
{
    /// <summary>
    /// Field name.
    /// </summary>
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// Field type.
    /// </summary>
    [JsonPropertyName("field_type")]
    public string FieldType { get; set; } = string.Empty;
}

/// <summary>
/// dBASE (DBF) file metadata.
/// </summary>
public sealed class DbfMetadata
{
    /// <summary>
    /// Number of records.
    /// </summary>
    [JsonPropertyName("record_count")]
    public int RecordCount { get; set; }

    /// <summary>
    /// Number of fields.
    /// </summary>
    [JsonPropertyName("field_count")]
    public int FieldCount { get; set; }

    /// <summary>
    /// Field definitions.
    /// </summary>
    [JsonPropertyName("fields")]
    public List<DbfFieldInfo> Fields { get; set; } = new();
}

/// <summary>
/// JATS contributor with role.
/// </summary>
public sealed class ContributorRole
{
    /// <summary>
    /// Contributor name.
    /// </summary>
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// Contributor role.
    /// </summary>
    [JsonPropertyName("role")]
    public string? Role { get; set; }
}

/// <summary>
/// JATS (Journal Article Tag Suite) metadata.
/// </summary>
public sealed class JatsMetadata
{
    /// <summary>
    /// Copyright statement.
    /// </summary>
    [JsonPropertyName("copyright")]
    public string? Copyright { get; set; }

    /// <summary>
    /// License information.
    /// </summary>
    [JsonPropertyName("license")]
    public string? License { get; set; }

    /// <summary>
    /// History dates (e.g., received, accepted, published).
    /// </summary>
    [JsonPropertyName("history_dates")]
    public Dictionary<string, string> HistoryDates { get; set; } = new();

    /// <summary>
    /// Contributors with roles.
    /// </summary>
    [JsonPropertyName("contributor_roles")]
    public List<ContributorRole> ContributorRoles { get; set; } = new();
}

/// <summary>
/// EPUB metadata (Dublin Core extensions).
/// </summary>
public sealed class EpubMetadata
{
    /// <summary>
    /// Coverage.
    /// </summary>
    [JsonPropertyName("coverage")]
    public string? Coverage { get; set; }

    /// <summary>
    /// Dublin Core format.
    /// </summary>
    [JsonPropertyName("dc_format")]
    public string? DcFormat { get; set; }

    /// <summary>
    /// Relation.
    /// </summary>
    [JsonPropertyName("relation")]
    public string? Relation { get; set; }

    /// <summary>
    /// Source.
    /// </summary>
    [JsonPropertyName("source")]
    public string? Source { get; set; }

    /// <summary>
    /// Dublin Core type.
    /// </summary>
    [JsonPropertyName("dc_type")]
    public string? DcType { get; set; }

    /// <summary>
    /// Cover image path.
    /// </summary>
    [JsonPropertyName("cover_image")]
    public string? CoverImage { get; set; }
}

/// <summary>
/// Outlook PST archive metadata.
/// </summary>
public sealed class PstMetadata
{
    /// <summary>
    /// Number of messages in the archive.
    /// </summary>
    [JsonPropertyName("message_count")]
    public int MessageCount { get; set; }
}

/// <summary>
/// Represents an entry extracted from an archive (ZIP, TAR, etc.).
/// </summary>
public sealed class ArchiveEntry
{
    /// <summary>
    /// The path of the entry within the archive.
    /// </summary>
    [JsonPropertyName("path")]
    public string Path { get; set; } = string.Empty;

    /// <summary>
    /// The detected MIME type of the entry.
    /// </summary>
    [JsonPropertyName("mime_type")]
    public string MimeType { get; set; } = string.Empty;

    /// <summary>
    /// The extraction result for this entry.
    /// </summary>
    [JsonPropertyName("result")]
    public ExtractionResult? Result { get; set; }
}

/// <summary>
/// Content layer classification for document content regions.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<ContentLayer>))]
public enum ContentLayer
{
    /// <summary>
    /// Main body content.
    /// </summary>
    [JsonStringEnumMemberName("body")]
    Body,

    /// <summary>
    /// Header content.
    /// </summary>
    [JsonStringEnumMemberName("header")]
    Header,

    /// <summary>
    /// Footer content.
    /// </summary>
    [JsonStringEnumMemberName("footer")]
    Footer,

    /// <summary>
    /// Footnote content.
    /// </summary>
    [JsonStringEnumMemberName("footnote")]
    Footnote,
}

/// <summary>
/// Output format for document extraction.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<OutputFormat>))]
public enum OutputFormat
{
    /// <summary>
    /// Plain text output.
    /// </summary>
    [JsonStringEnumMemberName("plain")]
    Plain,

    /// <summary>
    /// Markdown formatted output.
    /// </summary>
    [JsonStringEnumMemberName("markdown")]
    Markdown,

    /// <summary>
    /// Djot formatted output.
    /// </summary>
    [JsonStringEnumMemberName("djot")]
    Djot,

    /// <summary>
    /// HTML formatted output.
    /// </summary>
    [JsonStringEnumMemberName("html")]
    Html,

    /// <summary>
    /// JSON tree format with heading-driven sections.
    /// </summary>
    [JsonStringEnumMemberName("json")]
    Json,

    /// <summary>
    /// Structured output with semantic elements.
    /// </summary>
    [JsonStringEnumMemberName("structured")]
    Structured,
}

/// <summary>
/// Unit type for page-like divisions in a document.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<PageUnitType>))]
public enum PageUnitType
{
    /// <summary>
    /// A document page (e.g., PDF page).
    /// </summary>
    [JsonStringEnumMemberName("page")]
    Page,

    /// <summary>
    /// A presentation slide.
    /// </summary>
    [JsonStringEnumMemberName("slide")]
    Slide,

    /// <summary>
    /// A spreadsheet sheet/tab.
    /// </summary>
    [JsonStringEnumMemberName("sheet")]
    Sheet,
}

/// <summary>
/// Classification of PDF annotation types.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<PdfAnnotationType>))]
public enum PdfAnnotationType
{
    /// <summary>
    /// Text annotation (sticky note or comment).
    /// </summary>
    [JsonStringEnumMemberName("text")]
    Text,

    /// <summary>
    /// Highlight annotation.
    /// </summary>
    [JsonStringEnumMemberName("highlight")]
    Highlight,

    /// <summary>
    /// Link annotation.
    /// </summary>
    [JsonStringEnumMemberName("link")]
    Link,

    /// <summary>
    /// Stamp annotation.
    /// </summary>
    [JsonStringEnumMemberName("stamp")]
    Stamp,

    /// <summary>
    /// Underline annotation.
    /// </summary>
    [JsonStringEnumMemberName("underline")]
    Underline,

    /// <summary>
    /// Strikeout annotation.
    /// </summary>
    [JsonStringEnumMemberName("strike_out")]
    StrikeOut,

    /// <summary>
    /// Other/unclassified annotation type.
    /// </summary>
    [JsonStringEnumMemberName("other")]
    Other,
}

/// <summary>
/// Semantic relationship kinds between document elements.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<RelationshipKind>))]
public enum RelationshipKind
{
    /// <summary>
    /// Reference to a footnote.
    /// </summary>
    [JsonStringEnumMemberName("footnote_reference")]
    FootnoteReference,

    /// <summary>
    /// Reference to a citation.
    /// </summary>
    [JsonStringEnumMemberName("citation_reference")]
    CitationReference,

    /// <summary>
    /// Internal link within the document.
    /// </summary>
    [JsonStringEnumMemberName("internal_link")]
    InternalLink,

    /// <summary>
    /// Caption for a figure, table, or other element.
    /// </summary>
    [JsonStringEnumMemberName("caption")]
    Caption,

    /// <summary>
    /// Label for an element.
    /// </summary>
    [JsonStringEnumMemberName("label")]
    Label,

    /// <summary>
    /// Table of contents entry.
    /// </summary>
    [JsonStringEnumMemberName("toc_entry")]
    TocEntry,

    /// <summary>
    /// Cross-reference to another element.
    /// </summary>
    [JsonStringEnumMemberName("cross_reference")]
    CrossReference,
}

/// <summary>
/// Format of the extraction result output.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<ResultFormat>))]
public enum ResultFormat
{
    /// <summary>
    /// Unified text content result.
    /// </summary>
    [JsonStringEnumMemberName("unified")]
    Unified,

    /// <summary>
    /// Element-based structured result.
    /// </summary>
    [JsonStringEnumMemberName("element_based")]
    ElementBased,
}

/// <summary>
/// Semantic classification of extracted URIs.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<UriKind>))]
public enum UriKind
{
    /// <summary>
    /// A standard hyperlink.
    /// </summary>
    [JsonStringEnumMemberName("hyperlink")]
    Hyperlink,

    /// <summary>
    /// An image reference URI.
    /// </summary>
    [JsonStringEnumMemberName("image")]
    Image,

    /// <summary>
    /// An anchor within the document.
    /// </summary>
    [JsonStringEnumMemberName("anchor")]
    Anchor,

    /// <summary>
    /// A citation reference.
    /// </summary>
    [JsonStringEnumMemberName("citation")]
    Citation,

    /// <summary>
    /// A general reference.
    /// </summary>
    [JsonStringEnumMemberName("reference")]
    Reference,

    /// <summary>
    /// An email address URI.
    /// </summary>
    [JsonStringEnumMemberName("email")]
    Email,
}

/// <summary>
/// Represents a keyword extracted from a document.
/// Alias for <see cref="ExtractedKeyword"/> for parity across language bindings.
/// </summary>
public sealed class Keyword
{
    /// <summary>
    /// The keyword text.
    /// </summary>
    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;

    /// <summary>
    /// Relevance score (higher is better, algorithm-specific range).
    /// </summary>
    [JsonPropertyName("score")]
    public float Score { get; set; }

    /// <summary>
    /// Algorithm that extracted this keyword (e.g., "yake", "rake").
    /// </summary>
    [JsonPropertyName("algorithm")]
    public string Algorithm { get; set; } = string.Empty;

    /// <summary>
    /// Optional positions where keyword appears in text (character offsets).
    /// </summary>
    [JsonPropertyName("positions")]
    public List<int>? Positions { get; set; }
}

/// <summary>
/// Represents a URI/link discovered during document extraction.
/// Alias for <see cref="ExtractedUri"/> for parity across language bindings.
/// </summary>
public sealed class Uri
{
    /// <summary>
    /// Semantic classification of the URI (hyperlink, image, anchor, citation, reference, email).
    /// </summary>
    [JsonPropertyName("kind")]
    public string Kind { get; init; } = string.Empty;

    /// <summary>
    /// Optional display text / label for the link.
    /// </summary>
    [JsonPropertyName("label")]
    public string? Label { get; init; }

    /// <summary>
    /// Optional page number where the URI was found (1-indexed).
    /// </summary>
    [JsonPropertyName("page")]
    public uint? Page { get; init; }

    /// <summary>
    /// The URL or path string.
    /// </summary>
    [JsonPropertyName("url")]
    public string Url { get; init; } = string.Empty;
}
