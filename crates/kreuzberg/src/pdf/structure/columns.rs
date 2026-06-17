//! Column detection for multi-column PDF layouts.
//!
//! Detects column boundaries by analyzing the x-position distribution of
//! PDF page objects and splits them into separate column groups for
//! independent paragraph extraction.

use pdfium_render::prelude::{PdfPageObject, PdfPageObjectCommon};

/// Minimum number of text objects per column to be considered valid.
const MIN_OBJECTS_PER_COLUMN: usize = 10;

/// Minimum gap between columns as fraction of page width.
const MIN_COLUMN_GAP_FRACTION: f32 = 0.04;

/// Minimum fraction of page height that both columns must span vertically.
const MIN_VERTICAL_SPAN_FRACTION: f32 = 0.3;

/// A bounding box extracted from a page object for column analysis.
struct ObjectBounds {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

/// Detect column boundaries from page objects and return index groups.
///
/// Returns a list of index vectors, each representing objects belonging to
/// the same column, ordered left-to-right. If no columns are detected,
/// returns a single group containing all indices.
pub(super) fn split_objects_into_columns(objects: &[PdfPageObject]) -> Vec<Vec<usize>> {
    let bounds: Vec<ObjectBounds> = objects
        .iter()
        .filter_map(|obj| {
            // Only consider text objects for column detection
            obj.as_text_object()?;
            obj.bounds().ok().map(|b| ObjectBounds {
                left: b.left().value,
                right: b.right().value,
                top: b.top().value,
                bottom: b.bottom().value,
            })
        })
        .collect();

    if bounds.len() < MIN_OBJECTS_PER_COLUMN * 2 {
        return vec![(0..objects.len()).collect()];
    }

    let (page_width, page_y_min, page_y_max) = estimate_page_bounds(&bounds);
    if page_width < 1.0 {
        return vec![(0..objects.len()).collect()];
    }

    let min_gap = page_width * MIN_COLUMN_GAP_FRACTION;

    if let Some(split_x) = find_column_split(&bounds, min_gap, page_y_min, page_y_max) {
        let mut left_indices: Vec<usize> = Vec::new();
        let mut right_indices: Vec<usize> = Vec::new();

        // Partition ALL objects (not just text) by midpoint relative to split
        for (i, obj) in objects.iter().enumerate() {
            let mid_x = obj
                .bounds()
                .ok()
                .map(|b| (b.left().value + b.right().value) / 2.0)
                .unwrap_or(0.0);

            if mid_x < split_x {
                left_indices.push(i);
            } else {
                right_indices.push(i);
            }
        }

        // Validate column sizes (text objects only)
        let left_text_count = left_indices
            .iter()
            .filter(|&&i| objects[i].as_text_object().is_some())
            .count();
        let right_text_count = right_indices
            .iter()
            .filter(|&&i| objects[i].as_text_object().is_some())
            .count();

        if left_text_count < MIN_OBJECTS_PER_COLUMN || right_text_count < MIN_OBJECTS_PER_COLUMN {
            return vec![(0..objects.len()).collect()];
        }

        vec![left_indices, right_indices]
    } else {
        vec![(0..objects.len()).collect()]
    }
}

/// Estimate page bounds from object bounding boxes.
fn estimate_page_bounds(bounds: &[ObjectBounds]) -> (f32, f32, f32) {
    let mut x_min = f32::MAX;
    let mut x_max = f32::MIN;
    let mut y_min = f32::MAX;
    let mut y_max = f32::MIN;

    for b in bounds {
        x_min = x_min.min(b.left);
        x_max = x_max.max(b.right);
        y_min = y_min.min(b.bottom);
        y_max = y_max.max(b.top);
    }

    (x_max - x_min, y_min, y_max)
}

/// Find the best x-position to split columns using gap analysis.
///
/// Sorts object left edges, finds the widest gap exceeding `min_gap`,
/// and validates that objects on both sides span enough of the page height.
fn find_column_split(bounds: &[ObjectBounds], min_gap: f32, page_y_min: f32, page_y_max: f32) -> Option<f32> {
    let page_y_range = page_y_max - page_y_min;
    if page_y_range < 1.0 {
        return None;
    }

    // Collect (left, right) edges sorted by left edge
    let mut edges: Vec<(f32, f32)> = bounds.iter().map(|b| (b.left, b.right)).collect();
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));

    // Track the running maximum right edge to find true gaps
    let mut max_right = f32::MIN;
    let mut best_gap = 0.0_f32;
    let mut best_split = None;

    for &(left, right) in &edges {
        if max_right > f32::MIN {
            let gap = left - max_right;
            if gap > min_gap && gap > best_gap {
                best_gap = gap;
                best_split = Some((max_right + left) / 2.0);
            }
        }
        max_right = max_right.max(right);
    }

    // Validate: both sides must span a significant portion of page height
    if let Some(split_x) = best_split {
        let left_y_range = vertical_span(bounds.iter().filter(|b| b.left < split_x));
        let right_y_range = vertical_span(bounds.iter().filter(|b| b.left >= split_x));

        if left_y_range > page_y_range * MIN_VERTICAL_SPAN_FRACTION
            && right_y_range > page_y_range * MIN_VERTICAL_SPAN_FRACTION
        {
            return Some(split_x);
        }
    }

    None
}

/// Compute the vertical span (top - bottom) of an iterator of bounds.
fn vertical_span<'a>(bounds: impl Iterator<Item = &'a ObjectBounds>) -> f32 {
    let mut y_min = f32::MAX;
    let mut y_max = f32::MIN;

    for b in bounds {
        y_min = y_min.min(b.bottom);
        y_max = y_max.max(b.top);
    }

    if y_max > y_min { y_max - y_min } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns_single_group() {
        let objects: Vec<PdfPageObject> = vec![];
        let groups = split_objects_into_columns(&objects);
        assert_eq!(groups.len(), 1);
        assert!(groups[0].is_empty());
    }
}
