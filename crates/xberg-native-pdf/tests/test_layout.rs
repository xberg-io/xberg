#![allow(dead_code)]
//! Integration tests for layout analysis algorithms.
//!
//! These tests verify the complete layout analysis pipeline with mock data
//! simulating realistic PDF document structures.

use xberg_native_pdf::geometry::{Point, Rect};
use xberg_native_pdf::layout::{
    FontWeight, TextBlock, TextChar,
    clustering::{cluster_chars_into_words, cluster_words_into_lines},
    reading_order::graph_based_reading_order,
};

/// Create a mock character with minimal required data.
fn mock_char(c: char, x: f32, y: f32, size: f32) -> TextChar {
    let width = size * 0.6;
    TextChar {
        char: c,
        bbox: Rect::new(x, y, width, size),
        font_size: size,
        origin_x: x,
        origin_y: y,
        advance_width: width,
        ..Default::default()
    }
}

/// Create a mock character with bold weight.
fn mock_bold_char(c: char, x: f32, y: f32, size: f32) -> TextChar {
    let width = size * 0.6;
    TextChar {
        char: c,
        bbox: Rect::new(x, y, width, size),
        font_name: "Times-Bold".into(),
        font_size: size,
        font_weight: FontWeight::Bold,
        origin_x: x,
        origin_y: y,
        advance_width: width,
        ..Default::default()
    }
}

/// Create a text block from a string at a specific position.
fn mock_block(text: &str, x: f32, y: f32, size: f32, bold: bool) -> TextBlock {
    let chars: Vec<TextChar> = text
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if bold {
                mock_bold_char(c, x + i as f32 * size * 0.6, y, size)
            } else {
                mock_char(c, x + i as f32 * size * 0.6, y, size)
            }
        })
        .collect();

    TextBlock::from_chars(chars)
}

/// Create a two-column layout with multiple lines per column.
fn create_two_column_layout() -> Vec<TextBlock> {
    vec![
        mock_block("First", 0.0, 0.0, 12.0, false),
        mock_block("line", 50.0, 0.0, 12.0, false),
        mock_block("Second", 0.0, 20.0, 12.0, false),
        mock_block("line", 50.0, 20.0, 12.0, false),
        mock_block("Third", 300.0, 0.0, 12.0, false),
        mock_block("line", 350.0, 0.0, 12.0, false),
        mock_block("Fourth", 300.0, 20.0, 12.0, false),
        mock_block("line", 350.0, 20.0, 12.0, false),
    ]
}

#[test]
fn test_geometry_point() {
    let p = Point::new(10.0, 20.0);
    assert_eq!(p.x, 10.0);
    assert_eq!(p.y, 20.0);
}

#[test]
fn test_geometry_rect_operations() {
    let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
    let r2 = Rect::new(50.0, 50.0, 100.0, 100.0);

    assert!(r1.intersects(&r2));

    let union = r1.union(&r2);
    assert_eq!(union.left(), 0.0);
    assert_eq!(union.right(), 150.0);

    let p1 = Point::new(50.0, 50.0);
    assert!(r1.contains_point(&p1));
}

#[test]
fn test_cluster_chars_into_words_simple() {
    let chars = vec![
        mock_char('H', 0.0, 0.0, 12.0),
        mock_char('i', 8.0, 0.0, 12.0),
        mock_char('B', 50.0, 0.0, 12.0),
        mock_char('y', 58.0, 0.0, 12.0),
        mock_char('e', 66.0, 0.0, 12.0),
    ];

    let clusters = cluster_chars_into_words(&chars, 15.0);

    assert_eq!(clusters.len(), 2);

    let hi_cluster = clusters.iter().find(|c| c.contains(&0)).unwrap();
    assert!(hi_cluster.contains(&0));
    assert!(hi_cluster.contains(&1));

    let bye_cluster = clusters.iter().find(|c| c.contains(&2)).unwrap();
    assert!(bye_cluster.contains(&2));
    assert!(bye_cluster.contains(&3));
    assert!(bye_cluster.contains(&4));
}

