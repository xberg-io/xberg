//! Column-aware geometric reading order strategy.

use crate::error::Result;
use crate::layout::TextSpan;
use crate::pipeline::{OrderedTextSpan, ReadingOrderInfo};

use super::{ReadingOrderContext, ReadingOrderStrategy};

/// Column-aware geometric reading order strategy.
///
/// This strategy detects columns based on horizontal gaps and processes
/// each column from top to bottom before moving to the next column.
///
/// This is useful for multi-column documents like academic papers,
/// newspapers, and magazines.
pub struct GeometricStrategy {
    /// Minimum gap between columns (in points).
    column_gap_threshold: f32,
}

impl GeometricStrategy {
    /// Create a new geometric strategy with default settings.
    pub fn new() -> Self {
        Self {
            column_gap_threshold: 20.0,
        }
    }

    /// Create a geometric strategy with custom column gap threshold.
    pub fn with_column_gap(threshold: f32) -> Self {
        Self {
            column_gap_threshold: threshold,
        }
    }

    /// Detect columns based on horizontal gaps.
    ///
    /// Returns column boundaries as X coordinates.
    ///
    /// # Phase 8 Enhancement: Adaptive Column Detection
    ///
    /// Instead of using a fixed threshold, this method now analyzes the gap
    /// distribution to find natural column boundaries:
    /// 1. Collects all horizontal gaps between span right edges and next span left edges
    /// 2. Calculates median gap to understand typical word spacing
    /// 3. Uses a multiplier to detect column gaps (significantly larger than word gaps)
    fn detect_columns(&self, spans: &[TextSpan]) -> Vec<f32> {
        if spans.is_empty() {
            return Vec::new();
        }

        let effective_threshold = self.calculate_adaptive_threshold(spans);

        let mut x_coords: Vec<f32> = spans.iter().map(|s| s.bbox.x).collect();
        x_coords.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        x_coords.dedup();

        if x_coords.len() < 2 {
            return vec![x_coords.first().copied().unwrap_or(0.0)];
        }

        let mut boundaries = vec![x_coords[0]];

        for i in 1..x_coords.len() {
            let gap = x_coords[i] - x_coords[i - 1];
            if gap > effective_threshold {
                boundaries.push(x_coords[i]);
            }
        }

        boundaries
    }

    /// Calculate adaptive column gap threshold based on document characteristics.
    ///
    /// Phase 8: Uses statistical analysis of horizontal gaps to detect
    /// column boundaries more accurately for documents with varying layouts.
    ///
    /// Uses left-edge-to-left-edge gaps (same as column detection) for consistency.
    fn calculate_adaptive_threshold(&self, spans: &[TextSpan]) -> f32 {
        if spans.len() < 2 {
            return self.column_gap_threshold;
        }

        let mut x_coords: Vec<f32> = spans.iter().map(|s| s.bbox.x).collect();
        x_coords.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        x_coords.dedup();

        if x_coords.len() < 2 {
            return self.column_gap_threshold;
        }

        let mut gaps: Vec<f32> = Vec::new();
        for i in 1..x_coords.len() {
            let gap = x_coords[i] - x_coords[i - 1];
            if gap > 0.0 {
                gaps.push(gap);
            }
        }

        if gaps.is_empty() {
            return self.column_gap_threshold;
        }

        // Need multiple gaps to compute meaningful statistics
        // If only one or two gaps, use the configured threshold ~keep
        if gaps.len() < 3 {
            return self.column_gap_threshold;
        }

        gaps.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));

        // Use the 25th percentile as "typical" word spacing
        // This is more robust than median for documents with varying layouts ~keep
        let p25_idx = gaps.len() / 4;
        let typical_gap = gaps[p25_idx];

        // Column gaps should be significantly larger than typical word gaps
        // Use 4x typical as the threshold (columns are much wider than word spacing) ~keep
        let adaptive_threshold = typical_gap * 4.0;

        let final_threshold = adaptive_threshold.max(self.column_gap_threshold);

        tracing::debug!(
            typical_gap,
            adaptive_threshold,
            final_threshold,
            "adaptive column detection"
        );

        final_threshold
    }
}

impl Default for GeometricStrategy {
    fn default() -> Self {
        Self::new()
    }
}

