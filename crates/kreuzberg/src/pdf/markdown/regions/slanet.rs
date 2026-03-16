//! SLANet-based table recognition for native PDF pages.

use super::super::geometry::Rect;

#[cfg(feature = "layout-detection")]
use crate::pdf::markdown::render::escape_html_entities;
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

/// Recognize tables on a native PDF page using SLANet structure prediction.
///
/// Crops table regions from the rendered layout detection image, runs SLANet
/// inference, then matches predicted cell bboxes against native PDF words.
///
/// # Coordinate conversion
///
/// Three coordinate spaces are involved:
/// - **PDF coords**: LayoutHint bboxes and HocrWord positions (y=0 at bottom for hints;
///   HocrWord uses image-coords with y=0 at top, converted via `page_height - pdf_top`).
/// - **Rendered image pixels**: The ~640px image used for layout detection.
/// - **SLANet crop pixels**: Cell bboxes relative to the cropped table region.
#[cfg(feature = "layout-detection")]
pub(in crate::pdf::markdown) fn recognize_tables_for_native_page(
    page_image: &image::DynamicImage,
    hints: &[LayoutHint],
    words: &[crate::pdf::table_reconstruct::HocrWord],
    page_result: &crate::pdf::layout_runner::PageLayoutResult,
    page_height: f32,
    page_index: usize,
    slanet: &mut crate::layout::models::slanet::SlaNetModel,
) -> Vec<Table> {
    let rgb_image = page_image.to_rgb8();
    let img_w = rgb_image.width();
    let img_h = rgb_image.height();

    // Scale factors: PDF points → rendered image pixels
    let sx = img_w as f32 / page_result.page_width_pts;
    let sy = img_h as f32 / page_result.page_height_pts;

    let table_hints: Vec<&LayoutHint> = hints
        .iter()
        .filter(|h| h.class == LayoutHintClass::Table && h.confidence >= 0.5)
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

        // Guard: skip SLANet on extremely large crops that would slow inference
        if (crop_w as u64) * (crop_h as u64) > 1_000_000 {
            tracing::debug!(
                page = page_index,
                crop_w,
                crop_h,
                "Skipping SLANet for oversized table crop"
            );
            continue;
        }

        // Crop table region from rendered image
        let cropped = image::imageops::crop_imm(&rgb_image, px_left, px_top, crop_w, crop_h).to_image();

        // Run SLANet inference
        let slanet_result = match slanet.recognize(&cropped) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("SLANet inference failed for table on page {}: {e}", page_index);
                continue;
            }
        };

        if slanet_result.structure_tokens.is_empty() {
            tracing::debug!(page = page_index, "SLANet: empty structure tokens");
            continue;
        }

        let rows = crate::layout::models::slanet::parse_table_structure(&slanet_result);
        tracing::debug!(
            page = page_index,
            structure_tokens = slanet_result.structure_tokens.len(),
            rows = rows.len(),
            cells = rows.iter().map(|r| r.len()).sum::<usize>(),
            cell_bboxes = slanet_result.cell_bboxes.len(),
            crop = format!("{}x{}", crop_w, crop_h),
            tokens_preview = slanet_result.structure_tokens.iter().take(15).cloned().collect::<Vec<_>>().join(", "),
            "SLANet inference result"
        );
        if rows.is_empty() {
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

        // Build markdown by matching words to SLANet cells
        let markdown = build_native_slanet_table(&rows, &table_words, px_left as f32, px_top as f32, sx, sy);

        tracing::debug!(
            page = page_index,
            table_words = table_words.len(),
            markdown_len = markdown.len(),
            "SLANet: word matching and markdown generation"
        );
        if markdown.is_empty() {
            tracing::debug!(page = page_index, "SLANet: empty markdown output");
            continue;
        }

        // Validate: reject SLANet output if too few cells have content.
        // This catches cases where SLANet cell bboxes don't align with the
        // actual text, producing mostly empty cells.
        let total_cells: usize = rows.iter().map(|r| r.len()).sum();
        let filled_cells = markdown
            .split('|')
            .filter(|s| !s.trim().is_empty() && s.trim() != "---")
            .count();
        if total_cells > 4 && filled_cells < total_cells / 4 {
            tracing::debug!(
                page = page_index,
                total_cells,
                filled_cells,
                "SLANet table rejected: too few filled cells"
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

/// Build markdown table from SLANet structure + native PDF words.
///
/// SLANet cell bboxes are in crop-pixel space. Words are in PDF image-coord space
/// (HocrWord: left in PDF x-units, top = page_height - pdf_top).
#[cfg(feature = "layout-detection")]
fn build_native_slanet_table(
    rows: &[Vec<crate::layout::models::slanet::TableCell>],
    words: &[&crate::pdf::table_reconstruct::HocrWord],
    crop_offset_px_x: f32,
    crop_offset_px_y: f32,
    sx: f32,
    sy: f32,
) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let max_cols = rows
        .iter()
        .map(|row| row.iter().map(|c| c.colspan).sum::<u32>())
        .max()
        .unwrap_or(0) as usize;

    if max_cols == 0 {
        return String::new();
    }

    let mut grid: Vec<Vec<String>> = Vec::with_capacity(rows.len());

    for row in rows {
        let mut grid_row = vec![String::new(); max_cols];
        let mut col_idx = 0usize;

        for cell in row {
            if col_idx >= max_cols {
                break;
            }

            let cell_text = if let Some(ref bbox) = cell.bbox {
                // Convert SLANet crop-pixel bbox to HocrWord coordinate space:
                // 1. Add crop offset to get page-image-pixel coords
                // 2. Divide by scale factors to get PDF point coords
                //    (HocrWord.left is in PDF x-units, HocrWord.top is page_height - pdf_y)
                let cell_left = (bbox.x1 + crop_offset_px_x) / sx;
                let cell_right = (bbox.x2 + crop_offset_px_x) / sx;
                let cell_top = (bbox.y1 + crop_offset_px_y) / sy;
                let cell_bottom = (bbox.y2 + crop_offset_px_y) / sy;

                match_words_to_cell(words, cell_left, cell_top, cell_right, cell_bottom)
            } else {
                String::new()
            };

            grid_row[col_idx] = cell_text;
            col_idx += cell.colspan as usize;
        }

        grid.push(grid_row);
    }

    render_grid_as_markdown(&grid)
}

/// Match HocrWords to a cell using intersection-over-word-area (IoW).
///
/// Assigns a word to this cell if the overlap between the word bbox and cell bbox
/// covers at least 20% of the word's area (matching docling's cell assignment
/// threshold). This is more robust than center-point containment for words that
/// straddle cell boundaries.
#[cfg(feature = "layout-detection")]
fn match_words_to_cell(
    words: &[&crate::pdf::table_reconstruct::HocrWord],
    cell_left: f32,
    cell_top: f32,
    cell_right: f32,
    cell_bottom: f32,
) -> String {
    let mut matched: Vec<(&crate::pdf::table_reconstruct::HocrWord, f32, f32)> = Vec::new();

    for &word in words {
        let iow = word_hint_iow(word, cell_left, cell_top, cell_right, cell_bottom);
        if iow >= 0.2 {
            let cx = word.left as f32 + word.width as f32 / 2.0;
            let cy = word.top as f32 + word.height as f32 / 2.0;
            matched.push((word, cx, cy));
        }
    }

    if matched.is_empty() {
        return String::new();
    }

    // Sort by y then x for reading order
    matched.sort_by(|a, b| a.2.total_cmp(&b.2).then_with(|| a.1.total_cmp(&b.1)));

    matched
        .iter()
        .map(|(w, _, _)| w.text.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

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
            let escaped = escape_html_entities(&pipe_escaped);
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
