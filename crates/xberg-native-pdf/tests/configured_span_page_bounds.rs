use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::SpanMergingConfig;
use xberg_native_pdf::layout::TextSpan;

const PAGE_BOX: &str = "/MediaBox [0 0 200 200]";
const PAYLOAD_LENGTH: usize = 280;

fn pdf_with_page_entries(content: &str, page_entries: &str, parent_entries: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!("<< /Type /Pages /Kids [3 0 R] /Count 1 {parent_entries} >>"),
        format!(
            "<< /Type /Page /Parent 2 0 R {page_entries} /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> >> >>"
        ),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
    ];
    let mut pdf = b"%PDF-1.5\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn document_with_text(text: &str, x: f32) -> PdfDocument {
    let content = format!("BT /F1 1 Tf {x} 100 Td ({text}) Tj ET");
    let document = PdfDocument::from_bytes(pdf_with_page_entries(&content, PAGE_BOX, "")).expect("valid PDF");
    assert_eq!(document.page_count().expect("page count"), 1);
    document
}

fn payload() -> String {
    let text = "Qazvexlunorpti".repeat(20);
    assert_eq!(text.len(), PAYLOAD_LENGTH);
    text
}

fn span_texts(spans: Vec<TextSpan>) -> Vec<String> {
    spans.into_iter().map(|span| span.text).collect()
}

fn configured_texts(document: &PdfDocument, config: SpanMergingConfig) -> Vec<String> {
    span_texts(document.extract_spans_with_config(0, config).expect("configured spans"))
}

fn assert_raw_payload(document: &PdfDocument, expected: &str, x: f32) {
    let chars = document.extract_chars(0).expect("raw characters");
    assert_eq!(chars.len(), expected.len());
    assert_eq!(
        chars.iter().map(|character| character.char).collect::<String>(),
        expected
    );
    assert_eq!(chars.first().expect("nonempty payload").bbox.x, x);
}

#[test]
fn default_config_drops_offpage_spans_but_raw_chars_keep_the_exact_payload() {
    let text = payload();
    let document = document_with_text(&text, 500.0);
    assert_raw_payload(&document, &text, 500.0);
    assert_eq!(
        span_texts(document.extract_spans(0).expect("ordinary spans")),
        Vec::<String>::new()
    );
    assert_eq!(
        configured_texts(&document, SpanMergingConfig::default()),
        Vec::<String>::new()
    );
    assert_raw_payload(&document, &text, 500.0);
}

#[test]
fn default_config_keeps_the_exact_onpage_payload() {
    let text = payload();
    let document = document_with_text(&text, 10.0);
    assert_raw_payload(&document, &text, 10.0);
    assert_eq!(
        span_texts(document.extract_spans(0).expect("ordinary spans")),
        vec![text.clone()]
    );
    assert_eq!(
        configured_texts(&document, SpanMergingConfig::default()),
        vec![text.clone()]
    );
    assert_raw_payload(&document, &text, 10.0);
}

#[test]
fn page_membership_is_independent_of_cache_warm_order_and_merging_config() {
    for x in [10.0, 500.0] {
        for configured_first in [false, true] {
            for merge_tm_tj_runs in [false, true] {
                let document = document_with_text("Qazvex", x);
                let config = SpanMergingConfig {
                    merge_tm_tj_runs,
                    ..SpanMergingConfig::default()
                };
                let expected = if x < 200.0 {
                    vec!["Qazvex".to_string()]
                } else {
                    Vec::new()
                };
                if configured_first {
                    assert_eq!(configured_texts(&document, config.clone()), expected);
                } else {
                    assert_raw_payload(&document, "Qazvex", x);
                    assert_eq!(span_texts(document.extract_spans(0).expect("ordinary first")), expected);
                }
                assert_eq!(span_texts(document.extract_spans(0).expect("ordinary spans")), expected);
                assert_eq!(configured_texts(&document, config), expected);
                assert_raw_payload(&document, "Qazvex", x);
            }
        }
    }
}

#[test]
fn configured_spans_keep_partial_overlap_and_two_point_edge_tolerance() {
    for (x, keep) in [(199.75, true), (201.0, true), (202.0, false), (203.0, false)] {
        for merge_tm_tj_runs in [false, true] {
            let document = document_with_text("M", x);
            let expected = if keep { vec!["M".to_string()] } else { Vec::new() };
            assert_raw_payload(&document, "M", x);
            assert_eq!(
                span_texts(document.extract_spans(0).expect("ordinary spans")),
                expected,
                "x={x}"
            );
            let config = SpanMergingConfig {
                merge_tm_tj_runs,
                ..SpanMergingConfig::default()
            };
            assert_eq!(
                configured_texts(&document, config),
                expected,
                "x={x}, merge_tm_tj_runs={merge_tm_tj_runs}"
            );
        }
    }
}

#[test]
fn custom_merging_changes_segmentation_after_the_ordinary_cache_is_warm() {
    let content = "BT /F1 12 Tf 1 0 0 1 10 100 Tm (A) Tj 1 0 0 1 30 100 Tm (B) Tj ET";
    let document = PdfDocument::from_bytes(pdf_with_page_entries(content, PAGE_BOX, "")).expect("valid PDF");
    assert_eq!(
        span_texts(document.extract_spans(0).expect("ordinary spans")),
        vec!["AB"]
    );
    let split_config = SpanMergingConfig {
        merge_tm_tj_runs: false,
        ..SpanMergingConfig::default()
    };
    assert_eq!(configured_texts(&document, split_config.clone()), vec!["A", "B"]);
    assert_eq!(configured_texts(&document, SpanMergingConfig::default()), vec!["AB"]);
    assert_eq!(configured_texts(&document, split_config), vec!["A", "B"]);
}

#[test]
fn configured_page_membership_uses_normalized_inherited_bounds_before_rotation() {
    for (page_entries, parent_entries) in [
        (PAGE_BOX, ""),
        ("/MediaBox [200 200 0 0]", ""),
        ("/MediaBox [0 0 200 200] /Rotate 90", ""),
        ("", PAGE_BOX),
    ] {
        for (x, expected) in [(10.0, vec!["Qazvex".to_string()]), (500.0, Vec::new())] {
            let content = format!("BT /F1 1 Tf {x} 100 Td (Qazvex) Tj ET");
            let document = PdfDocument::from_bytes(pdf_with_page_entries(&content, page_entries, parent_entries))
                .expect("valid PDF");
            assert_raw_payload(&document, "Qazvex", x);
            assert_eq!(span_texts(document.extract_spans(0).expect("ordinary spans")), expected);
            assert_eq!(
                configured_texts(&document, SpanMergingConfig::default()),
                expected,
                "page entries={page_entries}, parent entries={parent_entries}, x={x}"
            );
        }
    }
}
