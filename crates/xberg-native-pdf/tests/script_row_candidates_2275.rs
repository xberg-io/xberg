//! Read-only geometric evidence; semantic powers and footnotes remain a caller decision.

use std::path::Path;

use sha2::{Digest, Sha256};
use xberg_native_pdf::document::{PdfDocument, ReadingOrder};
use xberg_native_pdf::extractors::text::script_row_candidates;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::object::ObjectRef;
use xberg_native_pdf::structure::McidScope;

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_size,
        font_name: "row-evidence-font".to_owned(),
        sequence: usize::MAX,
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn relations(spans: &[TextSpan]) -> Vec<(usize, usize, f32)> {
    script_row_candidates(spans)
        .iter()
        .map(|candidate| {
            (
                candidate.base_index(),
                candidate.script_index(),
                candidate.logical_baseline(),
            )
        })
        .collect()
}

fn metadata(spans: &[TextSpan]) -> (Vec<u8>, Vec<(bool, i32)>) {
    (
        serde_json::to_vec(spans).expect("serialize glyph metadata"),
        spans
            .iter()
            .map(|span| (span.mirrored, span.page_rotation_applied))
            .collect(),
    )
}

fn source_spans(name: &str, expected_sha256: &str, expected_count: usize) -> Vec<TextSpan> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../test_documents/pdf/regression_pages/{name}.pdf"));
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory SHA-bound source {}: {error}", path.display()));
    let actual_sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(actual_sha256, expected_sha256, "source identity");
    assert!(bytes.starts_with(b"%PDF-"), "source must be a PDF");
    let document = PdfDocument::from_bytes(bytes).expect("open mandatory source");
    assert_eq!(document.page_count().expect("source page count"), 1);
    let page = document
        .extract_page_text_with_script_geometry(0, ReadingOrder::TopToBottom)
        .expect("final preserved native spans");
    assert_eq!(page.spans.len(), expected_count, "source span census");
    page.spans
}

fn assert_source_relations(spans: &[TextSpan], anchors: &[(usize, usize, f32, f32, f32)]) {
    let before = metadata(spans);
    let actual = relations(spans);
    for &(base, script, base_x, script_x, baseline) in anchors {
        assert_eq!(spans[base].bbox.x, base_x, "base source anchor");
        assert_eq!(spans[script].bbox.x, script_x, "script source anchor");
        assert_eq!(spans[base].bbox.y, baseline, "logical row source anchor");
        assert_eq!(spans[base].font_size, 11.9552, "source base font");
        assert!(script < base, "preserved geometric order puts script first");
        assert!(
            actual.contains(&(base, script, baseline)),
            "missing exact source relation {base}/{script}: {actual:?}"
        );
        assert_eq!(actual.iter().filter(|edge| edge.1 == script).count(), 1);
    }
    assert!(actual.len() <= spans.len(), "bounded output cardinality");
    assert_eq!(metadata(spans), before, "all source glyph metadata stays exact");
}

#[test]
fn should_find_measured_raised_rows_with_both_input_orders_2275() {
    for (base_x, base_y, width, script_x, script_y, script_font) in [
        (96.413, 340.93997, 6.651874, 103.065, 345.27896, 7.9701),
        (215.573, 309.77197, 6.651874, 222.22499, 314.70798, 7.9701),
        (276.89917, 121.218, 5.3081093, 282.247, 126.155, 8.9664),
    ] {
        let base = span("x", base_x, base_y, width, 11.9552);
        let script = span("5", script_x, script_y, 4.0, script_font);
        let rise = (script_y - base_y) / base.font_size;
        assert!(rise > 0.35 && rise < 0.5, "measured .363/.413em witness");
        assert_eq!(relations(&[base.clone(), script.clone()]), [(0, 1, base_y)]);
        assert_eq!(relations(&[script, base]), [(1, 0, base_y)]);
    }
}

#[test]
fn should_choose_unique_nearest_right_edge_over_another_eligible_base_2275() {
    let spans = [
        span("8 ", 282.247, 126.155, 4.7332153, 8.9664),
        span(") =", 257.66324, 121.218, 16.59024, 11.9552),
        span("x", 276.89917, 121.218, 5.3081093, 11.9552),
    ];
    for base in &spans[1..] {
        let gap = spans[0].bbox.x - base.bbox.x - base.bbox.width;
        assert!((0.0..=base.font_size).contains(&gap), "both pass legacy X gate");
    }
    assert_eq!(relations(&spans), [(2, 0, 121.218)]);
}

