//! A blank run on another writing axis does not end the line it crosses.
//!
//! ISO 32000-1:2008 §9.4.4 interprets a glyph's displacement in text space, so
//! the writing axis is a property of the glyphs a run draws. A run with no
//! glyphs draws nothing on any axis: its rotation records the text matrix that
//! happened to be in force, not evidence about the page.
//!
//! Producers emit blank runs freely, and a rotated watermark scatters them
//! across every row it crosses. Where one lands between a table row's label and
//! its first cell, treating it as an axis change ends the line twice — once
//! entering it, once leaving — and the label is stranded on a line of its own
//! while its cells go to the next.
//!
//! Hand-built synthetic PDF; no third-party fixture.

use xberg_native_pdf::PdfDocument;

/// `0 1 -1 0` is a quarter turn, so the blank run below is on the cross axis
/// exactly as a rotated page stamp is.
fn page_with_a_blank_rotated_run_mid_row() -> Vec<u8> {
    let content = b"BT /F1 11 Tf 85 609 Td (LABEL) Tj ET\n\
                    BT /F1 11 Tf 0 1 -1 0 139 609 Tm ( ) Tj ET\n\
                    BT /F1 11 Tf 0 1 -1 0 156 609 Tm ( ) Tj ET\n\
                    BT /F1 11 Tf 173 609 Td (CELLA) Tj ET\n\
                    BT /F1 11 Tf 246 609 Td (CELLB) Tj ET"
        .to_vec();
    build(content)
}

/// The counter-case: the same row, but the crossing run carries ink. Two runs
/// on different axes are genuinely two lines and must still separate.
fn page_with_an_inked_rotated_run_mid_row() -> Vec<u8> {
    let content = b"BT /F1 11 Tf 85 609 Td (LABEL) Tj ET\n\
                    BT /F1 11 Tf 0 1 -1 0 139 609 Tm (STAMP) Tj ET\n\
                    BT /F1 11 Tf 173 609 Td (CELLA) Tj ET"
        .to_vec();
    build(content)
}

fn build(content: Vec<u8>) -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"\nendstream".to_vec(),
        ]
        .concat(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn text_of(pdf: Vec<u8>) -> String {
    PdfDocument::from_bytes(pdf)
        .expect("open")
        .extract_text(0)
        .expect("text")
}

#[test]
fn test_blank_crossing_run_leaves_the_row_whole() {
    let out = text_of(page_with_a_blank_rotated_run_mid_row());
    let row = out
        .lines()
        .find(|l| l.contains("LABEL"))
        .unwrap_or_else(|| panic!("no line carries the label; got:\n{out}"));
    assert!(
        row.contains("CELLA"),
        "the label must keep the cell that follows it across a blank crossing \
         run; the row came out as {row:?} in:\n{out}"
    );
}

#[test]
fn test_inked_crossing_run_still_ends_the_line() {
    let out = text_of(page_with_an_inked_rotated_run_mid_row());
    let row = out
        .lines()
        .find(|l| l.contains("LABEL"))
        .unwrap_or_else(|| panic!("no line carries the label; got:\n{out}"));
    assert!(
        !row.contains("CELLA"),
        "a run that draws glyphs on another axis is a different line and must \
         still break; got {row:?} in:\n{out}"
    );
}
