//! Bridge between pdfium extraction APIs and the markdown pipeline.
//!
//! Two conversion paths:
//! 1. Structure tree: `ExtractedBlock` → `PdfParagraph` (for tagged PDFs)
//! 2. Page objects: `PdfPage` → `(Vec<SegmentData>, Vec<ImagePosition>)` (heuristic extraction)
//!
//! The page objects path uses a Segment-based algorithm as its primary extraction:
//! pdfium segments are grouped into rows, merged horizontally, and text is
//! re-extracted from merged bounding boxes via `page.text().inside_rect()`.
//! This produces correct word boundaries (pdfium reassembles fragmented words
//! like "soft"+"ware" into "software" within bounded rects) and naturally
//! excludes sidebar text through tight per-group bboxes.
//!
//! Falls back to character-level extraction with column detection when the
//! segment-based path produces no results, and to the page objects API when
//! `page.text()` fails entirely.

use std::borrow::Cow;

use crate::pdf::hierarchy::SegmentData;
use crate::pdf::text_data::ExtractedSegment;
use pdfium_render::prelude::*;

use super::text_repair::{apply_ligature_repairs, build_ligature_repair_map, normalize_text_encoding};
use super::types::PdfParagraph;
use crate::pdf::text_data::{PageTextData, extract_page_text_data};

// Alias to distinguish from our local PdfParagraph type.
use pdfium_render::prelude::PdfParagraph as PdfiumParagraph;

/// Position and metadata of an image detected during object-based extraction.
#[derive(Debug, Clone)]
pub(super) struct ImagePosition {
    /// 1-indexed page number.
    pub page_number: usize,
    /// Global image index across the document.
    pub image_index: usize,
}

/// Filter sidebar artifacts from structure tree extracted blocks.
///
/// Removes blocks that appear to be sidebar text (e.g., arXiv identifiers
/// rendered vertically along page margins). Detection criteria:
/// - Block has bounds in the leftmost or rightmost margin (< 8% or > 92% of page width)
/// - Block text is very short (≤ 3 characters trimmed)
/// - At least 3 such blocks exist (to avoid false positives on legitimate margin content)
pub(super) fn filter_sidebar_blocks(blocks: &[ExtractedBlock], page_width: f32) -> Cow<'_, [ExtractedBlock]> {
    if page_width <= 0.0 {
        return Cow::Borrowed(blocks);
    }

    let left_cutoff = page_width * 0.08;
    let right_cutoff = page_width * 0.92;

    // Count short-text blocks in margins
    let sidebar_count = count_sidebar_blocks(blocks, left_cutoff, right_cutoff);

    if sidebar_count < 3 {
        return Cow::Borrowed(blocks);
    }

    // Filter them out
    Cow::Owned(filter_blocks_recursive(blocks, left_cutoff, right_cutoff))
}

fn count_sidebar_blocks(blocks: &[ExtractedBlock], left_cutoff: f32, right_cutoff: f32) -> usize {
    let mut count = 0;
    for block in blocks {
        if !block.children.is_empty() {
            count += count_sidebar_blocks(&block.children, left_cutoff, right_cutoff);
        } else if is_sidebar_block(block, left_cutoff, right_cutoff) {
            count += 1;
        }
    }
    count
}

fn is_sidebar_block(block: &ExtractedBlock, left_cutoff: f32, right_cutoff: f32) -> bool {
    let trimmed = block.text.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 3 {
        return false;
    }
    if let Some(bounds) = &block.bounds {
        let left = bounds.left().value;
        let right = bounds.right().value;
        // Block is entirely within left or right margin
        right < left_cutoff || left > right_cutoff
    } else {
        false
    }
}

fn filter_blocks_recursive(blocks: &[ExtractedBlock], left_cutoff: f32, right_cutoff: f32) -> Vec<ExtractedBlock> {
    blocks
        .iter()
        .filter_map(|block| {
            if !block.children.is_empty() {
                let filtered_children = filter_blocks_recursive(&block.children, left_cutoff, right_cutoff);
                if filtered_children.is_empty() {
                    return None;
                }
                Some(ExtractedBlock {
                    children: filtered_children,
                    ..block.clone()
                })
            } else if is_sidebar_block(block, left_cutoff, right_cutoff) {
                None
            } else {
                Some(block.clone())
            }
        })
        .collect()
}

/// Convert extracted blocks from the structure tree API into PdfParagraphs.
///
/// Converts via the unified DTO path:
/// `ExtractedBlock` → `PageContent` (via `adapters::from_structure_tree`) →
/// `Vec<PdfParagraph>` (via `content_convert::content_to_paragraphs`).
pub(super) fn extracted_blocks_to_paragraphs(blocks: &[ExtractedBlock]) -> Vec<PdfParagraph> {
    let page_content = super::adapters::from_structure_tree(blocks);
    super::content_convert::content_to_paragraphs(&page_content)
}

/// Extract text segments and image positions from a PDF page.
///
/// Primary path: Segment-based extraction using pdfium's segment API with
/// row grouping, cell merging, and text re-extraction from merged bounding
/// boxes. This produces correct word boundaries (pdfium reassembles fragmented
/// words like "soft"+"ware" into "software" within bounded rects).
///
/// Fallback: character-level extraction via `PageTextData` DTO when the
/// segment-based path produces no results.
///
/// Last resort: page objects API with column detection when `page.text()` fails.
///
/// Also detects image objects and records their positions for interleaving.
pub(super) fn objects_to_page_data(
    page: &PdfPage,
    page_number: usize,
    image_offset: &mut usize,
) -> (Vec<SegmentData>, Vec<ImagePosition>) {
    let objects: Vec<PdfPageObject> = page.objects().iter().collect();

    // Image scan BEFORE text extraction.
    let mut images = Vec::new();
    for obj in &objects {
        if obj.as_image_object().is_some() {
            images.push(ImagePosition {
                page_number,
                image_index: *image_offset,
            });
            *image_offset += 1;
        }
    }

    let page_height = page.height().value;
    let page_width = page.width().value;

    // Best-of-two extraction: run both segment-from-DTO and segment-merged,
    // compare token counts against the full page text, pick the one that
    // preserves more content. Prefer DTO when counts are close (better word
    // boundaries, no \x02 markers).
    if let Some(data) = extract_page_text_data(page) {
        let ref_tokens = data.full_text.split_whitespace().count();

        let dto_result = extract_segments_from_dto(&data, page_width);
        let merged_result = extract_segments_merged(page, page_height);

        let dto_tokens = dto_result.as_ref().map_or(0, |segs| {
            segs.iter().map(|s| s.text.split_whitespace().count()).sum::<usize>()
        });
        let merged_tokens = merged_result.as_ref().map_or(0, |segs| {
            segs.iter().map(|s| s.text.split_whitespace().count()).sum::<usize>()
        });

        // Pick the result whose token count is closer to the reference.
        // If DTO is within 10% of merged, prefer DTO (better word boundaries).
        let best = if ref_tokens == 0 {
            dto_result.or(merged_result)
        } else {
            let dto_diff = (dto_tokens as f64 - ref_tokens as f64).abs();
            let merged_diff = (merged_tokens as f64 - ref_tokens as f64).abs();

            if dto_result.is_some() && merged_result.is_some() {
                // Both available: pick closer to reference, prefer DTO on ties
                let dto_within_10pct = dto_diff <= merged_diff * 1.1;
                if dto_within_10pct { dto_result } else { merged_result }
            } else {
                // Only one available
                dto_result.or(merged_result)
            }
        };

        if let Some(segments) = best {
            return (segments, images);
        }

        // Fallback: char-level with column detection.
        if let Some(segments) = chars_to_segments_from_data(&data, page_width) {
            return (segments, images);
        }
    }

    // Last resort: page objects API with column detection.
    // Used when page.text() fails entirely (rare edge case).
    let mut segments = Vec::new();
    let column_groups = super::columns::split_objects_into_columns(&objects);
    let column_vecs = partition_objects_by_columns(objects, &column_groups);
    for column_objects in &column_vecs {
        let paragraphs: Vec<PdfiumParagraph> = PdfiumParagraph::from_objects(column_objects);
        extract_paragraphs_to_segments(paragraphs, &mut segments);
    }

    // Apply ligature repair for last-resort path.
    if let Some(repair_map) = build_ligature_repair_map(page) {
        for seg in &mut segments {
            seg.text = apply_ligature_repairs(&seg.text, &repair_map);
        }
    }

    (segments, images)
}

// ── Segment-based segment extraction ──

/// A text cell extracted from pdfium's segment API, with coordinates
/// converted to page top-left origin for row grouping.
struct TextCell {
    text: String,
    /// Left edge in PDF coordinates (bottom-left origin).
    pdf_left: f32,
    /// Bottom edge in PDF coordinates (bottom-left origin).
    pdf_bottom: f32,
    /// Right edge in PDF coordinates (bottom-left origin).
    pdf_right: f32,
    /// Top edge in PDF coordinates (bottom-left origin).
    pdf_top: f32,
    /// Top edge in page top-left coordinate system.
    top: f32,
    /// Bottom edge in page top-left coordinate system.
    bottom: f32,
    /// Font size in points.
    font_size: f32,
    /// Whether the font is bold.
    is_bold: bool,
    /// Whether the font is italic.
    is_italic: bool,
    /// Whether the font is monospace.
    is_monospace: bool,
    /// Baseline Y in PDF coordinates (bottom-left origin).
    baseline_y: f32,
}

/// A row of text cells sharing approximately the same vertical position.
struct TextRow {
    cells: Vec<TextCell>,
    /// Top edge of the row (page top-left coordinates).
    top: f32,
    /// Bottom edge of the row (page top-left coordinates).
    bottom: f32,
}

impl TextRow {
    fn height(&self) -> f32 {
        (self.bottom - self.top).abs()
    }
}

/// A group of merged cells within a row, potentially requiring text re-extraction.
struct MergedCellGroup {
    cells: Vec<TextCell>,
    /// Merged left edge in PDF coordinates.
    pdf_left: f32,
    /// Merged bottom edge in PDF coordinates.
    pdf_bottom: f32,
    /// Merged right edge in PDF coordinates.
    pdf_right: f32,
    /// Merged top edge in PDF coordinates.
    pdf_top: f32,
}

