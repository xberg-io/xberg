use super::{
    BAND_RULE_SPAN_TOL, IntersectionCell, SNAP_TOL, Table, TableDetectionConfig, TableRow, TextSpan,
    assign_spans_to_intersection_grid, finalize_intersection_tables, is_valid_table,
};
use crate::elements::{PathContent, PathOperation};
use crate::geometry::Rect;
use crate::layout::Color;

const MAX_PATHS: usize = 512;
const MAX_TRACKS: usize = 32;
const MAX_SPANS: usize = 4_096;
const MAX_SOURCE_BYTES: usize = 65_536;
const WIDE_GRID_COLUMNS: usize = 8;
const RULE_THICKNESS_LIMIT: f32 = 2.0;

fn painted_rectangle(path: &PathContent) -> Option<(Rect, Color)> {
    let [PathOperation::Rectangle(x, y, width, height)] = path.operations.as_slice() else {
        return None;
    };
    if path.stroke_color.is_some() || path.matrix.is_some() || path.layer.is_some() {
        return None;
    }
    let bounds = path.bbox;
    let drawn = Rect::new(*x, *y, *width, *height);
    if bounds != drawn
        || ![bounds.left(), bounds.right(), bounds.top(), bounds.bottom()]
            .iter()
            .all(|v| v.is_finite())
    {
        return None;
    }
    let color = path.fill_color?;
    (bounds.width > 0.
        && bounds.height > 0.
        && [color.r, color.g, color.b]
            .iter()
            .all(|value| (0.0..=1.0).contains(value)))
    .then_some((bounds, color))
}

fn cell_band(fills: &[(Rect, Color)], xs: &[f32], snapped: [f32; 2]) -> Option<(Rect, Color)> {
    let mut first: Option<(Rect, Color)> = None;
    for pair in xs.windows(2) {
        let mut matches = fills.iter().copied().filter(|(bbox, _)| {
            bbox.left() == pair[0]
                && bbox.right() == pair[1]
                && (bbox.top() - snapped[0]).abs() <= SNAP_TOL
                && (bbox.bottom() - snapped[1]).abs() <= SNAP_TOL
        });
        let (cell, color) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        if let Some((prior, prior_color)) = first {
            if cell.top() != prior.top() || cell.bottom() != prior.bottom() || color != prior_color {
                return None;
            }
        } else {
            first = Some((cell, color));
        }
    }
    first
}

fn has_joined_double_rule(fills: &[(Rect, Color)], xs: &[f32], snapped: f32, source: f32) -> bool {
    fills.iter().any(|(outer, color)| {
        outer.height < RULE_THICKNESS_LIMIT
            && outer.width > outer.height
            && outer.left() >= xs[0]
            && outer.right() <= xs[xs.len() - 1]
            && outer.center().y == snapped
            && outer.bottom() < source
            && fills.iter().any(|(inner, inner_color)| {
                inner.top() == source
                    && inner.left() == outer.left()
                    && inner.right() == outer.right()
                    && inner.height == outer.height
                    && inner_color == color
            })
    })
}

// A detached lower accounting bar can win horizontal snapping over the bar
// touching the paint edge. The paint must independently establish every
// source seam before the snapped grid can use those seams again. ~keep
fn exact_source_bands(lines: &[PathContent], xs: &[f32], ys: &[f32]) -> Option<Vec<f32>> {
    let fills: Vec<_> = lines.iter().filter_map(painted_rectangle).collect();
    let (low, mut previous_color) = cell_band(&fills, xs, [ys[0], ys[1]])?;
    if low.top() - ys[0] <= BAND_RULE_SPAN_TOL
        || (low.bottom() - ys[1]).abs() > BAND_RULE_SPAN_TOL
        || !has_joined_double_rule(&fills, xs, ys[0], low.top())
    {
        return None;
    }
    let mut bands = Vec::with_capacity(ys.len());
    bands.extend([low.top(), low.bottom()]);
    for pair in ys[1..].windows(2) {
        let (band, color) = cell_band(&fills, xs, [pair[0], pair[1]])?;
        if color == previous_color
            || bands.last() != Some(&band.top())
            || (band.bottom() - pair[1]).abs() > BAND_RULE_SPAN_TOL
        {
            return None;
        }
        previous_color = color;
        bands.push(band.bottom());
    }
    Some(bands)
}

