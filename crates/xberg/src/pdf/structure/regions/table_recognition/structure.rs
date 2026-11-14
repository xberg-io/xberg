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
    let header_rows = header_row_count(dense, &plan, inference.structure.header_row_count);
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
            cells.push(grid_cell(
                dense,
                inference,
                crop,
                page_height,
                &region,
                row < header_rows,
            ));
        }
    }
    TableGrid {
        rows: rows as u32,
        cols: cols as u32,
        cells,
    }
}

// Two consecutive record labels in a later section corroborate the earlier
// section that the model included in the header band.
fn header_row_count(dense: &[Vec<String>], plan: &SpanPlan, detected: usize) -> usize {
    for (row, bands) in dense.windows(4).enumerate().take(detected.saturating_sub(1)) {
        let [years, section, first, second] = bands else {
            continue;
        };
        if years.len() >= 3
            && years[1..]
                .iter()
                .all(|text| text.len() == 4 && text.bytes().all(|byte| byte.is_ascii_digit()))
            && years[1..].windows(2).all(|pair| pair[0] != pair[1])
            && is_section_stub(section)
            && plan[row..row + 4].iter().flatten().all(|span| *span == Some((1, 1)))
            && [first, second].into_iter().all(|record| is_year_aligned_record(record))
            && first[0].trim() != second[0].trim()
            && dense[(row + 4).max(detected).min(dense.len())..]
                .windows(3)
                .any(|later| {
                    is_section_stub(&later[0])
                        && later[0][0].trim() != section[0].trim()
                        && later[1][0].trim() == first[0].trim()
                        && is_year_aligned_record(&later[1])
                        && later[2][0].trim() == second[0].trim()
                        && has_year_aligned_values(&later[2])
                        && later[2][1..].iter().any(|value| !value.trim().is_empty())
                })
        {
            return row + 1;
        }
    }
    detected
}

fn is_section_stub(row: &[String]) -> bool {
    row[0].chars().any(char::is_alphabetic) && row[1..].iter().all(|text| text.trim().is_empty())
}

fn is_year_aligned_record(row: &[String]) -> bool {
    row[0].chars().any(char::is_alphabetic)
        && has_year_aligned_values(row)
        && row[1..]
            .iter()
            .any(|value| value.bytes().any(|byte| byte.is_ascii_digit()))
}

fn has_year_aligned_values(row: &[String]) -> bool {
    row[1..].iter().all(|value| {
        let value = value.trim();
        value.is_empty()
            || matches!(value, "-" | "–" | "—")
            || (value.bytes().any(|byte| byte.is_ascii_digit())
                && value
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || "$€£¥,.()+-% ".contains(ch)))
    })
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
    is_header: bool,
) -> GridCell {
    GridCell {
        content: dense[region.row][region.col].clone(),
        row: region.row as u32,
        col: region.col as u32,
        row_span: region.row_span as u32,
        col_span: region.col_span as u32,
        is_header,
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
