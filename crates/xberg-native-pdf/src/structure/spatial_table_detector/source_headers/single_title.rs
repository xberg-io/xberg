//! Preserve the leaf roles under one leading merged title in a fully ruled native table.

use super::{
    HEADER_RULE_TOLERANCE, contained_source_label, dark_fill, dark_solid_stroke, fill_only_bar, same_rule_position,
    visible_rule_styles,
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
            .all(|y| visible_rule_styles(paths, band.left(), band.right(), y, true).is_some())
        && divider_starts_at_leaf(paths, divider, upper, band)
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

fn row_band(cells: &[TableCell]) -> Option<Rect> {
    let first = cells.first()?.bbox.filter(|bounds| valid(*bounds))?;
    let mut band = first;
    for cell in &cells[1..] {
        let bounds = cell.bbox.filter(|bounds| valid(*bounds))?;
        if !same_rule_position(bounds.left(), band.right())
            || !same_rule_position(bounds.top(), first.top())
            || !same_rule_position(bounds.bottom(), first.bottom())
        {
            return None;
        }
        band = band.union(&bounds);
    }
    Some(band)
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
