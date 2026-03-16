//! Layout-guided segment assembly using model bounding boxes.
//!
//! When layout detection is enabled, this module assigns text segments to
//! layout regions *before* line/paragraph assembly, ensuring paragraph
//! boundaries align with the model's structural predictions.

mod assignment;
mod heading;
mod merge;
mod reading_order;
pub(super) mod table_recognition;
mod tables;

use crate::pdf::hierarchy::SegmentData;

use super::classify::classify_paragraphs;
use super::columns::split_segments_into_columns;
use super::lines::segments_to_lines;
use super::paragraphs::{lines_to_paragraphs, merge_continuation_paragraphs};
use super::types::{LayoutHint, LayoutHintClass, PdfParagraph};

// Re-exports for use by pipeline.rs and other siblings
pub(super) use heading::looks_like_figure_label;
#[cfg(feature = "layout-detection")]
pub(super) use table_recognition::recognize_tables_for_native_page;
pub(super) use tables::extract_tables_from_layout_hints;

/// A layout region with its assigned segment indices.
struct LayoutRegion<'a> {
    hint: &'a LayoutHint,
    segment_indices: Vec<usize>,
    /// Overridden bounding box after merging fragmented regions.
    /// When set, spatial comparisons use this instead of `hint`'s bbox.
    merged_bbox: Option<(f32, f32, f32, f32)>, // (left, bottom, right, top)
}

impl<'a> LayoutRegion<'a> {
    /// Effective bounding box: merged union if available, otherwise hint's bbox.
    fn bbox(&self) -> (f32, f32, f32, f32) {
        self.merged_bbox
            .unwrap_or((self.hint.left, self.hint.bottom, self.hint.right, self.hint.top))
    }
}

