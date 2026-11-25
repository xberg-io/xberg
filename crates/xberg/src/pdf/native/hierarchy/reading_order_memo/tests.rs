use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;

use super::{ReadingOrderMemoScope, computed_orders, same_spans};
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