/// Segment-based text extraction from a PDF page.
///
/// Implements a segment-based cell-merging extraction algorithm:
/// 1. Extract text rects from pdfium's segment API
/// 2. Group cells into rows (vertical_threshold = 0.5)
/// 3. Merge adjacent cells within rows (horizontal_threshold = 1.0)
/// 4. Re-extract text from merged bboxes using `page.text().inside_rect()`
/// 5. Convert to `SegmentData`
///
/// The re-extraction step is the key: pdfium re-assembles fragmented words
/// (e.g., "soft" + "ware" becomes "software") when given a bounding rect
/// that spans both fragments. Tight per-group bboxes naturally exclude
/// sidebar text without explicit filtering.
fn extract_segments_merged(page: &PdfPage, page_height: f32) -> Option<Vec<SegmentData>> {
    let text_obj = page.text().ok()?;
    let pdfium_segments = text_obj.segments();
    let seg_count = pdfium_segments.len();
    if seg_count == 0 {
        return None;
    }

    // Step 1: Extract text rects into cells.
    let mut cells: Vec<TextCell> = Vec::with_capacity(seg_count);
    for i in 0..seg_count {
        let seg = match pdfium_segments.get(i) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let text = seg.text();
        if text.trim().is_empty() {
            continue;
        }
        let bounds = seg.bounds();
        let pdf_left = bounds.left().value;
        let pdf_bottom = bounds.bottom().value;
        let pdf_right = bounds.right().value;
        let pdf_top = bounds.top().value;

        // Convert from PDF bottom-left origin to page top-left origin.
        let top = page_height - pdf_top;
        let bottom = page_height - pdf_bottom;

        // Sample font properties from the first non-whitespace character.
        let (font_size, is_bold, is_italic, is_monospace, baseline_y) = sample_font_from_segment(&seg);

        cells.push(TextCell {
            text,
            pdf_left,
            pdf_bottom,
            pdf_right,
            pdf_top,
            top,
            bottom,
            font_size,
            is_bold,
            is_italic,
            is_monospace,
            baseline_y,
        });
    }

    if cells.is_empty() {
        return None;
    }

    // Filter sidebar cells: cells whose right edge is within 5% of page width
    // from the left margin are likely rotated sidebar text (e.g., arXiv IDs).
    let page_width = page.width().value;
    let sidebar_cutoff = page_width * 0.05;
    cells.retain(|c| c.pdf_right > sidebar_cutoff);

    if cells.is_empty() {
        return None;
    }

    // Step 2: Group cells into rows.
    let rows = group_cells_into_rows(cells);

    // Step 3 & 4: Merge adjacent cells within rows, re-extract text from merged bboxes.
    let mut segments = Vec::new();
    for row in rows {
        let merged_groups = merge_cells_in_row(row);
        for group in merged_groups {
            // Step 4: Re-extract text from merged bbox.
            let text = if group.cells.len() == 1 {
                // Single cell: use text as-is.
                group.cells[0].text.clone()
            } else {
                // Multi-cell group: re-extract from merged bbox using pdfium.
                // The bbox is in PDF coordinates (bottom-left origin).
                let rect = PdfRect::new_from_values(group.pdf_bottom, group.pdf_left, group.pdf_top, group.pdf_right);
                let reextracted = text_obj.inside_rect(rect);
                if reextracted.trim().is_empty() {
                    // Fallback: concatenate individual cell texts.
                    group.cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("")
                } else {
                    reextracted
                }
            };

            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Step 5: Convert to SegmentData.
            // Use the first cell's font info as representative.
            let first = &group.cells[0];
            let width = group.pdf_right - group.pdf_left;
            let height = group.pdf_top - group.pdf_bottom;

            segments.push(SegmentData {
                text: trimmed.to_string(),
                x: group.pdf_left,
                y: first.baseline_y,
                width: width.max(first.font_size),
                height: height.max(first.font_size),
                font_size: first.font_size,
                is_bold: first.is_bold,
                is_italic: first.is_italic,
                is_monospace: first.is_monospace,
                baseline_y: first.baseline_y,
            });
        }
    }

    if segments.is_empty() { None } else { Some(segments) }
}

/// Sample font properties from a pdfium text segment's first non-whitespace character.
///
/// Returns (font_size, is_bold, is_italic, is_monospace, baseline_y).
fn sample_font_from_segment(seg: &pdfium_render::prelude::PdfPageTextSegment<'_>) -> (f32, bool, bool, bool, f32) {
    let bounds = seg.bounds();
    let default_baseline = bounds.bottom().value;

    if let Ok(seg_chars) = seg.chars() {
        for ch in seg_chars.iter() {
            let uv = ch.unicode_value();
            if let Some(uc) = char::from_u32(uv)
                && uc.is_whitespace()
            {
                continue;
            }
            let scaled = ch.scaled_font_size().value;
            let fs = if scaled > 0.0 { scaled } else { 12.0 };
            let info = ch.font_info();
            let mono = ch.font_is_fixed_pitch();
            let bl_y = ch.origin().map(|o| o.1.value).unwrap_or(default_baseline);
            return (fs, info.1, info.2, mono, bl_y);
        }
    }

    (12.0, false, false, false, default_baseline)
}

/// Group cells into rows based on vertical proximity.
///
/// A cell belongs to the current row if its top and bottom
/// are both within `row_height * vertical_threshold` of the row's top and bottom.
/// `vertical_threshold = 0.5` (half the row height).
fn group_cells_into_rows(cells: Vec<TextCell>) -> Vec<TextRow> {
    const VERTICAL_THRESHOLD: f32 = 0.5;

    let mut rows: Vec<TextRow> = Vec::new();

    for cell in cells {
        let cell_top = cell.top;
        let cell_bottom = cell.bottom;
        // Find matching row index.
        let matching_row = rows.iter().position(|row| {
            let row_h = row.height().max(1.0);
            let tolerance = row_h * VERTICAL_THRESHOLD;
            (cell_top - row.top).abs() <= tolerance && (cell_bottom - row.bottom).abs() <= tolerance
        });
        if let Some(idx) = matching_row {
            rows[idx].top = rows[idx].top.min(cell_top);
            rows[idx].bottom = rows[idx].bottom.max(cell_bottom);
            rows[idx].cells.push(cell);
        } else {
            rows.push(TextRow {
                cells: vec![cell],
                top: cell_top,
                bottom: cell_bottom,
            });
        }
    }

    // Sort rows top-to-bottom (ascending top in page top-left coordinates).
    rows.sort_by(|a, b| a.top.partial_cmp(&b.top).unwrap_or(std::cmp::Ordering::Equal));
    rows
}

/// Merge adjacent cells within a row based on horizontal proximity.
///
/// Cells are sorted left-to-right. If the gap between
/// consecutive cells is <= `avg_height * horizontal_threshold`, they are
/// merged into one group. `horizontal_threshold = 1.0`.
fn merge_cells_in_row(mut row: TextRow) -> Vec<MergedCellGroup> {
    const HORIZONTAL_THRESHOLD: f32 = 1.0;

    // Sort cells left-to-right by their left edge (PDF coordinates).
    row.cells
        .sort_by(|a, b| a.pdf_left.partial_cmp(&b.pdf_left).unwrap_or(std::cmp::Ordering::Equal));

    // Compute average height across all cells in the row.
    let avg_height = if row.cells.is_empty() {
        12.0
    } else {
        row.cells.iter().map(|c| (c.pdf_top - c.pdf_bottom).abs()).sum::<f32>() / row.cells.len() as f32
    };
    let merge_threshold = avg_height * HORIZONTAL_THRESHOLD;

    let mut groups: Vec<MergedCellGroup> = Vec::new();

    for cell in row.cells {
        let should_merge = if let Some(last_group) = groups.last() {
            let gap = cell.pdf_left - last_group.pdf_right;
            gap <= merge_threshold
        } else {
            false
        };

        if should_merge {
            let group = groups.last_mut().unwrap();
            group.pdf_left = group.pdf_left.min(cell.pdf_left);
            group.pdf_bottom = group.pdf_bottom.min(cell.pdf_bottom);
            group.pdf_right = group.pdf_right.max(cell.pdf_right);
            group.pdf_top = group.pdf_top.max(cell.pdf_top);
            group.cells.push(cell);
        } else {
            groups.push(MergedCellGroup {
                pdf_left: cell.pdf_left,
                pdf_bottom: cell.pdf_bottom,
                pdf_right: cell.pdf_right,
                pdf_top: cell.pdf_top,
                cells: vec![cell],
            });
        }
    }

    groups
}

// ── Segment-level extraction from DTO ──

/// A row of extracted segments sharing approximately the same vertical position.
struct SegmentRow {
    segments: Vec<ExtractedSegment>,
    top: f32,
    bottom: f32,
}