/// Assemble paragraphs using layout-region-guided segment assignment.
///
/// Instead of assembling all segments into paragraphs first and then matching
/// to layout hints, this assigns segments to layout regions *before* assembly.
/// Each region's segments are independently assembled into lines and paragraphs,
/// then the region's layout class is applied directly.
///
/// Segments not covered by any layout region fall through to the standard
/// pipeline (XY-Cut → lines → paragraphs → font-size classification).
pub(super) fn assemble_region_paragraphs(
    segments: Vec<SegmentData>,
    hints: &[LayoutHint],
    heading_map: &[(f32, Option<u8>)],
    min_confidence: f32,
    doc_body_font_size: Option<f32>,
    page_index: usize,
    extracted_table_bboxes: &[crate::types::BoundingBox],
) -> Vec<PdfParagraph> {
    // Page-level text quality gate: if the overall page text is mostly non-alphanumeric
    // (e.g., music notation, symbol-heavy content), the layout model's regions are
    // likely to produce garbled output. Fall back to standard pipeline.
    let all_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    let total_chars = all_text.chars().count();
    let alnum_chars = all_text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .count();
    if total_chars >= 20 && (alnum_chars as f32 / total_chars as f32) < 0.4 {
        tracing::trace!(
            page = page_index,
            total_chars,
            alnum_chars,
            ratio = alnum_chars as f32 / total_chars as f32,
            "page text is mostly non-alphanumeric — skipping layout-guided assembly"
        );
        return assemble_fallback(segments, heading_map);
    }

    // Pre-assignment quality gate: detect multi-column pages where the layout
    // model produces many narrow Text hints (each ~single-column width).
    // On such pages, the assignment step clips text segments at region boundaries,
    // losing words near the column gutter. The baseline XY-Cut pipeline handles
    // two-column layouts correctly, so redirect the entire page to fallback.
    {
        let page_left_pre = segments.iter().map(|s| s.x).fold(f32::MAX, f32::min);
        let page_right_pre = segments.iter().map(|s| s.x + s.width).fold(0.0_f32, f32::max);
        let page_width_pre = page_right_pre - page_left_pre;
        let is_multicol = detect_page_multicolumn(&segments, page_width_pre, page_left_pre);
        if is_multicol && page_width_pre > 0.0 {
            let confident_text_hints: Vec<&LayoutHint> = hints
                .iter()
                .filter(|h| h.confidence >= min_confidence && h.class == LayoutHintClass::Text)
                .collect();
            if confident_text_hints.len() >= 4 {
                let narrow_count = confident_text_hints
                    .iter()
                    .filter(|h| (h.right - h.left) < page_width_pre * 0.55)
                    .count();
                if narrow_count as f32 / confident_text_hints.len() as f32 >= 0.75 {
                    tracing::trace!(
                        page = page_index,
                        text_hints = confident_text_hints.len(),
                        narrow_count,
                        "multi-column page with narrow Text hints — skipping layout assembly"
                    );
                    return assemble_fallback(segments, heading_map);
                }
            }
        }
    }

    let (mut regions, unassigned_indices) =
        assignment::assign_segments_to_regions_refined(&segments, hints, min_confidence, extracted_table_bboxes);

    if regions.is_empty() {
        tracing::trace!(page = page_index, "no layout regions — using fallback pipeline");
        return assemble_fallback(segments, heading_map);
    }

    tracing::trace!(
        page = page_index,
        regions = regions.len(),
        unassigned = unassigned_indices.len(),
        "layout regions assigned"
    );

    // Determine page extents for reading order and TOC detection
    let page_height = segments.iter().map(|s| s.y + s.height).fold(0.0_f32, f32::max);
    let page_left = segments.iter().map(|s| s.x).fold(f32::MAX, f32::min);
    let page_right = segments.iter().map(|s| s.x + s.width).fold(0.0_f32, f32::max);
    let page_width = page_right - page_left;

    // Detect Text regions that span multiple columns and redirect to fallback.
    // When the layout model produces a Text region covering both columns of a
    // multi-column page, the region assembly interleaves left/right column text
    // into a single paragraph. The baseline XY-Cut pipeline handles this correctly
    // via column splitting.
    //
    // Two checks:
    // 1. Oversized regions (>= 30% page area and wide) — likely full-page blocks.
    // 2. Multi-column regions — wide regions where segments cluster into two
    //    distinct X-bands, indicating they span a column gutter.
    // Page-level multi-column detection: if the page's segments cluster into
    // two distinct X-bands, ALL wide Text regions should use the fallback
    // pipeline (XY-Cut column splitting) regardless of per-region segment count.
    // This prevents text interleaving when the layout model creates regions
    // that span both columns of a 2-column page.
    let page_is_multicolumn = detect_page_multicolumn(&segments, page_width, page_left);

    let mut extra_unassigned: Vec<usize> = Vec::new();
    if page_width > 0.0 && page_height > 0.0 {
        let page_area = page_width * page_height;

        for region in regions.iter_mut() {
            if region.hint.class != LayoutHintClass::Text || region.segment_indices.len() < 4 {
                continue;
            }

            let region_width = region.hint.right - region.hint.left;
            let region_height = region.hint.top - region.hint.bottom;
            let region_area = region_width * region_height;

            // Check 1: oversized region (area-based).
            // A Text region covering >= 25% of page area and >= 65% of page width
            // is likely a TOC, multi-paragraph block, or full-page text that the
            // fallback XY-Cut pipeline handles more accurately.
            if region_area >= page_area * 0.25 && region_width >= page_width * 0.65 {
                tracing::trace!(
                    page = page_index,
                    segments = region.segment_indices.len(),
                    area_fraction = region_area / page_area,
                    "oversized Text region detected — redirecting to fallback pipeline"
                );
                extra_unassigned.append(&mut region.segment_indices);
                continue;
            }

            // Check 1b: dense Text region — many segments in a wide region.
            // Regions with >= 15 segments spanning >= 50% of page width are
            // likely TOC pages, dense lists, or multi-paragraph blocks where
            // segment-to-line assembly within a single region merges entries
            // that should stay separate. The fallback pipeline's column
            // splitting preserves the correct line breaks.
            if region.segment_indices.len() >= 15 && region_width >= page_width * 0.50 {
                tracing::trace!(
                    page = page_index,
                    segments = region.segment_indices.len(),
                    region_width,
                    page_width,
                    "dense Text region detected — redirecting to fallback pipeline"
                );
                extra_unassigned.append(&mut region.segment_indices);
                continue;
            }

            // Check 2: multi-column page with wide region.
            // On pages detected as multi-column, any Text region spanning >= 55%
            // of page width likely covers both columns. Redirect to fallback where
            // XY-Cut column splitting handles the layout correctly.
            if page_is_multicolumn && region_width >= page_width * 0.55 {
                tracing::trace!(
                    page = page_index,
                    segments = region.segment_indices.len(),
                    region_width,
                    page_width,
                    "wide Text region on multi-column page — redirecting to fallback pipeline"
                );
                extra_unassigned.append(&mut region.segment_indices);
                continue;
            }

            // Check 3: multi-column detection for wide regions (per-region check).
            // A region wider than 55% of page width with >= 6 segments might span
            // two columns. Check if segments cluster into left/right halves.
            if region.segment_indices.len() >= 6 && region_width >= page_width * 0.55 {
                let mid_x = (region.hint.left + region.hint.right) / 2.0;
                let mut left_count = 0_usize;
                let mut right_count = 0_usize;
                for &idx in &region.segment_indices {
                    let seg_cx = segments[idx].x + segments[idx].width / 2.0;
                    if seg_cx < mid_x {
                        left_count += 1;
                    } else {
                        right_count += 1;
                    }
                }
                let total = left_count + right_count;
                let minority = left_count.min(right_count);
                // If both halves have >= 25% of segments, this is likely two columns
                if total > 0 && minority as f32 / total as f32 >= 0.25 {
                    tracing::trace!(
                        page = page_index,
                        segments = total,
                        left_count,
                        right_count,
                        region_width,
                        "multi-column Text region detected — redirecting to fallback pipeline"
                    );
                    extra_unassigned.append(&mut region.segment_indices);
                }
            }
        }
    }

    // Pre-merge fragmented Title/SectionHeader regions before reading order.
    // The layout model sometimes splits a single semantic element (e.g., a multi-line
    // title) into multiple overlapping regions. Merging prevents the reading order
    // from interleaving them with unrelated regions.
    merge_fragmented_regions(&mut regions);

    reading_order::order_regions_reading_order(&mut regions, page_height);

    let mut all_paragraphs: Vec<PdfParagraph> = Vec::new();

    // Assemble paragraphs per region
    for region in &regions {
        if region.segment_indices.is_empty() {
            continue;
        }

        let region_segments: Vec<SegmentData> = region
            .segment_indices
            .iter()
            .map(|&idx| segments[idx].clone())
            .collect();

        let lines = segments_to_lines(region_segments);
        let mut paragraphs = lines_to_paragraphs(lines);

        // For ListItem regions, the layout model identifies one bbox per list item.
        // If paragraph splitting created multiple paragraphs, merge them back into
        // a single list item before applying the class.
        if region.hint.class == LayoutHintClass::ListItem && paragraphs.len() > 1 {
            let mut merged_lines = Vec::new();
            for para in paragraphs.drain(..) {
                merged_lines.extend(para.lines);
            }
            paragraphs.push(super::paragraphs::finalize_paragraph(merged_lines));
        }

        // Text quality gate: skip regions with garbled/non-text content.
        // Only applied to regions NOT classified as Text, SectionHeader, or Title,
        // since those are inherently text regions.
        if !matches!(
            region.hint.class,
            LayoutHintClass::Text | LayoutHintClass::SectionHeader | LayoutHintClass::Title
        ) {
            let region_text: String = paragraphs
                .iter()
                .flat_map(|p| p.lines.iter())
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            let total = region_text.chars().count();
            let alnum = region_text
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .count();
            if total >= 10 && (alnum as f32 / total as f32) < 0.3 {
                tracing::trace!(
                    class = ?region.hint.class,
                    total_chars = total,
                    alnum_chars = alnum,
                    "skipping garbled region"
                );
                continue;
            }
        }

        heading::apply_region_class(
            &mut paragraphs,
            region.hint,
            heading_map,
            doc_body_font_size,
            page_height,
            page_index,
        );

        all_paragraphs.extend(paragraphs);
    }

    // Handle unassigned segments (including oversized-region-redirected) via standard pipeline
    let mut all_unassigned = unassigned_indices;
    all_unassigned.extend(extra_unassigned);
    if !all_unassigned.is_empty() {
        let unassigned_segments: Vec<SegmentData> = all_unassigned.iter().map(|&idx| segments[idx].clone()).collect();
        let mut fallback = assemble_fallback(unassigned_segments, heading_map);
        all_paragraphs.append(&mut fallback);
    }

    // Merge continuation paragraphs, but only within same layout class
    merge::merge_continuation_paragraphs_region_aware(&mut all_paragraphs);

    // Merge consecutive code blocks (layout model often gives one region per line)
    merge::merge_consecutive_code_blocks(&mut all_paragraphs);

    // Validate code blocks: reject those that look like image data or artifacts
    // rather than actual code (e.g., hex dumps from embedded images).
    merge::demote_non_code_blocks(&mut all_paragraphs);

    // Merge list item continuations (layout model may split one reference across bboxes)
    merge::merge_list_continuations(&mut all_paragraphs);

    // Associate captions with their parent table/picture elements
    associate_captions(&mut all_paragraphs);

    // Associate footnotes with their preceding table/picture elements
    associate_footnotes(&mut all_paragraphs);

    all_paragraphs
}

