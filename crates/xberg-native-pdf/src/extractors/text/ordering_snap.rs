use super::span_ordering::{SnapReach, snapped_script_baselines};

/// One span as the reading-order snap sees it: text, x, y, width, font size.
pub(crate) type OrderingSpan<'a> = (&'a str, f32, f32, f32, f32);

fn is_list_marker(text: &str) -> bool {
    matches!(
        text.trim(),
        "\u{2022}" | "\u{25a0}" | "\u{25aa}" | "\u{25cf}" | "\u{25e6}" | "\u{b7}" | "\u{2013}" | "\u{2014}" | "-"
    )
}

fn is_phrase(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.chars().count() > 3 || trimmed.contains(char::is_whitespace)
}

/// Base-line Y each span snaps to for reading order, or `None` when it stays on its own line.
///
/// Raised glyphs that touch on one baseline form a cluster, and a cluster snaps only when every
/// glyph in it does, so `(\u{2217})` is never split between two rows. List markers never count as
/// marks, and a phrase snaps only when it abuts its base.
pub(crate) fn ordering_script_baselines(spans: &[OrderingSpan<'_>]) -> Vec<Option<f32>> {
    let snapshot: Vec<_> = spans.iter().map(|&(_, x, y, w, fs)| (x, y, w, fs)).collect();
    let single = snapped_script_baselines(&snapshot, SnapReach::ORDERING);
    let phrase = snapped_script_baselines(&snapshot, SnapReach::ORDERING_PHRASE);
    let raised = snapped_script_baselines(&snapshot, SnapReach::RAISED);
    let own: Vec<Option<f32>> = spans
        .iter()
        .enumerate()
        .map(|(i, &(text, ..))| {
            if is_list_marker(text) {
                None
            } else if is_phrase(text) {
                phrase[i]
            } else {
                single[i]
            }
        })
        .collect();

    let mut members: Vec<usize> = (0..spans.len()).filter(|&i| raised[i].is_some()).collect();
    members.sort_by(|&a, &b| {
        crate::utils::safe_float_cmp(spans[a].2, spans[b].2).then(crate::utils::safe_float_cmp(spans[a].1, spans[b].1))
    });
    let mut result = own.clone();
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
        if members[start..end].iter().any(|&i| own[i].is_none()) {
            for &i in &members[start..end] {
                result[i] = None;
            }
        }
        start = end;
    }
    result
}
