//! One advance per glyph, however many bytes the glyph's character occupies.
//!
//! `TextSpan::char_widths` carries one entry per character of `text`, and its
//! consumers index it that way — the span's extent is the sum of it, and the
//! word-gap tests measure from that extent. It was filled by asking how much
//! the accumulating buffer had grown, but that buffer is a `String`, so the
//! answer was in **bytes**. An em dash is three bytes of UTF-8 and one
//! character, so it contributed three entries of a third of its advance each,
//! and from that point on every glyph in the run carried a neighbour's width.
//!
//! ISO 32000-1:2008 §9.4.4 gives each glyph a single displacement along the
//! writing axis. The array holds those displacements, so it has exactly as
//! many entries as the text has characters.
//!
//! The consequence is not visible in the characters — `extract_chars` reads
//! measured origins, not this array — but the span's own extent falls short by
//! whatever the misalignment loses, and a gap appears at the end of the run
//! where the page has none.

use xberg_native_pdf::PdfDocument;

/// A page drawing one show string in WinAnsi, where `\227` is the em dash.
fn one_run(text: &str) -> Vec<u8> {
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
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
           /Encoding /WinAnsiEncoding >>"
            .to_vec(),
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

fn span_of(text: &str) -> xberg_native_pdf::layout::TextSpan {
    let doc = PdfDocument::from_bytes(one_run(text)).expect("open");
    doc.extract_spans(0)
        .expect("spans")
        .into_iter()
        .find(|s| !s.text.trim().is_empty())
        .expect("a span")
}

#[test]
fn test_em_dash_contributes_one_advance_not_three() {
    let span = span_of("AB\\227CD");
    assert_eq!(span.text, "AB—CD");
    assert_eq!(
        span.char_widths.len(),
        span.text.chars().count(),
        "char_widths must hold one advance per character, got {:?} for {:?}",
        span.char_widths,
        span.text
    );
}

/// The misalignment's real cost: with three entries spent on the em dash, the
/// glyphs after it take the widths of glyphs before them. `C` and `D` are both
/// 7.22 pt at 10 pt Helvetica; under the defect they were handed a third of the
/// dash's advance instead.
///
/// The dash's own advance is whatever the font reports for it; only that it
/// is one positive advance is asserted, since the Standard-14 metric for
/// glyphs above printable ASCII is outside this fix.
#[test]
fn test_glyphs_after_a_multibyte_character_keep_their_own_advances() {
    let span = span_of("AB\\227CD");
    let widths = &span.char_widths;
    assert_eq!(widths.len(), 5, "expected one per character, got {widths:?}");
    assert!(
        widths[2] > 0.0,
        "the em dash must carry its own advance, got {widths:?}"
    );
    for (i, c) in [(3usize, 'C'), (4, 'D')] {
        assert!(
            (widths[i] - 7.22).abs() < 0.05,
            "{c:?} advances 7.22 pt at 10 pt Helvetica, got {:.3} in {widths:?}",
            widths[i]
        );
    }
}

/// The control: an all-single-byte run was never affected, so a fixture built
/// only from ASCII would pass against the defect.
#[test]
fn test_ascii_only_run_was_already_in_step() {
    let span = span_of("ABCD");
    assert_eq!(span.char_widths.len(), 4, "{:?}", span.char_widths);
}
