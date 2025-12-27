use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[cfg(feature = "pdf")]
use crate::pdf::metadata::PdfMetadata;

// ============================================================================
// ============================================================================

/// Module providing transparent serde support for Arc<T>.
///
/// Allows Arc-wrapped types to serialize/deserialize as if unwrapped,
/// maintaining exact JSON format while preserving memory efficiency benefits.
///
/// # Arc Sharing Semantics
///
/// **Important**: Arc sharing semantics are **NOT** preserved across serialization.
/// When deserializing, each Arc is independently created with `Arc::new()`.
/// This means that if two Arcs referenced the same data before serialization,
/// they will be separate Arcs after deserialization.
///
/// Example:
/// ```ignore
/// let shared = Arc::new(Table { /* ... */ });
/// let tables = vec![Arc::clone(&shared), Arc::clone(&shared)];
/// // Both in-memory Arcs point to the same Table
///
/// let json = serde_json::to_string(&tables)?;
/// let deserialized: Vec<Arc<Table>> = serde_json::from_str(&json)?;
/// // deserialized[0] and deserialized[1] are now independent Arcs,
/// // even though they contain identical data
/// ```
///
/// This design choice maintains:
/// - Exact JSON format compatibility (no sharing metadata in JSON)
/// - Predictable deserialization behavior
/// - Zero additional serialization overhead
///
/// If in-memory sharing is required, callers must implement custom sharing logic
/// or use a different data structure (like a HashMap of deduplicated values).
#[allow(dead_code)]
mod serde_arc {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::sync::Arc;

    /// Serialize an Arc<T> by serializing the inner value directly.
    ///
    /// This makes Arc<T> serialize identically to T, maintaining API compatibility.
    /// The outer Arc wrapper is transparent during serialization.
    pub fn serialize<S, T>(arc_value: &Arc<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: serde::Serialize,
    {
        (**arc_value).serialize(serializer)
    }

    /// Deserialize a T and wrap it in Arc.
    ///
    /// This makes Arc<T> deserialize from the same format as T.
    /// Each Arc is independently created during deserialization;
    /// Arc sharing from before serialization is NOT preserved.
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Arc<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        T::deserialize(deserializer).map(Arc::new)
    }
}

/// Module for serializing Vec<Arc<T>> with transparent Arc handling.
///
/// Serializes a Vec<Arc<T>> as Vec<T> for compatibility, while preserving
/// Arc semantics for memory efficiency.
///
/// # Arc Sharing Semantics
///
/// **Important**: Arc sharing semantics are **NOT** preserved across serialization.
/// When deserializing, each element's Arc is independently created with `Arc::new()`.
/// This is important for `PageContent` where tables/images may be shared across pages.
///
/// Example with shared tables:
/// ```ignore
/// let shared_table = Arc::new(Table { /* ... */ });
/// let page_contents = vec![
///     PageContent { tables: vec![Arc::clone(&shared_table)], ... },
///     PageContent { tables: vec![Arc::clone(&shared_table)], ... },
/// ];
/// // In-memory: both pages' tables point to the same Arc
///
/// let json = serde_json::to_string(&page_contents)?;
/// let deserialized = serde_json::from_str::<Vec<PageContent>>(&json)?;
/// // After deserialization: each page has independent Arc instances,
/// // even though the table data is identical
/// ```
///
/// Design rationale:
/// - JSON has no mechanism to represent shared references
/// - Preserving sharing would require complex metadata and deduplication
/// - Current approach is simple, predictable, and maintains compatibility
/// - In-memory sharing (via Arc) is an implementation detail for the Rust side
///
/// If in-memory sharing is required after deserialization, implement custom
/// deduplication logic using hashing or content comparison.
mod serde_vec_arc {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::sync::Arc;

    /// Serialize Vec<Arc<T>> by serializing each T directly.
    ///
    /// Each element is unwrapped from its Arc and serialized independently.
    /// No sharing metadata is included in the serialized output.
    pub fn serialize<S, T>(vec: &[Arc<T>], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: serde::Serialize,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(vec.len()))?;
        for arc_item in vec {
            seq.serialize_element(&**arc_item)?;
        }
        seq.end()
    }

    /// Deserialize Vec<T> and wrap each element in Arc.
    ///
    /// Each element is independently wrapped in a new Arc.
    /// Sharing relationships from before serialization are lost.
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Vec<Arc<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let vec: Vec<T> = Deserialize::deserialize(deserializer)?;
        Ok(vec.into_iter().map(Arc::new).collect())
    }
}

/// General extraction result used by the core extraction API.
///
/// This is the main result type returned by all extraction functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: Metadata,
    pub tables: Vec<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_languages: Option<Vec<String>>,

    /// Text chunks when chunking is enabled.
    ///
    /// When chunking configuration is provided, the content is split into
    /// overlapping chunks for efficient processing. Each chunk contains the text,
    /// optional embeddings (if enabled), and metadata about its position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunks: Option<Vec<Chunk>>,

    /// Extracted images from the document.
    ///
    /// When image extraction is enabled via `ImageExtractionConfig`, this field
    /// contains all images found in the document with their raw data and metadata.
    /// Each image may optionally contain a nested `ocr_result` if OCR was performed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ExtractedImage>>,

    /// Per-page content when page extraction is enabled.
    ///
    /// When page extraction is configured, the document is split into per-page content
    /// with tables and images mapped to their respective pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<PageContent>>,
}

