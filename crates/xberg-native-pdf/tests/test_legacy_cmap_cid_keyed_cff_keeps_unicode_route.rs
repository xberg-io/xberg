//! A `CIDFontType0` under a predefined CMap this crate carries no code-to-CID
//! table for (`90ms-RKSJ-H`) has no CID for its codes, so it must not take the
//! CID-keyed direct CFF route, where the raw code would be read as a CID and
//! select an unrelated glyph. It keeps the Unicode route instead.
//!
//! The fixture is the repo's `StandardSymbolsPS.otf`, an OpenType CFF whose
//! `cmap` has no glyph for U+0020 but whose GID 32 is a real glyph, so a page
//! of code `0x20` paints nothing on the Unicode route and paints a glyph if
//! the code is taken as a CID.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const OTF: &[u8] = include_bytes!("fixtures/fonts/StandardSymbolsPS.otf");

/// A page showing `content_hex` through a CIDFontType0 under the given `/Encoding` name.
fn page_with_cid_keyed_cff(encoding: &str, content_hex: &str) -> Vec<u8> {
    let content = format!("BT /F1 36 Tf 20 40 Td <{content_hex}> Tj ET\n").into_bytes();

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
    push!(format!(
        "5 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /StandardSymbolsPS \
         /Encoding /{encoding} /DescendantFonts [6 0 R] >>\nendobj\n"
    ));
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

fn inked_pixels(pdf: Vec<u8>) -> usize {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    px.pixels()
        .filter(|p| p[3] > 0 && (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3 < 200)
        .count()
}

#[test]
fn identity_h_control_paints_gid_32_directly() {
    let inked = inked_pixels(page_with_cid_keyed_cff("Identity-H", "0020002000200020"));
    assert!(
        inked > 200,
        "control failed: CID 32 painted nothing ({inked} inked pixels)"
    );
}

#[test]
fn legacy_predefined_cmap_codes_are_not_read_as_cids() {
    let inked = inked_pixels(page_with_cid_keyed_cff("90ms-RKSJ-H", "20202020"));
    assert!(
        inked < 50,
        "code 0x20 was taken as CID 32 and painted a glyph ({inked} inked pixels)"
    );
}
