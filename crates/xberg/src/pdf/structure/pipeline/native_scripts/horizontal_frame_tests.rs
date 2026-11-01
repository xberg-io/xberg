use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::types::{PdfLine, PdfParagraph};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

const UPPER: &str = "ordinary source prose continues";
const LOWER: &str = "and the ordinary account continues here";
const FOLLOWING: &str = "and another physical row carries the account";
const FONT: f32 = 12.0;

struct FrameCase {
    segments: Vec<SegmentData>,
    evidence: NativeSegmentEvidence,
    gaps: Vec<f32>,
    foreign: bool,
}

fn span(text: &str, x: f32, y: f32, width: f32, font: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font),
        font_size: font,
        font_name: "horizontal-frame-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn fixture(foreign: bool) -> FrameCase {
    let (left, width) = if foreign { (72.0, 228.0) } else { (299.0, 261.0) };
    let spans = vec![
        span("x", 80.0, 800.0, 10.0, FONT),
        span("4 ", 90.0, 804.9568, 4.0, 8.0),
        span(".", 94.0, 800.0, 4.0, FONT),
        span("a", 80.0, 750.0, 8.0, FONT),
        span("b", 80.0, 742.0, 8.0, FONT),
        span("ordinary bridge", 250.0, 740.0, 100.0, FONT),
        span(UPPER, 315.0, 700.0, 120.0, FONT),
        span(LOWER, 315.0, 680.8, 245.0, FONT),
        span(FOLLOWING, left, 661.6, width, FONT),
    ];
    let page = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let (segments, evidence) = prepare_segments(page.segments().cloned().collect(), page, &[]);
    let evidence = evidence.expect("actual native carrier");
    assert_eq!(segments.len(), 9);
    assert!(evidence.origin.is_some());
    assert_eq!(evidence.owners.len(), 1, "one unrelated real native source cohort");
    assert!(evidence.owners[0].base_index < 3 && evidence.owners[0].script_index < 3);
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    FrameCase {
        segments,
        evidence,
        gaps,
        foreign,
    }
}

fn fingerprint(segment: &SegmentData) -> (String, [u32; 7], [bool; 3], Option<u8>) {
    super::line_tests::fingerprint(segment)
}

fn line_for<'a>(paragraphs: &'a [PdfParagraph], text: &str) -> &'a PdfLine {
    let lines: Vec<_> = paragraphs
        .iter()
        .flat_map(|p| &p.lines)
        .filter(|line| line.segments.iter().any(|segment| segment.text == text))
        .collect();
    assert_eq!(lines.len(), 1);
    lines[0]
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
    assert_eq!(matches.len(), 1, "actual target appears exactly once");
    matches[0]
}

fn assert_no_corridor(ordered: &[SegmentData], index: &NativeScriptIndex) {
    let ordinary: Vec<_> = ordered
        .iter()
        .enumerate()
        .filter(|(i, _)| !index.by_script.contains_key(i))
        .map(|(_, segment)| segment)
        .collect();
    assert!(super::super::page_column_corridor_for_segments(ordinary.iter().copied()).is_none());
    let left = ordinary
        .iter()
        .map(|s| s.upright_advance_extent().0)
        .fold(f32::INFINITY, f32::min);
    let right = ordinary
        .iter()
        .map(|s| s.upright_advance_extent().1)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        ordinary
            .iter()
            .all(|segment| segment.width < (right - left) * super::super::PARAGRAPH_FULL_WIDTH_FURNITURE_FRACTION),
        "the ordinary bridge closes the gutter without full-width furniture"
    );
}

fn assert_original_votes(
    case: &FrameCase,
    ordered: &[SegmentData],
    index: &NativeScriptIndex,
    upper: &PdfLine,
    lower: &PdfLine,
) {
    let bands = super::super::paragraph_line_bands(ordered.iter());
    let height = super::super::median_line_height(&bands).unwrap();
    let leading = super::super::body_leading(&bands, height);
    assert_eq!(height, FONT);
    assert_eq!(leading, FONT, "real original estimator noise is preserved");
    let ordinary = ordered
        .iter()
        .enumerate()
        .filter(|(i, _)| !index.by_script.contains_key(i))
        .map(|(_, s)| s);
    let bands = super::super::paragraph_line_bands(ordinary);
    let mut local = super::super::paragraph_gaps_with_leading(&bands, height, leading);
    let mut provided = case.gaps.clone();
    local.sort_by(f32::total_cmp);
    provided.sort_by(f32::total_cmp);
    for gaps in [&provided, &local] {
        assert!(super::super::baselines_cross_source_gap(
            upper.baseline_y,
            lower.baseline_y,
            gaps
        ));
    }
    let store = source_projection::ProjectionStore::new(ordered, &index.original_indices, &index.by_script).unwrap();
    let grouped =
        super::super::blocks_to_paragraphs_with_native_scripts(ordered.to_vec(), &[], &case.gaps, Some(index));
    let rows = source_rows::SourceRows::new(&store, &grouped).unwrap();
    let upper_row = rows.complete_row(upper).expect("complete immutable upper source row");
    let lower_row = rows.complete_row(lower).expect("complete immutable lower source row");
    let following_row = rows
        .complete_row(line_for(&grouped, FOLLOWING))
        .expect("complete immediate following row");
    assert_eq!(lower_row, upper_row + 1);
    assert_eq!(following_row, lower_row + 1);
    assert!(rows.actual_pair_breaks(upper, lower, height, leading));
}

