use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::assembly::extract_text_and_annotations;
use crate::pdf::structure::types::{PdfLine, PdfParagraph};
use crate::types::document_structure::AnnotationKind;

use super::*;

fn fixture(script_first: bool) -> (Vec<SegmentData>, NativeSegmentEvidence) {
    let span = |text: &str, x, y, width, font_size| TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_size,
        font_name: "immutable-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    };
    let prefix = span("é = ", 80.0, 700.0, 20.0, 12.0);
    let base = span("x", 100.0, 700.0, 10.0, 12.0);
    let script = span("4 ", 110.0, 704.9568, 4.0, 8.0);
    let suffix = span("/l", 114.0, 700.0, 8.0, 12.0);
    let spans = if script_first {
        vec![script, prefix, base, suffix]
    } else {
        vec![prefix, base, script, suffix]
    };
    let page = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let segments = page.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(segments, page, &[]);
    let evidence = evidence.expect("real native source evidence");
    assert_eq!(
        evidence.owners.len(),
        1,
        "source predicate must establish this measured-rise suffix"
    );
    (segments, evidence)
}

fn fingerprint(segment: &SegmentData) -> (String, [u32; 7], [bool; 3], Option<u8>) {
    (
        segment.text.clone(),
        [
            segment.x,
            segment.y,
            segment.width,
            segment.height,
            segment.font_size,
            segment.baseline_y,
            segment.rotation_degrees,
        ]
        .map(f32::to_bits),
        [segment.is_bold, segment.is_italic, segment.is_monospace],
        segment.assigned_role,
    )
}

fn line_with_proof() -> PdfLine {
    let (segments, evidence) = fixture(false);
    PdfLine {
        segments,
        native_scripts: evidence.owners,
        baseline_y: 700.0,
        dominant_font_size: 12.0,
        is_bold: false,
        is_monospace: false,
    }
}

fn paragraph(line: PdfLine) -> PdfParagraph {
    PdfParagraph {
        text: String::new(),
        lines: vec![line],
        dominant_font_size: 12.0,
        heading_level: None,
        is_bold: false,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: None,
        word_count: 3,
    }
}

#[test]
fn should_put_proven_point413_em_suffix_on_its_owner_line_in_both_source_orders_2275() {
    for script_first in [false, true] {
        let (segments, evidence) = fixture(script_first);
        let refs: Vec<_> = segments.iter().collect();
        let lines = reconstruct_native_lines(&refs, &evidence);
        assert_eq!(
            lines.len(),
            1,
            "logical row proof must supersede the ordinary three-point line cutoff"
        );
        assert_eq!(
            lines[0]
                .segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>(),
            ["é = ", "x", "4 ", "/l"]
        );
        assert_eq!(lines[0].native_scripts.len(), 1);
        let owner = &lines[0].native_scripts[0];
        assert_eq!((owner.base_index, owner.script_index), (1, 2));
        assert_eq!(owner.kind(), AnnotationKind::Superscript);
        assert_eq!(owner.byte_range(), 0..1);
        assert_eq!(owner.logical_baseline(), 700.0);
    }
}

#[test]
fn should_leave_every_source_projection_field_unchanged_during_logical_line_reconstruction_2275() {
    let (segments, evidence) = fixture(true);
    let refs: Vec<_> = segments.iter().collect();
    let lines = reconstruct_native_lines(&refs, &evidence);
    let mut expected: Vec<_> = segments.iter().map(fingerprint).collect();
    let mut actual: Vec<_> = lines.iter().flat_map(|line| &line.segments).map(fingerprint).collect();
    expected.sort();
    actual.sort();
    assert_eq!(actual, expected);
    assert!(
        lines
            .iter()
            .flat_map(|line| &line.segments)
            .any(|segment| segment.text == "4 " && segment.baseline_y == 704.9568)
    );
}

#[test]
fn should_shift_line_local_attachment_indices_when_prepend_inserts_a_marker_2275() {
    let mut line = line_with_proof();
    let original = line.native_scripts[0].clone();
    let mut marker = line.segments[0].clone();
    marker.text = "• ".to_owned();
    marker.x = 60.0;
    line.prepend_segment(marker);
    assert_eq!(line.native_scripts.len(), 1);
    let shifted = &line.native_scripts[0];
    assert_eq!(
        (shifted.base_index, shifted.script_index),
        (original.base_index + 1, original.script_index + 1)
    );
    assert_eq!(line.segments[shifted.base_index].text, "x");
    assert_eq!(line.segments[shifted.script_index].text, "4 ");
    assert_eq!(shifted.byte_range(), original.byte_range());
}

#[test]
fn should_drop_stale_line_proof_after_any_endpoint_projection_field_changes_2275() {
    for (endpoint, field) in (0..2).flat_map(|endpoint| (0..12).map(move |field| (endpoint, field))) {
        let mut line = line_with_proof();
        assert_eq!(line.valid_native_scripts().len(), 1, "unmodified source proof is valid");
        let index = if endpoint == 0 {
            line.native_scripts[0].base_index
        } else {
            line.native_scripts[0].script_index
        };
        let segment = &mut line.segments[index];
        match field {
            0 => segment.text.push('!'),
            1 => segment.x += 1.0,
            2 => segment.y += 1.0,
            3 => segment.width += 1.0,
            4 => segment.height += 1.0,
            5 => segment.font_size += 1.0,
            6 => segment.baseline_y += 1.0,
            7 => segment.rotation_degrees += 1.0,
            8 => segment.is_bold = !segment.is_bold,
            9 => segment.is_italic = !segment.is_italic,
            10 => segment.is_monospace = !segment.is_monospace,
            11 => segment.assigned_role = Some(2),
            _ => unreachable!(),
        }
        assert!(
            line.valid_native_scripts().is_empty(),
            "stale endpoint {endpoint}, field {field}"
        );
    }
}

