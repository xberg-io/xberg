//! A form XObject starts from the graphics state in force at its `Do`.
//!
//! ISO 32000-1:2008 §8.10.1 (`docs/spec/pdf.md`:15226-15227) closes the list
//! of what `Do` does to a form with:
//!
//! > Except as described above, the initial graphics state for the form shall
//! > be inherited from the graphics state that is in effect at the time **Do**
//! > is invoked.
//!
//! "Except as described above" covers only the `q`/`Q` bracket, the form's
//! `/Matrix` concatenated onto the CTM, and the `/BBox` clip. Everything else
//! — constant alpha, blend mode, colour and colour space — comes from the
//! caller.
//!
//! The renderer instead began every form with a fresh state reset to
//! DeviceGray black, so an alpha or colour set before `Do` was silently
//! discarded: a highlight annotation painted opaque over the text it should
//! tint, and a fill in a Separation or DeviceN space resolved to black.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page that sets constant alpha to `alpha` and then invokes a form which
/// fills the whole page with black. With inheritance the result is grey; with
/// a fresh state it is solid black.
fn page_with_alpha_then_form(alpha: f32) -> Vec<u8> {
    let content = "/GA gs /Fm0 Do\n".to_string().into_bytes();
    let form = b"0 0 0 rg 0 0 100 100 re f\n".to_vec();

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
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Contents 4 0 R /Resources << /XObject << /Fm0 5 0 R >> \
         /ExtGState << /GA 6 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");
    off[5] = pdf.len();
    push!(format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 \
         /BBox [0 0 100 100] /Resources << >> /Length {} >>\nstream\n",
        form.len()
    ));
    pdf.extend_from_slice(&form);
    push!("endstream\nendobj\n");
    off[6] = pdf.len();
    push!(format!(
        "6 0 obj\n<< /Type /ExtGState /ca {alpha} /CA {alpha} >>\nendobj\n"
    ));

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

/// Mean luminance of the rendered page, 0 (black) to 255 (white).
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
fn constant_alpha_set_before_do_applies_inside_the_form() {
    let opaque = mean_luma(page_with_alpha_then_form(1.0));
    let half = mean_luma(page_with_alpha_then_form(0.5));

    // Fully opaque: the form's black fill covers the white page.
    assert!(
        opaque < 20.0,
        "with /ca 1 the form's black fill should cover the page, mean luma {opaque:.1}"
    );

    // Half alpha: black at 50% over white is mid-grey. Without inheritance
    // the form starts from a fresh state, the alpha is lost, and this comes
    // out as black too.
    assert!(
        (90.0..165.0).contains(&half),
        "with /ca 0.5 the form's fill should be half-transparent over white \
         (mid-grey), got mean luma {half:.1} — the alpha set before Do is not \
         reaching the form's initial graphics state"
    );

    assert!(
        half > opaque + 60.0,
        "alpha made no difference inside the form: opaque {opaque:.1} vs half {half:.1}"
    );
}
