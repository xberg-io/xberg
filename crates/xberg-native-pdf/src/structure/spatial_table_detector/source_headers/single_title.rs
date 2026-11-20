//! Preserve the leaf roles under one leading merged title in a fully ruled native table.

use super::{
    HEADER_RULE_TOLERANCE, band_rule, contained_source_label, dark_fill, dark_solid_stroke, fill_only_bar,
    same_rule_position,
};
use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::structure::table_extractor::{TableCell, TableRow};

// Native font metric boxes can extend just above the painted row in PDF coordinates. ~keep
const MAX_UPPER_FONT_METRIC_OVERHANG: f32 = 0.5;

pub(super) fn has_leaf_header(
    title_row: &TableRow,
    leaves: &TableRow,
    first_body: &TableRow,
    paths: &[&PathContent],
    columns: usize,
) -> bool {
    let Some((title, peers)) = title_row.cells.split_first() else {
        return false;
    };
    if columns < 3
        || title.colspan != 2
        || peers.len() != columns - 2
        || !contained_source_label(title, true, MAX_UPPER_FONT_METRIC_OVERHANG)
        || !alphabetic_label(title)
        || leaves.cells.len() != columns
        || !leaves.cells.iter().all(|cell| {
            cell.colspan == 1
                && !cell.is_header
                && contained_source_label(cell, true, MAX_UPPER_FONT_METRIC_OVERHANG)
                && alphabetic_label(cell)
        })
        || first_body.cells.len() != columns
        || !first_body.cells.iter().all(|cell| {
            cell.colspan == 1 && !cell.is_header && contained_source_label(cell, false, MAX_UPPER_FONT_METRIC_OVERHANG)
        })
        || first_body
            .cells
            .iter()
            .filter(|cell| cell.text.trim().parse::<f64>().is_ok_and(f64::is_finite))
            .take(2)
            .count()
            < 2
    {
        return false;
    }
    let Some((upper, band)) = aligned_bands(title, peers, leaves, first_body) else {
        return false;
    };
    let Some(first_leaf) = leaves.cells.first().and_then(|cell| cell.bbox) else {
        return false;
    };
    let divider = first_leaf.right();
    title_ink_crosses_divider(title, divider)
        && [band.top(), band.bottom()]
            .into_iter()
            .all(|y| band_rule(paths, band.left(), band.right(), y))
        && divider_starts_at_leaf(paths, divider, upper, band)
}

/// Preserve the leaf band under one spanning title when the source draws the title as a single
/// merged cell above it, with an empty corner cell over the first column. The title's column
/// extent is source geometry, not a guess, so the leaf band under it names those columns
/// (xberg-io/xberg#3347). ~keep
pub(super) fn has_stacked_leaf_header(
    groups: &TableRow,
    leaves: &TableRow,
    body: &[TableRow],
    paths: &[&PathContent],
    columns: usize,
) -> bool {
    if columns < 3 || body.is_empty() || leaves.is_header || groups.is_header {
        return false;
    }
    let Some((corner, title)) = stacked_title(groups, columns) else {
        return false;
    };
    let Some(tracks) = leaf_tracks(leaves, columns) else {
        return false;
    };
    let Some(band) = tracks.iter().copied().reduce(|all, bounds| all.union(&bounds)) else {
        return false;
    };
    title_covers_leaves(corner, title, &tracks, band)
        && stacked_body_is_data(body, columns)
        && body_band_aligns(body, leaves, band)
        && [band.top(), band.bottom()]
            .into_iter()
            .all(|y| band_rule(paths, band.left(), band.right(), y))
}

/// An empty corner cell over column 0 and one bold title cell over every other column, with both
/// cells sharing one drawn band. Returns their bounds.
fn stacked_title(groups: &TableRow, columns: usize) -> Option<(Rect, Rect)> {
    let [corner, title] = groups.cells.as_slice() else {
        return None;
    };
    let corner_bounds = corner.bbox.filter(|bounds| valid(*bounds))?;
    let title_bounds = title.bbox?;
    let empty_corner =
        corner.colspan == 1 && corner.rowspan == 1 && corner.text.trim().is_empty() && corner.spans.is_empty();
    let spanning_title = title.colspan as usize + 1 == columns
        && title.rowspan == 1
        && contained_source_label(title, true, MAX_UPPER_FONT_METRIC_OVERHANG)
        && alphabetic_label(title);
    (empty_corner && spanning_title).then_some((corner_bounds, title_bounds))
}