/// Extract text segments from pre-extracted DTO data.
///
/// Uses pdfium's segment-level text (no character-level reconstruction),
/// groups segments into rows, and concatenates with geometric space detection.
/// Each segment's text comes directly from pdfium — no `inside_rect()`
/// re-extraction on merged bboxes, avoiding `\x02` markers and cross-line breaks.
fn extract_segments_from_dto(data: &PageTextData, page_width: f32) -> Option<Vec<SegmentData>> {
    if data.segments.is_empty() {
        return None;
    }

    // Filter sidebar segments.
    let sidebar_cutoff = page_width * 0.06;
    let filtered: Vec<&ExtractedSegment> = data
        .segments
        .iter()
        .filter(|s| (s.left + s.right) * 0.5 > sidebar_cutoff)
        .filter(|s| !s.text.trim().is_empty())
        .collect();

    if filtered.is_empty() {
        return None;
    }

    // Group segments into rows by vertical proximity.
    let mut rows: Vec<SegmentRow> = Vec::new();
    for seg in &filtered {
        let seg_top = seg.top;
        let seg_bottom = seg.bottom;
        let seg_height = (seg_top - seg_bottom).abs().max(1.0);
        let tolerance = seg_height * 0.5;

        let matching_row = rows
            .iter()
            .position(|row| (seg_top - row.top).abs() <= tolerance && (seg_bottom - row.bottom).abs() <= tolerance);

        if let Some(idx) = matching_row {
            rows[idx].top = rows[idx].top.max(seg_top);
            rows[idx].bottom = rows[idx].bottom.min(seg_bottom);
            rows[idx].segments.push((*seg).clone());
        } else {
            rows.push(SegmentRow {
                segments: vec![(*seg).clone()],
                top: seg_top,
                bottom: seg_bottom,
            });
        }
    }

    // Sort rows top-to-bottom (descending top in PDF bottom-left coordinates).
    rows.sort_by(|a, b| b.top.partial_cmp(&a.top).unwrap_or(std::cmp::Ordering::Equal));

    // Build one SegmentData per row by concatenating segment texts.
    let mut result: Vec<SegmentData> = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut sorted_segs = row.segments.clone();
        sorted_segs.sort_by(|a, b| a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal));

        // De-duplicate overlapping segments with identical text (bold/shadow rendering).
        sorted_segs.dedup_by(|b_seg, a_seg| a_seg.text == b_seg.text && (a_seg.left - b_seg.left).abs() < 1.0);

        let mut row_text = String::new();
        let mut row_left = f32::MAX;
        let mut row_right = f32::MIN;
        let mut row_font_size = 12.0_f32;
        let mut row_bold = false;
        let mut row_italic = false;
        let mut row_mono = false;
        let mut row_baseline = 0.0_f32;
        let mut prev_right = f32::MIN;

        for seg in &sorted_segs {
            if row_text.is_empty() {
                // First segment in row.
                row_left = seg.left;
                row_font_size = seg.font_size;
                row_bold = seg.is_bold;
                row_italic = seg.is_italic;
                row_mono = seg.is_monospace;
                row_baseline = seg.baseline_y;
            } else {
                // Insert space between segments if the gap exceeds a fraction
                // of the average character width. This avoids inserting spaces
                // between character-level segments in the same word.
                let gap = seg.left - prev_right;
                let seg_width = (seg.right - seg.left).max(0.1);
                let seg_chars = seg.text.chars().count().max(1) as f32;
                let avg_char_w = seg_width / seg_chars;
                let space_threshold = avg_char_w * 0.33;
                if gap > space_threshold {
                    row_text.push(' ');
                }
            }
            row_text.push_str(&seg.text);
            row_right = seg.right;
            prev_right = seg.right;
        }

        let trimmed = row_text.trim();
        if trimmed.is_empty() {
            continue;
        }

        result.push(SegmentData {
            text: trimmed.to_string(),
            x: row_left,
            y: row_baseline,
            width: (row_right - row_left).max(row_font_size),
            height: (row.top - row.bottom).max(row_font_size),
            font_size: row_font_size,
            is_bold: row_bold,
            is_italic: row_italic,
            is_monospace: row_mono,
            baseline_y: row_baseline,
        });
    }

    if result.is_empty() {
        return None;
    }

    Some(result)
}

/// Partition page objects into column groups by moving objects out of the source vec.
///
/// Each column group is a `Vec<usize>` of indices into `objects`. This function
/// consumes the objects vec and returns one `Vec<PdfPageObject>` per column.
fn partition_objects_by_columns<'a>(
    objects: Vec<PdfPageObject<'a>>,
    column_groups: &[Vec<usize>],
) -> Vec<Vec<PdfPageObject<'a>>> {
    if column_groups.len() <= 1 {
        return vec![objects];
    }

    let total = objects.len();
    let num_columns = column_groups.len();
    let mut col_for_obj = vec![0usize; total];
    for (col_idx, group) in column_groups.iter().enumerate() {
        for &obj_idx in group {
            if obj_idx < total {
                col_for_obj[obj_idx] = col_idx;
            }
        }
    }

    let mut result: Vec<Vec<PdfPageObject<'a>>> = (0..num_columns).map(|_| Vec::new()).collect();
    for (i, obj) in objects.into_iter().enumerate() {
        result[col_for_obj[i]].push(obj);
    }

    result
}

/// Per-character data extracted from pdfium's text API.
#[derive(Clone)]
struct CharInfo {
    ch: char,
    x: f32,
    y: f32,
    font_size: f32,
    /// Right edge of the character from `tight_bounds()`, falling back to `x + font_size * 0.6`.
    right_x: f32,
    is_bold: bool,
    is_italic: bool,
    is_monospace: bool,
    has_map_error: bool,
    is_symbolic: bool,
    /// True if the character is a hyphen as determined by pdfium's `is_hyphen()` API.
    #[allow(dead_code)]
    is_hyphen: bool,
}

/// Remove characters from sidebar annotations (e.g., arXiv identifiers along the left margin).
///
/// Sidebar text in papers is typically rotated 90° along the left margin, producing
/// isolated characters at very low X positions that span most of the page height.
/// This distinguishes them from bullets/labels which only span a small region.
///
/// Detection criteria:
/// 1. Characters in the leftmost 5% of page width
/// 2. Constitute < 5% of total characters
/// 3. Span at least 30% of the page's vertical text extent
fn filter_sidebar_characters(char_infos: &mut Vec<CharInfo>, page_width: f32) {
    if char_infos.len() < 20 || page_width <= 0.0 {
        return;
    }

    let total_non_space = char_infos.iter().filter(|c| c.ch != ' ').count();
    if total_non_space < 20 {
        return;
    }

    let margin_band = page_width * 0.065;

    let margin_indices: Vec<usize> = char_infos
        .iter()
        .enumerate()
        .filter(|(_, c)| c.ch != ' ' && c.x < margin_band)
        .map(|(i, _)| i)
        .collect();

    // Need some margin chars, but not too many (< 5% of total)
    if margin_indices.is_empty() || margin_indices.len() * 20 > total_non_space {
        return;
    }

    // Sidebar text spans most of the page height; bullets/labels don't.
    let (y_min, y_max) = char_infos
        .iter()
        .filter(|c| c.ch != ' ')
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), c| {
            (lo.min(c.y), hi.max(c.y))
        });
    let page_text_height = (y_max - y_min).max(1.0);

    let (margin_y_min, margin_y_max) =
        margin_indices
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &i| {
                let y = char_infos[i].y;
                (lo.min(y), hi.max(y))
            });
    let margin_y_span = (margin_y_max - margin_y_min).abs();

    if margin_y_span < page_text_height * 0.3 {
        return; // Margin chars don't span the page — not a sidebar
    }

    // Safety check: if most margin characters are the START of words (i.e.,
    // immediately followed by a non-margin character at a similar Y), they're
    // line-initial characters, not sidebar annotations. Don't filter them.
    let mut word_start_count = 0usize;
    for &idx in &margin_indices {
        // Check if the next non-space character is close by on the same line.
        if idx + 1 < char_infos.len() {
            let curr = &char_infos[idx];
            let next = &char_infos[idx + 1];
            let same_line = (curr.y - next.y).abs() < curr.font_size * 0.5;
            let close_x = (next.x - curr.x) < curr.font_size * 1.2;
            if same_line && close_x && next.ch != ' ' {
                word_start_count += 1;
            }
        }
    }

    // If >50% of margin chars are word-starts, this is normal left-aligned text,
    // not a sidebar. Abort filtering.
    if word_start_count * 2 > margin_indices.len() {
        return;
    }

    // Remove sidebar characters using a swap-compact to avoid O(n²) shifting.
    let mut keep = vec![true; char_infos.len()];
    for &idx in &margin_indices {
        keep[idx] = false;
    }
    let mut write = 0;
    // The swap-compact pattern requires index-based access; an iterator cannot simultaneously
    // provide the read index and mutably borrow `char_infos` for `swap`.
    #[allow(clippy::needless_range_loop)]
    for read in 0..char_infos.len() {
        if keep[read] {
            char_infos.swap(write, read);
            write += 1;
        }
    }
    char_infos.truncate(write);
}

/// Build the text for a single line from character info, inserting spaces where
/// large X-position gaps indicate word or column boundaries.
///
/// This is critical for positioned/tabular PDFs where characters are placed at
/// specific coordinates without explicit space characters between words.
fn build_line_text(chars: &[CharInfo], repair_map: Option<&[(char, &str)]>) -> String {
    let mut line_text = String::new();
    for (idx, ci) in chars.iter().enumerate() {
        if ci.has_map_error
            && !ci.is_symbolic
            && let Some(map) = repair_map
            && let Some((_, replacement)) = map.iter().find(|(c, _)| *c == ci.ch)
        {
            line_text.push_str(replacement);
            continue;
        }

        // Geometric veto for generated spaces (broken CMap fix).
        //
        // pdfium inserts generated chars as word boundaries, but for fonts with
        // broken CMaps these appear mid-word (e.g., "co mputer"). We keep generated
        // spaces from pdfium but veto them when the geometric gap between the
        // surrounding real characters is small enough to indicate same-word.
        //
        // For non-space → non-space transitions (no generated space), insert a
        // space when the gap exceeds the threshold (positioned text detection).
        if idx > 0 && ci.ch != ' ' {
            let prev = &chars[idx - 1];
            if prev.ch == ' ' {
                // Previous char is a generated space. Check if we should keep it
                // by looking at the gap between the last real char and current char.
                // Find the last non-space char before the space.
                let last_real = chars[..idx - 1].iter().rev().find(|c| c.ch != ' ');
                if let Some(real_prev) = last_real {
                    let gap = ci.x - real_prev.right_x;
                    let real_prev_width = (real_prev.right_x - real_prev.x).max(0.0);
                    let curr_width = (ci.right_x - ci.x).max(0.0);
                    let avg_char_width = if real_prev_width > 0.0 && curr_width > 0.0 {
                        (real_prev_width + curr_width) * 0.5
                    } else {
                        (ci.font_size + real_prev.font_size) * 0.3
                    };
                    // Veto the space if gap < 50% of character width.
                    // This threshold is calibrated from the reference cell-merging algorithm's 0.33 on
                    // advance widths, adjusted up because tight_bounds are narrower.
                    if gap < avg_char_width * 0.5 {
                        // Remove already-pushed space — chars are too close.
                        line_text.pop();
                    }
                }
                // If no real_prev found, the space stands as-is.
            } else {
                // Non-space to non-space: insert space on large gaps (positioned text).
                let gap = ci.x - prev.right_x;
                let avg_height = (ci.font_size + prev.font_size) * 0.5;
                if gap > avg_height {
                    line_text.push(' ');
                }
            }
        }

        line_text.push(ci.ch);
    }
    line_text
}

// ── Character-level column detection constants ──

/// Minimum non-space characters per column side to validate a split.
const MIN_CHARS_PER_COLUMN: usize = 20;

/// Minimum gap as fraction of content X-span to qualify as column boundary.
const MIN_CHAR_COLUMN_GAP_FRACTION: f32 = 0.04;

/// Minimum absolute gap in points to qualify as a column boundary.
/// Prevents false positives on narrow content where 4% is very small.
const MIN_CHAR_COLUMN_GAP_ABS: f32 = 20.0;

