use super::{IntersectionCell, SNAP_TOL, extract_cell_text, source_headers};
use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::layout::text_block::TextSpan;
use crate::structure::table_extractor::{TableCell, TableRow};

const HEADER_BANDS: usize = 2;
const RULE_AXIS_TOLERANCE: f32 = 2.0;

struct HeaderPlan {
    label: TableCell,
    indices: Vec<usize>,
}

fn same_bounds(left: Rect, right: Rect) -> bool {
    [
        left.left() - right.left(),
        left.right() - right.right(),
        left.top() - right.top(),
        left.bottom() - right.bottom(),
    ]
    .iter()
    .all(|difference| difference.abs() <= SNAP_TOL)
}

fn unique_source_cell(bounds: Rect, cells: &[IntersectionCell]) -> bool {
    cells
        .iter()
        .filter(|cell| {
            same_bounds(
                bounds,
                Rect::new(cell.x1, cell.y1, cell.x2 - cell.x1, cell.y2 - cell.y1),
            )
        })
        .count()
        == 1
}

fn drawn_interval(path: &PathContent, vertical: bool, coordinate: f32) -> Option<(f32, f32)> {
    if path.matrix.is_some() || (!path.has_stroke() && !path.has_fill()) {
        return None;
    }
    let bounds = path.rendered_bbox();
    let (near, far, low, high) = if vertical {
        (bounds.left(), bounds.right(), bounds.top(), bounds.bottom())
    } else {
        (bounds.top(), bounds.bottom(), bounds.left(), bounds.right())
    };
    let thin = if vertical {
        path.is_vertical_line(RULE_AXIS_TOLERANCE)
    } else {
        path.is_horizontal_line(RULE_AXIS_TOLERANCE)
    };
    let aligned = if thin {
        ((near + far) / 2. - coordinate).abs() <= SNAP_TOL
    } else {
        path.has_stroke()
            && path.is_rectangle()
            && ((near - coordinate).abs() <= SNAP_TOL || (far - coordinate).abs() <= SNAP_TOL)
    };
    (aligned && low.is_finite() && high.is_finite() && low <= high).then_some((low, high))
}

fn drawn_edge(paths: &[PathContent], vertical: bool, coordinate: f32, start: f32, end: f32) -> bool {
    let mut intervals: Vec<_> = paths
        .iter()
        .filter_map(|path| drawn_interval(path, vertical, coordinate))
        .collect();
    intervals.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut covered = start;
    for (low, high) in intervals {
        if high < covered - SNAP_TOL {
            continue;
        }
        if low > covered + SNAP_TOL {
            return false;
        }
        covered = covered.max(high);
        if covered >= end - SNAP_TOL {
            return true;
        }
    }
    false
}

fn independently_closed(bounds: Rect, paths: &[PathContent]) -> bool {
    bounds.width > 0.
        && bounds.height > 0.
        && [bounds.left(), bounds.right(), bounds.top(), bounds.bottom()]
            .iter()
            .all(|value| value.is_finite())
        && drawn_edge(paths, false, bounds.top(), bounds.left(), bounds.right())
        && drawn_edge(paths, false, bounds.bottom(), bounds.left(), bounds.right())
        && drawn_edge(paths, true, bounds.left(), bounds.top(), bounds.bottom())
        && drawn_edge(paths, true, bounds.right(), bounds.top(), bounds.bottom())
}

fn eligible_shape(rows: &[TableRow], indices: &[Vec<Vec<usize>>]) -> Option<Rect> {
    if rows.len() < HEADER_BANDS + 2 || indices.len() != rows.len() {
        return None;
    }
    let columns = rows.first()?.cells.len();
    if columns < 2
        || rows.iter().zip(indices).any(|(row, indices)| {
            row.is_header
                || row.cells.len() != columns
                || indices.len() != columns
                || row
                    .cells
                    .iter()
                    .any(|cell| cell.is_header || cell.rowspan != 1 || cell.colspan != 1)
        })
    {
        return None;
    }
    let upper = rows[0].cells[0].bbox?;
    let lower = rows[1].cells[0].bbox?;
    ((upper.top() - lower.bottom()).abs() <= SNAP_TOL
        && (upper.left() - lower.left()).abs() <= SNAP_TOL
        && (upper.right() - lower.right()).abs() <= SNAP_TOL)
        .then(|| upper.union(&lower))
}

fn normal_body(rows: &[TableRow], shading: &[Rect]) -> bool {
    rows.iter().all(|row| {
        row.cells.iter().filter(|cell| !cell.text.trim().is_empty()).count() >= 2
            && row.cells.iter().all(|cell| {
                !source_headers::cell_is_shaded(cell, shading)
                    && cell
                        .spans
                        .iter()
                        .all(|span| !span.font_weight.is_bold() && span.rotation_degrees == 0. && span.wmode == 0)
            })
    })
}

fn proven_header_cell(cell: &TableCell, cells: &[IntersectionCell], paths: &[PathContent], shading: &[Rect]) -> bool {
    let Some(bounds) = cell.bbox else {
        return false;
    };
    !cell.spans.is_empty()
        && cell.text.chars().any(char::is_alphabetic)
        && cell.spans.iter().all(|span| {
            span.font_weight.is_bold()
                && span.rotation_degrees == 0.
                && span.wmode == 0
                && bounds.contains_point(&span.bbox.center())
        })
        && source_headers::cell_is_shaded(cell, shading)
        && unique_source_cell(bounds, cells)
        && independently_closed(bounds, paths)
}

