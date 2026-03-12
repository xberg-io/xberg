//! Core extraction types and results.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

use super::djot::DjotContent;
use super::document_structure::DocumentStructure;
use super::metadata::Metadata;
use super::ocr_elements::OcrElement;
use super::page::PageContent;
use super::tables::Table;

/// General extraction result used by the core extraction API.
///
/// This is the main result type returned by all extraction functions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "api", schema(no_recursion))]
pub struct ExtractionResult {
    pub content: String,
    #[cfg_attr(feature = "api", schema(value_type = String))]
    pub mime_type: Cow<'static, str>,
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

    /// Semantic elements when element-based result format is enabled.
    ///
    /// When result_format is set to ElementBased, this field contains semantic
    /// elements with type classification, unique identifiers, and metadata for
    /// Unstructured-compatible element-based processing.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub elements: Option<Vec<Element>>,

    /// Rich Djot content structure (when extracting Djot documents).
    ///
    /// When extracting Djot documents with structured extraction enabled,
    /// this field contains the full semantic structure including:
    /// - Block-level elements with nesting
    /// - Inline formatting with attributes
    /// - Links, images, footnotes
    /// - Math expressions
    /// - Complete attribute information
    ///
    /// The `content` field still contains plain text for backward compatibility.
    ///
    /// Always `None` for non-Djot documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub djot_content: Option<DjotContent>,

    /// OCR elements with full spatial and confidence metadata.
    ///
    /// When OCR is performed with element extraction enabled, this field contains
    /// the structured representation of detected text including:
    /// - Bounding geometry (rectangles or quadrilaterals)
    /// - Confidence scores (detection and recognition)
    /// - Rotation information
    /// - Hierarchical relationships (Tesseract only)
    ///
    /// This field preserves all metadata that would otherwise be lost when
    /// converting to plain text or markdown output formats.
    ///
    /// Only populated when `OcrElementConfig.include_elements` is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub ocr_elements: Option<Vec<OcrElement>>,

    /// Structured document tree (when document structure extraction is enabled).
    ///
    /// When `include_document_structure` is true in `ExtractionConfig`, this field
    /// contains the full hierarchical representation of the document including:
    /// - Heading-driven section nesting
    /// - Table grids with cell-level metadata
    /// - Content layer classification (body, header, footer, footnote)
    /// - Inline text annotations (formatting, links)
    /// - Bounding boxes and page numbers
    ///
    /// Independent of `result_format` — can be combined with Unified or ElementBased.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub document: Option<DocumentStructure>,

    /// Extracted keywords when keyword extraction is enabled.
    ///
    /// When keyword extraction (RAKE or YAKE) is configured, this field contains
    /// the extracted keywords with scores, algorithm info, and position data.
    /// Previously stored in `metadata.additional["keywords"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    pub extracted_keywords: Option<Vec<crate::keywords::Keyword>>,

    /// Document quality score from quality analysis.
    ///
    /// A value between 0.0 and 1.0 indicating the overall text quality.
    /// Previously stored in `metadata.additional["quality_score"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub quality_score: Option<f64>,

    /// Non-fatal warnings collected during processing pipeline stages.
    ///
    /// Captures errors from optional pipeline features (embedding, chunking,
    /// language detection, output formatting) that don't prevent extraction
    /// but may indicate degraded results.
    /// Previously stored as individual keys in `metadata.additional`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub processing_warnings: Vec<ProcessingWarning>,

    /// PDF annotations extracted from the document.
    ///
    /// When annotation extraction is enabled via `PdfConfig::extract_annotations`,
    /// this field contains text notes, highlights, links, stamps, and other
    /// annotations found in PDF documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub annotations: Option<Vec<super::annotations::PdfAnnotation>>,
}

/// A non-fatal warning from a processing pipeline stage.
///
/// Captures errors from optional features that don't prevent extraction
/// but may indicate degraded results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessingWarning {
    /// The pipeline stage or feature that produced this warning
    /// (e.g., "embedding", "chunking", "language_detection", "output_format").
    pub source: String,
    /// Human-readable description of what went wrong.
    pub message: String,
}

/// A text chunk with optional embedding and metadata.
///
/// Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
/// contains the text content, optional embedding vector (if embedding generation
/// is configured), and metadata about its position in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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

