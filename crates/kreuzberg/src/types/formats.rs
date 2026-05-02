//! Format-specific extraction results and OCR configuration types.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::document_structure::DocumentStructure;
use super::extraction::ExtractedImage;
use super::metadata::PptxMetadata;
use super::page::{PageContent, PageStructure};

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
    /// Structured document representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentStructure>,
    /// Hyperlinks discovered in slides as (url, optional_label) pairs.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub hyperlinks: Vec<(String, Option<String>)>,
    /// Office metadata extracted from docProps/core.xml and docProps/app.xml.
    ///
    /// Contains keys like "title", "author", "created_by", "subject", "keywords",
    /// "modified_by", "created_at", "modified_at", etc.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub office_metadata: HashMap<String, String>,
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
    /// Structured OCR elements with bounding boxes and confidence scores.
    /// Available when TSV output is requested or table detection is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_elements: Option<Vec<super::OcrElement>>,
    /// Structured document produced from hOCR parsing.
    /// Carries paragraph structure, bounding boxes, and confidence scores
    /// that the flattened `content` string discards.
    #[serde(skip)]
    pub internal_document: Option<super::internal::InternalDocument>,
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
    /// - 3: Fully automatic page segmentation (native default)
    /// - 6: Assume a single uniform block of text (WASM default — avoids layout-analysis hang)
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
            // PSM_AUTO (3) hangs 60-90s on sparse/no-text images in WASM (issue #855)
            #[cfg(target_arch = "wasm32")]
            psm: 6,
            #[cfg(not(target_arch = "wasm32"))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
