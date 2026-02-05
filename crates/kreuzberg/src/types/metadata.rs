//! Metadata types for extraction results.
//!
//! This module defines metadata structures for various document formats.

use std::borrow::Cow;

use ahash::AHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};

#[cfg(feature = "pdf")]
use crate::pdf::metadata::PdfMetadata;

use super::formats::ImagePreprocessingMetadata;
use super::page::PageStructure;

/// Custom serialization and deserialization for AHashMap<Cow<'static, str>, Value>.
///
/// serde doesn't natively support serializing Cow keys, so we convert to/from
/// a HashMap<String, Value> for the wire format, while keeping the in-memory
/// representation optimized with Cow keys (avoiding allocations for static strings).
mod additional_serde {
    use super::*;

    pub fn serialize<S>(map: &AHashMap<Cow<'static, str>, serde_json::Value>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert to HashMap for serialization
        let converted: HashMap<String, serde_json::Value> =
            map.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        converted.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AHashMap<Cow<'static, str>, serde_json::Value>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize from HashMap
        let map = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let result = map.into_iter().map(|(k, v)| (Cow::Owned(k), v)).collect();
        Ok(result)
    }
}

/// Format-specific metadata (discriminated union).
///
/// Only one format type can exist per extraction result. This provides
/// type-safe, clean metadata without nested optionals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
    #[cfg_attr(feature = "api", schema(value_type = HtmlMetadata))]
    Html(Box<HtmlMetadata>),
    Ocr(OcrMetadata),
}

/// Extraction result metadata.
///
/// Contains common fields applicable to all formats, format-specific metadata
/// via a discriminated union, and additional custom fields from postprocessors.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
    #[cfg_attr(feature = "api", schema(value_type = Option<Object>))]
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

    /// Extraction duration in milliseconds (for benchmarking).
    ///
    /// This field is populated by batch extraction to provide per-file timing
    /// information. It's `None` for single-file extraction (which uses external timing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_duration_ms: Option<u64>,

    /// Additional custom fields from postprocessors.
    ///
    /// This flattened map allows Python/TypeScript postprocessors to add
    /// arbitrary fields (entity extraction, keyword extraction, etc.).
    /// Fields are merged at the root level during serialization.
    /// Uses `Cow<'static, str>` keys so static string keys avoid allocation.
    #[serde(
        flatten,
        serialize_with = "additional_serde::serialize",
        deserialize_with = "additional_serde::deserialize"
    )]
    #[cfg_attr(feature = "api", schema(value_type = HashMap<String, serde_json::Value>))]
    pub additional: AHashMap<Cow<'static, str>, serde_json::Value>,
}

/// Excel/spreadsheet metadata.
///
/// Contains information about sheets in Excel, LibreOffice Calc, and other
/// spreadsheet formats (.xlsx, .xls, .ods, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ArchiveMetadata {
    /// Archive format ("ZIP", "TAR", "7Z", etc.)
    #[cfg_attr(feature = "api", schema(value_type = String))]
    pub format: Cow<'static, str>,
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
    pub attributes: Vec<(String, String)>,
}

/// Link type classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
    pub attributes: Vec<(String, String)>,
}

/// Image type classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ErrorMetadata {
    pub error_type: String,
    pub message: String,
}

/// PowerPoint presentation metadata.
///
/// Extracted from PPTX files containing slide counts and presentation details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PptxMetadata {
    /// Total number of slides in the presentation
    pub slide_count: usize,
    /// Names of slides (if available)
    pub slide_names: Vec<String>,
}
