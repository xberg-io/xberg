//! Ordered text spans for output conversion.
//!
//! This module provides the OrderedTextSpan type which wraps TextSpan
//! with reading order information.

use crate::layout::TextSpan;
use std::sync::Arc;

/// Source of reading order assignment.
///
/// Tracks which strategy/method determined the reading order for a span.
/// This follows the SpaceSource pattern for consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadingOrderSource {
    /// Order from PDF structure tree (Tagged PDF).
    ///
    /// Confidence: 1.0 (explicit PDF semantic markup per ISO 32000-1:2008 Section 14.7).
    StructureTree,
    /// Order from XY-Cut recursive partitioning.
    ///
    /// Confidence: 0.90 (robust for multi-column layouts).
    XYCut,
    /// Order from article threads (`/Threads`, ISO 32000-1:2008 §12.4.3).
    ///
    /// Confidence: 0.95 (author-supplied explicit reading order; ranks below
    /// the structure tree but above geometric inference).
    ArticleThread,
    /// Order from geometric column analysis.
    ///
    /// Confidence: 0.85 (good for standard column layouts).
    Geometric,
    /// Order from simple top-to-bottom, left-to-right.
    ///
    /// Confidence: 0.75 (basic, works for single-column).
    #[default]
    Simple,
    /// Order explicitly set by user/API.
    ///
    /// Confidence: 1.0 (explicit assignment).
    UserAssigned,
    /// Fallback order (e.g., untagged spans in mixed document).
    ///
    /// Confidence: 0.65 (best-effort).
    Fallback,
}

impl ReadingOrderSource {
    /// Get the default confidence for this source type.
    pub fn default_confidence(&self) -> f32 {
        match self {
            ReadingOrderSource::StructureTree => 1.0,
            ReadingOrderSource::ArticleThread => 0.95,
            ReadingOrderSource::XYCut => 0.90,
            ReadingOrderSource::Geometric => 0.85,
            ReadingOrderSource::Simple => 0.75,
            ReadingOrderSource::UserAssigned => 1.0,
            ReadingOrderSource::Fallback => 0.65,
        }
    }

    /// Get strategy name for debugging.
    pub fn name(&self) -> &'static str {
        match self {
            ReadingOrderSource::StructureTree => "StructureTree",
            ReadingOrderSource::ArticleThread => "ArticleThread",
            ReadingOrderSource::XYCut => "XYCut",
            ReadingOrderSource::Geometric => "Geometric",
            ReadingOrderSource::Simple => "Simple",
            ReadingOrderSource::UserAssigned => "UserAssigned",
            ReadingOrderSource::Fallback => "Fallback",
        }
    }
}

/// Reading order metadata for a span.
///
/// Contains the source and confidence of the reading order assignment,
/// following the SpaceDecision pattern.
#[derive(Debug, Clone, Default)]
pub struct ReadingOrderInfo {
    /// Which strategy assigned this reading order.
    pub source: ReadingOrderSource,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
}

impl ReadingOrderInfo {
    /// Create with source and default confidence.
    pub fn from_source(source: ReadingOrderSource) -> Self {
        Self {
            confidence: source.default_confidence(),
            source,
        }
    }