/// Format-specific metadata (discriminated union).
///
/// Only one format type can exist per extraction result. This provides
/// type-safe, clean metadata without nested optionals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "format_type", rename_all = "snake_case")]
pub enum FormatMetadata {
    #[cfg(feature = "pdf")]
    Pdf(PdfMetadata),
    Excel(ExcelMetadata),
    Email(EmailMetadata),
    Pptx(PptxMetadata),
    Archive(ArchiveMetadata),
    Image(ImageMetadata),
    Xml(XmlMetadata),
    Text(TextMetadata),
    Html(Box<HtmlMetadata>),
    Ocr(OcrMetadata),
}

/// Extraction result metadata.
///
/// Contains common fields applicable to all formats, format-specific metadata
/// via a discriminated union, and additional custom fields from postprocessors.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata {
    /// Document title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Document subject or description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Primary author(s) - always Vec for consistency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,

    /// Keywords/tags - always Vec for consistency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Primary language (ISO 639 code)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Creation timestamp (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last modification timestamp (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,

    /// User who created the document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// User who last modified the document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_by: Option<String>,

    /// Page/slide/sheet structure with boundaries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<PageStructure>,

    /// Format-specific metadata (discriminated union)
    ///
    /// Contains detailed metadata specific to the document format.
    /// Serializes with a `format_type` discriminator field.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub format: Option<FormatMetadata>,

    /// Image preprocessing metadata (when OCR preprocessing was applied)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_preprocessing: Option<ImagePreprocessingMetadata>,

    /// JSON schema (for structured data extraction)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,

    /// Error metadata (for batch operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorMetadata>,

    /// Additional custom fields from postprocessors.
    ///
    /// This flattened HashMap allows Python/TypeScript postprocessors to add
    /// arbitrary fields (entity extraction, keyword extraction, etc.).
    /// Fields are merged at the root level during serialization.
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

/// Unified page structure for documents.
///
/// Supports different page types (PDF pages, PPTX slides, Excel sheets)
/// with character offset boundaries for chunk-to-page mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageStructure {
    /// Total number of pages/slides/sheets
    pub total_count: usize,

    /// Type of paginated unit
    pub unit_type: PageUnitType,

    /// Character offset boundaries for each page
    ///
    /// Maps character ranges in the extracted content to page numbers.
    /// Used for chunk page range calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundaries: Option<Vec<PageBoundary>>,

    /// Detailed per-page metadata (optional, only when needed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<PageInfo>>,
}

/// Type of paginated unit in a document.
///
/// Distinguishes between different types of "pages" (PDF pages, presentation slides, spreadsheet sheets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageUnitType {
    /// Standard document pages (PDF, DOCX, images)
    Page,
    /// Presentation slides (PPTX, ODP)
    Slide,
    /// Spreadsheet sheets (XLSX, ODS)
    Sheet,
}

/// Byte offset boundary for a page.
///
/// Tracks where a specific page's content starts and ends in the main content string,
/// enabling mapping from byte positions to page numbers. Offsets are guaranteed to be
/// at valid UTF-8 character boundaries when using standard String methods (push_str, push, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageBoundary {
    /// Byte offset where this page starts in the content string (UTF-8 valid boundary, inclusive)
    pub byte_start: usize,
    /// Byte offset where this page ends in the content string (UTF-8 valid boundary, exclusive)
    pub byte_end: usize,
    /// Page number (1-indexed)
    pub page_number: usize,
}

/// Metadata for individual page/slide/sheet.
///
/// Captures per-page information including dimensions, content counts,
/// and visibility state (for presentations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    /// Page number (1-indexed)
    pub number: usize,

    /// Page title (usually for presentations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Dimensions in points (PDF) or pixels (images): (width, height)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<(f64, f64)>,

    /// Number of images on this page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_count: Option<usize>,

    /// Number of tables on this page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_count: Option<usize>,

    /// Whether this page is hidden (e.g., in presentations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

/// Content for a single page/slide.
///
/// When page extraction is enabled, documents are split into per-page content
/// with associated tables and images mapped to each page.
///
/// # Performance
///
/// Uses Arc-wrapped tables and images for memory efficiency:
/// - `Vec<Arc<Table>>` enables zero-copy sharing of table data
/// - `Vec<Arc<ExtractedImage>>` enables zero-copy sharing of image data
/// - Maintains exact JSON compatibility via custom Serialize/Deserialize
///
/// This reduces memory overhead for documents with shared tables/images
/// by avoiding redundant copies during serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContent {
    /// Page number (1-indexed)
    pub page_number: usize,

    /// Text content for this page
    pub content: String,

    /// Tables found on this page (uses Arc for memory efficiency)
    ///
    /// Serializes as Vec<Table> for JSON compatibility while maintaining
    /// Arc semantics in-memory for zero-copy sharing.
    #[serde(skip_serializing_if = "Vec::is_empty", default, with = "serde_vec_arc")]
    pub tables: Vec<Arc<Table>>,

    /// Images found on this page (uses Arc for memory efficiency)
    ///
    /// Serializes as Vec<ExtractedImage> for JSON compatibility while maintaining
    /// Arc semantics in-memory for zero-copy sharing.
    #[serde(skip_serializing_if = "Vec::is_empty", default, with = "serde_vec_arc")]
    pub images: Vec<Arc<ExtractedImage>>,
}

/// Excel/spreadsheet metadata.
///
/// Contains information about sheets in Excel, LibreOffice Calc, and other
/// spreadsheet formats (.xlsx, .xls, .ods, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelMetadata {
    /// Total number of sheets in the workbook
    pub sheet_count: usize,
    /// Names of all sheets in order
    pub sheet_names: Vec<String>,
}

