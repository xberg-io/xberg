//! Unified internal DTO for the PDF markdown pipeline.
//!
//! All extraction backends (pdfium structure tree, OCR)
//! produce `PageContent` which the shared pipeline converts to markdown.

use super::geometry::Rect;

/// Semantic role hint from the extraction source.
///
/// Set from PDF structure tree tags (`ContentRole`) or layout model predictions.
/// `None` when the source doesn't provide semantic information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SemanticRole {
    Heading { level: u8 },
    Paragraph,
    ListItem,
    Code,
    Formula,
    Caption,
    TableCell,
    Figure,
    BlockQuote,
    Other,
}

/// Granularity level of a content element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementLevel {
    /// Single word (OCR word-level, or whitespace-split from structure tree).
    Word,
    /// A full line of text (pdfium segment = one baseline run).
    Line,
    /// A block/paragraph of text (structure tree block).
    Block,
}

/// A single content element with normalized spatial and style data.
///
/// This is the unified representation that all extraction backends produce.
/// Fields that a backend cannot provide are set to defaults (`None`/`false`).
#[derive(Debug, Clone)]
pub(crate) struct ContentElement {
    /// Text content.
    pub text: String,
    /// Bounding box in PDF coordinate space (points, y=0 at page bottom).
    /// `None` when positional data is unavailable (e.g. some structure tree blocks).
    pub bbox: Option<Rect>,
    /// Font size in points. `None` for OCR (no font info available).
    pub font_size: Option<f32>,
    /// Whether the text is bold.
    pub is_bold: bool,
    /// Whether the text is italic.
    pub is_italic: bool,
    /// Whether the font is monospace/fixed-pitch.
    pub is_monospace: bool,
    /// Semantic role from the extraction source (structure tree or layout model).
    pub semantic_role: Option<SemanticRole>,
    /// Granularity of this element.
    pub level: ElementLevel,
    /// List item label (e.g. "1.", "a)", "•") when `semantic_role == ListItem`.
    pub list_label: Option<String>,
    /// Layout class hint from a layout detection model (e.g. OCR block type).
    /// `None` when layout detection is unavailable or not applicable.
    pub layout_class: Option<super::types::LayoutHintClass>,
}

/// All content extracted from a single page.
#[derive(Debug, Clone)]
pub(crate) struct PageContent {
    /// Extracted content elements in reading order.
    pub elements: Vec<ContentElement>,
}
