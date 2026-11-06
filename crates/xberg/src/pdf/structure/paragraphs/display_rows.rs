use super::PdfParagraph;
use crate::pdf::hierarchy::SegmentData;
use crate::pdf::structure::pipeline::{INLINE_STYLE_BASELINE_TOLERANCE, is_script_run_of};

const RUNS_PER_ASSIGNMENT: usize = 4;
const MIN_ASSIGNMENTS: usize = 3;
const MAX_ASSIGNMENTS: usize = 10;
const MAX_VALUE_BYTES: usize = 32;
const MAX_WITHIN_LANE_GAP_EM: f32 = 1.0;
const MAX_WITHIN_LANE_OVERLAP_EM: f32 = 0.25;

pub(super) fn is_indexed_display_row(paragraph: &PdfParagraph) -> bool {
    let [line] = paragraph.lines.as_slice() else {
        return false;
    };
    let (lanes, extra) = line.segments.as_chunks::<RUNS_PER_ASSIGNMENT>();
    if paragraph.is_page_furniture
        || paragraph.layout_class.is_some()
        || paragraph.layout_region_path.is_some()
        || paragraph.caption_for.is_some()
        || !extra.is_empty()
        || !(MIN_ASSIGNMENTS..=MAX_ASSIGNMENTS).contains(&lanes.len())
    {
        return false;
    }
    let [base, index, ..] = &lanes[0];
    let label = base.text.trim();
    let [first_digit] = index.text.trim().as_bytes() else {
        return false;
    };
    label.len() == 1
        && label.as_bytes()[0].is_ascii_alphabetic()
        && first_digit.is_ascii_digit()
        && lanes.iter().enumerate().all(|(position, lane)| {
            is_assignment(lane, base, *first_digit + position as u8)
        })
        // An ordinary inline sequence has word spacing, rather than a full
        // font-size gap between complete assignment lanes.
        && lanes.windows(2).all(|pair| {
            let gap = advance_gap(&pair[0][3], &pair[1][0]);
            gap.is_finite() && gap >= base.font_size
        })
}

fn is_assignment(lane: &[SegmentData; RUNS_PER_ASSIGNMENT], anchor: &SegmentData, expected_index: u8) -> bool {
    let [base, index, equals, value] = lane;
    let literal = value.text.trim();
    base.text.trim() == anchor.text.trim()
        && expected_index.is_ascii_digit()
        && index.text.trim().as_bytes() == [expected_index]
        && equals.text.trim() == "="
        && literal.len() <= MAX_VALUE_BYTES
        && literal.bytes().all(|byte| byte.is_ascii_alphanumeric())
        && literal.bytes().any(|byte| byte.is_ascii_digit())
        && lane.iter().all(|run| {
            run.is_unrotated()
                && run.assigned_role.is_none()
                && run.x.is_finite()
                && run.width.is_finite()
                && run.width > 0.0
                && run.upright_advance_extent().1.is_finite()
        })
        && lane.windows(2).all(|pair| {
            let gap = advance_gap(&pair[0], &pair[1]);
            pair[0].x < pair[1].x
                && (-anchor.font_size * MAX_WITHIN_LANE_OVERLAP_EM..=anchor.font_size * MAX_WITHIN_LANE_GAP_EM)
                    .contains(&gap)
        })
        && [base, equals, value].iter().all(|run| {
            (run.baseline_y - anchor.baseline_y).abs() <= INLINE_STYLE_BASELINE_TOLERANCE
                && run.font_size == anchor.font_size
        })
        && base.baseline_y - index.baseline_y > INLINE_STYLE_BASELINE_TOLERANCE
        && index.font_size < base.font_size
        && is_script_run_of(base, index)
}

fn advance_gap(previous: &SegmentData, next: &SegmentData) -> f32 {
    next.x - previous.upright_advance_extent().1
}
