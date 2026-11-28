//! What makes a group of pieces a formula: footnote marks, closing brackets, operator-only
//! runs, numeric rows and math glyphs.

use super::*;

/// A raised script-size number right after sentence punctuation: a footnote mark that
/// follows the formula, not its exponent.
pub(super) fn is_trailing_note_mark(members: &[&Piece<'_>]) -> bool {
    let [.., before, last] = members else {
        return false;
    };
    let size = members.iter().map(|piece| piece.size).fold(0.0_f32, f32::max);
    let digits = last.text.trim();
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && last.size < size * SCRIPT_SIZE_RATIO
        && last.base > before.base + size * STRUCTURE_OFFSET_EM
        && before.text.trim_end().ends_with(['.', ',', ';', ':'])
}

/// The prose span right after an inline formula that opens with the closing brackets the
/// formula is missing, and how many of them it lends: (span, count).
pub(super) fn missing_closers(
    members: &[&Piece<'_>],
    prose: &[&Piece<'_>],
    x1: f32,
    base: f32,
    size: f32,
) -> Option<(usize, usize)> {
    let balance = |open: char, close: char| {
        members
            .iter()
            .flat_map(|piece| piece.text.chars())
            .fold(0_isize, |depth, c| {
                depth + isize::from(c == open) - isize::from(c == close)
            })
    };
    let (mut parens, mut brackets) = (balance('(', ')'), balance('[', ']'));
    if parens <= 0 && brackets <= 0 {
        return None;
    }
    let reach = size * STRUCTURE_OFFSET_EM;
    let next = prose_near(prose, base - reach, base + reach)
        .iter()
        .filter(|piece| piece.x0 >= x1 - size * AXIS_OVERLAP_EM && piece.x0 - x1 <= size * CLUSTER_GAP_EM)
        .min_by(|a, b| a.x0.total_cmp(&b.x0))?;
    let mut count = 0;
    for c in next.text.chars() {
        match c {
            ')' if parens > 0 => parens -= 1,
            ']' if brackets > 0 => brackets -= 1,
            _ => break,
        }
        count += 1;
    }
    (count > 0).then_some((next.span, count))
}

/// Whether every math-font glyph is a binary operator, as a LaTeX table's `$-$3.4` minus is.
pub(super) fn only_operator_glyphs(members: &[&Piece<'_>]) -> bool {
    members
        .iter()
        .filter(|piece| piece.is_math())
        .flat_map(|piece| piece.text.chars())
        .filter(|&c| !c.is_whitespace() && !is_unmapped(c))
        .all(is_operator)
}

/// Several text runs that are all signed numbers: table cells, not a formula.
pub(super) fn reads_as_numeric_row(members: &[&Piece<'_>]) -> bool {
    let numeric = |text: &str| {
        let text = text.trim();
        text.chars().any(|c| c.is_ascii_digit())
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || c.is_whitespace() || matches!(c, '.' | ',' | '%' | '−' | '-' | '+'))
    };
    let neutral: Vec<&&Piece<'_>> = members.iter().filter(|piece| piece.class == Class::Neutral).collect();
    neutral.len() > MAX_NUMERIC_RUNS && neutral.iter().all(|piece| numeric(piece.text))
}

/// A math-font glyph that is more than a footnote or list mark.
pub(super) fn has_math_evidence(members: &[&Piece<'_>]) -> bool {
    members.iter().filter(|piece| piece.is_math()).any(|piece| {
        piece.text.chars().any(|c| {
            !c.is_whitespace()
                && !is_unmapped(c)
                && !is_mark(c)
                && (c.is_alphanumeric()
                    || glyphs::greek_command(c).is_some()
                    || glyphs::symbol_command(c).is_some()
                    || is_relation(c)
                    || is_operator(c))
        })
    })
}

/// Scripts, stacked parts or accents: what prose assembly breaks apart.
pub(super) fn has_structure(members: &[&Piece<'_>], size: f32) -> bool {
    let first = members[0].base;
    members.iter().any(|piece| {
        (piece.base - first).abs() > size * STRUCTURE_OFFSET_EM
            || piece.size < size * SCRIPT_SIZE_RATIO
            || piece.accent().is_some()
    })
}