/// Minimum vertical span fraction for each column side.
const MIN_CHAR_COLUMN_VERTICAL_SPAN: f32 = 0.3;

/// Maximum column split recursion depth (supports up to 2^3 = 8 columns).
const MAX_CHAR_COLUMN_DEPTH: usize = 3;

/// Detect column boundaries from character X-positions.
///
/// Uses edge-sweep gap analysis (same pattern as `columns.rs`):
/// build sorted edges, sweep left-to-right tracking max_right,
/// find largest gap exceeding threshold. Validates vertical span
/// and character count on each side. Recurses for 3+ columns.
///
/// Returns sorted split X-positions, or empty vec if single-column.
fn detect_char_column_splits(char_infos: &[CharInfo]) -> Vec<f32> {
    fn detect_recursive(chars: &[CharInfo], depth: usize) -> Vec<f32> {
        if depth >= MAX_CHAR_COLUMN_DEPTH {
            return Vec::new();
        }

        // Filter to non-space characters
        let non_space: Vec<&CharInfo> = chars.iter().filter(|c| c.ch != ' ').collect();
        if non_space.len() < MIN_CHARS_PER_COLUMN * 2 {
            return Vec::new();
        }

        // Compute content extent
        let x_min = non_space.iter().map(|c| c.x).fold(f32::MAX, f32::min);
        let x_max = non_space.iter().map(|c| c.right_x).fold(f32::MIN, f32::max);
        let x_span = x_max - x_min;
        if x_span < 1.0 {
            return Vec::new();
        }

        let y_min = non_space.iter().map(|c| c.y).fold(f32::MAX, f32::min);
        let y_max = non_space.iter().map(|c| c.y).fold(f32::MIN, f32::max);
        let y_span = y_max - y_min;
        if y_span < 1.0 {
            return Vec::new();
        }

        // Build sorted edge list: (left_x, right_x)
        let mut edges: Vec<(f32, f32)> = non_space.iter().map(|c| (c.x, c.right_x)).collect();
        edges.sort_by(|a, b| a.0.total_cmp(&b.0));

        // Sweep to find largest gap
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

        let min_gap = (x_span * MIN_CHAR_COLUMN_GAP_FRACTION).max(MIN_CHAR_COLUMN_GAP_ABS);
        if best_gap < min_gap {
            return Vec::new();
        }

        let split_x = match best_split {
            Some(x) => x,
            None => return Vec::new(),
        };

        // Validate: each side has enough chars and vertical span
        let left_chars: Vec<&CharInfo> = non_space.iter().filter(|c| c.x < split_x).copied().collect();
        let right_chars: Vec<&CharInfo> = non_space.iter().filter(|c| c.x >= split_x).copied().collect();

        if left_chars.len() < MIN_CHARS_PER_COLUMN || right_chars.len() < MIN_CHARS_PER_COLUMN {
            return Vec::new();
        }

        let left_y_min = left_chars.iter().map(|c| c.y).fold(f32::MAX, f32::min);
        let left_y_max = left_chars.iter().map(|c| c.y).fold(f32::MIN, f32::max);
        let right_y_min = right_chars.iter().map(|c| c.y).fold(f32::MAX, f32::min);
        let right_y_max = right_chars.iter().map(|c| c.y).fold(f32::MIN, f32::max);

        let left_y_span = left_y_max - left_y_min;
        let right_y_span = right_y_max - right_y_min;

        if left_y_span < y_span * MIN_CHAR_COLUMN_VERTICAL_SPAN || right_y_span < y_span * MIN_CHAR_COLUMN_VERTICAL_SPAN
        {
            return Vec::new();
        }

        // Valid split found. Recurse on each side for 3+ columns.
        let left_all: Vec<CharInfo> = chars.iter().filter(|c| c.x < split_x).cloned().collect();
        let right_all: Vec<CharInfo> = chars.iter().filter(|c| c.x >= split_x).cloned().collect();

        let mut splits = detect_recursive(&left_all, depth + 1);
        splits.push(split_x);
        splits.extend(detect_recursive(&right_all, depth + 1));
        splits.sort_by(|a, b| a.total_cmp(b));
        splits
    }

    detect_recursive(char_infos, 0)
}

/// Partition characters into column groups based on split X-positions.
///
/// Characters are assigned to columns by their X-position relative to split points.
/// Returns one `Vec<CharInfo>` per column, ordered left-to-right.
///
/// Within each column, characters are sorted into reading order (top-to-bottom,
/// left-to-right) so that downstream line detection in `assemble_segments_from_chars`
/// works correctly regardless of pdfium's original iteration order.
fn partition_chars_by_columns(chars: Vec<CharInfo>, splits: &[f32]) -> Vec<Vec<CharInfo>> {
    let num_columns = splits.len() + 1;
    let mut columns: Vec<Vec<CharInfo>> = (0..num_columns).map(|_| Vec::new()).collect();

    for ci in chars {
        let col = splits.iter().filter(|&&s| ci.x >= s).count();
        columns[col].push(ci);
    }

    // Sort each column into reading order: top-to-bottom (descending Y in PDF coords),
    // then left-to-right (ascending X) within the same line.
    for col in &mut columns {
        sort_chars_reading_order(col);
    }

    columns
}

/// Sort characters into reading order: top-to-bottom, left-to-right.
///
/// PDF coordinate system has y=0 at bottom, so higher Y values are at the top
/// of the page and should come first. Within the same line (similar Y), characters
/// are sorted left-to-right by X position.
///
/// Uses a two-pass approach:
/// 1. Compute a line-height estimate from the font sizes.
/// 2. Quantize Y positions into line bands, then sort by (band descending, X ascending).
fn sort_chars_reading_order(chars: &mut [CharInfo]) {
    if chars.len() < 2 {
        return;
    }

    // Estimate line height from median font size.
    let avg_font_size = chars.iter().map(|c| c.font_size).sum::<f32>() / chars.len() as f32;
    let y_tolerance = avg_font_size * 0.5;

    if y_tolerance <= 0.0 {
        return;
    }

    chars.sort_by(|a, b| {
        // Quantize Y to detect same-line characters (within half a font size).
        let a_band = (a.y / y_tolerance).round() as i64;
        let b_band = (b.y / y_tolerance).round() as i64;
        // Higher Y (top of page) comes first → descending band order.
        b_band.cmp(&a_band).then_with(|| a.x.total_cmp(&b.x))
    });
}

/// Assemble segments from a slice of characters using Y-position line breaks.
///
/// Extracted from `chars_to_segments` for reuse in per-column assembly.
/// Detects line breaks by Y-position changes, builds text per line using
/// `build_line_text`, and emits one `SegmentData` per line.
///
/// When a word is split across PDF lines (e.g., "soft" on line 1, "ware" on
/// line 2), the two fragments are merged into a single segment to prevent
/// the downstream renderer from inserting a space between them. This is
/// detected by checking whether the current line extends to near the right
/// margin (full-width) and both the trailing character and the leading
/// character of the next line are lowercase alphabetic.
fn assemble_segments_from_chars(char_infos: &[CharInfo], repair_map: Option<&[(char, &str)]>) -> Vec<SegmentData> {
    if char_infos.is_empty() {
        return Vec::new();
    }

    // Compute line break threshold from Y-position changes.
    let mut y_jumps: Vec<f32> = Vec::new();
    for i in 1..char_infos.len() {
        if char_infos[i].ch == ' ' || char_infos[i - 1].ch == ' ' {
            continue;
        }
        let dy = (char_infos[i].y - char_infos[i - 1].y).abs();
        if dy > 1.0 && dy < 200.0 {
            y_jumps.push(dy);
        }
    }
    let line_height_threshold = if y_jumps.len() >= 3 {
        y_jumps.sort_by(|a, b| a.total_cmp(b));
        y_jumps[y_jumps.len() / 2] * 0.6 // median, not minimum
    } else {
        let avg_fs = char_infos.iter().map(|c| c.font_size).sum::<f32>() / char_infos.len() as f32;
        avg_fs * 0.5
    };
    let line_break_threshold = line_height_threshold.max(2.0);

    // ── Pass 1: identify line boundaries ──
    // Collect (start, end) index pairs for each visual line so we can compute
    // the right margin before emitting segments.
    let mut line_ranges: Vec<(usize, usize)> = Vec::new();
    {
        let mut ls = 0;
        for i in 1..=char_infos.len() {
            let brk = if i == char_infos.len() {
                true
            } else {
                let dy = (char_infos[i].y - char_infos[ls].y).abs();
                dy > line_break_threshold && char_infos[i].ch != ' '
            };
            if brk {
                line_ranges.push((ls, i));
                if i < char_infos.len() {
                    ls = i;
                }
            }
        }
    }

    // Compute the right margin: maximum right_x of the last non-space char
    // across all lines that have at least a few characters (to exclude short
    // title/heading lines from inflating the margin).
    let _right_margin = compute_right_margin(char_infos, &line_ranges);

    // ── Pass 2: emit segments, merging cross-line hyphenated word breaks ──
    //
    // Follows the reference `sanitize_text()` approach: when a line ends with a
    // hyphen that pdfium recognises as a line-break hyphen (`is_hyphen` flag
    // from `FPDFText_IsHyphen`), strip the hyphen and join directly with the
    // next line's text.  This precisely handles "soft-" + "ware" → "software",
    // "recog-" + "nition" → "recognition", etc., without the false positives
    // of the previous full-width heuristic.
    let mut segments = Vec::new();
    let mut pending_text: Option<String> = None;
    let mut pending_start: usize = 0;

    for (range_idx, &(start, end)) in line_ranges.iter().enumerate() {
        let line_text = build_line_text(&char_infos[start..end], repair_map);
        let trimmed = line_text.trim();
        if trimmed.is_empty() {
            continue;
        }

        // If we have pending text from a previous line that ended with a
        // detected word break, append this line's text directly (no space).
        if let Some(ref mut pending) = pending_text {
            pending.push_str(trimmed);
        } else {
            pending_text = Some(trimmed.to_string());
            pending_start = start;
        }

        // Check if this line ends with a pdfium-flagged hyphen that should be
        // removed to rejoin a word split across lines.  The `is_hyphen` flag
        // from pdfium's `FPDFText_IsHyphen` fires for soft hyphens (U+00AD),
        // discretionary hyphens, and regular hyphens at line-break positions.
        let merge_with_next = range_idx + 1 < line_ranges.len() && line_ends_with_break_hyphen(&char_infos[start..end]);

        if merge_with_next {
            // Strip the trailing hyphen character from the accumulated text so
            // "soft-" becomes "soft" before "ware" is appended on the next
            // iteration.
            if let Some(ref mut pending) = pending_text {
                strip_trailing_hyphen(pending);
            }
            continue;
        }

        // Emit the segment.
        if let Some(text) = pending_text.take() {
            let first = &char_infos[pending_start];
            let last_idx = (pending_start..end)
                .rev()
                .find(|&j| char_infos[j].ch != ' ')
                .unwrap_or(pending_start);
            let last = &char_infos[last_idx];
            let width = (last.right_x - first.x).max(first.font_size);

            segments.push(SegmentData {
                text,
                x: first.x,
                y: first.y,
                width,
                height: first.font_size,
                font_size: first.font_size,
                is_bold: first.is_bold,
                is_italic: first.is_italic,
                is_monospace: first.is_monospace,
                baseline_y: first.y,
            });
        }
    }

    // Flush any remaining pending text (last line in a merge chain).
    if let Some(text) = pending_text.take() {
        let last_range = line_ranges.last().unwrap();
        let first = &char_infos[pending_start];
        let last_idx = (pending_start..last_range.1)
            .rev()
            .find(|&j| char_infos[j].ch != ' ')
            .unwrap_or(pending_start);
        let last = &char_infos[last_idx];
        let width = (last.right_x - first.x).max(first.font_size);

        segments.push(SegmentData {
            text,
            x: first.x,
            y: first.y,
            width,
            height: first.font_size,
            font_size: first.font_size,
            is_bold: first.is_bold,
            is_italic: first.is_italic,
            is_monospace: first.is_monospace,
            baseline_y: first.y,
        });
    }

    segments
}