/// The leaf band's column tracks, each contiguous with the last one.
fn leaf_tracks(leaves: &TableRow, columns: usize) -> Option<Vec<Rect>> {
    if leaves.cells.len() != columns
        || !leaves.cells.iter().all(|cell| {
            cell.colspan == 1
                && cell.rowspan == 1
                && !cell.is_header
                && contained_source_label(cell, true, MAX_UPPER_FONT_METRIC_OVERHANG)
        })
        || !alphabetic_label(&leaves.cells[0])
    {
        return None;
    }
    row_tracks(&leaves.cells)
}

/// Whether the title occupies exactly the leaf columns after the corner and closes on the leaf
/// band's upper edge.
fn title_covers_leaves(corner: Rect, title: Rect, tracks: &[Rect], band: Rect) -> bool {
    same_rule_position(title.left(), tracks[1].left())
        && same_rule_position(title.right(), tracks[tracks.len() - 1].right())
        && same_track(corner, tracks[0])
        && same_rule_position(corner.top(), title.top())
        && same_rule_position(corner.bottom(), title.bottom())
        && same_rule_position(title.top(), band.bottom())
}

/// A body band under the leaves: complete rows of contained cells, of which at least one of the
/// first two holds two values and a regular-weight cell. A body's first column may be a bold
/// row-label track, so only the band as a whole has to contrast with the bold header. ~keep
fn stacked_body_is_data(body: &[TableRow], columns: usize) -> bool {
    let contained = |cell: &TableCell| {
        contained_source_label(cell, true, MAX_UPPER_FONT_METRIC_OVERHANG)
            || contained_source_label(cell, false, MAX_UPPER_FONT_METRIC_OVERHANG)
    };
    body.iter().all(|row| {
        row.cells.len() == columns
            && !row.is_header
            && row.cells.iter().all(|cell| {
                cell.colspan == 1
                    && cell.rowspan == 1
                    && !cell.is_header
                    && (cell.text.trim().is_empty() || contained(cell))
            })
    }) && body.iter().take(2).any(|row| {
        row.cells
            .iter()
            .filter(|cell| super::super::is_data_value(cell.text.trim()))
            .count()
            >= 2
            && row
                .cells
                .iter()
                .any(|cell| !cell.text.trim().is_empty() && cell.spans.iter().all(|span| !span.font_weight.is_bold()))
    })
}

/// The first body band closes on the leaf band and repeats its column tracks.
fn body_band_aligns(body: &[TableRow], leaves: &TableRow, band: Rect) -> bool {
    let Some(body_band) = body[0]
        .cells
        .iter()
        .filter_map(|cell| cell.bbox)
        .reduce(|all, bounds| all.union(&bounds))
    else {
        return false;
    };
    same_rule_position(body_band.bottom(), band.top())
        && body[0].cells.iter().zip(&leaves.cells).all(
            |(value, leaf)| matches!((leaf.bbox, value.bbox), (Some(leaf), Some(value)) if same_track(leaf, value)),
        )
}

fn title_ink_crosses_divider(title: &TableCell, divider: f32) -> bool {
    title
        .spans
        .iter()
        .map(|span| span.bbox)
        .reduce(|ink, bounds| ink.union(&bounds))
        .is_some_and(|ink| {
            ink.left() < divider - HEADER_RULE_TOLERANCE && ink.right() > divider + HEADER_RULE_TOLERANCE
        })
}

fn alphabetic_label(cell: &TableCell) -> bool {
    cell.text.chars().any(char::is_alphabetic)
}

