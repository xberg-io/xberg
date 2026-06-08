//! Metadata types for extraction results.
//!
//! This module defines metadata structures for various document formats.

use std::borrow::Cow;

use ahash::AHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};

#[cfg(feature = "pdf")]
pub use crate::pdf::metadata::PdfMetadata;

use super::formats::ImagePreprocessingMetadata;
use super::page::PageStructure;

/// Wrapper for tree-sitter language pack code metadata (internal, not exposed in bindings).
///
/// Hides the external tree_sitter_language_pack::ProcessResult type from FFI/binding
/// surface while preserving tree-sitter code analysis results for Rust consumers.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[doc(hidden)]
#[cfg_attr(alef, alef(skip))]
pub struct CodeMetadataInner(pub tree_sitter_language_pack::ProcessResult);

/// Custom serialization and deserialization for AHashMap<Cow<'static, str>, Value>.
///
/// serde doesn't natively support serializing Cow keys, so we convert to/from
/// a HashMap<String, Value> for the wire format, while keeping the in-memory
/// representation optimized with Cow keys (avoiding allocations for static strings).
mod additional_serde {
    use super::*;

    pub(crate) fn serialize<S>(
        map: &AHashMap<Cow<'static, str>, serde_json::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert to HashMap for serialization
        let converted: HashMap<String, serde_json::Value> =
            map.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        converted.serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<AHashMap<Cow<'static, str>, serde_json::Value>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize from HashMap
        let map = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let result = map.into_iter().map(|(k, v)| (Cow::Owned(k), v)).collect();
        Ok(result)
    }

    pub(crate) fn is_empty(map: &AHashMap<Cow<'static, str>, serde_json::Value>) -> bool {
        map.is_empty()
    }
}

/// Format-specific metadata (discriminated union).
///
/// Only one format type can exist per extraction result. This provides
/// type-safe, clean metadata without nested optionals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "format_type", rename_all = "snake_case")]
pub enum FormatMetadata {
    /// Metadata extracted from a PDF document.
    #[cfg(feature = "pdf")]
    Pdf(PdfMetadata),
    /// Metadata extracted from a DOCX Word document.
    #[cfg(feature = "office")]
    Docx(Box<DocxMetadata>),
    /// Metadata extracted from an Excel spreadsheet.
    Excel(ExcelMetadata),
    /// Metadata extracted from an email message (EML/MSG).
    Email(EmailMetadata),
    /// Metadata extracted from a PowerPoint presentation.
    Pptx(PptxMetadata),
    /// Metadata extracted from an archive (ZIP, TAR, 7Z, etc.).
    Archive(ArchiveMetadata),
    /// Metadata extracted from a raster or vector image.
    Image(ImageMetadata),
    /// Metadata extracted from an XML document.
    Xml(XmlMetadata),
    /// Metadata extracted from a plain-text file.
    Text(TextMetadata),
    /// Metadata extracted from an HTML document.
    Html(Box<HtmlMetadata>),
    /// Metadata produced by an OCR pipeline.
    Ocr(OcrMetadata),
    /// Metadata extracted from a CSV or TSV file.
    Csv(CsvMetadata),
    /// Metadata extracted from a BibTeX bibliography file.
    #[cfg(feature = "office")]
    Bibtex(BibtexMetadata),
    /// Metadata extracted from a citation file (RIS, PubMed, EndNote).
    #[cfg(feature = "office")]
    Citation(CitationMetadata),
    /// Metadata extracted from a FictionBook (FB2) e-book.
    #[cfg(feature = "office")]
    FictionBook(FictionBookMetadata),
    /// Metadata extracted from a dBASE (DBF) database file.
    #[cfg(feature = "office")]
    Dbf(DbfMetadata),
    /// Metadata extracted from a JATS (Journal Article Tag Suite) XML file.
    #[cfg(feature = "xml")]
    Jats(JatsMetadata),
    /// Metadata extracted from an EPUB e-book.
    #[cfg(feature = "office")]
    Epub(EpubMetadata),
    /// Metadata extracted from an Outlook PST archive.
    Pst(PstMetadata),
    /// Metadata extracted from an audio or video file.
    #[cfg(feature = "transcription-types")]
    Audio(AudioMetadata),
    /// Code metadata (tree-sitter analysis results, not exposed in bindings).
    #[cfg(feature = "tree-sitter")]
    #[cfg_attr(alef, alef(skip))]
    #[serde(skip)]
    #[doc(hidden)]
    Code(CodeMetadataInner),
}

