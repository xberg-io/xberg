use super::SNAP_TOL;
use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::layout::TextSpan;
use crate::structure::table_extractor::{Table, TableRow};
use std::collections::HashSet;

pub(crate) const MAX_SOURCE_SPANNED_COLUMNS: usize = 18;
const MIN_SOURCE_SPANNED_COLUMNS: usize = 16;
const MAX_SOURCE_SPANNED_PATHS: usize = 128;
const MAX_SOURCE_SPANNED_WORDS: usize = 1_024;
const MAX_SOURCE_SPANNED_TEXT_BYTES: usize = 64 * 1024;
const MAX_RULE_STROKE_WIDTH: f32 = 2.0;
const MAX_DARK_CHANNEL: f32 = 0.5;
const RULE_AXIS_TOLERANCE: f32 = 1.0;
const RULE_ENDPOINT_TOLERANCE: f32 = 1.0;

pub(crate) fn source_spanned_work_fits(spans: &[TextSpan], paths: &[PathContent]) -> bool {
    (MIN_SOURCE_SPANNED_COLUMNS + 5..=MAX_SOURCE_SPANNED_PATHS).contains(&paths.len())
        && spans.len() <= MAX_SOURCE_SPANNED_WORDS
        && spans
            .iter()
            .try_fold(0usize, |size, span| size.checked_add(span.text.len()))
            .is_some_and(|size| size <= MAX_SOURCE_SPANNED_TEXT_BYTES)
}

pub(crate) fn source_spanned_rules_present(paths: &[PathContent]) -> bool {
    paths
        .iter()
        .filter(|path| path.is_vertical_line(RULE_AXIS_TOLERANCE))
        .count()
        > MIN_SOURCE_SPANNED_COLUMNS
        && paths
            .iter()
            .filter(|path| path.is_horizontal_line(RULE_AXIS_TOLERANCE))
            .count()
            >= 4
}

pub(super) fn certify(table: &Table, spans: &[TextSpan], paths: &[PathContent]) -> Option<Rect> {
    let [upper, merged, lower] = table.rows.as_slice() else {
        return None;
    };
    let columns = table.col_count;
    let bounds = table.bbox?;
    if !(MIN_SOURCE_SPANNED_COLUMNS..=MAX_SOURCE_SPANNED_COLUMNS).contains(&columns)
        || !source_spanned_work_fits(spans, paths)
        || upper.cells.len() != columns
        || lower.cells.len() != columns
        || merged.cells.iter().filter(|cell| cell.colspan > 1).count() < 2
    {
        return None;
    }

    let tracks = &upper.cells.iter().map(|cell| cell.bbox).collect::<Option<Vec<_>>>()?;
    let upper_bounds = row_bounds(upper)?;
    let merged_bounds = row_bounds(merged)?;
    let lower_bounds = row_bounds(lower)?;
    if upper_bounds.top() != merged_bounds.bottom()
        || merged_bounds.top() != lower_bounds.bottom()
        || upper_bounds.union(&lower_bounds) != bounds
        || !row_matches_rules(upper, tracks, upper_bounds, paths, false)
        || !row_matches_rules(merged, tracks, merged_bounds, paths, true)
        || !row_matches_rules(lower, tracks, lower_bounds, paths, false)
        || !same_source_words(table, spans, bounds)
    {
        return None;
    }
    Some(bounds)
}

fn row_bounds(row: &TableRow) -> Option<Rect> {
    let first = row.cells.first()?.bbox?;
    Some(first.union(&row.cells.last()?.bbox?))
}

fn row_matches_rules(row: &TableRow, tracks: &[Rect], band: Rect, paths: &[PathContent], merged: bool) -> bool {
    let mut remaining = tracks;
    for cell in &row.cells {
        let width = cell.colspan as usize;
        let Some((owned, rest)) = remaining.split_at_checked(width) else {
            return false;
        };
        let [first, ..] = owned else { return false };
        let Some(last) = owned.last() else { return false };
        let Some(bounds) = cell.bbox else { return false };
        if cell.rowspan != 1
            || (!merged && width != 1)
            || bounds.left() != first.left()
            || bounds.right() != last.right()
            || bounds.top() != band.top()
            || bounds.bottom() != band.bottom()
            || !closed_cell(paths, bounds)
            || owned[1..]
                .iter()
                .any(|inner| painted_rule(paths, true, inner.left(), band.top(), band.bottom()))
        {
            return false;
        }
        remaining = rest;
    }
    remaining.is_empty()
}

fn closed_cell(paths: &[PathContent], bounds: Rect) -> bool {
    bounds.width > 2.0 * SNAP_TOL
        && bounds.height > 2.0 * SNAP_TOL
        && [bounds.top(), bounds.bottom()]
            .into_iter()
            .all(|y| painted_rule(paths, false, y, bounds.left(), bounds.right()))
        && [bounds.left(), bounds.right()]
            .into_iter()
            .all(|x| painted_rule(paths, true, x, bounds.top(), bounds.bottom()))
}

fn painted_rule(paths: &[PathContent], vertical: bool, coordinate: f32, start: f32, end: f32) -> bool {
    paths.iter().any(|path| {
        let bounds = path.bbox;
        let (minor, center, low, high) = if vertical {
            (bounds.width, bounds.center().x, bounds.top(), bounds.bottom())
        } else {
            (bounds.height, bounds.center().y, bounds.left(), bounds.right())
        };
        path.is_straight_line()
            && path.matrix.is_none()
            && path.layer.is_none()
            && path.dash_pattern.is_none()
            && (0.0..=MAX_RULE_STROKE_WIDTH).contains(&path.stroke_width)
            && path.stroke_color.is_some_and(|color| {
                let channels = [color.r, color.g, color.b];
                channels.iter().all(|channel| (0.0..=1.0).contains(channel))
                    && channels.iter().any(|channel| *channel <= MAX_DARK_CHANNEL)
            })
            && minor.abs() <= RULE_AXIS_TOLERANCE
            && (center - coordinate).abs() <= SNAP_TOL
            && low <= start + RULE_ENDPOINT_TOLERANCE
            && high >= end - RULE_ENDPOINT_TOLERANCE
    })
}

fn same_source_words(table: &Table, spans: &[TextSpan], bounds: Rect) -> bool {
    let mut claimed = HashSet::new();
    for cell in table.rows.iter().flat_map(|row| &row.cells) {
        let Some(bounds) = cell.bbox else { return false };
        if cell.text.trim().is_empty() || cell.spans.is_empty() {
            return false;
        }
        for span in &cell.spans {
            if span.text.trim().is_empty()
                || span.rotation_degrees != 0.0
                || span.wmode != 0
                || !bounds.contains_rect(&span.bbox)
                || !claimed.insert(span_identity(span))
            {
                return false;
            }
        }
    }
    spans
        .iter()
        .filter(|span| !span.text.trim().is_empty() && bounds.intersects(&span.bbox))
        .all(|span| claimed.remove(&span_identity(span)))
        && claimed.is_empty()
}

fn span_identity(span: &TextSpan) -> (usize, &str, [u32; 4]) {
    let bounds = span.bbox;
    (
        span.sequence,
        &span.text,
        [bounds.x, bounds.y, bounds.width, bounds.height].map(f32::to_bits),
    )
}

#[cfg(test)]
#[path = "source_spanned_rows_tests.rs"]
mod tests;