#[test]
fn should_keep_trailing_space_and_all_glyph_metadata_read_only_2275() {
    let mut script = span("4 \u{2009}", 110.0, 704.936, 4.75, 8.0);
    script.char_x_offsets = vec![110.0, 114.0, 114.5];
    script.char_widths = vec![4.0, 0.5, 0.25];
    script.text_rise = 0.0;
    script.mcid = Some(7);
    script.is_italic = true;
    let spans = [script, span("x", 100.0, 700.0, 10.0, 12.0)];
    let before = metadata(&spans);
    assert_eq!(relations(&spans), [(1, 0, 700.0)]);
    assert_eq!(metadata(&spans), before);
    assert_eq!(spans[0].text, "4 \u{2009}");
}

#[test]
fn should_use_original_indices_across_many_fragments_and_reused_source_ids_2275() {
    let mut spans = vec![
        span("8 ", 110.0, 704.936, 4.75, 8.0),
        span("3", 210.0, 704.936, 4.0, 8.0),
    ];
    spans.extend((0..12).map(|_| span(" ", 400.0, 300.0, 2.0, 12.0)));
    spans.push(span("x", 100.0, 700.0, 10.0, 12.0));
    spans.push(span("x", 200.0, 700.0, 10.0, 12.0));
    assert!(spans.iter().all(|span| span.sequence == usize::MAX));
    assert_eq!(relations(&spans), [(14, 0, 700.0), (15, 1, 700.0)]);
}

#[test]
fn should_omit_equal_edge_and_float_near_tie_ambiguity_2275() {
    for right in [110.0, f32::from_bits(110.0_f32.to_bits() + 1)] {
        let spans = [
            span("x", 100.0, 700.0, 10.0, 12.0),
            span("y", 99.0, 700.0, right - 99.0, 12.0),
            span("2", 110.0, 704.936, 4.0, 8.0),
        ];
        assert_eq!(relations(&spans), [], "no arbitrary owner for competing right edges");
    }
}

#[test]
fn should_exclude_whitespace_only_base_even_when_its_edge_is_nearer_2275() {
    let spans = [
        span("x", 100.0, 700.0, 9.9, 12.0),
        span(" \t", 109.9, 700.0, 0.1, 12.0),
        span("2", 110.0, 704.936, 4.0, 8.0),
    ];
    assert_eq!(relations(&spans), [(0, 2, 700.0)]);
    assert_eq!(relations(&spans[1..]), []);
}

#[test]
fn should_not_assign_power_semantics_to_a_reference_digit_candidate_2275() {
    let spans = [
        span("sentence", 100.0, 700.0, 10.0, 12.0),
        span("1", 110.0, 704.0, 4.0, 8.0),
    ];
    assert_eq!(relations(&spans), [(0, 1, 700.0)]);
    assert_eq!(spans[1].text, "1", "evidence does not replace a reference glyph");
}

#[test]
fn should_return_no_self_relation_for_empty_or_singleton_input_2275() {
    assert_eq!(relations(&[]), []);
    assert_eq!(relations(&[span("2", 110.0, 704.0, 4.0, 8.0)]), []);
}

#[test]
fn should_reject_lowered_distant_wrapped_and_similar_size_runs_2275() {
    for script in [
        span("2", 110.0, 696.0, 4.0, 8.0),
        span("2", 123.0, 704.0, 4.0, 8.0),
        span("2", 106.0, 704.0, 4.0, 8.0),
        span("2", 110.0, 714.0, 4.0, 8.0),
        span("word", 110.0, 704.0, 4.0, 11.0),
    ] {
        assert_eq!(relations(&[span("x", 100.0, 700.0, 10.0, 12.0), script]), []);
    }
}

#[test]
fn should_not_cross_content_stream_scope_boundaries_2275() {
    for scope in [
        None,
        Some(McidScope::Page(1)),
        Some(McidScope::Form(ObjectRef::new(1, 0))),
        Some(McidScope::Pattern(ObjectRef::new(1, 0))),
    ] {
        let mut script = span("2", 110.0, 704.0, 4.0, 8.0);
        script.mcid_scope = scope;
        assert_eq!(relations(&[span("x", 100.0, 700.0, 10.0, 12.0), script]), []);
    }
}

