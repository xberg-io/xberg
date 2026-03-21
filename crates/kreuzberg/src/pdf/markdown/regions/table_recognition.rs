//! TATR-based table recognition for native PDF pages.

use super::super::geometry::Rect;

#[cfg(feature = "layout-detection")]
use crate::pdf::markdown::render::escape_html_entities_no_underscores;
#[cfg(feature = "layout-detection")]
use crate::pdf::markdown::types::{LayoutHint, LayoutHintClass};
#[cfg(feature = "layout-detection")]
use crate::types::Table;

/// Compute intersection-over-word-area between an HocrWord and a rectangular region.
///
/// Both word and region must be in the same coordinate space (image coords).
pub(in crate::pdf::markdown) fn word_hint_iow(
    w: &crate::pdf::table_reconstruct::HocrWord,
    region_left: f32,
    region_top: f32,
    region_right: f32,
    region_bottom: f32,
) -> f32 {
    let word_rect = Rect::from_xywh(w.left as f32, w.top as f32, w.width as f32, w.height as f32);
    let region_rect = Rect::from_ltrb(region_left, region_top, region_right, region_bottom);
    if word_rect.area() <= 0.0 {
        // Zero-area word: fall back to center-point containment (0 or 1)
        return if region_rect.contains_point(word_rect.center_x(), word_rect.center_y()) {
            1.0
        } else {
            0.0
        };
    }
    word_rect.intersection_over_self(&region_rect)
}

