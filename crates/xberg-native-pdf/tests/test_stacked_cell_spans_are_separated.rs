//! A table cell must not run its vertically stacked members together.
//!
//! All three cell renderers decided whether to separate two consecutive spans
//! by asking `has_horizontal_gap`, which compares x. A cell whose members are
//! stacked — at nearly the same x and different y — has no gap by that test,
//! so they were concatenated. On an architectural site plan whose contour
//! lines carry stacked elevation labels, `128` above `126` came out as
//! `128126` and `124` above `122` as `124122`; `LOCATION` above its address
//! became `LOCATION123`.
//!
//! ISO 32000-1:2008 §9.4.3 — the show operators paint the glyphs they are
//! given. A cell that stacks its members renders them on separate lines, and
//! concatenating them invents a token the page never draws. The paragraph
//! path has always separated lines; the cell path had no equivalent.

use xberg_native_pdf::PdfDocument;

/// A ruled two-cell table whose left cell holds two numbers stacked one above
/// the other at the same x — the shape of a contour label on a survey sheet.
fn stacked_cell_pdf() -> Vec<u8> {
    let content = b"0.5 w\n\
        50 40 m 350 40 l S\n\
        50 140 m 350 140 l S\n\
        50 40 m 50 140 l S\n\
        200 40 m 200 140 l S\n\
        350 40 m 350 140 l S\n\
        BT /F1 10 Tf\n\
        1 0 0 1 90 110 Tm (128) Tj\n\
        1 0 0 1 90 70 Tm (126) Tj\n\
        1 0 0 1 240 90 Tm (Elevation) Tj\n\
        ET\n";

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
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>",
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

fn assert_unfused(raw: &str, what: &str) {
    let surface = raw.to_string();
    let squashed: String = surface.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        !squashed.contains("128126"),
        "stacked cell members were concatenated in {what}:\n{surface}"
    );
    for n in ["128", "126"] {
        assert!(
            surface
                .split_whitespace()
                .any(|t| t.trim_matches(|c: char| !c.is_alphanumeric()) == n),
            "{n} must survive as its own token in {what}:\n{surface}"
        );
    }
}

#[test]
fn stacked_cell_members_are_separated_in_text() {
    let doc = PdfDocument::from_bytes(stacked_cell_pdf()).expect("parse");
    assert_unfused(&doc.extract_text(0).expect("text"), "text");
}

/// Control: two members on the SAME line with no gap between them must stay
/// joined, so the new rule cannot be a blanket "always separate".
#[test]
fn same_line_members_without_a_gap_stay_joined() {
    // 10pt Courier advances 6pt per glyph, so `co` ends exactly where `mpany`
    // begins: one word drawn as two show operations.
    let content = b"0.5 w\n\
        50 40 m 350 40 l S\n\
        50 140 m 350 140 l S\n\
        50 40 m 50 140 l S\n\
        200 40 m 200 140 l S\n\
        350 40 m 350 140 l S\n\
        BT /F1 10 Tf\n\
        1 0 0 1 90 90 Tm (co) Tj\n\
        1 0 0 1 102 90 Tm (mpany) Tj\n\
        1 0 0 1 240 90 Tm (Owner) Tj\n\
        ET\n";
    let mut pdf = stacked_cell_pdf();
    // Rebuild with the alternate content by regenerating from scratch is
    // simpler than splicing, so just assert on the shared builder's shape:
    // replace the stream wholesale and fix /Length.
    let old_stream_start = pdf.windows(7).position(|w| w == b"stream\n").expect("stream") + 7;
    let old_stream_end = pdf.windows(10).position(|w| w == b"\nendstream").expect("endstream");
    let old_len_marker = format!("<< /Length {} >>", old_stream_end - old_stream_start);
    let new_len_marker = format!("<< /Length {} >>", content.len());
    pdf.splice(old_stream_start..old_stream_end, content.iter().copied());
    let pos = pdf
        .windows(old_len_marker.len())
        .position(|w| w == old_len_marker.as_bytes())
        .expect("length marker");
    pdf.splice(pos..pos + old_len_marker.len(), new_len_marker.bytes());

    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let md = doc.extract_text(0).expect("text");
    let squashed: String = md.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        squashed.contains("company"),
        "one word drawn as two same-line show operations must stay joined:\n{md}"
    );
}
