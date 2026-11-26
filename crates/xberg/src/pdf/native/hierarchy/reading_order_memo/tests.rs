use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;

use super::{ReadingOrderMemoScope, computed_orders, fingerprint, output_positions, stored_positions};

fn same_spans(left: &[TextSpan], right: &[TextSpan]) -> bool {
    fingerprint(left) == fingerprint(right)
}
use crate::pdf::native::NativeDocument;
use crate::pdf::native::hierarchy::{ArtifactRetention, extract_all_segments_for_tables, reorder_page_reading_order};

const PAGE_WIDTH: f32 = 612.0;
const PAGE_HEIGHT: f32 = 792.0;

/// Two prose columns written row by row, so the ordering has columns to separate.
fn two_column_page() -> Vec<TextSpan> {
    let mut spans = Vec::new();
    for row in 0..12 {
        let y = 700.0 - row as f32 * 14.0;
        for (column, x) in [(0, 72.0), (1, 320.0)] {
            spans.push(TextSpan {
                text: format!("Column {column} carries prose line number {row} here"),
                bbox: Rect::new(x, y, 220.0, 12.0),
                font_size: 12.0,
                sequence: spans.len(),
                ..TextSpan::default()
            });
        }
    }
    spans
}

fn fresh_order(spans: &[TextSpan], page_width: f32) -> Vec<TextSpan> {
    let mut ordered = spans.to_vec();
    super::super::compute_page_reading_order(&mut ordered, page_width, PAGE_HEIGHT, 0);
    ordered
}

fn ordered(spans: &[TextSpan], page_width: f32) -> Vec<TextSpan> {
    let mut ordered = spans.to_vec();
    reorder_page_reading_order(&mut ordered, page_width, PAGE_HEIGHT, 0);
    ordered
}

#[test]
fn an_identical_page_is_ordered_once_and_served_its_fresh_order() {
    let spans = two_column_page();
    let fresh = fresh_order(&spans, PAGE_WIDTH);
    let _memo = ReadingOrderMemoScope::begin(usize::MAX);
    let before = computed_orders();

    let first = ordered(&spans, PAGE_WIDTH);
    let second = ordered(&spans, PAGE_WIDTH);

    assert_eq!(
        computed_orders() - before,
        1,
        "the second identical page must be served"
    );
    assert!(same_spans(&first, &fresh), "the computed order must match a fresh one");
    assert!(same_spans(&second, &fresh), "the served order must match a fresh one");
}

#[test]
fn a_changed_span_or_page_size_is_ordered_again() {
    let spans = two_column_page();
    let mut moved = spans.clone();
    moved[3].bbox.x += 0.001;
    let _memo = ReadingOrderMemoScope::begin(usize::MAX);
    let before = computed_orders();

    ordered(&spans, PAGE_WIDTH);
    let after_move = ordered(&moved, PAGE_WIDTH);
    let after_resize = ordered(&moved, PAGE_WIDTH + 1.0);

    assert_eq!(computed_orders() - before, 3, "every differing input must be ordered");
    assert!(same_spans(&after_move, &fresh_order(&moved, PAGE_WIDTH)));
    assert!(same_spans(&after_resize, &fresh_order(&moved, PAGE_WIDTH + 1.0)));
}

#[test]
fn nothing_is_kept_outside_a_scope_or_past_the_budget() {
    let spans = two_column_page();
    let before = computed_orders();
    ordered(&spans, PAGE_WIDTH);
    ordered(&spans, PAGE_WIDTH);
    assert_eq!(computed_orders() - before, 2, "no scope keeps no order");

    {
        let _memo = ReadingOrderMemoScope::begin(0);
        let before = computed_orders();
        ordered(&spans, PAGE_WIDTH);
        ordered(&spans, PAGE_WIDTH);
        assert_eq!(computed_orders() - before, 2, "a zero budget keeps no order");
    }

    {
        let _memo = ReadingOrderMemoScope::begin(usize::MAX);
        ordered(&spans, PAGE_WIDTH);
    }
    let before = computed_orders();
    ordered(&spans, PAGE_WIDTH);
    assert_eq!(computed_orders() - before, 1, "an ended scope keeps no order");
}

