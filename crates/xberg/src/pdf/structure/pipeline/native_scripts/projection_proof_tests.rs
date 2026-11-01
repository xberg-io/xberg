use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::pipeline::native_scripts::{NativePageSegments, prepare_segments};
use crate::pdf::structure::pipeline::{blocks_to_paragraphs_with_native_scripts, native_scripts};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

const QUOTED: &str = "thus: “dee-eks.”";
const SECOND: &str = "second piece";

fn prepared(filter_second: bool) -> (Vec<SegmentData>, native_scripts::NativeSegmentEvidence) {
    let span = |text: &str, x, baseline, width, font| TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, baseline, width, font),
        font_size: font,
        font_name: "proof-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    };
    let page = NativePageSegments::from_final_spans(
        vec![
            span("x", 80.0, 800.0, 10.0, 12.0),
            span("4 ", 90.0, 804.9568, 4.0, 8.0),
            span(".", 94.0, 800.0, 4.0, 12.0),
            span(QUOTED, 80.0, 700.0, 150.0, 12.0),
            span(SECOND, 240.0, 700.0, 50.0, 12.0),
            span("and the next physical row continues", 80.0, 680.8, 210.0, 12.0),
        ],
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let tables = if filter_second {
        vec![crate::pdf::structure::pipeline::TableCoverage::from_rows(
            crate::types::BoundingBox {
                x0: 240.0,
                y0: 700.0,
                x1: 290.0,
                y1: 712.0,
            },
            [vec![SECOND]],
        )]
    } else {
        Vec::new()
    };
    let (segments, evidence) = prepare_segments(page.segments().cloned().collect(), page, &tables);
    let evidence = evidence.expect("real native carrier");
    assert_eq!(evidence.owners.len(), 1);
    assert_eq!(segments.len(), if filter_second { 5 } else { 6 });
    (segments, evidence)
}

fn fixture(filter_second: bool) -> (ProjectionStore, Vec<PdfParagraph>) {
    let (segments, evidence) = prepared(filter_second);
    let gaps = crate::pdf::structure::pipeline::compute_paragraph_gap_ys(&segments);
    let (ordered, index) = native_scripts::order_native_paragraph_segments(segments, evidence, &gaps);
    let store =
        ProjectionStore::new(&ordered, &index.original_indices, &index.by_script).expect("shared immutable page");
    let paragraphs = blocks_to_paragraphs_with_native_scripts(ordered, &[], &gaps, Some(&index));
    assert!(!paragraphs.is_empty());
    assert_eq!(
        paragraphs
            .iter()
            .flat_map(|p| &p.lines)
            .map(|line| line.valid_native_scripts().len())
            .sum::<usize>(),
        1
    );
    assert!(store.view(&paragraphs).resolve(segment(&paragraphs, QUOTED)).is_some());
    (store, paragraphs)
}

fn segment<'a>(paragraphs: &'a [PdfParagraph], text: &str) -> &'a SegmentData {
    paragraphs
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|l| &l.segments)
        .find(|segment| segment.text == text)
        .unwrap()
}

fn target_line(paragraphs: &[PdfParagraph]) -> &PdfLine {
    paragraphs
        .iter()
        .flat_map(|p| &p.lines)
        .find(|line| line.segments.iter().any(|segment| segment.text == QUOTED))
        .unwrap()
}

#[test]
fn should_reject_unobserved_ordinary_projection_changes_2275() {
    let (store, original) = fixture(false);
    assert!(
        super::super::source_rows::SourceRows::new(&store, &original)
            .unwrap()
            .complete_row(target_line(&original))
            .is_some()
    );
    let changes: [fn(&mut SegmentData); 4] = [
        |segment| segment.text.push('!'),
        |segment| segment.x += 1.0,
        |segment| segment.is_italic = !segment.is_italic,
        |segment| segment.assigned_role = Some(2),
    ];
    for change in changes {
        let mut paragraphs = original.clone();
        let changed = paragraphs
            .iter_mut()
            .flat_map(|p| &mut p.lines)
            .flat_map(|l| &mut l.segments)
            .find(|segment| segment.text == QUOTED)
            .unwrap();
        change(changed);
        assert!(store.view(&paragraphs).resolve(changed_segment(&paragraphs)).is_none());
    }
}

fn changed_segment(paragraphs: &[PdfParagraph]) -> &SegmentData {
    paragraphs
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|l| &l.segments)
        .find(|segment| segment.text.starts_with("thus:"))
        .unwrap()
}

