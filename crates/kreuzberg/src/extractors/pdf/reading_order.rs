//! Layout-guided PDF reading-order reconstruction.
//!
//! When enabled, this module projects text spans onto layout-detected regions,
//! performs column detection, and reorders spans in natural reading order
//! (top-to-bottom within a column, left-to-right across columns).
//!
//! This is critical for multi-column academic PDFs where native PDF text
//! extraction reads in column order rather than visual reading order.

#[cfg(feature = "layout-detection")]
use crate::pdf::structure::types::LayoutHint;

/// Region x-centers closer than this (in PDF points) are merged into one column.
const COLUMN_MERGE_THRESHOLD_PTS: f32 = 20.0;

/// A text span with bounding box information.
#[derive(Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Detect columns by clustering region x-centers.
///
/// Analyzes the horizontal positions of regions (using their x-centers) to
/// identify distinct columns. Uses k-means-like clustering with a distance
/// threshold to group regions that belong to the same column.
///
/// Returns a Vec of column assignments, one per region, mapping region index
/// to column ID (0 = leftmost column).
fn detect_columns(regions: &[RegionProjection]) -> Vec<usize> {
    if regions.is_empty() {
        return Vec::new();
    }

    // Collect x-centers of all regions
    let mut x_centers: Vec<f32> = regions.iter().map(|r| (r.left + r.right) / 2.0).collect();

    // Sort to identify cluster boundaries
    x_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Deduplicate x_centers (merge nearly-identical values)
    let mut unique_centers: Vec<f32> = Vec::new();
    let merge_threshold: f32 = COLUMN_MERGE_THRESHOLD_PTS;

    for &center in &x_centers {
        if let Some(&last) = unique_centers.last() {
            if (center - last).abs() > merge_threshold {
                unique_centers.push(center);
            }
        } else {
            unique_centers.push(center);
        }
    }

    // Assign each region to the nearest cluster center
    let mut assignments = vec![0usize; regions.len()];
    for (i, region) in regions.iter().enumerate() {
        let center = (region.left + region.right) / 2.0;
        let mut best_col = 0;
        let mut best_dist = f32::INFINITY;

        for (col_id, &cluster_center) in unique_centers.iter().enumerate() {
            let dist = (center - cluster_center).abs();
            if dist < best_dist {
                best_dist = dist;
                best_col = col_id;
            }
        }

        assignments[i] = best_col;
    }

    assignments
}

/// A region projection: layout region with indices of spans it contains.
#[derive(Debug, Clone)]
struct RegionProjection {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
    span_indices: Vec<usize>,
}

/// Project spans onto regions using bounding box intersection/containment.
///
/// For each span, determines which region(s) it overlaps with using a simple
/// containment heuristic: if the span's center is within the region, the span
/// belongs to that region.
fn project_spans_to_regions(spans: &[TextSpan], hints: &[LayoutHint]) -> Vec<RegionProjection> {
    let mut regions: Vec<RegionProjection> = hints
        .iter()
        .map(|hint| RegionProjection {
            left: hint.left,
            bottom: hint.bottom,
            right: hint.right,
            top: hint.top,
            span_indices: Vec::new(),
        })
        .collect();

    for (span_idx, span) in spans.iter().enumerate() {
        let span_center_x = span.x + span.width / 2.0;
        let span_center_y = span.y + span.height / 2.0;

        // Find the best-matching region (by area overlap or containment).
        // For simplicity, use center-point containment with IoU fallback.
        let mut best_region = None;
        let mut best_overlap = 0.0;

        for (region_idx, region) in regions.iter().enumerate() {
            // Check if span center is within region bounds
            if span_center_x >= region.left
                && span_center_x <= region.right
                && span_center_y >= region.bottom
                && span_center_y <= region.top
            {
                // Prefer containment; if multiple regions contain the span,
                // pick the one with the smallest area (most specific).
                let area = (region.right - region.left) * (region.top - region.bottom);
                if best_region.is_none() || area < best_overlap {
                    best_region = Some(region_idx);
                    best_overlap = area;
                }
            }
        }

        if let Some(region_idx) = best_region {
            regions[region_idx].span_indices.push(span_idx);
        }
    }

    // Filter out empty regions
    regions.retain(|r| !r.span_indices.is_empty());
    regions
}

