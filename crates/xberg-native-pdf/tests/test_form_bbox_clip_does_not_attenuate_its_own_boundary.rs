//! A form's `/BBox` clip must not dim the geometry it was sized around.
//!
//! Table 95 makes the `/BBox` a clip on the form, and this release began
//! applying it. Producers routinely size the box to *exactly* the content
//! inside it, so the boundary pixels are already partially covered by the
//! content's own antialiasing. Multiplying that by the clip's partial
//! coverage attenuates a one-pixel ring that no other renderer touches.
//!
//! Measured on a luminosity soft mask whose `/BBox` maps to precisely the
//! outer extent of the stroke it bounds: with the clip we sat 0.54 grey levels
//! off MuPDF, without any clip 0.06. Rasterising the clip non-antialiased
//! instead makes it *worse* (0.68), because a box edge landing on a half-pixel
//! then loses the whole column rather than half of it.
//!
//! The clip is therefore grown by half a device pixel. At its real job —
//! excluding content that lies outside the box — half a pixel changes nothing.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page drawing a form whose `/BBox` is `bbox`, containing a filled black
/// square that spans `0 0 40 40` in form space.
fn page_with_form(bbox: &str) -> Vec<u8> {
    let content = b"/Fm0 Do\n".to_vec();
    let form = b"0 0 0 rg 0 0 40 40 re f\n".to_vec();

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
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
         /Contents 4 0 R /Resources << /XObject << /Fm0 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");
    off[5] = pdf.len();
    push!(format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox {bbox} \
         /Resources << >> /Length {} >>\nstream\n",
        form.len()
    ));
    pdf.extend_from_slice(&form);
    push!("endstream\nendobj\n");

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

fn mean_luma(pdf: Vec<u8>) -> f64 {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let n = px.pixels().len() as f64;
    px.pixels()
        .map(|p| (f64::from(p[0]) + f64::from(p[1]) + f64::from(p[2])) / 3.0)
        .sum::<f64>()
        / n
}

#[test]
fn test_bbox_sized_to_its_content_does_not_dim_it() {
    // The box is exactly the square. Clipping to it must be a no-op, so this
    // has to match a box comfortably larger than the content.
    let exact = mean_luma(page_with_form("[0 0 40 40]"));
    let loose = mean_luma(page_with_form("[-10 -10 50 50]"));
    assert!(
        (exact - loose).abs() < 0.75,
        "a /BBox sized to its own content dimmed it: exact box {exact:.2} vs \
         loose box {loose:.2}"
    );
}

#[test]
fn test_bbox_smaller_than_its_content_still_clips() {
    // The counter-case, and the reason the growth stays at half a pixel: a box
    // genuinely smaller than the content must still cut it away.
    let full = mean_luma(page_with_form("[0 0 40 40]"));
    let half = mean_luma(page_with_form("[0 0 20 40]"));
    assert!(
        half > full + 40.0,
        "a /BBox covering half the content should clip the other half away: \
         full {full:.2} vs half {half:.2}"
    );
}
