//! A Pattern colour space named through a resource must be recognised as one.
//!
//! `gs.fill_color_space` stores the *resource name* the content stream used,
//! so the renderer's `== "Pattern"` test only matched a stream that wrote
//! `/Pattern cs` literally. A file reaching the space the ordinary way —
//! `/CS0 cs` where `/CS0` resolves to `/Pattern` — was not recognised as a
//! pattern at all: `scn` never recorded the pattern name, the tiling
//! rasteriser was never invoked, and the fill fell through to the solid-colour
//! path with `fill_color_rgb` at its untouched default, black.
//!
//! ISO 32000-1:2008 §8.7.3.2: in a Pattern space the `scn` operands name a
//! pattern, and §8.6.8 reaches that space through the resource dictionary like
//! any other. A whole-page fill therefore came out solid black.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A 100x100 page filled through `/CS0 cs /P0 scn`, where `/CS0` resolves to
/// `/Pattern` and `/P0` is a coloured tiling pattern painting solid red.
fn pattern_by_resource_name_pdf() -> Vec<u8> {
    let page_content = b"/CS0 cs /P0 scn 0 0 100 100 re f";
    let tile = b"1 0 0 rg 0 0 10 10 re f";

    let mut pdf: Vec<u8> = Vec::new();
    let mut off = [0usize; 7];
    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }
    push!("%PDF-1.4\n");
    off[1] = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    off[2] = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    off[3] = pdf.len();
    push!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Contents 4 0 R /Resources << /ColorSpace << /CS0 /Pattern >> \
         /Pattern << /P0 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", page_content.len()));
    pdf.extend_from_slice(page_content);
    push!("\nendstream\nendobj\n");
    off[5] = pdf.len();
    push!(format!(
        "5 0 obj\n<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
         /BBox [0 0 10 10] /XStep 10 /YStep 10 /Resources << >> /Length {} >>\nstream\n",
        tile.len()
    ));
    pdf.extend_from_slice(tile);
    push!("\nendstream\nendobj\n");

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

/// Mean RGB of the rendered page.
fn mean_rgb(pdf: Vec<u8>) -> (f64, f64, f64) {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let n = px.pixels().len() as f64;
    let (mut r, mut g, mut b) = (0f64, 0f64, 0f64);
    for p in px.pixels() {
        r += p[0] as f64;
        g += p[1] as f64;
        b += p[2] as f64;
    }
    (r / n, g / n, b / n)
}

#[test]
fn test_tiling_pattern_named_through_a_resource_paints_its_own_colour() {
    let (r, g, b) = mean_rgb(pattern_by_resource_name_pdf());

    // The tile paints pure red. Black — the old solid fallback — has a low
    // red channel too, so the discriminating fact is that red is HIGH while
    // green and blue are low.
    assert!(
        r > 200.0,
        "the pattern's own red must reach the page; got r={r:.1} g={g:.1} b={b:.1}. \
         An r near zero means the fill fell through to the solid-colour default."
    );
    assert!(
        g < 60.0 && b < 60.0,
        "the fill must be the pattern's red, not white or grey; \
         got r={r:.1} g={g:.1} b={b:.1}"
    );
}
