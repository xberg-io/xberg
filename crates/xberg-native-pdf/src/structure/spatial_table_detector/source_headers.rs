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

mod single_title;

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
pub(super) fn infer_complete_leading_header(table: &mut Table, paths: &[PathContent]) {
    if table.has_header || !complete_plain_cells(table) {
        return;
    }
    let Some(bounds) = table.bbox else {
        return;
    };
    let nearby: Vec<_> = paths
        .iter()
        .filter(|path| path.fill_color.is_some() && path.is_rectangle() && near_header(path, bounds))
        .take(MAX_GROUPED_HEADER_PATHS + 1)
        .collect();
    if nearby.len() > MAX_GROUPED_HEADER_PATHS {
        return;
    }
    let possible_shading = shading_rectangles(nearby.iter().copied());
    let first = &table.rows[0];
    let header_evidence = first.cells.iter().all(|cell| {
        cell.text.chars().any(char::is_alphabetic)
            && cell.spans.iter().all(|span| span.font_weight.is_bold())
            && cell_has_visible_shading(cell, &nearby)
    });
    let body_contrasts = table.rows[1..].iter().flat_map(|row| &row.cells).all(|cell| {
        cell.spans.iter().all(|span| !span.font_weight.is_bold()) && !cell_is_shaded(cell, &possible_shading)
    });
    if !header_evidence || !body_contrasts {
        return;
    }
    table.has_header = true;
    table.rows[0].is_header = true;
    for cell in &mut table.rows[0].cells {
        cell.is_header = true;
    }
}

/// Minimum distinct text baselines a leading band needs before its cells can read as wrapped.
const MIN_WRAPPED_HEADER_LINES: usize = 2;

/// Minimum populated cells a wrapped band and a body band need for either to prove the other.
const MIN_WRAPPED_HEADER_CELLS: usize = 2;

/// Thickest paint that still reads as a rule bar rather than a filled cell. This is a paint
/// property: `row_tolerance` clusters text baselines and says nothing about how a producer styles
/// a rule, so the two are deliberately unrelated. ~keep
const MAX_RULE_BAR_THICKNESS: f32 = 2.0 * HEADER_RULE_TOLERANCE;

/// Keep a leading ruled band whose cells wrap over several lines as ONE header row.
///
/// A header band is often the only band in a table with no rule between its lines: the labels of
/// adjacent columns wrap independently, so their baselines interleave while every body band holds
/// a single line. `split_rows_by_text_positions` reads those baselines as separate rows and the
/// header then loses its role. This fires only when the source says all of that: the table is
/// ruled, its first non-numbering band is drawn as one row, every band below is single-line, no
/// line of the band is a data value, and the band below it holds values (xberg-io/xberg#3346).
/// The strip that removes form-numbering bands runs later, so a leading run of blank or lone-digit
/// bands is skipped here instead of being mistaken for the header.
///
/// No font-weight or size contrast test can stand in for those checks: the #3346 producer sets the
/// wrapped labels in the same regular 11 pt face as its body values, so the two bands are
/// indistinguishable by weight and a contrast test would reject the very record this targets. The
/// discriminating evidence is structural instead: the band is the only one carrying several
/// baselines, it is one drawn row, and every band below it is single-line over a numeric body. ~keep
pub(super) fn retain_wrapped_leading_header(rows: &mut [TableRow], paths: &[PathContent], tolerance: f32) {
    if rows.len() < 2 || !tolerance.is_finite() || tolerance < 0.0 {
        return;
    }
    let Some(lead) = rows
        .iter()
        .position(|row| !form_numbering_band(row) && row.cells.iter().any(|cell| !cell.text.trim().is_empty()))
    else {
        return;
    };
    let (Some(header), Some(body)) = (rows.get(lead), rows.get(lead + 1)) else {
        return;
    };
    if header.is_header
        || header.cells.iter().any(|cell| cell.colspan != 1 || cell.rowspan != 1)
        || band_line_count(header, tolerance) < MIN_WRAPPED_HEADER_LINES
        || rows[lead + 1..].iter().any(|row| band_line_count(row, tolerance) > 1)
    {
        return;
    }
    let populated: Vec<&TableCell> = header
        .cells
        .iter()
        .filter(|cell| !cell.text.trim().is_empty())
        .collect();
    if populated.len() < MIN_WRAPPED_HEADER_CELLS
        || populated.iter().any(|cell| {
            !cell.text.chars().any(char::is_alphabetic)
                || cell.text.split('\n').any(|line| super::is_data_value(line.trim()))
        })
        || body
            .cells
            .iter()
            .filter(|cell| super::is_data_value(cell.text.trim()))
            .count()
            < MIN_WRAPPED_HEADER_CELLS
    {
        return;
    }
    let Some(bounds) = populated
        .iter()
        .filter_map(|cell| cell.bbox)
        .reduce(|all, cell| all.union(&cell))
    else {
        return;
    };
    if !one_drawn_band(bounds, paths) {
        return;
    }
    let header = &mut rows[lead];
    header.is_header = true;
    for cell in &mut header.cells {
        cell.is_header = true;
        cell.text = single_line(&cell.text);
    }
}