impl Default for FormatMetadata {
    fn default() -> Self {
        Self::Text(TextMetadata {
            line_count: 0,
            word_count: 0,
            character_count: 0,
            headers: None,
            links: None,
            code_blocks: None,
        })
    }
}

impl FormatMetadata {
    /// Returns the Excel metadata if this is an Excel format, or `None` otherwise.
    pub fn excel(&self) -> Option<&ExcelMetadata> {
        if let Self::Excel(e) = self { Some(e) } else { None }
    }
}

impl std::fmt::Display for FormatMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "pdf")]
            Self::Pdf(_) => f.write_str("pdf"),
            #[cfg(feature = "office")]
            Self::Docx(_) => f.write_str("docx"),
            Self::Excel(_) => f.write_str("excel"),
            Self::Email(_) => f.write_str("email"),
            Self::Pptx(_) => f.write_str("pptx"),
            Self::Archive(_) => f.write_str("archive"),
            Self::Image(image) => f.write_str(&image.format.to_uppercase()),
            Self::Xml(_) => f.write_str("xml"),
            Self::Text(_) => f.write_str("text"),
            Self::Html(_) => f.write_str("html"),
            Self::Ocr(_) => f.write_str("ocr"),
            Self::Csv(_) => f.write_str("csv"),
            #[cfg(feature = "office")]
            Self::Bibtex(_) => f.write_str("bibtex"),
            #[cfg(feature = "office")]
            Self::Citation(_) => f.write_str("citation"),
            #[cfg(feature = "office")]
            Self::FictionBook(_) => f.write_str("fictionbook"),
            #[cfg(feature = "office")]
            Self::Dbf(_) => f.write_str("dbf"),
            #[cfg(feature = "xml")]
            Self::Jats(_) => f.write_str("jats"),
            #[cfg(feature = "office")]
            Self::Epub(_) => f.write_str("epub"),
            Self::Pst(_) => f.write_str("pst"),
            #[cfg(feature = "transcription-types")]
            Self::Audio(_) => f.write_str("audio"),
            #[cfg(feature = "tree-sitter")]
            Self::Code(_) => f.write_str("code"),
        }
    }
}

#[cfg(feature = "api")]
impl utoipa::PartialSchema for FormatMetadata {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::Ref;
        use utoipa::openapi::schema::{Discriminator, OneOfBuilder};

        // Emit a flat oneOf with $ref items and a discriminator so that
        // codegen tools (openapi-python-client, swagger_parser) do not choke
        // on the allOf-of-ref pattern that utoipa's derive macro would produce.
        let builder = OneOfBuilder::new()
            .description(Some(
                "Format-specific metadata (discriminated union). \
                 Only one format type can exist per extraction result.",
            ))
            .discriminator(Some(Discriminator::with_mapping(
                "format_type",
                [
                    #[cfg(feature = "pdf")]
                    ("pdf", "#/components/schemas/PdfMetadata"),
                    #[cfg(feature = "office")]
                    ("docx", "#/components/schemas/DocxMetadata"),
                    ("excel", "#/components/schemas/ExcelMetadata"),
                    ("email", "#/components/schemas/EmailMetadata"),
                    ("pptx", "#/components/schemas/PptxMetadata"),
                    ("archive", "#/components/schemas/ArchiveMetadata"),
                    ("image", "#/components/schemas/ImageMetadata"),
                    ("xml", "#/components/schemas/XmlMetadata"),
                    ("text", "#/components/schemas/TextMetadata"),
                    ("html", "#/components/schemas/HtmlMetadata"),
                    ("ocr", "#/components/schemas/OcrMetadata"),
                    ("csv", "#/components/schemas/CsvMetadata"),
                    #[cfg(feature = "office")]
                    ("bibtex", "#/components/schemas/BibtexMetadata"),
                    #[cfg(feature = "office")]
                    ("citation", "#/components/schemas/CitationMetadata"),
                    #[cfg(feature = "office")]
                    ("fiction_book", "#/components/schemas/FictionBookMetadata"),
                    #[cfg(feature = "office")]
                    ("dbf", "#/components/schemas/DbfMetadata"),
                    #[cfg(feature = "xml")]
                    ("jats", "#/components/schemas/JatsMetadata"),
                    #[cfg(feature = "office")]
                    ("epub", "#/components/schemas/EpubMetadata"),
                    ("pst", "#/components/schemas/PstMetadata"),
                    #[cfg(feature = "transcription-types")]
                    ("audio", "#/components/schemas/AudioMetadata"),
                ],
            )));

