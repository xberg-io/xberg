//! Format-specific extraction results and OCR configuration types.

use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

use super::document_structure::DocumentStructure;
use super::extraction::ExtractedImage;
use super::metadata::PptxMetadata;
use super::page::{PageContent, PageStructure};

/// Deserialize a language field that accepts either a string or a list of strings.
fn deserialize_languages<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::String(s) => {
            if s.contains('+') {
                Ok(s.split('+').map(|l| l.to_string()).collect())
            } else {
                Ok(vec![s])
            }
        }
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(|v| {
                v.as_str()
                    .map(String::from)
                    .ok_or_else(|| Error::custom("each language must be a string"))
            })
            .collect(),
        _ => Err(Error::custom(
            "language must be a string (e.g., \"eng\") or an array of strings (e.g., [\"eng\", \"deu\"])",
        )),
    }
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
    /// Collaborative-edit revision headers from `xl/revisions/revisionHeaders.xml`.
    ///
    /// Populated for legacy shared-workbook `.xlsx` files that contain the
    /// `xl/revisions/` directory. Each `<header>` element maps to one
    /// `DocumentRevision { kind: FormatChange }` carrying the header's `guid`
    /// (→ `revision_id`), `userName` (→ `author`), and `dateTime` (→ `timestamp`).
    /// `anchor` and `delta` are `None`/empty for v1 (per-cell log parsing is a
    /// follow-up). `None` when `xl/revisions/revisionHeaders.xml` is absent.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub revisions: Option<Vec<super::revisions::DocumentRevision>>,
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
    /// Markdown links (Markdown files only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<super::metadata::MarkdownLink>>,
    /// Code blocks (Markdown files only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_blocks: Option<Vec<super::metadata::MarkdownCodeBlock>>,
}

/// A hyperlink discovered in a presentation slide.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PresentationHyperlink {
    /// Link destination.
    pub url: String,
    /// Optional visible label.
    pub label: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PresentationHyperlinkWire {
    Positional((String, Option<String>)),
    Named {
        url: String,
        #[serde(default)]
        label: Option<String>,
    },
}

impl<'de> Deserialize<'de> for PresentationHyperlink {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PresentationHyperlinkWire::deserialize(deserializer)? {
            PresentationHyperlinkWire::Positional(hyperlink) => hyperlink.into(),
            PresentationHyperlinkWire::Named { url, label } => Self { url, label },
        })
    }
}

#[cfg(feature = "api")]
impl utoipa::PartialSchema for PresentationHyperlink {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{Object, ObjectBuilder, SchemaType, Type};

        let nullable_string = ObjectBuilder::new()
            .schema_type(SchemaType::from_iter([Type::String, Type::Null]))
            .build();

        ObjectBuilder::new()
            .property("url", Object::with_type(Type::String))
            .required("url")
            .property("label", nullable_string)
            .required("label")
            .into()
    }
}

#[cfg(feature = "api")]
impl utoipa::ToSchema for PresentationHyperlink {}

impl From<(String, Option<String>)> for PresentationHyperlink {
    fn from((url, label): (String, Option<String>)) -> Self {
        Self { url, label }
    }
}

impl From<PresentationHyperlink> for (String, Option<String>) {
    fn from(hyperlink: PresentationHyperlink) -> Self {
        (hyperlink.url, hyperlink.label)
    }
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
    /// Structured document representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentStructure>,
    /// Hyperlinks discovered in slides.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub hyperlinks: Vec<PresentationHyperlink>,
    /// Office metadata extracted from docProps/core.xml and docProps/app.xml.
    ///
    /// Contains keys like "title", "author", "created_by", "subject", "keywords",
    /// "modified_by", "created_at", "modified_at", etc.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub office_metadata: HashMap<String, String>,
    /// Slide comments as revisions.
    ///
    /// Each `<p:cm>` element in `ppt/comments/comment{N}.xml` becomes a
    /// `DocumentRevision { kind: Comment }` with author (resolved from
    /// `ppt/commentAuthors.xml`), ISO-8601 timestamp, and
    /// `RevisionAnchor::Slide { index }`. `None` when no comment XML parts exist.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub revisions: Option<Vec<super::revisions::DocumentRevision>>,
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
    /// Cleaned/processed text content. Aliased as `cleaned_text` for back-compat.
    #[serde(alias = "cleaned_text")]
    pub content: String,
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
    /// Attachment data (if extracted).
    /// Uses `bytes::Bytes` for cheap cloning of large buffers.
    pub data: Option<Bytes>,
}