/// Strict check for whether a page's spans form a real multi-column layout.
///
/// Per ISO 32000-1:2008 §14.8.2.3.1 reading order proceeds top-to-bottom
/// (and "from column to column" only "in a multiple-column layout"). The
/// criterion for "is this multiple-column?" is up to the implementer.
/// Phase 3 of the reading-order refactor tightens it to:
///
///   - ≥3 distinct vertical whitespace gutters between merged text bands
///   - Each gutter ≥ `median_char_width × 4` wide
///   - Text bands on both sides of each gutter (implicit: a gutter is a
///     gap between two non-empty merged x-intervals)
///
/// The 3-gutter bar is deliberately strict: it picks up 4+-column
/// newsletters and not 2-column form layouts. 2-column academic papers
/// SHOULD reach the column-aware path via the struct tree (tagged) or
/// by passing `reading_order="column_aware"` explicitly. Untagged
/// 2-column docs default to single-column ordering, matching pdfplumber.
fn is_likely_columnar(spans: &[TextSpan]) -> bool {
    if spans.len() < 6 {
        return false;
    }

    // Median char width estimate — font_size × 0.5 is a rough but stable
    // proxy across most Latin fonts (lowercase x-height ≈ 0.5 em). ~keep
    let mut sizes: Vec<f32> = spans.iter().map(|s| s.font_size).collect();
    sizes.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    let median_size = sizes[sizes.len() / 2];
    let gutter_min = (median_size * 0.5) * 4.0;

    let mut intervals: Vec<(f32, f32)> = spans.iter().map(|s| (s.bbox.x, s.bbox.x + s.bbox.width)).collect();
    intervals.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));

    let mut merged: Vec<(f32, f32)> = Vec::new();
    for (start, end) in intervals {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
            continue;
        }
        merged.push((start, end));
    }

    let mut gutters = 0;
    for w in merged.windows(2) {
        let gap = w[1].0 - w[0].1;
        if gap >= gutter_min {
            gutters += 1;
        }
    }

    gutters >= 3
}

/// Split each column group by large Y-gaps into sub-groups.
///
/// When a column has spans far apart vertically (e.g., header at y=651
/// and content at y=119), they should be separate groups. ~keep
fn split_columns_at_y_gaps(spans: &[TextSpan], column_indices: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut sub_groups: Vec<Vec<usize>> = Vec::new();
    for column in column_indices {
        if column.is_empty() {
            continue;
        }
        let mut sorted = column.clone();
        sorted.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[b].bbox.y, spans[a].bbox.y));

        if sorted.len() == 1 {
            sub_groups.push(sorted);
            continue;
        }

        let mut gaps: Vec<f32> = Vec::new();
        for i in 1..sorted.len() {
            let gap = spans[sorted[i - 1]].bbox.y - spans[sorted[i]].bbox.y;
            if gap > 0.0 {
                gaps.push(gap);
            }
        }

        let threshold = if gaps.is_empty() {
            spans[sorted[0]].font_size * 4.5
        } else {
            let avg = gaps.iter().sum::<f32>() / gaps.len() as f32;
            avg * 3.0
        };

        let mut current_sub = vec![sorted[0]];
        for i in 1..sorted.len() {
            let gap = spans[sorted[i - 1]].bbox.y - spans[sorted[i]].bbox.y;
            if gap > threshold {
                sub_groups.push(current_sub);
                current_sub = vec![sorted[i]];
            } else {
                current_sub.push(sorted[i]);
            }
        }
        sub_groups.push(current_sub);
    }
    sub_groups
}