        let builder = {
            #[cfg(feature = "pdf")]
            let builder = builder.item(Ref::from_schema_name("PdfMetadata"));
            #[cfg(not(feature = "pdf"))]
            let builder = builder;

            #[cfg(feature = "office")]
            let builder = builder.item(Ref::from_schema_name("DocxMetadata"));
            #[cfg(not(feature = "office"))]
            let builder = builder;

            let builder = builder
                .item(Ref::from_schema_name("ExcelMetadata"))
                .item(Ref::from_schema_name("EmailMetadata"))
                .item(Ref::from_schema_name("PptxMetadata"))
                .item(Ref::from_schema_name("ArchiveMetadata"))
                .item(Ref::from_schema_name("ImageMetadata"))
                .item(Ref::from_schema_name("XmlMetadata"))
                .item(Ref::from_schema_name("TextMetadata"))
                .item(Ref::from_schema_name("HtmlMetadata"))
                .item(Ref::from_schema_name("OcrMetadata"))
                .item(Ref::from_schema_name("CsvMetadata"));

            #[cfg(feature = "office")]
            let builder = builder
                .item(Ref::from_schema_name("BibtexMetadata"))
                .item(Ref::from_schema_name("CitationMetadata"))
                .item(Ref::from_schema_name("FictionBookMetadata"))
                .item(Ref::from_schema_name("DbfMetadata"))
                .item(Ref::from_schema_name("EpubMetadata"));
            #[cfg(not(feature = "office"))]
            let builder = builder;

            #[cfg(feature = "xml")]
            let builder = builder.item(Ref::from_schema_name("JatsMetadata"));
            #[cfg(not(feature = "xml"))]
            let builder = builder;

            let builder = builder.item(Ref::from_schema_name("PstMetadata"));

            #[cfg(feature = "transcription-types")]
            let builder = builder.item(Ref::from_schema_name("AudioMetadata"));
            #[cfg(not(feature = "transcription-types"))]
            let builder = builder;

            builder
        };

        builder.into()
    }
}

#[cfg(feature = "api")]
impl utoipa::ToSchema for FormatMetadata {
    fn schemas(schemas: &mut Vec<(String, utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>)>) {
        use utoipa::{PartialSchema, ToSchema};

        macro_rules! push_schema {
            ($t:ty) => {
                schemas.push((<$t as ToSchema>::name().into(), <$t as PartialSchema>::schema()));
                <$t as ToSchema>::schemas(schemas);
            };
        }

        #[cfg(feature = "pdf")]
        push_schema!(PdfMetadata);
        #[cfg(feature = "office")]
        {
            push_schema!(DocxMetadata);
            push_schema!(BibtexMetadata);
            push_schema!(CitationMetadata);
            push_schema!(FictionBookMetadata);
            push_schema!(DbfMetadata);
            push_schema!(EpubMetadata);
        }
        #[cfg(feature = "xml")]
        push_schema!(JatsMetadata);
        push_schema!(ExcelMetadata);
        push_schema!(EmailMetadata);
        push_schema!(PptxMetadata);
        push_schema!(ArchiveMetadata);
        push_schema!(ImageMetadata);
        push_schema!(XmlMetadata);
        push_schema!(TextMetadata);
        push_schema!(HtmlMetadata);
        push_schema!(OcrMetadata);
        push_schema!(CsvMetadata);
        push_schema!(PstMetadata);
        #[cfg(feature = "transcription-types")]
        push_schema!(AudioMetadata);
    }
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
    /// Serialized as a nested `"format"` object with a `format_type` discriminator field.
    #[serde(skip_serializing_if = "Option::is_none")]
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

