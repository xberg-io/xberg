use super::PdfParagraph;
use crate::pdf::hierarchy::SegmentData;
use crate::pdf::structure::pipeline::{INLINE_STYLE_BASELINE_TOLERANCE, is_script_run_of};

const RUNS_PER_ASSIGNMENT: usize = 4;
const MIN_ASSIGNMENTS: usize = 3;
const MAX_ASSIGNMENTS: usize = 10;
const MAX_RUN_BYTES: usize = 32;
const MAX_SEPARATOR_ADVANCE_EM: f32 = 1.0;
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
        || line.segments.iter().any(|run| run.text.len() > MAX_RUN_BYTES)
    {
        return false;
    }
    let anchor = &lanes[0];
    lanes.iter().enumerate().all(|(position, lane)| {
        is_assignment(lane, anchor)
            && lanes[..position]
                .iter()
                .all(|prior| prior[1].text.trim() != lane[1].text.trim())
    })
        // An ordinary inline sequence has word spacing, rather than a full
        // font-size gap between complete assignment lanes.
        && lanes.windows(2).all(|pair| {
            let gap = advance_gap(&pair[0][3], &pair[1][0]);
            gap.is_finite() && gap >= anchor[0].font_size
        })
}

fn is_assignment(lane: &[SegmentData; RUNS_PER_ASSIGNMENT], anchor: &[SegmentData; RUNS_PER_ASSIGNMENT]) -> bool {
    let [base, index, separator, value] = lane;
    let anchor_base = &anchor[0];
    base.text.trim() == anchor_base.text.trim()
        && separator.text.trim() == anchor[2].text.trim()
        && separator.width <= anchor_base.font_size * MAX_SEPARATOR_ADVANCE_EM
        && lane.iter().all(|run| {
            !run.text.trim().is_empty()
                && run.is_unrotated()
                && run.assigned_role.is_none()
                && run.x.is_finite()
                && run.width.is_finite()
                && run.width > 0.0
                && run.upright_advance_extent().1.is_finite()
        })
        && lane.windows(2).all(|pair| {
            let gap = advance_gap(&pair[0], &pair[1]);
            pair[0].x < pair[1].x
                && (-anchor_base.font_size * MAX_WITHIN_LANE_OVERLAP_EM
                    ..=anchor_base.font_size * MAX_WITHIN_LANE_GAP_EM)
                    .contains(&gap)
        })
        && [base, separator, value].iter().all(|run| {
            (run.baseline_y - anchor_base.baseline_y).abs() <= INLINE_STYLE_BASELINE_TOLERANCE
                && run.font_size == anchor_base.font_size
        })
        && base.baseline_y - index.baseline_y > INLINE_STYLE_BASELINE_TOLERANCE
        && index.font_size < base.font_size
        && is_script_run_of(base, index)
}

fn advance_gap(previous: &SegmentData, next: &SegmentData) -> f32 {
    next.x - previous.upright_advance_extent().1
}
