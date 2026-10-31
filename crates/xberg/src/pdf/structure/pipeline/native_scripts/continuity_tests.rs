use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::assembly::extract_text_and_annotations;
use crate::pdf::structure::types::PdfParagraph;
use crate::types::document_structure::AnnotationKind;

use super::*;

fn span(text: &str, x: f32, baseline: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, baseline, width, font_size),
        font_size,
        font_name: "immutable-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn unrelated_power() -> Vec<TextSpan> {
    vec![
        span("x", 80.0, 800.0, 10.0, 12.0),
        span("4 ", 90.0, 804.9568, 4.0, 8.0),
        span(".", 94.0, 800.0, 4.0, 12.0),
    ]
}

fn carrier(spans: Vec<TextSpan>) -> (Vec<SegmentData>, NativeSegmentEvidence) {
    let page = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let segments = page.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(segments, page, &[]);
    let evidence = evidence.expect("real source projection");
    assert_eq!(evidence.owners.len(), 1, "one actual accepted native source owner");
    (segments, evidence)
}

fn contains(paragraph: &PdfParagraph, prefix: &str) -> bool {
    paragraph
        .lines
        .iter()
        .flat_map(|line| &line.segments)
        .any(|segment| segment.text.starts_with(prefix))
}

fn assert_together(paragraphs: &[PdfParagraph], first: &str, second: &str) {
    let matching: Vec<_> = paragraphs.iter().filter(|p| contains(p, first)).collect();
    assert_eq!(matching.len(), 1, "first source row occurs exactly once");
    assert!(
        contains(matching[0], second),
        "ordinary continuation stays in its source paragraph"
    );
}

fn extract(segments: Vec<SegmentData>, evidence: NativeSegmentEvidence, gaps: &[f32]) -> Vec<PdfParagraph> {
    let mut before: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    let paragraphs = super::super::segments_to_paragraphs_with_native_scripts(
        segments,
        &[],
        gaps,
        &Default::default(),
        Some(evidence),
    );
    let mut after: Vec<_> = paragraphs
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|line| &line.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    before.sort();
    after.sort();
    assert_eq!(after, before, "every native source glyph remains immutable");
    paragraphs
}

#[test]
fn should_retain_ordinary_wrap_across_an_advance_only_marker_2275() {
    let mut spans = unrelated_power();
    spans.extend([
        span("a", 80.0, 750.0, 8.0, 12.0),
        span("b", 80.0, 742.0, 8.0, 12.0),
        span("ordinary prose continues", 80.0, 700.0, 180.0, 12.0),
        span("on the next physical row.", 80.0, 680.8, 180.0, 12.0),
    ]);
    let (segments, evidence) = carrier(spans);
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    assert!(gaps.iter().any(|gap| *gap < 700.0 && *gap > 680.8));
    let legacy = super::super::segments_to_paragraphs(segments.clone(), &[], &gaps, &Default::default());
    assert_together(&legacy, "ordinary prose", "on the next");
    assert_together(&extract(segments, evidence, &gaps), "ordinary prose", "on the next");
}

#[test]
fn should_not_apply_a_foreign_column_band_to_an_ordinary_wrap_2275() {
    let mut spans = unrelated_power();
    spans.extend([
        span("left column context.", 42.0, 610.0, 250.0, 10.0),
        span("another left row.", 42.0, 598.5, 250.0, 10.0),
        span("left context ends.", 42.0, 587.0, 250.0, 10.0),
        span("ter 3", 42.0, 566.8379, 20.0, 10.0),
        span("ordinary right prose", 315.0, 574.3379, 250.0, 10.0),
        span("continues on this row", 315.0, 562.8379, 250.0, 10.0),
        span("and the next physical row", 315.0, 551.3379, 250.0, 10.0),
        span("finishes the same sentence.", 315.0, 539.8379, 250.0, 10.0),
    ]);
    let (segments, evidence) = carrier(spans);
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    assert!(gaps.iter().any(|gap| *gap < 562.8379 && *gap > 551.3379));
    let legacy = super::super::segments_to_paragraphs(segments.clone(), &[], &gaps, &Default::default());
    assert_together(&legacy, "continues on", "and the next");
    assert_together(&extract(segments, evidence, &gaps), "continues on", "and the next");
}