    /// Create with explicit confidence.
    pub fn with_confidence(source: ReadingOrderSource, confidence: f32) -> Self {
        Self {
            source,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Create for structure tree source.
    pub fn structure_tree() -> Self {
        Self::from_source(ReadingOrderSource::StructureTree)
    }

    /// Create for XY-Cut source.
    pub fn xycut() -> Self {
        Self::from_source(ReadingOrderSource::XYCut)
    }

    /// Create for article-thread source (`/Threads`, §12.4.3).
    pub fn article_thread() -> Self {
        Self::from_source(ReadingOrderSource::ArticleThread)
    }

    /// Create for geometric source.
    pub fn geometric() -> Self {
        Self::from_source(ReadingOrderSource::Geometric)
    }

    /// Create for simple source.
    pub fn simple() -> Self {
        Self::from_source(ReadingOrderSource::Simple)
    }

    /// Create for fallback (untagged in mixed doc).
    pub fn fallback() -> Self {
        Self::from_source(ReadingOrderSource::Fallback)
    }
}

/// Logical role this span carries in the source PDF's structure tree.
///
/// Populated by the markdown / HTML converters when the underlying
/// document has a `/StructTreeRoot` and the span's MCID maps to a
/// recognised structure type (heading or list item). When `None`, the
/// converter falls back to its geometric/font-size heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructRole {
    /// Inside a heading element (H, H1..H6). Carries the level (1..6).
    Heading(u8),
    /// The MCR sits in an LI but no Lbl/LBody sub-element wraps it.
    /// Treat as a list-item body for emit purposes.
    ListItem,
    /// Inside the Lbl (label) sub-element of an LI.
    ListItemLabel,
    /// Inside the LBody (body) sub-element of an LI.
    ListItemBody,
}

/// A text span with an assigned reading order index.
///
/// This wrapper adds ordering information to TextSpan without modifying
/// the original span data. The reading_order field represents the position
/// in the final document output (0 = first to be read).
#[derive(Debug, Clone)]
pub struct OrderedTextSpan {
    /// The underlying text span.
    pub span: TextSpan,

    /// Index in reading order (0 = first to be read).
    pub reading_order: usize,

    /// Group ID for paragraph/section grouping (optional).
    pub group_id: Option<usize>,

    /// Reading order source and confidence information.
    pub order_info: ReadingOrderInfo,

    /// Structure-tree role for this span (heading level, list-item body
    /// etc.). Populated by the converter from the document's
    /// `/StructTreeRoot` when present. None when the document is untagged
    /// or the MCID has no recognised role.
    pub struct_role: Option<StructRole>,

    /// Block-id of this span's nearest paragraph-level structure
    /// ancestor (P, H*, LI, Sect, …). Two spans sharing this id belong
    /// to the same logical paragraph; a change between adjacent spans
    /// is a paragraph boundary even when the geometric gap is small
    /// (tight inter-paragraph layout).
    /// None for untagged documents.
    pub block_id: Option<u32>,

    /// Struct-tree-scope `/ActualText` replacement (ISO 32000-1:2008
    /// §14.9.4).
    ///
    /// When `Some(text)`, output converters emit `text` instead of
    /// `span.text` for this span. The span's bbox/font are still used
    /// for layout decisions (paragraph breaks, headings, list
    /// markers) so the replacement participates in the same reading
    /// flow as the underlying glyphs would have.
    ///
    /// When `Some("")`, the span is fully suppressed: its raw glyphs
    /// were covered by an ancestor ActualText scope whose replacement
    /// is emitted on a different span (the scope's anchor). Used to
    /// drop the non-anchor spans of a multi-MCID subtree without
    /// disturbing the reading-order vector's indexing.
    pub actualtext_replacement: Option<Arc<str>>,

    /// Hyperlink target for this span, if its bbox falls within a `/Link`
    /// annotation whose action resolves to a URI (ISO 32000-1 §12.5.6.5 +
    /// §12.6.4.7). Output converters emit `[text](uri)` / `<a href>` for runs
    /// of spans that share a URI. None when the span carries no link.
    pub link_uri: Option<Arc<str>>,

    /// True when this span is preformatted (nested under a `Code` element in
    /// the structure tree). Converters must preserve its line breaks and never
    /// reflow it into a paragraph.
    pub preformatted: bool,
}

impl OrderedTextSpan {
    /// Create a new ordered span with the given reading order.
    /// Uses Simple source as default for backward compatibility.
    pub fn new(span: TextSpan, reading_order: usize) -> Self {
        Self {
            span,
            reading_order,
            group_id: None,
            order_info: ReadingOrderInfo::default(),
            struct_role: None,
            block_id: None,
            actualtext_replacement: None,
            link_uri: None,
            preformatted: false,
        }
    }

    /// Create with explicit source info.
    pub fn with_info(span: TextSpan, reading_order: usize, order_info: ReadingOrderInfo) -> Self {
        Self {
            span,
            reading_order,
            group_id: None,
            order_info,
            struct_role: None,
            block_id: None,
            actualtext_replacement: None,
            link_uri: None,
            preformatted: false,
        }
    }

