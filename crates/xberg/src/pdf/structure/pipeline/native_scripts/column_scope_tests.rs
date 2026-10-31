use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::types::PdfParagraph;

use super::*;

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_size,
        font_name: "immutable-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn carrier(spans: Vec<TextSpan>) -> (Vec<SegmentData>, NativeSegmentEvidence) {
    let page = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let projected = page.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(projected, page, &[]);
    let evidence = evidence.expect("actual native source carrier");
    assert_eq!(
        evidence.owners.len(),
        1,
        "exactly one real accepted native source owner"
    );
    (segments, evidence)
}

fn paragraph_index(paragraphs: &[PdfParagraph], text: &str) -> usize {
    let indices: Vec<_> = paragraphs
        .iter()
        .enumerate()
        .filter_map(|(index, paragraph)| {
            paragraph
                .lines
                .iter()
                .flat_map(|line| &line.segments)
                .any(|segment| segment.text == text)
                .then_some(index)
        })
        .collect();
    assert_eq!(indices.len(), 1, "unique immutable source endpoint {text:?}");
    indices[0]
}

type Fingerprint = (String, [u32; 7], [bool; 3], Option<u8>);

fn assert_source_preserved(mut original: Vec<Fingerprint>, paragraphs: &[PdfParagraph]) {
    let mut observed: Vec<_> = paragraphs
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|line| &line.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    original.sort();
    observed.sort();
    assert_eq!(
        observed, original,
        "every source glyph/style/geometry projection survives"
    );
    assert_eq!(
        paragraphs
            .iter()
            .flat_map(|p| &p.lines)
            .map(|line| line.native_scripts.len())
            .sum::<usize>(),
        1,
        "retain the one real source-owned script endpoint"
    );
}

#[test]
fn should_preserve_sparse_column_boundary_without_learning_its_gap_as_pitch_2275() {
    let first = "first sparse paragraph ends without a terminator";
    let second = "lowercase separate sparse paragraph";
    let (segments, evidence) = carrier(vec![
        span(first, 42.0, 700.0, 250.0, 12.0),
        span(second, 42.0, 678.0, 250.0, 12.0),
        span("x", 315.0, 900.0, 10.0, 12.0),
        span("4 ", 325.0, 904.9568, 4.0, 8.0),
        span(".", 329.0, 900.0, 4.0, 12.0),
        span("right column establishes ordinary pitch", 315.0, 820.0, 250.0, 12.0),
        span("and another normal physical row.", 315.0, 808.0, 250.0, 12.0),
    ]);
    let scripts: std::collections::HashSet<_> = evidence.owners.iter().map(|owner| owner.script_index).collect();
    let ordinary = segments
        .iter()
        .enumerate()
        .filter(|(index, _)| !scripts.contains(index))
        .map(|(_, segment)| segment);
    let previous_bands = super::super::paragraph_line_bands(ordinary);
    let previous_gaps = super::super::paragraph_gaps_from_bands(&previous_bands, super::super::body_leading);
    assert!(
        super::super::baselines_cross_source_gap(700.0, 678.0, &previous_gaps),
        "the previous ordinary page calibration preserves this boundary using real 12pt context"
    );
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    let (segments, index) = order_native_paragraph_segments(segments, evidence, &gaps);
    let initial: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    let source_pair: Vec<_> = segments
        .iter()
        .filter(|segment| segment.text == first || segment.text == second)
        .collect();
    assert_eq!(source_pair.len(), 2);
    assert!(
        super::super::super::paragraphs::source_rows_continue(source_pair[0], source_pair[1]),
        "legacy continuation would merge without a gap barrier"
    );
    let mut paragraphs = super::super::blocks_to_paragraphs_with_native_scripts(segments, &[], &gaps, Some(&index));
    assert_ne!(
        paragraph_index(&paragraphs, first),
        paragraph_index(&paragraphs, second),
        "the actual native grouper initially retains the old detected boundary"
    );
    let plan = index
        .into_continuation_gaps()
        .expect("accepted native owner enables actual gap plan");
    plan.merge(&mut paragraphs);
    assert_source_preserved(initial, &paragraphs);
    assert_ne!(
        paragraph_index(&paragraphs, first),
        paragraph_index(&paragraphs, second),
        "do not learn an isolated sparse-column paragraph gap as its normal body pitch"
    );
}