/// Reorder segments (higher-level blocks) based on layout regions and column detection.
///
/// Similar to reorder_spans_by_layout but operates on PDF hierarchy SegmentData.
/// Each segment has a bounding box (x, y, width, height) that maps onto layout regions.
///
/// Returns the reordered segments in reading order (column-major, top-to-bottom).
#[cfg(feature = "layout-detection")]
pub(crate) fn reorder_segments_by_layout(
    segments: Vec<crate::pdf::hierarchy::SegmentData>,
    hints: &[LayoutHint],
) -> Vec<crate::pdf::hierarchy::SegmentData> {
    if segments.is_empty() || hints.is_empty() {
        return segments;
    }

    // Convert segments to simple (index, bbox) for projection onto regions
    let seg_indices: Vec<(usize, f32, f32, f32, f32)> = segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            let right = seg.x + seg.width;
            let bottom = seg.y; // PDF coords: y=0 at bottom, so "bottom" is the y value
            let top = seg.y + seg.height;
            (i, seg.x, bottom, right, top)
        })
        .collect();

    // Project indices onto regions using center-point containment
    let mut regions: Vec<(RegionProjection, Vec<usize>)> = hints
        .iter()
        .map(|hint| {
            (
                RegionProjection {
                    left: hint.left,
                    bottom: hint.bottom,
                    right: hint.right,
                    top: hint.top,
                    span_indices: Vec::new(),
                },
                Vec::new(),
            )
        })
        .collect();

    for (seg_idx, seg_left, seg_bottom, seg_right, seg_top) in &seg_indices {
        let seg_center_x = seg_left + (seg_right - seg_left) / 2.0;
        let seg_center_y = seg_bottom + (seg_top - seg_bottom) / 2.0;

        // Find the best-matching region (smallest enclosing region)
        let mut best_region = None;
        let mut best_area = f32::INFINITY;

        for (region_idx, (region, _)) in regions.iter().enumerate() {
            if seg_center_x >= region.left
                && seg_center_x <= region.right
                && seg_center_y >= region.bottom
                && seg_center_y <= region.top
            {
                let area = (region.right - region.left) * (region.top - region.bottom);
                if area < best_area {
                    best_region = Some(region_idx);
                    best_area = area;
                }
            }
        }

        if let Some(region_idx) = best_region {
            regions[region_idx].1.push(*seg_idx);
        }
    }

    // Keep only regions that captured at least one segment, preserving the
    // captured segment indices (the earlier per-region projection result).
    let active: Vec<(RegionProjection, Vec<usize>)> =
        regions.into_iter().filter(|(_, indices)| !indices.is_empty()).collect();

    if active.is_empty() {
        return segments;
    }

    // Detect columns from region x-centers: merge centers within the threshold.
    let mut x_centers: Vec<f32> = active.iter().map(|(r, _)| (r.left + r.right) / 2.0).collect();
    x_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut unique_centers: Vec<f32> = Vec::new();
    for &center in &x_centers {
        match unique_centers.last() {
            Some(&last) if (center - last).abs() <= COLUMN_MERGE_THRESHOLD_PTS => {}
            _ => unique_centers.push(center),
        }
    }

    // Tag each region with its nearest column, carrying its captured indices.
    let mut ordered: Vec<(usize, f32, Vec<usize>)> = Vec::with_capacity(active.len());
    for (region, indices) in active {
        let center = (region.left + region.right) / 2.0;
        let mut best_col = 0usize;
        let mut best_dist = f32::INFINITY;
        for (cand, &cluster_center) in unique_centers.iter().enumerate() {
            let dist = (center - cluster_center).abs();
            if dist < best_dist {
                best_dist = dist;
                best_col = cand;
            }
        }
        ordered.push((best_col, region.top, indices));
    }

    // Column-major (left-to-right), then top-to-bottom (descending y in PDF coords).
    ordered.sort_by(|a, b| match a.0.cmp(&b.0) {
        std::cmp::Ordering::Equal => b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal),
        other => other,
    });

    // Emit segments in sorted-region order (each at most once), then append any
    // segments that projected to no region, in their original order.
    let mut included = vec![false; segments.len()];
    let mut reorder_map: Vec<usize> = Vec::with_capacity(segments.len());
    for (_, _, indices) in &ordered {
        for &idx in indices {
            if idx < segments.len() && !included[idx] {
                included[idx] = true;
                reorder_map.push(idx);
            }
        }
    }
    for (idx, done) in included.iter().enumerate() {
        if !*done {
            reorder_map.push(idx);
        }
    }

    reorder_map.into_iter().map(|idx| segments[idx].clone()).collect()
}