    /// Returns true when the span has been suppressed by a struct-tree
    /// ActualText emission attached to a sibling (or by the non-first-
    /// page coverage of a multi-page scope).
    ///
    /// Suppressed spans are dropped from the output vector by the
    /// applier in `document.rs`; this predicate is the single source
    /// of truth for "drop me".
    pub fn is_suppressed(&self) -> bool {
        matches!(self.actualtext_replacement.as_deref(), Some(""))
    }

    /// Set the structure-tree role propagated from the source PDF's
    /// `/StructTreeRoot`.
    pub fn with_struct_role(mut self, role: StructRole) -> Self {
        self.struct_role = Some(role);
        self
    }

    /// Set the group ID for paragraph grouping.
    pub fn with_group(mut self, group_id: usize) -> Self {
        self.group_id = Some(group_id);
        self
    }

    /// Set the reading order info.
    pub fn with_order_info(mut self, order_info: ReadingOrderInfo) -> Self {
        self.order_info = order_info;
        self
    }

    /// Get the reading order source.
    pub fn source(&self) -> ReadingOrderSource {
        self.order_info.source
    }

    /// Get the reading order confidence.
    pub fn confidence(&self) -> f32 {
        self.order_info.confidence
    }
}

/// A collection of ordered spans with helper methods.
pub struct OrderedSpans {
    spans: Vec<OrderedTextSpan>,
}

impl OrderedSpans {
    /// Create a new collection from a vector of ordered spans.
    pub fn new(spans: Vec<OrderedTextSpan>) -> Self {
        Self { spans }
    }

    /// Get the number of spans.
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Get spans sorted by reading order.
    pub fn in_reading_order(&self) -> Vec<&OrderedTextSpan> {
        let mut sorted: Vec<_> = self.spans.iter().collect();
        sorted.sort_by_key(|s| s.reading_order);
        sorted
    }

    /// Get the underlying spans.
    pub fn spans(&self) -> &[OrderedTextSpan] {
        &self.spans
    }

    /// Convert to a vector of ordered spans.
    pub fn into_vec(self) -> Vec<OrderedTextSpan> {
        self.spans
    }

