use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::assembly::assemble_internal_document;
use crate::pdf::structure::types::PdfParagraph;
use crate::types::document_structure::AnnotationKind;
use crate::types::internal::InternalDocument;

type Fingerprint = (String, [u32; 7], [bool; 3], Option<u8>);

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_size,
        font_name: "native-source-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn native_paragraphs(spans: Vec<TextSpan>, expected_owners: usize) -> (Vec<PdfParagraph>, Vec<Fingerprint>) {
    let page = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let projected = page.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(projected, page, &[]);
    let evidence = evidence.expect("actual source projection carrier");
    assert_eq!(evidence.owners.len(), expected_owners, "real complete source ownership");
    let mut before: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    before.sort();
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    let paragraphs = super::super::segments_to_paragraphs_with_native_scripts(
        segments,
        &[],
        &gaps,
        &Default::default(),
        Some(evidence),
    );
    (paragraphs, before)
}

fn owner_count(paragraphs: &[PdfParagraph]) -> usize {
    paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .map(|line| line.valid_native_scripts().len())
        .sum()
}

fn fingerprints(paragraphs: &[PdfParagraph]) -> Vec<Fingerprint> {
    let mut values: Vec<_> = paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .flat_map(|line| &line.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    values.sort();
    values
}

fn document(paragraphs: &[PdfParagraph]) -> InternalDocument {
    assemble_internal_document(vec![paragraphs.to_vec()], &[], None, &[], &Default::default())
}

fn assert_power(doc: &InternalDocument, anchor: &str, expected: &str) {
    let elements: Vec<_> = doc
        .elements
        .iter()
        .filter(|element| element.text.contains(anchor))
        .collect();
    assert_eq!(elements.len(), 1, "one emitted source context {anchor:?}");
    let element = elements[0];
    let powers: Vec<_> = element
        .annotations
        .iter()
        .filter(|annotation| annotation.kind == AnnotationKind::Superscript)
        .collect();
    assert_eq!(powers.len(), 1, "retain the complete source power in {anchor:?}");
    let power = powers[0];
    assert_eq!(
        element.text.get(power.start as usize..power.end as usize),
        Some(expected),
        "exact source superscript byte range"
    );
}

#[test]
fn should_not_materialize_a_displayed_native_derivative_as_a_prose_list_2275() {
    let (paragraphs, _) = native_paragraphs(
        vec![
            span("a", 80.0, 750.0, 8.0, 12.0),
            span("b", 80.0, 742.0, 8.0, 12.0),
            span(
                "The differential coefficients use the follow-",
                42.0,
                700.0,
                300.0,
                12.0,
            ),
            span("ing way of reading expressions:", 42.0, 680.8, 300.0, 12.0),
            span("d", 78.0, 661.6, 8.0, 12.0),
            span("2", 86.0, 666.5568, 4.0, 8.0),
            span("y", 90.0, 661.6, 8.0, 12.0),
            span("; which is read in the ordinary way.", 98.0, 661.6, 210.0, 12.0),
        ],
        1,
    );
    let mut selected: Vec<_> = paragraphs
        .into_iter()
        .filter(|paragraph| super::super::paragraph_text_raw(paragraph).contains("The differential coefficients"))
        .collect();
    assert_eq!(
        selected.len(),
        1,
        "actual native continuation yields one source explanatory paragraph"
    );
    assert_eq!(owner_count(&selected), 1);
    let before = document(&selected);
    assert!(
        before
            .elements
            .iter()
            .any(|element| element.text.contains("the following way of reading expressions:"))
    );
    assert_power(&before, "The differential coefficients", "2");
    let mut pages = vec![std::mem::take(&mut selected)];
    super::super::split_colon_semicolon_run_in_lists(&mut pages);
    let after = document(&pages[0]);
    assert_eq!(
        pages[0].len(),
        1,
        "a displayed derivative and explanation are not a prose enumeration"
    );
    assert!(!pages[0][0].is_list_item);
    assert!(
        after
            .elements
            .iter()
            .any(|element| element.text.contains("the following way of reading expressions:"))
    );
    assert_power(&after, "The differential coefficients", "2");
    assert_eq!(owner_count(&pages[0]), 1);
}

fn genuine_list(with_power: bool) -> (Vec<PdfParagraph>, Vec<Fingerprint>) {
    let mut spans = vec![span("The source has a genuine list:", 42.0, 700.0, 300.0, 12.0)];
    if with_power {
        spans.extend([
            span("to retain the source power", 42.0, 680.8, 150.0, 12.0),
            span("x", 200.0, 680.8, 8.0, 12.0),
            span("1", 208.0, 685.7568, 4.0, 8.0),
            span("/", 212.0, 685.7568, 3.0, 8.0),
            span("3", 215.0, 685.7568, 4.0, 8.0),
            span(";", 219.0, 680.8, 4.0, 12.0),
        ]);
    } else {
        spans.push(span("to retain ordinary source text;", 42.0, 680.8, 250.0, 12.0));
    }
    spans.push(span("where the ordinary sentence continues.", 42.0, 661.6, 250.0, 12.0));
    native_paragraphs(spans, if with_power { 3 } else { 0 })
}

#[test]
fn should_split_genuine_prose_lists_without_losing_native_cohorts_2275() {
    for with_power in [false, true] {
        let (paragraphs, source) = genuine_list(with_power);
        assert_eq!(paragraphs.len(), 1, "one actual pre-split native source paragraph");
        assert_eq!(
            fingerprints(&paragraphs),
            source,
            "fixture retains original source glyph fields"
        );
        assert_eq!(owner_count(&paragraphs), if with_power { 3 } else { 0 });
        if with_power {
            assert_power(&document(&paragraphs), "The source has a genuine list:", "1/3");
        }
        let mut pages = vec![paragraphs];
        super::super::split_colon_semicolon_run_in_lists(&mut pages);
        let actual = document(&pages[0]);
        assert_eq!(
            pages[0].len(),
            3,
            "lead plus two real prose clauses, including ownerless compatibility"
        );
        assert!(!pages[0][0].is_list_item && pages[0][1].is_list_item && pages[0][2].is_list_item);
        if with_power {
            assert_power(&actual, "to retain the source power", "1/3");
            assert_eq!(owner_count(&pages[0]), 3, "the entire original source cohort survives");
            assert_eq!(
                fingerprints(&pages[0]),
                source,
                "no glyph fields changed, dropped or duplicated"
            );
        }
        let first_item = if with_power {
            "to retain the source power x1/3;"
        } else {
            "to retain ordinary source text;"
        };
        let text: Vec<_> = actual
            .elements
            .iter()
            .filter(|e| !e.text.is_empty())
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(
            text,
            [
                "The source has a genuine list:",
                first_item,
                "where the ordinary sentence continues."
            ]
        );
        super::super::split_colon_semicolon_run_in_lists(&mut pages);
        assert_eq!(pages[0].len(), 3, "run-in list splitting is idempotent");
        if with_power {
            assert_power(&document(&pages[0]), "to retain the source power", "1/3");
            assert_eq!(fingerprints(&pages[0]), source);
        }
    }
}
