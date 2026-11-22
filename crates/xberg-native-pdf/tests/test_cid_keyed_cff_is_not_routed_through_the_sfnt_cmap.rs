//! A `CIDFontType0`'s codes are CIDs, resolved through the CFF — never
//! through the font's `cmap` table.
//!
//! ISO 32000-1:2008 §9.7.4.2 (`docs/spec/pdf.md`:18643-18652) gives the two
//! cases for a CIDFont with a CFF program:
//!
//! > The "CFF" font program has a Top DICT that uses CIDFont operators: The
//! > CIDs shall be used to determine the GID value for the glyph procedure
//! > using the charset table in the CFF program.
//!
//! > The "CFF" font program has a Top DICT that does not use CIDFont
//! > operators: The CIDs shall be used directly as GID values.
//!
//! Either way the route runs through the CFF. The `cmap` belongs to the Type 2
//! mechanism, which the same clause describes separately as TrueType's way of
//! mapping "character codes to glyph indices".
//!
//! The trap is that Table 126 (pdf.md:19786) *requires* an OpenType
//! `CIDFontType0` to include a `cmap` table. Its presence therefore says
//! nothing about how codes should be resolved — but the renderer dispatched on
//! "does this font have a Unicode cmap?" and let that win, sending CIDs
//! through a Unicode lookup where they resolved to glyph 0. A real page whose
//! entire content was one `Tj` rendered blank for that reason.
//!
//! The fixture font is the repo's own `StandardSymbolsPS.otf`: an `OTTO`
//! (CFF) program that carries a `cmap`, which is exactly the shape that
//! triggered the misdispatch.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const OTF: &[u8] = include_bytes!("fixtures/fonts/StandardSymbolsPS.otf");

/// A page showing four glyphs by GID through an Identity-H CIDFontType0.
fn page_with_cid_keyed_cff() -> Vec<u8> {
    // Identity-H makes code == CID; this CFF has no CIDFont operators in its
    // Top DICT, so the CIDs are used directly as GID values. GID 0 is
    // .notdef, so ask for four real glyphs.
    let content = b"BT /F1 36 Tf 20 40 Td <0001000200030004> Tj ET\n".to_vec();

    let mut pdf = Vec::new();
    let mut off = [0usize; 10];
    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }
    push!("%PDF-1.7\n");
    off[1] = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    off[2] = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    off[3] = pdf.len();
    push!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");
    off[5] = pdf.len();
    push!(
        "5 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /StandardSymbolsPS \
         /Encoding /Identity-H /DescendantFonts [6 0 R] >>\nendobj\n"
    );
    off[6] = pdf.len();
    push!(
        "6 0 obj\n<< /Type /Font /Subtype /CIDFontType0 /BaseFont /StandardSymbolsPS \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         /FontDescriptor 7 0 R /DW 1000 >>\nendobj\n"
    );
    off[7] = pdf.len();
    push!(
        "7 0 obj\n<< /Type /FontDescriptor /FontName /StandardSymbolsPS /Flags 4 \
         /FontBBox [-200 -300 1200 1000] /ItalicAngle 0 /Ascent 900 /Descent -200 \
         /CapHeight 700 /StemV 80 /FontFile3 8 0 R >>\nendobj\n"
    );
    off[8] = pdf.len();
    push!(format!(
        "8 0 obj\n<< /Subtype /OpenType /Length {} >>\nstream\n",
        OTF.len()
    ));
    pdf.extend_from_slice(OTF);
    push!("\nendstream\nendobj\n");

    let xref = pdf.len();
    push!("xref\n0 9\n0000000000 65535 f \r\n");
    for offset in &off[1..=8] {
        push!(format!("{:010} 00000 n \r\n", offset));
    }
    push!(format!(
        "trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    pdf
}

#[test]
fn glyphs_render_for_a_cid_keyed_cff_that_also_carries_a_cmap() {
    let doc = PdfDocument::from_bytes(page_with_cid_keyed_cff()).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();

    let inked = px
        .pixels()
        .filter(|p| p[3] > 0 && (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3 < 200)
        .count();

    assert!(
        inked > 200,
        "no glyphs were painted ({inked} inked pixels) — the CIDs are being \
         resolved through the font's cmap instead of its CFF"
    );
}
