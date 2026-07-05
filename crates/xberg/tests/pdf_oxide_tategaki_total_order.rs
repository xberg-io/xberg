//! Regression test for xberg #1198 — pdf_oxide's tategaki
//! (vertical-writing) reading-order sort panicked with *"user-provided
//! comparison function does not correctly implement a total order"* on pages
//! whose vertical-mode span X-centers chain closer together than the
//! median span width (scanned pages with vertical `Identity-V` OCR layers,
//! typeset tategaki books).
//!
//! The `guard_oxide_panic` wrapper already keeps the panic from aborting the
//! whole extraction, but the page's text is lost. This test asserts the real
//! fix (yfedoseev/pdf_oxide#808): extraction must SUCCEED and return the
//! page's text, not merely survive.
//!
//! The PDF is hand-built (no third-party fixture): one page, an `Identity-V`
//! (vertical writing mode) Type0 font, 240 single-glyph runs whose X
//! positions step by 0.8 pt. The extracted spans carry `wmode = 1` with a
//! ~1 pt median width, so the tategaki sort sees a long chain of
//! "same column" neighbors spanning far more than the tolerance — the
//! intransitive case that panicked pdf_oxide ≤ 0.3.72.

#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::ExtractionConfig;

/// Build a minimal single-page PDF whose text is emitted under a vertical
/// (`Identity-V`) CMap at chained X positions.
fn make_identity_v_chained_pdf() -> Vec<u8> {
    let mut stream = String::from("/F1 12 Tf\n");
    for i in 0..240 {
        let x = 20.0 + i as f32 * 0.8;
        let y = 700 - ((i * 37) % 96) * 7;
        stream.push_str(&format!("BT 1 0 0 1 {x:.1} {y} Tm <0041> Tj ET\n"));
    }

    let mut pdf: Vec<u8> = Vec::new();

    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }

    push!("%PDF-1.4\n");

    let off1 = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    push!(
        "3 0 obj\n\
         << /Type /Page /Parent 2 0 R /MediaBox [0 0 842 792]\
         \n   /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\n\
         endobj\n"
    );

    let off4 = pdf.len();
    let stream_bytes = stream.as_bytes();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", stream_bytes.len()));
    pdf.extend_from_slice(stream_bytes);
    push!("\nendstream\nendobj\n");

    let off5 = pdf.len();
    push!(
        "5 0 obj\n\
         << /Type /Font /Subtype /Type0 /BaseFont /MingLiU /Encoding /Identity-V\
         \n   /DescendantFonts [6 0 R] >>\n\
         endobj\n"
    );

    let off6 = pdf.len();
    push!(
        "6 0 obj\n\
         << /Type /Font /Subtype /CIDFontType2 /BaseFont /MingLiU\
         \n   /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>\
         \n   /FontDescriptor 7 0 R /DW 1000 /CIDToGIDMap /Identity >>\n\
         endobj\n"
    );

    let off7 = pdf.len();
    push!(
        "7 0 obj\n\
         << /Type /FontDescriptor /FontName /MingLiU /Flags 4\
         \n   /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 800\
         \n   /Descent -200 /CapHeight 700 /StemV 80 >>\n\
         endobj\n"
    );

    let xref_off = pdf.len();
    push!(format!(
        "xref\n0 8\n\
         0000000000 65535 f \r\n\
         {off1:010} 00000 n \r\n\
         {off2:010} 00000 n \r\n\
         {off3:010} 00000 n \r\n\
         {off4:010} 00000 n \r\n\
         {off5:010} 00000 n \r\n\
         {off6:010} 00000 n \r\n\
         {off7:010} 00000 n \r\n"
    ));
    push!(format!(
        "trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n"
    ));

    pdf
}

/// A vertical-majority page with chained X-centers must extract its text —
/// not panic (pdf_oxide ≤ 0.3.72) and not fall back to a guarded per-page
/// error that drops the content.
#[test]
fn test_identity_v_chained_centers_extracts_text() {
    let pdf = make_identity_v_chained_pdf();
    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(&pdf, "application/pdf", &config)
        .expect("Identity-V chained-centers PDF must extract without error");

    let non_ws = result.content.chars().filter(|c| !c.is_whitespace()).count();
    assert!(
        non_ws >= 240,
        "vertical page text was lost (got {non_ws} non-whitespace chars, want ≥ 240):\n{:?}",
        result.content
    );
}
