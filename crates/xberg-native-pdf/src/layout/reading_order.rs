//! Reading order determination for layout analysis.
//!
//! This module provides algorithms for determining the correct reading order
//! of text blocks in a document. Uses O(n log n) sort-based ordering by
//! spatial position (top-to-bottom, left-to-right).

use crate::layout::text_block::TextBlock;

/// Determine reading order using sort-based ordering.
///
/// Sorts blocks by spatial position: top-to-bottom (Y descending in PDF coords),
/// then left-to-right (X ascending) for blocks on the same line.
///
/// This is O(n log n) — replacing the previous O(n²) graph-based approach
/// which built a full precedence graph via nested loops.
///
/// # Arguments
///
/// * `blocks` - The text blocks to order
///
/// # Returns
///
/// A vector of block indices in reading order.
pub fn graph_based_reading_order(blocks: &[TextBlock]) -> Vec<usize> {
    if blocks.is_empty() {
        return vec![];
    }

    if blocks.len() == 1 {
        return vec![0];
    }

    let mut indices: Vec<usize> = (0..blocks.len()).collect();

    // Sort by reading order: top-to-bottom, left-to-right.
    // PDF coordinates: origin at bottom-left, Y increases upward.
    // "Top of page" = larger Y values, so sort Y descending.
    // Tolerance of 5 units for "same line" detection. ~keep
    let y_tolerance = 5.0;
    indices.sort_by(|&a, &b| {
        let ay = blocks[a].bbox.top();
        let by = blocks[b].bbox.top();

        let primary = if (ay - by).abs() < y_tolerance {
            crate::utils::safe_float_cmp(blocks[a].bbox.left(), blocks[b].bbox.left())
        } else {
            crate::utils::safe_float_cmp(by, ay)
        };

        // Tie-break by original index for deterministic ordering
        // when blocks have identical positions. ~keep
        if primary == std::cmp::Ordering::Equal {
            a.cmp(&b)
        } else {
            primary
        }
    });

    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight, TextChar};

    fn mock_block(text: &str, x: f32, y: f32) -> TextBlock {
        let chars: Vec<TextChar> = text
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let bbox = Rect::new(x + i as f32 * 10.0, y, 10.0, 12.0);
                TextChar {
                    char: c,
                    bbox,
                    font_name: "Times".into(),
                    font_size: 12.0,
                    font_weight: FontWeight::Normal,
                    is_italic: false,
                    is_monospace: false,
                    color: Color::black(),
                    mcid: None,
                    origin_x: bbox.x,
                    origin_y: bbox.y,
                    rotation_degrees: 0.0,
                    advance_width: bbox.width,
                    rendered_advance: bbox.width,
                    ascent: 0.95 * 12.0,
                    descent: -0.35 * 12.0,
                    matrix: None,
                }
            })
            .collect();

        TextBlock::from_chars(chars)
    }

    #[test]
    fn test_reading_order_same_line() {
        let blocks = vec![mock_block("Right", 100.0, 1.0), mock_block("Left", 0.0, 0.0)];

        let order = graph_based_reading_order(&blocks);
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn test_reading_order_different_lines() {
        // PDF coordinates: Y increases upward, so top has LARGER Y ~keep
        let blocks = vec![mock_block("Bottom", 0.0, 50.0), mock_block("Top", 0.0, 100.0)];

        let order = graph_based_reading_order(&blocks);
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn test_reading_order_simple_grid() {
        let blocks = vec![
            mock_block("A", 0.0, 100.0),
            mock_block("B", 100.0, 100.0),
            mock_block("C", 0.0, 50.0),
            mock_block("D", 100.0, 50.0),
        ];

        let order = graph_based_reading_order(&blocks);

        assert_eq!(order, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_reading_order_two_columns() {
        let blocks = vec![
            mock_block("Col1-Line1", 0.0, 100.0),
            mock_block("Col1-Line2", 0.0, 50.0),
            mock_block("Col2-Line1", 300.0, 100.0),
            mock_block("Col2-Line2", 300.0, 50.0),
        ];

        let order = graph_based_reading_order(&blocks);

        assert_eq!(order, vec![0, 2, 1, 3]);
    }

    #[test]
    fn test_reading_order_empty() {
        let blocks: Vec<TextBlock> = vec![];
        let order = graph_based_reading_order(&blocks);
        assert_eq!(order.len(), 0);
    }

    #[test]
    fn test_reading_order_single() {
        let blocks = vec![mock_block("Single", 0.0, 0.0)];
        let order = graph_based_reading_order(&blocks);
        assert_eq!(order, vec![0]);
    }
}