    /// Group spans into lines based on Y-coordinate proximity.
    ///
    /// Returns groups of spans that appear on the same line.
    pub fn group_into_lines(&self, tolerance: f32) -> Vec<Vec<&OrderedTextSpan>> {
        if self.spans.is_empty() {
            return Vec::new();
        }

        let mut sorted: Vec<_> = self.spans.iter().collect();
        sorted.sort_by(|a, b| crate::utils::safe_float_cmp(b.span.bbox.y, a.span.bbox.y));

        let mut lines: Vec<Vec<&OrderedTextSpan>> = Vec::new();
        let mut current_line: Vec<&OrderedTextSpan> = vec![sorted[0]];
        let mut current_y = sorted[0].span.bbox.y;

        for span in sorted.into_iter().skip(1) {
            if (current_y - span.span.bbox.y).abs() <= tolerance {
                current_line.push(span);
            } else {
                lines.push(std::mem::take(&mut current_line));
                current_line = vec![span];
                current_y = span.span.bbox.y;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}

impl From<Vec<OrderedTextSpan>> for OrderedSpans {
    fn from(spans: Vec<OrderedTextSpan>) -> Self {
        Self::new(spans)
    }
}

impl IntoIterator for OrderedSpans {
    type Item = OrderedTextSpan;
    type IntoIter = std::vec::IntoIter<OrderedTextSpan>;

    fn into_iter(self) -> Self::IntoIter {
        self.spans.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight};

    fn make_span(text: &str, x: f32, y: f32, w: f32, h: f32) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(x, y, w, h),
            font_name: "Helvetica".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::new(0.0, 0.0, 0.0),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        }
    }

    #[test]
    fn test_reading_order_source_default() {
        let source = ReadingOrderSource::default();
        assert_eq!(source, ReadingOrderSource::Simple);
    }

    #[test]
    fn test_reading_order_source_confidences() {
        assert_eq!(ReadingOrderSource::StructureTree.default_confidence(), 1.0);
        assert_eq!(ReadingOrderSource::XYCut.default_confidence(), 0.90);
        assert_eq!(ReadingOrderSource::Geometric.default_confidence(), 0.85);
        assert_eq!(ReadingOrderSource::Simple.default_confidence(), 0.75);
        assert_eq!(ReadingOrderSource::UserAssigned.default_confidence(), 1.0);
        assert_eq!(ReadingOrderSource::Fallback.default_confidence(), 0.65);
    }

    #[test]
    fn test_reading_order_source_names() {
        assert_eq!(ReadingOrderSource::StructureTree.name(), "StructureTree");
        assert_eq!(ReadingOrderSource::XYCut.name(), "XYCut");
        assert_eq!(ReadingOrderSource::Geometric.name(), "Geometric");
        assert_eq!(ReadingOrderSource::Simple.name(), "Simple");
        assert_eq!(ReadingOrderSource::UserAssigned.name(), "UserAssigned");
        assert_eq!(ReadingOrderSource::Fallback.name(), "Fallback");
    }

    #[test]
    fn test_reading_order_source_debug() {
        let debug = format!("{:?}", ReadingOrderSource::XYCut);
        assert!(debug.contains("XYCut"));
    }

    #[test]
    fn test_reading_order_source_clone_copy_eq() {
        let source = ReadingOrderSource::Geometric;
        let copied = source;
        let cloned = source;
        assert_eq!(source, copied);
        assert_eq!(source, cloned);
        assert_ne!(source, ReadingOrderSource::Fallback);
    }

    #[test]
    fn test_reading_order_info_default() {
        let info = ReadingOrderInfo::default();
        assert_eq!(info.source, ReadingOrderSource::Simple);
        assert_eq!(info.confidence, 0.0); // Default f32 ~keep
    }

    #[test]
    fn test_reading_order_info_from_source() {
        let info = ReadingOrderInfo::from_source(ReadingOrderSource::StructureTree);
        assert_eq!(info.source, ReadingOrderSource::StructureTree);
        assert_eq!(info.confidence, 1.0);
    }

    #[test]
    fn test_reading_order_info_with_confidence() {
        let info = ReadingOrderInfo::with_confidence(ReadingOrderSource::XYCut, 0.95);
        assert_eq!(info.source, ReadingOrderSource::XYCut);
        assert_eq!(info.confidence, 0.95);
    }

    #[test]
    fn test_reading_order_info_with_confidence_clamped() {
        let info = ReadingOrderInfo::with_confidence(ReadingOrderSource::Simple, 1.5);
        assert_eq!(info.confidence, 1.0);

        let info2 = ReadingOrderInfo::with_confidence(ReadingOrderSource::Simple, -0.5);
        assert_eq!(info2.confidence, 0.0);
    }

    #[test]
    fn test_reading_order_info_convenience_constructors() {
        assert_eq!(
            ReadingOrderInfo::structure_tree().source,
            ReadingOrderSource::StructureTree
        );
        assert_eq!(ReadingOrderInfo::xycut().source, ReadingOrderSource::XYCut);
        assert_eq!(ReadingOrderInfo::geometric().source, ReadingOrderSource::Geometric);
        assert_eq!(ReadingOrderInfo::simple().source, ReadingOrderSource::Simple);
        assert_eq!(ReadingOrderInfo::fallback().source, ReadingOrderSource::Fallback);
    }

    #[test]
    fn test_ordered_text_span_new() {
        let span = make_span("Hello", 10.0, 20.0, 50.0, 12.0);
        let ordered = OrderedTextSpan::new(span, 0);
        assert_eq!(ordered.reading_order, 0);
        assert!(ordered.group_id.is_none());
        assert_eq!(ordered.source(), ReadingOrderSource::Simple);
    }

    #[test]
    fn test_ordered_text_span_with_info() {
        let span = make_span("World", 10.0, 20.0, 50.0, 12.0);
        let info = ReadingOrderInfo::structure_tree();
        let ordered = OrderedTextSpan::with_info(span, 5, info);
        assert_eq!(ordered.reading_order, 5);
        assert_eq!(ordered.source(), ReadingOrderSource::StructureTree);
        assert_eq!(ordered.confidence(), 1.0);
    }

    #[test]
    fn test_ordered_text_span_with_group() {
        let span = make_span("Test", 10.0, 20.0, 50.0, 12.0);
        let ordered = OrderedTextSpan::new(span, 0).with_group(3);
        assert_eq!(ordered.group_id, Some(3));
    }

    #[test]
    fn test_ordered_text_span_with_order_info() {
        let span = make_span("Test", 10.0, 20.0, 50.0, 12.0);
        let ordered = OrderedTextSpan::new(span, 0).with_order_info(ReadingOrderInfo::xycut());
        assert_eq!(ordered.source(), ReadingOrderSource::XYCut);
    }

    #[test]
    fn test_ordered_spans_empty() {
        let spans = OrderedSpans::new(vec![]);
        assert!(spans.is_empty());
        assert_eq!(spans.len(), 0);
    }

    #[test]
    fn test_ordered_spans_basic() {
        let s1 = OrderedTextSpan::new(make_span("A", 10.0, 20.0, 50.0, 12.0), 1);
        let s2 = OrderedTextSpan::new(make_span("B", 70.0, 20.0, 50.0, 12.0), 0);
        let spans = OrderedSpans::new(vec![s1, s2]);
        assert_eq!(spans.len(), 2);
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_ordered_spans_in_reading_order() {
        let s1 = OrderedTextSpan::new(make_span("Second", 10.0, 20.0, 50.0, 12.0), 1);
        let s2 = OrderedTextSpan::new(make_span("First", 70.0, 20.0, 50.0, 12.0), 0);
        let spans = OrderedSpans::new(vec![s1, s2]);

        let ordered = spans.in_reading_order();
        assert_eq!(ordered[0].span.text, "First");
        assert_eq!(ordered[1].span.text, "Second");
    }

    #[test]
    fn test_ordered_spans_spans() {
        let s1 = OrderedTextSpan::new(make_span("A", 10.0, 20.0, 50.0, 12.0), 0);
        let spans = OrderedSpans::new(vec![s1]);
        assert_eq!(spans.spans().len(), 1);
    }

    #[test]
    fn test_ordered_spans_into_vec() {
        let s1 = OrderedTextSpan::new(make_span("A", 10.0, 20.0, 50.0, 12.0), 0);
        let spans = OrderedSpans::new(vec![s1]);
        let vec = spans.into_vec();
        assert_eq!(vec.len(), 1);
    }

    #[test]
    fn test_ordered_spans_from_vec() {
        let s1 = OrderedTextSpan::new(make_span("A", 10.0, 20.0, 50.0, 12.0), 0);
        let spans: OrderedSpans = vec![s1].into();
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn test_ordered_spans_into_iter() {
        let s1 = OrderedTextSpan::new(make_span("A", 10.0, 20.0, 50.0, 12.0), 0);
        let s2 = OrderedTextSpan::new(make_span("B", 70.0, 20.0, 50.0, 12.0), 1);
        let spans = OrderedSpans::new(vec![s1, s2]);

        let collected: Vec<_> = spans.into_iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn test_ordered_spans_group_into_lines_empty() {
        let spans = OrderedSpans::new(vec![]);
        let lines = spans.group_into_lines(2.0);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_ordered_spans_group_into_lines_single() {
        let s1 = OrderedTextSpan::new(make_span("A", 10.0, 100.0, 50.0, 12.0), 0);
        let spans = OrderedSpans::new(vec![s1]);
        let lines = spans.group_into_lines(2.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1);
    }

    #[test]
    fn test_ordered_spans_group_into_lines_same_line() {
        let s1 = OrderedTextSpan::new(make_span("A", 10.0, 100.0, 50.0, 12.0), 0);
        let s2 = OrderedTextSpan::new(make_span("B", 70.0, 101.0, 50.0, 12.0), 1);
        let spans = OrderedSpans::new(vec![s1, s2]);
        let lines = spans.group_into_lines(2.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 2);
    }

    #[test]
    fn test_ordered_spans_group_into_lines_different_lines() {
        let s1 = OrderedTextSpan::new(make_span("Line1", 10.0, 100.0, 50.0, 12.0), 0);
        let s2 = OrderedTextSpan::new(make_span("Line2", 10.0, 80.0, 50.0, 12.0), 1);
        let spans = OrderedSpans::new(vec![s1, s2]);
        let lines = spans.group_into_lines(2.0);
        assert_eq!(lines.len(), 2);
    }
}