/// Reorder spans based on layout regions and column detection.
///
/// Given a set of spans with bounding boxes and layout-detected regions:
/// 1. Project spans onto regions
/// 2. Detect columns from region x-centers
/// 3. Sort regions by (column_id, top-to-bottom within column)
/// 4. Emit spans in the order of their sorted regions
///
/// Returns a Vec of span indices in reading order.
pub(crate) fn reorder_spans_by_layout(spans: &[TextSpan], hints: &[LayoutHint]) -> Vec<usize> {
    if spans.is_empty() || hints.is_empty() {
        // No reordering needed
        return (0..spans.len()).collect();
    }

    // Project spans onto regions
    let regions = project_spans_to_regions(spans, hints);
    if regions.is_empty() {
        // No spans projected to regions; return original order
        return (0..spans.len()).collect();
    }

    // Detect columns
    let column_assignments = detect_columns(&regions);

    // Build a sortable structure: (column_id, top_y, region_index)
    // For PDF coordinates, "top" is the higher y value; we want to read top-to-bottom
    // so we sort by descending y (top first).
    let mut sorted_regions: Vec<(usize, f32, usize)> = regions
        .iter()
        .enumerate()
        .map(|(region_idx, region)| {
            let col_id = column_assignments[region_idx];
            let top_y = region.top;
            (col_id, top_y, region_idx)
        })
        .collect();

    // Sort by (column_id ascending, top_y descending)
    sorted_regions.sort_by(|a, b| {
        match a.0.cmp(&b.0) {
            std::cmp::Ordering::Equal => {
                // Same column: sort by y descending (top-to-bottom in PDF coords)
                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
            }
            other => other,
        }
    });

    // Emit spans in sorted region order
    let mut result = Vec::new();
    let mut projected_spans = std::collections::HashSet::new();

    for (_, _, region_idx) in sorted_regions {
        for &span_idx in &regions[region_idx].span_indices {
            result.push(span_idx);
            projected_spans.insert(span_idx);
        }
    }

    // Append unprojected spans in their original order
    for span_idx in 0..spans.len() {
        if !projected_spans.contains(&span_idx) {
            result.push(span_idx);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_columns_two_column_layout() {
        // Two columns: left at x=100 (center 150), right at x=400 (center 450)
        let regions = vec![
            RegionProjection {
                left: 100.0,
                bottom: 100.0,
                right: 200.0,
                top: 500.0,
                span_indices: vec![],
            },
            RegionProjection {
                left: 400.0,
                bottom: 100.0,
                right: 500.0,
                top: 500.0,
                span_indices: vec![],
            },
        ];

        let assignments = detect_columns(&regions);
        assert_eq!(assignments.len(), 2);
        // First region should be column 0, second should be column 1
        assert_ne!(assignments[0], assignments[1]);
        assert_eq!(assignments[0], 0);
        assert_eq!(assignments[1], 1);
    }

    #[test]
    fn test_project_spans_to_regions() {
        let spans = vec![
            TextSpan {
                text: "Left column".to_string(),
                x: 110.0,
                y: 450.0,
                width: 70.0,
                height: 12.0,
            },
            TextSpan {
                text: "Right column".to_string(),
                x: 410.0,
                y: 450.0,
                width: 75.0,
                height: 12.0,
            },
        ];

        let hints = vec![
            LayoutHint {
                class_name: crate::pdf::structure::types::LayoutHintClass::Text,
                confidence: 0.95,
                left: 100.0,
                bottom: 100.0,
                right: 200.0,
                top: 500.0,
            },
            LayoutHint {
                class_name: crate::pdf::structure::types::LayoutHintClass::Text,
                confidence: 0.95,
                left: 400.0,
                bottom: 100.0,
                right: 500.0,
                top: 500.0,
            },
        ];

        let regions = project_spans_to_regions(&spans, &hints);
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].span_indices.len(), 1);
        assert_eq!(regions[0].span_indices[0], 0);
        assert_eq!(regions[1].span_indices.len(), 1);
        assert_eq!(regions[1].span_indices[0], 1);
    }

    #[test]
    fn test_reorder_spans_two_column_layout() {
        // Create a 2-column layout with 4 spans:
        // Left column: "A" (top), "B" (bottom)
        // Right column: "C" (top), "D" (bottom)
        //
        // Expected reading order: A, B, C, D (left column first, top-to-bottom, then right column)
        let spans = vec![
            TextSpan {
                text: "A".to_string(),
                x: 110.0,
                y: 450.0,
                width: 10.0,
                height: 12.0,
            },
            TextSpan {
                text: "B".to_string(),
                x: 110.0,
                y: 200.0,
                width: 10.0,
                height: 12.0,
            },
            TextSpan {
                text: "C".to_string(),
                x: 410.0,
                y: 450.0,
                width: 10.0,
                height: 12.0,
            },
            TextSpan {
                text: "D".to_string(),
                x: 410.0,
                y: 200.0,
                width: 10.0,
                height: 12.0,
            },
        ];

        let hints = vec![
            LayoutHint {
                class_name: crate::pdf::structure::types::LayoutHintClass::Text,
                confidence: 0.95,
                left: 100.0,
                bottom: 100.0,
                right: 200.0,
                top: 500.0,
            },
            LayoutHint {
                class_name: crate::pdf::structure::types::LayoutHintClass::Text,
                confidence: 0.95,
                left: 400.0,
                bottom: 100.0,
                right: 500.0,
                top: 500.0,
            },
        ];

        let order = reorder_spans_by_layout(&spans, &hints);
        assert_eq!(order.len(), 4);
        // Order should be: 0 (A), 1 (B), 2 (C), 3 (D)
        assert_eq!(order[0], 0); // A from left column, top
        assert_eq!(order[1], 1); // B from left column, bottom
        assert_eq!(order[2], 2); // C from right column, top
        assert_eq!(order[3], 3); // D from right column, bottom
    }

    /// Segment-level reorder must produce true column-major reading order from
    /// interleaved input, independent of the layout-hint ordering. The hints
    /// here are supplied right-column-first; a correct reorder still yields
    /// A, B, C, D (left column top-to-bottom, then right column). A previous
    /// implementation emitted segments in raw hint order and would yield
    /// C, D, A, B here — this is the regression guard.
    #[test]
    fn test_reorder_segments_two_column_independent_of_hint_order() {
        fn seg(text: &str, x: f32, y: f32) -> crate::pdf::hierarchy::SegmentData {
            crate::pdf::hierarchy::SegmentData {
                text: text.to_string(),
                x,
                y,
                width: 10.0,
                height: 12.0,
                font_size: 10.0,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: y,
                assigned_role: None,
            }
        }

        // Interleaved input order across columns (as native extraction may emit).
        let segments = vec![
            seg("A", 110.0, 450.0), // left column, top
            seg("C", 410.0, 450.0), // right column, top
            seg("B", 110.0, 200.0), // left column, bottom
            seg("D", 410.0, 200.0), // right column, bottom
        ];

        // Hints supplied right-column-first to prove the result is column-major,
        // not hint-order-dependent.
        let hints = vec![
            LayoutHint {
                class_name: crate::pdf::structure::types::LayoutHintClass::Text,
                confidence: 0.95,
                left: 400.0,
                bottom: 100.0,
                right: 500.0,
                top: 500.0,
            },
            LayoutHint {
                class_name: crate::pdf::structure::types::LayoutHintClass::Text,
                confidence: 0.95,
                left: 100.0,
                bottom: 100.0,
                right: 200.0,
                top: 500.0,
            },
        ];

        let reordered = reorder_segments_by_layout(segments, &hints);
        let order: Vec<&str> = reordered.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(
            order,
            vec!["A", "B", "C", "D"],
            "segments must be reordered column-major top-to-bottom regardless of hint order"
        );
    }

    #[test]
    fn test_reorder_spans_mixed_columns() {
        // Create a more realistic scenario:
        // Left column: "A" (very top), "B" (middle)
        // Right column: "C" (top), "D" (middle), "E" (bottom)
        // And one unprojected span "X"
        let spans = vec![
            TextSpan {
                text: "A".to_string(),
                x: 110.0,
                y: 480.0,
                width: 10.0,
                height: 12.0,
            },
            TextSpan {
                text: "B".to_string(),
                x: 110.0,
                y: 300.0,
                width: 10.0,
                height: 12.0,
            },
            TextSpan {
                text: "C".to_string(),
                x: 410.0,
                y: 470.0,
                width: 10.0,
                height: 12.0,
            },
            TextSpan {
                text: "D".to_string(),
                x: 410.0,
                y: 300.0,
                width: 10.0,
                height: 12.0,
            },
            TextSpan {
                text: "E".to_string(),
                x: 410.0,
                y: 150.0,
                width: 10.0,
                height: 12.0,
            },
            TextSpan {
                text: "X".to_string(), // Will not project to any region
                x: 550.0,
                y: 300.0,
                width: 10.0,
                height: 12.0,
            },
        ];

        let hints = vec![
            LayoutHint {
                class_name: crate::pdf::structure::types::LayoutHintClass::Text,
                confidence: 0.95,
                left: 100.0,
                bottom: 100.0,
                right: 200.0,
                top: 500.0,
            },
            LayoutHint {
                class_name: crate::pdf::structure::types::LayoutHintClass::Text,
                confidence: 0.95,
                left: 400.0,
                bottom: 100.0,
                right: 500.0,
                top: 500.0,
            },
        ];

        let order = reorder_spans_by_layout(&spans, &hints);
        assert_eq!(order.len(), 6);
        // Spans 0, 1 are in left column (top-to-bottom)
        // Spans 2, 3, 4 are in right column (top-to-bottom)
        // Span 5 (X) is unprojected so it stays in original position at the end
        assert_eq!(order[0], 0); // A
        assert_eq!(order[1], 1); // B
        assert_eq!(order[2], 2); // C
        assert_eq!(order[3], 3); // D
        assert_eq!(order[4], 4); // E
        assert_eq!(order[5], 5); // X (unprojected)
    }

    #[test]
    fn test_reorder_spans_empty_input() {
        let spans = vec![];
        let hints = vec![];
        let order = reorder_spans_by_layout(&spans, &hints);
        assert!(order.is_empty());
    }

    #[test]
    fn test_reorder_spans_no_hints() {
        let spans = vec![
            TextSpan {
                text: "A".to_string(),
                x: 100.0,
                y: 100.0,
                width: 10.0,
                height: 12.0,
            },
            TextSpan {
                text: "B".to_string(),
                x: 120.0,
                y: 100.0,
                width: 10.0,
                height: 12.0,
            },
        ];
        let hints = vec![];
        let order = reorder_spans_by_layout(&spans, &hints);
        // No hints means no reordering; should return original order
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn test_config_default_reading_order_is_false() {
        // Verify that the default PdfConfig has reading_order disabled
        let pdf_config = crate::core::config::PdfConfig::default();
        assert!(
            !pdf_config.reading_order,
            "Default reading_order must be false for backward compatibility"
        );
    }
}