/// Recognize tables on a native PDF page using TATR structure prediction.
///
/// Crops table regions from the rendered layout detection image, runs TATR
/// inference, then matches predicted cell bboxes against native PDF words.
///
/// # Coordinate conversion
///
/// Three coordinate spaces are involved:
/// - **PDF coords**: LayoutHint bboxes and HocrWord positions (y=0 at bottom for hints;
///   HocrWord uses image-coords with y=0 at top, converted via `page_height - pdf_top`).
/// - **Rendered image pixels**: The ~640px image used for layout detection.
/// - **TATR crop pixels**: Cell bboxes relative to the cropped table region.
#[cfg(feature = "layout-detection")]
pub(in crate::pdf::markdown) fn recognize_tables_for_native_page(
    page_image: &image::DynamicImage,
    hints: &[LayoutHint],
    words: &[crate::pdf::table_reconstruct::HocrWord],
    page_result: &crate::pdf::layout_runner::PageLayoutResult,
    page_height: f32,
    page_index: usize,
    tatr_model: &mut crate::layout::models::tatr::TatrModel,
) -> Vec<Table> {
    let rgb_image = page_image.to_rgb8();
    let img_w = rgb_image.width();
    let img_h = rgb_image.height();

    // Scale factors: PDF points → rendered image pixels
    let sx = img_w as f32 / page_result.page_width_pts;
    let sy = img_h as f32 / page_result.page_height_pts;

    let table_hints: Vec<&LayoutHint> = hints
        .iter()
        .filter(|h| {
            if h.class != LayoutHintClass::Table || h.confidence < 0.5 {
                return false;
            }
            // Structural hint guard relaxed: region assignment now handles
            // text/table overlap correctly by assigning segments to Table
            // regions instead of suppressing them. Small tables on structured
            // pages are now allowed through since double-counting is prevented
            // by the region-first assembly approach.
            true
        })
        .collect();

    let mut tables = Vec::new();

    for hint in &table_hints {
        // Convert hint bbox from PDF coords to rendered image pixel coords.
        // PDF: y=0 at bottom, increases upward.
        // Image: y=0 at top, increases downward.
        let px_left = (hint.left * sx).round().max(0.0) as u32;
        let px_top = ((page_height - hint.top) * sy).round().max(0.0) as u32;
        let px_right = (hint.right * sx).round().min(img_w as f32) as u32;
        let px_bottom = ((page_height - hint.bottom) * sy).round().min(img_h as f32) as u32;

        let crop_w = px_right.saturating_sub(px_left);
        let crop_h = px_bottom.saturating_sub(px_top);

        if crop_w < 10 || crop_h < 10 {
            continue;
        }

        // Guard: skip TATR on extremely large crops that would slow inference.
        // DETR preprocessing resizes the crop (shortest edge → 800, cap 1333),
        // so even large crops are feasible; 4M pixels (~2000x2000) is generous
        // enough for tables rendered from the ~640px layout image.
        if (crop_w as u64) * (crop_h as u64) > 4_000_000 {
            tracing::debug!(
                page = page_index,
                crop_w,
                crop_h,
                "Skipping TATR for oversized table crop"
            );
            continue;
        }

        // Crop table region from rendered image
        let cropped = image::imageops::crop_imm(&rgb_image, px_left, px_top, crop_w, crop_h).to_image();

        // Run TATR inference
        let tatr_result = match tatr_model.recognize(&cropped) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("TATR inference failed for table on page {}: {e}", page_index);
                continue;
            }
        };

        // Check if TATR detected any rows and columns
        if tatr_result.rows.is_empty() || tatr_result.columns.is_empty() {
            tracing::debug!(
                page = page_index,
                rows = tatr_result.rows.len(),
                columns = tatr_result.columns.len(),
                "TATR: no rows or columns detected"
            );
            continue;
        }

        // Build cell grid from row × column intersections
        let cell_grid = crate::layout::models::tatr::build_cell_grid(&tatr_result, None);
        let num_rows = cell_grid.len();
        let num_cols = if num_rows > 0 { cell_grid[0].len() } else { 0 };

        tracing::debug!(
            page = page_index,
            detected_rows = tatr_result.rows.len(),
            detected_columns = tatr_result.columns.len(),
            grid_rows = num_rows,
            grid_cols = num_cols,
            crop = format!("{}x{}", crop_w, crop_h),
            "TATR inference result"
        );

        if num_rows == 0 || num_cols == 0 {
            continue;
        }

        // Filter words that overlap the table hint bbox (≥20% of word area).
        // HocrWord uses image coordinates (y=0 at top).
        let hint_img_top = (page_height - hint.top).max(0.0);
        let hint_img_bottom = (page_height - hint.bottom).max(0.0);

        let table_words: Vec<&crate::pdf::table_reconstruct::HocrWord> = words
            .iter()
            .filter(|w| {
                if w.text.trim().is_empty() {
                    return false;
                }
                word_hint_iow(w, hint.left, hint_img_top, hint.right, hint_img_bottom) >= 0.2
            })
            .collect();

        // Match words to cells and build markdown table.
        // Cell bboxes are in crop-pixel space; words are in PDF coords.
        // Convert cell bboxes to PDF coords for matching.
        let markdown = build_tatr_grid_table(&cell_grid, &table_words, px_left as f32, px_top as f32, sx, sy);

        tracing::debug!(
            page = page_index,
            table_words = table_words.len(),
            markdown_len = markdown.len(),
            "TATR: word matching and markdown generation"
        );
        if markdown.is_empty() {
            tracing::debug!(page = page_index, "TATR: empty markdown output");
            continue;
        }

        // Validate: reject TATR output if too few cells have content.
        let total_cells = num_rows * num_cols;
        let filled_cells = markdown
            .split('|')
            .filter(|s| !s.trim().is_empty() && s.trim() != "---")
            .count();
        if total_cells > 4 && filled_cells < total_cells / 4 {
            tracing::debug!(
                page = page_index,
                total_cells,
                filled_cells,
                "TATR table rejected: too few filled cells"
            );
            continue;
        }

        let bounding_box = Some(crate::types::BoundingBox {
            x0: hint.left as f64,
            y0: hint.bottom as f64,
            x1: hint.right as f64,
            y1: hint.top as f64,
        });

        tables.push(Table {
            cells: Vec::new(),
            markdown,
            page_number: page_index + 1,
            bounding_box,
        });
    }

    tables
}