    /// Document category (from frontmatter or classification).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Document tags (from frontmatter).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Document version string (from frontmatter).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_version: Option<String>,

    /// Abstract or summary text (from frontmatter).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,

    /// Output format identifier (e.g., "markdown", "html", "text").
    ///
    /// Set by the output format pipeline stage when format conversion is applied.
    /// Previously stored in `metadata.additional["output_format"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    /// Whether OCR was used during extraction.
    ///
    /// Set to `true` whenever the extraction pipeline ran an OCR backend
    /// (Tesseract, PaddleOCR, VLM, etc.) and used that output as the primary
    /// or fallback text. `false` means native text extraction was used exclusively.
    #[serde(default)]
    pub ocr_used: bool,

    /// Additional custom fields from postprocessors.
    ///
    /// Serialized as a nested `"additional"` object (not flattened at root level).
    /// Uses `Cow<'static, str>` keys so static string keys avoid allocation.
    #[serde(
        skip_serializing_if = "additional_serde::is_empty",
        serialize_with = "additional_serde::serialize",
        deserialize_with = "additional_serde::deserialize",
        default
    )]
    #[cfg_attr(feature = "api", schema(value_type = HashMap<String, serde_json::Value>))]
    pub additional: AHashMap<Cow<'static, str>, serde_json::Value>,
}

impl Metadata {
    /// Returns `true` when no metadata fields, format-specific metadata, or
    /// additional postprocessor fields are populated.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.subject.is_none()
            && self.authors.is_none()
            && self.keywords.is_none()
            && self.language.is_none()
            && self.created_at.is_none()
            && self.modified_at.is_none()
            && self.created_by.is_none()
            && self.modified_by.is_none()
            && self.pages.is_none()
            && self.format.is_none()
            && self.image_preprocessing.is_none()
            && self.json_schema.is_none()
            && self.error.is_none()
            && self.extraction_duration_ms.is_none()
            && self.category.is_none()
            && self.tags.is_none()
            && self.document_version.is_none()
            && self.abstract_text.is_none()
            && self.output_format.is_none()
            && !self.ocr_used
            && self.additional.is_empty()
    }
}

/// Excel/spreadsheet format metadata.
///
/// Identifies the document as a spreadsheet source via the `FormatMetadata::Excel`
/// discriminant. Sheet count and sheet names are stored inside this struct.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ExcelMetadata {
    /// Number of sheets in the workbook.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_count: Option<u32>,

    /// Names of all sheets in the workbook.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_names: Option<Vec<String>>,
}

/// Email metadata extracted from .eml and .msg files.
///
/// Includes sender/recipient information, message ID, and attachment list.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ArchiveMetadata {
    /// Archive format ("ZIP", "TAR", "7Z", etc.)
    #[cfg_attr(feature = "api", schema(value_type = String))]
    pub format: Cow<'static, str>,
    /// Total number of files in the archive
    pub file_count: u32,
    /// List of file paths within the archive
    pub file_list: Vec<String>,
    /// Total uncompressed size in bytes
    pub total_size: u64,

    /// Compressed size in bytes (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed_size: Option<u64>,
}

/// Image metadata extracted from image files.
///
/// Includes dimensions, format, and EXIF data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct XmlMetadata {
    /// Total number of XML elements processed
    pub element_count: u32,
    /// List of unique element tag names (sorted)
    pub unique_elements: Vec<String>,
}

