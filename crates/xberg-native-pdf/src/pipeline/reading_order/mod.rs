//! Reading order strategies for text extraction.
//!
//! This module provides pluggable strategies for determining the reading order
//! of text spans extracted from PDF pages.
//!
//! # Available Strategies
//!
//! - [`StructureTreeStrategy`]: Uses PDF structure tree MCIDs for Tagged PDFs
//! - [`GeometricStrategy`]: Column-aware geometric analysis
//! - [`XYCutStrategy`]: Recursive XY-Cut spatial partitioning (newspapers, academic papers)
//! - [`SimpleStrategy`]: Simple top-to-bottom, left-to-right ordering

pub mod article_thread;
pub mod detectors;
pub mod geometric;
pub mod simple;
pub mod structure_tree;
pub mod tategaki;
pub mod xycut;
mod xycut_legend;

pub use article_thread::ArticleThreadStrategy;
pub use detectors::{
    DetectorGlyph, ReadingOrderClass, classify_region, detect_dense_single_line, detect_dramatic_script,
    detect_narrow_tracked, detect_sub_super_glyphs,
};
pub use geometric::GeometricStrategy;
pub use simple::SimpleStrategy;
pub use structure_tree::StructureTreeStrategy;
pub use tategaki::TategakiStrategy;
pub use xycut::XYCutStrategy;

use crate::error::Result;
use crate::geometry::Rect;
use crate::layout::TextSpan;
use crate::pipeline::OrderedTextSpan;
use crate::pipeline::config::ReadingOrderStrategyType;

/// Trait for determining reading order of text spans.
///
/// Implementations decide how to order spans for reading. This is a key
/// abstraction point as different PDF types require different strategies:
///
/// - Tagged PDFs: Use structure tree MCIDs (PDF-spec-compliant)
/// - Multi-column: Use geometric column detection
/// - Simple: Top-to-bottom, left-to-right
pub trait ReadingOrderStrategy: Send + Sync {
    /// Apply reading order to a collection of spans.
    ///
    /// # Arguments
    ///
    /// * `spans` - Unordered text spans extracted from the page
    /// * `context` - Optional context information (structure tree, page info)
    ///
    /// # Returns
    ///
    /// Spans with assigned reading order indices.
    fn apply(&self, spans: Vec<TextSpan>, context: &ReadingOrderContext) -> Result<Vec<OrderedTextSpan>>;

    /// Return the name of this strategy for debugging.
    fn name(&self) -> &'static str;
}

/// Context information for reading order determination.
#[derive(Debug, Default)]
pub struct ReadingOrderContext {
    /// Current page number (0-indexed).
    pub page_number: u32,

    /// Page bounding box (if available).
    pub page_bbox: Option<Rect>,

    /// Whether the document has a structure tree (Tagged PDF).
    pub has_structure_tree: bool,

    /// MCID to reading order mapping (if structure tree available).
    pub mcid_order: Option<Vec<u32>>,

    /// Ordered article-thread bead rectangles for this page, in `/N` order
    /// (ISO 32000-1:2008 §12.4.3). Set only when the document declares
    /// `/Threads`, no trustworthy structure tree governs the page, and the
    /// beads cover enough of the page text to be trusted. When present, the
    /// reading-order strategy threads spans through these regions instead of
    /// using pure geometric inference. `None` ⇒ the geometric path is unchanged
    /// (fails closed).
    pub bead_rects: Option<Vec<Rect>>,

    /// Whether the structure tree contains suspect (unreliable) content.
    ///
    /// Per ISO 32000-1:2008 Section 14.7.1, when this is true, the structure
    /// tree may be unreliable and reading order strategies should consider
    /// falling back to geometric ordering.
    pub suspects: bool,

    /// Mid-X of the column gutter the caller already detected for this page,
    /// when it established the page is multi-column.
    ///
    /// The XY-cut's heading-run pre-pass walks ONE page-global top-to-bottom
    /// order, so a heading opening the other column can sort between a wrapped
    /// heading's two lines. Knowing where the gutter is lets the pre-pass tell
    /// that case apart from two `Tj` segments of one heading line (GH#1757).
    /// `None` ⇒ no gutter is known and a conservative horizontal-adjacency test
    /// stands in.
    pub column_gutter: Option<f32>,

    /// Preserve the caller-supplied span order verbatim instead of running a
    /// reading-order strategy. Set when the converter has already established a
    /// non-geometric order the strategies cannot reproduce — currently the
    /// two-column-prose column-major reorder, whose ordering the
    /// geometric XY-cut fallback would otherwise re-derive as row-major and
    /// interleave the columns. Ignored when `mcid_order` is present (a tagged
    /// PDF's structure order wins).
    pub preserve_input_order: bool,
}

impl ReadingOrderContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the page number.
    pub fn with_page(mut self, page_number: u32) -> Self {
        self.page_number = page_number;
        self
    }

    /// Set the page bounding box.
    pub fn with_bbox(mut self, bbox: Rect) -> Self {
        self.page_bbox = Some(bbox);
        self
    }

    /// Set MCID order from structure tree traversal.
    pub fn with_mcid_order(mut self, mcid_order: Vec<u32>) -> Self {
        self.has_structure_tree = true;
        self.mcid_order = Some(mcid_order);
        self
    }

    /// Set whether the structure tree contains suspect content.
    ///
    /// When true, the StructureTreeStrategy will fall back to geometric
    /// ordering instead of trusting the potentially unreliable structure tree.
    pub fn with_suspects(mut self, suspects: bool) -> Self {
        self.suspects = suspects;
        self
    }

    /// Preserve the caller-supplied span order (see `preserve_input_order`).
    pub fn with_preserve_input_order(mut self, preserve: bool) -> Self {
        self.preserve_input_order = preserve;
        self
    }

    /// Set the detected column gutter mid-X (see `column_gutter`).
    pub fn with_column_gutter(mut self, gutter_x: f32) -> Self {
        self.column_gutter = Some(gutter_x);
        self
    }

    /// Set the ordered article-thread bead rectangles for this page.
    pub fn with_bead_rects(mut self, bead_rects: Vec<Rect>) -> Self {
        self.bead_rects = Some(bead_rects);
        self
    }
}

/// Mid-X of the page's column gutter, or `None` when the page is not a clean
/// two-column layout.
///
/// Feed the result to [`ReadingOrderContext::with_column_gutter`] at every
/// XY-cut entry point whose ordering reaches a caller. The heading-run
/// pre-pass uses it to tell a heading that opens the other column apart from a
/// second `Tj` segment of the same heading line, and is inert without it
/// (GH#1757) — so an entry point that omits it leaves that defect live on its
/// own path regardless of what the other entry points pass.
pub fn detect_column_gutter(spans: &[TextSpan]) -> Option<f32> {
    crate::document::PdfDocument::detect_column_gutter(spans)
}

/// Create a reading order strategy based on configuration.
pub fn create_strategy(config: &crate::pipeline::ReadingOrderConfig) -> Box<dyn ReadingOrderStrategy> {
    match config.strategy {
        ReadingOrderStrategyType::StructureTreeFirst => Box::new(StructureTreeStrategy::new()),
        ReadingOrderStrategyType::Geometric => Box::new(GeometricStrategy::new()),
        ReadingOrderStrategyType::XYCut => Box::new(XYCutStrategy::new()),
        ReadingOrderStrategyType::Simple => Box::new(SimpleStrategy),
    }
}