/// OCR extraction result.
///
/// Result of performing OCR on an image or scanned document,
/// including recognized text and detected tables.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OcrExtractionResult {
    /// Recognized text content
    pub content: String,
    /// Original MIME type of the processed image
    pub mime_type: String,
    /// OCR processing metadata (confidence scores, language, etc.)
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tables detected and extracted via OCR
    pub tables: Vec<OcrTable>,
    /// Structured OCR elements with bounding boxes and confidence scores.
    /// Available when TSV output is requested or table detection is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_elements: Option<Vec<super::OcrElement>>,
    /// Structured document produced from hOCR parsing.
    /// Carries paragraph structure, bounding boxes, and confidence scores
    /// that the flattened `content` string discards.
    #[serde(skip)]
    #[allow(dead_code)]
    #[cfg_attr(alef, alef(skip))]
    pub(crate) internal_document: Option<super::internal::InternalDocument>,
}

impl OcrExtractionResult {
    /// Create an OCR extraction result without an internal hOCR document.
    #[must_use]
    pub fn new(
        content: String,
        mime_type: String,
        metadata: HashMap<String, serde_json::Value>,
        tables: Vec<OcrTable>,
        ocr_elements: Option<Vec<super::OcrElement>>,
    ) -> Self {
        Self {
            content,
            mime_type,
            metadata,
            tables,
            ocr_elements,
            internal_document: None,
        }
    }
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
    pub page_number: u32,
    /// Bounding box of the table in pixel coordinates (from OCR word positions).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub bounding_box: Option<OcrTableBoundingBox>,
}

/// Bounding box for an OCR-detected table in pixel coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OcrTableBoundingBox {
    /// Left x-coordinate (pixels)
    pub left: u32,
    /// Top y-coordinate (pixels)
    pub top: u32,
    /// Right x-coordinate (pixels)
    pub right: u32,
    /// Bottom y-coordinate (pixels)
    pub bottom: u32,
}

/// Image preprocessing configuration for OCR.
///
/// These settings control how images are preprocessed before OCR to improve
/// text recognition quality. Different preprocessing strategies work better
/// for different document types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(default, deny_unknown_fields)]
pub struct ImagePreprocessingConfig {
    /// Target DPI for the image (300 is standard, 600 for small text).
    pub target_dpi: i32,

    /// Auto-detect and correct image rotation.
    pub auto_rotate: bool,

    /// Correct skew (tilted images). Must be `false` with `none` or `off` binarization.
    pub deskew: bool,

    /// Remove noise from the image.
    pub denoise: bool,

    /// Enhance contrast for better text visibility. With `none` or `off` binarization, this
    /// applies background normalization and sharpening while preserving grayscale pixels.
    pub contrast_enhance: bool,

    /// Binarization method: "none" (alias "off"), "otsu", "sauvola", or "adaptive".
    /// `deskew` must be `false` when this is `none` or `off`.
    pub binarization_method: String,

    /// Invert colors (white text on black → black on white).
    pub invert_colors: bool,

    /// Flatten shaded table rows before recognition. A row drawn as dark text on a light
    /// grey band, or as white text on a grey or dark band, is stretched to dark text on
    /// white, band by band, so it survives the page-wide threshold that otherwise drops the
    /// whole row. Rows outside such bands are not changed. Off by default: measured on
    /// table pages with shaded subtotal and total rows, where it recovers those rows; on
    /// other pages it can move a few percent of the words either way.
    pub normalize_shaded_rows: bool,
}

impl Default for ImagePreprocessingConfig {
    fn default() -> Self {
        Self {
            target_dpi: 300,
            auto_rotate: false,
            deskew: true,
            denoise: false,
            contrast_enhance: false,
            binarization_method: "otsu".to_string(),
            invert_colors: false,
            normalize_shaded_rows: false,
        }
    }
}

