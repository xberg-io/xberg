//! Shading patterns keep their blend mode, their pattern matrix on radial geometry and their flat
//! stencil fallback; a Type 2 function honours `/Range`; a form's `/BBox` may be indirect.
//!
//! ISO 32000-1:2008 §7.10.3 (Type 2 `/Range`), §8.7.4.3 (pattern `/ExtGState`, `/Matrix`),
//! §7.3.10 (indirect objects), §8.10.2 (form `/BBox` clip).

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

fn build_pdf(objects: &[String]) -> Vec<u8> {
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

fn stream(dict: &str, data: &str) -> String {
    format!("<< {dict} /Length {} >>\nstream\n{data}\nendstream", data.len())
}

/// A 200x100 page whose content stream is `content`; `resources` fills the page's `/Resources`
/// and `extra` supplies objects from number 5.
fn page(content: &str, resources: &str, extra: &[String]) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << {resources} >> >>"),
        stream("", content),
    ];
    objects.extend_from_slice(extra);
    build_pdf(&objects)
}

fn render(pdf: Vec<u8>) -> image::RgbaImage {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72)).expect("page renders");
    image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8()
}

#[test]
fn test_type2_function_range_limits_its_output() {
    let pdf = page(
        "/Sh0 sh",
        "/Shading << /Sh0 5 0 R >>",
        &[
            "<< /ShadingType 2 /ColorSpace /DeviceGray /Coords [0 0 200 0] /Extend [true true] /Function 6 0 R >>"
                .to_string(),
            "<< /FunctionType 2 /Domain [0 1] /Range [0.25 0.75] /C0 [0] /C1 [1] /N 1 >>".to_string(),
        ],
    );
    let px = render(pdf);
    let (left, right) = (px.get_pixel(0, 50)[0], px.get_pixel(199, 50)[0]);
    assert!(
        (60..=70).contains(&left),
        "left end must clamp to 0.25 (about 64), got {left}"
    );
    assert!(
        (185..=195).contains(&right),
        "right end must clamp to 0.75 (about 191), got {right}"
    );
}

fn form_clipped_by_bbox(bbox: &str, extra: &[String]) -> image::RgbaImage {
    let mut objects = vec![
        stream(
            &format!("/Type /XObject /Subtype /Form /BBox {bbox}"),
            "1 0 0 rg 0 0 200 100 re f",
        ),
        "[0 0 50 50]".to_string(),
        "50".to_string(),
    ];
    objects.extend_from_slice(extra);
    render(page("/F0 Do", "/XObject << /F0 5 0 R >>", &objects))
}

#[test]
fn test_indirect_form_bbox_clips_like_an_inline_one() {
    for bbox in ["[0 0 50 50]", "6 0 R", "[0 0 7 0 R 50]"] {
        let px = form_clipped_by_bbox(bbox, &[]);
        assert_eq!(px.get_pixel(25, 75).0, [255, 0, 0, 255], "{bbox}: inside the box");
        assert_eq!(px.get_pixel(150, 75).0, [255, 255, 255, 255], "{bbox}: outside the box");
    }
}

/// An axial pattern over a flat fill, with `pattern_extra` in the pattern dictionary and `shading`
/// as the shading dictionary.
fn pattern_over_yellow(matrix: &str, pattern_extra: &str, shading: &str, c0: &str, c1: &str) -> image::RgbaImage {
    render(page(
        "1 1 0 rg 0 0 200 100 re f /Pattern cs /P0 scn 0 0 200 100 re f",
        "/Pattern << /P0 5 0 R >>",
        &[
            format!(
                "<< /Type /Pattern /PatternType 2 /Matrix {matrix} {pattern_extra} /Shading << {shading} /Function 6 0 R >> >>"
            ),
            format!("<< /FunctionType 2 /Domain [0 1] /N 1 /C0 {c0} /C1 {c1} >>"),
        ],
    ))
}

#[test]
fn test_shading_pattern_blend_mode_applies_to_axial_and_radial() {
    let axial = "/ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 200 0] /Extend [true true]";
    let radial = "/ShadingType 3 /ColorSpace /DeviceRGB /Coords [100 50 0 100 50 300] /Extend [true true]";
    for (name, shading) in [("axial", axial), ("radial", radial)] {
        let px = pattern_over_yellow(
            "[1 0 0 1 0 0]",
            "/ExtGState << /Type /ExtGState /BM /Multiply >>",
            shading,
            "[0 1 1]",
            "[0 1 1]",
        );
        let p = px.get_pixel(100, 50);
        assert!(
            p[0] < 10 && p[1] > 235 && p[2] < 20,
            "{name}: cyan multiplied over yellow is green, got {:?}",
            p.0
        );
    }
}

#[test]
fn test_radial_pattern_keeps_a_non_uniform_matrix() {
    // End radius 20 under a (2, 1) scale about (100, 50): an ellipse of radii 40 and 20.
    let px = pattern_over_yellow(
        "[2 0 0 1 100 50]",
        "",
        "/ShadingType 3 /ColorSpace /DeviceRGB /Coords [0 0 0 0 0 20] /Extend [false false]",
        "[1 0 0]",
        "[0 0 1]",
    );
    let yellow = [255, 255, 0, 255];
    assert_eq!(
        px.get_pixel(100, 20).0,
        yellow,
        "30 px above the centre is outside the y radius of 20"
    );
    assert_ne!(
        px.get_pixel(130, 50).0,
        yellow,
        "30 px beside the centre is inside the x radius of 40"
    );
}

fn stencil_with_pattern(pattern: &str) -> image::RgbaImage {
    render(page(
        "/Cs1 cs /P1 scn 0 0 200 100 re W n q 200 0 0 100 0 0 cm /Im0 Do Q",
        "/ColorSpace << /Cs1 [/Pattern] >> /Pattern << /P1 6 0 R >> /XObject << /Im0 5 0 R >>",
        &[
            stream(
                "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ImageMask true /BitsPerComponent 1",
                &"\u{0}".repeat(8),
            ),
            pattern.to_string(),
        ],
    ))
}

#[test]
fn test_stencil_over_a_shadingless_pattern_keeps_its_flat_fill() {
    let px = stencil_with_pattern("<< /Type /Pattern /PatternType 2 /Matrix [1 0 0 1 0 0] >>");
    assert!(
        px.pixels().any(|p| p[0] < 60 && p[1] < 60 && p[2] < 60),
        "a pattern with no /Shading must leave the stencil painted in the flat colour"
    );
}