/// Email metadata extracted from .eml and .msg files.
///
/// Includes sender/recipient information, message ID, and attachment list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMetadata {
    /// Sender's email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_email: Option<String>,

    /// Sender's display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_name: Option<String>,

    /// Primary recipients
    pub to_emails: Vec<String>,
    /// CC recipients
    pub cc_emails: Vec<String>,
    /// BCC recipients
    pub bcc_emails: Vec<String>,

    /// Message-ID header value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// List of attachment filenames
    pub attachments: Vec<String>,
}

/// Archive (ZIP/TAR/7Z) metadata.
///
/// Extracted from compressed archive files containing file lists and size information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// Archive format ("ZIP", "TAR", "7Z", etc.)
    pub format: String,
    /// Total number of files in the archive
    pub file_count: usize,
    /// List of file paths within the archive
    pub file_list: Vec<String>,
    /// Total uncompressed size in bytes
    pub total_size: usize,

    /// Compressed size in bytes (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed_size: Option<usize>,
}

/// Image metadata extracted from image files.
///
/// Includes dimensions, format, and EXIF data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Image format (e.g., "PNG", "JPEG", "TIFF")
    pub format: String,
    /// EXIF metadata tags
    pub exif: HashMap<String, String>,
}

/// XML metadata extracted during XML parsing.
///
/// Provides statistics about XML document structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlMetadata {
    /// Total number of XML elements processed
    pub element_count: usize,
    /// List of unique element tag names (sorted)
    pub unique_elements: Vec<String>,
}

/// Text/Markdown metadata.
///
/// Extracted from plain text and Markdown files. Includes word counts and,
/// for Markdown, structural elements like headers and links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMetadata {
    /// Number of lines in the document
    pub line_count: usize,
    /// Number of words
    pub word_count: usize,
    /// Number of characters
    pub character_count: usize,

    /// Markdown headers (headings text only, for Markdown files)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<String>>,

    /// Markdown links as (text, url) tuples (for Markdown files)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<(String, String)>>,

    /// Code blocks as (language, code) tuples (for Markdown files)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_blocks: Option<Vec<(String, String)>>,
}

/// Text direction enumeration for HTML documents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextDirection {
    /// Left-to-right text direction
    #[serde(rename = "ltr")]
    LeftToRight,
    /// Right-to-left text direction
    #[serde(rename = "rtl")]
    RightToLeft,
    /// Automatic text direction detection
    #[serde(rename = "auto")]
    Auto,
}

/// Header/heading element metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderMetadata {
    /// Header level: 1 (h1) through 6 (h6)
    pub level: u8,
    /// Normalized text content of the header
    pub text: String,
    /// HTML id attribute if present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Document tree depth at the header element
    pub depth: usize,
    /// Byte offset in original HTML document
    pub html_offset: usize,
}

/// Link element metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMetadata {
    /// The href URL value
    pub href: String,
    /// Link text content (normalized)
    pub text: String,
    /// Optional title attribute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Link type classification
    pub link_type: LinkType,
    /// Rel attribute values
    pub rel: Vec<String>,
    /// Additional attributes as key-value pairs
    pub attributes: HashMap<String, String>,
}

/// Link type classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    /// Anchor link (#section)
    Anchor,
    /// Internal link (same domain)
    Internal,
    /// External link (different domain)
    External,
    /// Email link (mailto:)
    Email,
    /// Phone link (tel:)
    Phone,
    /// Other link type
    Other,
}

/// Image element metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadataType {
    /// Image source (URL, data URI, or SVG content)
    pub src: String,
    /// Alternative text from alt attribute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// Title attribute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Image dimensions as (width, height) if available
    pub dimensions: Option<(u32, u32)>,
    /// Image type classification
    pub image_type: ImageType,
    /// Additional attributes as key-value pairs
    pub attributes: HashMap<String, String>,
}

/// Image type classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageType {
    /// Data URI image
    #[serde(rename = "data-uri")]
    DataUri,
    /// Inline SVG
    #[serde(rename = "inline-svg")]
    InlineSvg,
    /// External image URL
    External,
    /// Relative path image
    Relative,
}

/// Structured data (Schema.org, microdata, RDFa) block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredData {
    /// Type of structured data
    pub data_type: StructuredDataType,
    /// Raw JSON string representation
    pub raw_json: String,
    /// Schema type if detectable (e.g., "Article", "Event", "Product")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,
}

/// Structured data type classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StructuredDataType {
    /// JSON-LD structured data
    #[serde(rename = "json-ld")]
    JsonLd,
    /// Microdata
    Microdata,
    /// RDFa
    #[serde(rename = "rdfa")]
    RDFa,
}

/// HTML metadata extracted from HTML documents.
///
/// Includes document-level metadata, Open Graph data, Twitter Card metadata,
/// and extracted structural elements (headers, links, images, structured data).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HtmlMetadata {
    /// Document title from `<title>` tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Document description from `<meta name="description">` tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Document keywords from `<meta name="keywords">` tag, split on commas
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Document author from `<meta name="author">` tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Canonical URL from `<link rel="canonical">` tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_url: Option<String>,

    /// Base URL from `<base href="">` tag for resolving relative URLs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_href: Option<String>,

    /// Document language from `lang` attribute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Document text direction from `dir` attribute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_direction: Option<TextDirection>,

    /// Open Graph metadata (og:* properties) for social media
    /// Keys like "title", "description", "image", "url", etc.
    #[serde(default)]
    pub open_graph: BTreeMap<String, String>,

    /// Twitter Card metadata (twitter:* properties)
    /// Keys like "card", "site", "creator", "title", "description", "image", etc.
    #[serde(default)]
    pub twitter_card: BTreeMap<String, String>,

    /// Additional meta tags not covered by specific fields
    /// Keys are meta name/property attributes, values are content
    #[serde(default)]
    pub meta_tags: BTreeMap<String, String>,

    /// Extracted header elements with hierarchy
    #[serde(default)]
    pub headers: Vec<HeaderMetadata>,

    /// Extracted hyperlinks with type classification
    #[serde(default)]
    pub links: Vec<LinkMetadata>,

    /// Extracted images with source and dimensions
    #[serde(default)]
    pub images: Vec<ImageMetadataType>,

    /// Extracted structured data blocks
    #[serde(default)]
    pub structured_data: Vec<StructuredData>,
}

