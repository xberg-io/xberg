use super::script_stack::{StackSpan, script_stack};
use super::span_ordering::{SnapReach, snapped_script_baselines};

/// One span as the reading-order snap sees it: text, x, y, width, font size.
pub(crate) type OrderingSpan<'a> = (&'a str, f32, f32, f32, f32);

pub(super) fn is_list_marker(text: &str) -> bool {
    matches!(
        text.trim(),
        "\u{2022}" | "\u{25a0}" | "\u{25aa}" | "\u{25cf}" | "\u{25e6}" | "\u{b7}" | "\u{2013}" | "\u{2014}" | "-"
    )
}

/// Largest rise of a phrase above its base line, in ems of the phrase: a phrase is a line of text, not a mark.
const PHRASE_MAX_RISE_EM: f32 = 0.25;

pub(super) fn is_phrase(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.chars().count() > 3 || trimmed.contains(char::is_whitespace)
}

/// Base-line Y each span snaps to for reading order, or `None` when it stays on its own line.
///
/// Raised glyphs that touch on one baseline form a cluster. The cluster snaps whole when its
/// first glyph abuts the base, so `(\u{2217})` and `1/t` are never split between two rows. List
/// markers never count as marks, and a phrase snaps only when it abuts its base on its own line.
pub(crate) fn ordering_script_baselines(spans: &[OrderingSpan<'_>]) -> Vec<Option<f32>> {
    let snapshot: Vec<_> = spans.iter().map(|&(_, x, y, w, fs)| (x, y, w, fs)).collect();
    let single = snapped_script_baselines(&snapshot, SnapReach::ORDERING);
    let phrase = snapped_script_baselines(&snapshot, SnapReach::ORDERING_PHRASE);
    let raised = snapped_script_baselines(&snapshot, SnapReach::RAISED);
    let own: Vec<Option<f32>> = spans
        .iter()
        .enumerate()
        .map(|(i, &(text, _, y, _, fs))| {
            if is_list_marker(text) {
                None
            } else if is_phrase(text) {
                phrase[i].filter(|&base_y| y - base_y <= fs * PHRASE_MAX_RISE_EM)
            } else {
                single[i]
            }
        })
        .collect();

    let mut members: Vec<usize> = (0..spans.len()).filter(|&i| raised[i].is_some()).collect();
    members.sort_by(|&a, &b| {
        crate::utils::safe_float_cmp(spans[a].2, spans[b].2).then(crate::utils::safe_float_cmp(spans[a].1, spans[b].1))
    });
    let mut result = vec![None; spans.len()];
    let mut start = 0;
    while start < members.len() {
        let mut end = start + 1;
        while end < members.len() {
            let (_, px, py, pw, pfs) = spans[members[end - 1]];
            let (_, x, y, _, fs) = spans[members[end]];
            let touches = (y - py).abs() <= pfs * 0.1 && (fs - pfs).abs() <= pfs * 0.1 && x - (px + pw) <= pfs * 0.1;
            if !touches {
                break;
            }
            end += 1;
        }
        let cluster = &members[start..end];
        if let Some(base_y) = own[cluster[0]]
            && !cluster.iter().any(|&i| is_list_marker(spans[i].0))
        {
            for &i in cluster {
                result[i] = Some(base_y);
            }
        }
        start = end;
    }
    snap_script_stacks(spans, &own, &mut result);
    result
}

/// Snaps the members of a nested script stack (`e^{-c_1 \tau}`, `R^{n_x}`) onto their base's line.
///
/// Which spans form a stack is decided by [`script_stack`]. Spans that already snapped are neither
/// bases nor moved.
fn snap_script_stacks(spans: &[OrderingSpan<'_>], own: &[Option<f32>], result: &mut [Option<f32>]) {
    let to_stack = |i: usize| {
        let (text, x, baseline, width, font_size) = spans[i];
        StackSpan {
            text,
            x,
            baseline,
            width,
            font_size,
        }
    };
    let mut members: Vec<usize> = (0..spans.len()).filter(|&i| spans[i].4 > 0.0).collect();
    members.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[a].1, spans[b].1));
    let candidates: Vec<StackSpan<'_>> = members.iter().map(|&i| to_stack(i)).collect();
    let mut bases: Vec<usize> = (0..spans.len())
        .filter(|&i| spans[i].4 > 0.0 && !spans[i].0.trim().is_empty())
        .collect();
    bases.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[b].4, spans[a].4));

    let mut claimed = vec![false; spans.len()];
    for base in bases {
        if claimed[base] || result[base].is_some() || own[base].is_some() {
            continue;
        }
        let stack: Vec<usize> = script_stack(&to_stack(base), &candidates)
            .into_iter()
            .map(|i| members[i])
            .collect();
        if stack.is_empty() || stack.iter().any(|&m| claimed[m]) {
            continue;
        }
        let by = spans[base].2;
        for m in stack {
            claimed[m] = true;
            if result[m].is_none() {
                result[m] = Some(by);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_snap_a_stack_of_two_sizes_onto_its_base_line() {
        let spans: Vec<OrderingSpan<'_>> = vec![
            ("(", 65.41, 76.24, 4.24, 10.91),
            ("an", 69.65, 76.24, 17.05, 10.91),
            ("1", 75.42, 74.60, 4.23, 7.97),
            ("s", 86.70, 80.74, 3.92, 7.97),
            ("1", 90.61, 79.63, 3.65, 5.98),
            ("+", 97.69, 76.24, 8.49, 10.91),
        ];
        let snapped = ordering_script_baselines(&spans);
        assert_eq!(snapped, [None, None, None, Some(76.24), Some(76.24), None]);
    }

    #[test]
    fn should_leave_a_fraction_and_a_lone_mark_beyond_reach_unstacked() {
        let fraction: Vec<OrderingSpan<'_>> = vec![
            ("4", 10.0, 100.0, 5.0, 10.0),
            ("1", 15.0, 104.5, 3.0, 6.0),
            ("2", 15.0, 99.0, 3.0, 6.0),
        ];
        assert_eq!(ordering_script_baselines(&fraction), [None, None, None]);
        let lone: Vec<OrderingSpan<'_>> = vec![("e", 10.0, 100.0, 5.0, 10.0), ("c", 15.0, 104.5, 3.0, 7.0)];
        assert_eq!(ordering_script_baselines(&lone), [None, None]);
    }
}
