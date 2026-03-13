//! Segment-to-region spatial assignment logic.

use crate::pdf::hierarchy::SegmentData;

use super::LayoutRegion;
use crate::pdf::markdown::types::{LayoutHint, LayoutHintClass};

/// Minimum intersection-over-self (IoS) for a segment to be assigned to a region.
/// Matches docling's threshold of 0.2 (20% of the segment's area must overlap).
const MIN_IOS_THRESHOLD: f32 = 0.2;

/// Padding (in points) added to each side of the tight bbox during refinement.
const BBOX_REFINEMENT_PADDING: f32 = 2.0;

/// Maximum number of assign→shrink→re-assign iterations.
const MAX_REFINEMENT_ITERATIONS: usize = 3;

/// Assign each segment to its best-matching layout region.
///
/// Uses intersection-over-self (IoS): the fraction of the segment's bounding box
/// that overlaps with a region. If IoS >= 0.2, the segment can be assigned.
/// Among qualifying regions, the one with highest IoS wins; ties broken by
/// smallest area (most specific region).
///
/// Table and Picture regions are excluded from text assignment (handled by
/// separate pipelines). Segments within those regions are dropped entirely
/// to avoid duplicating content that appears in the extracted tables.
pub(in crate::pdf::markdown) fn assign_segments_to_regions<'a>(
    segments: &[SegmentData],
    hints: &'a [LayoutHint],
    min_confidence: f32,
    extracted_table_bboxes: &[crate::types::BoundingBox],
) -> (Vec<LayoutRegion<'a>>, Vec<usize>) {
    let confident_hints: Vec<&LayoutHint> = hints
        .iter()
        .filter(|h| h.confidence >= min_confidence)
        // Exclude Table and Picture — handled by separate pipelines
        .filter(|h| !matches!(h.class, LayoutHintClass::Table | LayoutHintClass::Picture))
        .collect();

    // Only suppress segments in Table bboxes that actually produced valid tables.
    // If a Table hint failed extraction, its segments should fall through to
    // the unassigned pool so text isn't silently lost.
    let suppress_bboxes = extracted_table_bboxes;

    // Collect Picture region bounding boxes for suppression.
    // Segments within Picture regions are OCR artifacts (text rendered inside
    // images) and should not appear in the extracted text output.
    let picture_hints: Vec<&LayoutHint> = hints
        .iter()
        .filter(|h| h.confidence >= min_confidence && h.class == LayoutHintClass::Picture)
        .collect();

    if confident_hints.is_empty() && suppress_bboxes.is_empty() && picture_hints.is_empty() {
        let all_indices: Vec<usize> = (0..segments.len()).collect();
        return (Vec::new(), all_indices);
    }

    // Pre-compute hint areas for tie-breaking
    let hint_areas: Vec<f32> = confident_hints
        .iter()
        .map(|h| (h.right - h.left) * (h.top - h.bottom))
        .collect();

    // Build region containers
    let mut regions: Vec<LayoutRegion> = confident_hints
        .iter()
        .map(|&hint| LayoutRegion {
            hint,
            segment_indices: Vec::new(),
            merged_bbox: None,
        })
        .collect();

    let mut unassigned: Vec<usize> = Vec::new();

    let mut suppressed_count = 0_usize;

    for (seg_idx, seg) in segments.iter().enumerate() {
        if seg.text.trim().is_empty() {
            continue; // Skip whitespace-only segments
        }

        let seg_left = seg.x;
        let seg_right = seg.x + seg.width;
        let seg_bottom = seg.y;
        let seg_top = seg.y + seg.height;
        let seg_area = seg.width * seg.height;

        // Check if this segment overlaps a successfully extracted table.
        // Use IoS (intersection-over-self) instead of center-point to catch
        // segments that straddle the table boundary.
        let in_extracted_table = suppress_bboxes.iter().any(|bb| {
            intersection_over_self(
                seg_left,
                seg_bottom,
                seg_right,
                seg_top,
                seg_area,
                bb.x0 as f32,
                bb.y0 as f32,
                bb.x1 as f32,
                bb.y1 as f32,
            ) >= 0.5
        });
        if in_extracted_table {
            suppressed_count += 1;
            continue;
        }

        // Suppress segments inside Picture regions — but only if the text
        // looks like OCR artifacts (garbled hex, figure labels, etc.).
        // Substantive text (lyrics, captions, embedded prose) is preserved
        // as unassigned so it still appears in the output.
        let in_picture = picture_hints.iter().any(|ph| {
            intersection_over_self(
                seg_left, seg_bottom, seg_right, seg_top, seg_area, ph.left, ph.bottom, ph.right, ph.top,
            ) >= 0.5
        });
        if in_picture {
            let trimmed = seg.text.trim();
            if is_substantive_text(trimmed) {
                // Keep substantive text — push to unassigned
                unassigned.push(seg_idx);
            } else {
                suppressed_count += 1;
            }
            continue;
        }

        let mut best_hint_idx: Option<usize> = None;
        let mut best_ios = 0.0_f32;
        let mut best_area = f32::MAX;

        for (hi, hint) in confident_hints.iter().enumerate() {
            let ios = intersection_over_self(
                seg_left,
                seg_bottom,
                seg_right,
                seg_top,
                seg_area,
                hint.left,
                hint.bottom,
                hint.right,
                hint.top,
            );

            if ios >= MIN_IOS_THRESHOLD {
                // Prefer higher IoS; break ties by smallest area
                if ios > best_ios || (ios == best_ios && hint_areas[hi] < best_area) {
                    best_ios = ios;
                    best_area = hint_areas[hi];
                    best_hint_idx = Some(hi);
                }
            }
        }

        match best_hint_idx {
            Some(hi) => regions[hi].segment_indices.push(seg_idx),
            None => unassigned.push(seg_idx),
        }
    }

    tracing::trace!(
        confident_hints = confident_hints.len(),
        segments = segments.len(),
        suppressed = suppressed_count,
        assigned_to_regions = regions.iter().map(|r| r.segment_indices.len()).sum::<usize>(),
        unassigned = unassigned.len(),
        "segment-to-region assignment complete"
    );

    (regions, unassigned)
}

