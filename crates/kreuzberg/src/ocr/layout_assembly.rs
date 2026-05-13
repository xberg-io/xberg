//! Layout-aware OCR table recognition.
//!
//! This module provides TATR-based table structure recognition for OCR pages.
//! It operates entirely in pixel space — no coordinate conversion is needed
//! because both OCR elements and layout detections use the same image
//! coordinate system (origin top-left, y increases downward).

use crate::layout::models::tatr::{self, TatrModel};
use crate::layout::types::{BBox, DetectionResult, LayoutClass, RecognizedTable};
use crate::types::OcrElement;

/// Default confidence threshold for layout detections.
const MIN_CONFIDENCE: f32 = 0.3;

/// Run TATR table recognition for all Table regions in a page.
///
/// For each Table detection, crops the page image, runs TATR inference,
/// matches OCR elements to cells, and produces markdown tables.
pub(crate) fn recognize_page_tables(
    page_image: &image::RgbImage,
    detection: &DetectionResult,
    elements: &[OcrElement],
    tatr_model: &mut TatrModel,
) -> Vec<RecognizedTable> {
    let mut tables = Vec::new();

    for det in &detection.detections {
        if det.class_name != LayoutClass::Table || det.confidence < MIN_CONFIDENCE {
            continue;
        }

        let result = recognize_single_table(page_image, &det.bbox, elements, tatr_model);
        if let Some((cells, markdown)) = result {
            tables.push(RecognizedTable {
                detection_bbox: det.bbox,
                cells,
                markdown,
            });
        }
    }

    tables
}

/// Recognize a single table from a cropped region of the page.
///
/// Returns `(cells, markdown)` where cells is the 2D grid of cell text content.
fn recognize_single_table(
    page_image: &image::RgbImage,
    table_bbox: &BBox,
    elements: &[OcrElement],
    tatr_model: &mut TatrModel,
) -> Option<(Vec<Vec<String>>, String)> {
    // Crop the table region from the page image
    let crop_x = table_bbox.x1.max(0.0) as u32;
    let crop_y = table_bbox.y1.max(0.0) as u32;
    let crop_w = (table_bbox.width() as u32).min(page_image.width().saturating_sub(crop_x));
    let crop_h = (table_bbox.height() as u32).min(page_image.height().saturating_sub(crop_y));

    if crop_w == 0 || crop_h == 0 {
        return None;
    }

    let cropped = image::imageops::crop_imm(page_image, crop_x, crop_y, crop_w, crop_h).to_image();

    // Run TATR inference
    let tatr_result = match tatr_model.recognize(&cropped) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("TATR inference failed: {e}");
            return None;
        }
    };

    // Check if TATR detected any rows and columns
    if tatr_result.rows.is_empty() || tatr_result.columns.is_empty() {
        return None;
    }

    // Build cell grid from row × column intersections
    let cell_grid = tatr::build_cell_grid(&tatr_result, None);
    if cell_grid.is_empty() || cell_grid[0].is_empty() {
        return None;
    }

    // Collect OCR elements that overlap the table region (≥20% of element area)
    let table_elements: Vec<&OcrElement> = elements
        .iter()
        .filter(|e| {
            if e.text.trim().is_empty() {
                return false;
            }
            element_bbox_iow(e, table_bbox) >= 0.2
        })
        .collect();

    // Build markdown table by matching OCR elements to cells
    let (cells, markdown) = build_markdown_table(&cell_grid, &table_elements, crop_x as f32, crop_y as f32);
    Some((cells, markdown))
}