#[test]
fn should_emit_native_proof_annotations_at_utf8_byte_offsets_without_trailing_script_space_2275() {
    let line = line_with_proof();
    let stored_script = line.segments[2].clone();
    let (text, annotations) = extract_text_and_annotations(&paragraph(line), &Default::default());
    let scripts: Vec<_> = annotations
        .iter()
        .filter(|annotation| annotation.kind == AnnotationKind::Superscript)
        .collect();
    assert_eq!(
        scripts.len(),
        1,
        "proof must survive the existing point35-em heuristic ceiling"
    );
    assert_eq!(text, "é = x4 /l");
    assert_eq!((scripts[0].start, scripts[0].end), (6, 7));
    assert_eq!(&text[scripts[0].start as usize..scripts[0].end as usize], "4");
    assert_eq!(stored_script.text, "4 ");
    assert_eq!(stored_script.baseline_y, 704.9568);
}

#[test]
fn should_preserve_attachment_local_indices_when_complete_lines_are_cloned_and_merged_2275() {
    let first = line_with_proof();
    let second = first.clone();
    let mut lines = vec![first];
    lines.extend(vec![second]);
    assert_eq!(lines.len(), 2);
    for line in lines {
        assert_eq!(line.valid_native_scripts().len(), 1);
        assert_eq!(
            (line.native_scripts[0].base_index, line.native_scripts[0].script_index),
            (1, 2)
        );
    }
}

fn rotated_row_segment(text: &str, advance: f32, baseline: f32, height: f32, role: Option<u8>) -> SegmentData {
    let rotation_degrees = 90.0_f32;
    let (sin, cos) = rotation_degrees.to_radians().sin_cos();
    let x = advance * cos - baseline * sin;
    let y = advance * sin + baseline * cos;
    SegmentData {
        text: text.to_owned(),
        x,
        y,
        width: 8.0,
        height,
        font_size: 12.0,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y: y,
        rotation_degrees,
        assigned_role: role,
    }
}

fn ownerless_rotated_page() -> Vec<SegmentData> {
    vec![
        rotated_row_segment("A", 0.0, 100.0, 40.0, Some(1)),
        rotated_row_segment("B", 20.0, 96.0, 12.0, None),
        rotated_row_segment("C", 10.0, 88.0, 12.0, None),
    ]
}

#[test]
fn should_preserve_ownerless_rotated_paragraph_order_after_global_row_anchor_is_removed_2275() {
    let (ordered, index) = order_native_segments(
        ownerless_rotated_page(),
        NativeSegmentEvidence {
            original_indices: vec![0, 1, 2],
            owners: Vec::new(),
        },
    );
    assert_eq!(
        ordered.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>(),
        ["A", "C", "B"],
        "the tall global row anchor must make this a causal non-idempotent ordering case"
    );
    let legacy = super::super::blocks_to_paragraphs(ordered.clone(), &[], &[]);
    let actual = super::super::blocks_to_paragraphs_with_native_scripts(ordered, &[], &[], Some(&index));
    assert_eq!(
        legacy.len(),
        2,
        "heading role removes the tall anchor from the body paragraph"
    );
    assert_eq!(actual.len(), legacy.len());
    let legacy_body = legacy
        .iter()
        .find(|paragraph| paragraph.heading_level.is_none())
        .unwrap();
    let actual_body = actual
        .iter()
        .find(|paragraph| paragraph.heading_level.is_none())
        .unwrap();
    assert_eq!(
        legacy_body
            .lines
            .iter()
            .flat_map(|line| &line.segments)
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>(),
        ["C", "B"]
    );
    assert_eq!(
        actual_body
            .lines
            .iter()
            .flat_map(|line| &line.segments)
            .map(fingerprint)
            .collect::<Vec<_>>(),
        legacy_body
            .lines
            .iter()
            .flat_map(|line| &line.segments)
            .map(fingerprint)
            .collect::<Vec<_>>(),
        "ownerless paragraph reconstruction must preserve the existing global reading order"
    );
}

#[test]
fn should_keep_legacy_rotated_subset_order_when_all_supplied_script_witnesses_are_invalid_2275() {
    let ordered = super::super::order_segments_in_reading_frames(ownerless_rotated_page());
    assert_eq!(
        ordered.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>(),
        ["A", "C", "B"]
    );
    let body: Vec<_> = ordered
        .iter()
        .filter(|segment| segment.assigned_role.is_none())
        .collect();
    assert_eq!(
        body.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>(),
        ["C", "B"]
    );
    let (_, mut evidence) = fixture(false);
    let mut owner = evidence.owners.remove(0);
    owner.base_index = 1;
    owner.script_index = 0;
    let evidence = NativeSegmentEvidence {
        original_indices: vec![2, 1],
        owners: vec![owner],
    };
    assert_eq!(evidence.owners.len(), 1, "exercise nonempty supplied evidence");
    assert!(
        valid_owners(&body, &evidence).is_empty(),
        "rotated/current glyphs do not match the source witness"
    );
    let legacy = super::super::reconstruct_pdf_lines(&body);
    let actual = reconstruct_native_lines(&body, &evidence);
    assert_eq!(
        actual
            .iter()
            .flat_map(|line| &line.segments)
            .map(fingerprint)
            .collect::<Vec<_>>(),
        legacy
            .iter()
            .flat_map(|line| &line.segments)
            .map(fingerprint)
            .collect::<Vec<_>>(),
        "the fallback depends on valid owners, not merely a nonempty evidence vector"
    );
}
