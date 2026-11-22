//! A fill whose colour is a shading pattern paints the gradient, not a solid.
//!
//! ISO 32000-1:2008 §8.7.4.1 (`docs/spec/pdf.md`:12899-12902):
//!
//! > By setting a shading pattern as the current colour in the graphics state,
//! > a PDF content stream may use it with painting operators such as **f**
//! > (fill), **S** (stroke), **Tj** (show text), or **Do** (paint external
//! > object) with an image mask to paint a path, character glyph, or mask with
//! > a smooth colour transition. When a shading is used in this way, the
//! > geometry of the gradient fill is independent of that of the object being
//! > painted.
//!
//! The renderer handled tiling patterns (PatternType 1) and fell back to a
//! solid colour for shading patterns (PatternType 2). That fallback painted
//! `fill_color_components`, which a *coloured* pattern never supplies: it is
//! selected by `/P0 scn` with no operands at all. So the stale fill colour was
//! used, and in a stream whose first colour operator is that `scn` the stale
//! value is the initial black — a pale gradient rendered as a black flood.
//!
//! The fixture mirrors that shape deliberately: a light gradient, selected
//! with a no-operand `scn`, with no preceding colour operator.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// One page whose whole area is filled with an axial shading pattern running
/// between two light colours.
fn shading_pattern_page() -> Vec<u8> {
    // No colour operator before `scn`: the fill colour is still the initial
    // black, which is what the old fallback would paint.
    let content = b"/Pattern cs /P0 scn 0 0 200 100 re f\n".to_vec();

    let mut pdf = Vec::new();
    let mut off = [0usize; 8];
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
         /Contents 4 0 R /Resources << /Pattern << /P0 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");

    // PatternType 2, axial (ShadingType 2), left-to-right, both ends light.
    off[5] = pdf.len();
    push!(
        "5 0 obj\n<< /Type /Pattern /PatternType 2 /Matrix [1 0 0 1 0 0] \
         /Shading << /ShadingType 2 /ColorSpace /DeviceRGB \
         /Coords [0 0 200 0] /Extend [true true] /Function 6 0 R >> >>\nendobj\n"
    );
    off[6] = pdf.len();
    push!(
        "6 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 \
         /C0 [0.8 0.9 0.8] /C1 [0.95 0.95 0.9] >>\nendobj\n"
    );

    let xref = pdf.len();
    push!("xref\n0 7\n0000000000 65535 f \r\n");
    for offset in &off[1..=6] {
        push!(format!("{:010} 00000 n \r\n", offset));
    }
    push!(format!(
        "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    pdf
}

fn render(pdf: Vec<u8>) -> image::RgbaImage {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8()
}

#[test]
fn test_shading_pattern_fill_is_not_painted_as_a_solid() {
    let px = render(shading_pattern_page());

    let mean = |c: usize| -> f64 { px.pixels().map(|p| f64::from(p[c])).sum::<f64>() / px.pixels().len() as f64 };
    let (r, g, b) = (mean(0), mean(1), mean(2));

    // Both gradient endpoints are 0.8..0.95, so every painted pixel is light.
    // The old fallback flooded the rectangle with the stale initial black.
    assert!(
        r > 180.0 && g > 180.0 && b > 180.0,
        "expected a light gradient, got mean ({r:.1}, {g:.1}, {b:.1}) — a shading \
         pattern fill is being painted as a solid colour instead of its gradient"
    );
}

#[test]
fn test_shading_pattern_fill_actually_varies_across_the_page() {
    // Not just "light": it must be a *gradient*. Sampling the two ends of the
    // axis catches a fix that paints one flat colour from the shading.
    let px = render(shading_pattern_page());
    let h = px.height() / 2;

    let left = px.get_pixel(2, h);
    let right = px.get_pixel(px.width() - 3, h);

    // C0 is greener (0.8, 0.9, 0.8), C1 is brighter overall (0.95, 0.95, 0.9);
    // the red channel separates them by ~38 levels.
    let dr = i32::from(right[0]) - i32::from(left[0]);
    assert!(
        dr > 15,
        "expected the axial gradient to brighten left-to-right, red went {} -> {} \
         (delta {dr}) — the shading is being painted as a single flat colour",
        left[0],
        right[0]
    );
}
