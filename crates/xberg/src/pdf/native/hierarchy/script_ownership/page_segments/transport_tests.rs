use std::path::Path;

use sha2::{Digest, Sha256};
use xberg_native_pdf::extractors::text::{ArtifactType, PaginationSubtype};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use super::super::super::{
    ArtifactRetention, NativeDocument, ScriptGeometry, dedupe_redrawn_segments, extract_all_segments,
    extract_all_segments_for_tables,
};
use super::super::collect_suffix_script_owners;
use super::*;

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_size,
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn pair() -> Vec<TextSpan> {
    vec![span("q", 100.0, 700.0, 10.0, 12.0), span("2 ", 110.0, 704.0, 4.0, 8.0)]
}

fn source(name: &str, sha: &str) -> NativeDocument {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../test_documents/pdf/regression_pages/{name}.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory source {}: {error}", path.display()));
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), sha);
    assert!(bytes.starts_with(b"%PDF-"));
    let document = NativeDocument::open_bytes(&bytes).expect("open SHA-bound source");
    assert_eq!(document.doc.page_count().expect("source page count"), 1);
    document
}

fn assert_projection_eq(actual: &SegmentData, expected: &SegmentData) {
    assert!(SegmentProjection::from(actual) == SegmentProjection::from(expected));
}

#[test]
fn should_construct_exact_projection_roles_and_original_witnesses_from_final_spans_2275() {
    let mut spans = pair();
    spans[0].mcid = Some(7);
    spans[0].font_name = "immutable-base-font".to_owned();
    spans[1].font_name = "immutable-script-font".to_owned();
    assert_eq!(collect_suffix_script_owners(&spans).len(), 1);
    let roles = HashMap::from([(7, Some(2))]);
    let page = NativePageSegments::from_final_spans(
        spans.clone(),
        &roles,
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    assert_eq!(page.segments.len(), 2);
    assert_eq!(page.segments[0].original_index, 0);
    assert_eq!(page.segments[1].original_index, 1);
    assert_eq!(page.segments[0].segment.assigned_role, Some(2));
    assert_eq!(page.segments[1].segment.baseline_y, 704.0);
    assert_eq!(page.segments[1].segment.text, "2 ");
    let owners = page.remap_script_owners(&page.segments);
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].source.logical_baseline, 700.0);
    assert_eq!(owners[0].source.script_byte_range, 0..1);
    for (witness, original) in [
        (&owners[0].source.base_witness, &spans[0]),
        (&owners[0].source.script_witness, &spans[1]),
    ] {
        assert_eq!(
            serde_json::to_vec(&*witness.snapshot).unwrap(),
            serde_json::to_vec(original).unwrap()
        );
        assert_eq!(witness.snapshot.mirrored, original.mirrored);
        assert_eq!(witness.snapshot.page_rotation_applied, original.page_rotation_applied);
    }
}

