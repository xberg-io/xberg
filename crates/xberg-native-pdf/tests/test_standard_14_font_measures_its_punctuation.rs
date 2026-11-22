//! A Standard-14 font's built-in metrics cover its punctuation, not just ASCII.
//!
//! ISO 32000-1:2008 §9.6.2.2 (docs/spec/pdf.md:17706) lets a Standard-14 font
//! dictionary omit `/Widths` — the reader supplies the metrics — and §9.4.4
//! (:17433) then spends them on the text matrix after each glyph is painted. A
//! width that is merely plausible moves the pen to the wrong place.
//!
//! The built-in tables ran from code 32 to 126, so every glyph above printable
//! ASCII fell through to a generic 550/1000 em default. Helvetica advances an em
//! dash by a full em, so a run containing one measured 0.45 em short of its own
//! ink for every dash in it, and the gap turned up at the end of the run where
//! the page has none.
//!
//! Which code carries which glyph depends on the named encoding, and Annex D.2
//! lists both: StandardEncoding puts the em dash at 208 and the en dash at 177,
//! WinAnsiEncoding at 151 and 150. A fixture written against only one of them
//! passes against the defect in the other.

use xberg_native_pdf::PdfDocument;

/// A page drawing `text` in a bare Standard-14 Helvetica under `encoding`,
/// with no `/Widths` — the shape §9.6.2.2 describes.
fn page(encoding: &str, text: &str) -> Vec<u8> {
    let content = format!("BT /F1 10 Tf 72 700 Td ({text}) Tj ET\n").into_bytes();
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
        format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /{encoding} >>").into_bytes(),
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

/// The advance recorded for the one non-ASCII glyph in the run.
fn dash_advance(encoding: &str, escape: &str) -> f32 {
    let doc = PdfDocument::from_bytes(page(encoding, &format!("A{escape}B"))).expect("open");
    let span = doc
        .extract_spans(0)
        .expect("spans")
        .into_iter()
        .find(|s| !s.text.trim().is_empty())
        .expect("a span");
    let i = span
        .text
        .chars()
        .position(|c| c == '—' || c == '–')
        .unwrap_or_else(|| panic!("no dash in {:?}", span.text));
    span.char_widths[i]
}

/// Helvetica's em dash is a full em: 1000/1000 × 10 pt.
#[test]
fn test_em_dash_under_standard_encoding_advances_a_full_em() {
    // \320 is octal 208, StandardEncoding's emdash (Annex D.2).
    let w = dash_advance("StandardEncoding", "\\320");
    assert!(
        (w - 10.0).abs() < 0.05,
        "Helvetica's em dash advances 10.00 pt at 10 pt, got {w:.3}"
    );
}

/// And 556/1000 for the en dash, at StandardEncoding's own code.
#[test]
fn test_en_dash_under_standard_encoding_advances_its_own_width() {
    // \261 is octal 177, StandardEncoding's endash.
    let w = dash_advance("StandardEncoding", "\\261");
    assert!(
        (w - 5.56).abs() < 0.05,
        "Helvetica's en dash advances 5.56 pt at 10 pt, got {w:.3}"
    );
}

/// The same two glyphs live at different codes under WinAnsiEncoding, and a
/// fixture covering only one encoding would pass against the other's defect.
#[test]
fn test_same_glyphs_measure_the_same_under_winansi() {
    // \227 is octal 151, \226 is 150.
    let em = dash_advance("WinAnsiEncoding", "\\227");
    let en = dash_advance("WinAnsiEncoding", "\\226");
    assert!((em - 10.0).abs() < 0.05, "em dash under WinAnsi: {em:.3}");
    assert!((en - 5.56).abs() < 0.05, "en dash under WinAnsi: {en:.3}");
}
