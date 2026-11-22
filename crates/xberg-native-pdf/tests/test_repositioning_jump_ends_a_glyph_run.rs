//! An absolute repositioning that skips a gap ends a glyph run, exactly as a
//! relative one does.
//!
//! ISO 32000-1:2008 §9.4.2, Table 108 gives `Tm` and `Td` the same effect: both
//! set the text matrix and the text line matrix, differing only in whether the
//! displacement is absolute or relative to the current line matrix. A
//! displacement that ends a run of contiguous glyphs for one operator must end
//! it for the other — run continuity is a property of the resulting pen
//! position, not of the operator that moved the pen.
//!
//! The extractor batches `Tm`+`Tj` pairs into one span, which is what keeps a
//! PDF that positions every glyph individually from producing thousands of
//! one-character spans. That test required the same line, the same transform
//! and forward progression, but placed no bound on *how far* forward: a jump
//! into the next column was accepted as a continuation, so two show operations
//! separated by empty space were glued into a single span with no separator
//! and a width spanning the gap between them.
//!
//! Downstream that costs more than a missing space. Anything reasoning about a
//! span's extent — table-cell ownership, column detection, reading order —
//! sees one span straddling the gap, and a cell can no longer claim the text
//! drawn inside it.

use xberg_native_pdf::PdfDocument;

/// A one-line page whose content stream is `body`, in 10 pt Courier.
fn page(body: &str) -> Vec<u8> {
    let content = format!("BT /F1 10 Tf {body} ET\n");

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
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>",
    );

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for o in &off[1..=5] {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// `(text, x, width)` of every span on the page.
fn spans(body: &str) -> Vec<(String, f32, f32)> {
    let doc = PdfDocument::from_bytes(page(body)).expect("parse");
    doc.extract_spans(0)
        .expect("spans")
        .into_iter()
        .map(|s| (s.text.clone(), s.bbox.x, s.bbox.width))
        .collect()
}

/// 10 pt Courier advances 6 pt per glyph, so "Three" occupies 60..90 and the
/// next word starts at 210 — a 120 pt gap of empty page.
const JUMP_TM: &str = "1 0 0 1 60 130 Tm (Three) Tj 1 0 0 1 210 130 Tm (Four) Tj";
const JUMP_TD: &str = "1 0 0 1 60 130 Tm (Three) Tj 150 0 Td (Four) Tj";

/// The relative form is the reference: it has always ended the run.
#[test]
fn test_relative_jump_ends_the_run() {
    let s = spans(JUMP_TD);
    assert_eq!(s.len(), 2, "expected two spans, got {s:?}");
    assert_eq!(s[0].0, "Three");
    assert_eq!(s[1].0, "Four");
}

/// The defect: the absolute form glued the two show operations.
#[test]
fn test_absolute_jump_ends_the_run_too() {
    let s = spans(JUMP_TM);
    assert_eq!(
        s.len(),
        2,
        "a 120 pt repositioning was treated as a continuation, so the two show \
         operations were glued into one span: {s:?}"
    );
    assert_eq!(s[0].0, "Three");
    assert_eq!(s[1].0, "Four");
}

/// Stated as the invariant, so the two operators cannot drift apart again.
#[test]
fn test_two_forms_of_the_same_jump_agree() {
    assert_eq!(spans(JUMP_TM), spans(JUMP_TD));
}

/// The glued span also claimed the empty space between the words as its own
/// width, which is what breaks table-cell ownership downstream.
#[test]
fn test_span_does_not_claim_the_gap_as_its_width() {
    let s = spans(JUMP_TM);
    let first = &s[0];
    assert!(
        first.2 < 40.0,
        "the first span's width should cover its five glyphs (~30 pt), not the \
         gap to the next word: {first:?}"
    );
}

/// The control that keeps this from being a regression: the batching this
/// bound is attached to must still batch. A PDF that positions every glyph
/// with its own `Tm` must still yield one span, not one span per glyph.
#[test]
fn test_glyph_by_glyph_run_still_batches_into_one_span() {
    let body: String = "Contiguous"
        .chars()
        .enumerate()
        .map(|(i, c)| format!("1 0 0 1 {} 130 Tm ({c}) Tj ", 60.0 + 6.0 * i as f32))
        .collect();
    let s = spans(&body);
    assert_eq!(s.len(), 1, "a contiguous glyph run must stay one span: {s:?}");
    assert_eq!(s[0].0, "Contiguous");
}

/// A word-sized gap is not a jump. The bound is an em, not a word space:
/// a producer can leave an intra-word repositioning seam wider than the same
/// font's declared space advance, so no word-space constant separates a seam
/// from a space. Sub-em gaps belong to the span merger, which reads the
/// source-order evidence that actually distinguishes them.
#[test]
fn test_word_sized_gap_is_not_a_jump() {
    let s = spans("1 0 0 1 60 130 Tm (ab) Tj 1 0 0 1 78 130 Tm (cd) Tj");
    assert_eq!(s.len(), 1, "a one-space gap must not split the run: {s:?}");
}

/// The inversion this bound must not re-litigate: a seam wider than the
/// font's own space advance still belongs to the merger. Courier declares a
/// 600/1000 em space (6 pt at 10 pt), so an 8 pt seam exceeds it and would
/// split under any word-space threshold, yet stays well inside an em.
#[test]
fn test_seam_wider_than_the_space_advance_still_belongs_to_the_merger() {
    let s = spans("1 0 0 1 60 130 Tm (ab) Tj 1 0 0 1 80 130 Tm (cd) Tj");
    assert_eq!(
        s.len(),
        1,
        "a sub-em seam must be left to the span merger, not split here: {s:?}"
    );
}