#[test]
fn should_keep_original_ordinals_after_blank_and_artifact_filtering_2275() {
    let mut spans = pair();
    spans.insert(0, span(" ", 50.0, 700.0, 4.0, 12.0));
    let mut header = span("running header", 100.0, 780.0, 80.0, 12.0);
    header.artifact_type = Some(ArtifactType::Pagination(PaginationSubtype::Header));
    spans.push(header);
    assert_eq!(collect_suffix_script_owners(&spans).len(), 1);
    let page = NativePageSegments::from_final_spans(
        spans,
        &HashMap::new(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    assert_eq!(
        page.segments
            .iter()
            .map(|record| record.original_index)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    let retained = [page.segments[1].clone(), page.segments[0].clone()];
    let owners = page.remap_script_owners(&retained);
    assert_eq!(owners.len(), 1);
    assert_eq!((owners[0].base_index, owners[0].script_index), (1, 0));
    assert_eq!(owners[0].source.base_witness.original_index, 1);
    assert_eq!(owners[0].source.script_witness.original_index, 2);
    assert!(page.remap_script_owners(&retained[..1]).is_empty());
}

#[test]
fn should_preserve_legacy_redraw_projection_and_omit_changed_or_ambiguous_edges_2275() {
    let mut spans = pair();
    let mut duplicate = spans[0].clone();
    duplicate.is_italic = true;
    duplicate.font_weight = xberg_native_pdf::layout::text_block::FontWeight::Bold;
    spans.push(duplicate);
    let expected = dedupe_redrawn_segments(vec![
        SegmentData {
            text: "q".to_owned(),
            x: 100.0,
            y: 700.0,
            width: 10.0,
            height: 12.0,
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            rotation_degrees: 0.0,
            assigned_role: None,
        },
        SegmentData {
            text: "2 ".to_owned(),
            x: 110.0,
            y: 704.0,
            width: 4.0,
            height: 8.0,
            font_size: 8.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 704.0,
            rotation_degrees: 0.0,
            assigned_role: None,
        },
        SegmentData {
            text: "q".to_owned(),
            x: 100.0,
            y: 700.0,
            width: 10.0,
            height: 12.0,
            font_size: 12.0,
            is_bold: true,
            is_italic: true,
            is_monospace: false,
            baseline_y: 700.0,
            rotation_degrees: 0.0,
            assigned_role: None,
        },
    ]);
    assert_eq!(expected.len(), 2);
    assert!(expected[0].is_bold && expected[0].is_italic);
    let page = NativePageSegments::from_final_spans(
        spans,
        &HashMap::new(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    assert_eq!(page.segments.len(), expected.len());
    for (actual, expected) in page.segments.iter().zip(&expected) {
        assert_projection_eq(&actual.segment, expected);
    }
    assert!(page.remap_script_owners(&page.segments).is_empty());
}

fn assert_source_transport(name: &str, sha: &str, expected_powers: &[(&str, f32, f32)]) {
    let mut document = source(name, sha);
    let (legacy, used_tree) = extract_all_segments(&mut document, ArtifactRetention::default()).unwrap();
    assert_eq!(legacy.len(), 1);
    assert!(!legacy[0].is_empty(), "real legacy source extraction must run");
    let (pages, transported_tree) = extract_all_segment_carriers(&mut document, ArtifactRetention::default()).unwrap();
    assert_eq!(transported_tree, used_tree);
    assert_eq!(pages.len(), 1, "actual carrier extraction preserves the source page");
    assert_eq!(pages[0].segments.len(), legacy[0].len());
    for (actual, expected) in pages[0].segments.iter().zip(&legacy[0]) {
        assert_projection_eq(&actual.segment, expected);
    }
    let owners = pages[0].remap_script_owners(&pages[0].segments);
    for &(text, x, y) in expected_powers {
        let matching: Vec<_> = owners
            .iter()
            .filter(|owner| {
                let witness = &owner.source.script_witness.snapshot;
                witness.text == text && witness.bbox.x == x && witness.bbox.y == y
            })
            .collect();
        assert_eq!(matching.len(), 1, "exact source power {text:?} at {x}/{y}");
        let owner = matching[0];
        assert_eq!(pages[0].segments[owner.script_index].segment.baseline_y, y);
        assert!(
            pages[0]
                .remap_script_owners(&[pages[0].segments[owner.script_index].clone()])
                .is_empty()
        );
    }
}

#[test]
fn should_transport_all_six_sha_bound_ca18_powers_without_projection_changes_2275() {
    assert_source_transport(
        "gutenberg-33283-p59",
        "26154b7fb99ba469af046603135b49f87ce26e8bf6b7b191bc30de66f558ca2c",
        &[
            ("5", 103.065, 345.27896),
            ("4", 222.22499, 314.70798),
            ("3", 242.68896, 292.50598),
            ("3", 331.53598, 292.50598),
            ("2", 177.66597, 75.868996),
            ("n", 213.18996, 75.868996),
        ],
    );
}

#[test]
fn should_transport_three_sha_bound_aes_powers_without_projection_changes_2275() {
    assert_source_transport(
        "nist-fips-197-upd1-p16",
        "ddbb5a63987345c809b99487f583bc7eca2e9883131b38cc8e5fea59763ef33d",
        &[
            ("8 ", 282.247, 126.155),
            ("4 ", 305.1822, 126.155),
            ("3", 328.11737, 126.155),
        ],
    );
}

#[test]
fn should_leave_legacy_table_entrypoint_projection_and_tree_flag_unchanged_2275() {
    let mut document = source(
        "gutenberg-33283-p59",
        "26154b7fb99ba469af046603135b49f87ce26e8bf6b7b191bc30de66f558ca2c",
    );
    let (before, before_tree) = extract_all_segments_for_tables(&mut document, ArtifactRetention::default()).unwrap();
    assert_eq!(before.len(), 1);
    assert!(!before[0].is_empty());
    let _ = extract_all_segment_carriers(&mut document, ArtifactRetention::default()).unwrap();
    let (after, after_tree) = extract_all_segments_for_tables(&mut document, ArtifactRetention::default()).unwrap();
    assert_eq!(after_tree, before_tree);
    assert_eq!(after.len(), before.len());
    for (actual_page, expected_page) in after.iter().zip(&before) {
        assert_eq!(actual_page.len(), expected_page.len());
        for (actual, expected) in actual_page.iter().zip(expected_page) {
            assert_projection_eq(actual, expected);
        }
    }
}

#[test]
fn should_keep_pre_dedupe_projection_ambiguity_when_other_scope_duplicate_is_removed_2275() {
    let mut spans = pair();
    let mut duplicate = spans[0].clone();
    duplicate.mcid_scope = Some(McidScope::Page(1));
    spans.push(duplicate);
    assert_eq!(
        collect_suffix_script_owners(&spans).len(),
        1,
        "source scope resolves native evidence"
    );
    let page = NativePageSegments::from_final_spans(
        spans,
        &HashMap::new(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    assert_eq!(page.segments.len(), 2, "legacy redraw projection removes the duplicate");
    assert!(page.remap_script_owners(&page.segments).is_empty());
}

#[test]
fn should_honor_header_retention_without_treating_artifacts_as_script_owners_2275() {
    let mut header = span("running header", 100.0, 780.0, 80.0, 12.0);
    header.artifact_type = Some(ArtifactType::Pagination(PaginationSubtype::Header));
    assert!(collect_suffix_script_owners(&[header.clone()]).is_empty());
    let page = NativePageSegments::from_final_spans(
        vec![header],
        &HashMap::new(),
        ArtifactRetention {
            headers: true,
            ..Default::default()
        },
        ScriptGeometry::Preserve,
    );
    assert_eq!(page.segments.len(), 1);
    assert_eq!(page.segments[0].segment.text, "running header");
    assert!(page.script_owners.is_empty());
}
