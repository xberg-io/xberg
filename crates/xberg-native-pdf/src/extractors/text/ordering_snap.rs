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
    result
}

/// The nested script stacks (`e^{-c_1 \tau}`, `R^{n_x}`) among `spans`: each base with the indices of
/// its members in x order.
///
/// Which spans form a stack is decided by [`script_stack`]. A span that a single-mark snap already
/// moved (`snapped`) is not a base; it still counts as a member, so the stack reads as one run. The
/// stacks are not snapped: a snap moves spans before the page's column and band passes read them, and
/// a stack must not change how those split the page, so the caller places the members after the page
/// is ordered.
pub(crate) fn ordering_script_stacks(spans: &[OrderingSpan<'_>], snapped: &[Option<f32>]) -> Vec<(usize, Vec<usize>)> {
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
    let mut stacks = Vec::new();
    for base in bases {
        if claimed[base] || snapped[base].is_some() {
            continue;
        }
        let stack: Vec<usize> = script_stack(&to_stack(base), &candidates)
            .into_iter()
            .map(|i| members[i])
            .collect();
        if stack.is_empty() || stack.iter().any(|&m| claimed[m]) {
            continue;
        }
        for &m in &stack {
            claimed[m] = true;
        }
        stacks.push((base, stack));
    }
    stacks
}

/// The nested script stacks among `spans`: each base with the indices of its members in x order.
///
/// Nothing is snapped: the caller places the members after the page has been ordered and
/// partitioned, the raised members first and the lowered ones after them, each side in x order.
#[doc(hidden)]
#[must_use]
pub fn script_stacks(spans: &[StackSpan<'_>]) -> Vec<(usize, Vec<usize>)> {
    let ordering: Vec<OrderingSpan<'_>> = spans
        .iter()
        .map(|span| (span.text, span.x, span.baseline, span.width, span.font_size))
        .collect();
    ordering_script_stacks(&ordering, &ordering_script_baselines(&ordering))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_report_a_stack_of_two_sizes_on_its_base_without_snapping_it() {
        let spans: Vec<OrderingSpan<'_>> = vec![
            ("(", 65.41, 76.24, 4.24, 10.91),
            ("an", 69.65, 76.24, 17.05, 10.91),
            ("1", 75.42, 74.60, 4.23, 7.97),
            ("s", 86.70, 80.74, 3.92, 7.97),
            ("1", 90.61, 79.63, 3.65, 5.98),
            ("+", 97.69, 76.24, 8.49, 10.91),
        ];
        let snapped = ordering_script_baselines(&spans);
        assert_eq!(snapped, [None; 6]);
        assert_eq!(ordering_script_stacks(&spans, &snapped), [(1, vec![3, 4])]);
    }

    #[test]
    fn should_leave_a_fraction_and_a_lone_mark_beyond_reach_unstacked() {
        let fraction: Vec<OrderingSpan<'_>> = vec![
            ("4", 10.0, 100.0, 5.0, 10.0),
            ("1", 15.0, 104.5, 3.0, 6.0),
            ("2", 15.0, 99.0, 3.0, 6.0),
        ];
        assert!(ordering_script_stacks(&fraction, &ordering_script_baselines(&fraction)).is_empty());
        let lone: Vec<OrderingSpan<'_>> = vec![("e", 10.0, 100.0, 5.0, 10.0), ("c", 15.0, 104.5, 3.0, 7.0)];
        assert!(ordering_script_stacks(&lone, &ordering_script_baselines(&lone)).is_empty());
    }

    #[test]
    fn should_report_stacks_of_stack_spans_and_skip_a_large_bracket_piece() {
        let span = |text, x, baseline, width, font_size| StackSpan {
            text,
            x,
            baseline,
            width,
            font_size,
        };
        let stack = [
            span("e", 100.0, 100.0, 4.0, 10.0),
            span("c", 104.0, 104.2, 3.0, 7.5),
            span("1", 107.0, 102.6, 3.0, 6.0),
        ];
        assert_eq!(script_stacks(&stack), [(0, vec![1, 2])]);
        let bracket = [
            span("\u{b5}", 100.0, 100.0, 5.0, 10.0),
            span("\u{239e}", 105.2, 104.1, 2.2, 4.97),
            span("2", 107.8, 104.1, 4.0, 8.0),
        ];
        assert!(script_stacks(&bracket).is_empty());
    }
}