#[test]
fn should_reject_nonhorizontal_and_incompatible_display_frames_2275() {
    let base = span("x", 100.0, 700.0, 10.0, 12.0);
    let script = span("2", 110.0, 704.0, 4.0, 8.0);
    for field in 0..5 {
        let mut pair = [base.clone(), script.clone()];
        match field {
            0 => pair[1].rotation_degrees = 90.0,
            1 => pair.iter_mut().for_each(|span| span.rotation_degrees = 90.0),
            2 => pair.iter_mut().for_each(|span| span.wmode = 1),
            3 => pair[1].mirrored = true,
            4 => pair[1].page_rotation_applied = 90,
            _ => unreachable!(),
        }
        assert_eq!(relations(&pair), [], "incompatible frame case {field}");
    }
}

#[test]
fn should_reject_nonfinite_and_nonpositive_geometry_on_either_endpoint_2275() {
    for endpoint in 0..2 {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for field in 0..7 {
                let mut spans = [span("x", 100.0, 700.0, 10.0, 12.0), span("2", 110.0, 704.0, 4.0, 8.0)];
                let target = &mut spans[endpoint];
                match field {
                    0 => target.bbox.x = value,
                    1 => target.bbox.y = value,
                    2 => target.bbox.width = value,
                    3 => target.bbox.height = value,
                    4 => target.font_size = value,
                    5 => target.rotation_degrees = value,
                    6 => target.text_rise = value,
                    _ => unreachable!(),
                }
                assert_eq!(relations(&spans), [], "endpoint {endpoint}, field {field}");
            }
        }
        for value in [0.0, -1.0] {
            let mut spans = [span("x", 100.0, 700.0, 10.0, 12.0), span("2", 110.0, 704.0, 4.0, 8.0)];
            spans[endpoint].font_size = value;
            assert_eq!(relations(&spans), []);
        }
    }
}

#[test]
fn should_omit_dense_overlapping_base_ambiguity_without_expanding_output_2275() {
    let mut spans = vec![span("x", 100.0, 700.0, 10.0, 12.0); 4096];
    spans.extend((0..4096).map(|_| span("2", 110.0, 704.0, 4.0, 8.0)));
    let actual = relations(&spans);
    assert_eq!(actual.len(), 0, "dense indistinguishable bases have no candidate");
    assert!(actual.len() <= spans.len(), "output is bounded by input count");
}

#[test]
fn should_find_source_ca18_closest_base_candidates_without_rewriting_spans_2275() {
    let spans = source_spans(
        "gutenberg-33283-p59",
        "26154b7fb99ba469af046603135b49f87ce26e8bf6b7b191bc30de66f558ca2c",
        98,
    );
    assert_eq!(spans[5].text, "5");
    assert_eq!(spans[11].text, "4");
    assert_eq!(spans[16].text, "3");
    assert_eq!(spans[17].text, "3");
    assert_eq!(spans[84].text, "2");
    assert_eq!(spans[85].text, "n");
    assert_source_relations(
        &spans,
        &[
            (9, 5, 96.413, 103.065, 340.93997),
            (14, 11, 215.573, 222.22499, 309.77197),
            (21, 16, 236.03697, 242.68896, 287.56998),
            (23, 17, 324.88397, 331.53598, 287.56998),
            (89, 84, 171.01398, 177.66597, 71.53),
            (91, 85, 200.39296, 213.18996, 71.53),
        ],
    );
}

#[test]
fn should_find_source_f489_trailing_space_and_distant_index_candidates_2275() {
    let spans = source_spans(
        "nist-fips-197-upd1-p16",
        "ddbb5a63987345c809b99487f583bc7eca2e9883131b38cc8e5fea59763ef33d",
        425,
    );
    assert_eq!(spans[389].text, "8 ");
    assert_eq!(spans[390].text, "4 ");
    assert_eq!(spans[391].text, "3");
    assert!(
        spans[390..401].len() > 8 && spans[391..405].len() > 8,
        "source base lies beyond eight preserved fragments"
    );
    assert_source_relations(
        &spans,
        &[
            (397, 389, 276.89917, 282.247, 121.218),
            (401, 390, 299.82925, 305.1822, 121.218),
            (405, 391, 322.7593, 328.11737, 121.218),
        ],
    );
}
