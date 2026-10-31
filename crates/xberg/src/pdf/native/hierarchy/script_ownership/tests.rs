use std::path::Path;

use sha2::{Digest, Sha256};
use xberg_native_pdf::extractors::text::{ArtifactType, script_row_candidates};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::structure::McidScope;

use super::super::{
    ScriptGeometry, apply_rtl_structure_order, rejoin_inline_scripts_inner, reorder_page_reading_order,
};
use super::*;
use crate::pdf::native::NativeDocument;
use crate::pdf::native::text::{needs_numeric_script_boundary, numeric_notes};

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_name: "source-glyph-font".to_owned(),
        font_size,
        sequence: 0,
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn fingerprint(span: &TextSpan) -> (Vec<u8>, bool, i32) {
    (
        serde_json::to_vec(span).expect("serialize preserved glyph metadata"),
        span.mirrored,
        span.page_rotation_applied,
    )
}

fn fingerprints(spans: &[TextSpan]) -> Vec<(Vec<u8>, bool, i32)> {
    spans.iter().map(fingerprint).collect()
}

fn assert_native_candidate(spans: &[TextSpan], base: usize, script: usize) {
    assert!(
        script_row_candidates(spans)
            .iter()
            .any(|candidate| candidate.base_index() == base && candidate.script_index() == script),
        "native geometric candidate must exist before testing semantic exclusion"
    );
}

fn assert_owner(owners: &[NativeScriptOwner], spans: &[TextSpan], base: usize, script: usize, range: Range<usize>) {
    let expected = NativeScriptOwner {
        base_witness: NativeSpanWitness {
            original_index: base,
            snapshot: Arc::new(spans[base].clone()),
        },
        script_witness: NativeSpanWitness {
            original_index: script,
            snapshot: Arc::new(spans[script].clone()),
        },
        logical_baseline: spans[base].bbox.y,
        kind: AnnotationKind::Superscript,
        script_byte_range: range,
    };
    let matching: Vec<_> = owners
        .iter()
        .filter(|owner| owner.script_witness.original_index == script)
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "one validated suffix owner for script {script}: {owners:?}"
    );
    let owner = matching[0];
    assert_eq!(
        owner.base_witness.original_index, expected.base_witness.original_index,
        "exact base ordinal"
    );
    assert_eq!(owner.logical_baseline, expected.logical_baseline);
    assert_eq!(owner.kind, expected.kind);
    assert_eq!(owner.script_byte_range, expected.script_byte_range);
    assert_eq!(
        fingerprint(&owner.base_witness.snapshot),
        fingerprint(&expected.base_witness.snapshot)
    );
    assert_eq!(
        fingerprint(&owner.script_witness.snapshot),
        fingerprint(&expected.script_witness.snapshot)
    );
    assert!(spans[script].text.is_char_boundary(owner.script_byte_range.start));
    assert!(spans[script].text.is_char_boundary(owner.script_byte_range.end));
}

fn source_spans(name: &str, expected_sha256: &str, raw_count: usize) -> Vec<TextSpan> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../test_documents/pdf/regression_pages/{name}.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory source {}: {error}", path.display()));
    let actual_sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(actual_sha256, expected_sha256, "source identity");
    assert!(bytes.starts_with(b"%PDF-"), "mandatory source is a PDF");
    let document = NativeDocument::open_bytes(&bytes).expect("open SHA-bound source");
    assert_eq!(document.doc.page_count().expect("source page count"), 1);
    let mut page = document
        .doc
        .extract_page_text_with_script_geometry(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
        .expect("source preserves native script glyph geometry");
    assert_eq!(page.spans.len(), raw_count, "raw preserved span census");
    if !apply_rtl_structure_order(&document, 0, &mut page.spans) {
        reorder_page_reading_order(&mut page.spans, page.page_width, page.page_height, 0);
        xberg_native_pdf::document::reorder_ltr_key_value_rows(&mut page.spans);
    }
    rejoin_inline_scripts_inner(page.spans, ScriptGeometry::Preserve)
}

fn source_anchor(spans: &[TextSpan], text: &str, x: f32, y: f32, font_size: f32) -> usize {
    let matching: Vec<_> = spans
        .iter()
        .enumerate()
        .filter(|(_, span)| span.text == text && span.bbox.x == x && span.bbox.y == y && span.font_size == font_size)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(matching.len(), 1, "unique immutable source anchor {text:?} at {x}/{y}");
    matching[0]
}

