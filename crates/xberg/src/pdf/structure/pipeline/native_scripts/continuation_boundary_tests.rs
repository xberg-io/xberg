use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::assembly::extract_text_and_annotations;
use crate::pdf::structure::types::PdfParagraph;
use crate::types::document_structure::AnnotationKind;

const BODY_FONT: f32 = 12.0;
const BODY_PITCH: f32 = 19.2;

fn span(text: &str, x: f32, baseline: f32, width: f32, font: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, baseline, width, font),
        font_size: font,
        font_name: "boundary-source-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn source(rows: Vec<TextSpan>, context_font: f32) -> (Vec<SegmentData>, NativeSegmentEvidence, usize) {
    let mut spans = vec![
        span("x", 80.0, 800.0, 10.0, BODY_FONT),
        span("4 ", 90.0, 804.9568, 4.0, 8.0),
        span(".", 94.0, 800.0, 4.0, BODY_FONT),
        span("a", 80.0, 750.0, 8.0, context_font),
        span("b", 80.0, 742.0, 8.0, context_font),
    ];
    let start = spans.len();
    spans.extend(rows);
    let expected_records = spans.len();
    let page = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let (segments, evidence) = prepare_segments(page.segments().cloned().collect(), page, &[]);
    let evidence = evidence.expect("actual native preparation must succeed");
    assert_eq!(segments.len(), expected_records);
    let bands = super::super::paragraph_line_bands(segments.iter());
    assert_eq!(
        super::super::median_line_height(&bands),
        Some(context_font),
        "actual physical-band median matches the source body calibration"
    );
    assert_eq!(
        evidence.owners.len(),
        1,
        "one real source power outside the target rows"
    );
    let owner = &evidence.owners[0];
    assert!(owner.base_index < start && owner.script_index < start);
    assert_eq!(segments[owner.script_index].text, "4 ");
    (segments, evidence, start)
}

fn paragraph_for(paragraphs: &[PdfParagraph], text: &str) -> usize {
    let matches: Vec<_> = paragraphs
        .iter()
        .enumerate()
        .filter(|(_, paragraph)| {
            paragraph
                .lines
                .iter()
                .flat_map(|line| &line.segments)
                .any(|segment| segment.text == text)
        })
        .map(|(index, _)| index)
        .collect();
    assert_eq!(matches.len(), 1, "source row {text:?} occurs exactly once");
    matches[0]
}

