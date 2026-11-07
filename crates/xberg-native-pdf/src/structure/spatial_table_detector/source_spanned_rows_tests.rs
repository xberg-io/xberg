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
    grid.table.rows[0].cells[0].spans.clear();
    assert!(!certified(&grid));
}