impl HtmlMetadata {
    /// Check if metadata is empty (no meaningful content extracted).
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.keywords.is_empty()
            && self.author.is_none()
            && self.canonical_url.is_none()
            && self.base_href.is_none()
            && self.language.is_none()
            && self.text_direction.is_none()
            && self.open_graph.is_empty()
            && self.twitter_card.is_empty()
            && self.meta_tags.is_empty()
            && self.headers.is_empty()
            && self.links.is_empty()
            && self.images.is_empty()
            && self.structured_data.is_empty()
    }
}

#[cfg(feature = "html")]
impl From<html_to_markdown_rs::ExtendedMetadata> for HtmlMetadata {
    fn from(metadata: html_to_markdown_rs::ExtendedMetadata) -> Self {
        let text_dir = metadata.document.text_direction.map(|td| match td {
            html_to_markdown_rs::TextDirection::LeftToRight => TextDirection::LeftToRight,
            html_to_markdown_rs::TextDirection::RightToLeft => TextDirection::RightToLeft,
            html_to_markdown_rs::TextDirection::Auto => TextDirection::Auto,
        });

        HtmlMetadata {
            title: metadata.document.title,
            description: metadata.document.description,
            keywords: metadata.document.keywords,
            author: metadata.document.author,
            canonical_url: metadata.document.canonical_url,
            base_href: metadata.document.base_href,
            language: metadata.document.language,
            text_direction: text_dir,
            open_graph: metadata.document.open_graph,
            twitter_card: metadata.document.twitter_card,
            meta_tags: metadata.document.meta_tags,
            headers: metadata
                .headers
                .into_iter()
                .map(|h| HeaderMetadata {
                    level: h.level,
                    text: h.text,
                    id: h.id,
                    depth: h.depth,
                    html_offset: h.html_offset,
                })
                .collect(),
            links: metadata
                .links
                .into_iter()
                .map(|l| LinkMetadata {
                    href: l.href,
                    text: l.text,
                    title: l.title,
                    link_type: match l.link_type {
                        html_to_markdown_rs::LinkType::Anchor => LinkType::Anchor,
                        html_to_markdown_rs::LinkType::Internal => LinkType::Internal,
                        html_to_markdown_rs::LinkType::External => LinkType::External,
                        html_to_markdown_rs::LinkType::Email => LinkType::Email,
                        html_to_markdown_rs::LinkType::Phone => LinkType::Phone,
                        html_to_markdown_rs::LinkType::Other => LinkType::Other,
                    },
                    rel: l.rel,
                    attributes: l.attributes.into_iter().collect(),
                })
                .collect(),
            images: metadata
                .images
                .into_iter()
                .map(|img| ImageMetadataType {
                    src: img.src,
                    alt: img.alt,
                    title: img.title,
                    dimensions: img.dimensions,
                    image_type: match img.image_type {
                        html_to_markdown_rs::ImageType::DataUri => ImageType::DataUri,
                        html_to_markdown_rs::ImageType::InlineSvg => ImageType::InlineSvg,
                        html_to_markdown_rs::ImageType::External => ImageType::External,
                        html_to_markdown_rs::ImageType::Relative => ImageType::Relative,
                    },
                    attributes: img.attributes.into_iter().collect(),
                })
                .collect(),
            structured_data: metadata
                .structured_data
                .into_iter()
                .map(|sd| StructuredData {
                    data_type: match sd.data_type {
                        html_to_markdown_rs::StructuredDataType::JsonLd => StructuredDataType::JsonLd,
                        html_to_markdown_rs::StructuredDataType::Microdata => StructuredDataType::Microdata,
                        html_to_markdown_rs::StructuredDataType::RDFa => StructuredDataType::RDFa,
                    },
                    raw_json: sd.raw_json,
                    schema_type: sd.schema_type,
                })
                .collect(),
        }
    }
}

/// OCR processing metadata.
///
/// Captures information about OCR processing configuration and results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrMetadata {
    /// OCR language code(s) used
    pub language: String,
    /// Tesseract Page Segmentation Mode (PSM)
    pub psm: i32,
    /// Output format (e.g., "text", "hocr")
    pub output_format: String,
    /// Number of tables detected
    pub table_count: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_rows: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_cols: Option<usize>,
}

/// Error metadata (for batch operations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetadata {
    pub error_type: String,
    pub message: String,
}

/// Extracted table structure.
///
/// Represents a table detected and extracted from a document (PDF, image, etc.).
/// Tables are converted to both structured cell data and Markdown format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Table cells as a 2D vector (rows × columns)
    pub cells: Vec<Vec<String>>,
    /// Markdown representation of the table
    pub markdown: String,
    /// Page number where the table was found (1-indexed)
    pub page_number: usize,
}

