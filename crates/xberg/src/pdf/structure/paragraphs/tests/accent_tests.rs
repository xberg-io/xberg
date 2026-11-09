//! Lone accent letters and formula lines in continuation merging.

use super::{PdfParagraph, make_body_paragraph_at, merge_continuation_paragraphs};

/// A one-segment paragraph at an exact position, for accent and formula-line geometry.
fn make_positioned_paragraph(text: &str, x: f32, width: f32, baseline_y: f32) -> PdfParagraph {
    let mut para = make_body_paragraph_at(text, 11.0, baseline_y);
    if let Some(segment) = para.lines[0].segments.first_mut() {
        segment.x = x;
        segment.width = width;
    }
    para
}

/// A lone accent letter on the formula line's baseline, right over its first letter, fuses
/// into "bX" if the two join. The accent line stays apart from the formula line.
#[test]
fn should_not_merge_accent_letter_into_formula_line() {
    let mut paragraphs = vec![
        make_positioned_paragraph("b", 100.0, 5.0, 700.0),
        make_positioned_paragraph("X = V DU", 105.0, 40.0, 700.0),
    ];
    merge_continuation_paragraphs(&mut paragraphs);

    assert_eq!(
        paragraphs.len(),
        2,
        "accent letter must stay apart from the formula line"
    );
}

/// The accent letter also stays apart from a formula line that opens with a capital that is
/// no roman numeral.
#[test]
fn should_not_merge_accent_letter_into_non_roman_formula_line() {
    for formula in ["Y = V DU", "A = V DU", "Z + V DU"] {
        let mut paragraphs = vec![
            make_positioned_paragraph("b", 100.0, 5.0, 700.0),
            make_positioned_paragraph(formula, 105.0, 40.0, 700.0),
        ];
        merge_continuation_paragraphs(&mut paragraphs);

        assert_eq!(
            paragraphs.len(),
            2,
            "accent letter must stay apart from the formula line {formula:?}"
        );
    }
}

/// The same accent ends a line of prose: the paragraph before it still holds apart.
#[test]
fn should_not_merge_accent_letter_that_ends_a_prose_line() {
    let mut paragraphs = vec![
        make_positioned_paragraph("we do not have access to b", 100.0, 105.0, 700.0),
        make_positioned_paragraph("X = X \u{2212} \u{00B5}.", 205.0, 40.0, 700.0),
    ];
    merge_continuation_paragraphs(&mut paragraphs);

    assert_eq!(
        paragraphs.len(),
        2,
        "accent letter must stay apart from the formula line"
    );
}

/// Control: a one-letter variable on its own line, a line above the formula, needs a space
/// there and still joins, as it does on the base.
#[test]
fn should_still_merge_variable_line_into_formula_line_below_it() {
    let mut paragraphs = vec![
        make_positioned_paragraph("n", 72.0, 5.0, 700.0),
        make_positioned_paragraph("X = V DU", 72.0, 40.0, 686.0),
    ];
    merge_continuation_paragraphs(&mut paragraphs);

    assert_eq!(paragraphs.len(), 1, "a variable a line above the formula still joins");
}

/// Control: the same adjacent lone letter before a formula line that opens with a lowercase
/// variable keeps today's join.
#[test]
fn should_still_merge_lone_letter_before_lowercase_formula_line() {
    let mut paragraphs = vec![
        make_positioned_paragraph("b", 100.0, 5.0, 700.0),
        make_positioned_paragraph("y = V DU", 105.0, 40.0, 700.0),
    ];
    merge_continuation_paragraphs(&mut paragraphs);

    assert_eq!(paragraphs.len(), 1, "a formula line that opens lowercase still joins");
}

/// Control: a word ending a line (not one letter) before a formula line still joins.
#[test]
fn should_still_merge_word_before_adjacent_formula_line() {
    let mut paragraphs = vec![
        make_positioned_paragraph("given by the projection of", 100.0, 105.0, 700.0),
        make_positioned_paragraph("X = V DU", 205.0, 40.0, 700.0),
    ];
    merge_continuation_paragraphs(&mut paragraphs);

    assert_eq!(paragraphs.len(), 1, "a word is not an accent letter");
}