/// Detect whether a page has a multi-column layout by analyzing segment positions.
///
/// Checks if segments cluster into two or more distinct horizontal bands with a
/// clear gutter between them. A page is considered multi-column if:
/// 1. There are enough segments to detect columns (>= 8).
/// 2. Segments form two distinct X-position clusters with a gap > 10% of page width.
/// 3. Both clusters have a meaningful number of segments (>= 20% each).
fn detect_page_multicolumn(segments: &[SegmentData], page_width: f32, page_left: f32) -> bool {
    if segments.len() < 8 || page_width <= 0.0 {
        return false;
    }

    // Collect left-edge x positions of all segments (relative to page left)
    let mut x_positions: Vec<f32> = segments
        .iter()
        .filter(|s| !s.text.trim().is_empty() && s.width > 0.0)
        .map(|s| s.x - page_left)
        .collect();

    if x_positions.len() < 8 {
        return false;
    }

    x_positions.sort_by(|a, b| a.total_cmp(b));

    // Look for a gap in the distribution of x positions that indicates a column gutter.
    // The gutter should be:
    // - At least 10% of page width
    // - Located roughly in the middle half of the page (25%-75%)
    // - Dividing segments into two groups of at least 20% each
    let n = x_positions.len();
    let min_group_size = (n as f32 * 0.20).ceil() as usize;
    let gutter_threshold = page_width * 0.10;

    for i in min_group_size..n.saturating_sub(min_group_size) {
        let gap = x_positions[i] - x_positions[i - 1];
        if gap >= gutter_threshold {
            // Check that the gap is in the middle half of the page
            let gap_center = (x_positions[i] + x_positions[i - 1]) / 2.0;
            let relative_position = gap_center / page_width;
            if (0.25..=0.75).contains(&relative_position) {
                tracing::trace!(
                    gap,
                    gap_center,
                    left_count = i,
                    right_count = n - i,
                    "page multi-column detection: gutter found"
                );
                return true;
            }
        }
    }

    false
}

/// Maximum gap (in points) between regions to consider them adjacent for merging.
const MERGE_GAP_THRESHOLD: f32 = 5.0;

/// Merge adjacent/overlapping Title and SectionHeader regions.
///
/// When the layout model fragments a single semantic element (e.g., a multi-line
/// title) into multiple regions, the reading order can interleave them with
/// unrelated regions. This merges same-class regions that overlap or are within
/// a small gap, producing a single region with the union bbox and combined segments.
fn merge_fragmented_regions(regions: &mut Vec<LayoutRegion>) {
    if regions.len() < 2 {
        return;
    }

    let mut merged_count = 0;
    let mut i = 0;
    while i < regions.len() {
        // Only merge Title and SectionHeader regions
        if !matches!(
            regions[i].hint.class,
            LayoutHintClass::Title | LayoutHintClass::SectionHeader
        ) {
            i += 1;
            continue;
        }

        let mut j = i + 1;
        while j < regions.len() {
            if regions[j].hint.class != regions[i].hint.class {
                j += 1;
                continue;
            }

            // Check if bboxes overlap or are within gap threshold
            let h_gap = (regions[j].hint.left - regions[i].hint.right)
                .max(regions[i].hint.left - regions[j].hint.right)
                .max(0.0);
            let v_gap = (regions[j].hint.bottom - regions[i].hint.top)
                .max(regions[i].hint.bottom - regions[j].hint.top)
                .max(0.0);

            // Check Y-band proximity: vertical centers within 50% of the smaller region's height
            let i_height = regions[i].hint.top - regions[i].hint.bottom;
            let j_height = regions[j].hint.top - regions[j].hint.bottom;
            let min_height = i_height.min(j_height);
            let i_cy = (regions[i].hint.top + regions[i].hint.bottom) / 2.0;
            let j_cy = (regions[j].hint.top + regions[j].hint.bottom) / 2.0;
            let cy_gap = (i_cy - j_cy).abs();

            let in_same_band = cy_gap <= min_height.max(1.0) * 0.5;
            let close_enough = h_gap <= MERGE_GAP_THRESHOLD && v_gap <= MERGE_GAP_THRESHOLD;

            if close_enough || (in_same_band && (h_gap <= MERGE_GAP_THRESHOLD || v_gap <= MERGE_GAP_THRESHOLD)) {
                // Merge j into i: take segment indices and update merged bbox
                let j_segments = std::mem::take(&mut regions[j].segment_indices);
                regions[i].segment_indices.extend(j_segments);

                let (i_left, i_bottom, i_right, i_top) = regions[i].bbox();
                let (j_left, j_bottom, j_right, j_top) = regions[j].bbox();
                regions[i].merged_bbox = Some((
                    i_left.min(j_left),
                    i_bottom.min(j_bottom),
                    i_right.max(j_right),
                    i_top.max(j_top),
                ));

                regions.remove(j);
                merged_count += 1;
                // Don't increment j — check next element at same index
            } else {
                j += 1;
            }
        }
        i += 1;
    }

    if merged_count > 0 {
        tracing::trace!(merged = merged_count, "merged fragmented Title/SectionHeader regions");
    }
}

