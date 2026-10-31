use super::*;

fn assert_row_evidence(
    segments: &[SegmentData],
    evidence: &NativeSegmentEvidence,
    first: &str,
    next: &str,
    room: bool,
) {
    let previous = segments.iter().find(|segment| segment.text == first).unwrap();
    let following = segments.iter().find(|segment| segment.text == next).unwrap();
    let right_edge = following.upright_advance_extent().1;
    assert!(!super::super::super::heading_fills_column(previous, right_edge));
    assert_eq!(
        super::super::super::heading_line_ran_out_of_room(previous, following, right_edge),
        room
    );
    let all_bands = super::super::super::paragraph_line_bands(segments.iter());
    let height = super::super::super::median_line_height(&all_bands).unwrap();
    let leading = super::super::super::body_leading(&all_bands, height);
    let scripts: std::collections::HashSet<_> = evidence.owners.iter().map(|owner| owner.script_index).collect();
    let ordinary: Vec<_> = segments
        .iter()
        .enumerate()
        .filter(|(index, _)| !scripts.contains(index))
        .map(|(_, segment)| segment)
        .collect();
    let corridor = super::super::super::page_column_corridor_for_segments(ordinary.iter().copied());
    assert_eq!(corridor.is_none(), room, "separate page-local and foreign-column cases");
    let local = ordinary
        .into_iter()
        .filter(|segment| corridor.is_none_or(|(_, right)| segment.upright_advance_extent().0 >= right));
    let local_bands = super::super::super::paragraph_line_bands(local);
    let local_gaps = super::super::super::paragraph_gaps_with_leading(&local_bands, height, leading);
    let crosses = local_gaps
        .iter()
        .any(|gap| *gap < previous.baseline_y && *gap > following.baseline_y);
    assert_eq!(
        crosses, room,
        "the foreign global marker has no same-column source-gap witness"
    );
}

fn assert_wrap(spans: Vec<TextSpan>, first: &str, next: &str, room: bool) {
    let count = spans.len();
    let (segments, evidence) = carrier(spans);
    assert_eq!(segments.len(), count);
    let gaps = super::super::super::compute_paragraph_gap_ys(&segments);
    let previous = segments.iter().find(|segment| segment.text == first).unwrap();
    let following = segments.iter().find(|segment| segment.text == next).unwrap();
    assert!(
        gaps.iter()
            .any(|gap| *gap < previous.baseline_y && *gap > following.baseline_y)
    );
    assert_row_evidence(&segments, &evidence, first, next, room);
    let copy = NativeSegmentEvidence {
        origin: evidence.origin,
        original_indices: evidence.original_indices.clone(),
        owners: evidence.owners.clone(),
    };
    let (ordered, index) = super::super::order_native_paragraph_segments(segments.clone(), copy, &gaps);
    let first_index = ordered.iter().position(|segment| segment.text == first).unwrap();
    let next_index = ordered.iter().position(|segment| segment.text == next).unwrap();
    let groups = index.ordinary_groups.as_ref().expect("actual source group plan");
    assert_ne!(groups.original[first_index], groups.original[next_index]);
    assert_ne!(groups.repaired[first_index], groups.repaired[next_index]);
    assert_together(&extract(segments, evidence, &gaps), first, next);
}

#[test]
fn should_keep_a_ragged_source_wrap_when_the_next_word_does_not_fit_2275() {
    let first = "ordinary source prose continues";
    let next = "transcendentalparameterization carries onward";
    let mut spans = unrelated_power();
    spans.extend([
        span("a", 80.0, 750.0, 8.0, 12.0),
        span("b", 80.0, 742.0, 8.0, 12.0),
        span(first, 80.0, 700.0, 120.0, 12.0),
        span(next, 80.0, 680.8, 180.0, 12.0),
    ]);
    assert_wrap(spans, first, next, true);
}

#[test]
fn should_keep_a_ragged_source_wrap_when_the_raw_marker_is_foreign_2275() {
    let first = "continues on this row";
    let next = "and carries the ordinary account forward";
    let mut spans = unrelated_power();
    spans.extend([
        span("left column context.", 42.0, 610.0, 250.0, 10.0),
        span("another left row.", 42.0, 598.5, 250.0, 10.0),
        span("left context ends.", 42.0, 587.0, 250.0, 10.0),
        span("ter 3", 42.0, 566.8379, 20.0, 10.0),
        span("ordinary right prose", 315.0, 574.3379, 250.0, 10.0),
        span(first, 315.0, 562.8379, 180.0, 10.0),
        span(next, 315.0, 551.3379, 250.0, 10.0),
        span("finishes the same sentence.", 315.0, 539.8379, 250.0, 10.0),
    ]);
    assert_wrap(spans, first, next, false);
}