/// Compute the right margin for a set of lines.
///
/// Returns the maximum `right_x` of the last non-space character across lines
/// that have at least 3 non-space characters (to exclude very short lines like
/// headings or labels from inflating the margin estimate).
fn compute_right_margin(char_infos: &[CharInfo], line_ranges: &[(usize, usize)]) -> f32 {
    let mut max_right = f32::MIN;
    for &(start, end) in line_ranges {
        let non_space_count = char_infos[start..end].iter().filter(|c| c.ch != ' ').count();
        if non_space_count < 3 {
            continue;
        }
        if let Some(last) = (start..end).rev().find(|&j| char_infos[j].ch != ' ') {
            max_right = max_right.max(char_infos[last].right_x);
        }
    }
    max_right
}

/// Check whether a line (slice of `CharInfo`) ends with a hyphen that pdfium
/// recognises as a line-break hyphen.
///
/// Returns `true` when the last non-space character satisfies **both**:
/// 1. `is_hyphen` flag is set (pdfium's `FPDFText_IsHyphen` API), and
/// 2. The character before the hyphen is alphabetic, and
/// 3. The character is one of the standard hyphen/minus codepoints (`-`,
///    U+2010 HYPHEN, U+00AD SOFT HYPHEN) — excluding em/en dashes that are
///    intentional punctuation.
///
/// This mirrors the reference `sanitize_text` which strips trailing `-` at line
/// boundaries to rejoin hyphenated words.
fn line_ends_with_break_hyphen(line_chars: &[CharInfo]) -> bool {
    // Find the last non-space character.
    let last = match line_chars.iter().rev().find(|c| c.ch != ' ') {
        Some(c) => c,
        None => return false,
    };

    if !last.is_hyphen {
        return false;
    }

    // Only treat standard hyphen-like chars as break hyphens — not em/en dashes.
    if !matches!(last.ch, '-' | '\u{2010}' | '\u{00AD}' | '\u{2011}') {
        return false;
    }

    // The character before the hyphen must be alphabetic to confirm this is a
    // word-break hyphen rather than a standalone dash or numeric range.
    line_chars
        .iter()
        .rev()
        .filter(|c| c.ch != ' ')
        .nth(1) // second-to-last non-space
        .is_some_and(|c| c.ch.is_alphabetic())
}

/// Strip the trailing hyphen character from a string.
///
/// Removes the last character if it is a hyphen/minus (`-`, U+2010, U+00AD,
/// U+2011). If the text ends with trailing spaces after the hyphen, only the
/// hyphen itself is removed (spaces were already trimmed by the caller).
fn strip_trailing_hyphen(text: &mut String) {
    if let Some(ch) = text.chars().next_back()
        && matches!(ch, '-' | '\u{2010}' | '\u{00AD}' | '\u{2011}')
    {
        text.pop();
    }
}

/// Convert pre-extracted `PageTextData` into `CharInfo` values and assemble segments.
///
/// This is the core segment assembly logic, decoupled from pdfium page access.
/// All character data comes from the single-pass `PageTextData` DTO.
fn chars_to_segments_from_data(data: &PageTextData, page_width: f32) -> Option<Vec<SegmentData>> {
    if data.chars.is_empty() {
        return None;
    }

    // Convert ExtractedChar → CharInfo for the existing assembly pipeline.
    let mut char_infos: Vec<CharInfo> = data
        .chars
        .iter()
        .map(|ec| CharInfo {
            ch: ec.ch,
            x: ec.x,
            y: ec.y,
            right_x: ec.right_x,
            font_size: ec.font_size,
            is_bold: ec.is_bold,
            is_italic: ec.is_italic,
            is_monospace: ec.is_monospace,
            has_map_error: ec.has_map_error,
            is_symbolic: ec.is_symbolic,
            is_hyphen: ec.is_hyphen,
        })
        .collect();

    // Filter out sidebar/margin characters (e.g., arXiv identifiers along left margin).
    filter_sidebar_characters(&mut char_infos, page_width);

    if char_infos.is_empty() {
        return None;
    }

    // Detect character-level column boundaries before line assembly.
    let column_splits = detect_char_column_splits(&char_infos);

    let repair_map = data.ligature_repair_map.as_deref();

    let segments = if column_splits.is_empty() {
        // Single column: assemble lines directly
        assemble_segments_from_chars(&char_infos, repair_map)
    } else {
        // Multi-column: partition chars by column, assemble per column, concatenate
        let columns = partition_chars_by_columns(char_infos, &column_splits);
        columns
            .iter()
            .flat_map(|col| assemble_segments_from_chars(col, repair_map))
            .collect()
    };

    if segments.is_empty() { None } else { Some(segments) }
}