/// Associate CAPTION paragraphs with their nearest TABLE or PICTURE parent.
///
/// Scans backward and forward from each caption for adjacent table/picture
/// elements. Unambiguous cases (only one direction) are assigned first;
/// ambiguous cases prefer the closer element by index distance.
fn associate_captions(paragraphs: &mut [PdfParagraph]) {
    // First pass: collect caption indices
    let caption_indices: Vec<usize> = paragraphs
        .iter()
        .enumerate()
        .filter(|(_, p)| p.layout_class == Some(LayoutHintClass::Caption))
        .map(|(i, _)| i)
        .collect();

    if caption_indices.is_empty() {
        return;
    }

    // For each caption, find nearest table/picture in both directions
    for &cap_idx in &caption_indices {
        let backward = find_parent_backward(paragraphs, cap_idx);
        let forward = find_parent_forward(paragraphs, cap_idx);

        let parent_idx = match (backward, forward) {
            (Some(b), None) => Some(b),
            (None, Some(f)) => Some(f),
            (Some(b), Some(f)) => {
                // Prefer the closer one by index distance
                if (cap_idx - b) <= (f - cap_idx) {
                    Some(b)
                } else {
                    Some(f)
                }
            }
            (None, None) => None, // No parent found — leave as body text
        };

        if let Some(pi) = parent_idx {
            paragraphs[cap_idx].caption_for = Some(pi);
        }
    }
}

/// Scan backward from `cap_idx` for the nearest table/picture/code paragraph.
/// Skips other captions but stops at any non-caption, non-parent element.
fn find_parent_backward(paragraphs: &[PdfParagraph], cap_idx: usize) -> Option<usize> {
    for i in (0..cap_idx).rev() {
        let class = paragraphs[i].layout_class;
        if class == Some(LayoutHintClass::Table)
            || class == Some(LayoutHintClass::Picture)
            || class == Some(LayoutHintClass::Code)
        {
            return Some(i);
        }
        if class == Some(LayoutHintClass::Caption) {
            continue; // Skip other captions
        }
        break; // Non-caption, non-parent element — stop
    }
    None
}

/// Scan forward from `cap_idx` for the nearest table/picture/code paragraph.
fn find_parent_forward(paragraphs: &[PdfParagraph], cap_idx: usize) -> Option<usize> {
    for (offset, p) in paragraphs[(cap_idx + 1)..].iter().enumerate() {
        let class = p.layout_class;
        if class == Some(LayoutHintClass::Table)
            || class == Some(LayoutHintClass::Picture)
            || class == Some(LayoutHintClass::Code)
        {
            return Some(cap_idx + 1 + offset);
        }
        if class == Some(LayoutHintClass::Caption) {
            continue;
        }
        break;
    }
    None
}

/// Associate FOOTNOTE paragraphs with their preceding TABLE or PICTURE parent.
///
/// For each table/picture, scans forward for consecutive footnote paragraphs
/// and associates them. Stops at any non-footnote element (including captions),
/// matching Docling's behavior.
fn associate_footnotes(paragraphs: &mut [PdfParagraph]) {
    // Collect indices of table/picture paragraphs
    let parent_indices: Vec<usize> = paragraphs
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            matches!(
                p.layout_class,
                Some(LayoutHintClass::Table) | Some(LayoutHintClass::Picture)
            )
        })
        .map(|(i, _)| i)
        .collect();

    for &parent_idx in &parent_indices {
        // Scan forward from the parent for consecutive footnotes
        for item in paragraphs.iter_mut().skip(parent_idx + 1) {
            let class = item.layout_class;
            if class == Some(LayoutHintClass::Footnote) {
                item.caption_for = Some(parent_idx);
            } else {
                break; // Any non-footnote element — stop (including captions)
            }
        }
    }
}

