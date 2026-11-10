use super::span_ordering::{SnapReach, snapped_script_baselines};

/// One span as the reading-order snap sees it: text, x, y, width, font size.
pub(crate) type OrderingSpan<'a> = (&'a str, f32, f32, f32, f32);

fn is_list_marker(text: &str) -> bool {
    matches!(
        text.trim(),
        "\u{2022}" | "\u{25a0}" | "\u{25aa}" | "\u{25cf}" | "\u{25e6}" | "\u{b7}" | "\u{2013}" | "\u{2014}" | "-"
    )
}

/// Largest rise of a phrase above its base line, in ems of the phrase: a phrase is a line of text, not a mark.
const PHRASE_MAX_RISE_EM: f32 = 0.25;

fn is_phrase(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.chars().count() > 3 || trimmed.contains(char::is_whitespace)
}

/// What the reading-order snap decided for a page of spans.
pub(crate) struct OrderingSnap {
    /// Base-line Y each span snaps to for reading order, or `None` when it stays on its own line.
    pub(crate) base_y: Vec<Option<f32>>,
    /// For a member of a nested script stack, the index of its base span. The extraction sets the
    /// member in its base's line and style for good: its raw baseline sits beyond the reach the
    /// later line assembly accepts for a mark.
    pub(crate) stack_base: Vec<Option<usize>>,
}

/// Base-line Y each span snaps to for reading order, or `None` when it stays on its own line.
///
/// Raised glyphs that touch on one baseline form a cluster. The cluster snaps whole when its
/// first glyph abuts the base, so `(\u{2217})` and `1/t` are never split between two rows. List
/// markers never count as marks, and a phrase snaps only when it abuts its base on its own line.
pub(crate) fn ordering_script_baselines(spans: &[OrderingSpan<'_>]) -> OrderingSnap {
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
    let stack_base = snap_script_stacks(spans, &own, &mut result);
    OrderingSnap {
        base_y: result,
        stack_base,
    }
}

/// Largest rise or drop of a stack member from its base line, in ems of the base.
const STACK_MAX_SHIFT_EM: f32 = 0.5;
/// Largest gap between a stack member and the right edge of the base or the stack so far, in ems of the base.
const STACK_MAX_GAP_EM: f32 = 0.25;
/// A stack member must be smaller than its base by at least this factor.
const STACK_MIN_BASE_SIZE_FACTOR: f32 = 1.15;
/// Members that differ in size by more than this fraction of the larger one sit at two levels of a stack. A
/// numerator and denominator, or a run of same-size exponent glyphs, are not a stack.
const STACK_MIN_SIZE_STEP: f32 = 0.1;

/// Snaps the members of a nested script stack (`e^{-c_1 \tau}`, `R^{n_x}`) onto their base's line.
///
/// The members are small spans that abut their base, or the member before them, in x and sit within half an
/// em of the base line. A stack needs two members at two sizes, one of them beyond the reach of a single
/// mark, so a lone mark, which the single-mark reach decides, a fraction and a run of same-size glyphs are
/// left alone. Spans that already snapped, and list markers and phrases, are neither members nor moved.
fn snap_script_stacks(
    spans: &[OrderingSpan<'_>],
    own: &[Option<f32>],
    result: &mut [Option<f32>],
) -> Vec<Option<usize>> {
    let is_member_shape = |i: usize| {
        let (text, _, _, _, fs) = spans[i];
        fs > 0.0 && !text.trim().is_empty() && !is_list_marker(text) && !is_phrase(text)
    };
    let mut members: Vec<usize> = (0..spans.len()).filter(|&i| is_member_shape(i)).collect();
    members.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[a].1, spans[b].1));
    let mut bases: Vec<usize> = (0..spans.len())
        .filter(|&i| spans[i].4 > 0.0 && !spans[i].0.trim().is_empty())
        .collect();
    bases.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[b].4, spans[a].4));

    let mut claimed = vec![false; spans.len()];
    let mut stack_base = vec![None; spans.len()];
    for base in bases {
        if claimed[base] || result[base].is_some() || own[base].is_some() {
            continue;
        }
        let (_, bx, by, bw, bfs) = spans[base];
        let mut stack: Vec<usize> = Vec::new();
        let mut edge = bx + bw;
        let first = members.partition_point(|&m| spans[m].1 < edge - bfs * STACK_MAX_GAP_EM);
        for &m in &members[first..] {
            let (_, mx, my, mw, mfs) = spans[m];
            if mx > edge + bfs * STACK_MAX_GAP_EM {
                break;
            }
            let fits = m != base
                && !claimed[m]
                && bfs >= mfs * STACK_MIN_BASE_SIZE_FACTOR
                && (my - by).abs() <= bfs * STACK_MAX_SHIFT_EM;
            if fits {
                stack.push(m);
                edge = edge.max(mx + mw);
            }
        }
        if stack.len() < 2 {
            continue;
        }
        let out_of_reach = stack
            .iter()
            .any(|&m| (spans[m].2 - by).abs() > bfs * SnapReach::ORDERING.max_shift_em);
        let (min_fs, max_fs) = stack.iter().fold((f32::MAX, 0.0f32), |(lo, hi), &m| {
            (lo.min(spans[m].4), hi.max(spans[m].4))
        });
        let two_sizes = max_fs - min_fs > max_fs * STACK_MIN_SIZE_STEP;
        if !(out_of_reach && two_sizes) {
            continue;
        }
        for m in stack {
            claimed[m] = true;
            if result[m].is_none() {
                result[m] = Some(by);
                stack_base[m] = Some(base);
            }
        }
    }
    stack_base
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
        let snap = ordering_script_baselines(&spans);
        assert_eq!(snap.base_y[3], Some(76.24), "{:?}", snap.base_y);
        assert_eq!(snap.base_y[4], Some(76.24), "{:?}", snap.base_y);
        assert_eq!(snap.stack_base, [None, None, None, Some(1), Some(1), None]);
    }

    #[test]
    fn should_leave_a_fraction_and_a_lone_mark_beyond_reach_unstacked() {
        let fraction: Vec<OrderingSpan<'_>> = vec![
            ("4", 10.0, 100.0, 5.0, 10.0),
            ("1", 15.0, 104.5, 3.0, 6.0),
            ("2", 15.0, 99.0, 3.0, 6.0),
        ];
        assert_eq!(ordering_script_baselines(&fraction).stack_base, [None, None, None]);
        let lone: Vec<OrderingSpan<'_>> = vec![("e", 10.0, 100.0, 5.0, 10.0), ("c", 15.0, 104.5, 3.0, 7.0)];
        assert_eq!(ordering_script_baselines(&lone).stack_base, [None, None]);
    }
}