fn note_definition(number: &str) -> [TextSpan; 2] {
    [
        span(number, 100.0, 20.0, 4.0, 6.0),
        span("A supporting note", 105.0, 20.0, 60.0, 8.0),
    ]
}

#[test]
fn should_validate_measured_suffix_rises_in_both_final_span_orders_2275() {
    for (base_x, base_y, width, script_x, script_y, script_font) in [
        (96.413, 340.93997, 6.651874, 103.065, 345.27896, 7.9701),
        (215.573, 309.77197, 6.651874, 222.22499, 314.70798, 7.9701),
        (276.89917, 121.218, 5.3081093, 282.247, 126.155, 8.9664),
    ] {
        let base = span("q", base_x, base_y, width, 11.9552);
        let script = span("2", script_x, script_y, 4.0, script_font);
        for (spans, base_index, script_index) in [
            ([base.clone(), script.clone()], 0, 1),
            ([script.clone(), base.clone()], 1, 0),
        ] {
            assert_native_candidate(&spans, base_index, script_index);
            let before = fingerprints(&spans);
            let owners = collect_suffix_script_owners(&spans);
            assert_owner(&owners, &spans, base_index, script_index, 0..1);
            assert_eq!(fingerprints(&spans), before);
        }
    }
}

#[test]
fn should_exclude_trailing_utf8_whitespace_from_range_without_rewriting_glyphs_2275() {
    let mut script = span("2 \u{2009}", 110.0, 704.936, 4.75, 8.0);
    script.char_x_offsets = vec![110.0, 114.0, 114.5];
    script.char_widths = vec![4.0, 0.5, 0.25];
    script.is_italic = true;
    let mut base = span("q", 100.0, 700.0, 10.0, 12.0);
    base.char_widths = vec![10.0];
    base.char_x_offsets = vec![100.0];
    let spans = [script, base];
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    assert_owner(&owners, &spans, 1, 0, 0..1);
    assert_eq!(fingerprints(&spans), before);
    assert_eq!(spans[0].text, "2 \u{2009}");
}

