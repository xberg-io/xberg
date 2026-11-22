//! A shading pattern keeps the affine frame it is defined in and the graphics state it declares.
//!
//! ISO 32000-1:2008 §8.7.4.3 (Table 76): the pattern's `/Matrix` maps pattern space to the parent
//! content stream's default space, and its `/ExtGState` is in effect while the pattern is painted.
//! An axial shading's colour bands are perpendicular to its axis in shading space, which a skew
//! does not preserve on the page.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A 200x100 page filled with a shading pattern whose dictionary carries `pattern_extra` and
/// whose `/Matrix` is `matrix`; the axial shading runs red to blue along `[0 0 100 0]`.
fn pattern_page(matrix: &str, pattern_extra: &str, c0: &str, c1: &str) -> Vec<u8> {
    let content = b"/Pattern cs /P0 scn 0 0 200 100 re f\n";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
         /Resources << /Pattern << /P0 5 0 R >> >> >>"
            .to_string(),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            String::from_utf8_lossy(content)
        ),
        format!(
            "<< /Type /Pattern /PatternType 2 /Matrix {matrix} {pattern_extra} \
             /Shading << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] \
             /Extend [true true] /Function 6 0 R >> >>"
        ),
        format!("<< /FunctionType 2 /Domain [0 1] /N 1 /C0 {c0} /C1 {c1} >>"),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = pdf.len();
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn render(pdf: Vec<u8>) -> image::RgbaImage {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72)).expect("page renders");
    image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8()
}

#[test]
fn test_axial_bands_follow_a_skewed_pattern_matrix() {
    // x' = x + y: the shading coordinate along the axis at device (X, Y) is X - (100 - Y).
    let px = render(pattern_page("[1 0 1 1 0 0]", "", "[1 0 0]", "[0 0 1]"));
    let low_row = px.get_pixel(100, 50)[0];
    let high_row = px.get_pixel(100, 90)[0];
    assert!(
        i32::from(low_row) - i32::from(high_row) > 50,
        "the same column must take different colours on different rows under a skew \
         (red {low_row} at row 50, {high_row} at row 90)"
    );
}

#[test]
fn test_unskewed_axial_pattern_stays_constant_down_a_column() {
    let px = render(pattern_page("[1 0 0 1 0 0]", "", "[1 0 0]", "[0 0 1]"));
    let (top, bottom) = (px.get_pixel(60, 10)[0], px.get_pixel(60, 90)[0]);
    assert!(top.abs_diff(bottom) <= 2, "red {top} vs {bottom}");
}

#[test]
fn test_pattern_ext_gstate_alpha_applies_to_the_gradient() {
    let opaque = render(pattern_page("[1 0 0 1 0 0]", "", "[0 0 0]", "[0 0 0]"));
    assert!(
        opaque.get_pixel(100, 50)[0] < 10,
        "an opaque black gradient must paint black"
    );
    let translucent = render(pattern_page(
        "[1 0 0 1 0 0]",
        "/ExtGState << /Type /ExtGState /ca 0.5 >>",
        "[0 0 0]",
        "[0 0 0]",
    ));
    let red = translucent.get_pixel(100, 50)[0];
    assert!(
        (100..=155).contains(&red),
        "a 0.5 alpha black gradient over white is mid-grey, got {red}"
    );
}
