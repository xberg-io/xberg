//! Layout-aware OCR markdown assembly.
//!
//! Takes `OcrElement`s from any OCR backend + `DetectionResult` from layout
//! detection (both in pixel coordinates) and produces structured markdown.
//!
//! This module operates entirely in pixel space — no coordinate conversion
//! is needed because both OCR elements and layout detections use the same
//! image coordinate system (origin top-left, y increases downward).

use crate::layout::models::tatr::{self, TatrModel};
use crate::layout::types::{BBox, DetectionResult, LayoutClass, LayoutDetection};
use crate::types::OcrElement;

/// Default confidence threshold for layout detections.
const MIN_CONFIDENCE: f32 = 0.3;

/// Minimum absolute horizontal gap (in pixels) between region columns.
const MIN_COLUMN_GAP_PX: f32 = 20.0;

/// Minimum vertical extent fraction each column must span.
const MIN_COLUMN_VERTICAL_FRACTION: f32 = 0.3;

/// Y-tolerance fraction for grouping elements into the same line.
/// Applied as a fraction of the median element height.
const LINE_Y_TOLERANCE_FRACTION: f32 = 0.5;

/// A layout region with assigned OCR element indices.
struct OcrRegion<'a> {
    detection: &'a LayoutDetection,
    element_indices: Vec<usize>,
}

/// Pre-computed table markdown for a table detection region.
pub struct RecognizedTable {
    /// Detection bbox that this table corresponds to (for matching).
    pub detection_bbox: BBox,
    /// Rendered markdown table.
    pub markdown: String,
}

/// Assemble structured markdown from OCR elements using layout detection results.
///
/// Both inputs must be in the same pixel coordinate space (from the same
/// rendered page image). Returns plain text join when `detection` is `None`.
///
/// `recognized_tables` provides pre-computed markdown for Table regions
/// (from TATR or other table structure recognizer). When empty, Table
/// regions fall back to heuristic grid reconstruction from OCR elements.
pub fn assemble_ocr_markdown(
    elements: &[OcrElement],
    detection: Option<&DetectionResult>,
    img_width: u32,
    img_height: u32,
    recognized_tables: &[RecognizedTable],
) -> String {
    let detection = match detection {
        Some(d) if !d.detections.is_empty() => d,
        _ => return plain_text_join(elements),
    };

    // Page-level text quality gate: if the overall OCR text is mostly non-alphanumeric
    // (e.g., music notation, symbol-heavy content), the layout model's regions are
    // likely to produce garbled output. Fall back to plain text join.
    let all_text: String = elements.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("");
    let total_chars = all_text.chars().count();
    let alnum_chars = all_text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .count();
    if total_chars >= 20 && (alnum_chars as f32 / total_chars as f32) < 0.4 {
        tracing::trace!(
            total_chars,
            alnum_chars,
            ratio = alnum_chars as f32 / total_chars as f32,
            "OCR page text is mostly non-alphanumeric — skipping layout-guided assembly"
        );
        return plain_text_join(elements);
    }

    let (regions, unassigned) = assign_elements_to_regions(elements, &detection.detections, img_width, img_height);

    let mut ordered_regions = regions;
    order_regions_reading_order(&mut ordered_regions, img_height);

    let mut output = String::new();

    for region in &ordered_regions {
        let region_elements: Vec<&OcrElement> = region.element_indices.iter().map(|&i| &elements[i]).collect();

        let region_text = if region.detection.class == LayoutClass::Table {
            render_table_region(region.detection, &region_elements, recognized_tables)
        } else {
            if region.element_indices.is_empty() {
                continue;
            }
            render_region(&region_elements, region.detection.class)
        };

        if !region_text.is_empty() {
            if !output.is_empty() {
                output.push_str("\n\n");
            }
            output.push_str(&region_text);
        }
    }

    // Unassigned elements → plain paragraphs
    if !unassigned.is_empty() {
        let unassigned_elements: Vec<&OcrElement> = unassigned.iter().map(|&i| &elements[i]).collect();
        let unassigned_text = elements_to_paragraphs(&unassigned_elements);
        if !unassigned_text.is_empty() {
            if !output.is_empty() {
                output.push_str("\n\n");
            }
            output.push_str(&unassigned_text);
        }
    }

    output
}

