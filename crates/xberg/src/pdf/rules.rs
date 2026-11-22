//! Drawn ruling-line detection from a PDF page's vector paths.
//!
//! Shared by the adaptive layout pre-screen ([`super::layout_gate`], which is
//! only compiled with the `layout-detection` feature) and the borderless
//! heuristic table gate in [`super::native::table`], which is compiled whenever
//! `pdf` is. It lives here rather than in `layout_gate` so the heuristic gate's
//! behaviour does not silently change with the `layout-detection` feature — a
//! table admitted in one build and rejected in another would be worse than
//! either answer on its own (xberg-io/xberg#1399).

/// Horizontal tolerance when clustering span edges into table column anchors,
/// and the maximum thickness of a line still considered a rule.
pub(crate) const GRID_EDGE_ALIGN_TOLERANCE_PTS: f32 = 3.0;

/// Straight lines shorter than this are decoration, not rules, in points.
pub(crate) const RULED_LINE_MIN_LENGTH_PTS: f32 = 20.0;

/// Count horizontal and vertical rules at least [`RULED_LINE_MIN_LENGTH_PTS`]
/// long. A rule must also be thin (minor bbox dimension within
/// [`GRID_EDGE_ALIGN_TOLERANCE_PTS`]) so long diagonals on chart-heavy pages
/// do not count.
///
/// Producers draw a rule either as a stroked segment or as a filled bar — a
/// 1 pt-high `re` rectangle with no stroke is what SERFF rate tables and many
/// print-era form generators emit. Counting only stroked segments makes a
/// ruled table look rule-less to the borderless gate, which then loses it to
/// the columnar-prose guard (xberg-io/xberg#3352).
#[cfg_attr(not(feature = "layout-detection"), allow(dead_code))] // ~keep: the adaptive layout pre-screen is the only build that still counts page-wide
pub(crate) fn count_rules(paths: &[xberg_native_pdf::elements::PathContent]) -> (usize, usize) {
    let mut horizontal = 0usize;
    let mut vertical = 0usize;
    for path in paths {
        match rule_orientation(path) {
            Some(RuleOrientation::Horizontal) => horizontal += 1,
            Some(RuleOrientation::Vertical) => vertical += 1,
            None => {}
        }
    }
    (horizontal, vertical)
}

/// Vertical centre, in PDF user space, of a horizontal rule that spans the
/// full width `[x0, x1]`.
///
/// A producer draws a table's top and bottom rule just outside the text it
/// frames, so a caller that only looks *between* a region's words sees no rule
/// at all on a table ruled only at its edges. Exposing the rule's own
/// coordinate lets that caller compare it with a pitch-extended band instead.
pub(crate) fn horizontal_rule_center_y(
    path: &xberg_native_pdf::elements::PathContent,
    x0: f32,
    x1: f32,
) -> Option<f32> {
    if rule_orientation(path) != Some(RuleOrientation::Horizontal) {
        return None;
    }
    let rule_x0 = path.bbox.x;
    let rule_x1 = path.bbox.x + path.bbox.width;
    let spans = rule_x0 <= x0 + GRID_EDGE_ALIGN_TOLERANCE_PTS && rule_x1 >= x1 - GRID_EDGE_ALIGN_TOLERANCE_PTS;
    spans.then_some(path.bbox.y + path.bbox.height * 0.5)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuleOrientation {
    Horizontal,
    Vertical,
}

fn rule_orientation(path: &xberg_native_pdf::elements::PathContent) -> Option<RuleOrientation> {
    let width = path.bbox.width.abs();
    let height = path.bbox.height.abs();
    let is_rule_shape = path.is_straight_line() || painted_rule_bar(path);
    if !is_rule_shape {
        return None;
    }
    if width >= RULED_LINE_MIN_LENGTH_PTS && height <= GRID_EDGE_ALIGN_TOLERANCE_PTS {
        Some(RuleOrientation::Horizontal)
    } else if height >= RULED_LINE_MIN_LENGTH_PTS && width <= GRID_EDGE_ALIGN_TOLERANCE_PTS {
        Some(RuleOrientation::Vertical)
    } else {
        None
    }
}

/// A filled, unstroked rectangle: the bar a producer paints in place of a
/// stroked rule. Requires the rectangle op or a four-corner closed path so a
/// text-shaped or glyph-shaped fill cannot be mistaken for one. ~keep
fn painted_rule_bar(path: &xberg_native_pdf::elements::PathContent) -> bool {
    use xberg_native_pdf::elements::PathOperation;
    if !path.has_fill() || path.has_stroke() {
        return false;
    }
    matches!(
        path.operations.as_slice(),
        [PathOperation::Rectangle(..)]
            | [
                PathOperation::MoveTo(..),
                PathOperation::LineTo(..),
                PathOperation::LineTo(..),
                PathOperation::LineTo(..),
            ]
            | [
                PathOperation::MoveTo(..),
                PathOperation::LineTo(..),
                PathOperation::LineTo(..),
                PathOperation::LineTo(..),
                PathOperation::ClosePath,
            ]
    )
}
#[cfg(test)]
mod tests {
    // Rule counting for the borderless-table admission gate.

    use super::{RuleOrientation, rule_orientation};
    use xberg_native_pdf::elements::PathContent;
    use xberg_native_pdf::elements::PathOperation;
    use xberg_native_pdf::layout::Color;

    const RULE_WIDTH: f32 = 400.0;

    fn stroked_line(y: f32) -> PathContent {
        PathContent::from_operations(vec![
            PathOperation::MoveTo(60.0, y),
            PathOperation::LineTo(60.0 + RULE_WIDTH, y),
        ])
    }

    fn painted_bar(y: f32) -> PathContent {
        PathContent::from_operations(vec![PathOperation::Rectangle(60.0, y, RULE_WIDTH, 1.0)])
            .with_fill(Color::black())
            .with_stroke_width(0.0)
    }

    #[test]
    fn counts_a_stroked_segment_as_a_horizontal_rule() {
        assert_eq!(
            rule_orientation(&stroked_line(300.0)),
            Some(RuleOrientation::Horizontal)
        );
    }

    #[test]
    fn counts_a_filled_unstroked_bar_as_a_horizontal_rule() {
        // How SERFF and other print-era form generators draw table rules
        // (xberg-io/xberg#3352).
        assert_eq!(rule_orientation(&painted_bar(300.0)), Some(RuleOrientation::Horizontal));
    }

    #[test]
    fn a_text_shaped_fill_is_not_a_rule() {
        // Glyph outlines are filled paths too; only a rectangle or a closed
        // four-corner polygon counts.
        let glyph = PathContent::from_operations(vec![
            PathOperation::MoveTo(60.0, 300.0),
            PathOperation::LineTo(70.0, 300.0),
            PathOperation::CurveTo(72.0, 302.0, 72.0, 306.0, 70.0, 308.0),
            PathOperation::LineTo(60.0, 308.0),
        ])
        .with_fill(Color::black())
        .with_stroke_width(0.0);

        assert_eq!(rule_orientation(&glyph), None);
    }

    #[test]
    fn a_filled_cell_is_not_a_rule() {
        // A shaded header or highlighted cell is a filled rectangle as well; its
        // height exceeds the rule tolerance.
        let cell = PathContent::from_operations(vec![PathOperation::Rectangle(60.0, 300.0, RULE_WIDTH, 26.0)])
            .with_fill(Color::black())
            .with_stroke_width(0.0);
        assert_eq!(rule_orientation(&cell), None);
    }
}