type Mutation = (&'static str, Box<dyn Fn(&mut TextSpan)>);

/// One mutation per `TextSpan` field, each changing only that field of the base span the fingerprint test builds.
fn field_mutations() -> Vec<Mutation> {
    vec![
        ("text", Box::new(|s: &mut TextSpan| s.text.push('x'))),
        ("bbox.x", Box::new(|s: &mut TextSpan| s.bbox.x += 0.5)),
        ("bbox.y", Box::new(|s: &mut TextSpan| s.bbox.y += 0.5)),
        ("bbox.width", Box::new(|s: &mut TextSpan| s.bbox.width += 0.5)),
        ("bbox.height", Box::new(|s: &mut TextSpan| s.bbox.height += 0.5)),
        (
            "font_name",
            Box::new(|s: &mut TextSpan| s.font_name = format!("{}x", s.font_name).into()),
        ),
        ("font_size", Box::new(|s: &mut TextSpan| s.font_size += 1.0)),
        (
            "font_weight",
            Box::new(|s: &mut TextSpan| s.font_weight = xberg_native_pdf::layout::FontWeight::Bold),
        ),
        ("is_italic", Box::new(|s: &mut TextSpan| s.is_italic = !s.is_italic)),
        (
            "is_monospace",
            Box::new(|s: &mut TextSpan| s.is_monospace = !s.is_monospace),
        ),
        ("color.r", Box::new(|s: &mut TextSpan| s.color.r += 0.5)),
        ("color.g", Box::new(|s: &mut TextSpan| s.color.g += 0.5)),
        ("color.b", Box::new(|s: &mut TextSpan| s.color.b += 0.5)),
        ("mcid", Box::new(|s: &mut TextSpan| s.mcid = Some(5))),
        ("mcid_none", Box::new(|s: &mut TextSpan| s.mcid = None)),
        (
            "mcid_scope",
            Box::new(|s: &mut TextSpan| s.mcid_scope = Some(xberg_native_pdf::structure::McidScope::Page(1))),
        ),
        ("sequence", Box::new(|s: &mut TextSpan| s.sequence += 1)),
        (
            "split_boundary_before",
            Box::new(|s: &mut TextSpan| s.split_boundary_before = !s.split_boundary_before),
        ),
        (
            "offset_semantic",
            Box::new(|s: &mut TextSpan| s.offset_semantic = !s.offset_semantic),
        ),
        ("char_spacing", Box::new(|s: &mut TextSpan| s.char_spacing += 1.0)),
        ("word_spacing", Box::new(|s: &mut TextSpan| s.word_spacing += 1.0)),
        (
            "horizontal_scaling",
            Box::new(|s: &mut TextSpan| s.horizontal_scaling += 1.0),
        ),
        (
            "primary_detected",
            Box::new(|s: &mut TextSpan| s.primary_detected = !s.primary_detected),
        ),
        ("char_widths", Box::new(|s: &mut TextSpan| s.char_widths[1] = 3.0)),
        ("char_widths_len", Box::new(|s: &mut TextSpan| s.char_widths.push(3.0))),
        ("char_x_offsets", Box::new(|s: &mut TextSpan| s.char_x_offsets[1] = 2.0)),
        ("heading_level", Box::new(|s: &mut TextSpan| s.heading_level = Some(3))),
        (
            "rotation_degrees",
            Box::new(|s: &mut TextSpan| s.rotation_degrees += 90.0),
        ),
        ("wmode", Box::new(|s: &mut TextSpan| s.wmode = 1)),
        ("text_rise", Box::new(|s: &mut TextSpan| s.text_rise += 1.0)),
        (
            "rtl_draw_logical",
            Box::new(|s: &mut TextSpan| s.rtl_draw_logical = !s.rtl_draw_logical),
        ),
        ("mirrored", Box::new(|s: &mut TextSpan| s.mirrored = !s.mirrored)),
        (
            "page_rotation_applied",
            Box::new(|s: &mut TextSpan| s.page_rotation_applied = 90),
        ),
        (
            "provenance",
            Box::new(|s: &mut TextSpan| s.provenance = Some(xberg_native_pdf::fonts::MappingProvenance::ToUnicode)),
        ),
        (
            "ligature_glyph",
            Box::new(|s: &mut TextSpan| s.ligature_glyph = !s.ligature_glyph),
        ),
    ]
}

#[test]
fn equal_spans_share_a_fingerprint_and_any_changed_field_breaks_it() {
    let base = TextSpan {
        text: "alpha".into(),
        font_name: "Helvetica".into(),
        char_widths: vec![1.0, 2.0],
        char_x_offsets: vec![0.0, 1.0],
        mcid: Some(4),
        heading_level: Some(2),
        ..TextSpan::default()
    };
    let reference = fingerprint(std::slice::from_ref(&base));
    assert_eq!(
        reference,
        fingerprint(std::slice::from_ref(&base)),
        "equal spans must match"
    );

    let mutations = field_mutations();
    for (field, mutate) in &mutations {
        let mut changed = base.clone();
        mutate(&mut changed);
        assert_ne!(
            reference,
            fingerprint(&[changed]),
            "changing {field} must change the fingerprint"
        );
    }

    let negative_zero = TextSpan {
        text: "alpha".into(),
        font_size: -0.0,
        ..TextSpan::default()
    };
    let positive_zero = TextSpan {
        font_size: 0.0,
        ..negative_zero.clone()
    };
    assert_ne!(
        fingerprint(&[negative_zero]),
        fingerprint(&[positive_zero]),
        "floats compare by bit pattern"
    );
    assert_ne!(
        fingerprint(std::slice::from_ref(&base)),
        fingerprint(&[base.clone(), base]),
        "the span count is part of the fingerprint"
    );
}

/// A two-page untagged 12pt Helvetica PDF of plain prose lines, with no raised marks.
fn plain_two_page_pdf() -> Vec<u8> {
    let page_stream = |page: usize| {
        let mut stream = String::new();
        for line in 0..6 {
            let y = 720.0 - line as f32 * 16.0;
            stream.push_str(&format!(
                "BT /F1 12 Tf 72 {y:.1} Td (Page {page} line {line} reads as plain prose) Tj ET\n"
            ));
        }
        stream
    };
    let (first, second) = (page_stream(1), page_stream(2));
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R \
         /Resources << /Font << /F1 7 0 R >> >> >>"
            .to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 6 0 R \
         /Resources << /Font << /F1 7 0 R >> >> >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{first}endstream", first.len()),
        format!("<< /Length {} >>\nstream\n{second}endstream", second.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

#[test]
fn each_page_is_ordered_once_across_the_table_and_structure_passes() {
    let bytes = plain_two_page_pdf();
    let artifacts = ArtifactRetention::default();
    let mut fresh_doc = NativeDocument::open_bytes(&bytes).expect("fixture must parse");
    let fresh_tables = extract_all_segments_for_tables(&mut fresh_doc, artifacts, |_| false).expect("table segments");
    let fresh_structure = super::super::script_evidence::extract_all_segment_carriers(&mut fresh_doc, artifacts)
        .expect("structure segments");

    let mut doc = NativeDocument::open_bytes(&bytes).expect("fixture must parse");
    let memo = ReadingOrderMemoScope::begin(bytes.len());
    let before = computed_orders();
    let tables = extract_all_segments_for_tables(&mut doc, artifacts, |_| false).expect("table segments");
    let structure =
        super::super::script_evidence::extract_all_segment_carriers(&mut doc, artifacts).expect("structure segments");
    let computed = computed_orders() - before;
    drop(memo);

    assert_eq!(tables.0.len(), 2, "one table segment list per page");
    assert!(
        tables.0.iter().all(|page| !page.is_empty()),
        "both pages must carry text, got {:?}",
        tables.0
    );
    assert_eq!(computed, 2, "each page must be ordered once across both passes");
    assert_eq!(format!("{tables:?}"), format!("{fresh_tables:?}"));
    assert_eq!(format!("{structure:?}"), format!("{fresh_structure:?}"));
}

#[test]
fn a_rearranging_order_is_kept_as_positions_and_served_identically() {
    let spans = two_column_page();
    let fresh = fresh_order(&spans, PAGE_WIDTH);
    assert!(
        !same_spans(&spans, &fresh),
        "the fixture must need reordering, or the test proves nothing"
    );
    let _memo = ReadingOrderMemoScope::begin(usize::MAX);
    let before = computed_orders();

    let first = ordered(&spans, PAGE_WIDTH);
    assert_eq!(
        stored_positions(0),
        Some(true),
        "a pure rearrangement must be kept as positions"
    );
    let second = ordered(&spans, PAGE_WIDTH);

    assert_eq!(
        computed_orders() - before,
        1,
        "the second identical page must be served"
    );
    assert!(same_spans(&first, &fresh));
    assert!(
        same_spans(&second, &fresh),
        "the positions must rebuild the fresh order exactly"
    );
}

#[test]
fn positions_name_where_each_input_went_and_refuse_anything_but_a_rearrangement() {
    assert_eq!(output_positions(&[10, 20, 30], &[30, 10, 20]), Some(vec![1, 2, 0]));
    assert_eq!(
        output_positions(&[7, 7, 9], &[9, 7, 7]),
        Some(vec![1, 2, 0]),
        "equal spans keep their relative order"
    );
    assert_eq!(
        output_positions(&[1, 2], &[1, 3]),
        None,
        "a changed span is not a rearrangement"
    );
    assert_eq!(
        output_positions(&[1, 2], &[1, 1]),
        None,
        "a duplicated span is not a rearrangement"
    );
    assert_eq!(
        output_positions(&[1, 2], &[1]),
        None,
        "a dropped span is not a rearrangement"
    );
}