#[test]
fn should_keep_a_gutter_crossing_source_script_in_its_ordinary_column_2275() {
    let first = "left prose before an inline power ";
    let second = "and the next physical row continues";
    let (segments, evidence) = carrier(vec![
        span("left column context", 42.0, 610.0, 250.0, 10.0),
        span("another left row", 42.0, 598.5, 250.0, 10.0),
        span("more left context", 42.0, 587.0, 250.0, 10.0),
        span("ordinary left prose begins", 42.0, 574.3379, 250.0, 10.0),
        span(first, 42.0, 562.8379, 240.0, 10.0),
        span("x", 282.0, 562.8379, 10.0, 10.0),
        span("4 ", 292.0, 565.0615, 3.892, 7.0),
        span(second, 42.0, 551.3379, 250.0, 10.0),
        span("finishes the same sentence.", 42.0, 539.8379, 250.0, 10.0),
        span("right column has a tighter ordinary pitch", 315.0, 650.0, 250.0, 10.0),
        span("second normal right row", 315.0, 640.0, 250.0, 10.0),
        span("ter 3", 315.0, 566.8379, 20.0, 10.0),
    ]);
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    let (segments, index) = order_native_paragraph_segments(segments, evidence, &gaps);
    assert_eq!(index.by_script.len(), 1);
    let owner = index.by_script.values().next().unwrap();
    let base = &segments[owner.base_index];
    let script = &segments[owner.script_index];
    let next = segments.iter().find(|segment| segment.text == second).unwrap();

    let ordinary: Vec<_> = segments
        .iter()
        .enumerate()
        .filter(|(i, _)| !index.by_script.contains_key(i))
        .map(|(_, s)| s)
        .collect();
    let corridor = super::super::page_column_corridor_for_segments(ordinary.iter().copied()).unwrap();
    assert!(
        base.upright_advance_extent().1 <= corridor.0
            && script.upright_advance_extent().1 > corridor.0
            && script.upright_advance_extent().0 < corridor.1,
        "actual raised source ink protrudes into the corridor"
    );
    let old_bands = super::super::paragraph_line_bands(ordinary.iter().copied());
    let old_page_gaps = super::super::paragraph_gaps_from_bands(&old_bands, super::super::body_leading);
    assert!(
        super::super::baselines_cross_source_gap(base.upright_baseline(), next.upright_baseline(), &old_page_gaps),
        "the foreign-column band provides a real global marker, not a manually injected gap"
    );
    let left_bands = super::super::paragraph_line_bands(
        ordinary
            .iter()
            .copied()
            .filter(|s| s.upright_advance_extent().1 <= corridor.0),
    );
    let left_gaps = super::super::paragraph_gaps_from_bands(&left_bands, super::super::body_leading);
    assert!(
        !super::super::baselines_cross_source_gap(base.upright_baseline(), next.upright_baseline(), &left_gaps),
        "the same ordinary endpoints have no column-local source gap"
    );
    assert!(
        super::super::super::paragraphs::source_rows_continue(base, next),
        "actual ordinary source rows satisfy unchanged continuation rules"
    );
    let initial: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    let mut paragraphs = super::super::blocks_to_paragraphs_with_native_scripts(segments, &[], &gaps, Some(&index));
    assert_ne!(
        paragraph_index(&paragraphs, first),
        paragraph_index(&paragraphs, second),
        "initial coarse marker separates rows before the real scoped merge"
    );
    let plan = index.into_continuation_gaps().unwrap();
    plan.merge(&mut paragraphs);
    assert_source_preserved(initial, &paragraphs);
    assert_eq!(
        paragraph_index(&paragraphs, first),
        paragraph_index(&paragraphs, second),
        "a source-owned exponent in the gutter must not import the foreign-column gap"
    );
}

#[test]
fn should_not_learn_full_height_stacked_band_advances_as_ordinary_body_pitch_2275() {
    let first = "ordinary wrapped paragraph begins";
    let second = "and continues on another physical row";
    let (segments, evidence) = carrier(vec![
        span("x", 80.0, 900.0, 10.0, 12.0),
        span("4 ", 90.0, 904.9568, 4.0, 8.0),
        span(".", 94.0, 900.0, 4.0, 12.0),
        span("unraised stacked numerator", 80.0, 800.0, 200.0, 12.0),
        span("unraised stacked denominator", 80.0, 787.7, 200.0, 12.0),
        span(first, 80.0, 700.0, 300.0, 12.0),
        span(second, 80.0, 680.8, 300.0, 12.0),
        span("with more ordinary source words", 80.0, 661.6, 300.0, 12.0),
        span("ending the same sentence.", 80.0, 642.4, 300.0, 12.0),
    ]);
    let numerator = segments
        .iter()
        .find(|s| s.text == "unraised stacked numerator")
        .unwrap();
    let denominator = segments
        .iter()
        .find(|s| s.text == "unraised stacked denominator")
        .unwrap();
    let advance = numerator.upright_baseline() - denominator.upright_baseline();
    assert!(
        advance >= 12.0 && advance < 12.4,
        "nonbody source pitch passes the one-line-height filter"
    );
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    assert!(
        super::super::baselines_cross_source_gap(700.0, 680.8, &gaps),
        "the actual coarse calibration exposes the false ordinary-wrap marker"
    );
    let (segments, index) = order_native_paragraph_segments(segments, evidence, &gaps);
    let previous = segments.iter().find(|s| s.text == first).unwrap();
    let next = segments.iter().find(|s| s.text == second).unwrap();
    assert!(
        super::super::super::paragraphs::source_rows_continue(previous, next),
        "unchanged actual source rows satisfy normal continuation rules"
    );
    let initial: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    let mut paragraphs = super::super::blocks_to_paragraphs_with_native_scripts(segments, &[], &gaps, Some(&index));
    assert_ne!(
        paragraph_index(&paragraphs, first),
        paragraph_index(&paragraphs, second),
        "the false coarse marker is present before the actual native continuation stage"
    );
    index
        .into_continuation_gaps()
        .expect("actual native gap plan")
        .merge(&mut paragraphs);
    assert_source_preserved(initial, &paragraphs);
    assert_eq!(
        paragraph_index(&paragraphs, first),
        paragraph_index(&paragraphs, second),
        "full-height stacked bands must not turn ordinary 19.2pt wrapping into paragraph gaps"
    );
}