fn owns_words(indices: &[Vec<Vec<usize>>], spans: &[TextSpan], xs: &[f32], bands: &[f32]) -> bool {
    let mut owned = 0;
    for (cells, band) in indices.iter().zip(bands.windows(2).rev()) {
        for (column, words) in cells.iter().enumerate() {
            let left = if column > 0 && cells[column - 1].is_empty() {
                xs[column - 1]
            } else {
                xs[column]
            };
            let right = if column + 1 < cells.len() && cells[column + 1].is_empty() {
                xs[column + 2]
            } else {
                xs[column + 1]
            };
            for &index in words {
                let span = &spans[index];
                let b = span.bbox;
                if !(b.left() >= left
                    && b.right() <= right
                    && b.top() >= band[0]
                    && b.bottom() <= band[1]
                    && b.width > 0.
                    && b.height > 0.
                    && span.rotation_degrees == 0.
                    && span.wmode == 0
                    && !span.text.trim().is_empty())
                {
                    return false;
                }
                owned += 1;
            }
        }
    }
    let intersecting = spans
        .iter()
        .filter(|span| {
            let b = span.bbox;
            b.right() > xs[0]
                && b.left() < xs[xs.len() - 1]
                && b.bottom() > bands[0]
                && b.top() < bands[bands.len() - 1]
                && !span.text.trim().is_empty()
        })
        .count();
    owned == intersecting
}

// Replace snapped intersections with the painted bounds. The preceding
// ownership proof allows horizontal overflow only into a discarded track. ~keep
fn restore_cell_bounds(rows: &mut [TableRow], xs: &[f32], bands: &[f32]) {
    for (output, band) in rows.iter_mut().zip(bands.windows(2).rev()) {
        for (column, cell) in output.cells.iter_mut().enumerate() {
            let paint = Rect::new(xs[column], band[0], xs[column + 1] - xs[column], band[1] - band[0]);
            cell.bbox = Some(cell.spans.iter().fold(paint, |bounds, span| bounds.union(&span.bbox)));
        }
    }
}

pub(super) fn recover(
    cells: &[IntersectionCell],
    xs: &[f32],
    ys: &[f32],
    spans: &[TextSpan],
    lines: &[PathContent],
    config: &TableDetectionConfig,
) -> Option<Table> {
    let tracks = xs.len().checked_sub(1)?;
    let row_count = ys.len().checked_sub(1)?;
    if row_count < 2
        || !(WIDE_GRID_COLUMNS..=MAX_TRACKS).contains(&tracks)
        || row_count > MAX_PATHS / tracks
        || cells.len() != tracks * row_count
        || spans.len() > MAX_SPANS
        || lines.len() > MAX_PATHS
        || spans
            .iter()
            .fold(0usize, |bytes, span| bytes.saturating_add(span.text.len()))
            > MAX_SOURCE_BYTES
    {
        return None;
    }
    let bands = exact_source_bands(lines, xs, ys)?;
    let (mut rows, mut indices) = assign_spans_to_intersection_grid(cells, xs, ys, tracks, spans, &[], false)?;
    if !owns_words(&indices, spans, xs, &bands) {
        return None;
    }
    restore_cell_bounds(&mut rows, xs, &bands);
    for column in (0..tracks).rev() {
        let occupied = indices.iter().filter(|row| !row[column].is_empty()).count();
        if occupied == 0 {
            for (row, ids) in rows.iter_mut().zip(&mut indices) {
                row.cells.remove(column);
                ids.remove(column);
            }
        } else if occupied != row_count {
            return None;
        }
    }
    let columns = rows[0].cells.len();
    if !(config.min_table_columns.max(3)..WIDE_GRID_COLUMNS).contains(&columns) || columns > config.max_table_columns {
        return None;
    }
    let tables = finalize_intersection_tables(rows, &indices, spans, config, columns, None);
    let [table] = tables.try_into().ok()?;
    if !is_valid_table(&table)
        || !table.is_real_grid()
        || crate::document::looks_like_prose_table(&table)
        || table.rows.len() != row_count
    {
        return None;
    }
    matches_owned_cells(&table, columns, indices, spans).then_some(table)
}

fn matches_owned_cells(table: &Table, columns: usize, indices: Vec<Vec<Vec<usize>>>, spans: &[TextSpan]) -> bool {
    let Some(table_bounds) = table.bbox else { return false };
    table.rows.iter().zip(indices).all(|(row, expected)| {
        row.cells.len() == columns
            && row.cells.windows(2).all(|pair| match (pair[0].bbox, pair[1].bbox) {
                (Some(left), Some(right)) => left.right() <= right.left(),
                _ => false,
            })
            && row.cells.iter().zip(expected).all(|(cell, ids)| {
                cell.spans.len() == ids.len()
                    && cell.bbox.is_some_and(|bounds| {
                        table_bounds.contains_rect(&bounds)
                            && cell.spans.iter().all(|span| bounds.contains_rect(&span.bbox))
                    })
                    && cell
                        .spans
                        .iter()
                        .zip(ids)
                        .all(|(word, id)| word.text == spans[id].text && word.bbox == spans[id].bbox)
                    && cell.text.chars().filter(|ch| !ch.is_whitespace()).eq(cell
                        .spans
                        .iter()
                        .flat_map(|span| span.text.chars())
                        .filter(|ch| !ch.is_whitespace()))
            })
    })
}

#[cfg(test)]
#[path = "painted_double_rule_grid_tests.rs"]
mod tests;