/// Run TATR table recognition for all Table regions in a page.
///
/// For each Table detection, crops the page image, runs TATR inference,
/// matches OCR elements to cells, and produces markdown tables.
pub fn recognize_page_tables(
    page_image: &image::RgbImage,
    detection: &DetectionResult,
    elements: &[OcrElement],
    tatr_model: &mut TatrModel,
) -> Vec<RecognizedTable> {
    let mut tables = Vec::new();

    for det in &detection.detections {
        if det.class != LayoutClass::Table || det.confidence < MIN_CONFIDENCE {
            continue;
        }

        let md = recognize_single_table(page_image, &det.bbox, elements, tatr_model);
        if let Some(markdown) = md {
            tables.push(RecognizedTable {
                detection_bbox: det.bbox,
                markdown,
            });
        }
    }

    tables
}

/// Recognize a single table from a cropped region of the page.
fn recognize_single_table(
    page_image: &image::RgbImage,
    table_bbox: &BBox,
    elements: &[OcrElement],
    tatr_model: &mut TatrModel,
) -> Option<String> {
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
    Some(build_markdown_table(
        &cell_grid,
        &table_elements,
        crop_x as f32,
        crop_y as f32,
    ))
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
) -> String {
    if cell_grid.is_empty() {
        return String::new();
    }

    let num_cols = cell_grid[0].len();

    if num_cols == 0 {
        return String::new();
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

    md
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

/// Render a Table region: use pre-computed TATR markdown if available,
/// otherwise fall back to heuristic grid reconstruction from OCR elements.
fn render_table_region(
    detection: &LayoutDetection,
    elements: &[&OcrElement],
    recognized_tables: &[RecognizedTable],
) -> String {
    // Look for pre-computed TATR table markdown matching this detection
    for rt in recognized_tables {
        if bboxes_match(&rt.detection_bbox, &detection.bbox) {
            return rt.markdown.clone();
        }
    }

    // Fallback: heuristic grid reconstruction from OCR elements
    if elements.is_empty() {
        return String::new();
    }
    heuristic_table_from_elements(elements)
}

/// Check if two bboxes refer to the same detection (fuzzy match).
fn bboxes_match(a: &BBox, b: &BBox) -> bool {
    (a.x1 - b.x1).abs() < 1.0 && (a.y1 - b.y1).abs() < 1.0 && (a.x2 - b.x2).abs() < 1.0 && (a.y2 - b.y2).abs() < 1.0
}

/// Heuristic table reconstruction: group elements into a grid by
/// y-proximity (rows) and x-proximity (columns).
fn heuristic_table_from_elements(elements: &[&OcrElement]) -> String {
    let lines = group_elements_into_lines(elements);
    if lines.is_empty() {
        return String::new();
    }

    // If only one line, not really a table
    if lines.len() == 1 {
        let text = lines[0]
            .iter()
            .map(|e| e.text.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        return text;
    }

    // Determine column count from max elements per line
    let max_cols = lines.iter().map(|l| l.len()).max().unwrap_or(1);

    // Reject degenerate heuristic tables: too many empty-ish cells or single-row.
    // False-positive Table hints (e.g., in RTL documents) produce tables where
    // content doesn't truly form a grid, hurting TF1 with markdown table syntax.
    let total_cells = lines.iter().map(|_| max_cols).sum::<usize>();
    let filled_cells: usize = lines
        .iter()
        .map(|line| line.iter().filter(|e| !e.text.trim().is_empty()).count())
        .sum();
    let empty_cells = total_cells.saturating_sub(filled_cells);
    if total_cells > 0 && empty_cells as f64 / total_cells as f64 > 0.4 {
        // Too many empty cells — not a real table, return as plain text
        let text = elements
            .iter()
            .map(|e| e.text.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        return text;
    }

    let mut md = String::new();
    for (row_idx, line) in lines.iter().enumerate() {
        md.push('|');
        for elem in line.iter() {
            let text = elem.text.trim().replace('|', "\\|");
            md.push(' ');
            md.push_str(&text);
            md.push_str(" |");
        }
        // Fill empty cells if this row has fewer elements
        for _ in line.len()..max_cols {
            md.push_str("  |");
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

/// Assign OCR elements to layout regions using center-point containment.
///
/// For each element, finds the detection region whose bbox contains the
/// element's center point. If multiple regions overlap, smallest-area wins.
/// Picture regions suppress their elements; Table regions collect elements
/// for potential table structure recognition.
fn assign_elements_to_regions<'a>(
    elements: &[OcrElement],
    detections: &'a [LayoutDetection],
    img_width: u32,
    img_height: u32,
) -> (Vec<OcrRegion<'a>>, Vec<usize>) {
    let confident: Vec<&LayoutDetection> = detections.iter().filter(|d| d.confidence >= MIN_CONFIDENCE).collect();

    // All confident regions (including Picture and Table) collect elements.
    // Picture regions no longer suppress text — their elements are assigned
    // normally but rendered as plain paragraphs, preventing silent text loss.
    let active_dets: Vec<&LayoutDetection> = confident;

    let mut regions: Vec<OcrRegion> = active_dets
        .iter()
        .map(|det| OcrRegion {
            detection: det,
            element_indices: Vec::new(),
        })
        .collect();

    let areas: Vec<f32> = active_dets.iter().map(|d| d.bbox.area()).collect();

    let mut unassigned: Vec<usize> = Vec::new();

    for (elem_idx, elem) in elements.iter().enumerate() {
        if elem.text.trim().is_empty() {
            continue;
        }

        let (cx, cy) = element_center_f32(elem);

        // Check if outside page bounds (unlikely but defensive)
        if cx < 0.0 || cy < 0.0 || cx > img_width as f32 || cy > img_height as f32 {
            continue;
        }

        // Find smallest containing region
        let mut best_idx: Option<usize> = None;
        let mut best_area = f32::MAX;

        for (ri, det) in active_dets.iter().enumerate() {
            if point_in_bbox(cx, cy, &det.bbox) && areas[ri] < best_area {
                best_area = areas[ri];
                best_idx = Some(ri);
            }
        }

        match best_idx {
            Some(ri) => regions[ri].element_indices.push(elem_idx),
            None => unassigned.push(elem_idx),
        }
    }

    (regions, unassigned)
}

/// Sort regions in reading order: column-aware if multi-column detected,
/// otherwise top-to-bottom with same-row left-to-right.
fn order_regions_reading_order(regions: &mut [OcrRegion], img_height: u32) {
    if let Some(split_x) = detect_column_split(regions) {
        regions.sort_by(|a, b| {
            let (a_cx, _) = a.detection.bbox.center();
            let (b_cx, _) = b.detection.bbox.center();
            let a_col = if a_cx < split_x { 0u8 } else { 1 };
            let b_col = if b_cx < split_x { 0u8 } else { 1 };

            if a_col != b_col {
                return a_col.cmp(&b_col);
            }

            // Same column: lower y1 = higher on page = comes first (pixel coords)
            a.detection.bbox.y1.total_cmp(&b.detection.bbox.y1)
        });
    } else {
        let y_tolerance = (img_height as f32 * 0.02).max(1.0);

        // Quantize y-centers into discrete rows to ensure transitive ordering.
        // A tolerance-based comparator (|a-b| < tol → Equal) is non-transitive.
        regions.sort_by(|a, b| {
            let (_, a_cy) = a.detection.bbox.center();
            let (_, b_cy) = b.detection.bbox.center();
            let a_row = (a_cy / y_tolerance) as i64;
            let b_row = (b_cy / y_tolerance) as i64;

            a_row
                .cmp(&b_row)
                .then_with(|| a.detection.bbox.x1.total_cmp(&b.detection.bbox.x1))
        });
    }
}

/// Detect if regions form two distinct columns. Returns split x-position.
fn detect_column_split(regions: &[OcrRegion]) -> Option<f32> {
    // Filter out page furniture for column detection
    let content_regions: Vec<&OcrRegion> = regions
        .iter()
        .filter(|r| !matches!(r.detection.class, LayoutClass::PageHeader | LayoutClass::PageFooter))
        .collect();

    if content_regions.len() < 4 {
        return None;
    }

    let mut edges: Vec<(f32, f32)> = content_regions
        .iter()
        .map(|r| (r.detection.bbox.x1, r.detection.bbox.x2))
        .collect();
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut max_right = f32::MIN;
    let mut best_gap = 0.0_f32;
    let mut best_split: Option<f32> = None;

    for &(left, right) in &edges {
        if max_right > f32::MIN {
            let gap = left - max_right;
            if gap > best_gap {
                best_gap = gap;
                best_split = Some((max_right + left) / 2.0);
            }
        }
        max_right = max_right.max(right);
    }

    if best_gap < MIN_COLUMN_GAP_PX {
        return None;
    }

    let split_x = best_split?;

    // Validate both sides have >=2 regions
    let left_count = content_regions
        .iter()
        .filter(|r| r.detection.bbox.center().0 < split_x)
        .count();
    let right_count = content_regions
        .iter()
        .filter(|r| r.detection.bbox.center().0 >= split_x)
        .count();

    if left_count < 2 || right_count < 2 {
        return None;
    }

    // Validate both columns span significant vertical extent
    let y_min = content_regions
        .iter()
        .map(|r| r.detection.bbox.y1)
        .fold(f32::MAX, f32::min);
    let y_max = content_regions
        .iter()
        .map(|r| r.detection.bbox.y2)
        .fold(f32::MIN, f32::max);
    let y_span = y_max - y_min;

    if y_span < 1.0 {
        return None;
    }

    let left_y_span = {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for r in content_regions.iter().filter(|r| r.detection.bbox.center().0 < split_x) {
            lo = lo.min(r.detection.bbox.y1);
            hi = hi.max(r.detection.bbox.y2);
        }
        hi - lo
    };
    let right_y_span = {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for r in content_regions
            .iter()
            .filter(|r| r.detection.bbox.center().0 >= split_x)
        {
            lo = lo.min(r.detection.bbox.y1);
            hi = hi.max(r.detection.bbox.y2);
        }
        hi - lo
    };

    if left_y_span < y_span * MIN_COLUMN_VERTICAL_FRACTION || right_y_span < y_span * MIN_COLUMN_VERTICAL_FRACTION {
        return None;
    }

    Some(split_x)
}

/// Render a region's elements as markdown based on its layout class.
fn render_region(elements: &[&OcrElement], class: LayoutClass) -> String {
    match class {
        LayoutClass::Title => {
            let text = join_element_texts(elements);
            if text.is_empty() {
                return String::new();
            }
            format!("# {text}")
        }
        LayoutClass::SectionHeader => {
            let text = join_element_texts(elements);
            if text.is_empty() {
                return String::new();
            }
            format!("## {text}")
        }
        LayoutClass::Code => {
            let text = join_element_texts_preserving_lines(elements);
            if text.is_empty() {
                return String::new();
            }
            format!("```\n{text}\n```")
        }
        LayoutClass::Formula => {
            let text = join_element_texts(elements);
            if text.is_empty() {
                return String::new();
            }
            format!("$${text}$$")
        }
        LayoutClass::ListItem => {
            // Each element line becomes a list item
            let lines = group_elements_into_lines(elements);
            let mut result = String::new();
            for line in &lines {
                let text: String = line.iter().map(|e| e.text.trim()).collect::<Vec<_>>().join(" ");
                if !text.is_empty() {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str("- ");
                    result.push_str(&text);
                }
            }
            result
        }
        LayoutClass::PageHeader | LayoutClass::PageFooter => {
            // Skip page furniture
            String::new()
        }
        LayoutClass::Picture => {
            // Picture regions: render any text as plain paragraphs
            // (diagrams/figures may contain embedded text worth preserving)
            elements_to_paragraphs(elements)
        }
        LayoutClass::Caption => {
            let text = join_element_texts(elements);
            if text.is_empty() {
                return String::new();
            }
            format!("*{text}*")
        }
        LayoutClass::Footnote => join_element_texts(elements),
        // Text, DocumentIndex, Form, KeyValueRegion, CheckboxSelected/Unselected, and any other
        _ => elements_to_paragraphs(elements),
    }
}

/// Join element texts into a single line, space-separated.
fn join_element_texts(elements: &[&OcrElement]) -> String {
    let lines = group_elements_into_lines(elements);
    lines
        .iter()
        .map(|line| {
            line.iter()
                .map(|e| e.text.trim())
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Join element texts preserving line structure (for code blocks).
fn join_element_texts_preserving_lines(elements: &[&OcrElement]) -> String {
    let lines = group_elements_into_lines(elements);
    lines
        .iter()
        .map(|line| line.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Assemble elements into paragraph text with blank-line separation.
fn elements_to_paragraphs(elements: &[&OcrElement]) -> String {
    let lines = group_elements_into_lines(elements);

    if lines.is_empty() {
        return String::new();
    }

    // Group lines into paragraphs by vertical gap
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current_para_lines: Vec<String> = Vec::new();
    let mut prev_line_bottom: Option<f32> = None;

    // Compute median line height for gap detection
    let line_heights: Vec<f32> = lines
        .iter()
        .map(|line| {
            let min_y = line
                .iter()
                .map(|e| {
                    let (_, top, _, _) = e.geometry.to_aabb();
                    top as f32
                })
                .fold(f32::MAX, f32::min);
            let max_y = line
                .iter()
                .map(|e| {
                    let (_, top, _, h) = e.geometry.to_aabb();
                    (top + h) as f32
                })
                .fold(f32::MIN, f32::max);
            (max_y - min_y).max(1.0)
        })
        .collect();

    let median_height = if !line_heights.is_empty() {
        let mut sorted = line_heights.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        sorted[sorted.len() / 2]
    } else {
        20.0
    };

    for line in &lines {
        let line_text: String = line
            .iter()
            .map(|e| e.text.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        if line_text.is_empty() {
            continue;
        }

        let line_top = line
            .iter()
            .map(|e| {
                let (_, top, _, _) = e.geometry.to_aabb();
                top as f32
            })
            .fold(f32::MAX, f32::min);

        let line_bottom = line
            .iter()
            .map(|e| {
                let (_, top, _, h) = e.geometry.to_aabb();
                (top + h) as f32
            })
            .fold(f32::MIN, f32::max);

        // Detect paragraph break: gap > 1.5x median line height
        if let Some(prev_bottom) = prev_line_bottom {
            let gap = line_top - prev_bottom;
            if gap > median_height * 1.5 && !current_para_lines.is_empty() {
                paragraphs.push(current_para_lines.join(" "));
                current_para_lines = Vec::new();
            }
        }

        current_para_lines.push(line_text);
        prev_line_bottom = Some(line_bottom);
    }

    if !current_para_lines.is_empty() {
        paragraphs.push(current_para_lines.join(" "));
    }

    paragraphs.join("\n\n")
}

/// Group OCR elements into lines by y-proximity.
///
/// Elements whose vertical centers are within `LINE_Y_TOLERANCE_FRACTION`
/// of the median element height are grouped into the same line.
/// Lines are sorted top-to-bottom, elements within a line left-to-right.
fn group_elements_into_lines<'a>(elements: &[&'a OcrElement]) -> Vec<Vec<&'a OcrElement>> {
    if elements.is_empty() {
        return Vec::new();
    }

    // Compute median element height for tolerance
    let mut heights: Vec<f32> = elements
        .iter()
        .map(|e| {
            let (_, _, _, h) = e.geometry.to_aabb();
            h as f32
        })
        .filter(|h| *h > 0.0)
        .collect();

    let median_height = if !heights.is_empty() {
        heights.sort_by(|a, b| a.total_cmp(b));
        heights[heights.len() / 2]
    } else {
        20.0
    };

    let tolerance = median_height * LINE_Y_TOLERANCE_FRACTION;

    // Sort elements by y-center, then x-left
    let mut sorted: Vec<(usize, f32, f32)> = elements
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let (_, cy) = e.geometry.center();
            let (left, _, _, _) = e.geometry.to_aabb();
            (i, cy as f32, left as f32)
        })
        .collect();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.2.total_cmp(&b.2)));

    let mut lines: Vec<Vec<&OcrElement>> = Vec::new();
    let mut current_line: Vec<(usize, f32)> = Vec::new(); // (elem_idx, x_left)
    let mut line_y_sum: f32 = 0.0;

    for (elem_idx, cy, x_left) in sorted {
        if current_line.is_empty() {
            current_line.push((elem_idx, x_left));
            line_y_sum = cy;
        } else {
            let avg_y = line_y_sum / current_line.len() as f32;
            if (cy - avg_y).abs() <= tolerance {
                current_line.push((elem_idx, x_left));
                line_y_sum += cy;
            } else {
                // Finalize current line
                current_line.sort_by(|a, b| a.1.total_cmp(&b.1));
                lines.push(current_line.iter().map(|(i, _)| elements[*i]).collect());
                current_line = vec![(elem_idx, x_left)];
                line_y_sum = cy;
            }
        }
    }

    if !current_line.is_empty() {
        current_line.sort_by(|a, b| a.1.total_cmp(&b.1));
        lines.push(current_line.iter().map(|(i, _)| elements[*i]).collect());
    }

    lines
}

/// Plain text join: concatenate all element texts with spaces.
fn plain_text_join(elements: &[OcrElement]) -> String {
    let refs: Vec<&OcrElement> = elements.iter().collect();
    elements_to_paragraphs(&refs)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::types::{BBox, DetectionResult, LayoutClass, LayoutDetection};
    use crate::types::{OcrBoundingGeometry, OcrConfidence, OcrElement};

    fn make_element(text: &str, left: u32, top: u32, width: u32, height: u32) -> OcrElement {
        OcrElement::new(
            text,
            OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            },
            OcrConfidence::from_tesseract(90.0),
        )
    }

    fn make_detection(class: LayoutClass, x1: f32, y1: f32, x2: f32, y2: f32) -> LayoutDetection {
        LayoutDetection::new(class, 0.9, BBox::new(x1, y1, x2, y2))
    }

    #[test]
    fn test_no_detections_returns_plain_text() {
        let elements = vec![
            make_element("Hello", 10, 10, 80, 20),
            make_element("World", 10, 40, 80, 20),
        ];
        let result = assemble_ocr_markdown(&elements, None, 800, 600, &[]);
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
    }

    #[test]
    fn test_title_region_produces_h1() {
        let elements = vec![make_element("Document Title", 50, 30, 300, 40)];
        let detection = DetectionResult::new(
            800,
            600,
            vec![make_detection(LayoutClass::Title, 40.0, 20.0, 400.0, 80.0)],
        );
        let result = assemble_ocr_markdown(&elements, Some(&detection), 800, 600, &[]);
        assert_eq!(result, "# Document Title");
    }

    #[test]
    fn test_section_header_produces_h2() {
        let elements = vec![make_element("Introduction", 50, 100, 200, 30)];
        let detection = DetectionResult::new(
            800,
            600,
            vec![make_detection(LayoutClass::SectionHeader, 40.0, 90.0, 300.0, 140.0)],
        );
        let result = assemble_ocr_markdown(&elements, Some(&detection), 800, 600, &[]);
        assert_eq!(result, "## Introduction");
    }

    #[test]
    fn test_code_region_produces_fenced_block() {
        let elements = vec![
            make_element("fn main() {", 50, 100, 200, 20),
            make_element("println!(\"hello\");", 50, 125, 200, 20),
            make_element("}", 50, 150, 30, 20),
        ];
        let detection = DetectionResult::new(
            800,
            600,
            vec![make_detection(LayoutClass::Code, 40.0, 90.0, 300.0, 180.0)],
        );
        let result = assemble_ocr_markdown(&elements, Some(&detection), 800, 600, &[]);
        assert!(result.starts_with("```\n"));
        assert!(result.ends_with("\n```"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_table_region_heuristic_markdown() {
        let elements = vec![
            make_element("Body text", 50, 30, 200, 20),
            make_element("Header1", 50, 190, 100, 20),
            make_element("Header2", 200, 190, 100, 20),
            make_element("cell1", 50, 220, 100, 20),
            make_element("cell2", 200, 220, 100, 20),
        ];
        let detection = DetectionResult::new(
            800,
            600,
            vec![
                make_detection(LayoutClass::Text, 40.0, 20.0, 300.0, 60.0),
                make_detection(LayoutClass::Table, 40.0, 180.0, 350.0, 250.0),
            ],
        );
        let result = assemble_ocr_markdown(&elements, Some(&detection), 800, 600, &[]);
        assert!(result.contains("Body text"));
        // Table region produces heuristic markdown table from OCR elements
        assert!(result.contains("Header1"));
        assert!(result.contains("cell1"));
        assert!(result.contains("|")); // markdown table format
    }

    #[test]
    fn test_table_region_with_recognized_table() {
        let elements = vec![
            make_element("Body text", 50, 30, 200, 20),
            make_element("cell1", 50, 200, 100, 20),
            make_element("cell2", 200, 200, 100, 20),
        ];
        let detection = DetectionResult::new(
            800,
            600,
            vec![
                make_detection(LayoutClass::Text, 40.0, 20.0, 300.0, 60.0),
                make_detection(LayoutClass::Table, 40.0, 180.0, 350.0, 250.0),
            ],
        );
        let recognized = vec![RecognizedTable {
            detection_bbox: BBox::new(40.0, 180.0, 350.0, 250.0),
            markdown: "| Header1 | Header2 |\n| --- | --- |\n| A | B |".to_string(),
        }];
        let result = assemble_ocr_markdown(&elements, Some(&detection), 800, 600, &recognized);
        assert!(result.contains("Body text"));
        assert!(result.contains("| Header1 | Header2 |"));
        assert!(result.contains("| A | B |"));
    }

    #[test]
    fn test_furniture_skipped() {
        let elements = vec![
            make_element("Page 1", 350, 10, 80, 15),
            make_element("Body content", 50, 300, 300, 20),
        ];
        let detection = DetectionResult::new(
            800,
            600,
            vec![
                make_detection(LayoutClass::PageHeader, 300.0, 0.0, 500.0, 30.0),
                make_detection(LayoutClass::Text, 40.0, 290.0, 400.0, 330.0),
            ],
        );
        let result = assemble_ocr_markdown(&elements, Some(&detection), 800, 600, &[]);
        assert!(!result.contains("Page 1"));
        assert!(result.contains("Body content"));
    }

    #[test]
    fn test_mixed_regions() {
        let elements = vec![
            make_element("My Report", 100, 30, 200, 40),
            make_element("Background", 50, 120, 150, 25),
            make_element("This is the first paragraph.", 50, 200, 400, 20),
        ];
        let detection = DetectionResult::new(
            800,
            600,
            vec![
                make_detection(LayoutClass::Title, 80.0, 20.0, 350.0, 80.0),
                make_detection(LayoutClass::SectionHeader, 40.0, 110.0, 250.0, 155.0),
                make_detection(LayoutClass::Text, 40.0, 190.0, 500.0, 230.0),
            ],
        );
        let result = assemble_ocr_markdown(&elements, Some(&detection), 800, 600, &[]);
        assert!(result.contains("# My Report"));
        assert!(result.contains("## Background"));
        assert!(result.contains("This is the first paragraph."));
    }

    #[test]
    fn test_list_items() {
        let elements = vec![
            make_element("First item", 70, 100, 200, 20),
            make_element("Second item", 70, 130, 200, 20),
        ];
        let detection = DetectionResult::new(
            800,
            600,
            vec![make_detection(LayoutClass::ListItem, 50.0, 90.0, 300.0, 160.0)],
        );
        let result = assemble_ocr_markdown(&elements, Some(&detection), 800, 600, &[]);
        assert!(result.contains("- First item"));
        assert!(result.contains("- Second item"));
    }

    #[test]
    fn test_point_in_bbox() {
        let bbox = BBox::new(10.0, 20.0, 100.0, 80.0);
        assert!(super::point_in_bbox(50.0, 50.0, &bbox));
        assert!(!super::point_in_bbox(5.0, 50.0, &bbox));
        assert!(!super::point_in_bbox(50.0, 90.0, &bbox));
    }
}
