use super::*;
use crate::layout::Color;

fn rule() -> PathContent {
    PathContent::line(0.0, 100.0, 200.0, 100.0)
}

fn certifies(path: PathContent) -> bool {
    painted_rule(&[path], false, 100.0, 10.0, 190.0)
}

#[test]
fn plain_dark_horizontal_rule_is_painted() {
    assert!(certifies(rule()));
}

#[test]
fn dashed_rule_is_not_painted() {
    let mut dashed = rule();
    dashed.dash_pattern = Some((vec![1.0, 12.0], 0.0));
    assert!(!certifies(dashed));
}

#[test]
fn white_or_light_rules_are_not_painted() {
    assert!(!certifies(rule().with_stroke(Color::new(1.0, 1.0, 1.0))));
    assert!(!certifies(rule().with_stroke(Color::new(0.9, 0.9, 0.9))));
    assert!(!certifies(rule().with_stroke(Color::new(1.0, 1.0, 0.0))));
    assert!(!certifies(rule().with_stroke(Color::new(0.0, 1.0, 0.0))));
    assert!(certifies(rule().with_stroke(Color::new(0.2, 0.2, 0.2))));
}

#[test]
fn unstroked_transformed_layered_or_thick_rules_are_not_painted() {
    let mut unstroked = rule();
    unstroked.stroke_color = None;
    assert!(!certifies(unstroked));
    let mut transformed = rule();
    transformed.matrix = Some([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    assert!(!certifies(transformed));
    assert!(!certifies(rule().with_layer("hidden")));
    assert!(!certifies(rule().with_stroke_width(3.0)));
}

#[test]
fn rule_must_span_the_requested_extent() {
    assert!(!painted_rule(&[rule()], false, 100.0, 10.0, 400.0));
    assert!(!painted_rule(&[rule()], false, 120.0, 10.0, 190.0));
}

use super::fixture::{self, Grid};

fn certified(grid: &Grid) -> bool {
    certify(&grid.table, &grid.spans, &grid.paths).is_some()
}

#[test]
fn fully_ruled_merged_middle_row_is_certified() {
    assert!(certified(&fixture::grid()));
}

#[test]
fn a_closed_cell_needs_every_horizontal_rule_painted() {
    for rule in 0..=3 {
        let mut grid = fixture::grid();
        grid.paths.remove(rule);
        assert!(
            !certified(&grid),
            "table certified without the horizontal rule at row boundary {rule}"
        );
    }
}

#[test]
fn a_cell_span_absent_from_the_source_words_is_not_certified() {
    let mut grid = fixture::grid();
    let mut invented = grid.spans[0].clone();
    invented.sequence = grid.spans.len();
    grid.table.rows[0].cells[0].spans.push(invented);
    assert!(!certified(&grid));
}

#[test]
fn a_source_word_inside_the_table_that_no_cell_claims_is_not_certified() {
    let mut grid = fixture::grid();
    let mut stray = grid.spans[0].clone();
    stray.sequence = grid.spans.len();
    stray.text = "X".to_string();
    assert!(grid.table.bbox.unwrap().contains_rect(&stray.bbox));
    grid.spans.push(stray);
    assert!(!certified(&grid));
}

#[test]
fn a_closed_cell_needs_every_vertical_rule_painted() {
    let vertical = fixture::grid()
        .paths
        .iter()
        .enumerate()
        .filter(|(_, path)| path.is_vertical_line(1.0))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert!(vertical.len() > fixture::COLUMNS);
    for index in vertical {
        let mut grid = fixture::grid();
        grid.paths.remove(index);
        assert!(
            !certified(&grid),
            "table certified without the vertical rule at path {index}"
        );
    }
}

#[test]
fn a_rule_inside_a_merged_cell_is_not_certified() {
    let mut grid = fixture::grid();
    grid.paths.push(PathContent::line(60.0, 20.0, 60.0, 40.0));
    assert!(!certified(&grid));
}

#[test]
fn only_sixteen_to_eighteen_columns_are_certified() {
    for (columns, expected) in [(15, false), (16, true), (18, true), (19, false)] {
        assert_eq!(
            certified(&fixture::grid_with_columns(columns)),
            expected,
            "{columns} columns"
        );
    }
}

#[test]
fn a_middle_row_needs_two_merged_cells() {
    assert!(certified(&fixture::grid_with_merged(&[1, 8, 6, 1])));
    assert!(!certified(&fixture::grid_with_merged(&[1, 14, 1])));
}
