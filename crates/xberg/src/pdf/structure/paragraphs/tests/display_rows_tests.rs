use super::{indexed_display_row, merge_after_prose};

#[test]
fn continuation_merge_keeps_indexed_display_rows_with_changed_source_literals() {
    for (run, literals) in [
        (0, ["λ", "λ", "λ"]),
        (1, ["9", "10", "11"]),
        (2, [":", ":", ":"]),
        (3, ["0.5", "-3", "(4)"]),
    ] {
        let mut row = indexed_display_row();
        for (segment, literal) in row.lines[0].segments.iter_mut().skip(run).step_by(4).zip(literals) {
            segment.text = literal.to_owned();
        }
        let paragraphs = merge_after_prose(row);
        assert_eq!(paragraphs.len(), 2, "display row with source literals {literals:?}");
        let retained: Vec<_> = paragraphs[1].lines[0]
            .segments
            .iter()
            .skip(run)
            .step_by(4)
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(retained, literals);
    }
}

#[test]
fn continuation_merge_refuses_inconsistent_display_lane_anchors() {
    for (run, literal) in [(0, "b"), (1, "1"), (2, ":")] {
        let mut row = indexed_display_row();
        row.lines[0].segments[4 + run].text = literal.to_owned();
        assert_eq!(
            merge_after_prose(row).len(),
            1,
            "inconsistent display lane at run {run}"
        );
    }
}

#[test]
fn continuation_merge_refuses_word_width_separators_and_unbounded_source_runs() {
    let mut row = indexed_display_row();
    for lane in row.lines[0].segments.as_chunks_mut::<4>().0 {
        lane[2].text = "therefore".to_owned();
        lane[2].width = 14.0;
        lane[3].x += 8.0;
    }
    assert_eq!(
        merge_after_prose(row).len(),
        1,
        "a wide prose run cannot certify a separator"
    );

    let mut row = indexed_display_row();
    row.lines[0].segments[3].text = format!("12{:>64}", "");
    assert_eq!(
        merge_after_prose(row).len(),
        1,
        "whitespace still counts toward the raw run bound"
    );
}