/// A text chunk with optional embedding and metadata.
///
/// Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
/// contains the text content, optional embedding vector (if embedding generation
/// is configured), and metadata about its position in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// The text content of this chunk.
    pub content: String,

    /// Optional embedding vector for this chunk.
    ///
    /// Only populated when `EmbeddingConfig` is provided in chunking configuration.
    /// The dimensionality depends on the chosen embedding model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,

    /// Metadata about this chunk's position and properties.
    pub metadata: ChunkMetadata,
}

/// Metadata about a chunk's position in the original document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Byte offset where this chunk starts in the original text (UTF-8 valid boundary).
    pub byte_start: usize,

    /// Byte offset where this chunk ends in the original text (UTF-8 valid boundary).
    pub byte_end: usize,

    /// Number of tokens in this chunk (if available).
    ///
    /// This is calculated by the embedding model's tokenizer if embeddings are enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<usize>,

    /// Zero-based index of this chunk in the document.
    pub chunk_index: usize,

    /// Total number of chunks in the document.
    pub total_chunks: usize,

    /// First page number this chunk spans (1-indexed).
    ///
    /// Only populated when page tracking is enabled in extraction configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_page: Option<usize>,

    /// Last page number this chunk spans (1-indexed, equal to first_page for single-page chunks).
    ///
    /// Only populated when page tracking is enabled in extraction configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_page: Option<usize>,
}

/// Extracted image from a document.
///
/// Contains raw image data, metadata, and optional nested OCR results.
/// Raw bytes allow cross-language compatibility - users can convert to
/// PIL.Image (Python), Sharp (Node.js), or other formats as needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedImage {
    /// Raw image data (PNG, JPEG, WebP, etc. bytes)
    pub data: Vec<u8>,

    /// Image format (e.g., "jpeg", "png", "webp")
    pub format: String,

    /// Zero-indexed position of this image in the document/page
    pub image_index: usize,

    /// Page/slide number where image was found (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<usize>,

    /// Image width in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,

    /// Image height in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,

    /// Colorspace information (e.g., "RGB", "CMYK", "Gray")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colorspace: Option<String>,

    /// Bits per color component (e.g., 8, 16)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bits_per_component: Option<u32>,

    /// Whether this image is a mask image
    #[serde(default)]
    pub is_mask: bool,

    /// Optional description of the image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Nested OCR extraction result (if image was OCRed)
    ///
    /// When OCR is performed on this image, the result is embedded here
    /// rather than in a separate collection, making the relationship explicit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr_result: Option<Box<ExtractionResult>>,
}

/// Excel workbook representation.
///
/// Contains all sheets from an Excel file (.xlsx, .xls, etc.) with
/// extracted content and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelWorkbook {
    /// All sheets in the workbook
    pub sheets: Vec<ExcelSheet>,
    /// Workbook-level metadata (author, creation date, etc.)
    pub metadata: HashMap<String, String>,
}

/// Single Excel worksheet.
///
/// Represents one sheet from an Excel workbook with its content
/// converted to Markdown format and dimensional statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelSheet {
    /// Sheet name as it appears in Excel
    pub name: String,
    /// Sheet content converted to Markdown tables
    pub markdown: String,
    /// Number of rows
    pub row_count: usize,
    /// Number of columns
    pub col_count: usize,
    /// Total number of non-empty cells
    pub cell_count: usize,
    /// Pre-extracted table cells (2D vector of cell values)
    /// Populated during markdown generation to avoid re-parsing markdown.
    /// None for empty sheets.
    #[serde(skip)]
    pub table_cells: Option<Vec<Vec<String>>>,
}

/// XML extraction result.
///
/// Contains extracted text content from XML files along with
/// structural statistics about the XML document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlExtractionResult {
    /// Extracted text content (XML structure filtered out)
    pub content: String,
    /// Total number of XML elements processed
    pub element_count: usize,
    /// List of unique element names found (sorted)
    pub unique_elements: Vec<String>,
}

/// Plain text and Markdown extraction result.
///
/// Contains the extracted text along with statistics and,
/// for Markdown files, structural elements like headers and links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextExtractionResult {
    /// Extracted text content
    pub content: String,
    /// Number of lines
    pub line_count: usize,
    /// Number of words
    pub word_count: usize,
    /// Number of characters
    pub character_count: usize,
    /// Markdown headers (text only, Markdown files only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<String>>,
    /// Markdown links as (text, URL) tuples (Markdown files only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<(String, String)>>,
    /// Code blocks as (language, code) tuples (Markdown files only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_blocks: Option<Vec<(String, String)>>,
}

/// PowerPoint (PPTX) extraction result.
///
/// Contains extracted slide content, metadata, and embedded images/tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptxExtractionResult {
    /// Extracted text content from all slides
    pub content: String,
    /// Presentation metadata
    pub metadata: PptxMetadata,
    /// Total number of slides
    pub slide_count: usize,
    /// Total number of embedded images
    pub image_count: usize,
    /// Total number of tables
    pub table_count: usize,
    /// Extracted images from the presentation
    pub images: Vec<ExtractedImage>,
    /// Slide structure with boundaries (when page tracking is enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_structure: Option<PageStructure>,
    /// Per-slide content (when page tracking is enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_contents: Option<Vec<PageContent>>,
}

/// PowerPoint presentation metadata.
///
/// Contains PPTX-specific metadata. Common fields like title, author, and description
/// are now in the base `Metadata` struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptxMetadata {
    /// List of fonts used in the presentation
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fonts: Vec<String>,
}

