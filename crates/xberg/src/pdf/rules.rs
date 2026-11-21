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

/// Horizontal rules that lie inside `bounds`, given in PDF user space
/// (`x0`, `y0` bottom-left to `x1`, `y1` top-right).
///
/// The borderless gate admits a candidate on drawn rules, so the rules must be
/// the candidate's own. Counting them page-wide admits every short region on a
/// page whose only real table is ruled — the failure that made a wrapped
/// two-column title read as a table on the SERFF page (xberg-io/xberg#3352).
pub(crate) fn count_horizontal_rules_in(
    paths: &[xberg_native_pdf::elements::PathContent],
    bounds: (f32, f32, f32, f32),
) -> usize {
    let (x0, y0, x1, y1) = bounds;
    paths
        .iter()
        .filter(|path| rule_orientation(path) == Some(RuleOrientation::Horizontal))
        .filter(|path| {
            let rule_y = path.bbox.y + path.bbox.height * 0.5;
            let rule_x0 = path.bbox.x;
            let rule_x1 = path.bbox.x + path.bbox.width;
            rule_y >= y0 && rule_y <= y1 && rule_x1 >= x0 && rule_x0 <= x1
        })
        .count()
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
    match path.operations.as_slice() {
        [PathOperation::Rectangle(..)] => true,
        [
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
        ] => true,
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    // Rule counting for the borderless-table admission gate.

    use super::{RuleOrientation, count_horizontal_rules_in, rule_orientation};
    use xberg_native_pdf::elements::PathContent;
    use xberg_native_pdf::elements::PathOperation;
    use xberg_native_pdf::geometry::Rect;
    use xberg_native_pdf::layout::Color;

    const PAGE_WIDTH: f32 = 595.0;
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

    fn rect(x: f32, y: f32, width: f32, height: f32) -> (f32, f32, f32, f32) {
        (x, y, x + width, y + height)
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
    fn a_filled_bar_counts_only_inside_its_own_region() {
        let paths = vec![painted_bar(300.0), painted_bar(500.0)];

        assert_eq!(
            count_horizontal_rules_in(&paths, rect(60.0, 280.0, RULE_WIDTH, 40.0)),
            1,
            "the bar inside the region belongs to it; the other one does not"
        );
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
