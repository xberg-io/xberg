//! A hat drawn as a lone letter over a formula line, through the line grouper and the continuation merge.
//! The hat glyph "b" sits a quarter em above the formula's first letter and inside its width.

use super::{TextRepairWitnesses, segments_to_paragraphs};
use crate::pdf::hierarchy::SegmentData;
use crate::pdf::structure::types::PdfParagraph;

const FONT_SIZE: f32 = 11.96;
/// The hat's baseline above the formula's, a quarter em.
const HAT_RISE: f32 = 3.02;

fn seg(text: &str, x: f32, width: f32, baseline_y: f32) -> SegmentData {
    SegmentData {
        text: text.to_string(),
        x,
        y: baseline_y,
        width,
        height: FONT_SIZE,
        font_size: FONT_SIZE,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

/// The formula line `<first> = V DU` at `baseline`.
fn formula(first: &str, baseline: f32) -> Vec<SegmentData> {
    vec![
        seg(first, 274.92, 9.71, baseline),
        seg("=", 288.90, 9.11, baseline),
        seg("V DU", 301.32, 27.42, baseline),
    ]
}

/// The hat "b" over the first letter of the formula line at `baseline`, then that line.
fn hatted_formula(first: &str, baseline: f32) -> Vec<SegmentData> {
    let mut segments = vec![seg("b", 277.90, 6.64, baseline + HAT_RISE)];
    segments.extend(formula(first, baseline));
    segments
}

fn texts(paragraphs: &[PdfParagraph]) -> Vec<String> {
    paragraphs
        .iter()
        .map(|para| {
            para.lines
                .iter()
                .flat_map(|line| line.segments.iter())
                .map(|segment| segment.text.trim())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn group(segments: Vec<SegmentData>) -> Vec<PdfParagraph> {
    segments_to_paragraphs(segments, &[(FONT_SIZE, None)], &[], &TextRepairWitnesses::default())
}

/// The hat stays apart from the displayed formula under it, so the two never print as "bX".
#[test]
fn should_keep_hat_letter_apart_from_formula_line_under_it() {
    let paragraphs = group(hatted_formula("X", 143.62));

    assert_eq!(
        texts(&paragraphs),
        vec!["b".to_string(), "X = V DU".to_string()],
        "the hat must not join the formula line"
    );
}

/// The hat stays apart from a formula line that opens with a capital that is no roman numeral.
#[test]
fn should_keep_hat_letter_apart_from_formula_line_with_non_roman_capital() {
    for (first, operator) in [("Y", "="), ("A", "="), ("Z", "+")] {
        let mut segments = hatted_formula(first, 143.62);
        segments[2].text = operator.to_string();
        let paragraphs = group(segments);

        assert_eq!(
            texts(&paragraphs),
            vec!["b".to_string(), format!("{first} {operator} V DU")],
            "the hat must not join the formula line that opens with {first} {operator}"
        );
    }
}

/// The same hat over a formula that continues a line of prose: the prose keeps its hat and the
/// formula starts its own paragraph.
#[test]
fn should_keep_hat_letter_apart_from_formula_that_continues_prose() {
    let mut segments = vec![seg("because we do not have access to", 350.60, 189.39, 507.08)];
    segments.extend([
        seg("b", 104.25, 6.64, 491.43 + HAT_RISE),
        seg("X", 101.26, 9.71, 491.43),
        seg("=", 115.24, 9.11, 491.43),
        seg("X \u{2212} \u{00B5}.", 127.67, 35.56, 491.43),
    ]);
    let paragraphs = group(segments);

    assert_eq!(
        texts(&paragraphs),
        vec![
            "because we do not have access to b".to_string(),
            "X = X \u{2212} \u{00B5}.".to_string()
        ],
        "the hat must not join the formula line"
    );
}

/// Control: a hat over a formula that opens with a lowercase variable joins, as on the base.
#[test]
fn should_still_join_hat_letter_over_lowercase_formula_line() {
    let paragraphs = group(hatted_formula("y", 143.62));

    assert_eq!(paragraphs.len(), 1, "a formula line that opens lowercase still joins");
}

/// Control: a one-letter variable a full line above the formula needs a space and joins.
#[test]
fn should_still_join_variable_line_above_formula_line() {
    let mut segments = vec![seg("n", 274.92, 6.64, 143.62 + FONT_SIZE * 1.2)];
    segments.extend(formula("X", 143.62));
    let paragraphs = group(segments);

    assert_eq!(paragraphs.len(), 1, "a variable a line above the formula still joins");
}

/// Control: an uppercase lone letter over the formula line is a variable or a label, not a hat,
/// and joins. The hat rule covers lowercase letters only.
#[test]
fn should_still_join_uppercase_letter_over_formula_line() {
    let mut segments = vec![seg("B", 277.90, 6.64, 143.62 + HAT_RISE)];
    segments.extend(formula("X", 143.62));
    let paragraphs = group(segments);

    assert_eq!(
        texts(&paragraphs),
        vec!["B X = V DU".to_string()],
        "an uppercase letter is not a hat and joins the formula line"
    );
}

/// A lone letter inside a formula that sits on one visual row is part of that row: only a letter
/// on the row above the formula is a hat. The row "X = a V = W" is a formula line and the letter
/// "a" touches the "V" after it, but all sit on one baseline and stay one paragraph. The merge
/// pass would reunite a split here, so the second half is itself a formula line: only the
/// grouper's new-line guard keeps the row whole.
#[test]
fn should_not_split_lone_letter_from_formula_on_the_same_row() {
    let segments = vec![
        seg("X", 100.0, 9.71, 143.62),
        seg("=", 114.0, 9.11, 143.62),
        seg("a", 127.0, 6.64, 143.62),
        seg("V", 128.0, 9.71, 143.62),
        seg("=", 140.0, 9.11, 143.62),
        seg("W", 152.0, 9.71, 143.62),
    ];
    let paragraphs = group(segments);

    assert_eq!(
        texts(&paragraphs),
        vec!["X = a V = W".to_string()],
        "a lone letter on the formula's own row must not start a new paragraph"
    );
}