/// Email extraction result.
///
/// Complete representation of an extracted email message (.eml or .msg)
/// including headers, body content, and attachments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailExtractionResult {
    /// Email subject line
    pub subject: Option<String>,
    /// Sender email address
    pub from_email: Option<String>,
    /// Primary recipient email addresses
    pub to_emails: Vec<String>,
    /// CC recipient email addresses
    pub cc_emails: Vec<String>,
    /// BCC recipient email addresses
    pub bcc_emails: Vec<String>,
    /// Email date/timestamp
    pub date: Option<String>,
    /// Message-ID header value
    pub message_id: Option<String>,
    /// Plain text version of the email body
    pub plain_text: Option<String>,
    /// HTML version of the email body
    pub html_content: Option<String>,
    /// Cleaned/processed text content
    pub cleaned_text: String,
    /// List of email attachments
    pub attachments: Vec<EmailAttachment>,
    /// Additional email headers and metadata
    pub metadata: HashMap<String, String>,
}

/// Email attachment representation.
///
/// Contains metadata and optionally the content of an email attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAttachment {
    /// Attachment name (from Content-Disposition header)
    pub name: Option<String>,
    /// Filename of the attachment
    pub filename: Option<String>,
    /// MIME type of the attachment
    pub mime_type: Option<String>,
    /// Size in bytes
    pub size: Option<usize>,
    /// Whether this attachment is an image
    pub is_image: bool,
    /// Attachment data (if extracted)
    pub data: Option<Vec<u8>>,
}

/// OCR extraction result.
///
/// Result of performing OCR on an image or scanned document,
/// including recognized text and detected tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrExtractionResult {
    /// Recognized text content
    pub content: String,
    /// Original MIME type of the processed image
    pub mime_type: String,
    /// OCR processing metadata (confidence scores, language, etc.)
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tables detected and extracted via OCR
    pub tables: Vec<OcrTable>,
}

/// Table detected via OCR.
///
/// Represents a table structure recognized during OCR processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrTable {
    /// Table cells as a 2D vector (rows × columns)
    pub cells: Vec<Vec<String>>,
    /// Markdown representation of the table
    pub markdown: String,
    /// Page number where the table was found (1-indexed)
    pub page_number: usize,
}

/// Image preprocessing configuration for OCR.
///
/// These settings control how images are preprocessed before OCR to improve
/// text recognition quality. Different preprocessing strategies work better
/// for different document types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ImagePreprocessingConfig {
    /// Target DPI for the image (300 is standard, 600 for small text).
    pub target_dpi: i32,

    /// Auto-detect and correct image rotation.
    pub auto_rotate: bool,

    /// Correct skew (tilted images).
    pub deskew: bool,

    /// Remove noise from the image.
    pub denoise: bool,

    /// Enhance contrast for better text visibility.
    pub contrast_enhance: bool,

    /// Binarization method: "otsu", "sauvola", "adaptive".
    pub binarization_method: String,

    /// Invert colors (white text on black → black on white).
    pub invert_colors: bool,
}

impl Default for ImagePreprocessingConfig {
    fn default() -> Self {
        Self {
            target_dpi: 300,
            auto_rotate: true,
            deskew: true,
            denoise: false,
            contrast_enhance: false,
            binarization_method: "otsu".to_string(),
            invert_colors: false,
        }
    }
}

/// Tesseract OCR configuration.
///
/// Provides fine-grained control over Tesseract OCR engine parameters.
/// Most users can use the defaults, but these settings allow optimization
/// for specific document types (invoices, handwriting, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TesseractConfig {
    /// Language code (e.g., "eng", "deu", "fra")
    pub language: String,

    /// Page Segmentation Mode (0-13).
    ///
    /// Common values:
    /// - 3: Fully automatic page segmentation (default)
    /// - 6: Assume a single uniform block of text
    /// - 11: Sparse text with no particular order
    pub psm: i32,

    /// Output format ("text" or "markdown")
    pub output_format: String,

    /// OCR Engine Mode (0-3).
    ///
    /// - 0: Legacy engine only
    /// - 1: Neural nets (LSTM) only (usually best)
    /// - 2: Legacy + LSTM
    /// - 3: Default (based on what's available)
    pub oem: i32,

    /// Minimum confidence threshold (0.0-100.0).
    ///
    /// Words with confidence below this threshold may be rejected or flagged.
    pub min_confidence: f64,

    /// Image preprocessing configuration.
    ///
    /// Controls how images are preprocessed before OCR. Can significantly
    /// improve quality for scanned documents or low-quality images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preprocessing: Option<ImagePreprocessingConfig>,

    /// Enable automatic table detection and reconstruction
    pub enable_table_detection: bool,

    /// Minimum confidence threshold for table detection (0.0-1.0)
    pub table_min_confidence: f64,

    /// Column threshold for table detection (pixels)
    pub table_column_threshold: i32,

    /// Row threshold ratio for table detection (0.0-1.0)
    pub table_row_threshold_ratio: f64,

    /// Enable OCR result caching
    pub use_cache: bool,

    /// Use pre-adapted templates for character classification
    pub classify_use_pre_adapted_templates: bool,

    /// Enable N-gram language model
    pub language_model_ngram_on: bool,

    /// Don't reject good words during block-level processing
    pub tessedit_dont_blkrej_good_wds: bool,

    /// Don't reject good words during row-level processing
    pub tessedit_dont_rowrej_good_wds: bool,

    /// Enable dictionary correction
    pub tessedit_enable_dict_correction: bool,

    /// Whitelist of allowed characters (empty = all allowed)
    pub tessedit_char_whitelist: String,

    /// Blacklist of forbidden characters (empty = none forbidden)
    pub tessedit_char_blacklist: String,

    /// Use primary language params model
    pub tessedit_use_primary_params_model: bool,

    /// Variable-width space detection
    pub textord_space_size_is_variable: bool,

    /// Use adaptive thresholding method
    pub thresholding_method: bool,
}

