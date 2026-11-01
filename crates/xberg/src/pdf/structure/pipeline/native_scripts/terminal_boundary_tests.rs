use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::assembly::extract_text_and_annotations;
use crate::pdf::structure::types::{PdfLine, PdfParagraph};
use crate::types::document_structure::AnnotationKind;
use sha2::{Digest, Sha256};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

const FONT: f32 = 12.0;
const OPENING_FONT: f32 = 13.6;
const OPENING: &str = "Fresh source prose explains the independent value";
const CONTINUATION: &str = "and the same source account continues with";

#[derive(Clone, Copy)]
enum Shape {
    Compact,
    Raised,
    Lowercase,
    Nonoverlap,
    LaterTail,
}

#[derive(Clone, Copy)]
enum Damage {
    None,
    UnobservedText,
    FilteredMember,
}

struct Case {
    segments: Vec<SegmentData>,
    evidence: NativeSegmentEvidence,
    gaps: Vec<f32>,
    opening: &'static str,
    raised: bool,
}

fn span(text: &str, x: f32, y: f32, width: f32, font: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font),
        font_size: font,
        font_name: "terminal-source-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn source_spans(shape: Shape) -> (Vec<TextSpan>, &'static str, bool) {
    let opening = if matches!(shape, Shape::Lowercase) {
        CONTINUATION
    } else {
        OPENING
    };
    let raised = matches!(shape, Shape::Raised);
    let tail_y = if matches!(shape, Shape::Nonoverlap) {
        680.8
    } else {
        691.8
    };
    let opening_y = tail_y - if raised { 21.612 } else { 11.012 };
    let tail_x = if matches!(shape, Shape::LaterTail) {
        220.0
    } else {
        132.0
    };
    let mut spans = vec![
        span("x", 80.0, 800.0, 10.0, FONT),
        span("4 ", 90.0, 804.9568, 4.0, 8.0),
        span(".", 94.0, 800.0, 4.0, FONT),
        span("a", 80.0, 750.0, 8.0, FONT),
        span("b", 80.0, 742.0, 8.0, FONT),
        span("=", 150.0, 700.0, 10.0, FONT),
        span("uv", 160.0, 700.0, 40.0, FONT),
        span(".", 210.0, 700.0, 3.0, FONT),
        span("dx", tail_x, tail_y, 12.0, FONT),
        span(opening, 80.0, opening_y, 180.0, OPENING_FONT),
        span("x", 260.0, opening_y, 10.0, OPENING_FONT),
    ];
    if raised {
        spans.push(span("8", 270.0, opening_y + 4.338, 4.0, 8.0));
    }
    (spans, opening, raised)
}

fn fixture(shape: Shape, damage: Damage) -> Case {
    let (spans, opening, raised) = source_spans(shape);
    let count = spans.len();
    let page = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let tables = if matches!(damage, Damage::FilteredMember) {
        vec![TableCoverage::from_rows(
            crate::types::BoundingBox {
                x0: 160.0,
                y0: 700.0,
                x1: 200.0,
                y1: 712.0,
            },
            [vec!["uv"]],
        )]
    } else {
        Vec::new()
    };
    let (mut segments, evidence) = prepare_segments(page.segments().cloned().collect(), page, &tables);
    let evidence = evidence.expect("actual source-ordinal native carrier");
    assert!(evidence.origin.is_some());
    assert_eq!(evidence.owners.len(), 1 + usize::from(raised));
    assert_eq!(
        segments.len(),
        count - usize::from(matches!(damage, Damage::FilteredMember))
    );
    if matches!(damage, Damage::UnobservedText) {
        let period = segments.iter_mut().find(|s| s.text == "." && s.x == 210.0).unwrap();
        period.text = "!".to_owned();
    }
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    Case {
        segments,
        evidence,
        gaps,
        opening,
        raised,
    }
}

