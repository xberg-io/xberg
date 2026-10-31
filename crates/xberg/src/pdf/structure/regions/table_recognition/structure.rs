use crate::types::{BoundingBox, GridCell, TableGrid};

use super::{TatrCrop, TatrInference};

type SpanPlan = Vec<Vec<Option<(usize, usize)>>>;

struct CellRegion {
    row: usize,
    col: usize,
    row_span: usize,
    col_span: usize,
}

pub(super) fn apply_structure(
    dense: &mut [Vec<String>],
    inference: &TatrInference,
    crop: &TatrCrop,
    page_height: f32,
) -> TableGrid {
    let rows = dense.len();
    let cols = dense.first().map_or(0, Vec::len);
    let mut plan = vec![vec![Some((1, 1)); cols]; rows];
    for &span in &inference.structure.spans {
        merge_span(dense, &mut plan, span);
    }
    let mut cells = Vec::new();
    for (row, columns) in plan.into_iter().enumerate() {
        for (col, span) in columns.into_iter().enumerate() {
            let Some((row_span, col_span)) = span else { continue };
            let region = CellRegion {
                row,
                col,
                row_span,
                col_span,
            };
            cells.push(grid_cell(dense, inference, crop, page_height, &region));
        }
    }
    TableGrid {
        rows: rows as u32,
        cols: cols as u32,
        cells,
    }
}

fn merge_span(
    dense: &mut [Vec<String>],
    plan: &mut SpanPlan,
    (row_start, row_end, col_start, col_end): (usize, usize, usize, usize),
) {
    let cols = dense.first().map_or(0, Vec::len);
    if row_start >= row_end || col_start >= col_end || row_end > dense.len() || col_end > cols {
        return;
    }
    if plan[row_start..row_end]
        .iter()
        .any(|row| row[col_start..col_end].iter().any(|cell| *cell != Some((1, 1))))
    {
        return;
    }
    let mut fragments = Vec::new();
    for row in row_start..row_end {
        for col in col_start..col_end {
            plan[row][col] = None;
            let text = std::mem::take(&mut dense[row][col]);
            if !text.is_empty() {
                fragments.push(text);
            }
        }
    }
    dense[row_start][col_start] = fragments.join(" ");
    plan[row_start][col_start] = Some((row_end - row_start, col_end - col_start));
}

fn grid_cell(
    dense: &[Vec<String>],
    inference: &TatrInference,
    crop: &TatrCrop,
    page_height: f32,
    region: &CellRegion,
) -> GridCell {
    GridCell {
        content: dense[region.row][region.col].clone(),
        row: region.row as u32,
        col: region.col as u32,
        row_span: region.row_span as u32,
        col_span: region.col_span as u32,
        is_header: region.row < inference.structure.header_row_count,
        bbox: cell_bbox(inference, crop, page_height, region),
        heading_level: None,
        style_name: None,
    }
}

fn cell_bbox(inference: &TatrInference, crop: &TatrCrop, page_height: f32, region: &CellRegion) -> Option<BoundingBox> {
    let first = inference.cells.get(region.row)?.get(region.col)?;
    let last = inference
        .cells
        .get(region.row + region.row_span - 1)?
        .get(region.col + region.col_span - 1)?;
    Some(BoundingBox {
        x0: f64::from((first.x1 + crop.px_left as f32) / crop.sx),
        y0: f64::from(page_height - (last.y2 + crop.px_top as f32) / crop.sy),
        x1: f64::from((last.x2 + crop.px_left as f32) / crop.sx),
        y1: f64::from(page_height - (first.y1 + crop.px_top as f32) / crop.sy),
    })
}