fn assert_source_preserved(before: &[SegmentData], after: &[PdfParagraph]) {
    let mut original: Vec<_> = before.iter().map(super::line_tests::fingerprint).collect();
    let mut current: Vec<_> = after
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .flat_map(|line| &line.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    assert!(!original.is_empty());
    original.sort();
    current.sort();
    assert_eq!(
        current, original,
        "all source records and immutable fields survive exactly once"
    );
    let powers: Vec<_> = after
        .iter()
        .flat_map(|paragraph| {
            let (text, annotations) = extract_text_and_annotations(paragraph, &Default::default());
            annotations
                .iter()
                .filter(|annotation| annotation.kind == AnnotationKind::Superscript)
                .map(|annotation| text[annotation.start as usize..annotation.end as usize].to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(powers, ["4"], "the unrelated source cohort remains complete");
}

fn extract(rows: Vec<TextSpan>, boundary: (usize, usize)) -> Vec<PdfParagraph> {
    extract_with_context_font(rows, boundary, BODY_FONT)
}

fn extract_with_context_font(rows: Vec<TextSpan>, boundary: (usize, usize), context_font: f32) -> Vec<PdfParagraph> {
    let (segments, evidence, start) = source(rows, context_font);
    let source_ids = [
        evidence.original_indices[start + boundary.0],
        evidence.original_indices[start + boundary.1],
    ];
    let raw_gaps = super::super::compute_paragraph_gap_ys(&segments);
    let copy = NativeSegmentEvidence {
        origin: evidence.origin,
        original_indices: evidence.original_indices.clone(),
        owners: evidence.owners.clone(),
    };
    let (ordered, index) = order_native_paragraph_segments(segments.clone(), copy, &raw_gaps);
    let positions = source_ids.map(|id| index.original_indices.iter().position(|source| *source == id).unwrap());
    let (upper, lower) = (&ordered[positions[0]], &ordered[positions[1]]);
    assert!(upper.baseline_y > lower.baseline_y);
    assert!(
        raw_gaps
            .iter()
            .any(|gap| *gap < upper.baseline_y && *gap > lower.baseline_y)
    );
    assert!(
        index.continuation_gaps.is_some(),
        "actual accepted source owner activates native calibration"
    );
    let grouped = super::super::blocks_to_paragraphs_with_native_scripts(ordered.clone(), &[], &raw_gaps, Some(&index));
    let first = paragraph_for(&grouped, &upper.text);
    let next = paragraph_for(&grouped, &lower.text);
    assert_ne!(
        first, next,
        "original ordinary groups already distinguish this source boundary"
    );
    let mut generic = vec![grouped[first].clone(), grouped[next].clone()];
    crate::pdf::structure::paragraphs::merge_continuation_paragraphs(&mut generic);
    assert_eq!(generic.len(), 1, "the unchanged generic merger admits this boundary");
    let output = super::super::segments_to_paragraphs_with_native_scripts(
        segments.clone(),
        &[],
        &raw_gaps,
        &Default::default(),
        Some(evidence),
    );
    assert_source_preserved(&segments, &output);
    output
}

#[test]
fn should_keep_a_quoted_tail_separate_from_a_new_indented_source_paragraph_2275() {
    let tail = "The spoken symbol ends with \"dee-eks.\"";
    let opening = "An indented paragraph begins a separate account";
    let continuation = "and carries that account onto its next physical row.";
    let output = extract(
        vec![
            span(tail, 80.0, 700.0, 90.0, BODY_FONT),
            span(opening, 96.0, 700.0 - BODY_PITCH, 164.0, BODY_FONT),
            span(continuation, 80.0, 700.0 - BODY_PITCH * 2.0, 180.0, BODY_FONT),
        ],
        (0, 1),
    );
    assert_eq!(paragraph_for(&output, opening), paragraph_for(&output, continuation));
    assert_ne!(
        paragraph_for(&output, tail),
        paragraph_for(&output, opening),
        "a quoted tail and a new indented paragraph remain distinct source paragraphs"
    );
}

#[test]
fn should_keep_a_stacked_display_denominator_separate_from_new_body_prose_2275() {
    let opening = "A fresh paragraph explains the next independent result";
    let continuation = "and carries that account onto its next physical row.";
    let output = extract(
        vec![
            span("dy", 160.0, 708.0, 16.0, BODY_FONT),
            span("dt", 160.0, 700.0, 16.0, BODY_FONT),
            span(opening, 96.0, 677.033, 164.0, BODY_FONT),
            span(continuation, 80.0, 677.033 - BODY_PITCH, 180.0, BODY_FONT),
        ],
        (1, 2),
    );
    assert_eq!(paragraph_for(&output, "dy"), paragraph_for(&output, "dt"));
    assert_eq!(paragraph_for(&output, opening), paragraph_for(&output, continuation));
    assert_ne!(
        paragraph_for(&output, "dt"),
        paragraph_for(&output, opening),
        "a separate displayed fraction must not become the next body paragraph"
    );
}

#[test]
fn should_keep_aligned_five_value_source_rows_as_distinct_paragraphs_2275() {
    let first = ["$100", "$200", "$300", "$400", "$500"];
    let second = ["$110", "$210", "$310", "$410", "$510"];
    let mut rows = Vec::new();
    for (values, baseline) in [(first, 700.0), (second, 683.1)] {
        for (column, value) in values.into_iter().enumerate() {
            rows.push(span(value, 80.0 + column as f32 * 36.0, baseline, 24.0, 9.5));
        }
    }
    let output = extract_with_context_font(rows, (0, first.len()), 9.5);
    for values in [first, second] {
        let row = paragraph_for(&output, values[0]);
        for value in values {
            assert_eq!(
                paragraph_for(&output, value),
                row,
                "all five source values belong to their row"
            );
        }
    }
    assert_ne!(
        paragraph_for(&output, first[0]),
        paragraph_for(&output, second[0]),
        "the two source numeric rows must not become one prose paragraph"
    );
}

#[test]
fn should_join_ordinary_wrapped_source_rows_at_the_same_body_pitch_2275() {
    let first = "An ordinary account continues over the column";
    let second = "and carries the same discussion into another";
    let third = "physical row without starting a new paragraph.";
    let output = extract(
        vec![
            span(first, 80.0, 700.0, 180.0, BODY_FONT),
            span(second, 80.0, 700.0 - BODY_PITCH, 180.0, BODY_FONT),
            span(third, 80.0, 700.0 - BODY_PITCH * 2.0, 180.0, BODY_FONT),
        ],
        (0, 1),
    );
    assert_eq!(paragraph_for(&output, first), paragraph_for(&output, second));
    assert_eq!(paragraph_for(&output, first), paragraph_for(&output, third));
}
