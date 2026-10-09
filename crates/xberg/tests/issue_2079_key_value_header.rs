//! Regression coverage for xberg-io/xberg#2079.

#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, OutputFormat};

const ROWS: [(&str, &[&str]); 7] = [
    ("Applicant:", &["Example Holdings Limited"]),
    ("Respondent:", &["Taylor Respondent"]),
    (
        "Address of premises:",
        &["12 Demo Street, Unit 4B, Sample Town,", "X00Y000"],
    ),
    (
        "Panel:",
        &["Jordan Example (Chair)", "Casey Placeholder, Morgan Fixture"],
    ),
    ("Venue:", &["Virtual"]),
    ("Date & time of hearing:", &["15/01/2030 10:30:00"]),
    (
        "Attendees:",
        &["Riley Synthetic (Applicant agent)", "Alex Sample (Responding party)"],
    ),
];

fn pdf_string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

fn text_run(font: &str, x: f32, top_y: f32, text: &str) -> String {
    let baseline_y = 842.0 - top_y;
    format!(
        "BT /{font} 11 Tf 1 0 0 1 {x:.1} {baseline_y:.1} Tm ({}) Tj ET\n",
        pdf_string(text)
    )
}

fn key_value_header_pdf(bold_labels: bool) -> Vec<u8> {
    let mut content = String::new();
    content.push_str(&text_run("F2", 235.0, 150.0, "Example Review Panel"));
    content.push_str(&text_run("F2", 215.0, 173.0, "EXAMPLE PROCEDURES ACT 2030"));
    content.push_str(&text_run(
        "F1",
        72.0,
        195.0,
        "Report of Case Reference: CR0000001 / File Reference: FR0000001",
    ));

    let label_font = if bold_labels { "F2" } else { "F1" };
    let mut y = 218.0;
    for (label, values) in ROWS {
        content.push_str(&text_run(label_font, 77.0, y + (values.len() - 1) as f32 * 7.0, label));
        for (line, value) in values.iter().enumerate() {
            content.push_str(&text_run("F1", 231.0, y + line as f32 * 14.0, value));
        }
        y += values.len() as f32 * 14.0 + 12.0;
    }

    content.push_str(&text_run("F2", 72.0, y + 12.0, "1.  Background:"));
    content.push_str(&text_run(
        "F1",
        72.0,
        y + 32.0,
        "This is a synthetic test document. All names and identifiers are fictitious.",
    ));

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}endstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_owned(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn assert_labels_keep_wrapped_values_in_page_order(bold_labels: bool) {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        disable_ocr: true,
        use_cache: false,
        ..ExtractionConfig::default()
    };
    let document = extract_bytes_document_blocking(&key_value_header_pdf(bold_labels), "application/pdf", &config)
        .expect("synthetic PDF extraction must succeed");
    let normalized = document
        .content
        .replace("**", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let mut search_from = 0;
    for (label, values) in ROWS {
        for expected in std::iter::once(label).chain(values.iter().copied()) {
            assert_eq!(
                normalized.match_indices(expected).count(),
                1,
                "expected {expected:?} exactly once, got:\n{normalized}"
            );
            let relative = normalized[search_from..]
                .find(expected)
                .unwrap_or_else(|| panic!("expected {expected:?} after byte {search_from}, got:\n{normalized}"));
            search_from += relative + expected.len();
        }
    }
}

#[test]
fn same_font_labels_keep_wrapped_values_in_page_order() {
    assert_labels_keep_wrapped_values_in_page_order(false);
}

#[test]
fn bold_labels_keep_wrapped_values_in_page_order() {
    assert_labels_keep_wrapped_values_in_page_order(true);
}

#[test]
fn explicit_native_non_column_modes_remain_opt_in_free() {
    use xberg_native_pdf::document::{PdfDocument, ReadingOrder};

    let document = PdfDocument::from_bytes(key_value_header_pdf(false)).expect("synthetic PDF must parse");
    let top_to_bottom: Vec<String> = document
        .extract_spans_with_reading_order(0, ReadingOrder::TopToBottom)
        .expect("top-to-bottom extraction")
        .into_iter()
        .map(|span| span.text)
        .collect();
    let address = top_to_bottom
        .iter()
        .position(|text| text == "Address of premises:")
        .expect("address label");
    let first_line = top_to_bottom
        .iter()
        .position(|text| text == "12 Demo Street, Unit 4B, Sample Town,")
        .expect("first address line");
    assert!(
        first_line < address,
        "TopToBottom must retain its geometric ordering contract"
    );
}
