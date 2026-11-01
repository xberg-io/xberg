use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::assembly::extract_text_and_annotations;
use crate::pdf::structure::types::PdfParagraph;
use crate::types::document_structure::AnnotationKind;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

const OPENING: &str = "New source account begins here";

fn source_spans(tall: bool) -> Vec<TextSpan> {
    let tail_y = if tall { 680.8 } else { 691.8 };
    let opening_y = if tall { 663.05 } else { 680.788 };
    [
        ("x", 80.0, 800.0, 10.0, 12.0),
        ("4 ", 90.0, 804.9568, 4.0, 8.0),
        (".", 94.0, 800.0, 4.0, 12.0),
        ("a", 80.0, 750.0, 8.0, 12.0),
        ("b", 80.0, 742.0, 8.0, 12.0),
        ("=", 150.0, 700.0, 10.0, 12.0),
        ("x", 190.0, 700.0, 10.0, 12.0),
        ("n−1", 200.0, 704.9568, 15.0, 8.0),
        (".", 210.0, 700.0, 3.0, 12.0),
        ("dx", 132.0, tail_y, 12.0, 12.0),
        (OPENING, 80.0, opening_y, 180.0, 13.6),
    ]
    .into_iter()
    .filter(|(text, ..)| !tall || *text != "n−1")
    .map(|(text, x, y, width, font)| TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, if tall && text == "dx" { 35.0 } else { font }),
        font_size: font,
        font_name: "terminal-safety-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    })
    .collect()
}

fn fixture(tall: bool) -> (Vec<SegmentData>, NativeSegmentEvidence) {
    let page = NativePageSegments::from_final_spans(
        source_spans(tall),
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let (segments, evidence) = prepare_segments(page.segments().cloned().collect(), page, &[]);
    let evidence = evidence.expect("real original page projection");
    assert!(evidence.origin.is_some());
    assert_eq!(segments.len(), if tall { 10 } else { 11 });
    assert_eq!(evidence.owners.len(), if tall { 1 } else { 2 });
    (segments, evidence)
}

fn paragraph_for(paragraphs: &[PdfParagraph], text: &str) -> usize {
    let matches: Vec<_> = paragraphs
        .iter()
        .enumerate()
        .filter(|(_, p)| p.lines.iter().flat_map(|l| &l.segments).any(|s| s.text == text))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}

fn assert_preconditions(segments: &[SegmentData], evidence: &NativeSegmentEvidence, gaps: &[f32], tall: bool) {
    let copy = NativeSegmentEvidence {
        origin: evidence.origin,
        original_indices: evidence.original_indices.clone(),
        owners: evidence.owners.clone(),
    };
    let (ordered, index) = order_native_paragraph_segments(segments.to_vec(), copy, gaps);
    let grouped = super::super::blocks_to_paragraphs_with_native_scripts(ordered.clone(), &[], gaps, Some(&index));
    let store = source_projection::ProjectionStore::new(&ordered, &index.original_indices, &index.by_script).unwrap();
    let rows = source_rows::SourceRows::new(&store, &grouped).unwrap();
    assert!(
        super::super::page_column_corridor_for_segments(
            ordered
                .iter()
                .enumerate()
                .filter(|(i, _)| !index.by_script.contains_key(i))
                .map(|(_, segment)| segment)
        )
        .is_none()
    );
    let previous = paragraph_for(&grouped, "dx");
    let next = paragraph_for(&grouped, OPENING);
    assert_ne!(previous, next);
    assert!(!index.continuation_gaps.as_ref().unwrap().crosses_rows(
        grouped[previous].lines.last().unwrap().segments.last().unwrap(),
        grouped[next].lines.first().unwrap().segments.first().unwrap()
    ));
    let main = &grouped[previous].lines[grouped[previous].lines.len() - 2];
    let tail = grouped[previous].lines.last().unwrap();
    for line in [main, tail, grouped[next].lines.first().unwrap()] {
        assert!(rows.complete_row(line).is_some());
    }
    assert_shape(main, tail, tall);
    assert!(!super::super::paragraphs_cross_source_gap(
        &grouped[previous],
        &grouped[next],
        gaps
    ));
    let mut generic = vec![grouped[previous].clone(), grouped[next].clone()];
    crate::pdf::structure::paragraphs::merge_continuation_paragraphs(&mut generic);
    assert_eq!(generic.len(), 1);
}

fn assert_shape(
    main: &crate::pdf::structure::types::PdfLine,
    tail: &crate::pdf::structure::types::PdfLine,
    tall: bool,
) {
    let terminal = main.segments.last().unwrap();
    assert_eq!(terminal.text, ".");
    assert_eq!(main.valid_native_scripts().len(), usize::from(!tall));
    if tall {
        assert_eq!(tail.segments.len(), 1);
        let fragment = &tail.segments[0];
        assert_eq!(fragment.height, 35.0);
        assert_eq!(fragment.font_size, 12.0);
        assert!(fragment.y + fragment.height > terminal.y);
        assert!(main.baseline_y - tail.baseline_y >= terminal.font_size.min(fragment.font_size));
        assert!(fragment.upright_advance_extent().1 < terminal.upright_advance_extent().0);
    } else {
        let script = main.segments.iter().find(|s| s.text == "n−1").unwrap();
        assert!(script.upright_advance_extent().1 > terminal.upright_advance_extent().1);
    }
}

#[test]
fn should_count_valid_raised_content_right_of_a_purported_terminal_period_2275() {
    let (segments, evidence) = fixture(false);
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    assert_preconditions(&segments, &evidence, &gaps, false);
    let mut before: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    let output = super::super::segments_to_paragraphs_with_native_scripts(
        segments,
        &[],
        &gaps,
        &Default::default(),
        Some(evidence),
    );
    let mut after: Vec<_> = output
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|l| &l.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    before.sort();
    after.sort();
    assert_eq!(after, before);
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
    assert_eq!(powers, ["4", "n−1"]);
    assert_eq!(
        paragraph_for(&output, "dx"),
        paragraph_for(&output, OPENING),
        "a period with later raised content cannot independently veto the original join"
    );
}

#[test]
fn should_not_treat_tall_native_boxes_at_ordinary_spacing_as_a_lower_fragment_2275() {
    let (segments, evidence) = fixture(true);
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    assert_preconditions(&segments, &evidence, &gaps, true);
    let mut before: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    let output = super::super::segments_to_paragraphs_with_native_scripts(
        segments,
        &[],
        &gaps,
        &Default::default(),
        Some(evidence),
    );
    let mut after: Vec<_> = output
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|l| &l.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    before.sort();
    after.sort();
    assert_eq!(after, before);
    assert_eq!(
        paragraph_for(&output, "dx"),
        paragraph_for(&output, OPENING),
        "a tall box cannot reinterpret an ordinary-spaced source row as preceding terminal punctuation"
    );
}