/// Standard pipeline fallback for segments not covered by layout regions.
fn assemble_fallback(segments: Vec<SegmentData>, heading_map: &[(f32, Option<u8>)]) -> Vec<PdfParagraph> {
    let column_groups = split_segments_into_columns(&segments);
    let mut paragraphs: Vec<PdfParagraph> = if column_groups.len() <= 1 {
        let lines = segments_to_lines(segments);
        lines_to_paragraphs(lines)
    } else {
        let mut all_paragraphs = Vec::new();
        for group in column_groups {
            let col_segments: Vec<_> = group.into_iter().map(|idx| segments[idx].clone()).collect();
            let lines = segments_to_lines(col_segments);
            all_paragraphs.extend(lines_to_paragraphs(lines));
        }
        all_paragraphs
    };
    classify_paragraphs(&mut paragraphs, heading_map);
    merge_continuation_paragraphs(&mut paragraphs);
    paragraphs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::SegmentData;

    fn make_segment(text: &str, x: f32, y: f32, width: f32, height: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            font_size: height,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: y,
        }
    }

    fn make_hint(class: LayoutHintClass, confidence: f32, left: f32, bottom: f32, right: f32, top: f32) -> LayoutHint {
        LayoutHint {
            class,
            confidence,
            left,
            bottom,
            right,
            top,
        }
    }

    #[test]
    fn test_assign_segments_single_region() {
        let segments = vec![
            make_segment("Hello", 10.0, 700.0, 40.0, 12.0),
            make_segment("world", 55.0, 700.0, 40.0, 12.0),
        ];
        let hints = vec![make_hint(LayoutHintClass::Text, 0.9, 0.0, 690.0, 200.0, 720.0)];
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].segment_indices.len(), 2);
        assert!(unassigned.is_empty());
    }

    #[test]
    fn test_assign_segments_two_columns() {
        let segments = vec![
            make_segment("Left", 10.0, 700.0, 40.0, 12.0),
            make_segment("Right", 300.0, 700.0, 40.0, 12.0),
        ];
        let hints = vec![
            make_hint(LayoutHintClass::Text, 0.9, 0.0, 690.0, 200.0, 720.0),
            make_hint(LayoutHintClass::Text, 0.9, 250.0, 690.0, 500.0, 720.0),
        ];
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].segment_indices.len(), 1);
        assert_eq!(regions[1].segment_indices.len(), 1);
        assert!(unassigned.is_empty());
    }

    #[test]
    fn test_assign_segments_unassigned() {
        let segments = vec![
            make_segment("Inside", 10.0, 700.0, 40.0, 12.0),
            make_segment("Outside", 500.0, 100.0, 40.0, 12.0),
        ];
        let hints = vec![make_hint(LayoutHintClass::Text, 0.9, 0.0, 690.0, 200.0, 720.0)];
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        assert_eq!(regions[0].segment_indices.len(), 1);
        assert_eq!(unassigned.len(), 1);
    }

    #[test]
    fn test_assign_segments_smallest_area_wins() {
        let segments = vec![make_segment("text", 50.0, 700.0, 40.0, 12.0)];
        let hints = vec![
            make_hint(LayoutHintClass::Text, 0.9, 0.0, 0.0, 600.0, 800.0), // large
            make_hint(LayoutHintClass::Code, 0.9, 30.0, 690.0, 200.0, 720.0), // small
        ];
        let (regions, _) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        // Segment should be in the Code region (smaller area)
        assert!(regions[0].segment_indices.is_empty()); // Text (large)
        assert_eq!(regions[1].segment_indices.len(), 1); // Code (small)
    }

    #[test]
    fn test_overlap_partial_assigns() {
        // Segment straddles the right edge of a region — partial overlap should still assign
        // Segment: x=180, w=40 → spans [180, 220], region right=200
        // Center point at x=200 is exactly on the boundary; overlap approach should capture it
        let segments = vec![make_segment("straddling", 180.0, 700.0, 40.0, 12.0)];
        let hints = vec![make_hint(LayoutHintClass::Text, 0.9, 0.0, 690.0, 200.0, 720.0)];
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        // With overlap-based assignment, partial overlap should assign the segment
        assert_eq!(regions[0].segment_indices.len(), 1);
        assert!(unassigned.is_empty());
    }

    #[test]
    fn test_overlap_below_threshold_unassigned() {
        // Segment barely touches region edge — overlap too small to assign
        // Segment: x=195, w=40 → spans [195, 235], region right=200
        // Only 5pt of 40pt width overlaps = 12.5% < 20% threshold
        let segments = vec![make_segment("barely", 195.0, 700.0, 40.0, 12.0)];
        let hints = vec![make_hint(LayoutHintClass::Text, 0.9, 0.0, 690.0, 200.0, 720.0)];
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        assert!(regions[0].segment_indices.is_empty());
        assert_eq!(unassigned.len(), 1);
    }

    #[test]
    fn test_highest_overlap_wins() {
        // Segment overlaps two regions — should go to the one with higher IoS
        // Segment: x=90, w=40 → spans [90, 130]
        // Region 1: [0, 110] → overlap = 20pt, IoS = 20/40 = 0.50
        // Region 2: [100, 300] → overlap = 30pt, IoS = 30/40 = 0.75
        let segments = vec![make_segment("overlapping", 90.0, 700.0, 40.0, 12.0)];
        let hints = vec![
            make_hint(LayoutHintClass::Text, 0.9, 0.0, 690.0, 110.0, 720.0),
            make_hint(LayoutHintClass::Text, 0.9, 100.0, 690.0, 300.0, 720.0),
        ];
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        // Should go to region 2 (higher IoS)
        assert!(regions[0].segment_indices.is_empty());
        assert_eq!(regions[1].segment_indices.len(), 1);
        assert!(unassigned.is_empty());
    }

    #[test]
    fn test_center_point_regression() {
        // Segments fully inside regions should still work (regression test)
        let segments = vec![make_segment("centered", 50.0, 700.0, 40.0, 12.0)];
        let hints = vec![make_hint(LayoutHintClass::Text, 0.9, 0.0, 690.0, 200.0, 720.0)];
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        assert_eq!(regions[0].segment_indices.len(), 1);
        assert!(unassigned.is_empty());
    }

    #[test]
    fn test_reading_order_two_columns() {
        let hints = [
            make_hint(LayoutHintClass::Text, 0.9, 300.0, 400.0, 550.0, 700.0), // right column
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 400.0, 250.0, 700.0),  // left column
        ];
        let mut regions: Vec<LayoutRegion> = hints
            .iter()
            .map(|h| LayoutRegion {
                hint: h,
                segment_indices: Vec::new(),
                merged_bbox: None,
            })
            .collect();
        reading_order::order_regions_reading_order(&mut regions, 800.0);
        // Left column should come first (same y-band, smaller x)
        assert!(regions[0].hint.left < regions[1].hint.left);
    }

    #[test]
    fn test_reading_order_vertical() {
        let hints = [
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 100.0, 500.0, 300.0), // bottom
            make_hint(LayoutHintClass::Title, 0.9, 10.0, 600.0, 500.0, 750.0), // top
        ];
        let mut regions: Vec<LayoutRegion> = hints
            .iter()
            .map(|h| LayoutRegion {
                hint: h,
                segment_indices: Vec::new(),
                merged_bbox: None,
            })
            .collect();
        reading_order::order_regions_reading_order(&mut regions, 800.0);
        // Title (top of page, higher Y) should come first
        assert_eq!(regions[0].hint.class, LayoutHintClass::Title);
    }

    #[test]
    fn test_assemble_code_region() {
        let segments = vec![
            make_segment("fn main() {", 10.0, 700.0, 80.0, 12.0),
            make_segment("println!(\"hi\");", 10.0, 685.0, 100.0, 12.0),
            make_segment("}", 10.0, 670.0, 10.0, 12.0),
        ];
        let hints = vec![make_hint(LayoutHintClass::Code, 0.9, 0.0, 660.0, 200.0, 720.0)];
        let paragraphs = assemble_region_paragraphs(segments, &hints, &[], 0.5, None, 0, &[]);
        assert!(!paragraphs.is_empty());
        assert!(paragraphs[0].is_code_block);
    }

    #[test]
    fn test_assemble_heading_region() {
        let segments = vec![make_segment("1 Introduction", 10.0, 700.0, 120.0, 18.0)];
        let hints = vec![make_hint(LayoutHintClass::SectionHeader, 0.9, 0.0, 690.0, 200.0, 725.0)];
        let paragraphs = assemble_region_paragraphs(segments, &hints, &[], 0.5, None, 0, &[]);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_low_confidence_hints_ignored() {
        let segments = vec![make_segment("text", 10.0, 700.0, 40.0, 12.0)];
        let hints = vec![make_hint(LayoutHintClass::Code, 0.3, 0.0, 690.0, 200.0, 720.0)];
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        assert!(regions.is_empty());
        assert_eq!(unassigned.len(), 1);
    }

    #[test]
    fn test_table_regions_excluded_when_extracted() {
        // Segments within successfully extracted table bboxes are dropped.
        let segments = vec![make_segment("text", 10.0, 700.0, 40.0, 12.0)];
        let hints = vec![make_hint(LayoutHintClass::Table, 0.9, 0.0, 690.0, 200.0, 720.0)];
        let extracted_bbox = crate::types::BoundingBox {
            x0: 0.0,
            y0: 690.0,
            x1: 200.0,
            y1: 720.0,
        };
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[extracted_bbox]);
        assert!(regions.is_empty());
        assert_eq!(unassigned.len(), 0); // dropped, not unassigned
    }

    #[test]
    fn test_table_regions_recovered_when_extraction_fails() {
        // Segments in Table hints that did NOT produce tables should fall through.
        let segments = vec![make_segment("text", 10.0, 700.0, 40.0, 12.0)];
        let hints = vec![make_hint(LayoutHintClass::Table, 0.9, 0.0, 690.0, 200.0, 720.0)];
        // No extracted bboxes → table extraction failed for this hint
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        assert!(regions.is_empty());
        assert_eq!(unassigned.len(), 1); // recovered to unassigned
    }

    #[test]
    fn test_picture_regions_excluded_and_segments_suppressed() {
        // Picture regions are excluded from region assignment and segments
        // inside them are suppressed (IoS >= 0.5) to avoid OCR artifacts.
        let segments = vec![make_segment("text", 10.0, 700.0, 40.0, 12.0)];
        let hints = vec![make_hint(LayoutHintClass::Picture, 0.9, 0.0, 690.0, 200.0, 720.0)];
        let (regions, unassigned) = assignment::assign_segments_to_regions(&segments, &hints, 0.5, &[]);
        assert!(regions.is_empty());
        assert_eq!(unassigned.len(), 0); // suppressed, not fallback
    }

    #[test]
    fn test_assemble_mixed_regions() {
        // Title at top, body text below, code at bottom
        let segments = vec![
            make_segment("Title Text", 10.0, 750.0, 100.0, 18.0),
            make_segment("Body paragraph here.", 10.0, 700.0, 150.0, 12.0),
            make_segment("let x = 1;", 10.0, 650.0, 80.0, 12.0),
        ];
        let hints = vec![
            make_hint(LayoutHintClass::Title, 0.9, 0.0, 740.0, 200.0, 775.0),
            make_hint(LayoutHintClass::Text, 0.9, 0.0, 690.0, 200.0, 720.0),
            make_hint(LayoutHintClass::Code, 0.9, 0.0, 640.0, 200.0, 665.0),
        ];
        let paragraphs = assemble_region_paragraphs(segments, &hints, &[], 0.5, None, 0, &[]);
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(paragraphs[0].heading_level, Some(1)); // Title
        assert_eq!(paragraphs[0].layout_class, Some(LayoutHintClass::Title));
        assert!(paragraphs[1].heading_level.is_none()); // Body
        assert!(paragraphs[2].is_code_block); // Code
    }

    #[test]
    fn test_reading_order_fullwidth_interleaved_with_columns() {
        // Page layout: full-width title at top, two text columns below, full-width footer
        // Page is 560pt wide, 800pt tall (PDF coords: y=0 at bottom)
        let hints = [
            make_hint(LayoutHintClass::Title, 0.9, 10.0, 700.0, 550.0, 780.0), // full-width top
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 300.0, 260.0, 690.0),  // left column
            make_hint(LayoutHintClass::Text, 0.9, 280.0, 300.0, 550.0, 690.0), // right column
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 100.0, 550.0, 290.0),  // full-width bottom
        ];
        let mut regions: Vec<LayoutRegion> = hints
            .iter()
            .map(|h| LayoutRegion {
                hint: h,
                segment_indices: Vec::new(),
                merged_bbox: None,
            })
            .collect();
        reading_order::order_regions_reading_order(&mut regions, 800.0);
        // Expected order: Title (top), Left col, Right col, Bottom full-width
        assert_eq!(regions[0].hint.class, LayoutHintClass::Title); // full-width top
        assert!(regions[1].hint.right < 280.0, "Second should be left column"); // left col
        assert!(regions[2].hint.left >= 270.0, "Third should be right column"); // right col
        assert!(regions[3].hint.top < 300.0, "Fourth should be bottom full-width"); // bottom
    }

    #[test]
    fn test_reading_order_all_narrow_regression() {
        // No full-width elements — should behave same as before (column detection)
        let hints = [
            make_hint(LayoutHintClass::Text, 0.9, 300.0, 400.0, 550.0, 700.0), // right col top
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 400.0, 250.0, 700.0),  // left col top
            make_hint(LayoutHintClass::Text, 0.9, 300.0, 100.0, 550.0, 390.0), // right col bottom
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 100.0, 250.0, 390.0),  // left col bottom
        ];
        let mut regions: Vec<LayoutRegion> = hints
            .iter()
            .map(|h| LayoutRegion {
                hint: h,
                segment_indices: Vec::new(),
                merged_bbox: None,
            })
            .collect();
        reading_order::order_regions_reading_order(&mut regions, 800.0);
        // Left column first (both top and bottom), then right column
        assert!(regions[0].hint.left < 260.0); // left col
        assert!(regions[1].hint.left < 260.0); // left col
        assert!(regions[2].hint.left >= 260.0); // right col
        assert!(regions[3].hint.left >= 260.0); // right col
    }

    // ── Reading Order Tests ──────────────────────────────────────────────────

    #[test]
    fn test_dag_reading_order_three_columns() {
        // Three columns: left (x=10-100), middle (x=150-240), right (x=290-380).
        // Two rows of regions per column so the DAG path has >= 4 body regions total.
        let hints = [
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 400.0, 100.0, 700.0), // left top
            make_hint(LayoutHintClass::Text, 0.9, 150.0, 400.0, 240.0, 700.0), // mid top
            make_hint(LayoutHintClass::Text, 0.9, 290.0, 400.0, 380.0, 700.0), // right top
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 100.0, 100.0, 390.0), // left bot
            make_hint(LayoutHintClass::Text, 0.9, 150.0, 100.0, 240.0, 390.0), // mid bot
            make_hint(LayoutHintClass::Text, 0.9, 290.0, 100.0, 380.0, 390.0), // right bot
        ];
        let mut regions: Vec<LayoutRegion> = hints
            .iter()
            .map(|h| LayoutRegion {
                hint: h,
                segment_indices: Vec::new(),
                merged_bbox: None,
            })
            .collect();
        reading_order::order_regions_reading_order(&mut regions, 800.0);

        // Left column should appear before middle, middle before right.
        let left_positions: Vec<usize> = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.hint.left < 120.0)
            .map(|(i, _)| i)
            .collect();
        let mid_positions: Vec<usize> = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.hint.left >= 120.0 && r.hint.left < 260.0)
            .map(|(i, _)| i)
            .collect();
        let right_positions: Vec<usize> = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.hint.left >= 260.0)
            .map(|(i, _)| i)
            .collect();

        assert_eq!(left_positions.len(), 2);
        assert_eq!(mid_positions.len(), 2);
        assert_eq!(right_positions.len(), 2);

        // All left positions come before all mid positions
        assert!(left_positions.iter().all(|&lp| mid_positions.iter().all(|&mp| lp < mp)));
        // All mid positions come before all right positions
        assert!(
            mid_positions
                .iter()
                .all(|&mp| right_positions.iter().all(|&rp| mp < rp))
        );
    }

    #[test]
    fn test_dag_reading_order_header_footer_separation() {
        // PageHeader, two body regions, PageFooter.
        // Headers must come first, footers last, body in the middle.
        let hints = [
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 200.0, 550.0, 600.0), // body 1
            make_hint(LayoutHintClass::PageFooter, 0.9, 10.0, 10.0, 550.0, 80.0), // footer
            make_hint(LayoutHintClass::PageHeader, 0.9, 10.0, 720.0, 550.0, 790.0), // header
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 100.0, 550.0, 190.0), // body 2
        ];
        let mut regions: Vec<LayoutRegion> = hints
            .iter()
            .map(|h| LayoutRegion {
                hint: h,
                segment_indices: Vec::new(),
                merged_bbox: None,
            })
            .collect();
        reading_order::order_regions_reading_order(&mut regions, 800.0);

        assert_eq!(regions[0].hint.class, LayoutHintClass::PageHeader);
        assert_eq!(regions[3].hint.class, LayoutHintClass::PageFooter);
        // The two middle positions should be the body Text regions
        assert_eq!(regions[1].hint.class, LayoutHintClass::Text);
        assert_eq!(regions[2].hint.class, LayoutHintClass::Text);
    }

    #[test]
    fn test_dag_reading_order_asymmetric_columns() {
        // Narrow sidebar (x=10-80) with 2 regions and wide body (x=120-500) with 3 regions.
        // We need >= 4 body regions total for DAG path to be exercised; 5 total qualifies.
        let hints = [
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 500.0, 80.0, 700.0), // sidebar top
            make_hint(LayoutHintClass::Text, 0.9, 120.0, 500.0, 500.0, 700.0), // body top
            make_hint(LayoutHintClass::Text, 0.9, 120.0, 300.0, 500.0, 490.0), // body mid
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 300.0, 80.0, 490.0), // sidebar bot
            make_hint(LayoutHintClass::Text, 0.9, 120.0, 100.0, 500.0, 290.0), // body bot
        ];
        let mut regions: Vec<LayoutRegion> = hints
            .iter()
            .map(|h| LayoutRegion {
                hint: h,
                segment_indices: Vec::new(),
                merged_bbox: None,
            })
            .collect();
        reading_order::order_regions_reading_order(&mut regions, 800.0);

        // Sidebar regions (left < 100) should all precede wide body regions (left >= 100)
        let sidebar_pos: Vec<usize> = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.hint.left < 100.0)
            .map(|(i, _)| i)
            .collect();
        let body_pos: Vec<usize> = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.hint.left >= 100.0)
            .map(|(i, _)| i)
            .collect();

        assert_eq!(sidebar_pos.len(), 2);
        assert_eq!(body_pos.len(), 3);
        assert!(sidebar_pos.iter().all(|&sp| body_pos.iter().all(|&bp| sp < bp)));
    }

    #[test]
    fn test_dag_reading_order_single_column() {
        // 4 regions in a single column — must come out top-to-bottom.
        let hints = [
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 550.0, 550.0, 700.0),
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 400.0, 550.0, 540.0),
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 250.0, 550.0, 390.0),
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 100.0, 550.0, 240.0),
        ];
        let mut regions: Vec<LayoutRegion> = hints
            .iter()
            .map(|h| LayoutRegion {
                hint: h,
                segment_indices: Vec::new(),
                merged_bbox: None,
            })
            .collect();
        reading_order::order_regions_reading_order(&mut regions, 800.0);

        // Each region's top (upper Y) should be strictly decreasing (reading top-to-bottom)
        for i in 0..regions.len() - 1 {
            assert!(
                regions[i].hint.top > regions[i + 1].hint.top,
                "region {} top={} should be above region {} top={}",
                i,
                regions[i].hint.top,
                i + 1,
                regions[i + 1].hint.top
            );
        }
    }

    #[test]
    fn test_reading_order_few_regions_fallback() {
        // 3 body regions — below MIN_REGIONS_FOR_DAG threshold, uses simple Y-sort.
        let hints = [
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 100.0, 550.0, 300.0), // bottom
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 500.0, 550.0, 700.0), // top
            make_hint(LayoutHintClass::Text, 0.9, 10.0, 310.0, 550.0, 490.0), // middle
        ];
        let mut regions: Vec<LayoutRegion> = hints
            .iter()
            .map(|h| LayoutRegion {
                hint: h,
                segment_indices: Vec::new(),
                merged_bbox: None,
            })
            .collect();
        reading_order::order_regions_reading_order(&mut regions, 800.0);

        // Should be sorted top-to-bottom: top (500-700) → middle (310-490) → bottom (100-300)
        assert!(regions[0].hint.bottom > regions[1].hint.bottom);
        assert!(regions[1].hint.bottom > regions[2].hint.bottom);
    }

    // ── Bbox Refinement Tests ────────────────────────────────────────────────

    #[test]
    fn test_bbox_refinement_shrinks_oversized_region() {
        // Large hint bbox (0,0 to 600,800) contains one small segment (50,700 to 90,712).
        // A second hint (200,695 to 400,715) covers text at x=250.
        // Without refinement both segments might go to the large region.
        // After refinement the large region bbox shrinks so the x=250 segment
        // should be captured by the tighter region.
        let segments = vec![
            make_segment("inside_large", 50.0, 700.0, 40.0, 12.0),
            make_segment("near_small", 250.0, 700.0, 60.0, 12.0),
        ];
        let hints = vec![
            make_hint(LayoutHintClass::Text, 0.9, 0.0, 0.0, 600.0, 800.0), // huge
            make_hint(LayoutHintClass::Code, 0.9, 200.0, 695.0, 400.0, 715.0), // tight
        ];
        // With refinement the tight region should claim the "near_small" segment
        let (regions, _) = assignment::assign_segments_to_regions_refined(&segments, &hints, 0.5, &[]);
        // The Code region (index 1) should have at least the segment near_small
        let code_region = regions.iter().find(|r| r.hint.class == LayoutHintClass::Code);
        assert!(code_region.is_some(), "Code region should exist");
        assert!(
            !code_region.unwrap().segment_indices.is_empty(),
            "Code region should contain at least one segment"
        );
    }

    #[test]
    fn test_bbox_refinement_preserves_original_class() {
        // After refinement the returned regions reference original hints, so
        // class and confidence should be unchanged.
        let segments = vec![make_segment("text", 50.0, 700.0, 40.0, 12.0)];
        let hints = vec![make_hint(
            LayoutHintClass::SectionHeader,
            0.85,
            0.0,
            690.0,
            200.0,
            720.0,
        )];
        let (regions, _) = assignment::assign_segments_to_regions_refined(&segments, &hints, 0.5, &[]);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].hint.class, LayoutHintClass::SectionHeader);
        assert!((regions[0].hint.confidence - 0.85).abs() < 1e-4);
    }

    // ── Caption Association Tests ────────────────────────────────────────────

    fn make_paragraph(layout_class: Option<LayoutHintClass>) -> super::super::types::PdfParagraph {
        super::super::types::PdfParagraph {
            lines: vec![],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class,
            caption_for: None,
            block_bbox: None,
        }
    }

    #[test]
    fn test_caption_association_below_table() {
        // [Text, Table, Caption, Text] — Caption should point to the Table (index 1).
        let mut paragraphs = vec![
            make_paragraph(Some(LayoutHintClass::Text)),
            make_paragraph(Some(LayoutHintClass::Table)),
            make_paragraph(Some(LayoutHintClass::Caption)),
            make_paragraph(Some(LayoutHintClass::Text)),
        ];
        associate_captions(&mut paragraphs);
        assert_eq!(paragraphs[2].caption_for, Some(1));
        // Other paragraphs unaffected
        assert_eq!(paragraphs[0].caption_for, None);
        assert_eq!(paragraphs[1].caption_for, None);
        assert_eq!(paragraphs[3].caption_for, None);
    }

    #[test]
    fn test_caption_association_above_figure() {
        // [Caption, Picture, Text] — Caption is above Picture, forward search finds Picture (index 1).
        let mut paragraphs = vec![
            make_paragraph(Some(LayoutHintClass::Caption)),
            make_paragraph(Some(LayoutHintClass::Picture)),
            make_paragraph(Some(LayoutHintClass::Text)),
        ];
        associate_captions(&mut paragraphs);
        assert_eq!(paragraphs[0].caption_for, Some(1));
    }

    #[test]
    fn test_caption_no_parent() {
        // [Text, Caption, Text] — no adjacent Table/Picture, so caption_for stays None.
        let mut paragraphs = vec![
            make_paragraph(Some(LayoutHintClass::Text)),
            make_paragraph(Some(LayoutHintClass::Caption)),
            make_paragraph(Some(LayoutHintClass::Text)),
        ];
        associate_captions(&mut paragraphs);
        assert_eq!(paragraphs[1].caption_for, None);
    }

    #[test]
    fn test_caption_ambiguous_prefers_closer() {
        // [Table, Text, Text, Caption, Picture]
        // Table is at index 0, Picture at index 4.
        // Distance from caption (index 3): Table distance = 3, Picture distance = 1.
        // Picture is closer, so caption should associate with Picture.
        let mut paragraphs = vec![
            make_paragraph(Some(LayoutHintClass::Table)),
            make_paragraph(Some(LayoutHintClass::Text)),
            make_paragraph(Some(LayoutHintClass::Text)),
            make_paragraph(Some(LayoutHintClass::Caption)),
            make_paragraph(Some(LayoutHintClass::Picture)),
        ];
        associate_captions(&mut paragraphs);
        assert_eq!(paragraphs[3].caption_for, Some(4));
    }

    // ── Footnote Association Tests ───────────────────────────────────────────

    #[test]
    fn test_footnote_after_table() {
        // [Text, Table, Footnote, Footnote, Text]
        // Both footnotes should be associated with the Table (index 1).
        let mut paragraphs = vec![
            make_paragraph(Some(LayoutHintClass::Text)),
            make_paragraph(Some(LayoutHintClass::Table)),
            make_paragraph(Some(LayoutHintClass::Footnote)),
            make_paragraph(Some(LayoutHintClass::Footnote)),
            make_paragraph(Some(LayoutHintClass::Text)),
        ];
        associate_footnotes(&mut paragraphs);
        assert_eq!(paragraphs[2].caption_for, Some(1));
        assert_eq!(paragraphs[3].caption_for, Some(1));
        // Non-footnote paragraphs unaffected
        assert_eq!(paragraphs[0].caption_for, None);
        assert_eq!(paragraphs[4].caption_for, None);
    }

    #[test]
    fn test_footnote_stops_at_non_footnote() {
        // [Table, Footnote, Text, Footnote]
        // Only the first Footnote (index 1) should be associated with Table.
        // The second Footnote (index 3) is separated by a Text paragraph and
        // should NOT be associated (no adjacent Table/Picture before it).
        let mut paragraphs = vec![
            make_paragraph(Some(LayoutHintClass::Table)),
            make_paragraph(Some(LayoutHintClass::Footnote)),
            make_paragraph(Some(LayoutHintClass::Text)),
            make_paragraph(Some(LayoutHintClass::Footnote)),
        ];
        associate_footnotes(&mut paragraphs);
        assert_eq!(paragraphs[1].caption_for, Some(0));
        assert_eq!(paragraphs[3].caption_for, None);
    }
}