#[test]
fn should_keep_an_ordinary_source_group_after_native_row_label_repair_2275() {
    let (segments, evidence) = carrier(vec![
        span("The example already spans an ordinary row", 80.0, 711.5, 260.0, 10.0),
        span("Because age 70", 80.0, 700.0, 100.0, 10.0),
        span("1", 180.0, 702.2236, 3.892, 7.0),
        span("/", 183.892, 700.0, 2.78, 10.0),
        span("2", 186.672, 700.0, 3.892, 7.0),
        span(" and ordinary prose", 190.564, 700.0, 80.0, 10.0),
        span("continues on the next physical row.", 80.0, 688.5, 240.0, 10.0),
    ]);
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    let legacy = super::super::segments_to_paragraphs(segments.clone(), &[], &gaps, &Default::default());
    assert_together(&legacy, " and ordinary", "continues on");
    let paragraphs = extract(segments, evidence, &gaps);
    assert_together(&paragraphs, " and ordinary", "continues on");
    let powers: Vec<_> = paragraphs
        .iter()
        .flat_map(|p| extract_text_and_annotations(p, &Default::default()).1)
        .filter(|annotation| annotation.kind == AnnotationKind::Superscript)
        .collect();
    assert_eq!(
        powers.len(),
        1,
        "the unraised denominator must not become another power"
    );
}

#[test]
fn should_keep_an_inline_power_inside_ordinary_wrapped_prose_2275() {
    let (segments, evidence) = carrier(vec![
        span("a", 80.0, 750.0, 8.0, 12.0),
        span("b", 80.0, 742.0, 8.0, 12.0),
        span("An inline ", 80.0, 700.0, 70.0, 12.0),
        span("x", 150.0, 700.0, 10.0, 12.0),
        span("4 ", 160.0, 704.9568, 4.0, 8.0),
        span(" continues with ordinary", 164.0, 700.0, 150.0, 12.0),
        span("words on another physical row.", 80.0, 680.8, 180.0, 12.0),
    ]);
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    assert!(gaps.iter().any(|gap| *gap < 700.0 && *gap > 680.8));
    let paragraphs = extract(segments, evidence, &gaps);
    assert_together(&paragraphs, "An inline", "words on another");
    let paragraph = paragraphs.iter().find(|p| contains(p, "An inline")).unwrap();
    let (text, annotations) = extract_text_and_annotations(paragraph, &Default::default());
    let powers: Vec<_> = annotations
        .iter()
        .filter(|annotation| annotation.kind == AnnotationKind::Superscript)
        .map(|a| &text[a.start as usize..a.end as usize])
        .collect();
    assert_eq!(powers, ["4"]);
}

#[test]
fn should_preserve_a_genuine_blank_gap_between_ordinary_paragraphs_2275() {
    let mut spans = unrelated_power();
    spans.extend([
        span("Wrapped prose begins", 80.0, 700.0, 180.0, 12.0),
        span("and continues across", 80.0, 686.0, 180.0, 12.0),
        span("another ordinary row:", 80.0, 672.0, 180.0, 12.0),
        span("separate paragraph starts here.", 80.0, 644.0, 180.0, 12.0),
    ]);
    let (segments, evidence) = carrier(spans);
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    assert!(gaps.iter().any(|gap| *gap < 672.0 && *gap > 644.0));
    let paragraphs = extract(segments, evidence, &gaps);
    assert_together(&paragraphs, "Wrapped prose", "another ordinary");
    let first = paragraphs.iter().find(|p| contains(p, "Wrapped prose")).unwrap();
    assert!(
        !contains(first, "separate paragraph"),
        "a genuine blank gap must remain a paragraph boundary"
    );
    assert_eq!(
        paragraphs.iter().filter(|p| contains(p, "separate paragraph")).count(),
        1
    );
}