/// Convert pdfium paragraphs into SegmentData, preserving per-line positions.
fn extract_paragraphs_to_segments(paragraphs: Vec<PdfiumParagraph>, segments: &mut Vec<SegmentData>) {
    for para in paragraphs {
        for line in para.into_lines() {
            let line_baseline = line.bottom.value;
            let line_left = line.left.value;
            let mut running_x = line_left;

            for fragment in &line.fragments {
                match fragment {
                    PdfParagraphFragment::StyledString(styled) => {
                        let text = normalize_text_encoding(styled.text());
                        if text.trim().is_empty() {
                            continue;
                        }

                        let font_size = styled.font_size().value;
                        let is_bold = styled.is_bold();
                        let is_italic = styled.is_italic();
                        let is_monospace = styled.is_monospace();
                        let estimated_width = text.len() as f32 * font_size * 0.5;

                        segments.push(SegmentData {
                            text: text.into_owned(),
                            x: running_x,
                            y: line_baseline,
                            width: estimated_width,
                            height: font_size,
                            font_size,
                            is_bold,
                            is_italic,
                            is_monospace,
                            baseline_y: line_baseline,
                        });

                        running_x += estimated_width;
                    }
                    PdfParagraphFragment::NonTextObject(_) | PdfParagraphFragment::LineBreak { .. } => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(role: ContentRole, text: &str) -> ExtractedBlock {
        ExtractedBlock {
            role,
            text: text.to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: Vec::new(),
        }
    }

    fn make_block_with_font(role: ContentRole, text: &str, font_size: f32) -> ExtractedBlock {
        ExtractedBlock {
            role,
            text: text.to_string(),
            bounds: None,
            font_size: Some(font_size),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: Vec::new(),
        }
    }

    #[test]
    fn test_heading_block() {
        // Heading must have meaningfully larger font than body for validation to pass
        let blocks = vec![
            make_block_with_font(ContentRole::Heading { level: 2 }, "Section Title", 18.0),
            make_block_with_font(ContentRole::Paragraph, "Body text line one", 12.0),
            make_block_with_font(ContentRole::Paragraph, "Body text line two", 12.0),
            make_block_with_font(ContentRole::Paragraph, "Body text line three", 12.0),
        ];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 4);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_heading_trusted_from_structure_tree() {
        // Structure tree heading tags are trusted (author-intent metadata),
        // even when font size matches body text.
        let blocks = vec![
            make_block(ContentRole::Heading { level: 3 }, "Not really a heading"),
            make_block(ContentRole::Paragraph, "Body text"),
            make_block(ContentRole::Paragraph, "More body text"),
        ];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(paragraphs[0].heading_level, Some(3)); // Trusted from structure tree
    }

    #[test]
    fn test_body_block() {
        let blocks = vec![make_block(ContentRole::Paragraph, "Body text")];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].heading_level, None);
        assert!(!paragraphs[0].is_list_item);
    }

    #[test]
    fn test_list_item_block() {
        let blocks = vec![ExtractedBlock {
            role: ContentRole::ListItem {
                label: Some("1.".to_string()),
            },
            text: "First item".to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: Vec::new(),
        }];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 1);
        assert!(paragraphs[0].is_list_item);
        // Check that the label is prepended
        let first_seg_text = &paragraphs[0].lines[0].segments[0].text;
        assert_eq!(first_seg_text, "1.");
    }

    #[test]
    fn test_empty_text_skipped() {
        let blocks = vec![make_block(ContentRole::Paragraph, "")];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert!(paragraphs.is_empty());
    }

    #[test]
    fn test_whitespace_only_skipped() {
        let blocks = vec![make_block(ContentRole::Paragraph, "   ")];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert!(paragraphs.is_empty());
    }

    #[test]
    fn test_children_processed() {
        let blocks = vec![ExtractedBlock {
            role: ContentRole::Other("Table".to_string()),
            text: String::new(),
            bounds: None,
            font_size: None,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: vec![
                make_block(ContentRole::Paragraph, "Cell 1"),
                make_block(ContentRole::Paragraph, "Cell 2"),
            ],
        }];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 2);
    }

    fn make_char(ch: char, x: f32, y: f32, font_size: f32) -> CharInfo {
        CharInfo {
            ch,
            x,
            y,
            font_size,
            right_x: x + font_size * 0.6, // test default
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            has_map_error: false,
            is_symbolic: false,
            is_hyphen: false,
        }
    }

    /// Regression test for GitHub issue #431:
    /// Positioned PDF text with large X-gaps between characters must have
    /// spaces inserted, otherwise "Main Deck" becomes "MainDeck" and
    /// "12,40" with gaps becomes "12,40" concatenated without spaces.
    #[test]
    fn test_issue_431_build_line_text_inserts_spaces_at_x_gaps() {
        let fs = 12.0;
        // "Main" at x=100 then "Deck" at x=200 (large gap = different words)
        let chars = vec![
            make_char('M', 100.0, 0.0, fs),
            make_char('a', 107.0, 0.0, fs),
            make_char('i', 114.0, 0.0, fs),
            make_char('n', 121.0, 0.0, fs),
            // Gap of ~79 units (>> font_size) before "Deck"
            make_char('D', 200.0, 0.0, fs),
            make_char('e', 207.0, 0.0, fs),
            make_char('c', 214.0, 0.0, fs),
            make_char('k', 221.0, 0.0, fs),
        ];
        let result = build_line_text(&chars, None);
        assert_eq!(result, "Main Deck", "Large X-gap should produce a space between words");
    }

    #[test]
    fn test_issue_431_build_line_text_no_false_spaces() {
        let fs = 12.0;
        // "Hello" with normal character advance (~7 units for 12pt font)
        let chars = vec![
            make_char('H', 100.0, 0.0, fs),
            make_char('e', 107.0, 0.0, fs),
            make_char('l', 114.0, 0.0, fs),
            make_char('l', 121.0, 0.0, fs),
            make_char('o', 128.0, 0.0, fs),
        ];
        let result = build_line_text(&chars, None);
        assert_eq!(result, "Hello", "Normal spacing should not insert extra spaces");
    }

    #[test]
    fn test_issue_431_tabular_numbers_with_gaps() {
        let fs = 12.0;
        // Tabular data: "12,40" at x=50 and "480" at x=200
        let chars = vec![
            make_char('1', 50.0, 0.0, fs),
            make_char('2', 57.0, 0.0, fs),
            make_char(',', 64.0, 0.0, fs),
            make_char('4', 71.0, 0.0, fs),
            make_char('0', 78.0, 0.0, fs),
            // Large gap to next column
            make_char('4', 200.0, 0.0, fs),
            make_char('8', 207.0, 0.0, fs),
            make_char('0', 214.0, 0.0, fs),
        ];
        let result = build_line_text(&chars, None);
        assert_eq!(result, "12,40 480", "Column gap should produce space between numbers");
    }

    #[test]
    fn test_issue_431_preserves_existing_spaces() {
        let fs = 12.0;
        // When pdfium already provides space characters, don't double-space
        let chars = vec![
            make_char('A', 100.0, 0.0, fs),
            make_char(' ', 107.0, 0.0, fs),
            make_char('B', 200.0, 0.0, fs),
        ];
        let result = build_line_text(&chars, None);
        assert_eq!(result, "A B", "Should not insert extra space when space char exists");
    }

    // ── Character-level column detection tests ──

    /// Helper: generate chars in a column region.
    fn make_column_chars(
        x_start: f32,
        x_end: f32,
        y_start: f32,
        y_end: f32,
        chars_per_line: usize,
        num_lines: usize,
        font_size: f32,
    ) -> Vec<CharInfo> {
        let mut chars = Vec::new();
        let x_step = if chars_per_line > 1 {
            (x_end - x_start) / (chars_per_line as f32 - 1.0)
        } else {
            0.0
        };
        let y_step = if num_lines > 1 {
            (y_end - y_start) / (num_lines as f32 - 1.0)
        } else {
            0.0
        };
        for line in 0..num_lines {
            let y = y_start + line as f32 * y_step;
            for c in 0..chars_per_line {
                let x = x_start + c as f32 * x_step;
                chars.push(make_char('a', x, y, font_size));
            }
        }
        chars
    }

    #[test]
    fn test_detect_no_split_single_column() {
        // 100 chars in one column with realistic spacing (font_size * 0.6 = 7.2pt per char)
        let chars = make_column_chars(10.0, 80.0, 0.0, 500.0, 10, 10, 12.0);
        let splits = detect_char_column_splits(&chars);
        assert!(splits.is_empty(), "Single column should produce no splits");
    }

    #[test]
    fn test_detect_two_columns() {
        // Left column: x=[10, 200], right column: x=[350, 540], gap=150pt
        let mut chars = make_column_chars(10.0, 200.0, 0.0, 400.0, 5, 6, 12.0);
        chars.extend(make_column_chars(350.0, 540.0, 0.0, 400.0, 5, 6, 12.0));
        let splits = detect_char_column_splits(&chars);
        assert_eq!(splits.len(), 1, "Should detect one column split");
        assert!(
            splits[0] > 200.0 && splits[0] < 350.0,
            "Split should be between columns"
        );
    }

    #[test]
    fn test_detect_three_columns() {
        // Three columns with clear gaps
        let mut chars = make_column_chars(10.0, 120.0, 0.0, 400.0, 4, 6, 12.0);
        chars.extend(make_column_chars(250.0, 370.0, 0.0, 400.0, 4, 6, 12.0));
        chars.extend(make_column_chars(500.0, 620.0, 0.0, 400.0, 4, 6, 12.0));
        let splits = detect_char_column_splits(&chars);
        assert_eq!(splits.len(), 2, "Should detect two column splits for 3 columns");
        assert!(splits[0] < splits[1], "Splits should be sorted");
    }

    #[test]
    fn test_no_false_split_table() {
        // Short vertical span (like a table row) — should not trigger column split
        let mut chars = make_column_chars(10.0, 100.0, 200.0, 230.0, 5, 3, 12.0);
        chars.extend(make_column_chars(300.0, 400.0, 200.0, 230.0, 5, 3, 12.0));
        // Y span = 30, neither side spans 30% of that meaningfully
        // But let's also add some chars at other Y to give vertical extent
        // Actually: vertical extent is 30pt total, and each side spans 30pt = 100%.
        // The issue is that the page itself is sparse. For tables, the vertical extent
        // is small in absolute terms but 100% of content. We need to ensure the total
        // content extent is large enough for column detection to be meaningful.
        // Better test: table-like data with small Y span relative to wide page content
        let total_chars = make_column_chars(10.0, 400.0, 200.0, 210.0, 20, 2, 12.0);
        let splits = detect_char_column_splits(&total_chars);
        assert!(splits.is_empty(), "Single-line table data should not split");
    }

    #[test]
    fn test_no_false_split_few_chars() {
        // Too few characters per side
        let mut chars = make_column_chars(10.0, 100.0, 0.0, 400.0, 3, 3, 12.0); // 9 chars
        chars.extend(make_column_chars(300.0, 400.0, 0.0, 400.0, 3, 3, 12.0)); // 9 chars
        let splits = detect_char_column_splits(&chars);
        assert!(splits.is_empty(), "Too few chars per side should not split");
    }

    #[test]
    fn test_no_false_split_word_spacing() {
        // Normal word spacing (~8pt) in a single line — should not trigger
        let fs = 12.0;
        let mut chars = Vec::new();
        // "Hello World" with ~8pt word gap, across 10 lines
        for line in 0..10 {
            let y = line as f32 * 15.0;
            for i in 0..5 {
                chars.push(make_char('a', 10.0 + i as f32 * 7.0, y, fs));
            }
            // 8pt gap (less than content_width * 0.04 which would be ~20pt for a 500pt page)
            for i in 0..5 {
                chars.push(make_char('b', 53.0 + i as f32 * 7.0, y, fs));
            }
        }
        let splits = detect_char_column_splits(&chars);
        assert!(splits.is_empty(), "Normal word spacing should not trigger column split");
    }

    #[test]
    fn test_assemble_segments_basic() {
        // Three lines at different Y positions
        let chars = vec![
            make_char('H', 10.0, 100.0, 12.0),
            make_char('i', 20.0, 100.0, 12.0),
            // Line 2 (different Y)
            make_char('B', 10.0, 80.0, 12.0),
            make_char('y', 20.0, 80.0, 12.0),
            make_char('e', 30.0, 80.0, 12.0),
            // Line 3
            make_char('!', 10.0, 60.0, 12.0),
        ];
        let segments = assemble_segments_from_chars(&chars, None);
        assert_eq!(segments.len(), 3, "Should produce 3 segments for 3 lines");
        assert_eq!(segments[0].text, "Hi");
        assert_eq!(segments[1].text, "Bye");
        assert_eq!(segments[2].text, "!");
    }

    #[test]
    fn test_two_column_ordered_segments() {
        // Simulate a 2-column page: left at x~50, right at x~350, same Y values
        // Need ≥20 chars per column to pass MIN_CHARS_PER_COLUMN threshold
        let mut chars = Vec::new();
        // Left column, 5 lines of 5 chars = 25 per column
        for line in 0..5 {
            let y = 300.0 - line as f32 * 20.0;
            for c in 0..5 {
                chars.push(make_char('L', 50.0 + c as f32 * 8.0, y, 12.0));
            }
        }
        // Right column, 5 lines at same Y positions
        for line in 0..5 {
            let y = 300.0 - line as f32 * 20.0;
            for c in 0..5 {
                chars.push(make_char('R', 350.0 + c as f32 * 8.0, y, 12.0));
            }
        }

        let splits = detect_char_column_splits(&chars);
        assert!(!splits.is_empty(), "Should detect column split");

        let columns = partition_chars_by_columns(chars, &splits);
        assert_eq!(columns.len(), 2);

        let left_segs = assemble_segments_from_chars(&columns[0], None);
        let right_segs = assemble_segments_from_chars(&columns[1], None);
        assert_eq!(left_segs.len(), 5, "Left column should have 5 lines");
        assert_eq!(right_segs.len(), 5, "Right column should have 5 lines");

        // Left column chars should all be 'L', right should all be 'R'
        for seg in &left_segs {
            assert!(
                seg.text.chars().all(|c| c == 'L'),
                "Left column should only have L chars"
            );
        }
        for seg in &right_segs {
            assert!(
                seg.text.chars().all(|c| c == 'R'),
                "Right column should only have R chars"
            );
        }
    }

    /// Regression test: when pdfium returns characters interleaved across columns
    /// (alternating left-right by line), column partitioning + sorting must still
    /// produce correct per-column reading order.
    #[test]
    fn test_two_column_interleaved_chars_reading_order() {
        // Simulate pdfium returning chars line-by-line across both columns:
        // line1-left, line1-right, line2-left, line2-right, ...
        let mut chars = Vec::new();
        for line in 0..5 {
            let y = 300.0 - line as f32 * 20.0;
            // Left column chars for this line
            for c in 0..5 {
                chars.push(make_char('L', 50.0 + c as f32 * 8.0, y, 12.0));
            }
            // Right column chars for this line (same Y)
            for c in 0..5 {
                chars.push(make_char('R', 350.0 + c as f32 * 8.0, y, 12.0));
            }
        }

        let splits = detect_char_column_splits(&chars);
        assert!(!splits.is_empty(), "Should detect column split");

        let columns = partition_chars_by_columns(chars, &splits);
        assert_eq!(columns.len(), 2);

        // After partitioning + sorting, left column should be in top-to-bottom order
        let left_segs = assemble_segments_from_chars(&columns[0], None);
        let right_segs = assemble_segments_from_chars(&columns[1], None);
        assert_eq!(left_segs.len(), 5, "Left column should have 5 lines");
        assert_eq!(right_segs.len(), 5, "Right column should have 5 lines");

        // Y positions should be descending (top of page first) within each column
        for i in 1..left_segs.len() {
            assert!(
                left_segs[i - 1].y >= left_segs[i].y,
                "Left column segments should be in top-to-bottom order: y[{}]={} < y[{}]={}",
                i - 1,
                left_segs[i - 1].y,
                i,
                left_segs[i].y
            );
        }
        for i in 1..right_segs.len() {
            assert!(
                right_segs[i - 1].y >= right_segs[i].y,
                "Right column segments should be in top-to-bottom order: y[{}]={} < y[{}]={}",
                i - 1,
                right_segs[i - 1].y,
                i,
                right_segs[i].y
            );
        }
    }

    /// When chars from pdfium arrive in reversed Y order (bottom-to-top),
    /// the sort_chars_reading_order function corrects the order.
    #[test]
    fn test_sort_chars_reading_order_reversed_y() {
        let mut chars = vec![
            make_char('C', 10.0, 60.0, 12.0),  // bottom line
            make_char('B', 10.0, 80.0, 12.0),  // middle line
            make_char('A', 10.0, 100.0, 12.0), // top line
        ];
        sort_chars_reading_order(&mut chars);
        // After sorting: A (y=100), B (y=80), C (y=60) — top first
        assert_eq!(chars[0].ch, 'A');
        assert_eq!(chars[1].ch, 'B');
        assert_eq!(chars[2].ch, 'C');
    }

    /// Within the same line (same Y), chars should be sorted left-to-right.
    #[test]
    fn test_sort_chars_reading_order_same_line_left_to_right() {
        let mut chars = vec![
            make_char('C', 30.0, 100.0, 12.0),
            make_char('A', 10.0, 100.0, 12.0),
            make_char('B', 20.0, 100.0, 12.0),
        ];
        sort_chars_reading_order(&mut chars);
        assert_eq!(chars[0].ch, 'A');
        assert_eq!(chars[1].ch, 'B');
        assert_eq!(chars[2].ch, 'C');
    }

    /// Full-page simulation: two-column chars from content stream that puts
    /// all left column text first, then all right column text.
    /// After column split + partition, segments must be left-first then right-first.
    #[test]
    fn test_two_column_full_page_segment_order() {
        // Build chars: left column first (all lines), then right column (all lines)
        let mut chars = Vec::new();
        // Left column: 5 lines, each with distinct text
        for line in 0..5 {
            let y = 300.0 - line as f32 * 20.0;
            for c in 0..5 {
                chars.push(make_char(char::from(b'A' + line as u8), 50.0 + c as f32 * 8.0, y, 12.0));
            }
        }
        // Right column: 5 lines
        for line in 0..5 {
            let y = 300.0 - line as f32 * 20.0;
            for c in 0..5 {
                chars.push(make_char(
                    char::from(b'a' + line as u8),
                    350.0 + c as f32 * 8.0,
                    y,
                    12.0,
                ));
            }
        }

        let splits = detect_char_column_splits(&chars);
        assert!(!splits.is_empty(), "Should detect column split");

        let columns = partition_chars_by_columns(chars, &splits);
        let all_segments: Vec<SegmentData> = columns
            .iter()
            .flat_map(|col| assemble_segments_from_chars(col, None))
            .collect();

        // Left column segments come first (uppercase), then right column (lowercase).
        // Word-break merging may reduce segment count (consecutive full-width lowercase
        // lines get merged). Verify ordering: all uppercase chars before lowercase.
        assert!(all_segments.len() >= 2, "Should have at least 2 segments");
        let first_lowercase_idx = all_segments
            .iter()
            .position(|s| s.text.chars().any(|c| c.is_ascii_lowercase()))
            .unwrap_or(all_segments.len());
        // All segments before first_lowercase_idx should be uppercase
        for seg in &all_segments[..first_lowercase_idx] {
            assert!(
                seg.text.chars().all(|c| c.is_ascii_uppercase()),
                "Left column segments should be uppercase, got: {}",
                seg.text
            );
        }
        // All segments from first_lowercase_idx should be lowercase
        for seg in &all_segments[first_lowercase_idx..] {
            assert!(
                seg.text.chars().all(|c| c.is_ascii_lowercase()),
                "Right column segments should be lowercase, got: {}",
                seg.text
            );
        }
    }

    // ── Cross-line word break merging tests ──

    /// Helper to build a CharInfo with a specific character and right_x.
    fn make_char_exact(ch: char, x: f32, y: f32, font_size: f32, right_x: f32) -> CharInfo {
        CharInfo {
            ch,
            x,
            y,
            font_size,
            right_x,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            has_map_error: false,
            is_symbolic: false,
            is_hyphen: false,
        }
    }

    /// Build chars for a word at a given x, y position. Each char is 7pt wide.
    fn make_word_chars(word: &str, x_start: f32, y: f32, font_size: f32) -> Vec<CharInfo> {
        let char_width = font_size * 0.6; // ~7.2pt for 12pt font
        word.chars()
            .enumerate()
            .map(|(i, ch)| {
                let x = x_start + i as f32 * char_width;
                make_char_exact(ch, x, y, font_size, x + char_width)
            })
            .collect()
    }

    /// "soft-" at end of line 1 (with pdfium is_hyphen flag), "ware" at start of line 2.
    /// Should merge into "software" by stripping the hyphen.
    #[test]
    fn test_word_break_merge_software() {
        let fs = 12.0;
        let cw = fs * 0.6; // char width
        let right_margin_x = 300.0;
        let mut chars = Vec::new();
        // Line 1: "this is soft-" — hyphen at end has is_hyphen = true
        chars.extend(make_word_chars("this", 10.0, 100.0, fs));
        chars.push(make_char_exact(
            ' ',
            10.0 + 4.0 * cw + 1.0,
            100.0,
            fs,
            10.0 + 4.0 * cw + 1.0 + cw,
        ));
        chars.extend(make_word_chars("is", 10.0 + 5.0 * cw + 2.0, 100.0, fs));
        chars.push(make_char_exact(
            ' ',
            10.0 + 7.0 * cw + 3.0,
            100.0,
            fs,
            10.0 + 7.0 * cw + 3.0 + cw,
        ));
        let soft_start = right_margin_x - 5.0 * cw; // 4 chars + hyphen
        chars.extend(make_word_chars("soft", soft_start, 100.0, fs));
        // Trailing hyphen with is_hyphen = true (pdfium flag)
        chars.push(make_hyphen_char(
            '-',
            soft_start + 4.0 * cw,
            100.0,
            fs,
            soft_start + 5.0 * cw,
        ));

        // Line 2: "ware is great" — starts with lowercase continuation
        chars.extend(make_word_chars("ware", 10.0, 80.0, fs));
        chars.push(make_char_exact(
            ' ',
            10.0 + 4.0 * cw + 1.0,
            80.0,
            fs,
            10.0 + 4.0 * cw + 1.0 + cw,
        ));
        chars.extend(make_word_chars("is", 10.0 + 5.0 * cw + 2.0, 80.0, fs));
        chars.push(make_char_exact(
            ' ',
            10.0 + 7.0 * cw + 3.0,
            80.0,
            fs,
            10.0 + 7.0 * cw + 3.0 + cw,
        ));
        let great_start = right_margin_x - 5.0 * cw;
        chars.extend(make_word_chars("great", great_start, 80.0, fs));

        let segments = assemble_segments_from_chars(&chars, None);
        let all_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(
            all_text.contains("software"),
            "Expected 'software' in merged text, got: {all_text}",
        );
    }

    /// Short line ending with lowercase + next line starting with lowercase
    /// should NOT merge (e.g., "table" on a short line, "structure" on the next).
    #[test]
    fn test_no_false_merge_short_line() {
        let fs = 12.0;
        let cw = fs * 0.6;
        // Line 1: "table" — short line, not reaching the right margin
        // Right margin will be defined by line 2 which is longer.
        let mut chars = Vec::new();
        chars.extend(make_word_chars("table", 10.0, 100.0, fs));
        // table right edge: 10 + 5*7.2 = 46.0 — well short of margin

        // Line 2: "structure is important here today" — long line defining the margin
        let words = ["structure", "is", "important", "here", "today"];
        let mut x = 10.0;
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                chars.push(make_char_exact(' ', x, 80.0, fs, x + cw));
                x += cw;
            }
            chars.extend(make_word_chars(word, x, 80.0, fs));
            x += word.len() as f32 * cw;
        }

        let segments = assemble_segments_from_chars(&chars, None);
        // "table" is a short line (doesn't reach margin), so no merge.
        assert!(
            segments.len() >= 2,
            "Short line should NOT merge with next: got {} segments",
            segments.len()
        );
        assert_eq!(segments[0].text, "table");
    }

    /// Line ending with uppercase should NOT trigger merge.
    #[test]
    fn test_no_merge_uppercase_end() {
        let fs = 12.0;
        let cw = fs * 0.6;
        let right_margin_x = 200.0;
        let mut chars = Vec::new();

        // Line 1: "TITLE" — full-width, ends with uppercase E
        let title_start = right_margin_x - 5.0 * cw;
        chars.extend(make_word_chars("TITLE", title_start, 100.0, fs));

        // Line 2: "details follow here" — starts with lowercase
        let words = ["details", "follow", "here"];
        let mut x = 10.0;
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                chars.push(make_char_exact(' ', x, 80.0, fs, x + cw));
                x += cw;
            }
            chars.extend(make_word_chars(word, x, 80.0, fs));
            x += word.len() as f32 * cw;
        }

        let segments = assemble_segments_from_chars(&chars, None);
        assert!(
            segments.len() >= 2,
            "Uppercase-ending line should NOT merge: got {} segments",
            segments.len()
        );
    }

    /// Line ending with punctuation should NOT trigger merge.
    #[test]
    fn test_no_merge_punctuation_end() {
        let fs = 12.0;
        let cw = fs * 0.6;
        let right_margin_x = 200.0;
        let mut chars = Vec::new();

        // Line 1: "sentence." — ends with period
        let word_start = right_margin_x - 9.0 * cw;
        chars.extend(make_word_chars("sentence.", word_start, 100.0, fs));

        // Line 2: "next line" — starts with lowercase
        chars.extend(make_word_chars("next", 10.0, 80.0, fs));
        chars.push(make_char_exact(
            ' ',
            10.0 + 4.0 * cw + 1.0,
            80.0,
            fs,
            10.0 + 5.0 * cw + 1.0,
        ));
        let line2_start = right_margin_x - 4.0 * cw;
        chars.extend(make_word_chars("line", line2_start, 80.0, fs));

        let segments = assemble_segments_from_chars(&chars, None);
        assert!(
            segments.len() >= 2,
            "Punctuation-ending line should NOT merge: got {} segments",
            segments.len()
        );
    }

    /// Multi-line word break chain: "recog-" + "ni-" + "tion" across 3 lines.
    /// Both hyphens have pdfium's is_hyphen flag set.
    #[test]
    fn test_word_break_merge_chain() {
        let fs = 12.0;
        let cw = fs * 0.6;
        let right_margin_x = 200.0;

        let mut chars = Vec::new();
        // Line 1: "the recog-" — ends with pdfium hyphen
        chars.extend(make_word_chars("the", 10.0, 100.0, fs));
        chars.push(make_char_exact(
            ' ',
            10.0 + 3.0 * cw + 1.0,
            100.0,
            fs,
            10.0 + 4.0 * cw + 1.0,
        ));
        let recog_start = right_margin_x - 6.0 * cw; // 5 chars + hyphen
        chars.extend(make_word_chars("recog", recog_start, 100.0, fs));
        chars.push(make_hyphen_char(
            '-',
            recog_start + 5.0 * cw,
            100.0,
            fs,
            recog_start + 6.0 * cw,
        ));

        // Line 2: "nition is" — starts with lowercase continuation
        let ni_start = 10.0;
        chars.extend(make_word_chars("nition", ni_start, 80.0, fs));
        chars.push(make_char_exact(
            ' ',
            ni_start + 6.0 * cw + 1.0,
            80.0,
            fs,
            ni_start + 7.0 * cw + 1.0,
        ));
        let is_start = right_margin_x - 2.0 * cw;
        chars.extend(make_word_chars("is", is_start, 80.0, fs));

        // Line 3: "great"
        chars.extend(make_word_chars("great", 10.0, 60.0, fs));

        let segments = assemble_segments_from_chars(&chars, None);
        let all_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(
            all_text.contains("recognition"),
            "Expected 'recognition' in output, got: {all_text}",
        );
    }

    // ── Tests for pdfium-hyphen-based cross-line word break merging ──

    /// Create a CharInfo with `is_hyphen = true`.
    fn make_hyphen_char(ch: char, x: f32, y: f32, font_size: f32, right_x: f32) -> CharInfo {
        CharInfo {
            ch,
            x,
            y,
            font_size,
            right_x,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            has_map_error: false,
            is_symbolic: false,
            is_hyphen: true,
        }
    }

    #[test]
    fn test_line_ends_with_break_hyphen_basic() {
        let fs = 12.0;
        let cw = fs * 0.6;
        // "soft-" where '-' has is_hyphen = true
        let mut chars = make_word_chars("soft", 10.0, 100.0, fs);
        chars.push(make_hyphen_char('-', 10.0 + 4.0 * cw, 100.0, fs, 10.0 + 5.0 * cw));
        assert!(line_ends_with_break_hyphen(&chars));
    }

    #[test]
    fn test_line_ends_with_break_hyphen_not_flagged() {
        let fs = 12.0;
        let cw = fs * 0.6;
        // "word-" where '-' has is_hyphen = false (e.g., intentional hyphen in compound word)
        let mut chars = make_word_chars("word", 10.0, 100.0, fs);
        chars.push(make_char_exact('-', 10.0 + 4.0 * cw, 100.0, fs, 10.0 + 5.0 * cw));
        assert!(!line_ends_with_break_hyphen(&chars));
    }

    #[test]
    fn test_line_ends_with_break_hyphen_standalone_dash() {
        let fs = 12.0;
        // Just a dash with no preceding alphabetic — should not be treated as break hyphen
        let chars = vec![make_hyphen_char('-', 10.0, 100.0, fs, 10.0 + fs * 0.6)];
        assert!(!line_ends_with_break_hyphen(&chars));
    }

    #[test]
    fn test_line_ends_with_break_hyphen_em_dash_ignored() {
        let fs = 12.0;
        let cw = fs * 0.6;
        // "word—" with em dash that has is_hyphen = true — should NOT be treated as break
        let mut chars = make_word_chars("word", 10.0, 100.0, fs);
        chars.push(make_hyphen_char(
            '\u{2014}',
            10.0 + 4.0 * cw,
            100.0,
            fs,
            10.0 + 5.0 * cw,
        ));
        assert!(!line_ends_with_break_hyphen(&chars));
    }

    #[test]
    fn test_strip_trailing_hyphen() {
        let mut text = "soft-".to_string();
        strip_trailing_hyphen(&mut text);
        assert_eq!(text, "soft");
    }

    #[test]
    fn test_strip_trailing_hyphen_no_hyphen() {
        let mut text = "word".to_string();
        strip_trailing_hyphen(&mut text);
        assert_eq!(text, "word");
    }

    #[test]
    fn test_strip_trailing_soft_hyphen() {
        let mut text = "soft\u{00AD}".to_string();
        strip_trailing_hyphen(&mut text);
        assert_eq!(text, "soft");
    }

    #[test]
    fn test_assemble_segments_dehyphenates_pdfium_hyphen() {
        // Simulate "soft-" on line 1 and "ware" on line 2, where the hyphen
        // has is_hyphen = true from pdfium. The result should be one segment
        // containing "software".
        let fs = 12.0;
        let cw = fs * 0.6;

        let mut chars = Vec::new();
        // Line 1 at y=100: "soft-"
        chars.extend(make_word_chars("soft", 10.0, 100.0, fs));
        chars.push(make_hyphen_char('-', 10.0 + 4.0 * cw, 100.0, fs, 10.0 + 5.0 * cw));
        // Line 2 at y=80: "ware is great"
        chars.extend(make_word_chars("ware", 10.0, 80.0, fs));
        chars.push(make_char_exact(
            ' ',
            10.0 + 4.0 * cw + 1.0,
            80.0,
            fs,
            10.0 + 5.0 * cw + 1.0,
        ));
        chars.extend(make_word_chars("is", 10.0 + 5.0 * cw + 2.0, 80.0, fs));
        chars.push(make_char_exact(
            ' ',
            10.0 + 7.0 * cw + 3.0,
            80.0,
            fs,
            10.0 + 8.0 * cw + 3.0,
        ));
        chars.extend(make_word_chars("great", 10.0 + 8.0 * cw + 4.0, 80.0, fs));

        let segments = assemble_segments_from_chars(&chars, None);
        let all_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(
            all_text.contains("software"),
            "Expected 'software' (dehyphenated), got: {all_text}",
        );
        assert!(
            !all_text.contains("soft-"),
            "Trailing hyphen should have been removed, got: {all_text}",
        );
    }

    #[test]
    fn test_assemble_segments_preserves_non_break_hyphen() {
        // "well-" where the hyphen does NOT have is_hyphen=true (e.g., compound word
        // like "well-known"). Should remain as separate segments with the hyphen intact.
        let fs = 12.0;
        let cw = fs * 0.6;

        let mut chars = Vec::new();
        // Line 1 at y=100: "well-"
        chars.extend(make_word_chars("well", 10.0, 100.0, fs));
        // Regular hyphen (is_hyphen = false)
        chars.push(make_char_exact('-', 10.0 + 4.0 * cw, 100.0, fs, 10.0 + 5.0 * cw));
        // Line 2 at y=80: "known"
        chars.extend(make_word_chars("known", 10.0, 80.0, fs));

        let segments = assemble_segments_from_chars(&chars, None);
        let all_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(
            all_text.contains("well-"),
            "Non-break hyphen should be preserved, got: {all_text}",
        );
        assert!(
            !all_text.contains("wellknown"),
            "Should NOT merge across non-break hyphen, got: {all_text}",
        );
    }

    #[test]
    fn test_assemble_segments_multi_hyphen_chain() {
        // "config-" on line 1, "ura-" on line 2, "tion" on line 3.
        // All hyphens have is_hyphen = true. Should produce "configuration".
        let fs = 12.0;
        let cw = fs * 0.6;

        let mut chars = Vec::new();
        // Line 1 at y=120: "config-"
        chars.extend(make_word_chars("config", 10.0, 120.0, fs));
        chars.push(make_hyphen_char('-', 10.0 + 6.0 * cw, 120.0, fs, 10.0 + 7.0 * cw));
        // Line 2 at y=100: "ura-"
        chars.extend(make_word_chars("ura", 10.0, 100.0, fs));
        chars.push(make_hyphen_char('-', 10.0 + 3.0 * cw, 100.0, fs, 10.0 + 4.0 * cw));
        // Line 3 at y=80: "tion"
        chars.extend(make_word_chars("tion", 10.0, 80.0, fs));

        let segments = assemble_segments_from_chars(&chars, None);
        let all_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(
            all_text.contains("configuration"),
            "Expected 'configuration' from multi-line dehyphenation, got: {all_text}",
        );
    }
}
