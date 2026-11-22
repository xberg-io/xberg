//! A shading's own `/BBox` bounds where it paints.
//!
//! ISO 32000-1:2008 Table 78 (`docs/spec/pdf.md`:12997):
//!
//! > **BBox** … An array of four numbers giving the left, bottom, right, and
//! > top coordinates, respectively, of the shading's bounding box. The
//! > coordinates shall be interpreted in the shading's target coordinate
//! > space. If present, this bounding box shall be applied as a temporary
//! > clipping boundary when the shading is painted, in addition to the current
//! > clipping path and any other clipping boundaries in effect at that time.
//!
//! It was not applied at all. A pattern fill covering more area than the
//! shading declares painted the whole fill: on the corpus file for this case,
//! a page filling 595x842 with a pattern whose shading is bounded to
//! `[72 72 540 720]`, we covered 74.3% of the page where four engines cover
//! 51.9%.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page that fills its whole 100x100 area with an axial shading pattern
/// whose shading carries `bbox` (or none, when empty).
fn page_with_shading_bbox(bbox: &str) -> Vec<u8> {
    let content = b"/Cs1 cs /P1 scn 0 0 100 100 re f\n".to_vec();
    let bbox_entry = if bbox.is_empty() {
        String::new()
    } else {
        format!("/BBox {bbox} ")
    };

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
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Contents 4 0 R /Resources << /ColorSpace << /Cs1 [/Pattern] >> \
         /Pattern << /P1 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");
    off[5] = pdf.len();
    push!(format!(
        "5 0 obj\n<< /Type /Pattern /PatternType 2 /Matrix [1 0 0 1 0 0] \
         /Shading << /ShadingType 2 /ColorSpace /DeviceRGB {bbox_entry}\
         /Coords [0 0 100 0] /Extend [true true] \
         /Function << /FunctionType 2 /Domain [0 1] /N 1 /C0 [1 0 0] /C1 [1 0 0] >> \
         >> >>\nendobj\n"
    ));

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

/// Fraction of the page that received the shading's red.
fn red_coverage(bbox: &str) -> f64 {
    let doc = PdfDocument::from_bytes(page_with_shading_bbox(bbox)).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let n = px.pixels().len() as f64;
    px.pixels()
        .filter(|p| p[3] > 0 && p[0] > 200 && p[1] < 80 && p[2] < 80)
        .count() as f64
        / n
}

#[test]
fn test_shading_without_a_bbox_paints_the_whole_fill() {
    let all = red_coverage("");
    assert!(
        all > 0.95,
        "an unbounded shading should paint the entire fill, got {all:.4}"
    );
}

#[test]
fn test_shading_bbox_bounds_the_paint_to_itself() {
    // The box is the left quarter of a full-page fill, so about 25% should
    // paint. Without the clip the pattern floods the whole page.
    let quarter = red_coverage("[0 0 25 100]");
    assert!(
        (0.18..0.32).contains(&quarter),
        "a /BBox covering a quarter of the fill painted {quarter:.4} of the \
         page; without the clip it floods to ~1.0"
    );
}
