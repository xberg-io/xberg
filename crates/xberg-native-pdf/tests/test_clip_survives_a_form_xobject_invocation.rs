//! The clipping path in force at `Do` also bounds the form's content.
//!
//! ISO 32000-1:2008 §8.10.2 lists what invoking a form XObject does to the
//! graphics state, and clipping to the form's `/BBox` is *in addition to*
//! what is already in force. §8.5.4 makes clipping cumulative: a new clipping
//! path "shall be intersected with the current clipping path". Nothing in
//! either clause lets a form escape the clip that was set before it.
//!
//! The renderer tracked the clip in a stack local to `execute_operators` and
//! passed it to the image branch of `Do` but not to the form branch, so the
//! form was rendered with only its own `/BBox`. A form whose `/BBox` covers
//! the page, invoked inside a small `re W n`, therefore painted across
//! everything the clip existed to exclude.
//!
//! Producers rely on this: a logo drawn by a shared form whose box spans the
//! whole sheet is routinely clipped down to its corner at the call site.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// A page that clips to a 30x30 box, flips the CTM, then invokes a form whose
/// `/BBox` is the whole 100x100 page and which fills all of it.
///
/// The flip matters: the clip is established in the pre-flip space, so a
/// renderer that re-evaluates it in the form's own space rather than carrying
/// the rasterised mask would place it wrongly.
fn clipped_full_page_form() -> Vec<u8> {
    let form = b"0 0 0 0.2 k\n0 0 100 100 re\nf\n";
    let content = b"q\n20 20 30 30 re\nW n\nq\n1 0 0 -1 0 100 cm\n/Fm0 Do\nQ\nQ\n";

    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
           /Resources << /XObject << /Fm0 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content.to_vec(),
            b"\nendstream".to_vec(),
        ]
        .concat(),
        [
            format!(
                "<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
                 /Length {} >>\nstream\n",
                form.len()
            )
            .into_bytes(),
            form.to_vec(),
            b"\nendstream".to_vec(),
        ]
        .concat(),
    ];

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

/// Fraction of pixels that carry any ink.
fn ink_fraction(pdf: &[u8]) -> f64 {
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("open");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    let total = img.data.len() / 4;
    let inked = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|px| px[0] < 250 || px[1] < 250 || px[2] < 250)
        .count();
    inked as f64 / total as f64
}

#[test]
fn test_form_is_bounded_by_the_clip_in_force_at_its_do() {
    let ink = ink_fraction(&clipped_full_page_form());

    // The clip is 30x30 on a 100x100 page: 9% of it. MuPDF, pdfium and
    // poppler all render exactly that. Unclipped, the form covers everything.
    assert!(
        ink < 0.15,
        "the form must be bounded by the 30x30 clip (~0.09 of the page); \
         got {ink:.5}, which means the clip was dropped at the form boundary"
    );
    assert!(
        ink > 0.03,
        "the form must still paint inside the clip; got {ink:.5}, which means \
         the clip erased it rather than bounding it"
    );
}