/// Assign segments to regions with iterative bounding box refinement.
///
/// After the initial assignment, each region's bbox is shrunk to tightly fit its
/// assigned segments (plus padding). Then segments are re-assigned using the refined
/// bboxes. This prevents over-large model bboxes from "stealing" text from adjacent
/// regions. Repeats up to 3 iterations or until assignments stabilize.
pub(in crate::pdf::markdown) fn assign_segments_to_regions_refined<'a>(
    segments: &[SegmentData],
    hints: &'a [LayoutHint],
    min_confidence: f32,
    extracted_table_bboxes: &[crate::types::BoundingBox],
) -> (Vec<LayoutRegion<'a>>, Vec<usize>) {
    // First pass: assign with original bboxes
    let (regions, unassigned) = assign_segments_to_regions(segments, hints, min_confidence, extracted_table_bboxes);

    if regions.is_empty() {
        return (regions, unassigned);
    }

    // Compute refined bboxes from assigned segments
    let refined_hints: Vec<LayoutHint> = compute_refined_hints(&regions, segments, hints);

    if refined_hints.is_empty() {
        return (regions, unassigned);
    }

    // Collect Picture hints from the original hints to carry through iterations.
    // The refined hints only contain text-bearing regions (non-Table/non-Picture),
    // but Picture bboxes must remain available for segment suppression.
    let picture_hints: Vec<LayoutHint> = hints
        .iter()
        .filter(|h| h.confidence >= min_confidence && h.class == LayoutHintClass::Picture)
        .cloned()
        .collect();

    // Iterate: re-assign with refined bboxes, re-shrink, repeat
    let mut current_hints = refined_hints;
    // Append Picture hints so they're available for suppression in each iteration
    current_hints.extend(picture_hints.iter().cloned());
    let mut prev_assignments: Vec<Vec<usize>> = regions.iter().map(|r| r.segment_indices.clone()).collect();

    for _ in 1..MAX_REFINEMENT_ITERATIONS {
        let (new_regions, _) =
            assign_segments_to_regions(segments, &current_hints, min_confidence, extracted_table_bboxes);

        // Check if assignments changed
        let new_assignments: Vec<Vec<usize>> = new_regions.iter().map(|r| r.segment_indices.clone()).collect();
        if new_assignments == prev_assignments {
            break; // Stable — stop iterating
        }

        prev_assignments = new_assignments;
        let mut new_refined = compute_refined_hints(&new_regions, segments, &current_hints);
        new_refined.extend(picture_hints.iter().cloned());
        current_hints = new_refined;
    }

    // Final assignment with the last refined hints, but we need to return regions
    // that reference the original hints (for class/confidence), with the refined assignments
    let (final_regions_refined, final_unassigned) =
        assign_segments_to_regions(segments, &current_hints, min_confidence, extracted_table_bboxes);

    // Map refined regions back to original hints by matching class + position
    let mut result_regions: Vec<LayoutRegion<'a>> = Vec::new();
    let confident_original: Vec<(usize, &'a LayoutHint)> = hints
        .iter()
        .enumerate()
        .filter(|(_, h)| h.confidence >= min_confidence)
        .filter(|(_, h)| !matches!(h.class, LayoutHintClass::Table | LayoutHintClass::Picture))
        .collect();

    for (ri, refined_region) in final_regions_refined.iter().enumerate() {
        if ri < confident_original.len() {
            result_regions.push(LayoutRegion {
                hint: confident_original[ri].1,
                segment_indices: refined_region.segment_indices.clone(),
                merged_bbox: None,
            });
        }
    }

    (result_regions, final_unassigned)
}

/// Compute refined hints by shrinking each region's bbox to tightly fit its segments.
///
/// Each region's `hint` field already points to the correct original hint for that
/// region (set when the region was created in `assign_segments_to_regions`), so we
/// use it directly as the base for clamping. The `source_hints` parameter is unused
/// but retained for API symmetry with the iterative caller.
fn compute_refined_hints(
    regions: &[LayoutRegion],
    segments: &[SegmentData],
    _source_hints: &[LayoutHint],
) -> Vec<LayoutHint> {
    let mut refined = Vec::with_capacity(regions.len());

    for region in regions.iter() {
        let base_hint = region.hint;

        if region.segment_indices.is_empty() {
            // No segments — keep original bbox
            refined.push(base_hint.clone());
            continue;
        }

        // Compute tight bbox from assigned segments
        let mut tight_left = f32::MAX;
        let mut tight_bottom = f32::MAX;
        let mut tight_right = f32::MIN;
        let mut tight_top = f32::MIN;

        for &idx in &region.segment_indices {
            let seg = &segments[idx];
            tight_left = tight_left.min(seg.x);
            tight_bottom = tight_bottom.min(seg.y);
            tight_right = tight_right.max(seg.x + seg.width);
            tight_top = tight_top.max(seg.y + seg.height);
        }

        // Sparsity guard: if segments fill < 15% of the tight bbox area,
        // the layout is sparse (e.g., TOC with dot leaders). Refinement
        // would fragment the line across regions — keep original bbox.
        let tight_area = (tight_right - tight_left) * (tight_top - tight_bottom);
        if tight_area > 0.0 {
            let seg_area_sum: f32 = region
                .segment_indices
                .iter()
                .map(|&idx| segments[idx].width * segments[idx].height)
                .sum();
            let fill_ratio = seg_area_sum / tight_area;
            if fill_ratio < 0.15 {
                tracing::trace!(
                    class = ?base_hint.class,
                    fill_ratio,
                    "refinement skipped: sparse layout"
                );
                refined.push(base_hint.clone());
                continue;
            }
        }

        // Apply padding and clamp to original bbox (never expand beyond model prediction)
        refined.push(LayoutHint {
            class: base_hint.class,
            confidence: base_hint.confidence,
            left: (tight_left - BBOX_REFINEMENT_PADDING).max(base_hint.left),
            bottom: (tight_bottom - BBOX_REFINEMENT_PADDING).max(base_hint.bottom),
            right: (tight_right + BBOX_REFINEMENT_PADDING).min(base_hint.right),
            top: (tight_top + BBOX_REFINEMENT_PADDING).min(base_hint.top),
        });
    }

    refined
}

/// Compute intersection-over-self: the fraction of the segment's area that
/// overlaps with the region's bounding box.
#[allow(clippy::too_many_arguments)]
fn intersection_over_self(
    seg_left: f32,
    seg_bottom: f32,
    seg_right: f32,
    seg_top: f32,
    seg_area: f32,
    hint_left: f32,
    hint_bottom: f32,
    hint_right: f32,
    hint_top: f32,
) -> f32 {
    if seg_area <= 0.0 {
        return 0.0;
    }

    let inter_left = seg_left.max(hint_left);
    let inter_right = seg_right.min(hint_right);
    let inter_bottom = seg_bottom.max(hint_bottom);
    let inter_top = seg_top.min(hint_top);

    if inter_left >= inter_right || inter_bottom >= inter_top {
        return 0.0;
    }

    let inter_area = (inter_right - inter_left) * (inter_top - inter_bottom);
    inter_area / seg_area
}

/// Minimum alphanumeric character count for text inside a Picture region
/// to be considered substantive (and thus preserved rather than suppressed).
const PICTURE_SUBSTANTIVE_MIN_ALNUM: usize = 10;

/// Minimum alphanumeric ratio for text inside a Picture region to be
/// considered substantive.
const PICTURE_SUBSTANTIVE_MIN_ALNUM_RATIO: f64 = 0.4;

/// Check whether a text segment is substantive content (lyrics, captions,
/// prose) rather than OCR artifacts (garbled hex, stray labels).
///
/// We use a simple two-part heuristic:
/// 1. At least `PICTURE_SUBSTANTIVE_MIN_ALNUM` alphanumeric characters
/// 2. Alphanumeric ratio >= `PICTURE_SUBSTANTIVE_MIN_ALNUM_RATIO`
fn is_substantive_text(text: &str) -> bool {
    let total = text.chars().count();
    if total == 0 {
        return false;
    }
    let alnum = text.chars().filter(|c| c.is_alphanumeric()).count();
    alnum >= PICTURE_SUBSTANTIVE_MIN_ALNUM && (alnum as f64 / total as f64) >= PICTURE_SUBSTANTIVE_MIN_ALNUM_RATIO
}
