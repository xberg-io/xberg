//! Page structure types for documents.
//!
//! This module defines types for representing paginated document structures.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Import serde helper and types from sibling modules
use super::extraction::{BoundingBox, ExtractedImage};
use super::serde_helpers::serde_vec_arc;
use super::tables::Table;

/// Unified page structure for documents.
///
/// Supports different page types (PDF pages, PPTX slides, Excel sheets)
/// with character offset boundaries for chunk-to-page mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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

    /// Whether this page is blank (no meaningful text, no images, no tables)
    ///
    /// A page is considered blank if it has fewer than 3 non-whitespace characters
    /// and contains no tables or images. This is useful for filtering out empty pages
    /// in scanned documents or PDFs with blank separator pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_blank: Option<bool>,
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
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
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
    #[cfg_attr(feature = "api", schema(value_type = Vec<Table>))]
    pub tables: Vec<Arc<Table>>,

    /// Images found on this page (uses Arc for memory efficiency)
    ///
    /// Serializes as Vec<ExtractedImage> for JSON compatibility while maintaining
    /// Arc semantics in-memory for zero-copy sharing.
    #[serde(skip_serializing_if = "Vec::is_empty", default, with = "serde_vec_arc")]
    #[cfg_attr(feature = "api", schema(value_type = Vec<ExtractedImage>))]
    pub images: Vec<Arc<ExtractedImage>>,

    /// Hierarchy information for the page (when hierarchy extraction is enabled)
    ///
    /// Contains text hierarchy levels (H1-H6) extracted from the page content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hierarchy: Option<PageHierarchy>,

    /// Whether this page is blank (no meaningful text content)
    ///
    /// Determined during extraction based on text content analysis.
    /// A page is blank if it has fewer than 3 non-whitespace characters
    /// and contains no tables or images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_blank: Option<bool>,

    /// Layout detection regions for this page (when layout detection is enabled).
    ///
    /// Contains detected layout regions with class, confidence, bounding box,
    /// and area fraction. Only populated when layout detection is configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_regions: Option<Vec<LayoutRegion>>,
}

/// A detected layout region on a page.
///
/// When layout detection is enabled, each page may have layout regions
/// identifying different content types (text, pictures, tables, etc.)
/// with confidence scores and spatial positions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct LayoutRegion {
    /// Layout class name (e.g. "picture", "table", "text", "section_header").
    #[serde(alias = "class")]
    pub class_name: String,
    /// Confidence score from the layout detection model (0.0 to 1.0).
    pub confidence: f64,
    /// Bounding box in document coordinate space.
    pub bounding_box: BoundingBox,
    /// Fraction of the page area covered by this region (0.0 to 1.0).
    pub area_fraction: f64,
}

impl LayoutRegion {
    /// Deprecated: use the `class_name` field directly.
    #[deprecated(since = "4.10.0", note = "Use `class_name` field instead")]
    pub fn class(&self) -> &str {
        &self.class_name
    }
}

/// Page hierarchy structure containing heading levels and block information.
///
/// Used when PDF text hierarchy extraction is enabled. Contains hierarchical
/// blocks with heading levels (H1-H6) for semantic document structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PageHierarchy {
    /// Number of hierarchy blocks on this page
    pub block_count: usize,

    /// Hierarchical blocks with heading levels
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub blocks: Vec<HierarchicalBlock>,
}

/// A text block with hierarchy level assignment.
///
/// Represents a block of text with semantic heading information extracted from
/// font size clustering and hierarchical analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct HierarchicalBlock {
    /// The text content of this block
    pub text: String,

    /// The font size of the text in this block
    pub font_size: f32,

    /// The hierarchy level of this block (H1-H6 or Body)
    ///
    /// Levels correspond to HTML heading tags:
    /// - "h1": Top-level heading
    /// - "h2": Secondary heading
    /// - "h3": Tertiary heading
    /// - "h4": Quaternary heading
    /// - "h5": Quinary heading
    /// - "h6": Senary heading
    /// - "body": Body text (no heading level)
    pub level: String,

    /// Bounding box information for the block
    ///
    /// Contains coordinates as (left, top, right, bottom) in PDF units.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<(f32, f32, f32, f32)>,
}
