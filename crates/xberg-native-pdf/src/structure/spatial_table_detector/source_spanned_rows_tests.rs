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
