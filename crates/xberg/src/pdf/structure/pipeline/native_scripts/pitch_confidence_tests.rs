use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::types::PdfParagraph;

use super::*;

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32, height: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, height),
        font_size,
        font_name: "immutable-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn carrier(rows: Vec<TextSpan>) -> (Vec<SegmentData>, NativeSegmentEvidence) {
    let mut spans = vec![
        span("x", 80.0, 1000.0, 10.0, 12.0, 12.0),
        span("4 ", 90.0, 1004.9568, 4.0, 8.0, 8.0),
        span(".", 94.0, 1000.0, 4.0, 12.0, 12.0),
    ];
    spans.extend(rows);
    let page = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let projected = page.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(projected, page, &[]);
    let evidence = evidence.expect("actual immutable native source carrier");
    assert_eq!(evidence.owners.len(), 1, "one real accepted remote script");
    (segments, evidence)
}

fn ordinary<'a>(segments: &'a [SegmentData], evidence: &NativeSegmentEvidence) -> Vec<&'a SegmentData> {
    let scripts: std::collections::HashSet<_> = evidence.owners.iter().map(|owner| owner.script_index).collect();
    segments
        .iter()
        .enumerate()
        .filter_map(|(index, segment)| (!scripts.contains(&index)).then_some(segment))
        .collect()
}

fn prior_gaps<'a>(segments: impl Iterator<Item = &'a SegmentData>) -> Vec<f32> {
    let bands = super::super::paragraph_line_bands(segments);
    assert!(bands.len() >= 2, "nonempty real source calibration");
    assert!(
        bands
            .windows(2)
            .all(|pair| { pair[0].anchor_y - pair[1].anchor_y >= pair[0].height.max(pair[1].height) })
    );
    // All advances exceed line height, so this matches the prior ordinary minimum calibration. ~keep
    let mut gaps = super::super::paragraph_gaps_from_bands(&bands, super::super::body_leading);
    gaps.sort_by(f32::total_cmp);
    gaps
}

fn stage(segments: Vec<SegmentData>, evidence: NativeSegmentEvidence) -> (Vec<PdfParagraph>, NativeParagraphGaps) {
    let mut gaps = super::super::compute_paragraph_gap_ys(&segments);
    gaps.sort_by(f32::total_cmp);
    let (segments, index) = order_native_paragraph_segments(segments, evidence, &gaps);
    assert_eq!(index.by_script.len(), 1);
    let paragraphs = super::super::blocks_to_paragraphs_with_native_scripts(segments, &[], &gaps, Some(&index));
    (
        paragraphs,
        index.into_continuation_gaps().expect("actual native gap plan"),
    )
}

fn paragraph_index(paragraphs: &[PdfParagraph], text: &str) -> usize {
    let matches: Vec<_> = paragraphs
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
    assert_eq!(matches.len(), 1, "exactly one retained source endpoint {text:?}");
    matches[0]
}

