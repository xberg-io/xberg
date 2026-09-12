//! Pure-arithmetic script-run decision rule, shared by running-prose assembly and table-cell
//! reading order.
//!
//! A sub/superscript sits at a baseline offset from the run it annotates and is drawn in a
//! materially smaller font; both properties are ordinary floats regardless of which domain type
//! (`SegmentData` for prose, `xberg_native_pdf::layout::TextSpan` for table cells) supplied them.
//! This module holds only that float arithmetic. Each caller keeps its own type-specific guards
//! (monospace, semantic role, rotation, finiteness) and geometry accessors, and calls in here for
//! the actual baseline/font-size and forward-gap decisions.
//!
//! ~keep The three constants below are read by every script-run consumer in the crate:
//! `pdf::structure::pipeline` (prose assembly, GH#1617), `pdf::native::table` (structural table
//! cells, GH#1628) and `pdf::table_reconstruct` (word-level table reconstruction, GH#1628). They
//! were duplicated across the first two for one release; keep them here so a threshold cannot be
//! tuned on one path and silently disagree with the others.

/// Largest baseline offset, as a multiple of the larger font size, at which a smaller abutting run
/// still reads as a sub/superscript of its neighbour rather than as the next wrapped line.
///
/// ~keep GH#1617: the five torn pairs on the reproducer sit 0.63--1.07pt off a 11.59pt baseline,
/// i.e. 0.054--0.092 font-sizes, while a wrapped line is separated by a full leading (>= 1.0). The
/// 3x margin either side is why this is expressible as a predicate instead of by widening
/// `pdf::structure::pipeline`'s `INLINE_STYLE_BASELINE_TOLERANCE`, which is read at seven sites
/// there and gates dehyphenation at `spans_visual_line_break`.
pub(crate) const SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR: f32 = 0.35;

/// Largest ratio of the smaller run's font size to the larger one's for the pair to read as a
/// sub/superscript.
///
/// ~keep GH#1617: 0.63--0.73 on the reproducer against 1.00 for every ordinary same-size style-run
/// boundary, so a same-size boundary cannot reach this predicate at all.
pub(crate) const SCRIPT_RUN_MAX_FONT_SIZE_RATIO: f32 = 0.85;

/// Largest forward gap, as a multiple of the larger font size, between a run's end and an abutting
/// sub/superscript's start.
///
/// ~keep GH#1617: `rated` starts at 211.92 where `P` ends at 211.94 -- scripts abut or overlap
/// their base, so this stays far below the ordinary inline-style forward-gap allowance.
pub(crate) const SCRIPT_RUN_MAX_FORWARD_GAP_FONT_FACTOR: f32 = 0.25;

/// Whether a non-zero baseline offset paired with a materially smaller font reads as a
/// sub/superscript relationship.
///
/// `font_size` is the larger of the two runs' font sizes; `smaller_font_size` is the smaller.
/// Deliberately requires a *non-zero* offset — a run at the identical baseline is a same-size
/// style transition, not a script run, and is handled by the caller's own on-baseline path.
pub(crate) fn is_script_run_baseline_offset(baseline_delta: f32, font_size: f32, smaller_font_size: f32) -> bool {
    baseline_delta > 0.0
        && baseline_delta <= font_size * SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR
        && smaller_font_size <= font_size * SCRIPT_RUN_MAX_FONT_SIZE_RATIO
}

/// Whether `next`'s advance-axis start reads as abutting or overlapping `previous`'s extent — the
/// forward-gap half of the sub/superscript decision, independent of the baseline/font-size half.
pub(crate) fn is_script_run_forward_gap(
    previous_start: f32,
    previous_end: f32,
    next_start: f32,
    font_size: f32,
) -> bool {
    next_start >= previous_start && next_start - previous_end <= font_size * SCRIPT_RUN_MAX_FORWARD_GAP_FONT_FACTOR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_accept_small_positive_offset_with_smaller_font() {
        // 11.59pt base, ~0.7pt offset (< 0.35 * 11.59), subscript at 8pt (<= 0.85 * 11.59). ~keep
        assert!(is_script_run_baseline_offset(0.7, 11.59, 8.0));
    }

    #[test]
    fn should_reject_zero_offset() {
        assert!(!is_script_run_baseline_offset(0.0, 11.59, 8.0));
    }

    #[test]
    fn should_reject_offset_beyond_baseline_factor() {
        assert!(!is_script_run_baseline_offset(5.0, 11.59, 8.0));
    }

    #[test]
    fn should_reject_font_size_ratio_too_close_to_one() {
        // 11.0 / 11.59 = 0.949, above the 0.85 ratio ceiling.
        assert!(!is_script_run_baseline_offset(0.7, 11.59, 11.0));
    }

    #[test]
    fn should_accept_abutting_forward_gap() {
        // A run starting at 205.0 and ending at 211.94 ("P"), abutted by a script run
        // starting at 211.92 -- 0.02pt inside the base's own extent, the kerned-TJ
        // containment shape GH#1617 (`rated` starting where `P` ends) exercises.
        assert!(is_script_run_forward_gap(205.0, 211.94, 211.92, 11.59));
    }

    #[test]
    fn should_reject_forward_gap_that_starts_before_previous() {
        assert!(!is_script_run_forward_gap(211.94, 211.94, 200.0, 11.59));
    }

    #[test]
    fn should_reject_forward_gap_beyond_font_factor() {
        assert!(!is_script_run_forward_gap(100.0, 100.0, 110.0, 11.59));
    }
}