fn assert_source_projection(ordered: &[SegmentData], index: &NativeScriptIndex) {
    let snapshot = index.by_script.values().next().unwrap().page_projection().unwrap();
    assert_eq!(snapshot.records().len(), 9);
    assert_eq!(index.original_indices.len(), 9);
    assert_eq!(
        index
            .original_indices
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        9
    );
    for record in snapshot.records() {
        let position = index
            .original_indices
            .iter()
            .position(|id| *id == record.original_index)
            .unwrap();
        assert_eq!(fingerprint(&ordered[position]), fingerprint(&record.segment));
    }
}

fn assert_intent_geometry(case: &FrameCase, upper: &PdfLine, lower: &PdfLine, following: &PdfLine) {
    for row in [upper, lower, following] {
        assert_eq!(row.segments.len(), 1);
        assert!(row.valid_native_scripts().is_empty());
        assert!(!crate::pdf::native::table::is_dense_numeric_row(
            row.segments[0].text.split_whitespace()
        ));
    }
    let (upper, lower, following) = (&upper.segments[0], &lower.segments[0], &following.segments[0]);
    let lower_right = lower.upright_advance_extent().1;
    assert!(!super::super::heading_fills_column(upper, lower_right));
    assert!(!super::super::heading_line_ran_out_of_room(upper, lower, lower_right));
    assert!(crate::pdf::structure::paragraphs::source_rows_continue(
        lower, following
    ));
    assert!(super::super::heading_fills_column(
        lower,
        following.upright_advance_extent().1
    ));
    assert!(lower.x - following.x > FONT * super::super::HEADING_HANGING_INDENT_LEFT_EDGE_TOLERANCE_FONT_FACTOR);
    if case.foreign {
        assert!(
            following.upright_advance_extent().1 < lower.upright_advance_extent().0,
            "the apparent outdent is a completely disjoint foreign column"
        );
    } else {
        assert!(
            following.x < lower.x && following.upright_advance_extent().1 == lower_right,
            "a genuine hanging indent shares the same body frame"
        );
    }
}

fn assert_preconditions(case: &FrameCase) {
    let evidence = NativeSegmentEvidence {
        origin: case.evidence.origin,
        original_indices: case.evidence.original_indices.clone(),
        owners: case.evidence.owners.clone(),
    };
    let (ordered, index) = order_native_paragraph_segments(case.segments.clone(), evidence, &case.gaps);
    assert_no_corridor(&ordered, &index);
    assert_source_projection(&ordered, &index);
    let grouped =
        super::super::blocks_to_paragraphs_with_native_scripts(ordered.clone(), &[], &case.gaps, Some(&index));
    let (upper, lower, following) = (
        line_for(&grouped, UPPER),
        line_for(&grouped, LOWER),
        line_for(&grouped, FOLLOWING),
    );
    assert_intent_geometry(&case, upper, lower, following);
    assert_original_votes(&case, &ordered, &index, upper, lower);
    let gaps = index.continuation_gaps.as_ref().expect("actual native gap plan");
    assert!(
        !gaps.crosses_rows(&upper.segments[0], &lower.segments[0]),
        "calibrated geometry allows the continuation"
    );
    let (a, b) = (paragraph_for(&grouped, UPPER), paragraph_for(&grouped, LOWER));
    assert_ne!(
        a, b,
        "original real grouping keeps the boundary before the final merger"
    );
    let mut generic = vec![grouped[a].clone(), grouped[b].clone()];
    crate::pdf::structure::paragraphs::merge_continuation_paragraphs(&mut generic);
    assert_eq!(generic.len(), 1, "the unchanged generic merger accepts the boundary");
}

fn assert_result(case: FrameCase, together: bool) {
    assert_preconditions(&case);
    let mut before: Vec<_> = case.segments.iter().map(fingerprint).collect();
    let output = super::super::segments_to_paragraphs_with_native_scripts(
        case.segments,
        &[],
        &case.gaps,
        &Default::default(),
        Some(case.evidence),
    );
    let mut after: Vec<_> = output
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|line| &line.segments)
        .map(fingerprint)
        .collect();
    before.sort();
    after.sort();
    assert_eq!(after, before, "all source glyph fields survive unchanged exactly once");
    assert_eq!(
        output
            .iter()
            .flat_map(|p| &p.lines)
            .map(|line| line.valid_native_scripts().len())
            .sum::<usize>(),
        1
    );
    assert_eq!(
        paragraph_for(&output, UPPER) == paragraph_for(&output, LOWER),
        together,
        "only same-horizontal-frame evidence may corroborate a new paragraph"
    );
}

#[test]
fn should_join_right_prose_when_only_a_disjoint_left_row_supplies_an_apparent_indent_2275() {
    assert_result(fixture(true), true);
}

#[test]
fn should_preserve_a_genuine_right_column_indent_with_a_common_following_body_frame_2275() {
    assert_result(fixture(false), false);
}
