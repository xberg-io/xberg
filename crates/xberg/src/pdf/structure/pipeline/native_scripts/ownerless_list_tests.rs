use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::assembly::{
    assemble_internal_document, extract_text_and_annotations, extract_text_with_source_ranges,
};
use crate::pdf::structure::types::PdfParagraph;
use crate::types::document_structure::AnnotationKind;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

type Fingerprint = (String, [u32; 7], [bool; 3], Option<u8>);

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_size,
        font_name: "source-list-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn spans(owned: bool, spaced: bool, outside: bool) -> Vec<TextSpan> {
    let (four, six, zero, hundredths, digit_width) = if spaced {
        ("4 ", " 6", "0 ", " 01", 10.0)
    } else {
        ("4", "6", "0", "01", 6.0)
    };
    let mut source = if owned {
        vec![
            span("é lists the source measurements ", 42.0, 700.0, 170.0, 12.0),
            span("x", 214.0, 700.0, 8.0, 12.0),
            span("2", 222.0, 704.9568, 4.0, 8.0),
            span(":", 226.0, 700.0, 3.0, 12.0),
        ]
    } else {
        vec![span("é lists the source measurements:", 42.0, 700.0, 187.0, 12.0)]
    };
    let dot = 309.0 + digit_width;
    let second_dot = 472.0 + digit_width;
    source.extend([
        span("read sample ", 237.0, 700.0, 72.0, 12.0),
        span(four, 309.0, 700.0, digit_width, 12.0),
        span(".", dot, 700.0, 3.0, 12.0),
        span(six, dot + 3.0, 700.0, digit_width, 12.0),
        span(" seconds;", dot + 3.0 + digit_width, 700.0, 52.0, 12.0),
        span("keep sample ", 400.0, 700.0, 72.0, 12.0),
        span(zero, 472.0, 700.0, digit_width, 12.0),
        span(".", second_dot, 700.0, 3.0, 12.0),
        span(hundredths, second_dot + 3.0, 700.0, digit_width + 6.0, 12.0),
        span(" second.", second_dot + 9.0 + digit_width, 700.0, 52.0, 12.0),
    ]);
    if outside {
        source.extend([
            span("The context retains a source power x", 42.0, 760.0, 210.0, 12.0),
            span("2", 252.0, 764.9568, 4.0, 8.0),
        ]);
    }
    source
}

fn fingerprints(paragraphs: &[PdfParagraph]) -> Vec<Fingerprint> {
    let mut records: Vec<_> = paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .flat_map(|line| &line.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    records.sort();
    records
}

fn owner_count(paragraphs: &[PdfParagraph]) -> usize {
    paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .map(|line| line.valid_native_scripts().len())
        .sum()
}

fn source_paragraphs(owned: bool, spaced: bool, outside: bool) -> Vec<PdfParagraph> {
    let page = NativePageSegments::from_final_spans(
        spans(owned, spaced, outside),
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let projected = page.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(projected, page, &[]);
    let evidence = evidence.expect("actual page-local source carrier");
    assert_eq!(evidence.owners.len(), usize::from(owned) + usize::from(outside));
    assert_eq!(segments.len(), 11 + 3 * usize::from(owned) + 2 * usize::from(outside));
    let mut original: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    original.sort();
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    let paragraphs = super::super::segments_to_paragraphs_with_native_scripts(
        segments,
        &[],
        &gaps,
        &Default::default(),
        Some(evidence),
    );
    assert_eq!(paragraphs.len(), 1 + usize::from(outside));
    assert_eq!(
        fingerprints(&paragraphs),
        original,
        "unchanged native source before list planning"
    );
    assert_eq!(owner_count(&paragraphs), usize::from(owned) + usize::from(outside));
    paragraphs
}

fn document(paragraphs: &[PdfParagraph]) -> InternalDocument {
    assemble_internal_document(vec![paragraphs.to_vec()], &[], None, &[], &Default::default())
}

fn canonical_source(paragraphs: &[PdfParagraph], owned: bool, spaced: bool) -> String {
    let selected: Vec<_> = paragraphs
        .iter()
        .filter(|paragraph| {
            paragraph
                .lines
                .iter()
                .any(|line| line.segments.iter().any(|s| s.text.starts_with('é')))
        })
        .collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(owner_count(std::slice::from_ref(selected[0])), usize::from(owned));
    let witnesses = Default::default();
    let canonical = extract_text_and_annotations(selected[0], &witnesses);
    let ledger = extract_text_with_source_ranges(selected[0], &witnesses);
    assert_eq!((&ledger.text, &ledger.annotations), (&canonical.0, &canonical.1));
    assert_eq!(ledger.segment_ranges.len(), 11 + 3 * usize::from(owned));
    assert!(
        ledger
            .segment_ranges
            .iter()
            .all(|range| ledger.text.get(range.clone()).is_some())
    );
    let first = &ledger.segment_ranges[0];
    assert_eq!(first.start, 0);
    let prefix = ledger.text.get(first.clone()).expect("real UTF8 source byte range");
    assert!(prefix.starts_with('é') && first.end > prefix.chars().count());
    let expected = if spaced {
        "read sample 4 . 6 seconds; keep sample 0 . 01 second."
    } else {
        "read sample 4.6 seconds; keep sample 0.01 second."
    };
    assert_eq!(canonical.0.split_once(": ").expect("real source lead").1, expected);
    let powers: Vec<_> = canonical
        .1
        .iter()
        .filter(|a| a.kind == AnnotationKind::Superscript)
        .collect();
    assert_eq!(powers.len(), usize::from(owned));
    if let Some(power) = powers.first() {
        assert_eq!(canonical.0.get(power.start as usize..power.end as usize), Some("2"));
        assert_eq!(power.start as usize, "é lists the source measurements x".len());
    }
    canonical.0
}

fn assert_semantic_elements(document: &InternalDocument, outside: bool) -> Vec<&InternalElement> {
    let shape: Vec<_> = document
        .elements
        .iter()
        .map(|element| (&element.kind, element.text.as_str()))
        .collect();
    let empty: Vec<_> = document
        .elements
        .iter()
        .filter(|element| element.text.is_empty())
        .collect();
    assert_eq!(empty.len(), 2, "actual list container shape: {shape:?}");
    assert!(
        empty
            .iter()
            .all(|element| matches!(element.kind, ElementKind::ListStart { .. } | ElementKind::ListEnd)),
        "unexpected empty source element: {shape:?}"
    );
    assert_eq!(
        document
            .elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::ListStart { .. }))
            .count(),
        1,
        "actual source list start: {shape:?}"
    );
    assert_eq!(
        document
            .elements
            .iter()
            .filter(|element| element.kind == ElementKind::ListEnd)
            .count(),
        1,
        "actual source list end: {shape:?}"
    );
    let semantic: Vec<_> = document
        .elements
        .iter()
        .filter(|element| !element.text.is_empty())
        .collect();
    assert_eq!(
        semantic.len(),
        3 + usize::from(outside),
        "lead, two clauses, and optional separate context: {shape:?}"
    );
    if outside {
        let context: Vec<_> = semantic
            .iter()
            .filter(|element| element.text.starts_with("The context"))
            .collect();
        assert_eq!(context.len(), 1, "actual distinct source context: {shape:?}");
        assert_eq!(context[0].text, "The context retains a source power x2");
    }
    semantic
}

