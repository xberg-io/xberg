//! The one rule that decides whether spans form a nested script stack on a base glyph.
//!
//! Reading-order snapping, paragraph-row assembly and script annotation all ask the same question
//! about `(text, x, baseline, width, font size)`, so they share this predicate and cannot drift.

use super::ordering_snap::{is_list_marker, is_phrase};
use super::span_ordering::SnapReach;

/// Largest rise or drop of a stack member from its base line, in ems of the base.
const MAX_SHIFT_EM: f32 = 0.5;
/// Largest gap between a member and the right edge of the base or the stack so far, in ems of the base.
const MAX_GAP_EM: f32 = 0.25;
/// A member must be smaller than its base by at least this factor.
const MIN_BASE_SIZE_FACTOR: f32 = 1.15;
/// A member must be raised or lowered by more than this many ems of the base: text set on the base
/// line, such as the body after a large bracket, is not a script.
const MIN_DISPLACEMENT_EM: f32 = 0.1;
/// Members differing in size by more than this fraction of the larger one sit at two levels. A
/// numerator and denominator, `x_i^2`, a footnote-marker pair or a run of same-size glyphs do not.
const MIN_SIZE_STEP: f32 = 0.1;
/// Most members a stack holds. Paragraph grouping and markup search one run more than this past a
/// base, so a longer stack is seen as too long there and refused, as it is here.
#[doc(hidden)]
pub const MAX_STACK_MEMBERS: usize = 8;
/// Two members form a fraction when their x ranges overlap by more than this share of the narrower.
const MIN_FRACTION_OVERLAP: f32 = 0.5;
/// Two stacked members form a fraction when their baselines differ by more than this many ems of
/// the smaller member.
const MIN_FRACTION_SEPARATION_EM: f32 = 0.5;
/// Largest rise or drop of a numerator or denominator from the base line, in ems of the base.
const MAX_FRACTION_PART_SHIFT_EM: f32 = 1.0;

/// Whether `text` holds a piece of a large delimiter or operator (`⎛ ⎞ ⎝ ⎠`, integral extensions,
/// over- and underbrackets). Such a glyph spans the line it sits on; it is not a script.
fn has_large_delimiter_piece(text: &str) -> bool {
    text.chars()
        .any(|c| matches!(c, '\u{2320}' | '\u{2321}' | '\u{239B}'..='\u{23B6}' | '\u{23DC}'..='\u{23E1}'))
}

/// One span as the stack rule sees it.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct StackSpan<'a> {
    pub text: &'a str,
    pub x: f32,
    pub baseline: f32,
    pub width: f32,
    pub font_size: f32,
}

/// Whether a piece of text is compact enough to be a script: not blank, a list marker, a phrase or a
/// piece of a large delimiter.
fn is_compact(c: &StackSpan<'_>) -> bool {
    c.font_size > 0.0
        && !c.text.trim().is_empty()
        && !is_list_marker(c.text)
        && !is_phrase(c.text)
        && !has_large_delimiter_piece(c.text)
}

/// Whether a member has a numerator or denominator partner among `partners`: a compact span of its
/// size over the same x range at a clearly different height, not across the base line from the
/// member. A script pair on the two sides of the base (`H^Z_2`) has one member above the line and
/// one below it. The partner may sit beyond a member's reach from the base line (a numerator raised
/// above its denominator) or on it (the numerator of an index fraction is centred on the subscript
/// line). `partners` are the spans in the stack's x window, so an unrelated span elsewhere on the
/// page cannot take a stack for a fraction.
fn holds_fraction(base: &StackSpan<'_>, members: &[&StackSpan<'_>], partners: &[StackSpan<'_>]) -> bool {
    let bfs = base.font_size;
    members.iter().any(|&member| {
        partners.iter().any(|partner| {
            let shift = partner.baseline - base.baseline;
            let on_base_line = shift.abs() <= bfs * MIN_DISPLACEMENT_EM;
            let same_side = shift * (member.baseline - base.baseline) > 0.0;
            let overlap = (member.x + member.width).min(partner.x + partner.width) - member.x.max(partner.x);
            let narrower = member.width.min(partner.width);
            let smaller = member.font_size.min(partner.font_size);
            let same_size =
                (member.font_size - partner.font_size).abs() <= member.font_size.max(partner.font_size) * MIN_SIZE_STEP;
            !std::ptr::eq(member, partner)
                && is_compact(partner)
                && bfs >= partner.font_size * MIN_BASE_SIZE_FACTOR
                && shift.abs() <= bfs * MAX_FRACTION_PART_SHIFT_EM
                && (on_base_line || same_side)
                && same_size
                && narrower > 0.0
                && overlap > narrower * MIN_FRACTION_OVERLAP
                && (member.baseline - partner.baseline).abs() > smaller * MIN_FRACTION_SEPARATION_EM
        })
    })
}