impl Default for TesseractConfig {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            psm: 3,
            output_format: "markdown".to_string(),
            oem: 3,
            min_confidence: 0.0,
            preprocessing: None,
            enable_table_detection: true,
            table_min_confidence: 0.0,
            table_column_threshold: 50,
            table_row_threshold_ratio: 0.5,
            use_cache: true,
            classify_use_pre_adapted_templates: true,
            language_model_ngram_on: false,
            tessedit_dont_blkrej_good_wds: true,
            tessedit_dont_rowrej_good_wds: true,
            tessedit_enable_dict_correction: true,
            tessedit_char_whitelist: String::new(),
            tessedit_char_blacklist: String::new(),
            tessedit_use_primary_params_model: true,
            textord_space_size_is_variable: true,
            thresholding_method: false,
        }
    }
}

/// Image preprocessing metadata.
///
/// Tracks the transformations applied to an image during OCR preprocessing,
/// including DPI normalization, resizing, and resampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePreprocessingMetadata {
    /// Original image dimensions (width, height) in pixels
    pub original_dimensions: (usize, usize),
    /// Original image DPI (horizontal, vertical)
    pub original_dpi: (f64, f64),
    /// Target DPI from configuration
    pub target_dpi: i32,
    /// Scaling factor applied to the image
    pub scale_factor: f64,
    /// Whether DPI was auto-adjusted based on content
    pub auto_adjusted: bool,
    /// Final DPI after processing
    pub final_dpi: i32,
    /// New dimensions after resizing (if resized)
    pub new_dimensions: Option<(usize, usize)>,
    /// Resampling algorithm used ("LANCZOS3", "CATMULLROM", etc.)
    pub resample_method: String,
    /// Whether dimensions were clamped to max_image_dimension
    pub dimension_clamped: bool,
    /// Calculated optimal DPI (if auto_adjust_dpi enabled)
    pub calculated_dpi: Option<i32>,
    /// Whether resize was skipped (dimensions already optimal)
    pub skipped_resize: bool,
    /// Error message if resize failed
    pub resize_error: Option<String>,
}

/// Image extraction configuration (internal use).
///
/// **Note:** This is an internal type used for image preprocessing.
/// For the main extraction configuration, see [`crate::core::config::ExtractionConfig`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Target DPI for image normalization
    pub target_dpi: i32,
    /// Maximum image dimension (width or height)
    pub max_image_dimension: i32,
    /// Whether to auto-adjust DPI based on content
    pub auto_adjust_dpi: bool,
    /// Minimum DPI threshold
    pub min_dpi: i32,
    /// Maximum DPI threshold
    pub max_dpi: i32,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            target_dpi: 300,
            max_image_dimension: 4096,
            auto_adjust_dpi: true,
            min_dpi: 72,
            max_dpi: 600,
        }
    }
}

/// Cache statistics.
///
/// Provides information about the extraction result cache,
/// including size, file count, and age distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cached files
    pub total_files: usize,
    /// Total cache size in megabytes
    pub total_size_mb: f64,
    /// Available disk space in megabytes
    pub available_space_mb: f64,
    /// Age of the oldest cached file in days
    pub oldest_file_age_days: f64,
    /// Age of the newest cached file in days
    pub newest_file_age_days: f64,
}