/// The detector joins a cell's baselines with `\n`. A header band kept whole stays exempt from
/// `split_rows_by_text_positions`, and `Table::render_text` prints `cell.text` verbatim, so the
/// embedded breaks would otherwise reach plain text as one row ending mid-cell (xberg-io/xberg#3346).
/// Collapse them the way the reading-order join does: one space between the wrapped lines.
fn single_line(text: &str) -> String {
    text.split('\n')
        .flat_map(|line| line.split_whitespace())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A band `strip_form_numbering_artifacts` removes: every cell empty or one digit 1-9, with at
/// least one such digit. ~keep
fn form_numbering_band(row: &TableRow) -> bool {
    let lone_digit = |text: &str| {
        let text = text.trim();
        text.len() == 1 && text.as_bytes()[0].is_ascii_digit() && text.as_bytes()[0] != b'0'
    };
    row.cells
        .iter()
        .all(|cell| cell.text.trim().is_empty() || lone_digit(&cell.text))
        && row.cells.iter().any(|cell| lone_digit(&cell.text))
}

/// Distinct text baselines in one band, counted to the caller's text-clustering `tolerance`.
fn band_line_count(row: &TableRow, tolerance: f32) -> usize {
    let mut baselines: Vec<f32> = Vec::new();
    for span in row.cells.iter().flat_map(|cell| &cell.spans) {
        let y = span.bbox.center().y;
        if !y.is_finite() {
            continue;
        }
        if !baselines.iter().any(|line| (line - y).abs() <= tolerance) {
            baselines.push(y);
        }
    }
    baselines.len()
}

/// Whether `bounds` is one source-drawn band: a visible rule on each side and no spanning rule
/// inside, so its cells are split by rules rather than by text.
fn one_drawn_band(bounds: Rect, paths: &[PathContent]) -> bool {
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return false;
    }
    let drawn: Vec<&PathContent> = paths.iter().collect();
    if ![bounds.top(), bounds.bottom()]
        .into_iter()
        .all(|y| band_rule(&drawn, bounds.left(), bounds.right(), y))
    {
        return false;
    }
    !rule_axis_positions(paths).into_iter().any(|y| {
        y > bounds.top() + HEADER_RULE_TOLERANCE
            && y < bounds.bottom() - HEADER_RULE_TOLERANCE
            && band_rule(&drawn, bounds.left(), bounds.right(), y)
    })
}

/// Whether the source painted a rule across `left..right` at `y`: a stroked line, or a filled bar
/// thin enough to be a rule rather than a cell background. Both go through
/// [`visible_rule_styles`], so a later fill crossing the rule hides it the same way for either.
pub(super) fn band_rule(paths: &[&PathContent], left: f32, right: f32, y: f32) -> bool {
    visible_rule_styles(paths, left, right, y, true).is_some()
}

/// A filled bar thin enough to be a painted rule rather than a cell background. Visible-region
/// accounting for the bar is the same one `visible_rule_styles` applies to a stroked rule; only the
/// axis acceptance differs.
fn is_rule_bar(path: &PathContent) -> bool {
    path.is_rectangle() && fill_only_bar(path) && path.bbox.height.abs() <= MAX_RULE_BAR_THICKNESS
}

