//! A word split by a soft hyphen (U+00AD) across a wrapped line inside one
//! paragraph must rejoin without a space and without the marker.
//!
//! Byte 0xAD under WinAnsiEncoding decodes to U+00AD per this project's own
//! encoding table (`src/fonts/font_dict.rs`) — written directly into the
//! content stream since a soft hyphen isn't representable in a `&str`
//! literal source-encoded as UTF-8 the way a real justified-text PDF
//! producer would emit it as a raw single byte.
//!
//! Hand-built, synthetic PDF; no third-party fixture.

use xberg_native_pdf::PdfDocument;

const SOFT_HYPHEN_BYTE: u8 = 0xAD;

/// One-page untagged PDF: a single paragraph wrapped across two visual
/// lines, with the first line ending in a word broken at a soft hyphen
/// (`wonder<SHY>` / `ful`), positioned so the layout pipeline reads them as
/// one paragraph (second line starts lowercase, first line runs long).
fn soft_hyphen_wrap_pdf() -> Vec<u8> {
    let mut content: Vec<u8> = Vec::new();
    content.extend_from_slice(b"BT /F1 12 Tf\n");
    content.extend_from_slice(b"1 0 0 1 72 700 Tm (This is a truly wonder");
    content.push(SOFT_HYPHEN_BYTE);
    content.extend_from_slice(b") Tj\n");
    content.extend_from_slice(b"1 0 0 1 72 686 Tm (ful example indeed) Tj\n");
    content.extend_from_slice(b"ET");

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 6];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(&content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in &off[1..=5] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", *offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

#[test]
fn soft_hyphen_at_intra_paragraph_wrap_is_stripped_and_rejoins_without_a_space() {
    let doc = PdfDocument::from_bytes(soft_hyphen_wrap_pdf()).expect("parse fixture");
    let text = doc.extract_text(0).expect("extract_text");

    assert!(
        !text.contains('\u{00AD}'),
        "a closed wrap must not leave its soft hyphen behind: {text:?}"
    );
    assert!(
        text.contains("wonderful"),
        "word split at a soft-hyphen wrap did not rejoin cleanly: {text:?}"
    );
    assert!(
        !text.contains("wonder ful"),
        "soft-hyphen wrap left a spurious space instead of joining the word: {text:?}"
    );
}
