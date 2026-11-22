//! The `/PlacedPDF` keep-gate must not read "I could not measure this" as
//! "this is not a duplicate".
//!
//! InDesign wraps placed artwork in a `/PlacedPDF` marked-content scope. A
//! placed galley that merely repeats the page body has to be suppressed, or
//! `extract_text` emits every word twice.
//!
//! The gate decides by tokenising the text-show operands and measuring how much
//! of the placed text also appears outside. But those operands carry **encoded
//! character codes, not text** (ISO 32000-1:2008 §9.4.3 — a show operand is a
//! string of character codes interpreted through the font's encoding). Under
//! Identity-H — the dominant modern encoding, and the one this producer emits —
//! the bytes are two-byte CIDs, so no run of ASCII alphanumerics forms, the
//! measured duplication came out as 0.0, and the gate kept.
//!
//! Failing closed on "no tokens" is the minimum fix. The real discriminator is
//! bounding-box overlap, which exists downstream and is not consulted here.

use xberg_native_pdf::PdfDocument;

/// A page whose body sits inside a `/PlacedPDF` scope and is repeated outside
/// it, using a simple WinAnsi font — the gate can read these operands.
fn placed_duplicate_winansi() -> Vec<u8> {
    let body: String = (0..40)
        .map(|i| format!("1 0 0 1 60 {} Tm (Duplicated body line {i}) Tj\n", 700 - i * 4))
        .collect();
    let placed: String = (0..40)
        .map(|i| format!("1 0 0 1 60 {} Tm (Duplicated body line {i}) Tj\n", 700 - i * 4))
        .collect();
    let content = format!(
        "BT /F1 9 Tf\n{body}ET\n\
         /PlacedPDF BMC\nBT /F1 9 Tf\n{placed}ET\nEMC\n"
    );
    build(&content, "/Font << /F1 5 0 R >>", None)
}

/// The same shape, but the placed text is shown through a Type 0 Identity-H
/// font, so its operands are two-byte codes the gate cannot tokenise.
fn placed_duplicate_identity_h() -> Vec<u8> {
    // Outside text, readable by the gate.
    let outside: String = (0..40)
        .map(|i| format!("1 0 0 1 60 {} Tm (Duplicated body line {i}) Tj\n", 700 - i * 4))
        .collect();
    // Placed text, shown as two-byte CIDs: no ASCII alphanumeric run forms.
    let mut placed = String::new();
    for i in 0..40 {
        placed.push_str(&format!("1 0 0 1 60 {} Tm <", 700 - i * 4));
        for c in 0..24u16 {
            placed.push_str(&format!("{:04X}", 0x0100 + c));
        }
        placed.push_str("> Tj\n");
    }
    let content = format!(
        "BT /F1 9 Tf\n{outside}ET\n\
         /PlacedPDF BMC\nBT /F2 9 Tf\n{placed}ET\nEMC\n"
    );
    build(&content, "/Font << /F1 5 0 R /F2 7 0 R >>", Some(7))
}

/// Assemble the page. `type0`, when set, emits a Type 0 Identity-H font at
/// that object number.
fn build(content: &str, resources: &str, type0: Option<usize>) -> Vec<u8> {
    let last = type0.map(|n| n + 1).unwrap_or(6);
    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; last + 2];
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
        &format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << {resources} >> /Contents 4 0 R >>"
        ),
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    // The marked-content property dictionary carrying the InDesign tag.
    obj(&mut buf, &mut off, 6, "<< /Type /OCG /Name (PlacedPDF) >>");
    if let Some(n) = type0 {
        obj(
            &mut buf,
            &mut off,
            n,
            "<< /Type /Font /Subtype /Type0 /BaseFont /Sub+Body /Encoding /Identity-H \
             /DescendantFonts [8 0 R] >>",
        );
        obj(
            &mut buf,
            &mut off,
            n + 1,
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Sub+Body \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
             /FontDescriptor 9 0 R /DW 500 >>",
        );
    }

    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", last + 1).as_bytes());
    for o in &off[1..=last] {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n", last + 1).as_bytes());
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// The control: when the gate *can* read the operands it already suppressed a
/// duplicate galley, and must keep doing so.
#[test]
fn test_readable_duplicate_galley_is_still_suppressed() {
    let doc = PdfDocument::from_bytes(placed_duplicate_winansi()).expect("parse");
    let text = doc.extract_text(0).expect("extract");
    let n = text.matches("Duplicated body line 7").count();
    assert_eq!(
        n, 1,
        "a duplicate placed galley should be emitted once, got {n}:\n{text}"
    );
}

/// The defect: the same duplicate galley, shown through Identity-H, yielded no
/// tokens and so was measured as 0.0 duplicate and kept.
#[test]
fn test_unreadable_placed_galley_does_not_double_the_page() {
    let doc = PdfDocument::from_bytes(placed_duplicate_identity_h()).expect("parse");
    let text = doc.extract_text(0).expect("extract");
    let stray: String = text.chars().filter(|c| !c.is_ascii()).collect();
    assert!(
        stray.is_empty(),
        "the placed galley's two-byte codes decode to non-ASCII text; finding {} such characters \
         means an unmeasurable placed galley was kept:\n{text}",
        stray.chars().count()
    );
    for i in [0usize, 7, 39] {
        let needle = format!("Duplicated body line {i}");
        assert!(
            text.matches(needle.as_str()).count() <= 1,
            "line {i} appears more than once:\n{text}"
        );
    }
}

/// The page's own text must survive either way: failing closed suppresses the
/// placed copy, not the body.
#[test]
fn test_page_body_survives_the_gate() {
    for pdf in [placed_duplicate_winansi(), placed_duplicate_identity_h()] {
        let doc = PdfDocument::from_bytes(pdf).expect("parse");
        let text = doc.extract_text(0).expect("extract");
        assert!(
            text.contains("Duplicated body line 0"),
            "the page body was lost entirely:\n{text}"
        );
    }
}