/// Build markdown table from TATR cell grid + PDF words.
///
/// Cell bboxes are in crop-pixel space. Words are in PDF image-coord space
/// (HocrWord: left in PDF x-units, top = page_height - pdf_top).
/// Converts cell coords to word space via crop offset + scale factors.
///
/// Uses best-match assignment: each word is assigned to the single cell with
/// the highest IoW overlap, preventing duplication across cells.
#[cfg(feature = "layout-detection")]
fn build_tatr_grid_table(
    cell_grid: &[Vec<crate::layout::models::tatr::CellBBox>],
    words: &[&crate::pdf::table_reconstruct::HocrWord],
    crop_offset_px_x: f32,
    crop_offset_px_y: f32,
    sx: f32,
    sy: f32,
) -> String {
    if cell_grid.is_empty() {
        return String::new();
    }

    let num_rows = cell_grid.len();
    let num_cols = cell_grid[0].len();
    if num_cols == 0 {
        return String::new();
    }

    // Convert all cell bboxes from crop-pixel space to HocrWord coordinate
    // space (PDF point units, image-oriented y).
    let mut converted_cells: Vec<Vec<(f32, f32, f32, f32)>> = Vec::with_capacity(num_rows);
    for row in cell_grid {
        let mut conv_row = Vec::with_capacity(num_cols);
        for cell in row {
            let cell_left = (cell.x1 + crop_offset_px_x) / sx;
            let cell_right = (cell.x2 + crop_offset_px_x) / sx;
            let cell_top = (cell.y1 + crop_offset_px_y) / sy;
            let cell_bottom = (cell.y2 + crop_offset_px_y) / sy;
            conv_row.push((cell_left, cell_top, cell_right, cell_bottom));
        }
        converted_cells.push(conv_row);
    }

    // Best-match assignment: assign each word to the single cell with the
    // highest IoW, preventing the same word from appearing in multiple cells.
    // Store (word_index, cx, cy) per cell for reading-order sorting.
    let mut cell_words: Vec<Vec<Vec<(usize, f32, f32)>>> = (0..num_rows)
        .map(|_| (0..num_cols).map(|_| Vec::new()).collect())
        .collect();

    for (wi, &word) in words.iter().enumerate() {
        let mut best_iow: f32 = 0.0;
        let mut best_row: usize = 0;
        let mut best_col: usize = 0;

        for (ri, conv_row) in converted_cells.iter().enumerate() {
            for (ci, &(cl, ct, cr, cb)) in conv_row.iter().enumerate() {
                let iow = word_hint_iow(word, cl, ct, cr, cb);
                if iow > best_iow {
                    best_iow = iow;
                    best_row = ri;
                    best_col = ci;
                }
            }
        }

        if best_iow >= 0.2 {
            let cx = word.left as f32 + word.width as f32 / 2.0;
            let cy = word.top as f32 + word.height as f32 / 2.0;
            cell_words[best_row][best_col].push((wi, cx, cy));
        }
    }

    // Build the text grid from the assigned words.
    let mut grid: Vec<Vec<String>> = Vec::with_capacity(num_rows);
    for row_cells in &cell_words {
        let mut grid_row = vec![String::new(); num_cols];
        for (ci, cell_word_indices) in row_cells.iter().enumerate() {
            if cell_word_indices.is_empty() {
                continue;
            }
            // Sort words within the cell by reading order (y then x).
            let mut sorted = cell_word_indices.clone();
            sorted.sort_by(|a, b| a.2.total_cmp(&b.2).then_with(|| a.1.total_cmp(&b.1)));
            let text: String = sorted
                .iter()
                .map(|(wi, _, _)| words[*wi].text.trim())
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            grid_row[ci] = text;
        }
        grid.push(grid_row);
    }

    render_grid_as_markdown(&grid)
}

// Word-to-cell matching is now handled inline in build_tatr_grid_table
// using best-match assignment (each word assigned to exactly one cell).

/// Render a grid of cell text strings as a markdown table.
#[cfg(feature = "layout-detection")]
fn render_grid_as_markdown(grid: &[Vec<String>]) -> String {
    if grid.is_empty() {
        return String::new();
    }

    let max_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return String::new();
    }

    let mut md = String::new();

    for (row_idx, row) in grid.iter().enumerate() {
        md.push('|');
        for col in 0..max_cols {
            let cell = row.get(col).map(|s| s.as_str()).unwrap_or("");
            // Escape pipe characters first, then HTML entities
            let pipe_escaped = cell.replace('|', "\\|");
            let escaped = escape_html_entities_no_underscores(&pipe_escaped);
            md.push(' ');
            md.push_str(escaped.trim());
            md.push_str(" |");
        }
        md.push('\n');

        if row_idx == 0 {
            md.push('|');
            for _ in 0..max_cols {
                md.push_str(" --- |");
            }
            md.push('\n');
        }
    }

    if md.ends_with('\n') {
        md.pop();
    }
    md
}