impl ReadingOrderStrategy for GeometricStrategy {
    fn apply(&self, spans: Vec<TextSpan>, _context: &ReadingOrderContext) -> Result<Vec<OrderedTextSpan>> {
        if spans.is_empty() {
            return Ok(Vec::new());
        }

        // Single-column path (the new default).
        // Sort by y descending (top first) using the row-aware comparator
        // that quantizes near-equal y values into bands so same-line items
        // sort by x within the band. Matches pdfplumber's default and
        // resolves the form-style false-column-detection bug. ~keep
        if !is_likely_columnar(&spans) {
            let mut indexed: Vec<(usize, TextSpan)> = spans.into_iter().enumerate().collect();
            indexed.sort_by(|(_, a), (_, b)| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
            return Ok(indexed
                .into_iter()
                .enumerate()
                .map(|(order, (_, span))| OrderedTextSpan::with_info(span, order, ReadingOrderInfo::geometric()))
                .collect());
        }

        let boundaries = self.detect_columns(&spans);

        let mut column_indices: Vec<Vec<usize>> = vec![Vec::new(); boundaries.len().max(1)];
        for (idx, span) in spans.iter().enumerate() {
            let column_idx = boundaries
                .iter()
                .enumerate()
                .rev()
                .find(|&(_, &boundary)| span.bbox.x >= boundary)
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            column_indices[column_idx].push(idx);
        }

        let sub_groups = split_columns_at_y_gaps(&spans, &column_indices);

        let mut ordered = Vec::new();
        let mut order = 0;

        for (group_id, group) in sub_groups.into_iter().enumerate() {
            for idx in group {
                ordered.push(
                    OrderedTextSpan::with_info(spans[idx].clone(), order, ReadingOrderInfo::geometric())
                        .with_group(group_id),
                );
                order += 1;
            }
        }

        Ok(ordered)
    }

    fn name(&self) -> &'static str {
        "GeometricStrategy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight};

    fn make_span(text: &str, x: f32, y: f32) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(x, y, 50.0, 12.0),
            font_name: "Test".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            offset_semantic: false,
            split_boundary_before: false,
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
    fn test_single_column() {
        let spans = vec![
            make_span("Line 3", 50.0, 50.0),
            make_span("Line 1", 50.0, 100.0),
            make_span("Line 2", 50.0, 75.0),
        ];

        let strategy = GeometricStrategy::new();
        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        assert_eq!(ordered[0].span.text, "Line 1");
        assert_eq!(ordered[1].span.text, "Line 2");
        assert_eq!(ordered[2].span.text, "Line 3");
    }

    #[test]
    fn test_two_columns_default_to_row_order() {
        // Phase 3: 2-column synthetic input (1 gutter) does NOT
        // trigger column-aware mode under the strict ≥3-gutter criterion.
        // Output is row-aware: same-y items together left-to-right, then
        // next y row. ~keep
        let spans = vec![
            make_span("Left 1", 50.0, 100.0),
            make_span("Left 2", 50.0, 50.0),
            make_span("Right 1", 200.0, 100.0),
            make_span("Right 2", 200.0, 50.0),
        ];

        let strategy = GeometricStrategy::new();
        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        assert_eq!(ordered[0].span.text, "Left 1");
        assert_eq!(ordered[1].span.text, "Right 1");
        assert_eq!(ordered[2].span.text, "Left 2");
        assert_eq!(ordered[3].span.text, "Right 2");
    }

    #[test]
    fn test_is_likely_columnar_gating() {
        let two_col = vec![
            make_span("A", 50.0, 100.0),
            make_span("B", 50.0, 80.0),
            make_span("C", 50.0, 60.0),
            make_span("D", 200.0, 100.0),
            make_span("E", 200.0, 80.0),
            make_span("F", 200.0, 60.0),
        ];
        assert!(
            !is_likely_columnar(&two_col),
            "2-column layout (1 gutter) must default to single-column"
        );

        let four_col = vec![
            make_span("A", 50.0, 100.0),
            make_span("A2", 50.0, 80.0),
            make_span("B", 200.0, 100.0),
            make_span("B2", 200.0, 80.0),
            make_span("C", 350.0, 100.0),
            make_span("C2", 350.0, 80.0),
            make_span("D", 500.0, 100.0),
            make_span("D2", 500.0, 80.0),
        ];
        assert!(
            is_likely_columnar(&four_col),
            "4-column layout (3 wide gutters) must trigger column-aware mode"
        );

        let tiny = vec![
            make_span("A", 50.0, 100.0),
            make_span("B", 200.0, 100.0),
            make_span("C", 350.0, 100.0),
            make_span("D", 500.0, 100.0),
        ];
        assert!(!is_likely_columnar(&tiny), "fewer than 6 spans is not enough signal");
    }

    #[test]
    fn test_single_column_y_gap_does_not_split_under_strict_criterion() {
        // Phase 3: a single x-band with a large y-gap is no longer
        // partitioned into separate "groups" by the geometric strategy. The
        // strategy is now responsible only for ordering — paragraph / section
        // breaks are derived elsewhere (struct tree on tagged docs, line
        // clustering on untagged). Output is simple top-to-bottom. ~keep
        let spans = vec![
            make_span("Header1", 50.0, 700.0),
            make_span("Header2", 50.0, 690.0),
            make_span("Header3", 50.0, 680.0),
            make_span("Content1", 50.0, 280.0),
            make_span("Content2", 50.0, 270.0),
            make_span("Content3", 50.0, 260.0),
        ];

        let strategy = GeometricStrategy::new();
        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        assert_eq!(ordered.len(), 6);
        assert_eq!(ordered[0].span.text, "Header1");
        assert_eq!(ordered[5].span.text, "Content3");
    }

    #[test]
    fn test_two_column_form_does_not_trigger_column_aware() {
        // Form-style 2-column label/value layout: 1 gutter, narrower than
        // the strict gutter_min. Must default to row-aware single-column. ~keep
        let spans = vec![
            make_span("Word1", 50.0, 100.0),
            make_span("Word2", 55.0, 100.0),
            make_span("Word3", 60.0, 100.0),
            make_span("Word4", 50.0, 50.0),
            make_span("Word5", 55.0, 50.0),
            // "Right column" — but only 140pt away with a single gutter. ~keep
            make_span("RightWord1", 200.0, 100.0),
            make_span("RightWord2", 200.0, 50.0),
        ];

        let strategy = GeometricStrategy::new();
        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        let pos = |t: &str| ordered.iter().position(|s| s.span.text == t).unwrap();
        assert!(
            pos("RightWord1") < pos("Word4"),
            "RightWord1 (y=100) must precede Word4 (y=50) in row-aware order"
        );
    }
}