fn label_plan(bounds: Rect, indices: &[Vec<Vec<usize>>], spans: &[TextSpan], tolerance: f32) -> Option<HeaderPlan> {
    if !tolerance.is_finite() || tolerance < 0. {
        return None;
    }
    let indices: Vec<_> = indices[..HEADER_BANDS]
        .iter()
        .flat_map(|row| row[0].iter().copied())
        .collect();
    if indices.is_empty() {
        return None;
    }
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    let mut source = Vec::new();
    for &index in &indices {
        let span = spans.get(index)?;
        let y = span.bbox.center().y;
        if !y.is_finite() {
            return None;
        }
        low = low.min(y);
        high = high.max(y);
        source.push(span.clone());
    }
    if high - low > tolerance {
        return None;
    }
    let label = TableCell {
        text: extract_cell_text(&indices, spans),
        spans: source,
        colspan: 1,
        rowspan: 2,
        mcids: indices.iter().filter_map(|&index| spans[index].mcid).collect(),
        bbox: Some(bounds),
        is_header: true,
    };
    Some(HeaderPlan { label, indices })
}

fn certify(
    rows: &[TableRow],
    indices: &[Vec<Vec<usize>>],
    cells: &[IntersectionCell],
    spans: &[TextSpan],
    paths: &[PathContent],
    tolerance: f32,
) -> Option<HeaderPlan> {
    let bounds = eligible_shape(rows, indices)?;
    let shading = source_headers::shading_rectangles(paths);
    if !normal_body(&rows[HEADER_BANDS..], &shading) {
        return None;
    }
    let plan = label_plan(bounds, indices, spans, tolerance)?;
    if !proven_header_cell(&plan.label, cells, paths, &shading) {
        return None;
    }
    let mut seen = std::collections::HashSet::new();
    for index in &plan.indices {
        if !seen.insert(*index) {
            return None;
        }
    }
    for row in 0..HEADER_BANDS {
        for column in 1..rows[row].cells.len() {
            if !proven_header_cell(&rows[row].cells[column], cells, paths, &shading)
                || indices[row][column].iter().any(|index| !seen.insert(*index))
            {
                return None;
            }
        }
    }
    Some(plan)
}

/// Preserve exactly two independently drawn, bold shaded header bands and one single-line
/// label whose source cell closes across both. Multiline label/value cells remain unchanged. ~keep
pub(super) fn retain_drawn_header_bands(
    rows: &mut [TableRow],
    indices: &mut [Vec<Vec<usize>>],
    cells: &[IntersectionCell],
    spans: &[TextSpan],
    paths: &[PathContent],
    tolerance: f32,
) {
    let Some(plan) = certify(rows, indices, cells, spans, paths, tolerance) else {
        return;
    };
    rows[0].cells[0] = plan.label;
    indices[0][0] = plan.indices;
    rows[1].cells.remove(0);
    indices[1].remove(0);
    for row in &mut rows[..HEADER_BANDS] {
        row.is_header = true;
        for cell in &mut row.cells {
            cell.is_header = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segmented_outline(gap: f32) -> Vec<PathContent> {
        vec![
            PathContent::line(0., 0., 80., 0.),
            PathContent::line(0., 40., 80., 40.),
            PathContent::line(80., 0., 80., 40.),
            PathContent::line(0., 0., 0., 20. - gap / 2.),
            PathContent::line(0., 20. + gap / 2., 0., 40.),
        ]
    }

    #[test]
    fn adjacent_drawn_segments_close_one_header_cell() {
        assert!(independently_closed(
            Rect::new(0., 0., 80., 40.),
            &segmented_outline(0.48)
        ));
    }

    #[test]
    fn missing_rule_segments_do_not_close_a_header_cell() {
        assert!(!independently_closed(
            Rect::new(0., 0., 80., 40.),
            &segmented_outline(12.)
        ));
    }

    #[test]
    fn paintless_boundary_paths_do_not_close_a_header_cell() {
        let mut paths = vec![
            PathContent::line(0., 0., 80., 0.),
            PathContent::line(0., 40., 80., 40.),
            PathContent::line(0., 0., 0., 40.),
            PathContent::line(80., 0., 80., 40.),
        ];
        for path in &mut paths {
            path.stroke_color = None;
            path.fill_color = None;
        }
        assert!(!independently_closed(Rect::new(0., 0., 80., 40.), &paths));
    }

    #[test]
    fn filled_thin_bars_close_a_header_cell_without_strokes() {
        let mut paths = vec![
            PathContent::rect(0., 0., 80., 0.48),
            PathContent::rect(0., 39.52, 80., 0.48),
            PathContent::rect(0., 0., 0.48, 40.),
            PathContent::rect(79.52, 0., 0.48, 40.),
        ];
        for path in &mut paths {
            path.stroke_color = None;
            path.fill_color = Some(crate::layout::Color::new(0., 0., 0.));
        }
        assert!(independently_closed(Rect::new(0., 0., 80., 40.), &paths));
    }
}