fn ordered(case: &Case) -> (Vec<SegmentData>, NativeScriptIndex, Vec<PdfParagraph>) {
    let evidence = NativeSegmentEvidence {
        origin: case.evidence.origin,
        original_indices: case.evidence.original_indices.clone(),
        owners: case.evidence.owners.clone(),
    };
    let (segments, index) = order_native_paragraph_segments(case.segments.clone(), evidence, &case.gaps);
    assert!(index.continuation_gaps.is_some());
    assert_eq!(index.original_indices.len(), segments.len());
    assert_eq!(
        index
            .original_indices
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        segments.len()
    );
    let grouped =
        super::super::blocks_to_paragraphs_with_native_scripts(segments.clone(), &[], &case.gaps, Some(&index));
    (segments, index, grouped)
}

fn paragraph_for(paragraphs: &[PdfParagraph], anchor: &str) -> usize {
    let matches: Vec<_> = paragraphs
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            p.lines
                .iter()
                .flat_map(|line| &line.segments)
                .any(|segment| segment.text == anchor)
        })
        .map(|(index, _)| index)
        .collect();
    assert_eq!(matches.len(), 1, "one actual source context {anchor:?}");
    matches[0]
}

fn target_line<'a>(paragraphs: &'a [PdfParagraph], anchor: &str) -> &'a PdfLine {
    let matches: Vec<_> = paragraphs
        .iter()
        .flat_map(|p| &p.lines)
        .filter(|line| line.segments.iter().any(|segment| segment.text == anchor))
        .collect();
    assert_eq!(matches.len(), 1, "unique actual line for {anchor:?}");
    matches[0]
}

fn assert_exact_line(store: &source_projection::ProjectionStore, grouped: &[PdfParagraph], line: &PdfLine) {
    let view = store.view(grouped);
    let scripts: std::collections::HashSet<_> = line.valid_native_scripts().iter().map(|s| s.script_index).collect();
    let mut observed = 0;
    for (_, segment) in line.segments.iter().enumerate().filter(|(i, _)| !scripts.contains(i)) {
        let slot = view
            .resolve(segment)
            .expect("unique complete immutable segment projection");
        let original = &view.records()[slot];
        assert_eq!(view.original_slot(original.original_index), Some(slot));
        assert!(
            crate::pdf::native::hierarchy::script_ownership::page_segments::SegmentProjection::from(segment)
                == crate::pdf::native::hierarchy::script_ownership::page_segments::SegmentProjection::from(
                    &original.segment
                )
        );
        observed += 1;
    }
    assert!(observed > 0);
}

fn assert_geometry(shape: Shape, main: &PdfLine, tail: &PdfLine, lower: &PdfLine) {
    let punctuation = main.segments.last().unwrap();
    assert!(matches!(punctuation.text.as_str(), "." | "!"));
    assert_eq!(tail.segments.len(), 1);
    let fragment = &tail.segments[0];
    assert!(main.baseline_y > tail.baseline_y && tail.baseline_y > lower.baseline_y);
    assert_eq!(
        fragment.y + fragment.height > punctuation.y,
        !matches!(shape, Shape::Nonoverlap)
    );
    assert_eq!(
        fragment.upright_advance_extent().1 < punctuation.upright_advance_extent().0,
        !matches!(shape, Shape::LaterTail)
    );
    assert_eq!(
        lower.segments[0].text.chars().next().unwrap().is_lowercase(),
        matches!(shape, Shape::Lowercase)
    );
    assert!(
        OPENING_FONT - FONT > 1.5 && OPENING_FONT - FONT < 2.0,
        "existing initial font split and generic continuation tolerances both apply"
    );
}