/// Tesseract OCR configuration.
///
/// Provides fine-grained control over Tesseract OCR engine parameters.
/// Most users can use the defaults, but these settings allow optimization
/// for specific document types (invoices, handwriting, etc.).
///
/// **This is the public-facing counterpart of `ocr::types::TesseractConfig`
/// (the internal, engine-facing representation with `u8`/`String` fields instead of
/// `i32`/`Vec<String>`).** They are two independent struct definitions bridged only by
/// an explicit `From<&TesseractConfig> for crate::ocr::types::TesseractConfig` impl in
/// `ocr/types.rs` — that conversion carries field *values* across, but each struct keeps
/// its own `Default` impl, and the conversion does nothing to keep those two defaults in
/// sync. Several production call sites (`extractors::image::apply_default_tesseract_psm`,
/// `configured_region_ocr`, `sparse_image_ocr_fallback_config`) construct *this* struct's
/// default and convert it, bypassing the internal struct's `Default` entirely — so if the
/// two defaults disagree, standalone image OCR silently uses this struct's value while
/// PDF-embedded OCR (which can reach the internal `Default` directly when no
/// `tesseract_config` is set) uses the other. When changing a default here, also update
/// `ocr::types::TesseractConfig::default`, and vice versa.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(default, deny_unknown_fields)]
pub struct TesseractConfig {
    /// Language code(s) for OCR recognition. For Tesseract, languages are joined with "+".
    ///
    /// A list is the canonical form and the only form accepted by the binding
    /// object APIs (Python, Node, PHP, WASM, etc.): `["eng", "deu"]`. When
    /// deserializing from a config file, JSON body, or the REST/MCP API, a
    /// single string is also accepted, either as one code ("eng") or
    /// "+"-joined ("eng+deu").
    #[serde(deserialize_with = "deserialize_languages")]
    pub language: Vec<String>,

    /// Page Segmentation Mode (1-13).
    ///
    /// PSM 0 is rejected: Tesseract's `PSM_OSD_ONLY` performs orientation and script
    /// detection with no character recognition, so it cannot satisfy a text-extraction
    /// request and previously yielded an empty document (GH#1586).
    ///
    /// `None` (the default) means the caller made no explicit choice: the extraction
    /// pipeline applies its own context-appropriate PSM (whole-image PSM 11, vertical-
    /// language PSM 5, layout-region PSM 6, or the sparse-text retry's PSM 3) exactly as
    /// it would with no `TesseractConfig` at all — see issue #1573. Setting any other
    /// field on this struct no longer changes that behaviour.
    ///
    /// Common explicit values:
    /// - 3: Fully automatic page segmentation (native engine default)
    /// - 6: Assume a single uniform block of text (WASM engine default — avoids
    ///   layout-analysis hang)
    /// - 11: Sparse text with no particular order
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psm: Option<i32>,

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

    /// Enable N-gram language model.
    ///
    /// Kept on by default (see [`Self::default`] and
    /// `ocr::types::TesseractConfig::language_model_ngram_on` for the rationale);
    /// keep this field's default in sync with the internal struct's.
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
            language: vec!["eng".to_string()],
            psm: None,
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
            // Must match crate::ocr::types::TesseractConfig::default() — see the struct-level
            // doc comment above for why the two defaults can silently diverge otherwise.
            language_model_ngram_on: true,
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct PixelDimensions {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PixelDimensionsWire {
    Positional((usize, usize)),
    Named { width: usize, height: usize },
}

impl<'de> Deserialize<'de> for PixelDimensions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PixelDimensionsWire::deserialize(deserializer)? {
            PixelDimensionsWire::Positional(dimensions) => dimensions.into(),
            PixelDimensionsWire::Named { width, height } => Self { width, height },
        })
    }
}

#[cfg(feature = "api")]
impl utoipa::PartialSchema for PixelDimensions {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{Object, ObjectBuilder, Type};

        ObjectBuilder::new()
            .property("width", Object::with_type(Type::Integer))
            .required("width")
            .property("height", Object::with_type(Type::Integer))
            .required("height")
            .into()
    }
}

#[cfg(feature = "api")]
impl utoipa::ToSchema for PixelDimensions {}

impl From<(usize, usize)> for PixelDimensions {
    fn from((width, height): (usize, usize)) -> Self {
        Self { width, height }
    }
}

impl From<PixelDimensions> for (usize, usize) {
    fn from(dimensions: PixelDimensions) -> Self {
        (dimensions.width, dimensions.height)
    }
}

/// Horizontal and vertical image resolution in dots per inch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct ImageDpi {
    /// Horizontal resolution.
    pub horizontal: f64,
    /// Vertical resolution.
    pub vertical: f64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ImageDpiWire {
    Positional((f64, f64)),
    Named { horizontal: f64, vertical: f64 },
}

impl<'de> Deserialize<'de> for ImageDpi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match ImageDpiWire::deserialize(deserializer)? {
            ImageDpiWire::Positional(dpi) => dpi.into(),
            ImageDpiWire::Named { horizontal, vertical } => Self { horizontal, vertical },
        })
    }
}