fn assert_split_text(document: &InternalDocument, canonical: &str, spaced: bool, outside: bool) {
    let semantic = assert_semantic_elements(document, outside);
    let items: Vec<_> = semantic
        .iter()
        .filter(|element| matches!(element.kind, ElementKind::ListItem { .. }))
        .collect();
    assert_eq!(items.len(), 2, "actual two source clauses rendered");
    assert_eq!(
        items[0].text,
        if spaced {
            "read sample 4 . 6 seconds;"
        } else {
            "read sample 4.6 seconds;"
        }
    );
    assert_eq!(
        items[1].text,
        if spaced {
            "keep sample 0 . 01 second."
        } else {
            "keep sample 0.01 second."
        }
    );
    let lead: Vec<_> = document
        .elements
        .iter()
        .filter(|element| element.text.starts_with('é'))
        .collect();
    assert_eq!(lead.len(), 1);
    assert_eq!(lead[0].text, format!("{}:", canonical.split_once(':').unwrap().0));
}

fn assert_powers(document: &InternalDocument, expected: usize) {
    let powers: Vec<_> = document
        .elements
        .iter()
        .flat_map(|element| {
            element
                .annotations
                .iter()
                .filter(|annotation| annotation.kind == AnnotationKind::Superscript)
                .map(move |annotation| (element, annotation))
        })
        .collect();
    assert_eq!(powers.len(), expected);
    for (element, power) in powers {
        assert_eq!(element.text.get(power.start as usize..power.end as usize), Some("2"));
        if element.text.starts_with('é') {
            assert_eq!(power.start as usize, "é lists the source measurements x".len());
        }
    }
}

fn exercise(owned: bool, spaced: bool, outside: bool) {
    let paragraphs = source_paragraphs(owned, spaced, outside);
    let canonical = canonical_source(&paragraphs, owned, spaced);
    let original = fingerprints(&paragraphs);
    let mut pages = vec![paragraphs];
    let witnesses = Default::default();
    super::super::run_in_lists::split_with_witnesses(&mut pages, &witnesses);
    let actual = document(&pages[0]);
    assert_split_text(&actual, &canonical, spaced, outside);
    assert_eq!(
        fingerprints(&pages[0]),
        original,
        "every source record survives exactly once with all fields"
    );
    assert_eq!(owner_count(&pages[0]), usize::from(owned) + usize::from(outside));
    assert_powers(&actual, usize::from(owned) + usize::from(outside));
    let rendered: Vec<_> = actual
        .elements
        .iter()
        .map(|element| (element.text.clone(), element.annotations.clone()))
        .collect();
    super::super::run_in_lists::split_with_witnesses(&mut pages, &witnesses);
    assert_eq!(fingerprints(&pages[0]), original);
    assert_eq!(
        document(&pages[0])
            .elements
            .into_iter()
            .map(|element| (element.text, element.annotations))
            .collect::<Vec<_>>(),
        rendered,
        "source projection is idempotent"
    );
}

#[test]
fn should_keep_touching_decimals_and_utf8_in_ownerless_source_lists_2275() {
    exercise(false, false, false);
}

#[test]
fn should_preserve_real_spaces_and_records_in_ownerless_numeric_lists_2275() {
    exercise(false, true, false);
}

#[test]
fn should_match_canonical_list_projection_with_an_actual_utf8_native_power_2275() {
    exercise(true, false, false);
}

#[test]
fn should_project_ownerless_list_when_its_page_has_an_owner_elsewhere_2275() {
    exercise(false, false, true);
}