/// Text/Markdown metadata.
///
/// Extracted from plain text and Markdown files. Includes word counts and,
/// for Markdown, structural elements like headers and links.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct TextMetadata {
    /// Number of lines in the document
    pub line_count: u32,
    /// Number of words
    pub word_count: u32,
    /// Number of characters
    pub character_count: u32,

    /// Markdown headers (headings text only, for Markdown files)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<String>>,

    /// Markdown links as (text, url) tuples (for Markdown files)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(value_type = Option<Vec<[String; 2]>>))]
    #[cfg_attr(alef, alef(skip))]
    pub links: Option<Vec<(String, String)>>,

    /// Code blocks as (language, code) tuples (for Markdown files)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(value_type = Option<Vec<[String; 2]>>))]
    #[cfg_attr(alef, alef(skip))]
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
    pub depth: u32,
    /// Byte offset in original HTML document
    pub html_offset: u32,
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
    #[cfg_attr(feature = "api", schema(value_type = Vec<[String; 2]>))]
    #[cfg_attr(alef, alef(skip))]
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
    #[cfg_attr(feature = "api", schema(value_type = Option<[u32; 2]>))]
    #[cfg_attr(alef, alef(skip))]
    pub dimensions: Option<(u32, u32)>,
    /// Image type classification
    pub image_type: ImageType,
    /// Additional attributes as key-value pairs
    #[cfg_attr(feature = "api", schema(value_type = Vec<[String; 2]>))]
    #[cfg_attr(alef, alef(skip))]
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
    #[cfg(feature = "html")]
    pub(crate) fn is_empty(&self) -> bool {
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
impl From<html_to_markdown_rs::HtmlMetadata> for HtmlMetadata {
    fn from(metadata: html_to_markdown_rs::HtmlMetadata) -> Self {
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
                    depth: h.depth as u32,
                    html_offset: h.html_offset as u32,
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct OcrMetadata {
    /// OCR language code(s) used
    pub language: String,
    /// Tesseract Page Segmentation Mode (PSM)
    pub psm: i32,
    /// Output format (e.g., "text", "hocr")
    pub output_format: String,
    /// Number of tables detected
    pub table_count: u32,

    /// Number of rows in the detected table (if a single table was found).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_rows: Option<u32>,

    /// Number of columns in the detected table (if a single table was found).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_cols: Option<u32>,
}

/// Error metadata (for batch operations).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ErrorMetadata {
    /// Machine-readable error type identifier (e.g. "UnsupportedFormat").
    pub error_type: String,
    /// Human-readable error description.
    pub message: String,
}

/// PowerPoint presentation metadata.
///
/// Extracted from PPTX files containing slide counts and presentation details.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PptxMetadata {
    /// Total number of slides in the presentation
    pub slide_count: u32,
    /// Names of slides (if available)
    pub slide_names: Vec<String>,
    /// Number of embedded images
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_count: Option<u32>,
    /// Number of tables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_count: Option<u32>,
}

/// Word document metadata.
///
/// Extracted from DOCX files using shared Office Open XML metadata extraction.
/// Integrates with `office_metadata` module for core/app/custom properties.
#[cfg(feature = "office")]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DocxMetadata {
    /// Core properties from docProps/core.xml (Dublin Core metadata)
    ///
    /// Contains title, creator, subject, keywords, dates, etc.
    /// Shared format across DOCX/PPTX/XLSX documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(value_type = Option<Object>))]
    pub core_properties: Option<crate::extraction::office_metadata::CoreProperties>,

    /// Application properties from docProps/app.xml (Word-specific statistics)
    ///
    /// Contains word count, page count, paragraph count, editing time, etc.
    /// DOCX-specific variant of Office application properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(value_type = Option<Object>))]
    pub app_properties: Option<crate::extraction::office_metadata::DocxAppProperties>,

    /// Custom properties from docProps/custom.xml (user-defined properties)
    ///
    /// Contains key-value pairs defined by users or applications.
    /// Values can be strings, numbers, booleans, or dates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<HashMap<String, serde_json::Value>>,
    // Future Week 1-21 additions (commented out for now):
    // style_catalog: OnceCell<Arc<StyleCatalog>>,       // Week 1-2: Style resolution
    // theme: OnceCell<Arc<Theme>>,                      // Week 5: Theme colors
    // numbering_catalog: OnceCell<Arc<NumberingCatalog>>, // Week 12-13: Numbering
    // sections: Vec<SectionProperties>,                 // Week 3-4: Section properties
    // document_settings: DocumentSettings,              // Week 11: Settings.xml
}

// ── Format-specific metadata structs (non-additional) ──────────────────

/// CSV/TSV file metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct CsvMetadata {
    /// Total number of data rows (excluding the header row if present).
    pub row_count: u32,
    /// Number of columns detected.
    pub column_count: u32,
    /// Field delimiter character (e.g. `","` or `"\t"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// Whether the first row was treated as a header.
    pub has_header: bool,
    /// Inferred data type for each column (e.g. `"string"`, `"integer"`, `"float"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_types: Option<Vec<String>>,
}