fn assert_source_preserved(segments: &[SegmentData], paragraphs: &[PdfParagraph]) {
    let mut original: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    let mut observed: Vec<_> = paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .flat_map(|line| &line.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    original.sort();
    observed.sort();
    assert_eq!(
        observed, original,
        "all source text/style/geometry survives exactly once"
    );
    assert_eq!(
        paragraphs
            .iter()
            .flat_map(|paragraph| &paragraph.lines)
            .map(|line| line.valid_native_scripts().len())
            .sum::<usize>(),
        1,
        "the complete immutable source proof remains valid"
    );
}

#[test]
fn should_preserve_repeated_paragraph_gaps_after_a_two_line_body_run_2275() {
    let texts = [
        "ordinary first paragraph begins across this full source row",
        "and completes its real two line body run without a terminator",
        "another separate paragraph starts in lowercase without a terminator",
        "yet another separate paragraph starts after the same blank gap",
    ];
    let (segments, evidence) = carrier(
        texts
            .iter()
            .zip([900.0, 888.0, 866.0, 844.0])
            .map(|(text, y)| span(text, 42.0, y, 250.0, 12.0, 12.0))
            .collect(),
    );
    let prior = prior_gaps(ordinary(&segments, &evidence).into_iter());
    for (upper, lower) in [(888.0, 866.0), (866.0, 844.0)] {
        assert!(super::super::baselines_cross_source_gap(upper, lower, &prior));
    }
    let (mut paragraphs, plan) = stage(segments.clone(), evidence);
    let mut control = paragraphs.clone();
    super::super::merge_continuation_paragraphs_at_source_gaps(&mut control, &prior);
    assert_eq!(paragraph_index(&control, texts[0]), paragraph_index(&control, texts[1]));
    assert_ne!(paragraph_index(&control, texts[1]), paragraph_index(&control, texts[2]));
    assert_ne!(paragraph_index(&control, texts[2]), paragraph_index(&control, texts[3]));
    assert_source_preserved(&segments, &control);
    plan.merge(&mut paragraphs);
    assert_source_preserved(&segments, &paragraphs);
    assert_ne!(
        paragraph_index(&paragraphs, texts[1]),
        paragraph_index(&paragraphs, texts[2]),
        "repeated blank paragraph gaps must not replace the genuine 12pt body pitch"
    );
    assert_ne!(
        paragraph_index(&paragraphs, texts[2]),
        paragraph_index(&paragraphs, texts[3])
    );
}

fn assert_prior_column_heals(paragraphs: &[PdfParagraph], texts: [&str; 2], gaps: &[f32]) {
    let first = paragraph_index(paragraphs, texts[0]);
    let next = paragraph_index(paragraphs, texts[1]);
    assert_ne!(
        first, next,
        "the actual 1.6pt font transition initially splits the rows"
    );
    let mut control = vec![paragraphs[first].clone(), paragraphs[next].clone()];
    super::super::merge_continuation_paragraphs_at_source_gaps(&mut control, gaps);
    assert_eq!(
        paragraph_index(&control, texts[0]),
        paragraph_index(&control, texts[1]),
        "the unchanged real continuation merger heals the style split with prior local calibration"
    );
}

#[test]
fn should_not_inherit_an_alternating_column_pitch_for_style_split_wraps_2275() {
    let left = [
        "ordinary left prose fills the source column",
        "and continues across the next left source row",
    ];
    let right = [
        "ordinary right prose fills the source column",
        "and continues across the next right source row",
    ];
    let (segments, evidence) = carrier(vec![
        span(left[0], 42.0, 900.0, 250.0, 12.0, 12.0),
        span(left[1], 42.0, 872.0, 250.0, 13.6, 12.0),
        span(right[0], 315.0, 886.0, 250.0, 12.0, 12.0),
        span(right[1], 315.0, 858.0, 250.0, 13.6, 12.0),
    ]);
    let ordinary = ordinary(&segments, &evidence);
    let corridor = super::super::page_column_corridor_for_segments(ordinary.iter().copied())
        .expect("real nonoverlapping native source columns");
    let left_gaps = prior_gaps(
        ordinary
            .iter()
            .copied()
            .filter(|s| s.upright_advance_extent().1 <= corridor.0),
    );
    let right_gaps = prior_gaps(
        ordinary
            .iter()
            .copied()
            .filter(|s| s.upright_advance_extent().0 >= corridor.1),
    );
    assert!(!super::super::baselines_cross_source_gap(900.0, 872.0, &left_gaps));
    assert!(!super::super::baselines_cross_source_gap(886.0, 858.0, &right_gaps));
    let (mut paragraphs, plan) = stage(segments.clone(), evidence);
    assert_prior_column_heals(&paragraphs, left, &left_gaps);
    assert_prior_column_heals(&paragraphs, right, &right_gaps);
    plan.merge(&mut paragraphs);
    assert_source_preserved(&segments, &paragraphs);
    assert_eq!(
        paragraph_index(&paragraphs, left[0]),
        paragraph_index(&paragraphs, left[1]),
        "a page pitch fabricated by alternating columns must not block a real local continuation"
    );
    assert_eq!(
        paragraph_index(&paragraphs, right[0]),
        paragraph_index(&paragraphs, right[1])
    );
    assert_ne!(
        paragraph_index(&paragraphs, left[0]),
        paragraph_index(&paragraphs, right[0])
    );
}