/// Heading context for a chunk within a Markdown document.
///
/// Contains the heading hierarchy from document root to this chunk's section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct HeadingContext {
    /// The heading hierarchy from document root to this chunk's section.
    /// Index 0 is the outermost (h1), last element is the most specific.
    pub headings: Vec<HeadingLevel>,
}

/// A single heading in the hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct HeadingLevel {
    /// Heading depth (1 = h1, 2 = h2, etc.)
    pub level: u8,
    /// The text content of the heading.
    pub text: String,
}

/// Metadata about a chunk's position in the original document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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

    /// Heading context when using Markdown chunker.
    ///
    /// Contains the heading hierarchy this chunk falls under.
    /// Only populated when `ChunkerType::Markdown` is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub heading_context: Option<HeadingContext>,
}

/// Extracted image from a document.
///
/// Contains raw image data, metadata, and optional nested OCR results.
/// Raw bytes allow cross-language compatibility - users can convert to
/// PIL.Image (Python), Sharp (Node.js), or other formats as needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ExtractedImage {
    /// Raw image data (PNG, JPEG, WebP, etc. bytes).
    /// Uses `bytes::Bytes` for cheap cloning of large buffers.
    #[cfg_attr(feature = "api", schema(value_type = Vec<u8>, format = "binary"))]
    pub data: Bytes,

    /// Image format (e.g., "jpeg", "png", "webp")
    /// Uses Cow<'static, str> to avoid allocation for static literals.
    #[cfg_attr(feature = "api", schema(value_type = String))]
    pub format: Cow<'static, str>,

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
    #[cfg_attr(feature = "api", schema(value_type = Option<ExtractionResult>))]
    pub ocr_result: Option<Box<ExtractionResult>>,

    /// Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top).
    /// Only populated for PDF-extracted images when position data is available from pdfium.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub bounding_box: Option<BoundingBox>,
}

// ============================================================================
// Element-based Output Format Types (Unstructured-compatible)
// ============================================================================

/// Output format selection for extraction results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// Unified format with all content in `content` field
    #[default]
    Unified,
    /// Element-based format with semantic element extraction
    ElementBased,
}

/// Unique identifier for semantic elements.
///
/// Wraps a string identifier that is deterministically generated
/// from element type, content, and page number.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "api", schema(value_type = String))]
pub struct ElementId(String);

impl ElementId {
    /// Create a new ElementId from a string.
    ///
    /// # Errors
    ///
    /// Returns error if the string is not valid.
    pub fn new(hex_str: impl Into<String>) -> std::result::Result<Self, String> {
        let s = hex_str.into();
        if s.is_empty() {
            return Err("ElementId cannot be empty".to_string());
        }
        Ok(ElementId(s))
    }
}

impl AsRef<str> for ElementId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Semantic element type classification.
///
/// Categorizes text content into semantic units for downstream processing.
/// Supports the element types commonly found in Unstructured documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    /// Document title
    Title,
    /// Main narrative text body
    NarrativeText,
    /// Section heading
    Heading,
    /// List item (bullet, numbered, etc.)
    ListItem,
    /// Table element
    Table,
    /// Image element
    Image,
    /// Page break marker
    PageBreak,
    /// Code block
    CodeBlock,
    /// Block quote
    BlockQuote,
    /// Footer text
    Footer,
    /// Header text
    Header,
}

/// Bounding box coordinates for element positioning.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct BoundingBox {
    /// Left x-coordinate
    pub x0: f64,
    /// Bottom y-coordinate
    pub y0: f64,
    /// Right x-coordinate
    pub x1: f64,
    /// Top y-coordinate
    pub y1: f64,
}

/// Metadata for a semantic element.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ElementMetadata {
    /// Page number (1-indexed)
    pub page_number: Option<usize>,
    /// Source filename or document name
    pub filename: Option<String>,
    /// Bounding box coordinates if available
    pub coordinates: Option<BoundingBox>,
    /// Position index in the element sequence
    pub element_index: Option<usize>,
    /// Additional custom metadata
    pub additional: HashMap<String, String>,
}

/// Semantic element extracted from document.
///
/// Represents a logical unit of content with semantic classification,
/// unique identifier, and metadata for tracking origin and position.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct Element {
    /// Unique element identifier
    pub element_id: ElementId,
    /// Semantic type of this element
    pub element_type: ElementType,
    /// Text content of the element
    pub text: String,
    /// Metadata about the element
    pub metadata: ElementMetadata,
}