/// BibTeX bibliography metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct BibtexMetadata {
    /// Number of entries in the bibliography.
    pub entry_count: usize,
    /// BibTeX citation keys (e.g. `"knuth1984"`) for all entries.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub citation_keys: Vec<String>,
    /// Author names collected across all bibliography entries.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub authors: Vec<String>,
    /// Earliest and latest publication years found in the bibliography.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_range: Option<YearRange>,
    /// Count of entries grouped by BibTeX entry type (e.g. `"article"` → 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_types: Option<BTreeMap<String, usize>>,
}

/// Citation file metadata (RIS, PubMed, EndNote).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct CitationMetadata {
    /// Total number of citation records in the file.
    pub citation_count: usize,
    /// Detected citation file format (e.g. `"ris"`, `"pubmed"`, `"endnote"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Author names collected across all citation records.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub authors: Vec<String>,
    /// Earliest and latest publication years found in the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_range: Option<YearRange>,
    /// DOI identifiers found in the citation records.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub dois: Vec<String>,
    /// Keywords collected from all citation records.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub keywords: Vec<String>,
}

/// Year range for bibliographic metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct YearRange {
    /// Earliest (minimum) year in the range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    /// Latest (maximum) year in the range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    /// All individual years present in the collection.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub years: Vec<u32>,
}

/// FictionBook (FB2) metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct FictionBookMetadata {
    /// Genre tags as declared in the FB2 `<genre>` elements.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub genres: Vec<String>,
    /// Book series (sequence) names, if any.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sequences: Vec<String>,
    /// Short annotation / summary from the FB2 `<annotation>` element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
}

/// dBASE (DBF) file metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DbfMetadata {
    /// Total number of data records in the DBF file.
    pub record_count: usize,
    /// Number of field (column) definitions.
    pub field_count: usize,
    /// Descriptor for each field in the table schema.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fields: Vec<DbfFieldInfo>,
}

/// dBASE field information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DbfFieldInfo {
    /// Field (column) name.
    pub name: String,
    /// dBASE field type character (e.g. `"C"` for character, `"N"` for numeric).
    pub field_type: String,
}

/// JATS (Journal Article Tag Suite) metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct JatsMetadata {
    /// Copyright statement from the article's `<permissions>` element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    /// Open-access license URI from the article's `<license>` element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Publication history dates keyed by event type (e.g. `"received"`, `"accepted"`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub history_dates: BTreeMap<String, String>,
    /// Authors and contributors with their stated roles.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub contributor_roles: Vec<ContributorRole>,
}

/// JATS contributor with role.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ContributorRole {
    /// Contributor display name.
    pub name: String,
    /// Contributor role (e.g. `"author"`, `"editor"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// EPUB metadata (Dublin Core extensions).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct EpubMetadata {
    /// Dublin Core `coverage` field (geographic or temporal scope).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<String>,
    /// Dublin Core `format` field (media type of the resource).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dc_format: Option<String>,
    /// Dublin Core `relation` field (related resource identifier).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    /// Dublin Core `source` field (origin resource identifier).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Dublin Core `type` field (nature or genre of the resource).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dc_type: Option<String>,
    /// Path or identifier of the cover image within the EPUB container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
}

/// Outlook PST archive metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PstMetadata {
    /// Total number of email messages found in the PST archive.
    pub message_count: usize,
}

/// Audio/video file metadata.
///
/// Populated from container tags (ID3v2, MP4 atoms, Vorbis comments, etc.) and
/// PCM decode properties. Available when the `transcription-types` feature is enabled.
#[cfg(feature = "transcription-types")]
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct AudioMetadata {
    /// Duration in milliseconds derived from the decoded audio stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Audio codec (e.g. "mp3", "aac", "opus", "flac").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    /// Container format (e.g. "mpeg", "mp4", "ogg", "wav").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    /// Sample rate in Hz after decode (always 16000 when resampled for Whisper).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate_hz: Option<u32>,
    /// Number of audio channels (1 = mono, 2 = stereo).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u16>,
    /// Audio bitrate in kbps from the source file tags/properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
}