fn aligned_bands(title: &TableCell, peers: &[TableCell], leaves: &TableRow, body: &TableRow) -> Option<(Rect, Rect)> {
    let upper = title.bbox.filter(|bounds| valid(*bounds))?;
    let band = row_band(&leaves.cells)?;
    if !same_rule_position(upper.top(), band.bottom()) {
        return None;
    }
    let [first, second, remaining @ ..] = leaves.cells.as_slice() else {
        return None;
    };
    let title_track = first.bbox?.union(&second.bbox?);
    if !same_track(upper, title_track) {
        return None;
    }
    for (peer, leaf) in peers.iter().zip(remaining) {
        let bounds = peer.bbox.filter(|bounds| valid(*bounds))?;
        if peer.colspan != 1
            || peer.rowspan != 1
            || !peer.text.trim().is_empty()
            || !peer.spans.is_empty()
            || !same_track(bounds, leaf.bbox?)
            || !same_rule_position(bounds.top(), upper.top())
            || !same_rule_position(bounds.bottom(), upper.bottom())
        {
            return None;
        }
    }
    let body_band = row_band(&body.cells)?;
    (same_rule_position(body_band.bottom(), band.top())
        && leaves.cells.iter().zip(&body.cells).all(
            |(leaf, value)| matches!((leaf.bbox, value.bbox), (Some(leaf), Some(value)) if same_track(leaf, value)),
        ))
    .then_some((upper, band))
}

/// The band's cells as contiguous column tracks, left to right.
fn row_tracks(cells: &[TableCell]) -> Option<Vec<Rect>> {
    let mut tracks: Vec<Rect> = Vec::with_capacity(cells.len());
    for cell in cells {
        let bounds = cell.bbox.filter(|bounds| valid(*bounds))?;
        if let Some(previous) = tracks.last()
            && (!same_rule_position(bounds.left(), previous.right())
                || !same_rule_position(bounds.top(), previous.top())
                || !same_rule_position(bounds.bottom(), previous.bottom()))
        {
            return None;
        }
        tracks.push(bounds);
    }
    (!tracks.is_empty()).then_some(tracks)
}

fn row_band(cells: &[TableCell]) -> Option<Rect> {
    row_tracks(cells)?.into_iter().reduce(|all, bounds| all.union(&bounds))
}

fn valid(bounds: Rect) -> bool {
    bounds.width > 2.0 * HEADER_RULE_TOLERANCE
        && bounds.height > 2.0 * HEADER_RULE_TOLERANCE
        && [bounds.left(), bounds.right(), bounds.top(), bounds.bottom()]
            .into_iter()
            .all(f32::is_finite)
}

fn same_track(left: Rect, right: Rect) -> bool {
    same_rule_position(left.left(), right.left()) && same_rule_position(left.right(), right.right())
}

fn divider_starts_at_leaf(paths: &[&PathContent], x: f32, title: Rect, leaves: Rect) -> bool {
    let slack = HEADER_RULE_TOLERANCE;
    let mut found = false;
    for path in paths {
        let bounds = path.rendered_bbox();
        let aligned_vertical = path.is_vertical_line(slack) && same_rule_position(path.bbox.center().x, x);
        if aligned_vertical {
            if bounds.bottom() > title.top() + slack && bounds.top() < title.bottom() - slack {
                return false;
            }
            let witness = if dark_solid_stroke(path) {
                Some(bounds)
            } else if fill_only_bar(path) && dark_fill(path) {
                Some(path.bbox)
            } else {
                None
            };
            if witness.is_some_and(|rule| {
                same_rule_position(rule.bottom(), leaves.bottom()) && rule.top() <= leaves.top() + slack
            }) {
                found = true;
                continue;
            }
        }
        let covers_divider = (aligned_vertical && path.stroke_color.is_some())
            || (path.has_fill() && path.is_rectangle() && bounds.left() <= x && bounds.right() >= x);
        if covers_divider && bounds.top() < leaves.bottom() - slack && bounds.bottom() > leaves.top() + slack {
            // Later pale or dashed strokes and fills can hide the divider. ~keep
            found = false;
        }
    }
    found
}
