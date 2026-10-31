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

fn evidence(spans: Vec<TextSpan>, expected_owners: usize) -> (Vec<SegmentData>, NativeSegmentEvidence) {
    let page = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let segments = page.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(segments, page, &[]);
    let evidence = evidence.expect("actual native source carrier");
    assert_eq!(evidence.owners.len(), expected_owners, "unique measured source owners");
    for owner in &evidence.owners {
        let base = &segments[owner.base_index];
        let script = &segments[owner.script_index];
        assert_eq!(base.font_size, 12.0);
        assert_eq!(script.font_size, 8.0);
        assert!((script.baseline_y - base.baseline_y - 4.9568).abs() < 0.0001);
        assert!(script.text.ends_with(' '), "preserve source script whitespace");
    }
    (segments, evidence)
}

fn project(paragraph: &PdfParagraph) -> (String, Vec<String>) {
    let (text, annotations) = extract_text_and_annotations(paragraph, &Default::default());
    let powers = annotations
        .iter()
        .filter(|annotation| annotation.kind == AnnotationKind::Superscript)
        .map(|annotation| text[annotation.start as usize..annotation.end as usize].to_owned())
        .collect();
    (text, powers)
}

fn group(segments: Vec<SegmentData>, evidence: NativeSegmentEvidence) -> Vec<PdfParagraph> {
    let mut before: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    let paragraphs = super::super::segments_to_paragraphs_with_native_scripts(
        segments,
        &[],
        &[],
        &Default::default(),
        Some(evidence),
    );
    let mut after: Vec<_> = paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .flat_map(|line| &line.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    before.sort();
    after.sort();
    assert_eq!(
        after, before,
        "every source glyph projection survives grouping unchanged"
    );
    paragraphs
}

fn separate_rows(script_first: bool) {
    let mut spans = Vec::new();
    for (prefix, power, baseline) in [("First expression ", "4 ", 700.0), ("Next expression ", "3 ", 670.0)] {
        let mut row = vec![
            span(prefix, 80.0, baseline, 80.0, 12.0),
            span("x", 160.0, baseline, 10.0, 12.0),
            span(".", 174.0, baseline, 4.0, 12.0),
        ];
        let script = span(power, 170.0, baseline + 4.9568, 4.0, 8.0);
        row.insert(if script_first { 0 } else { 2 }, script);
        spans.extend(row);
    }
    let (segments, evidence) = evidence(spans, 2);
    let legacy = super::super::segments_to_paragraphs(segments.clone(), &[], &[], &Default::default());
    for prefix in ["First expression", "Next expression"] {
        let matching: Vec<_> = legacy
            .iter()
            .map(project)
            .filter(|(text, _)| text.contains(prefix))
            .collect();
        assert_eq!(matching.len(), 1, "legacy ordinary row exists once");
        assert!(
            !matching[0].0.contains(if prefix == "First expression" {
                "Next expression"
            } else {
                "First expression"
            }),
            "legacy groups must actually distinguish the two ordinary rows"
        );
    }
    let paragraphs = group(segments, evidence);
    assert_eq!(
        paragraphs.len(),
        2,
        "attaching powers must not merge separate ordinary row groups"
    );
    for (paragraph, (prefix, expected)) in paragraphs
        .iter()
        .zip([("First expression", "4"), ("Next expression", "3")])
    {
        let (text, powers) = project(paragraph);
        assert!(text.starts_with(prefix));
        assert_eq!(powers, [expected], "each row receives only its own source power");
        assert_eq!(paragraph.lines.len(), 1, "raised glyph remains on its owner line");
    }
}

#[test]
fn should_preserve_ordinary_groups_when_two_source_rows_have_script_first_powers_2275() {
    separate_rows(true);
}

#[test]
fn should_preserve_ordinary_groups_when_two_source_rows_have_base_first_powers_2275() {
    separate_rows(false);
}

#[test]
fn should_keep_multiple_source_powers_and_ordinary_operators_in_one_polynomial_row_2275() {
    for script_first in [false, true] {
        let first = span("4 ", 110.0, 704.9568, 4.0, 8.0);
        let second = span("3 ", 134.0, 704.9568, 4.0, 8.0);
        let mut spans = vec![
            span("p = ", 80.0, 700.0, 20.0, 12.0),
            span("x", 100.0, 700.0, 10.0, 12.0),
            span("+", 114.0, 700.0, 10.0, 12.0),
            span("y", 124.0, 700.0, 10.0, 12.0),
            span("+ 1.", 138.0, 700.0, 20.0, 12.0),
        ];
        if script_first {
            spans.splice(0..0, [first, second]);
        } else {
            spans.insert(2, first);
            spans.insert(5, second);
        }
        let (segments, evidence) = evidence(spans, 2);
        let paragraphs = group(segments, evidence);
        assert_eq!(
            paragraphs.len(),
            1,
            "font-change barriers inside one row are not paragraph boundaries"
        );
        assert_eq!(paragraphs[0].lines.len(), 1);
        let (text, powers) = project(&paragraphs[0]);
        assert!(text.starts_with("p = x"));
        assert!(text.contains("+ 1."));
        assert_eq!(powers, ["4", "3"]);
    }
}

#[test]
fn should_keep_a_proven_power_and_ordinary_wrapped_line_in_the_same_paragraph_2275() {
    for script_first in [false, true] {
        let mut spans = vec![
            span("The expression ", 80.0, 700.0, 80.0, 12.0),
            span("x", 160.0, 700.0, 10.0, 12.0),
            span("continues", 174.0, 700.0, 60.0, 12.0),
            span("through another physical line.", 80.0, 686.0, 180.0, 12.0),
        ];
        spans.insert(if script_first { 0 } else { 2 }, span("4 ", 170.0, 704.9568, 4.0, 8.0));
        let (segments, evidence) = evidence(spans, 1);
        let legacy = super::super::segments_to_paragraphs(segments.clone(), &[], &[], &Default::default());
        let legacy_body: Vec<_> = legacy
            .iter()
            .map(project)
            .filter(|(text, _)| text.contains("continues"))
            .collect();
        assert_eq!(legacy_body.len(), 1);
        assert!(
            legacy_body[0].0.contains("through another physical line."),
            "legacy really permits this wrap"
        );
        let paragraphs = group(segments, evidence);
        assert_eq!(
            paragraphs.len(),
            1,
            "a source script is not a mandatory paragraph break"
        );
        assert_eq!(paragraphs[0].lines.len(), 2, "retain the actual two physical baselines");
        let (text, powers) = project(&paragraphs[0]);
        assert!(text.contains("The expression"));
        assert!(text.contains("through another physical line."));
        assert_eq!(powers, ["4"]);
    }
}

#[test]
fn should_preserve_unrelated_same_row_semantic_groups_on_a_page_with_a_native_power_2275() {
    let mut heading = span("Separate heading", 80.0, 670.0, 90.0, 12.0);
    heading.mcid = Some(7);
    let page = NativePageSegments::from_final_spans(
        vec![
            span("x", 100.0, 700.0, 10.0, 12.0),
            span("4 ", 110.0, 704.9568, 4.0, 8.0),
            span(".", 114.0, 700.0, 4.0, 12.0),
            heading,
            span("Independent body.", 180.0, 670.0, 100.0, 12.0),
        ],
        &std::collections::HashMap::from([(7, Some(1))]),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let segments = page.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(segments, page, &[]);
    let evidence = evidence.expect("source carrier");
    assert_eq!(
        evidence.owners.len(),
        1,
        "the page must actually carry accepted source proof"
    );
    let refs: Vec<_> = segments.iter().collect();
    assert_eq!(accepted_owners(&refs, &evidence).len(), 1);
    let legacy = super::super::segments_to_paragraphs(segments.clone(), &[], &[], &Default::default());
    for paragraphs in [&legacy, &group(segments, evidence)] {
        let heading: Vec<_> = paragraphs
            .iter()
            .filter(|paragraph| project(paragraph).0.contains("Separate heading"))
            .collect();
        let body: Vec<_> = paragraphs
            .iter()
            .filter(|paragraph| project(paragraph).0.contains("Independent body."))
            .collect();
        assert_eq!(heading.len(), 1);
        assert_eq!(body.len(), 1);
        assert!(!project(heading[0]).0.contains("Independent body."));
        assert!(!project(body[0]).0.contains("Separate heading"));
        assert_eq!(heading[0].heading_level, Some(1));
        assert_eq!(body[0].heading_level, None);
    }
}
