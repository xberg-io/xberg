//! A table drawn with a 90 or 270 degree text matrix (no page `/Rotate`) must extract row by row
//! with each label next to its value (xberg-io/xberg#3423). Before the fix the cells came out in
//! reverse order (90) or column by column (270) through the default extraction path.
#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::ExtractionConfig;

const ROWS: [[&str; 3]; 3] = [
    ["Engine oil", "6.0 quarts (5.7L)", "Motorcraft SAE 5W-20"],
    ["Coolant", "12.1 quarts (11.5L)", "Motorcraft Orange"],
    ["Brake fluid", "as needed", "DOT 4 LV"],
];
const COL_X: [i32; 3] = [0, 160, 340];
const ROW_PITCH: i32 = 24;

/// One Letter page; the table's own frame is turned by `rotation` (0, 90 or 270) through the text
/// matrix, row `n` sitting `ROW_PITCH * n` below the first row.
fn build_pdf(rotation: i32) -> Vec<u8> {
    let mut content = String::from("BT /F1 12 Tf\n");
    for (n, row) in ROWS.iter().enumerate() {
        for (cell, x) in row.iter().zip(COL_X) {
            let down = ROW_PITCH * n as i32;
            let tm = match rotation {
                0 => format!("1 0 0 1 {} {}", 100 + x, 600 - down),
                90 => format!("0 1 -1 0 {} {}", 300 + down, 100 + x),
                270 => format!("0 -1 1 0 {} {}", 300 - down, 500 - x),
                _ => unreachable!(),
            };
            content.push_str(&format!("{tm} Tm ({cell}) Tj\n"));
        }
    }
    content.push_str("ET");

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
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

fn extracted_text(rotation: i32) -> String {
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(&build_pdf(rotation), "application/pdf", &config).unwrap();
    document.content.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn expected_text() -> String {
    ROWS.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}

#[test]
fn upright_table_reads_row_by_row() {
    assert_eq!(extracted_text(0), expected_text());
}

#[test]
fn table_rotated_90_in_content_stream_reads_row_by_row() {
    assert_eq!(extracted_text(90), expected_text());
}

#[test]
fn table_rotated_270_in_content_stream_reads_row_by_row() {
    assert_eq!(extracted_text(270), expected_text());
}
