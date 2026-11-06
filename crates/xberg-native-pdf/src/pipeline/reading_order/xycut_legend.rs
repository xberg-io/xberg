use crate::layout::TextSpan;

use super::xycut::XYCUT_LINE_Y_TOLERANCE_PTS;

// Unusually dense baseline bands do not supply an unambiguous paired entry.
const MAX_BASELINE_SPANS: usize = 32;
const MAX_LEGEND_ROWS: usize = 16;

pub(super) fn is_paired_legend_run(
    spans: &[TextSpan],
    top_down: &[usize],
    run: &[usize],
    gutter: Option<f32>,
    body_size_ceiling: f32,
) -> bool {
    let Some(gutter) = gutter.filter(|x| x.is_finite()) else {
        return false;
    };
    if !(2..=MAX_LEGEND_ROWS).contains(&run.len()) {
        return false;
    }

    let first = &spans[run[0]];
    let mut right_track = None;
    let mut previous_y = f32::INFINITY;
    let mut previous_right_y = f32::INFINITY;
    for &index in run {
        let left = &spans[index];
        let y = left.bbox.top();
        if left.bbox.right() > gutter
            || (left.bbox.left() - first.bbox.left()).abs() > XYCUT_LINE_Y_TOLERANCE_PTS
            || left.font_name != first.font_name
            || left.font_size != first.font_size
            || previous_y - y <= XYCUT_LINE_Y_TOLERANCE_PTS
            || !is_legend_entry(left, body_size_ceiling)
        {
            return false;
        }

        let Some(right) = parallel_entry(spans, top_down, left, gutter, body_size_ceiling) else {
            return false;
        };
        let anchor = *right_track.get_or_insert(right.bbox.left());
        if (anchor - right.bbox.left()).abs() > XYCUT_LINE_Y_TOLERANCE_PTS
            || previous_right_y - right.bbox.top() <= XYCUT_LINE_Y_TOLERANCE_PTS
        {
            return false;
        }
        previous_y = y;
        previous_right_y = right.bbox.top();
    }
    true
}

fn parallel_entry<'a>(
    spans: &'a [TextSpan],
    top_down: &[usize],
    left: &TextSpan,
    gutter: f32,
    body_size_ceiling: f32,
) -> Option<&'a TextSpan> {
    let y = left.bbox.top();
    let high_y = y + XYCUT_LINE_Y_TOLERANCE_PTS;
    let start = top_down.partition_point(|&i| crate::utils::safe_float_cmp(spans[i].bbox.top(), high_y).is_gt());
    let band = top_down[start..]
        .iter()
        .map(|&i| &spans[i])
        .take_while(|span| span.bbox.top() >= y - XYCUT_LINE_Y_TOLERANCE_PTS);
    let mut match_on_right = None;
    for (index, right) in band.enumerate() {
        if index >= MAX_BASELINE_SPANS {
            return None;
        }
        if right.bbox.left() >= gutter
            && right.font_name == left.font_name
            && right.font_size == left.font_size
            && right.mcid_scope == left.mcid_scope
            && is_legend_entry(right, body_size_ceiling)
        {
            if match_on_right.is_some() {
                return None;
            }
            match_on_right = Some(right);
        }
    }
    match_on_right
}

fn is_legend_entry(span: &TextSpan, body_size_ceiling: f32) -> bool {
    span.mcid.is_none()
        && span.rotation_degrees == 0.0
        && span.wmode == 0
        && !span.mirrored
        && span.bbox.width > 0.0
        && span.bbox.height > 0.0
        && span.font_weight.is_bold()
        && (0.0..=body_size_ceiling).contains(&span.font_size)
        && [span.bbox.x, span.bbox.y, span.bbox.width, span.bbox.height]
            .into_iter()
            .all(f32::is_finite)
        && !span.text.trim().is_empty()
}