/// Whether `path` lies on the horizontal rule axis at `y`.
///
/// `is_horizontal_line` tests thinness strictly, so a one-point filled bar — a common producer
/// style — needs its own acceptance on the same axis. A bar is judged by its painted geometry only:
/// fill alpha, soft masks and blend modes are unmodelled here as everywhere in this file, so a fully
/// transparent bar still reads as a rule. ~keep
fn on_rule_axis(path: &PathContent, y: f32) -> bool {
    (path.is_horizontal_line(HEADER_RULE_TOLERANCE) || is_rule_bar(path)) && same_rule_position(path.bbox.center().y, y)
}

/// Axis positions of every horizontal rule candidate: a stroked line or a thin filled bar.
fn rule_axis_positions<'a>(paths: impl IntoIterator<Item = &'a PathContent>) -> Vec<f32> {
    let mut positions: Vec<f32> = Vec::new();
    for path in paths {
        let candidate = (path.is_horizontal_line(HEADER_RULE_TOLERANCE) && path.stroke_color.is_some())
            || (is_rule_bar(path) && dark_fill(path));
        let y = path.bbox.center().y;
        if candidate && !positions.iter().any(|seen| same_rule_position(*seen, y)) {
            positions.push(y);
        }
    }
    positions
}

fn cell_has_visible_shading(cell: &TableCell, paths: &[&PathContent]) -> bool {
    let Some(cell) = cell.bbox.filter(|cell| cell.area().is_finite() && cell.area() > 0.0) else {
        return false;
    };
    let required = cell.area() * MIN_SHADED_CELL_AREA_RATIO;
    paths.iter().enumerate().any(|(index, path)| {
        let Some(shaded) = shading_rectangles([*path])
            .first()
            .and_then(|fill| fill.intersection(&cell))
            .filter(|fill| fill.area() >= required)
        else {
            return false;
        };
        visible_area_at_least(shaded, &paths[index + 1..], path.fill_color, required)
    })
}

fn visible_area_at_least(
    shaded: Rect,
    later_paths: &[&PathContent],
    color: Option<crate::layout::Color>,
    required: f32,
) -> bool {
    // Bound fragmentation on dense pages and withhold header inference when the proof exceeds it. ~keep
    const MAX_VISIBLE_FRAGMENTS: usize = 512;
    let mut visible = vec![shaded];
    let mut remaining = Vec::new();
    for cover in later_paths.iter().filter(|path| path.fill_color != color) {
        let bounds = cover.rendered_bbox();
        for rect in visible.drain(..) {
            let Some(overlap) = rect.intersection(&bounds).filter(|overlap| overlap.area() > 0.0) else {
                remaining.push(rect);
                continue;
            };
            // Four disjoint strips leave only the area outside the overlap. ~keep
            let strips = [
                Rect::new(rect.x, rect.y, rect.width, overlap.top() - rect.top()),
                Rect::new(rect.x, overlap.bottom(), rect.width, rect.bottom() - overlap.bottom()),
                Rect::new(rect.x, overlap.y, overlap.left() - rect.left(), overlap.height),
                Rect::new(
                    overlap.right(),
                    overlap.y,
                    rect.right() - overlap.right(),
                    overlap.height,
                ),
            ];
            remaining.extend(strips.into_iter().filter(|strip| strip.area() > 0.0));
        }
        if remaining.len() > MAX_VISIBLE_FRAGMENTS || remaining.iter().map(Rect::area).sum::<f32>() < required {
            return false;
        }
        std::mem::swap(&mut visible, &mut remaining);
    }
    true
}

type RuleStyle = ([u32; 3], u32);

/// Accept either visible group/leaf rules or a repeated schema with independent first-body shading.
/// Regular decimal leaves under matching rules require a painted header background. ~keep
pub(super) fn retain_grouped_leaf_header(rows: &mut [TableRow], paths: &[PathContent], columns: usize) {
    if !leaf_bands_are_headers(rows, paths, columns) {
        return;
    }
    let (head, _) = rows.split_at_mut(2);
    let [groups, leaves] = head else {
        return;
    };
    mark_header_bands(groups, leaves);
}