fn assert_boundary_preconditions(case: &Case, shape: Shape, damage: Damage) {
    let (segments, index, grouped) = ordered(case);
    let store = source_projection::ProjectionStore::new(&segments, &index.original_indices, &index.by_script).unwrap();
    let rows = source_rows::SourceRows::new(&store, &grouped).expect("actual complete source-row index");
    let (main, tail, lower) = (
        target_line(&grouped, "="),
        target_line(&grouped, "dx"),
        target_line(&grouped, case.opening),
    );
    assert_geometry(shape, main, tail, lower);
    assert_eq!(rows.complete_row(main).is_some(), matches!(damage, Damage::None));
    for line in [tail, lower] {
        assert!(rows.complete_row(line).is_some());
        assert_exact_line(&store, &grouped, line);
    }
    if matches!(damage, Damage::None) {
        assert_exact_line(&store, &grouped, main);
        assert_eq!(rows.complete_row(tail).unwrap(), rows.complete_row(main).unwrap() + 1);
    }
    assert!(!super::super::paragraphs_cross_source_gap(
        &grouped[paragraph_for(&grouped, "dx")],
        &grouped[paragraph_for(&grouped, case.opening)],
        &case.gaps
    ));
    assert!(
        !index
            .continuation_gaps
            .as_ref()
            .unwrap()
            .crosses_rows(&tail.segments[0], &lower.segments[0])
    );
    let (previous, next) = (paragraph_for(&grouped, "dx"), paragraph_for(&grouped, case.opening));
    assert_ne!(previous, next, "real font grouping distinguishes the existing boundary");
    let mut generic = vec![grouped[previous].clone(), grouped[next].clone()];
    crate::pdf::structure::paragraphs::merge_continuation_paragraphs(&mut generic);
    assert_eq!(
        generic.len(),
        1,
        "existing generic merger reads the nonterminal lower fragment"
    );
}

