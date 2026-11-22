//! An `/Indexed` fill colour is the palette entry, and the index is clamped.
//!
//! ISO 32000-1:2008 §8.6.6.3 (`docs/spec/pdf.md`:11053-11054):
//!
//! > The index value should be an integer in the range 0 to *hival*. If the
//! > value is a real number, it shall be rounded to the nearest integer; if it
//! > is outside the range 0 to *hival*, it shall be adjusted to the nearest
//! > value within that range.
//!
//! Two things were wrong. The colour resolver never consulted the palette at
//! all — it returned `index / 255` as a grey level, so index 3 of a palette of
//! saturated colours painted near-black — and nothing rounded or clamped the
//! operand. On the corpus file named for this case we rendered a mean tone of
//! 222.34 where four engines agree on 231.72-235.49.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page painting one swatch per `sc` operand against a four-entry palette of
/// pure red, green, blue and white.
fn indexed_page(operands: &[&str]) -> Vec<u8> {
    let mut content = String::from("/Cs1 cs\n");
    for (i, op) in operands.iter().enumerate() {
        content.push_str(&format!("{op} sc {} 10 20 20 re f\n", 10 + i * 25));
    }
    let content = content.into_bytes();

    // hival 3; palette is RGB triples: red, green, blue, yellow.
    //
    // The last entry is deliberately NOT white. An out-of-range index used to
    // fall through to "first component as grey", which for a large operand
    // clamps to 1.0 and paints white — so a white hival entry would let the
    // clamp tests pass without any clamping happening.
    let palette: Vec<u8> = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];

    let mut pdf = Vec::new();
    let mut off = [0usize; 7];
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
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 40] \
         /Contents 4 0 R /Resources << /ColorSpace << /Cs1 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");
    off[5] = pdf.len();
    push!("5 0 obj\n[ /Indexed /DeviceRGB 3 <");
    for b in &palette {
        push!(format!("{b:02X}"));
    }
    push!("> ]\nendobj\n");

    let xref = pdf.len();
    push!("xref\n0 6\n0000000000 65535 f \r\n");
    for offset in &off[1..=5] {
        push!(format!("{:010} 00000 n \r\n", offset));
    }
    push!(format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    pdf
}

/// The colour at the centre of the swatch drawn for operand `i`.
fn swatch(operands: &[&str], i: usize) -> (u8, u8, u8) {
    let doc = PdfDocument::from_bytes(indexed_page(operands)).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    // Page is 200x40; swatch i spans x = 10+25i .. +20, y = 10..30 in PDF
    // space, which is bottom-up. Sample its centre.
    let sx = ((10 + i * 25 + 10) as f32 / 200.0 * px.width() as f32) as u32;
    let sy = (px.height() as f32 * 0.5) as u32;
    let p = px.get_pixel(sx.min(px.width() - 1), sy.min(px.height() - 1));
    (p[0], p[1], p[2])
}

#[test]
fn test_in_range_index_paints_its_palette_entry() {
    // Palette: 0 = red, 1 = green, 2 = blue, 3 = yellow.
    let ops = ["0", "1", "2"];
    assert_eq!(swatch(&ops, 0), (255, 0, 0), "index 0 should be red");
    assert_eq!(swatch(&ops, 1), (0, 255, 0), "index 1 should be green");
    assert_eq!(swatch(&ops, 2), (0, 0, 255), "index 2 should be blue");
}

#[test]
fn test_index_above_hival_snaps_to_hival() {
    // hival is 3 (yellow). 17 is far outside and must clamp to it — not
    // paint black, not read past the palette, and not fall through to a grey
    // that happens to look plausible.
    assert_eq!(swatch(&["17"], 0), (255, 255, 0));
}

#[test]
fn test_negative_index_snaps_to_zero() {
    assert_eq!(swatch(&["-17"], 0), (255, 0, 0), "should clamp to index 0, red");
}

#[test]
fn test_real_index_rounds_to_the_nearest_integer() {
    // 2.5 rounds to 3 (yellow); 1.4 rounds to 1 (green).
    assert_eq!(swatch(&["2.5"], 0), (255, 255, 0));
    assert_eq!(swatch(&["1.4"], 0), (0, 255, 0));
}
