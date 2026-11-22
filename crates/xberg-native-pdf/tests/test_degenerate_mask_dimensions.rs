//! A mask whose declared geometry is degenerate must not take the process
//! down with it.
//!
//! ISO 32000-1:2008 Table 89 requires `/Width` and `/Height` to be positive
//! integers, and §8.9.5.1 mandates the image-to-user matrix
//! `[1/w 0 0 -1/h 0 1]`, which is undefined at zero — so zero is invalid
//! rather than valid-but-empty, and a negative or out-of-range value is not
//! a number to truncate.
//!
//! Two defects met here. The extractor cast both entries with `as u32`, so
//! `-1` became 4294967295 and `2^32` became 0. The `/SMask` resample loop
//! then computed `sw - 1` on a zero width, which underflows to `u32::MAX` in
//! release and indexes out of bounds — and with `panic = "abort"` in the
//! release profile that is a host-process abort from a file that parses.
//! The sibling `/Mask` loop had been hardened for exactly this and the
//! `/SMask` copy had not, because the two are the same operation written
//! twice.
//!
//! Every assertion here is "the render returns and the page is intact".

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// A PDF stream object: its dictionary and its raw data.
struct Obj<'a>(String, &'a [u8]);

/// Assemble a one-page PDF whose content stream is `content`, with `objects`
/// numbered from 5 upward and the page resources given by `resources`.
fn build_pdf(resources: &str, content: &str, objects: &[Obj<'_>]) -> Vec<u8> {
    let total = 4 + objects.len();
    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; total + 1];

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let plain = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    plain(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    plain(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    plain(
        &mut buf,
        &mut off,
        3,
        &format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << {resources} >> /Contents 4 0 R >>"
        ),
    );

    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    for (i, o) in objects.iter().enumerate() {
        let id = 5 + i;
        off[id] = buf.len();
        let Obj(dict, data) = o;
        buf.extend_from_slice(format!("{id} 0 obj\n{dict}\nstream\n").as_bytes());
        buf.extend_from_slice(data);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total + 1).as_bytes());
    for offset in &off[1..=total] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n", total + 1).as_bytes());
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// A 2x2 opaque red DeviceRGB image, 8 bits per component.
const BASE_RGB: [u8; 12] = [255, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0];

/// Render page 0 and return `(width, height)` of the produced pixmap.
/// Reaching this at all is the assertion — the defect was an abort.
fn render_dimensions(pdf: Vec<u8>) -> (u32, u32) {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render must return, not abort");
    (img.width, img.height)
}

/// Build a base image carrying `/SMask 6 0 R`, where the soft mask declares
/// `smask_dims`.
fn pdf_with_smask(smask_dims: &str, smask_data: &[u8]) -> Vec<u8> {
    let base = Obj(
        format!(
            "<< /Type /XObject /Subtype /Image /Width 2 /Height 2 \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R \
             /Length {} >>",
            BASE_RGB.len()
        ),
        &BASE_RGB,
    );
    let smask = Obj(
        format!(
            "<< /Type /XObject /Subtype /Image {smask_dims} \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {} >>",
            smask_data.len()
        ),
        smask_data,
    );
    build_pdf(
        "/XObject << /Im0 5 0 R >>",
        "q 200 0 0 200 0 0 cm /Im0 Do Q\n",
        &[base, smask],
    )
}

/// The reported shape: a valid base image whose soft mask declares a zero
/// height. Before the fix the resample loop computed `sh - 1` on `0u32`.
#[test]
fn zero_height_smask_does_not_abort_the_render() {
    let (w, h) = render_dimensions(pdf_with_smask("/Width 2 /Height 0", &[]));
    assert_eq!((w, h), (200, 200), "page must still render at media size");
}

/// The other axis of the same degeneracy.
#[test]
fn zero_width_smask_does_not_abort_the_render() {
    let (w, h) = render_dimensions(pdf_with_smask("/Width 0 /Height 2", &[]));
    assert_eq!((w, h), (200, 200));
}

/// A negative dimension. `as u32` turned this into 4294967295, which then
/// flowed into allocation and sampling arithmetic.
#[test]
fn negative_smask_dimension_does_not_abort_the_render() {
    let (w, h) = render_dimensions(pdf_with_smask("/Width 2 /Height -1", &[]));
    assert_eq!((w, h), (200, 200));
}

/// A dimension past `u32`. `as u32` truncated `2^32` to zero.
#[test]
fn out_of_range_smask_dimension_does_not_abort_the_render() {
    let (w, h) = render_dimensions(pdf_with_smask("/Width 2 /Height 4294967296", &[]));
    assert_eq!((w, h), (200, 200));
}

/// The `/Mask` sibling was already guarded; pin it so the shared helper
/// cannot regress it while fixing the `/SMask` copy.
#[test]
fn zero_dimension_mask_does_not_abort_the_render() {
    let base = Obj(
        format!(
            "<< /Type /XObject /Subtype /Image /Width 2 /Height 2 \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask 6 0 R \
             /Length {} >>",
            BASE_RGB.len()
        ),
        &BASE_RGB,
    );
    let mask = Obj(
        "<< /Type /XObject /Subtype /Image /Width 2 /Height 0 /ImageMask true \
         /BitsPerComponent 1 /Length 0 >>"
            .to_string(),
        &[],
    );
    let pdf = build_pdf(
        "/XObject << /Im0 5 0 R >>",
        "q 200 0 0 200 0 0 cm /Im0 Do Q\n",
        &[base, mask],
    );
    let (w, h) = render_dimensions(pdf);
    assert_eq!((w, h), (200, 200));
}

/// A degenerate mask must not suppress the base image: with no usable mask
/// sample the base stays opaque rather than vanishing.
#[test]
fn base_image_survives_a_degenerate_smask() {
    let doc = PdfDocument::from_bytes(pdf_with_smask("/Width 2 /Height 0", &[])).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    let (w, h) = (img.width as usize, img.height as usize);
    let at = (h / 2 * w + w / 2) * 4;
    assert_eq!(
        &img.data[at..at + 3],
        &[255, 0, 0],
        "the base image should still paint when its soft mask is unusable"
    );
}