#[test]
fn should_not_revive_a_duplicate_ordinary_projection_2275() {
    let (store, mut paragraphs) = fixture(false);
    let line = paragraphs
        .iter_mut()
        .flat_map(|p| &mut p.lines)
        .find(|line| line.segments.iter().any(|segment| segment.text == QUOTED))
        .unwrap();
    line.segments.push(
        line.segments
            .iter()
            .find(|segment| segment.text == QUOTED)
            .unwrap()
            .clone(),
    );
    assert!(store.view(&paragraphs).resolve(segment(&paragraphs, QUOTED)).is_none());
    assert!(
        super::super::source_rows::SourceRows::new(&store, &paragraphs)
            .unwrap()
            .complete_row(target_line(&paragraphs))
            .is_none()
    );
}

#[test]
fn should_require_a_table_filtered_member_of_the_complete_original_row_2275() {
    let (store, paragraphs) = fixture(true);
    assert!(
        !paragraphs
            .iter()
            .flat_map(|p| &p.lines)
            .flat_map(|l| &l.segments)
            .any(|segment| segment.text == SECOND)
    );
    assert!(
        store
            .source
            .records()
            .iter()
            .any(|record| record.segment.text == SECOND)
    );
    assert!(
        super::super::source_rows::SourceRows::new(&store, &paragraphs)
            .unwrap()
            .complete_row(target_line(&paragraphs))
            .is_none()
    );
}

#[test]
fn should_prove_only_the_actual_single_fused_quote_repair_2275() {
    let (mut store, mut paragraphs) = fixture(false);
    let mut unobserved = paragraphs.clone();
    let calls = std::cell::Cell::new(0);
    let witnesses = Default::default();
    crate::pdf::structure::text_repair::apply_to_all_segments_observed(
        &mut paragraphs,
        |text| {
            calls.set(calls.get() + 1);
            crate::pdf::structure::pipeline::fused_text_repairs(text, &witnesses)
        },
        |before, after| store.observe_repair(before, after),
    );
    assert_eq!(calls.get(), 6);
    let repaired = "thus: \"dee-eks.\"";
    assert!(
        store
            .view(&paragraphs)
            .resolve(segment(&paragraphs, repaired))
            .is_some()
    );
    let unchanged_store = fixture(false).0;
    crate::pdf::structure::text_repair::apply_to_all_segments(&mut unobserved, |text| {
        crate::pdf::structure::pipeline::fused_text_repairs(text, &witnesses)
    });
    assert!(
        unchanged_store
            .view(&unobserved)
            .resolve(segment(&unobserved, repaired))
            .is_none()
    );
    assert_eq!(
        store
            .source
            .records()
            .iter()
            .filter(|record| record.segment.text == QUOTED)
            .count(),
        1
    );
}

#[test]
fn should_release_page_projections_after_the_real_native_gap_merge_2275() {
    let (segments, evidence) = prepared(false);
    assert!(evidence.owners.iter().all(|owner| owner.page_projection().is_some()));
    let gaps = crate::pdf::structure::pipeline::compute_paragraph_gap_ys(&segments);
    let output = crate::pdf::structure::pipeline::segments_to_paragraphs_with_native_scripts(
        segments,
        &[],
        &gaps,
        &Default::default(),
        Some(evidence),
    );
    let owners: Vec<_> = output
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|line| line.valid_native_scripts())
        .collect();
    assert_eq!(owners.len(), 1, "the real complete native script remains valid");
    assert!(owners.iter().all(|owner| owner.page_projection().is_none()));
}

#[test]
fn should_not_launder_an_unobserved_edit_through_the_actual_fused_repair_2275() {
    let (mut store, mut paragraphs) = fixture(false);
    assert!(store.view(&paragraphs).resolve(segment(&paragraphs, SECOND)).is_some());
    paragraphs
        .iter_mut()
        .flat_map(|p| &mut p.lines)
        .flat_map(|line| &mut line.segments)
        .find(|segment| segment.text == SECOND)
        .unwrap()
        .text = "second\x01 piece".to_owned();
    let witnesses = Default::default();
    crate::pdf::structure::text_repair::apply_to_all_segments_observed(
        &mut paragraphs,
        |text| crate::pdf::structure::pipeline::fused_text_repairs(text, &witnesses),
        |before, after| store.observe_repair(before, after),
    );
    assert_eq!(
        segment(&paragraphs, SECOND).text,
        SECOND,
        "the real repair restored the original text bytes"
    );
    assert!(
        store.view(&paragraphs).resolve(segment(&paragraphs, SECOND)).is_none(),
        "an unbound pre-repair transition cannot revive proof"
    );
}