/// Indices into `candidates` (sorted by `x`) of the members of the stack on `base`, or empty when
/// they do not form one.
///
/// A stack is two or more compact spans, smaller than the base, that abut the base or the member
/// before them in x and sit between a tenth and half an em off the base line. It needs two sizes and
/// a member beyond a single mark's reach, so a lone mark, a fraction, `x_i^2`, a marker pair and
/// chemical scripts stay with the single-mark rules. So does a fraction in an exponent or index
/// (`x^{-1/2}`, `f_{j+3/2}`), whose numerator and denominator share an x range. List markers,
/// phrases and pieces of a large delimiter (`⎞`) are never members, and a base must end in a letter,
/// digit or closing bracket: a large operator (`∑`) is followed by its body, not by scripts.
#[doc(hidden)]
#[must_use]
pub fn script_stack(base: &StackSpan<'_>, candidates: &[StackSpan<'_>]) -> Vec<usize> {
    let bfs = base.font_size;
    let carries_script = base
        .text
        .trim()
        .chars()
        .next_back()
        .is_some_and(|last| last.is_alphanumeric() || matches!(last, ')' | ']' | '}'));
    if bfs.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || !carries_script {
        return Vec::new();
    }
    let mut edge = base.x + base.width;
    let first = candidates.partition_point(|c| c.x < edge - bfs * MAX_GAP_EM);
    let mut members = Vec::new();
    for (i, c) in candidates.iter().enumerate().skip(first) {
        if c.x > edge + bfs * MAX_GAP_EM {
            break;
        }
        let compact = is_compact(c);
        let shift = (c.baseline - base.baseline).abs();
        if compact
            && bfs >= c.font_size * MIN_BASE_SIZE_FACTOR
            && shift > bfs * MIN_DISPLACEMENT_EM
            && shift <= bfs * MAX_SHIFT_EM
        {
            members.push(i);
            edge = edge.max(c.x + c.width);
        }
    }
    if members.len() < 2 || members.len() > MAX_STACK_MEMBERS {
        return Vec::new();
    }
    let window_end = candidates.partition_point(|c| c.x <= edge + bfs * MAX_GAP_EM);
    let member_spans: Vec<&StackSpan<'_>> = members.iter().map(|&i| &candidates[i]).collect();
    if holds_fraction(base, &member_spans, &candidates[first..window_end]) {
        return Vec::new();
    }
    let out_of_reach = members
        .iter()
        .any(|&i| (candidates[i].baseline - base.baseline).abs() > bfs * SnapReach::ORDERING.max_shift_em);
    let (min_fs, max_fs) = members.iter().fold((f32::MAX, 0.0f32), |(lo, hi), &i| {
        (lo.min(candidates[i].font_size), hi.max(candidates[i].font_size))
    });
    if out_of_reach && max_fs - min_fs > max_fs * MIN_SIZE_STEP {
        members
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(text: &str, x: f32, baseline: f32, width: f32, font_size: f32) -> StackSpan<'_> {
        StackSpan {
            text,
            x,
            baseline,
            width,
            font_size,
        }
    }

    #[test]
    fn should_not_stack_a_fraction_in_an_exponent() {
        let base = span("x", 100.0, 100.0, 5.0, 10.0);
        let candidates = [
            span("\u{2212}", 105.0, 104.0, 3.0, 7.0),
            span("1", 108.5, 107.0, 3.0, 5.0),
            span("2", 108.5, 101.5, 3.0, 5.0),
        ];
        assert!(script_stack(&base, &candidates).is_empty());
    }

    #[test]
    fn should_not_stack_a_fraction_in_an_index() {
        let base = span("f", 100.0, 100.0, 4.0, 10.0);
        let candidates = [
            span("j", 104.0, 97.0, 3.0, 7.0),
            span("+", 107.0, 97.0, 4.0, 7.0),
            span("3", 111.5, 99.8, 3.0, 5.0),
            span("2", 111.5, 95.2, 3.0, 5.0),
        ];
        assert!(script_stack(&base, &candidates).is_empty());
    }

    #[test]
    fn should_keep_a_superscript_and_subscript_pair_on_opposite_sides_of_the_base_in_a_stack() {
        let base = span("H", 100.0, 100.0, 7.0, 10.0);
        let candidates = [
            span("Z", 107.0, 104.2, 4.0, 7.0),
            span("2", 107.0, 96.0, 3.5, 5.0),
            span("f", 111.0, 96.0, 3.0, 5.0),
        ];
        assert_eq!(script_stack(&base, &candidates), [0, 1, 2]);
    }

    #[test]
    fn should_refuse_a_stack_longer_than_the_runs_grouping_searches() {
        let base = span("e", 100.0, 100.0, 4.0, 10.0);
        let build = |count: usize| -> Vec<(String, f32, f32)> {
            (0..count)
                .map(|n| {
                    let size = if n % 2 == 0 { 7.0 } else { 5.0 };
                    (n.to_string(), 104.0 + 3.0 * n as f32, size)
                })
                .collect()
        };
        for (count, stacked) in [(MAX_STACK_MEMBERS, true), (MAX_STACK_MEMBERS + 1, false)] {
            let members = build(count);
            let candidates: Vec<StackSpan<'_>> = members
                .iter()
                .map(|(text, x, size)| span(text, *x, 104.2, 3.0, *size))
                .collect();
            assert_eq!(!script_stack(&base, &candidates).is_empty(), stacked, "{count} members");
        }
    }

    #[test]
    fn should_not_take_an_unrelated_span_outside_the_stack_window_for_a_fraction_partner() {
        let base = span("e", 100.0, 100.0, 4.0, 10.0);
        let candidates = [
            span("a wide note", 95.0, 107.5, 30.0, 6.0),
            span("c", 104.0, 104.2, 3.0, 7.5),
            span("1", 107.0, 102.6, 3.0, 6.0),
        ];
        assert_eq!(script_stack(&base, &candidates), [1, 2]);
    }

    #[test]
    fn should_stack_a_superscript_and_subscript_of_one_size_that_share_an_x_range_across_the_base() {
        let base = span("H", 100.0, 100.0, 7.0, 10.0);
        let candidates = [
            span("Z", 107.0, 104.2, 4.0, 7.0),
            span("2", 107.0, 95.8, 4.0, 7.0),
            span("f", 111.0, 96.0, 3.0, 5.0),
        ];
        assert_eq!(script_stack(&base, &candidates), [0, 1, 2]);
    }

    #[test]
    fn should_take_a_same_size_span_on_the_base_line_under_a_member_for_its_fraction_partner() {
        let base = span("H", 100.0, 100.0, 7.0, 10.0);
        let candidates = [
            span("Z", 107.0, 104.2, 4.0, 7.0),
            span("2", 107.0, 100.5, 4.0, 7.0),
            span("f", 111.0, 103.0, 3.0, 5.0),
        ];
        assert!(script_stack(&base, &candidates).is_empty());
    }

    #[test]
    fn should_stack_kerned_same_size_members_that_overlap_by_a_third() {
        let base = span("e", 100.0, 100.0, 4.0, 10.0);
        let candidates = [
            span("c", 104.0, 104.9, 3.0, 7.0),
            span("d", 106.0, 101.1, 3.0, 7.0),
            span("1", 109.5, 103.0, 3.0, 5.0),
        ];
        assert_eq!(script_stack(&base, &candidates), [0, 1, 2]);
    }
}