fn leaf_bands_are_headers(rows: &[TableRow], paths: &[PathContent], columns: usize) -> bool {
    let [groups, leaves, first_body, ..] = rows else {
        return false;
    };
    let Some(bounds) = rows_bounds([groups, leaves, first_body]) else {
        return false;
    };
    let nearby: Vec<&PathContent> = paths.iter().filter(|path| near_header(path, bounds)).collect();
    if nearby.len() > MAX_GROUPED_HEADER_PATHS || leaves.is_header || first_body.is_header {
        return false;
    }
    if single_title::has_stacked_leaf_header(groups, leaves, &rows[2..], &nearby, columns) {
        return true;
    }
    if single_title::has_leaf_header(groups, leaves, first_body, &nearby, columns) {
        return true;
    }
    if groups.is_header || !complete_source_band(first_body, columns, false) {
        return false;
    }
    let repeating_rules = body_repeats_the_header_rule(leaves, first_body, &nearby);
    if repeating_rules {
        centered_decimal_leaf_band(groups, leaves, first_body, &nearby, columns)
            && rows_bounds([groups, leaves])
                .is_some_and(|bounds| has_painted_header_background(bounds, nearby.iter().copied()))
    } else {
        complete_source_band(leaves, columns, true)
            && (grouped_header_rules(groups, leaves, &nearby)
                || repeated_schema_over_shaded_body(groups, leaves, first_body, &nearby))
    }
}

fn mark_header_bands(groups: &mut TableRow, leaves: &mut TableRow) {
    for row in [groups, leaves] {
        row.is_header = true;
        for cell in &mut row.cells {
            cell.is_header = true;
        }
    }
}

fn centered_decimal_leaf_band(
    groups: &TableRow,
    leaves: &TableRow,
    first_body: &TableRow,
    paths: &[&PathContent],
    columns: usize,
) -> bool {
    complete_source_band(leaves, columns, false)
        && super::continues_decimal_records(leaves, first_body)
        && super::group_covers_decimal_leaves(groups, leaves, |group, covered| {
            let (Some(group_bounds), Some(first), Some(last)) = (
                group.bbox,
                covered.first().and_then(|cell| cell.bbox),
                covered.last().and_then(|cell| cell.bbox),
            ) else {
                return false;
            };
            source_label(group, false)
                && same_rule_position(group_bounds.left(), first.left())
                && same_rule_position(group_bounds.right(), last.right())
                && same_rule_position(group_bounds.y, first.bottom())
                && visible_rule(paths, group_bounds.left(), group_bounds.right(), group_bounds.y)
        })
        && leaves.cells.iter().zip(&first_body.cells).all(|(leaf, body)| {
            matches!((leaf.bbox, body.bbox), (Some(label), Some(value))
                if same_rule_position(label.left(), value.left())
                    && same_rule_position(label.right(), value.right())
                    && same_rule_position(label.y, value.bottom())
                    && super::centered_decimal_pair(leaf, body, label.left(), label.right()))
        })
}

fn complete_source_band(row: &TableRow, columns: usize, bold: bool) -> bool {
    row.cells.len() == columns
        && row
            .cells
            .iter()
            .all(|cell| cell.colspan == 1 && source_label(cell, bold))
}

fn source_label(cell: &TableCell, bold: bool) -> bool {
    !cell.is_header && contained_source_label(cell, bold, 0.0)
}