#[test]
fn should_keep_source_ordinals_and_snapshots_when_stream_sequence_ids_collide_2275() {
    let mut spans = vec![span("2", 110.0, 704.936, 4.0, 8.0), span("n", 210.0, 704.936, 4.0, 8.0)];
    spans.extend((0..12).map(|_| span(" ", 400.0, 300.0, 2.0, 12.0)));
    spans.push(span("q", 100.0, 700.0, 10.0, 12.0));
    spans.push(span("ab", 200.0, 700.0, 10.0, 12.0));
    assert!(spans.iter().all(|span| span.sequence == 0));
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    assert_eq!(owners.len(), 2);
    assert_owner(&owners, &spans, 14, 0, 0..1);
    assert_owner(&owners, &spans, 15, 1, 0..1);
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_reject_ordinary_multiword_or_oversized_small_runs_despite_row_evidence_2275() {
    for text in ["small note", "commentary", "2 3"] {
        let spans = [
            span("ordinary prose", 100.0, 700.0, 60.0, 12.0),
            span(text, 160.0, 704.0, 6.0, 8.0),
        ];
        assert_native_candidate(&spans, 0, 1);
        let before = fingerprints(&spans);
        assert!(collect_suffix_script_owners(&spans).is_empty());
        assert_eq!(fingerprints(&spans), before);
    }
}

#[test]
fn should_reject_internal_or_distant_runs_that_are_not_suffixes_2275() {
    for x in [109.0, 112.0] {
        let spans = [span("q", 100.0, 700.0, 10.0, 12.0), span("2", x, 704.0, 4.0, 8.0)];
        assert_native_candidate(&spans, 0, 1);
        assert!(collect_suffix_script_owners(&spans).is_empty());
    }
}

#[test]
fn should_omit_equal_edge_and_float_near_tie_owners_2275() {
    for right in [110.0, f32::from_bits(110.0_f32.to_bits() + 1)] {
        let spans = [
            span("q", 100.0, 700.0, 10.0, 12.0),
            span("r", 99.0, 700.0, right - 99.0, 12.0),
            span("2", 110.0, 704.0, 4.0, 8.0),
        ];
        assert!(
            script_row_candidates(&spans).is_empty(),
            "native candidate is ambiguous"
        );
        assert!(collect_suffix_script_owners(&spans).is_empty());
    }
}

#[test]
fn should_reject_scope_frame_role_and_monospace_conflicts_2275() {
    for case in 0..10 {
        let mut spans = [span("q", 100.0, 700.0, 10.0, 12.0), span("2", 110.0, 704.0, 4.0, 8.0)];
        match case {
            0 => spans[1].mcid_scope = Some(McidScope::Page(1)),
            1 => spans[1].rotation_degrees = 90.0,
            2 => spans[1].wmode = 1,
            3 => spans[1].mirrored = true,
            4 => spans[0].artifact_type = Some(ArtifactType::Layout),
            5 => spans[1].artifact_type = Some(ArtifactType::Layout),
            6 => spans[0].is_monospace = true,
            7 => spans[1].is_monospace = true,
            8 => spans[0].heading_level = Some(1),
            9 => spans[1].rtl_draw_logical = true,
            _ => unreachable!(),
        }
        let before = fingerprints(&spans);
        assert!(collect_suffix_script_owners(&spans).is_empty(), "conflict case {case}");
        assert_eq!(fingerprints(&spans), before);
    }
}

#[test]
fn should_reject_nonfinite_or_wrapped_ownership_2275() {
    for endpoint in 0..2 {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for field in 0..7 {
                let mut spans = [span("q", 100.0, 700.0, 10.0, 12.0), span("2", 110.0, 704.0, 4.0, 8.0)];
                let target = &mut spans[endpoint];
                match field {
                    0 => target.bbox.x = value,
                    1 => target.bbox.y = value,
                    2 => target.bbox.width = value,
                    3 => target.bbox.height = value,
                    4 => target.font_size = value,
                    5 => target.text_rise = value,
                    6 => target.rotation_degrees = value,
                    _ => unreachable!(),
                }
                assert!(collect_suffix_script_owners(&spans).is_empty());
            }
        }
    }
    let wrapped = [span("q", 100.0, 700.0, 10.0, 12.0), span("2", 110.0, 714.0, 4.0, 8.0)];
    assert!(collect_suffix_script_owners(&wrapped).is_empty());
}

