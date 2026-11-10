//! Lone accent letters kept apart from the formula line under them.

use super::PdfParagraph;
use crate::pdf::hierarchy::SegmentData;

/// The first visual line of a paragraph, its segments re-joined with single spaces.
pub(super) fn first_line_joined(para: &PdfParagraph) -> String {
    let Some(first_line) = para.lines.first() else {
        return String::new();
    };
    first_line
        .segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether `current` ends in a lone accent letter that sits over the start of the formula
/// line `next`. See [`accent_letter_over_formula_start`].
pub(super) fn accent_letter_over_formula_line(current: &PdfParagraph, next: &PdfParagraph) -> bool {
    let (Some(prev_seg), Some(next_seg)) = (
        current.lines.last().and_then(|line| line.segments.last()),
        next.lines.first().and_then(|line| line.segments.first()),
    ) else {
        return false;
    };
    accent_letter_over_formula_start(prev_seg, next_seg, &first_line_joined(next))
}

/// Whether `prev_seg` ends in a lone accent letter that sits over `next_seg`, the start of the
/// formula line `next_line` ("b" over "X = V DU", an estimator hat).
///
/// A math font draws the hat as its own glyph, a lowercase letter a quarter em above the
/// base letter and inside its width. The join sees no gap between the two, so it fuses them
/// into one token ("bX"). The line opens with a capital letter or a number and a math operator
/// (`X = Y`, `K = K − b`). A variable on a line of its own sits a line above the formula, needs
/// a space there, and still joins. ~keep
pub(in crate::pdf::structure) fn accent_letter_over_formula_start(
    prev_seg: &SegmentData,
    next_seg: &SegmentData,
    next_line: &str,
) -> bool {
    let prev_word = prev_seg.text.split_whitespace().next_back().unwrap_or("");
    let next_word = next_seg.text.split_whitespace().next().unwrap_or("");
    let mut letters = prev_word.chars();
    let is_lone_letter = matches!((letters.next(), letters.next()), (Some(c), None) if c.is_lowercase());
    is_lone_letter
        && !crate::pdf::structure::lines::segments_need_space(prev_seg, prev_word, next_seg, next_word)
        && crate::pdf::structure::classify::is_formula_line(next_line)
}