#[cfg(feature = "api")]
impl utoipa::PartialSchema for ImageDpi {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{Object, ObjectBuilder, Type};

        ObjectBuilder::new()
            .property("horizontal", Object::with_type(Type::Number))
            .required("horizontal")
            .property("vertical", Object::with_type(Type::Number))
            .required("vertical")
            .into()
    }
}

#[cfg(feature = "api")]
impl utoipa::ToSchema for ImageDpi {}

impl From<(f64, f64)> for ImageDpi {
    fn from((horizontal, vertical): (f64, f64)) -> Self {
        Self { horizontal, vertical }
    }
}

impl From<ImageDpi> for (f64, f64) {
    fn from(dpi: ImageDpi) -> Self {
        (dpi.horizontal, dpi.vertical)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ImagePreprocessingMetadata {
    /// Original image dimensions in pixels.
    pub original_dimensions: PixelDimensions,
    /// Original image resolution.
    pub original_dpi: ImageDpi,
    /// Target DPI from configuration
    pub target_dpi: i32,
    /// Scaling factor applied to the image
    pub scale_factor: f64,
    /// Whether DPI was auto-adjusted based on content
    pub auto_adjusted: bool,
    /// Final DPI after processing
    pub final_dpi: i32,
    /// New dimensions after resizing (if resized).
    pub new_dimensions: Option<PixelDimensions>,
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
#[cfg_attr(alef, alef(skip))]
/// Image extraction DPI configuration (internal use).
///
/// **Note:** This is an internal type used for image preprocessing.
/// For the main extraction configuration, see [`crate::core::config::ExtractionConfig`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDpiConfig {
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

impl Default for ImageDpiConfig {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[cfg(feature = "api")]
    fn assert_named_object_schema<T: utoipa::PartialSchema>(expected: serde_json::Value) {
        let schema = serde_json::to_value(T::schema()).expect("schema must serialize");
        assert_eq!(schema, expected);
    }

    #[cfg(feature = "api")]
    #[test]
    fn should_describe_binding_dtos_as_named_object_schemas() {
        assert_named_object_schema::<PresentationHyperlink>(json!({
            "type": "object",
            "required": ["url", "label"],
            "properties": {
                "url": {"type": "string"},
                "label": {"type": ["string", "null"]}
            }
        }));
        assert_named_object_schema::<PixelDimensions>(json!({
            "type": "object",
            "required": ["width", "height"],
            "properties": {
                "width": {"type": "integer"},
                "height": {"type": "integer"}
            }
        }));
        assert_named_object_schema::<ImageDpi>(json!({
            "type": "object",
            "required": ["horizontal", "vertical"],
            "properties": {
                "horizontal": {"type": "number"},
                "vertical": {"type": "number"}
            }
        }));
    }

    #[test]
    fn should_serialize_presentation_hyperlink_as_named_object() {
        let legacy = json!(["https://xberg.io", "Xberg"]);
        let named_json = json!({"url": "https://xberg.io", "label": "Xberg"});
        let hyperlink: PresentationHyperlink =
            serde_json::from_value(legacy).expect("legacy hyperlink must deserialize");
        let named: PresentationHyperlink =
            serde_json::from_value(named_json.clone()).expect("named hyperlink must deserialize");

        assert_eq!(hyperlink.url, "https://xberg.io");
        assert_eq!(hyperlink.label.as_deref(), Some("Xberg"));
        assert_eq!(named, hyperlink);
        assert_eq!(
            serde_json::to_value(hyperlink).expect("hyperlink must serialize"),
            named_json
        );
        assert_eq!(
            serde_json::to_value(named).expect("named hyperlink must serialize"),
            named_json
        );
    }

    #[test]
    fn should_serialize_missing_hyperlink_label_as_named_null() {
        let legacy = json!(["https://xberg.io", null]);
        let named_json = json!({"url": "https://xberg.io", "label": null});
        let positional: PresentationHyperlink =
            serde_json::from_value(legacy).expect("legacy null label must deserialize");
        let named: PresentationHyperlink =
            serde_json::from_value(json!({"url": "https://xberg.io"})).expect("omitted named label must deserialize");

        assert_eq!(named, positional);
        assert_eq!(
            serde_json::to_value(positional).expect("hyperlink must serialize"),
            named_json
        );
        assert_eq!(
            serde_json::to_value(named).expect("named hyperlink must serialize"),
            named_json
        );
    }

    #[test]
    fn should_still_accept_legacy_positional_array_for_pixel_dimensions() {
        let dimensions: PixelDimensions =
            serde_json::from_str("[1200, 800]").expect("legacy pixel dimensions must deserialize");

        assert_eq!(
            dimensions,
            PixelDimensions {
                width: 1200,
                height: 800
            }
        );
    }

    #[test]
    fn should_still_accept_legacy_positional_array_for_image_dpi() {
        let dpi: ImageDpi = serde_json::from_str("[72.0, 96.0]").expect("legacy image DPI must deserialize");

        assert_eq!(
            dpi,
            ImageDpi {
                horizontal: 72.0,
                vertical: 96.0
            }
        );
    }

    #[test]
    fn should_serialize_preprocessing_metadata_with_named_nested_types() {
        let legacy = json!({
            "original_dimensions": [1200, 800],
            "original_dpi": [72.0, 96.0],
            "target_dpi": 300,
            "scale_factor": 2.0,
            "auto_adjusted": true,
            "final_dpi": 288,
            "new_dimensions": [2400, 1600],
            "resample_method": "LANCZOS3",
            "dimension_clamped": false,
            "calculated_dpi": 288,
            "skipped_resize": false,
            "resize_error": null
        });
        let named = json!({
            "original_dimensions": {"width": 1200, "height": 800},
            "original_dpi": {"horizontal": 72.0, "vertical": 96.0},
            "target_dpi": 300,
            "scale_factor": 2.0,
            "auto_adjusted": true,
            "final_dpi": 288,
            "new_dimensions": {"width": 2400, "height": 1600},
            "resample_method": "LANCZOS3",
            "dimension_clamped": false,
            "calculated_dpi": 288,
            "skipped_resize": false,
            "resize_error": null
        });
        let metadata: ImagePreprocessingMetadata =
            serde_json::from_value(legacy).expect("legacy preprocessing metadata must deserialize");
        let named_dimensions: PixelDimensions = serde_json::from_value(json!({"width": 1200, "height": 800}))
            .expect("named pixel dimensions must deserialize");
        let named_dpi: ImageDpi = serde_json::from_value(json!({"horizontal": 72.0, "vertical": 96.0}))
            .expect("named image DPI must deserialize");

        assert_eq!(
            metadata.original_dimensions,
            PixelDimensions {
                width: 1200,
                height: 800
            }
        );
        assert_eq!(
            metadata.original_dpi,
            ImageDpi {
                horizontal: 72.0,
                vertical: 96.0
            }
        );
        assert_eq!(
            metadata.new_dimensions,
            Some(PixelDimensions {
                width: 2400,
                height: 1600
            })
        );
        assert_eq!(
            serde_json::to_value(metadata).expect("preprocessing metadata must serialize"),
            named
        );
        assert_eq!(
            serde_json::to_value(named_dimensions).expect("named pixel dimensions must serialize"),
            json!({"width": 1200, "height": 800})
        );
        assert_eq!(
            serde_json::to_value(named_dpi).expect("named image DPI must serialize"),
            json!({"horizontal": 72.0, "vertical": 96.0})
        );
    }

    /// This is the public-facing `TesseractConfig` (re-exported as `crate::types::
    /// TesseractConfig`), not `crate::ocr::types::TesseractConfig` (the internal,
    /// engine-facing struct with its own separate `Default` impl). The two defaults must
    /// agree: `extractors::image::apply_default_tesseract_psm` and related call sites
    /// construct *this* struct's default and convert it into the internal one, bypassing
    /// the internal struct's own `Default` — so a stale value here silently overrides the
    /// internal default for every standalone image OCR call, even after the internal
    /// default is changed.
    ///
    /// Against the unfixed code this struct's `language_model_ngram_on` default is
    /// `false`, disagreeing with `crate::ocr::types::TesseractConfig::default()`'s `true`
    /// (see that struct's doc comment for why `true` is the deliberate, documented
    /// default), so this assertion fails with `false` instead of `true`.
    #[test]
    fn test_tesseract_config_default_matches_internal_ngram_default() {
        let config = TesseractConfig::default();

        assert!(
            config.language_model_ngram_on,
            "public TesseractConfig::default() must match crate::ocr::types::TesseractConfig::default() \
             for language_model_ngram_on (true), or standalone image OCR silently gets the stale value"
        );
    }
}
