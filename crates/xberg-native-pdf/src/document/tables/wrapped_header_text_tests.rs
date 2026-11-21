//! Plain-text rendering of a wrapped header band (xberg-io/xberg#3346).
//!
//! The detector joins a cell's baselines with `\n`, and a kept header band is exempt from the row
//! splitter, so `Table::render_text` would print those breaks verbatim and push the remaining
//! columns onto their own lines. This fixture is the #3346 shape — one ruled band four lines tall
//! whose labels wrap, over four single-line numeric rows — and asserts `extract_text` keeps one
//! line per row.

use crate::document::PdfDocument;

const LEFT: f32 = 50.0;
const COLUMN: f32 = 60.0;
const COLUMNS: usize = 6;
const RIGHT: f32 = LEFT + COLUMN * COLUMNS as f32;
const TOP: f32 = 600.0;
const HEADER_BOTTOM: f32 = 548.0;
const BOTTOM: f32 = 500.0;
const BODY_ROWS: [f32; 4] = [500.0, 512.0, 524.0, 536.0];

fn text(ops: &mut String, column: usize, y: f32, value: &str) {
    let x = LEFT + column as f32 * COLUMN + 3.0;
    ops.push_str(&format!("BT /F1 8 Tf 1 0 0 1 {x} {y} Tm ({value}) Tj ET\n"));
}

fn content_stream() -> String {
    let mut ops = String::new();
    // Column rules first, then row rules, the order the producer paints them in. ~keep
    for column in 0..=COLUMNS {
        let x = LEFT + column as f32 * COLUMN - 0.5;
        ops.push_str(&format!("0 g {x} {BOTTOM} 1 {} re f\n", TOP - BOTTOM));
    }
    for y in [TOP, HEADER_BOTTOM, 536.0, 524.0, 512.0, BOTTOM] {
        ops.push_str(&format!("0 g {LEFT} {} {} 1 re f\n", y - 0.5, RIGHT - LEFT));
    }
    text(&mut ops, 0, 554.0, "Amount");
    for (label, baseline) in [
        ("Proposed", 590.0),
        ("Chicago,", 578.0),
        ("Commonwealth", 566.0),
        ("Label", 554.0),
    ] {
        text(&mut ops, 1, baseline, label);
    }
    for (label, baseline) in [("Current", 578.0), ("Rate", 566.0)] {
        text(&mut ops, 3, baseline, label);
    }
    text(&mut ops, 5, 554.0, "Percent");
    for (row, baseline) in BODY_ROWS.into_iter().zip([502.0, 514.0, 526.0, 538.0]) {
        debug_assert!(baseline > row);
        text(&mut ops, 0, baseline, "$1,000");
        text(&mut ops, 1, baseline, "$10.00");
        text(&mut ops, 3, baseline, "$9.00");
        text(&mut ops, 5, baseline, "20%");
    }
    ops
}

fn wrapped_header_pdf() -> Vec<u8> {
    let content = content_stream();
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();
    let mut object = |pdf: &mut Vec<u8>, body: String| {
        offsets.push(pdf.len());
        pdf.extend_from_slice(body.as_bytes());
    };
    object(
        &mut pdf,
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string(),
    );
    object(
        &mut pdf,
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_string(),
    );
    object(
        &mut pdf,
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n"
            .to_string(),
    );
    object(
        &mut pdf,
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n".to_string(),
    );
    object(
        &mut pdf,
        format!(
            "5 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n",
            content.len()
        ),
    );
    let size = offsets.len() + 1;
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

#[test]
fn a_wrapped_header_row_renders_on_one_plain_text_line() {
    let document = PdfDocument::from_bytes(wrapped_header_pdf()).expect("parse the ruled-table fixture");
    let rendered = document.extract_text(0).expect("extract plain text");
    let header: Vec<&str> = rendered.lines().filter(|line| line.contains("Proposed")).collect();
    assert_eq!(header.len(), 1, "the header band must be one line:\n{rendered}");
    for label in [
        "Amount",
        "Proposed",
        "Chicago,",
        "Commonwealth",
        "Label",
        "Current",
        "Rate",
        "Percent",
    ] {
        assert!(
            header[0].contains(label),
            "{label} must stay on the header line:\n{rendered}"
        );
    }
    let body: Vec<&str> = rendered.lines().filter(|line| line.contains("$1,000")).collect();
    assert_eq!(body.len(), BODY_ROWS.len(), "one line per body row:\n{rendered}");
    for line in body {
        assert!(
            line.contains("$10.00") && line.contains("$9.00") && line.contains("20%"),
            "each body row keeps its columns on one line:\n{rendered}"
        );
    }
    assert_eq!(
        rendered.lines().filter(|line| !line.trim().is_empty()).count(),
        BODY_ROWS.len() + 1
    );
}
