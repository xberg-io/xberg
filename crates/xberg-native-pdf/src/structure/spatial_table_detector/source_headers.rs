use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::structure::table_extractor::{Table, TableCell};

// Cell fills are often inset from their rules; thin strokes must not count as a background. ~keep
const MIN_SHADED_CELL_AREA_RATIO: f32 = 0.9;

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
