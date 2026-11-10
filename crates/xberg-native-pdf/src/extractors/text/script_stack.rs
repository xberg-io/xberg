//! The one rule that decides whether spans form a nested script stack on a base glyph.
//!
//! Reading-order snapping, paragraph-row assembly and script annotation all ask the same question
//! about `(text, x, baseline, width, font size)`, so they share this predicate and cannot drift.

use super::ordering_snap::{is_list_marker, is_phrase};

/// Largest rise or drop of a stack member from its base line, in ems of the base.
const MAX_SHIFT_EM: f32 = 0.5;
/// Largest rise or drop of a lone mark (`SnapReach::ORDERING`); a stack needs a member beyond it.
const SINGLE_MARK_REACH_EM: f32 = 0.35;
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

/// Indices into `candidates` (sorted by `x`) of the members of the stack on `base`, or empty when
/// they do not form one.
///
/// A stack is two or more compact spans, smaller than the base, that abut the base or the member
/// before them in x and sit between a tenth and half an em off the base line. It needs two sizes and a member
/// beyond a single mark's reach, so a lone mark, a fraction, `x_i^2`, a marker pair and chemical
/// scripts stay with the single-mark rules. List markers and phrases are never members, and a base
/// must end in a letter, digit or closing bracket: a large operator (`∑`) is followed by its body,
/// not by scripts.
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
        let compact = c.font_size > 0.0 && !c.text.trim().is_empty() && !is_list_marker(c.text) && !is_phrase(c.text);
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
    if members.len() < 2 {
        return Vec::new();
    }
    let out_of_reach = members
        .iter()
        .any(|&i| (candidates[i].baseline - base.baseline).abs() > bfs * SINGLE_MARK_REACH_EM);
    let (min_fs, max_fs) = members.iter().fold((f32::MAX, 0.0f32), |(lo, hi), &i| {
        (lo.min(candidates[i].font_size), hi.max(candidates[i].font_size))
    });
    if out_of_reach && max_fs - min_fs > max_fs * MIN_SIZE_STEP {
        members
    } else {
        Vec::new()
    }
}