fn contained_source_label(cell: &TableCell, bold: bool, upper_overhang: f32) -> bool {
    cell.bbox.is_some_and(|bounds| {
        cell.rowspan == 1
            && !cell.text.trim().is_empty()
            && !cell.spans.is_empty()
            && cell.spans.iter().all(|span| {
                span.rotation_degrees == 0.0
                    && span.wmode == 0
                    && span.font_weight.is_bold() == bold
                    && span.bbox.left() >= bounds.left()
                    && span.bbox.right() <= bounds.right()
                    && span.bbox.top() >= bounds.top()
                    && span.bbox.bottom() <= bounds.bottom() + upper_overhang
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

/// Certify repeated alphabetic leaf schemas under bold, aligned groups using unshaded headers and an
/// independently shaded, regular first body band. First-body cells after column zero must contain an
/// ASCII digit (`A1` qualifies); numeric-only strings are not required. This proof does not require visible
/// separators, including when they are white, dashed, zero-opacity or light. ~keep
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
/// If the first body row repeats the rule, a retained header needs independent painted evidence. ~keep
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
    let header = visible_rule_styles(paths, first.left(), last.right(), first.y, false);
    let below = visible_rule_styles(paths, first.left(), last.right(), body.y, false);
    matches!((header, below), (Some(header), Some(below)) if header == below)
}

fn rows_bounds<const N: usize>(rows: [&TableRow; N]) -> Option<Rect> {
    rows.into_iter()
        .flat_map(|row| &row.cells)
        .filter_map(|cell| cell.bbox)
        .reduce(|all, bounds| all.union(&bounds))
}

/// A single visible fill covering both heading bands and ending at the body boundary
/// distinguishes deliberate header styling from a centered but otherwise ordinary record. ~keep
pub(super) fn has_painted_header_background<'a>(
    bounds: Rect,
    paths: impl IntoIterator<Item = &'a PathContent>,
) -> bool {
    let area = bounds.area();
    if !area.is_finite() || area <= 0.0 {
        return false;
    }
    let nearby: Vec<_> = paths
        .into_iter()
        .filter(|path| near_header(path, bounds))
        .take(MAX_GROUPED_HEADER_PATHS + 1)
        .collect();
    if nearby.len() > MAX_GROUPED_HEADER_PATHS {
        return false;
    }
    let required = area * MIN_SHADED_CELL_AREA_RATIO;
    nearby.iter().enumerate().any(|(index, path)| {
        let Some(fill) = shading_rectangles([*path]).into_iter().next() else {
            return false;
        };
        if !same_rule_position(fill.left(), bounds.left())
            || !same_rule_position(fill.right(), bounds.right())
            || !same_rule_position(fill.y, bounds.y)
            || !same_rule_position(fill.bottom(), bounds.bottom())
        {
            return false;
        }
        fill.intersection(&bounds).is_some_and(|overlap| {
            overlap.area() >= required
                && visible_area_at_least(overlap, &nearby[index + 1..], path.fill_color, required)
        })
    })
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
    visible_rule_styles(paths, left, right, y, false).is_some()
}

/// The colours and weights of the segments that together make the rule visible, or `None`.
fn visible_rule_styles(
    paths: &[&PathContent],
    left: f32,
    right: f32,
    y: f32,
    accept_filled_bars: bool,
) -> Option<Vec<RuleStyle>> {
    let mut visible: Vec<(f32, f32, RuleStyle)> = Vec::new();
    for path in paths {
        let drawn = path.rendered_bbox();
        let (low, high) = (drawn.left(), drawn.right());
        let on_axis = on_rule_axis(path, y);
        let stroked_rule = on_axis && path.stroke_color.is_some();
        let filled_rule = on_axis && accept_filled_bars && fill_only_bar(path);
        if stroked_rule && dark_solid_stroke(path) {
            visible.push((low, high, rule_style(path)));
        } else if filled_rule && dark_fill(path) {
            visible.push((path.bbox.left(), path.bbox.right(), rule_style(path)));
        } else if stroked_rule || filled_rule || covering_fill(path, y) {
            // A vertical bar filled at or below MAX_HEADER_RULE_CHANNEL cannot hide a dark rule it crosses. ~keep
            if accept_filled_bars
                && path.is_vertical_line(HEADER_RULE_TOLERANCE)
                && fill_only_bar(path)
                && dark_fill(path)
            {
                continue;
            }
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

fn fill_only_bar(path: &PathContent) -> bool {
    path.is_rectangle() && path.stroke_color.is_none() && path.has_fill()
}

fn dark_fill(path: &PathContent) -> bool {
    path.fill_color.is_some_and(|color| {
        [color.r, color.g, color.b]
            .into_iter()
            .all(|component| (0.0..=MAX_HEADER_RULE_CHANNEL).contains(&component))
    })
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
pub(super) fn dark_solid_stroke(path: &PathContent) -> bool {
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