#[test]
fn should_preserve_confirmed_numeric_footnote_separation_2275() {
    let mut spans = vec![
        span("Section 4", 100.0, 700.0, 60.0, 12.0),
        span("1", 160.1, 704.0, 4.0, 8.0),
    ];
    spans.extend(note_definition("1"));
    let notes = numeric_notes(&spans);
    assert!(notes.contains_key("1"), "independent numbered-note evidence");
    assert!(needs_numeric_script_boundary(&spans[0], &spans[1], &notes, None));
    assert_native_candidate(&spans, 0, 1);
    let before = fingerprints(&spans);
    assert!(collect_suffix_script_owners(&spans).is_empty());
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_keep_power_one_when_a_matching_note_exists_and_expression_continues_2275() {
    let mut spans = vec![
        span("The value is 2", 100.0, 700.0, 60.0, 12.0),
        span("1", 160.1, 704.0, 4.0, 8.0),
        span("+ 1", 165.0, 700.0, 20.0, 12.0),
    ];
    spans.extend(note_definition("1"));
    let notes = numeric_notes(&spans);
    assert!(notes.contains_key("1"));
    assert!(needs_numeric_script_boundary(&spans[0], &spans[1], &notes, None));
    assert!(!needs_numeric_script_boundary(
        &spans[0],
        &spans[1],
        &notes,
        Some(&spans[2])
    ));
    assert_native_candidate(&spans, 0, 1);
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    assert_owner(&owners, &spans, 0, 1, 0..1);
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_not_classify_an_inline_multiplication_operator_as_a_power_2275() {
    let spans = [
        span("The symbol ", 100.0, 700.0, 60.0, 12.0),
        span("·", 160.0, 701.0, 4.0, 8.0),
    ];
    assert_native_candidate(&spans, 0, 1);
    let before = fingerprints(&spans);
    assert!(collect_suffix_script_owners(&spans).is_empty());
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_keep_distinct_exact_witnesses_for_repeated_variable_and_power_text_2275() {
    let spans = [
        span("q", 100.0, 700.0, 10.0, 12.0),
        span("2", 110.0, 704.0, 4.0, 8.0),
        span("q", 300.0, 700.0, 10.0, 12.0),
        span("2", 310.0, 704.0, 4.0, 8.0),
    ];
    let owners = collect_suffix_script_owners(&spans);
    assert_eq!(owners.len(), 2);
    assert_owner(&owners, &spans, 0, 1, 0..1);
    assert_owner(&owners, &spans, 2, 3, 0..1);
}

#[test]
fn should_validate_final_source_ca18_power_owners_with_exact_witnesses_2275() {
    let spans = source_spans(
        "gutenberg-33283-p59",
        "26154b7fb99ba469af046603135b49f87ce26e8bf6b7b191bc30de66f558ca2c",
        98,
    );
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    for (base_text, power, bx, sx, by, sy) in [
        ("x", "5", 96.413, 103.065, 340.93997, 345.27896),
        ("x", "4", 215.573, 222.22499, 309.77197, 314.70798),
        ("x", "3", 236.03697, 242.68896, 287.56998, 292.50598),
        ("x", "3", 324.88397, 331.53598, 287.56998, 292.50598),
        ("x", "2", 171.01398, 177.66597, 71.53, 75.868996),
        ("ax", "n", 200.39296, 213.18996, 71.53, 75.868996),
    ] {
        let base = source_anchor(&spans, base_text, bx, by, 11.9552);
        let script = source_anchor(&spans, power, sx, sy, 7.9701);
        assert_native_candidate(&spans, base, script);
        assert_owner(&owners, &spans, base, script, 0..1);
    }
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_validate_final_source_f489_modulus_owners_and_keep_true_note_distinct_2275() {
    let spans = source_spans(
        "nist-fips-197-upd1-p16",
        "ddbb5a63987345c809b99487f583bc7eca2e9883131b38cc8e5fea59763ef33d",
        425,
    );
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    for (power, bx, sx) in [
        ("8 ", 276.89917, 282.247),
        ("4 ", 299.82925, 305.1822),
        ("3", 322.7593, 328.11737),
    ] {
        let base = source_anchor(&spans, "x", bx, 121.218, 11.9552);
        let script = source_anchor(&spans, power, sx, 126.155, 8.9664);
        assert_native_candidate(&spans, base, script);
        assert_owner(&owners, &spans, base, script, 0..1);
    }
    for note in [
        source_anchor(&spans, "1 ", 133.238, 654.548, 8.9664),
        source_anchor(&spans, "1", 77.4, 76.048, 7.3723),
    ] {
        assert!(!owners.iter().any(|owner| owner.script_witness.original_index == note));
    }
    let operator = source_anchor(&spans, "The symbol •", 77.029, 184.49, 11.9552);
    assert!(
        !owners
            .iter()
            .any(|owner| owner.script_witness.original_index == operator)
    );
    assert!(
        !owners
            .iter()
            .any(|owner| owner.script_witness.snapshot.text.contains('•'))
    );
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_reject_leading_or_internal_script_whitespace_2275() {
    for text in [" 2", "2 3", " "] {
        let spans = [span("q", 100.0, 700.0, 10.0, 12.0), span(text, 110.0, 704.0, 4.0, 8.0)];
        let before = fingerprints(&spans);
        assert!(collect_suffix_script_owners(&spans).is_empty());
        assert_eq!(fingerprints(&spans), before);
    }
}

#[test]
fn should_find_note_exception_operator_when_power_precedes_its_base_2275() {
    let mut spans = vec![
        span("1", 160.1, 704.0, 4.0, 8.0),
        span("The value is 2", 100.0, 700.0, 60.0, 12.0),
        span("+ 1", 165.0, 700.0, 20.0, 12.0),
    ];
    spans.extend(note_definition("1"));
    let notes = numeric_notes(&spans);
    assert!(notes.contains_key("1"));
    assert!(needs_numeric_script_boundary(&spans[1], &spans[0], &notes, None));
    assert!(!needs_numeric_script_boundary(
        &spans[1],
        &spans[0],
        &notes,
        Some(&spans[2])
    ));
    assert_native_candidate(&spans, 1, 0);
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    assert_owner(&owners, &spans, 1, 0, 0..1);
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_not_choose_between_ambiguous_equally_positioned_continuation_operators_2275() {
    let mut spans = vec![
        span("The value is 2", 100.0, 700.0, 60.0, 12.0),
        span("1", 160.1, 704.0, 4.0, 8.0),
        span("+ 1", 165.0, 700.0, 20.0, 12.0),
        span("- 1", 165.0, 700.0, 20.0, 12.0),
    ];
    spans.extend(note_definition("1"));
    let notes = numeric_notes(&spans);
    assert!(needs_numeric_script_boundary(&spans[0], &spans[1], &notes, None));
    for operator in &spans[2..4] {
        assert!(!needs_numeric_script_boundary(
            &spans[0],
            &spans[1],
            &notes,
            Some(operator)
        ));
    }
    assert_native_candidate(&spans, 0, 1);
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    assert!(!owners.iter().any(|owner| owner.script_witness.original_index == 1));
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_not_use_an_operator_from_a_different_stream_scope_as_note_exception_2275() {
    let mut spans = vec![
        span("The value is 2", 100.0, 700.0, 60.0, 12.0),
        span("1", 160.1, 704.0, 4.0, 8.0),
        span("+ 1", 165.0, 700.0, 20.0, 12.0),
    ];
    spans[2].mcid_scope = Some(McidScope::Page(1));
    spans.extend(note_definition("1"));
    let notes = numeric_notes(&spans);
    assert!(needs_numeric_script_boundary(&spans[0], &spans[1], &notes, None));
    assert!(!needs_numeric_script_boundary(
        &spans[0],
        &spans[1],
        &notes,
        Some(&spans[2])
    ));
    assert_native_candidate(&spans, 0, 1);
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    assert!(!owners.iter().any(|owner| owner.script_witness.original_index == 1));
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_not_use_a_wrapped_operator_as_note_exception_2275() {
    let mut spans = vec![
        span("The value is 2", 100.0, 700.0, 60.0, 12.0),
        span("1", 160.1, 704.0, 4.0, 8.0),
        span("+ 1", 165.0, 686.0, 20.0, 12.0),
    ];
    spans.extend(note_definition("1"));
    let notes = numeric_notes(&spans);
    assert!(needs_numeric_script_boundary(&spans[0], &spans[1], &notes, None));
    assert!(needs_numeric_script_boundary(
        &spans[0],
        &spans[1],
        &notes,
        Some(&spans[2])
    ));
    assert_native_candidate(&spans, 0, 1);
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    assert!(!owners.iter().any(|owner| owner.script_witness.original_index == 1));
    assert_eq!(fingerprints(&spans), before);
}

#[test]
fn should_omit_cross_binade_ulp_tied_continuation_operators_2275() {
    let operator_x = 16.0_f32;
    let adjacent_x = f32::from_bits(operator_x.to_bits() + 1);
    let mut spans = vec![
        span("The value is 2", -56.0, 700.0, 60.0, 12.0),
        span("1", 4.0, 704.0, 4.0, 8.0),
        span("+ 1", operator_x, 700.0, 20.0, 12.0),
        span("- 1", adjacent_x, 700.0, 20.0, 12.0),
    ];
    spans.extend(note_definition("1"));
    assert_eq!(spans[1].bbox.right(), 8.0);
    assert!(adjacent_x - operator_x > f32::EPSILON * spans[1].bbox.right());
    assert!(adjacent_x - operator_x <= f32::EPSILON * adjacent_x);
    let notes = numeric_notes(&spans);
    assert!(notes.contains_key("1"));
    assert!(needs_numeric_script_boundary(&spans[0], &spans[1], &notes, None));
    for operator in &spans[2..4] {
        assert!(!needs_numeric_script_boundary(
            &spans[0],
            &spans[1],
            &notes,
            Some(operator)
        ));
    }
    assert_native_candidate(&spans, 0, 1);
    let before = fingerprints(&spans);
    let owners = collect_suffix_script_owners(&spans);
    assert!(!owners.iter().any(|owner| owner.script_witness.original_index == 1));
    assert_eq!(fingerprints(&spans), before);
}
