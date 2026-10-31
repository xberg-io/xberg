use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::assembly::extract_text_with_source_ranges;
use crate::pdf::structure::types::PdfParagraph;

type Fingerprint = (String, [u32; 7], [bool; 3], Option<u8>);
const TEXT: &str =
    "The source has a genuine list: to retain the ordinary source records; where the sentence continues exactly.";

#[derive(Clone, Copy)]
enum Route {
    Verified,
    Unproved,
    RejectedProjection,
}

fn source(route: Route) -> PdfParagraph {
    let page = NativePageSegments::from_final_spans(
        vec![TextSpan {
            text: TEXT.to_owned(),
            bbox: Rect::new(42.0, 700.0, 600.0, 12.0),
            font_size: 12.0,
            font_name: "source-origin-font".to_owned(),
            mcid_scope: Some(McidScope::Page(0)),
            ..Default::default()
        }],
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let mut segments: Vec<_> = page.segments().cloned().collect();
    assert_eq!(segments.len(), 1, "one actual whole native source record");
    if matches!(route, Route::RejectedProjection) {
        segments[0].width += 1.0;
    }
    let (segments, evidence) = match route {
        Route::Unproved => (segments, None),
        _ => prepare_segments(segments, page, &[]),
    };
    if matches!(route, Route::Verified) {
        let proof = evidence.as_ref().expect("actual verified projection carrier");
        assert_eq!(proof.original_indices, [0]);
        assert!(
            proof.owners.is_empty(),
            "source origin is independent of script ownership"
        );
    } else {
        assert!(
            evidence.is_none(),
            "unproved or rejected projection cannot acquire native lineage"
        );
    }
    reconstruct(segments, evidence)
}

fn reconstruct(segments: Vec<SegmentData>, evidence: Option<NativeSegmentEvidence>) -> PdfParagraph {
    let gaps = super::super::compute_paragraph_gap_ys(&segments);
    let mut paragraphs =
        super::super::segments_to_paragraphs_with_native_scripts(segments, &[], &gaps, &Default::default(), evidence);
    assert_eq!(
        paragraphs.len(),
        1,
        "real native/generic orchestration produced a paragraph"
    );
    let mut paragraph = paragraphs.pop().unwrap();
    assert_eq!(fingerprints(&paragraph).len(), 1);
    assert!(
        paragraph
            .lines
            .iter()
            .all(|line| line.valid_native_scripts().is_empty())
    );
    let ledger = extract_text_with_source_ranges(&paragraph, &Default::default());
    assert_eq!(ledger.text, TEXT);
    assert_eq!(
        ledger.segment_ranges,
        [0..TEXT.len()],
        "every list cut crosses the actual source record"
    );
    paragraph.text = TEXT.to_owned();
    paragraph
}

fn fingerprints(paragraph: &PdfParagraph) -> Vec<Fingerprint> {
    paragraph
        .lines
        .iter()
        .flat_map(|line| &line.segments)
        .map(super::line_tests::fingerprint)
        .collect()
}

fn assert_same_scalar_representation(proven: &PdfParagraph, unproved: &PdfParagraph) {
    assert_eq!(proven.text, unproved.text, "identical authoritative cached text");
    assert_eq!(
        fingerprints(proven),
        fingerprints(unproved),
        "identical text/geometry/style/role fields"
    );
    assert_eq!(proven.block_bbox, unproved.block_bbox);
    assert_eq!(
        proven.dominant_font_size.to_bits(),
        unproved.dominant_font_size.to_bits()
    );
    assert_eq!(proven.word_count, unproved.word_count);
    assert_eq!(proven.heading_level, unproved.heading_level);
    assert_eq!(proven.is_bold, unproved.is_bold);
    assert!(!proven.is_list_item && !unproved.is_list_item);
    assert!(!proven.is_formula && !unproved.is_formula);
    assert!(!proven.is_code_block && !unproved.is_code_block);
    assert!(!proven.is_page_furniture && !unproved.is_page_furniture);
}

fn assert_legacy_split(paragraphs: &[PdfParagraph]) {
    assert_eq!(
        paragraphs.len(),
        3,
        "unproved cached content retains the existing legacy split"
    );
    assert_eq!(
        paragraphs
            .iter()
            .map(|paragraph| paragraph.text.as_str())
            .collect::<Vec<_>>(),
        [
            "The source has a genuine list:",
            "to retain the ordinary source records;",
            "where the sentence continues exactly.",
        ]
    );
    assert!(!paragraphs[0].is_list_item && paragraphs[1].is_list_item && paragraphs[2].is_list_item);
}

#[test]
fn should_retain_verified_ownerless_record_when_a_list_cut_crosses_it_2275() {
    let paragraph = source(Route::Verified);
    let original = fingerprints(&paragraph);
    let mut pages = vec![vec![paragraph]];
    super::super::run_in_lists::split_with_witnesses(&mut pages, &Default::default());
    assert_eq!(
        pages[0].len(),
        1,
        "verified source must not be discarded for an intra-record cut"
    );
    assert_eq!(pages[0][0].text, TEXT);
    assert_eq!(fingerprints(&pages[0][0]), original);
    assert!(!pages[0][0].is_list_item);
}

#[test]
fn should_split_unproved_cached_text_with_identical_source_geometry_2275() {
    let proven = source(Route::Verified);
    let unproved = source(Route::Unproved);
    assert_same_scalar_representation(&proven, &unproved);
    let original = fingerprints(&proven);
    let mut pages = vec![vec![proven], vec![unproved]];
    super::super::run_in_lists::split_with_witnesses(&mut pages, &Default::default());
    assert_eq!(
        pages[0].len(),
        1,
        "real carrier lineage retains the unsafe source record"
    );
    assert_eq!(fingerprints(&pages[0][0]), original);
    assert_legacy_split(&pages[1]);
}

#[test]
fn should_not_promote_a_rejected_native_projection_to_verified_list_source_2275() {
    let paragraph = source(Route::RejectedProjection);
    assert_eq!(
        paragraph.lines[0].segments[0].width, 601.0,
        "current fallback data was not dropped"
    );
    let mut pages = vec![vec![paragraph]];
    super::super::run_in_lists::split_with_witnesses(&mut pages, &Default::default());
    assert_legacy_split(&pages[0]);
}