/// Build a markdown table from TATR cell grid + OCR elements.
///
/// Cell bboxes from TATR are in cropped-image coordinates.
/// OCR elements are in page coordinates. `offset_x/y` translates between them.
fn build_markdown_table(
    cell_grid: &[Vec<tatr::CellBBox>],
    elements: &[&OcrElement],
    offset_x: f32,
    offset_y: f32,
) -> (Vec<Vec<String>>, String) {
    if cell_grid.is_empty() {
        return (Vec::new(), String::new());
    }

    let num_cols = cell_grid[0].len();

    if num_cols == 0 {
        return (Vec::new(), String::new());
    }

    // Fill grid with cell text
    let mut grid: Vec<Vec<String>> = Vec::with_capacity(cell_grid.len());

    for row in cell_grid {
        let mut grid_row = vec![String::new(); num_cols];

        for (col_idx, cell) in row.iter().enumerate() {
            // Translate cell bbox from crop coords to page coords
            let page_bbox = BBox::new(
                cell.x1 + offset_x,
                cell.y1 + offset_y,
                cell.x2 + offset_x,
                cell.y2 + offset_y,
            );
            grid_row[col_idx] = match_elements_to_cell(elements, &page_bbox);
        }

        grid.push(grid_row);
    }

    // Render as markdown table
    let mut md = String::new();

    for (row_idx, row) in grid.iter().enumerate() {
        md.push('|');
        for cell in row {
            // Escape pipe characters in cell text
            let escaped = cell.replace('|', "\\|");
            md.push(' ');
            md.push_str(escaped.trim());
            md.push_str(" |");
        }
        md.push('\n');

        // Add separator after first row (header)
        if row_idx == 0 {
            md.push('|');
            for _ in 0..num_cols {
                md.push_str(" --- |");
            }
            md.push('\n');
        }
    }

    // Remove trailing newline
    if md.ends_with('\n') {
        md.pop();
    }

    (grid, md)
}

/// Match OCR elements to a cell bbox, returning the cell's text content.
///
/// Uses intersection-over-word-area (IoW) matching: an element is assigned to
/// this cell if the overlap between the element bbox and cell bbox covers at
/// least 20% of the element's area. This is more robust than center-point
/// containment for elements that straddle cell boundaries.
fn match_elements_to_cell(elements: &[&OcrElement], cell_bbox: &BBox) -> String {
    let mut matched: Vec<(&OcrElement, f32, f32)> = Vec::new();

    for elem in elements {
        let iow = element_bbox_iow(elem, cell_bbox);
        if iow >= 0.2 {
            let (cx, cy) = element_center_f32(elem);
            matched.push((elem, cx, cy));
        }
    }

    if matched.is_empty() {
        return String::new();
    }

    // Sort by y then x for reading order
    matched.sort_by(|a, b| a.2.total_cmp(&b.2).then_with(|| a.1.total_cmp(&b.1)));

    matched
        .iter()
        .map(|(e, _, _)| e.text.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Compute intersection-over-word-area (IoW) between an OCR element and a BBox.
///
/// Returns the fraction of the element's area that overlaps with the given bbox.
/// For zero-area elements, falls back to center-point containment (returns 0.0 or 1.0).
fn element_bbox_iow(elem: &OcrElement, bbox: &BBox) -> f32 {
    let (left, top, width, height) = elem.geometry.to_aabb();
    let e_left = left as f32;
    let e_top = top as f32;
    let e_right = e_left + width as f32;
    let e_bottom = e_top + height as f32;
    let elem_area = width as f32 * height as f32;

    if elem_area <= 0.0 {
        // Zero-area element: fall back to center-point containment
        let cx = e_left + width as f32 / 2.0;
        let cy = e_top + height as f32 / 2.0;
        return if point_in_bbox(cx, cy, bbox) { 1.0 } else { 0.0 };
    }

    let inter_left = e_left.max(bbox.x1);
    let inter_top = e_top.max(bbox.y1);
    let inter_right = e_right.min(bbox.x2);
    let inter_bottom = e_bottom.min(bbox.y2);
    let inter_area = (inter_right - inter_left).max(0.0) * (inter_bottom - inter_top).max(0.0);

    inter_area / elem_area
}

/// Get element center as f32 (for matching with BBox which uses f32).
fn element_center_f32(elem: &OcrElement) -> (f32, f32) {
    let (cx, cy) = elem.geometry.center();
    (cx as f32, cy as f32)
}

/// Check if a point (cx, cy) is inside a BBox (pixel coords: y increases downward).
fn point_in_bbox(cx: f32, cy: f32, bbox: &BBox) -> bool {
    cx >= bbox.x1 && cx <= bbox.x2 && cy >= bbox.y1 && cy <= bbox.y2
}