#[test]
fn test_cluster_words_into_lines_simple() {
    let word1 = mock_block("Hello", 0.0, 0.0, 12.0, false);
    let word2 = mock_block("World", 50.0, 1.0, 12.0, false);
    let word3 = mock_block("Next", 0.0, 30.0, 12.0, false);
    let word4 = mock_block("Line", 50.0, 31.0, 12.0, false);

    let words = vec![word1, word2, word3, word4];
    let lines = cluster_words_into_lines(&words, 5.0);

    assert_eq!(lines.len(), 2);

    let line1 = lines.iter().find(|l| l.contains(&0)).unwrap();
    assert!(line1.contains(&0));
    assert!(line1.contains(&1));

    let line2 = lines.iter().find(|l| l.contains(&2)).unwrap();
    assert!(line2.contains(&2));
    assert!(line2.contains(&3));
}

// ============================================================================
// NOTE: XY-Cut Column Detection Tests Removed
// These tests were removed in Phase 2.2 of CLEANUP_ROADMAP.md
// as the column_detector module and XY-Cut algorithm were deleted
// for PDF spec compliance (they are non-spec-compliant heuristics)
// ============================================================================ ~keep

// NOTE: test_reading_order_tree_based removed - relied on LayoutTree and determine_reading_order (both deleted)
// ~keep

#[test]
fn test_reading_order_graph_based_simple() {
    let blocks = vec![
        mock_block("TopLeft", 0.0, 100.0, 12.0, false),
        mock_block("TopRight", 100.0, 100.0, 12.0, false),
        mock_block("BottomLeft", 0.0, 50.0, 12.0, false),
        mock_block("BottomRight", 100.0, 50.0, 12.0, false),
    ];

    let order = graph_based_reading_order(&blocks);

    assert_eq!(order[0], 0);
    assert_eq!(order[1], 1);
    assert_eq!(order[2], 2);
    assert_eq!(order[3], 3);
}

#[test]
fn test_reading_order_two_columns() {
    let blocks = vec![
        mock_block("Col1Line1", 0.0, 100.0, 12.0, false),
        mock_block("Col1Line2", 0.0, 50.0, 12.0, false),
        mock_block("Col2Line1", 300.0, 100.0, 12.0, false),
        mock_block("Col2Line2", 300.0, 50.0, 12.0, false),
    ];

    let order = graph_based_reading_order(&blocks);

    assert!(order[0] == 0 || order[0] == 2);

    assert_eq!(order.len(), 4);
    assert!(order.contains(&0));
    assert!(order.contains(&1));
    assert!(order.contains(&2));
    assert!(order.contains(&3));
}

// ============================================================================
// NOTE: Heading and Table Detection Tests Removed
// These tests were removed in Phase 1 of CLEANUP_ROADMAP.md
// as heading_detector and table_detector modules were deleted
// for PDF spec compliance (they are non-spec-compliant heuristics) ~keep

// NOTE: test_full_pipeline_two_column_document removed - relied on xy_cut, determine_reading_order, and detect_headings (all deleted)
// ~keep

#[test]
fn test_empty_inputs() {
    let empty_blocks: Vec<TextBlock> = vec![];
    let empty_chars: Vec<TextChar> = vec![];

    assert_eq!(cluster_chars_into_words(&empty_chars, 10.0).len(), 0);
    assert_eq!(cluster_words_into_lines(&empty_blocks, 5.0).len(), 0);

    assert_eq!(graph_based_reading_order(&empty_blocks).len(), 0);
}

#[test]
fn test_single_element_inputs() {
    let single_char = vec![mock_char('A', 0.0, 0.0, 12.0)];
    let single_block = vec![mock_block("Single", 0.0, 0.0, 12.0, false)];

    assert_eq!(cluster_chars_into_words(&single_char, 10.0).len(), 1);
    assert_eq!(cluster_words_into_lines(&single_block, 5.0).len(), 1);

    assert_eq!(graph_based_reading_order(&single_block), vec![0]);
}
