use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::structure::table_extractor::{Table, TableCell, TableRow};

// Cell fills are often inset from their rules; thin strokes must not count as a background. ~keep
const MIN_SHADED_CELL_AREA_RATIO: f32 = 0.9;
const HEADER_RULE_TOLERANCE: f32 = 1.0;
const MAX_HEADER_RULE_CHANNEL: f32 = 0.9;

pub(super) fn shading_rectangles(paths: &[PathContent]) -> Vec<Rect> {
    paths
        .iter()
        .filter(|path| {
            path.is_rectangle()
                && path.fill_color.is_some_and(|color| {
                    let channels = [color.r, color.g, color.b];
                    channels
                        .iter()
                        .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
                        && channels.iter().any(|channel| *channel < 1.0)
                })
        })
        .map(PathContent::rendered_bbox)
        .collect()
}

pub(super) fn cell_is_shaded(cell: &TableCell, shading: &[Rect]) -> bool {
    cell.bbox.is_some_and(|cell| {
        cell.area().is_finite()
            && cell.area() > 0.0
            && shading.iter().any(|fill| {
                fill.intersection(&cell)
                    .is_some_and(|overlap| overlap.area() >= cell.area() * MIN_SHADED_CELL_AREA_RATIO)
            })
    })
}

fn complete_plain_cells(table: &Table) -> bool {
    table.col_count >= 2
        && table.rows.len() >= 3
        && table.rows.iter().all(|row| {
            !row.is_header
                && row.cells.len() == table.col_count
                && row.cells.iter().all(|cell| {
                    !cell.is_header
                        && cell.rowspan == 1
                        && cell.colspan == 1
                        && !cell.text.trim().is_empty()
                        && !cell.spans.is_empty()
                        && cell
                            .spans
                            .iter()
                            .all(|span| span.rotation_degrees == 0.0 && span.wmode == 0)
                })
        })
}

/// Infer one leading header only when text styling and independently painted cell backgrounds
/// agree. Numeric first rows and styled body tiers do not establish header roles. ~keep
pub(super) fn infer_complete_leading_header(table: &mut Table, shading: &[Rect]) {
    if table.has_header || !complete_plain_cells(table) {
        return;
    }
    let first = &table.rows[0];
    let header_evidence = first.cells.iter().all(|cell| {
        cell.text.chars().any(char::is_alphabetic)
            && cell.spans.iter().all(|span| span.font_weight.is_bold())
            && cell_is_shaded(cell, shading)
    });
    let body_contrasts = table.rows[1..]
        .iter()
        .flat_map(|row| &row.cells)
        .all(|cell| cell.spans.iter().all(|span| !span.font_weight.is_bold()) && !cell_is_shaded(cell, shading));
    if !header_evidence || !body_contrasts {
        return;
    }
    table.has_header = true;
    table.rows[0].is_header = true;
    for cell in &mut table.rows[0].cells {
        cell.is_header = true;
    }
}

pub(super) fn retain_grouped_leaf_header(rows: &mut [TableRow], paths: &[PathContent], columns: usize) {
    let [groups, leaves, first_body, ..] = rows else {
        return;
    };
    if [groups.is_header, leaves.is_header, first_body.is_header]
        .into_iter()
        .any(|header| header)
        || !complete_source_band(first_body, columns, false)
        || !complete_source_band(leaves, columns, true)
        || !grouped_header_rules(groups, leaves, paths)
    {
        return;
    }
    for row in [groups, leaves] {
        row.is_header = true;
        for cell in &mut row.cells {
            cell.is_header = true;
        }
    }
}

fn complete_source_band(row: &TableRow, columns: usize, bold: bool) -> bool {
    row.cells.len() == columns
        && row
            .cells
            .iter()
            .all(|cell| cell.colspan == 1 && source_label(cell, bold))
}

fn source_label(cell: &TableCell, bold: bool) -> bool {
    cell.bbox.is_some_and(|bounds| {
        !cell.is_header
            && cell.rowspan == 1
            && !cell.text.trim().is_empty()
            && !cell.spans.is_empty()
            && cell.spans.iter().all(|span| {
                span.rotation_degrees == 0.0
                    && span.wmode == 0
                    && span.font_weight.is_bold() == bold
                    && bounds.contains_rect(&span.bbox)
            })
    })
}

fn grouped_header_rules(groups: &TableRow, leaves: &TableRow, paths: &[PathContent]) -> bool {
    let Some(first) = leaves.cells.first().and_then(|cell| cell.bbox) else {
        return false;
    };
    let Some(last) = leaves.cells.last().and_then(|cell| cell.bbox) else {
        return false;
    };
    if !visible_rule(paths, first.left(), last.right(), first.y) {
        return false;
    }

    let mut remaining = leaves.cells.as_slice();
    let mut grouped = 0;
    for (index, group) in groups.cells.iter().enumerate() {
        let Some((owned, rest)) = remaining.split_at_checked(group.colspan as usize) else {
            return false;
        };
        let Some(bounds) = group.bbox else { return false };
        let [first, ..] = owned else { return false };
        let (Some(left), Some(right)) = (first.bbox, owned.last().and_then(|leaf| leaf.bbox)) else {
            return false;
        };
        if !same_rule_position(bounds.left(), left.left())
            || !same_rule_position(bounds.right(), right.right())
            || !same_rule_position(bounds.y, left.y + left.height)
        {
            return false;
        }
        if group.colspan >= 2
            && source_label(group, true)
            && visible_rule(paths, bounds.left(), bounds.right(), bounds.y)
        {
            grouped += 1;
        } else if index != 0
            || group.colspan != 1
            || group.rowspan != 1
            || !group.text.trim().is_empty()
            || !group.spans.is_empty()
        {
            return false;
        }
        remaining = rest;
    }
    grouped >= 2 && remaining.is_empty()
}

fn visible_rule(paths: &[PathContent], left: f32, right: f32, y: f32) -> bool {
    paths
        .iter()
        .rev()
        .find(|path| {
            path.is_horizontal_line(HEADER_RULE_TOLERANCE)
                && path.stroke_color.is_some()
                && same_rule_position(path.bbox.left(), left)
                && same_rule_position(path.bbox.right(), right)
                && same_rule_position(path.bbox.center().y, y)
        })
        .is_some_and(|path| {
            path.dash_pattern.is_none()
                && path.stroke_width.is_finite()
                && path.stroke_width > 0.0
                && path.stroke_color.is_some_and(|color| {
                    let channels = [color.r, color.g, color.b];
                    channels.iter().all(|channel| (0.0..=1.0).contains(channel))
                        && channels.iter().any(|channel| *channel <= MAX_HEADER_RULE_CHANNEL)
                })
        })
}

fn same_rule_position(left: f32, right: f32) -> bool {
    (left - right).abs() <= HEADER_RULE_TOLERANCE
}