fn assert_preserved(case: &Case, output: &[PdfParagraph]) {
    let mut before: Vec<_> = case.segments.iter().map(super::line_tests::fingerprint).collect();
    let mut after: Vec<_> = output
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|l| &l.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    before.sort();
    after.sort();
    assert_eq!(after, before, "all current glyph fields survive exactly once");
    let powers: Vec<_> = output
        .iter()
        .flat_map(|paragraph| {
            let (text, annotations) = extract_text_and_annotations(paragraph, &Default::default());
            annotations
                .into_iter()
                .filter(|a| a.kind == AnnotationKind::Superscript)
                .map(|a| text[a.start as usize..a.end as usize].to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(powers, if case.raised { vec!["4", "8"] } else { vec!["4"] });
}

fn output(case: &Case) -> Vec<PdfParagraph> {
    let evidence = NativeSegmentEvidence {
        origin: case.evidence.origin,
        original_indices: case.evidence.original_indices.clone(),
        owners: case.evidence.owners.clone(),
    };
    super::super::segments_to_paragraphs_with_native_scripts(
        case.segments.clone(),
        &[],
        &case.gaps,
        &Default::default(),
        Some(evidence),
    )
}

fn assert_real_opening_powers(case: &Case) {
    let paragraphs = output(case);
    let matches: Vec<_> = paragraphs
        .iter()
        .map(|paragraph| extract_text_and_annotations(paragraph, &Default::default()))
        .filter(|(text, _)| text.contains("and subtracting"))
        .collect();
    assert_eq!(matches.len(), 1, "unique actual source opening context");
    let (text, annotations) = &matches[0];
    assert!(text.contains("exactly as we supposed."));
    let powers: Vec<_> = annotations
        .iter()
        .filter(|a| a.kind == AnnotationKind::Superscript)
        .map(|a| {
            text.get(a.start as usize..a.end as usize)
                .expect("actual UTF8 native power range")
        })
        .collect();
    assert_eq!(
        powers,
        ["5", "4", "4"],
        "all three actual source opening powers preserved"
    );
}

fn assert_result(shape: Shape, damage: Damage, together: bool) {
    let case = fixture(shape, damage);
    assert_boundary_preconditions(&case, shape, damage);
    let output = output(&case);
    assert_preserved(&case, &output);
    assert_eq!(
        paragraph_for(&output, "dx") == paragraph_for(&output, case.opening),
        together,
        "source logical terminal punctuation must survive an overlapping lower-left fragment"
    );
}

#[test]
fn should_preserve_a_terminal_period_before_compact_capitalized_prose_2275() {
    assert_result(Shape::Compact, Damage::None, false);
}

#[test]
fn should_preserve_a_terminal_period_when_a_raised_power_masks_the_raw_gap_2275() {
    assert_result(Shape::Raised, Damage::None, false);
}

#[test]
fn should_keep_the_existing_lowercase_continuation_after_a_terminal_period_2275() {
    assert_result(Shape::Lowercase, Damage::None, true);
}

#[test]
fn should_not_ignore_a_nonoverlapping_ordinary_last_row_2275() {
    assert_result(Shape::Nonoverlap, Damage::None, true);
}

#[test]
fn should_not_ignore_a_fragment_right_of_the_terminal_punctuation_2275() {
    assert_result(Shape::LaterTail, Damage::None, true);
}

#[test]
fn should_abstain_from_terminal_reinterpretation_without_exact_complete_source_rows_2275() {
    for damage in [Damage::UnobservedText, Damage::FilteredMember] {
        assert_result(Shape::Compact, damage, true);
    }
}

fn real_source() -> Case {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/pdf/regression_pages/gutenberg-33283-p33.pdf");
    let bytes = std::fs::read(path).expect("mandatory public source page");
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "f0accfac5c9d9b1f819d1f475d947c59aeda0366bf9e98c9e90653720c75eaa3"
    );
    let mut document = crate::pdf::native::NativeDocument::open_bytes(&bytes).unwrap();
    let (mut pages, used_tree) =
        crate::pdf::native::hierarchy::script_ownership::page_segments::extract_all_segment_carriers(
            &mut document,
            ArtifactRetention::default(),
        )
        .unwrap();
    assert!(!used_tree);
    assert_eq!(pages.len(), 1);
    let page = pages.pop().unwrap();
    let (segments, evidence) = prepare_segments(page.segments().cloned().collect(), page, &[]);
    let evidence = evidence.expect("actual source immutable native carrier");
    assert!(evidence.origin.is_some() && !evidence.owners.is_empty());
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    Case {
        segments,
        evidence,
        gaps,
        opening: "For example, let",
        raised: true,
    }
}

#[test]
fn should_prove_real_source_main_tail_and_opening_rows_before_terminal_policy_2275() {
    let case = real_source();
    assert_real_opening_powers(&case);
    let (segments, index, grouped) = ordered(&case);
    let store = source_projection::ProjectionStore::new(&segments, &index.original_indices, &index.by_script).unwrap();
    let rows = source_rows::SourceRows::new(&store, &grouped).expect("real source-row index");
    for opening in ["For example, let", "And, indeed, the rule that differentiating"] {
        let next = paragraph_for(&grouped, opening);
        assert!(next > 0);
        let previous = &grouped[next - 1];
        assert!(previous.lines.len() >= 2);
        let main = &previous.lines[previous.lines.len() - 2];
        let tail = previous.lines.last().unwrap();
        let lower = target_line(&grouped, opening);
        assert_eq!(tail.segments.len(), 1);
        assert_eq!(tail.segments[0].text, "dx");
        assert!(!super::super::paragraphs_cross_source_gap(
            previous,
            &grouped[next],
            &case.gaps
        ));
        for line in [main, tail, lower] {
            rows.complete_row(line)
                .expect("mandatory complete REAL source row; never weaken this proof");
            assert_exact_line(&store, &grouped, line);
        }
        assert_eq!(rows.complete_row(tail).unwrap(), rows.complete_row(main).unwrap() + 1);
        let period = main.segments.last().unwrap();
        assert_eq!(period.text, ".");
        assert!(
            tail.segments
                .iter()
                .all(|s| s.upright_advance_extent().1 < period.upright_advance_extent().0)
        );
        assert!(tail.segments.iter().all(|s| s.y + s.height > period.y));
    }
}