/// LibreOffice conversion result.
///
/// Result of converting a legacy office document (e.g., .doc, .ppt)
/// to a modern format using LibreOffice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibreOfficeConversionResult {
    /// Converted file bytes
    pub converted_bytes: Vec<u8>,
    /// Original format identifier
    pub original_format: String,
    /// Target format identifier
    pub target_format: String,
    /// Target MIME type after conversion
    pub target_mime: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_serialization_with_format() {
        let mut metadata = Metadata {
            format: Some(FormatMetadata::Text(TextMetadata {
                line_count: 1,
                word_count: 2,
                character_count: 13,
                headers: None,
                links: None,
                code_blocks: None,
            })),
            ..Default::default()
        };

        metadata
            .additional
            .insert("quality_score".to_string(), serde_json::json!(1.0));

        let json = serde_json::to_value(&metadata).unwrap();
        println!("Serialized metadata: {}", serde_json::to_string_pretty(&json).unwrap());

        assert!(
            json.get("format_type").is_some(),
            "format_type should be present in serialized JSON"
        );
        assert_eq!(json.get("format_type").unwrap(), "text");

        assert_eq!(json.get("line_count").unwrap(), 1);
        assert_eq!(json.get("word_count").unwrap(), 2);
        assert_eq!(json.get("character_count").unwrap(), 13);

        assert_eq!(json.get("quality_score").unwrap(), 1.0);
    }

    #[test]
    fn test_arc_table_serialization_format() {
        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |\n|---|---|\n".to_string(),
            page_number: 1,
        };

        let json = serde_json::to_value(&table).unwrap();

        assert_eq!(json.get("cells").unwrap()[0][0], "A");
        assert_eq!(json.get("markdown").unwrap(), "| A | B |\n|---|---|\n");
        assert_eq!(json.get("page_number").unwrap(), 1);
    }

    #[test]
    fn test_arc_table_roundtrip() {
        let original = Table {
            cells: vec![
                vec!["X".to_string(), "Y".to_string()],
                vec!["1".to_string(), "2".to_string()],
            ],
            markdown: "| X | Y |\n|---|---|\n| 1 | 2 |\n".to_string(),
            page_number: 5,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Table = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.cells, original.cells);
        assert_eq!(deserialized.markdown, original.markdown);
        assert_eq!(deserialized.page_number, original.page_number);
    }

    #[test]
    fn test_arc_sharing_preserved_before_serialization() {
        let shared_table = Arc::new(Table {
            cells: vec![vec!["shared".to_string()]],
            markdown: "| shared |".to_string(),
            page_number: 1,
        });

        let tables_before = vec![Arc::clone(&shared_table), Arc::clone(&shared_table)];
        assert_eq!(Arc::strong_count(&tables_before[0]), 3);
        assert_eq!(Arc::strong_count(&tables_before[1]), 3);
        assert!(Arc::ptr_eq(&tables_before[0], &tables_before[1]));
    }

    #[test]
    fn test_vec_arc_table_serialization_format() {
        let tables = vec![
            Table {
                cells: vec![vec!["A".to_string()]],
                markdown: "| A |".to_string(),
                page_number: 1,
            },
            Table {
                cells: vec![vec!["B".to_string()]],
                markdown: "| B |".to_string(),
                page_number: 2,
            },
        ];

        let json = serde_json::to_string(&tables).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[0]["cells"][0][0], "A");
        assert_eq!(parsed[1]["cells"][0][0], "B");
    }

    #[test]
    fn test_page_content_arc_tables_roundtrip() {
        let page = PageContent {
            page_number: 3,
            content: "Page 3 content".to_string(),
            tables: vec![
                Arc::new(Table {
                    cells: vec![vec!["Table1".to_string()]],
                    markdown: "| Table1 |".to_string(),
                    page_number: 3,
                }),
                Arc::new(Table {
                    cells: vec![vec!["Table2".to_string()]],
                    markdown: "| Table2 |".to_string(),
                    page_number: 3,
                }),
            ],
            images: Vec::new(),
        };

        let json = serde_json::to_string(&page).unwrap();
        let deserialized: PageContent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.page_number, 3);
        assert_eq!(deserialized.content, "Page 3 content");
        assert_eq!(deserialized.tables.len(), 2);
        assert_eq!(deserialized.tables[0].cells[0][0], "Table1");
        assert_eq!(deserialized.tables[1].cells[0][0], "Table2");
    }

    #[test]
    fn test_page_content_arc_images_roundtrip() {
        let image1 = Arc::new(ExtractedImage {
            data: vec![0xFF, 0xD8, 0xFF],
            format: "jpeg".to_string(),
            image_index: 0,
            page_number: Some(1),
            width: Some(100),
            height: Some(200),
            colorspace: Some("RGB".to_string()),
            bits_per_component: Some(8),
            is_mask: false,
            description: Some("Image 1".to_string()),
            ocr_result: None,
        });

        let image2 = Arc::new(ExtractedImage {
            data: vec![0x89, 0x50, 0x4E],
            format: "png".to_string(),
            image_index: 1,
            page_number: Some(1),
            width: Some(300),
            height: Some(400),
            colorspace: Some("RGBA".to_string()),
            bits_per_component: Some(8),
            is_mask: false,
            description: Some("Image 2".to_string()),
            ocr_result: None,
        });

        let page = PageContent {
            page_number: 1,
            content: "Page with images".to_string(),
            tables: Vec::new(),
            images: vec![image1, image2],
        };

        let json = serde_json::to_string(&page).unwrap();
        let deserialized: PageContent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.images.len(), 2);
        assert_eq!(deserialized.images[0].format, "jpeg");
        assert_eq!(deserialized.images[0].width, Some(100));
        assert_eq!(deserialized.images[1].format, "png");
        assert_eq!(deserialized.images[1].height, Some(400));
    }

    #[test]
    fn test_arc_sharing_loss_with_page_content() {
        let shared_table = Arc::new(Table {
            cells: vec![vec!["shared across pages".to_string()]],
            markdown: "| shared across pages |".to_string(),
            page_number: 0,
        });

        let page1 = PageContent {
            page_number: 1,
            content: "Page 1".to_string(),
            tables: vec![Arc::clone(&shared_table)],
            images: Vec::new(),
        };

        let page2 = PageContent {
            page_number: 2,
            content: "Page 2".to_string(),
            tables: vec![Arc::clone(&shared_table)],
            images: Vec::new(),
        };

        assert!(Arc::ptr_eq(&page1.tables[0], &page2.tables[0]));

        let pages = vec![page1, page2];
        let json = serde_json::to_string(&pages).unwrap();
        let deserialized: Vec<PageContent> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].tables[0].cells, deserialized[1].tables[0].cells);
        assert!(!Arc::ptr_eq(&deserialized[0].tables[0], &deserialized[1].tables[0]));
    }

    #[test]
    fn test_empty_page_content_arcs() {
        let page = PageContent {
            page_number: 5,
            content: "No tables or images".to_string(),
            tables: Vec::new(),
            images: Vec::new(),
        };

        let json = serde_json::to_string(&page).unwrap();
        let deserialized: PageContent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.page_number, 5);
        assert_eq!(deserialized.tables.len(), 0);
        assert_eq!(deserialized.images.len(), 0);
    }

    #[test]
    fn test_serde_vec_arc_module_behavior() {
        let table1 = Table {
            cells: vec![vec!["A".to_string()]],
            markdown: "| A |".to_string(),
            page_number: 1,
        };

        let table2 = Table {
            cells: vec![vec!["B".to_string()]],
            markdown: "| B |".to_string(),
            page_number: 2,
        };

        let json = serde_json::to_string(&vec![table1, table2]).unwrap();
        assert!(json.contains("\"A\""));
        assert!(json.contains("\"B\""));
    }
}
