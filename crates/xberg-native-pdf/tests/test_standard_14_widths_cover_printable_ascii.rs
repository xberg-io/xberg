//! A Standard-14 font with no `/Widths` measures its text correctly.
//!
//! ISO 32000-1:2008 §9.6.2.2 (`docs/spec/pdf.md`:17706) lets such a font
//! dictionary omit `/Widths` entirely:
//!
//! > These fonts, or their font metrics and suitable substitution fonts, shall
//! > be available to the conforming reader.
//!
//! So the reader supplies the metrics, and §9.4.4 (`docs/spec/pdf.md`:17433)
//! then spends them: "After the glyph is painted, the text matrix shall be
//! updated according to the glyph displacement." A width that is merely
//! plausible is not enough — it moves the pen.
//!
//! The built-in tables were incomplete. Helvetica and Helvetica-Bold each
//! omitted 24 printable codes, `(` and `)` among them, and every omission fell
//! through to a 550-unit default. A citation like `31330(a)(2)` therefore
//! measured 8.7 pt wider at 10 pt than it draws, which turned a real
//! inter-word gap into an apparent overlap and glued the run to the text
//! after it. Helvetica's `z` also carried 444 — the Times value; Helvetica's
//! is 500.
//!
//! Measuring a real span exercises the metric the way the extractor does,
//! rather than asserting on a lookup table in isolation.

use xberg_native_pdf::PdfDocument;

/// One line of `text` in Helvetica at 10 pt, with **no** `/Widths` array, so
/// the built-in Standard-14 metrics are what answer.
fn helvetica_page(text: &str) -> Vec<u8> {
    let content = format!("BT /F1 10 Tf 50 700 Td ({text}) Tj ET").into_bytes();
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

fn measured_width(text: &str) -> f32 {
    let doc = PdfDocument::from_bytes(helvetica_page(text)).expect("open");
    let spans = doc.extract_spans(0).expect("spans");
    assert!(!spans.is_empty(), "the page must yield a span for {text:?}");
    spans.iter().map(|s| s.bbox.width).fold(0.0f32, f32::max)
}

/// The citation that exposed this. Adobe's metrics sum to 9837/1000 em, so at
/// 10 pt the run is 98.37 pt wide. With `(` and `)` falling through to the
/// 550-unit default it measured 107.05 — 8.7 pt too wide.
#[test]
fn test_run_containing_parentheses_is_not_over_measured() {
    let w = measured_width("46 U.S.C. 31330\\(a\\)\\(2\\)");
    assert!(
        (w - 98.37).abs() < 1.5,
        "expected ~98.37 pt from Adobe metrics, got {w:.2}. \
         107.05 means `(` and `)` are still falling through to the 550 default"
    );
}

/// Every previously-absent range in one run: `#$%&`, `*+`, `<=>?@`, `[]^_`,
/// `{|}~`. Adobe's metrics sum to 10057/1000 em, so 100.57 pt at 10 pt.
/// Backslash is left out deliberately — it needs escaping in a PDF literal
/// string and the escape, not the metric, would decide what is measured.
#[test]
fn test_previously_missing_punctuation_ranges_measure_correctly() {
    let w = measured_width("#$%&*+<=>?@[]^_{|}~");
    assert!(
        (w - 100.57).abs() < 2.0,
        "expected ~100.57 pt from Adobe metrics, got {w:.2}; a large excess means \
         some of these codes still resolve to the 550-unit default"
    );
}

/// Helvetica `z` is 500 per Adobe; the table carried 444, which is the Times
/// value. Twenty `z` glyphs make the 56-unit error visible well above noise:
/// 100.0 pt correct against 88.8 pt with the wrong metric.
#[test]
fn helvetica_z_uses_the_helvetica_metric_not_the_times_one() {
    let w = measured_width("zzzzzzzzzzzzzzzzzzzz");
    assert!(
        (w - 100.0).abs() < 1.5,
        "expected ~100.0 pt (20 x 500/1000 x 10), got {w:.2}; ~88.8 means `z` is \
         still carrying the Times width of 444"
    );
}
