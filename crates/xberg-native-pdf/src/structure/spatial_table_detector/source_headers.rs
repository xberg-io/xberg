use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::structure::table_extractor::{Table, TableCell, TableRow};

// Cell fills are often inset from their rules; thin strokes must not count as a background. ~keep
const MIN_SHADED_CELL_AREA_RATIO: f32 = 0.9;
// One point is a typical hairline rule: it is the snap distance for a rule's position and ends, the
// slack when matching group edges to leaf edges, and the proximity window around the header. ~keep
const HEADER_RULE_TOLERANCE: f32 = 1.0;
// A stroke lighter than this on any channel is indistinguishable from the white page it is drawn on. ~keep
const MAX_HEADER_RULE_CHANNEL: f32 = 0.9;
// Segments of one rule abut or overlap; a gap above a tenth of a point is a dot or a dash. ~keep
const RULE_GAP_TOLERANCE: f32 = 0.1;
// Bounds the work per grid on pages dense with vector art, where a header's neighbourhood holds
// hundreds of paths and no single rule can be told apart. ~keep
const MAX_GROUPED_HEADER_PATHS: usize = 128;

pub(super) fn shading_rectangles<'a>(paths: impl IntoIterator<Item = &'a PathContent>) -> Vec<Rect> {
    paths
        .into_iter()
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

type RuleStyle = ([u32; 3], u32);

pub(super) fn retain_grouped_leaf_header(rows: &mut [TableRow], paths: &[PathContent], columns: usize) {
    let [groups, leaves, first_body, ..] = rows else {
        return;
    };
    let Some(bounds) = header_bounds(groups, leaves, first_body) else {
        return;
    };
    let nearby: Vec<&PathContent> = paths.iter().filter(|path| near_header(path, bounds)).collect();
    if nearby.len() > MAX_GROUPED_HEADER_PATHS
        || [groups.is_header, leaves.is_header, first_body.is_header]
            .into_iter()
            .any(|header| header)
        || !complete_source_band(first_body, columns, false)
        || !complete_source_band(leaves, columns, true)
        || !(grouped_header_rules(groups, leaves, &nearby)
            || repeated_schema_over_shaded_body(groups, leaves, first_body, &nearby))
        || body_repeats_the_header_rule(leaves, first_body, &nearby)
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

fn grouped_header_rules(groups: &TableRow, leaves: &TableRow, paths: &[&PathContent]) -> bool {
    let Some(first) = leaves.cells.first().and_then(|cell| cell.bbox) else {
        return false;
    };
    let Some(last) = leaves.cells.last().and_then(|cell| cell.bbox) else {
        return false;
    };
    if !visible_rule(paths, first.left(), last.right(), first.y) {
        return false;
    }
    grouped_leaves(groups, leaves).is_some_and(|groups| {
        groups.iter().all(|(group, _)| {
            group
                .bbox
                .is_some_and(|bounds| visible_rule(paths, bounds.left(), bounds.right(), bounds.y))
        })
    })
}

fn grouped_leaves<'a>(groups: &'a TableRow, leaves: &'a TableRow) -> Option<Vec<(&'a TableCell, &'a [TableCell])>> {
    let mut remaining = leaves.cells.as_slice();
    let mut grouped = Vec::new();
    for (index, group) in groups.cells.iter().enumerate() {
        let (owned, rest) = remaining.split_at_checked(group.colspan as usize)?;
        let bounds = group.bbox?;
        let left = owned.first()?.bbox?;
        let right = owned.last()?.bbox?;
        if !same_rule_position(bounds.left(), left.left())
            || !same_rule_position(bounds.right(), right.right())
            || !same_rule_position(bounds.y, left.y + left.height)
        {
            return None;
        }
        if group.colspan >= 2 && source_label(group, true) {
            grouped.push((group, owned));
        } else if index != 0 || group.rowspan != 1 || !group.text.trim().is_empty() || !group.spans.is_empty() {
            return None;
        }
        remaining = rest;
    }
    (grouped.len() >= 2 && remaining.is_empty()).then_some(grouped)
}

/// A repeated column schema under separate groups and an independently shaded, regular body band
/// establish headers even when the producer paints its grid white. Font size alone is not evidence. ~keep
fn repeated_schema_over_shaded_body(
    groups: &TableRow,
    leaves: &TableRow,
    first_body: &TableRow,
    paths: &[&PathContent],
) -> bool {
    let Some(grouped) = grouped_leaves(groups, leaves) else {
        return false;
    };
    let schema: Vec<_> = grouped[0].1.iter().map(|cell| cell.text.trim()).collect();
    if !schema.iter().all(|label| label.chars().any(char::is_alphabetic))
        || !schema.iter().any(|label| *label != schema[0])
        || !grouped
            .iter()
            .all(|(_, cells)| cells.iter().map(|cell| cell.text.trim()).eq(schema.iter().copied()))
    {
        return false;
    }
    let possible_header_shading = shading_rectangles(paths.iter().copied());
    let body_shading = uncovered_shading(paths);
    groups.cells.iter().chain(&leaves.cells).all(|cell| {
        cell.bbox.is_some_and(|bounds| {
            possible_header_shading
                .iter()
                .all(|fill| fill.intersection(&bounds).is_none_or(|overlap| overlap.area() == 0.0))
        })
    }) && first_body.cells.iter().all(|cell| cell_is_shaded(cell, &body_shading))
        && first_body.cells[1..]
            .iter()
            .all(|cell| cell.text.chars().any(|character| character.is_ascii_digit()))
        && leaves.cells.iter().zip(&first_body.cells).all(|(leaf, body)| {
            matches!((leaf.bbox, body.bbox), (Some(leaf), Some(body))
                if same_rule_position(leaf.left(), body.left())
                    && same_rule_position(leaf.right(), body.right())
                    && same_rule_position(leaf.y, body.y + body.height))
        })
}

/// Reject a background touched by later rectangular paint, even if that paint is transparent:
/// PathContent does not carry fill opacity, so visibility cannot be certified in that case. ~keep
fn uncovered_shading(paths: &[&PathContent]) -> Vec<Rect> {
    shading_rectangles(paths.iter().enumerate().filter_map(|(index, path)| {
        if path.fill_color.is_none() || !path.is_rectangle() {
            return None;
        }
        let covered = paths[index + 1..].iter().any(|later| {
            later.fill_color.is_some()
                && later.is_rectangle()
                && later
                    .rendered_bbox()
                    .intersection(&path.rendered_bbox())
                    .is_some_and(|overlap| overlap.area() > 0.0)
        });
        (!covered).then_some(*path)
    }))
}

/// In a grid ruled alike on every row, a rule under the leaf band says nothing about the header.
/// The band is a header only when no rule of the same colour and weight also closes the first body
/// row. Tables that rule every row at one weight get no retained header from this path. ~keep
fn body_repeats_the_header_rule(leaves: &TableRow, first_body: &TableRow, paths: &[&PathContent]) -> bool {
    let (Some(first), Some(last)) = (
        leaves.cells.first().and_then(|c| c.bbox),
        leaves.cells.last().and_then(|c| c.bbox),
    ) else {
        return false;
    };
    let Some(body) = first_body.cells.first().and_then(|cell| cell.bbox) else {
        return false;
    };
    let header = visible_rule_styles(paths, first.left(), last.right(), first.y);
    let below = visible_rule_styles(paths, first.left(), last.right(), body.y);
    matches!((header, below), (Some(header), Some(below)) if header == below)
}

fn header_bounds(groups: &TableRow, leaves: &TableRow, first_body: &TableRow) -> Option<Rect> {
    groups
        .cells
        .iter()
        .chain(&leaves.cells)
        .chain(&first_body.cells)
        .filter_map(|cell| cell.bbox)
        .reduce(|all, bounds| all.union(&bounds))
}

fn near_header(path: &PathContent, bounds: Rect) -> bool {
    let drawn = path.rendered_bbox();
    drawn.right() >= bounds.left() - HEADER_RULE_TOLERANCE
        && drawn.left() <= bounds.right() + HEADER_RULE_TOLERANCE
        && drawn.bottom() >= bounds.top() - HEADER_RULE_TOLERANCE
        && drawn.top() <= bounds.bottom() + HEADER_RULE_TOLERANCE
}

/// Paint order decides visibility: a later dashed, light or filled rectangle covering the stroke band
/// hides the part of the rule it crosses, and a rule may be split into collinear segments. PathContent
/// omits zero-opacity strokes but carries no partial alpha or fill opacity (`ca`). Soft masks
/// (`SMask`) and blend modes (`BM`) are also unmodelled. Fills are treated as opaque, so a fill
/// crossing a rule can withhold a header even when that fill is transparent in the PDF. ~keep
fn visible_rule(paths: &[&PathContent], left: f32, right: f32, y: f32) -> bool {
    visible_rule_styles(paths, left, right, y).is_some()
}

/// The colours and weights of the segments that together make the rule visible, or `None`.
fn visible_rule_styles(paths: &[&PathContent], left: f32, right: f32, y: f32) -> Option<Vec<RuleStyle>> {
    let mut visible: Vec<(f32, f32, RuleStyle)> = Vec::new();
    for path in paths {
        let drawn = path.rendered_bbox();
        let (low, high) = (drawn.left(), drawn.right());
        let is_rule = path.is_horizontal_line(HEADER_RULE_TOLERANCE)
            && path.stroke_color.is_some()
            && same_rule_position(path.bbox.center().y, y);
        if is_rule && dark_solid_stroke(path) {
            visible.push((low, high, rule_style(path)));
        } else if is_rule || covering_fill(path, y) {
            visible = visible
                .into_iter()
                .flat_map(|(start, end, style)| {
                    let before = (start, end.min(low), style);
                    let after = (start.max(high), end, style);
                    [before, after].into_iter().filter(|(a, b, _)| a < b)
                })
                .collect();
        }
    }
    visible.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut covered: Option<f32> = None;
    let mut styles: Vec<RuleStyle> = Vec::new();
    for (low, high, style) in visible {
        let (reached, tolerance) = covered.map_or((left, HEADER_RULE_TOLERANCE), |end| (end, RULE_GAP_TOLERANCE));
        if low > reached + tolerance {
            return None;
        }
        styles.push(style);
        let end = covered.unwrap_or(left).max(high);
        covered = Some(end);
        if end >= right - HEADER_RULE_TOLERANCE {
            styles.sort_unstable();
            styles.dedup();
            return Some(styles);
        }
    }
    None
}

fn rule_style(path: &PathContent) -> RuleStyle {
    let channels = path.stroke_color.map_or([0; 3], |color| {
        [color.r.to_bits(), color.g.to_bits(), color.b.to_bits()]
    });
    (channels, path.stroke_width.to_bits())
}

fn covering_fill(path: &PathContent, y: f32) -> bool {
    path.fill_color.is_some()
        && path.is_rectangle()
        && path.bbox.top() <= y - HEADER_RULE_TOLERANCE / 2.0
        && path.bbox.bottom() >= y + HEADER_RULE_TOLERANCE / 2.0
}

/// Dash comes from `d` or ExtGState `/D`; zero-opacity strokes have no stroke colour. ~keep
fn dark_solid_stroke(path: &PathContent) -> bool {
    path.dash_pattern.is_none()
        && path.stroke_width >= 0.0
        && path.stroke_color.is_some_and(|color| {
            let channels = [color.r, color.g, color.b];
            channels.iter().all(|channel| (0.0..=1.0).contains(channel))
                && channels.iter().all(|channel| *channel <= MAX_HEADER_RULE_CHANNEL)
        })
}

fn same_rule_position(left: f32, right: f32) -> bool {
    (left - right).abs() <= HEADER_RULE_TOLERANCE
}

#[cfg(test)]
mod tests;
