use super::{Word, plan};

fn word(text: &str, left: u32, top: u32, confidence: f64) -> Word<'_> {
    Word {
        text,
        bounds: [left, top, left + 10, top + 10],
        confidence,
    }
}

#[test]
fn ambiguous_overlapping_boxes_prevent_substitution() {
    let baseline = [
        word("the", 0, 0, 0.9),
        word("track", 12, 0, 0.9),
        word("ends", 24, 0, 0.9),
    ];
    let mut candidate = [
        baseline[0],
        word("truck", 12, 0, 0.4),
        baseline[2],
        word("stack", 13, 0, 0.5),
    ];
    assert_eq!(plan(&baseline, &candidate[..3], &[]).substitutions, [(1, 1)]);
    assert_eq!(plan(&baseline, &candidate, &[]).substitutions, []);
    candidate[3].bounds = [80, 0, 90, 10];
    assert_eq!(plan(&baseline, &candidate, &[]).substitutions, [(1, 1)]);
}

#[test]
fn duplicate_words_are_paired_at_their_own_source_occurrences() {
    let baseline = [
        word("turn", 0, 0, 0.9),
        word("right", 12, 0, 0.9),
        word("here", 24, 0, 0.9),
        word("turn", 0, 20, 0.9),
        word("right", 12, 20, 0.3),
        word("here", 24, 20, 0.9),
    ];
    let candidate = [
        baseline[3],
        word("light", 12, 20, 0.8),
        baseline[5],
        baseline[0],
        word("light", 12, 0, 0.4),
        baseline[2],
    ];
    assert_eq!(plan(&baseline, &candidate, &[]).substitutions, [(1, 4)]);
}

#[test]
fn reunites_a_detached_continuation_only_when_its_source_line_is_contiguous() {
    let baseline = [
        word("one", 0, 0, 0.9),
        word("two", 12, 0, 0.9),
        word("three", 24, 0, 0.9),
        word("four", 36, 0, 0.9),
        word("other", 0, 20, 0.9),
    ];
    let mut candidate = [baseline[0], baseline[1], baseline[4], baseline[2], baseline[3]];
    let source_line = std::iter::once(0..4).collect::<Vec<_>>();
    assert_eq!(plan(&baseline, &candidate, &source_line).detached_lines, [(0..2, 3..5)]);
    candidate.swap(3, 4);
    assert_eq!(plan(&baseline, &candidate, &source_line).detached_lines, []);
}
