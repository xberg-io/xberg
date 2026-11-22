//! A page may declare a `/DefaultGray`, `/DefaultRGB` or `/DefaultCMYK`
//! override whose component count differs from the paint operator that
//! reaches it. ISO 32000-1:2008 §8.6.5.6 makes a bare `g`/`rg`/`k` behave as
//! if it had named the override colour space, so a one-operand `0.5 g` under
//! `/DefaultGray [/DeviceCMYK]` arrives at the DeviceCMYK projection with a
//! single component.
//!
//! The colour space's declared family and the operand count the content
//! stream supplies are independent, so every projection must be total over
//! the slice it is handed. Two of the three dispatch sites guarded for that
//! and one did not; the unguarded one indexed `components[1..3]` and, with
//! `panic = "abort"` in the release profile, took the host process down on a
//! file that is otherwise ordinary.
//!
//! Degrading to the first component as gray is the same disposition the two
//! guarded sites already used.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// One-page PDF that fills its whole media box with `<paint>` while
/// `/Resources /ColorSpace` declares `<default_key> <default_space>`.
fn override_pdf(default_key: &str, default_space: &str, paint: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 5];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        &format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << /ColorSpace << {default_key} {default_space} >> >> \
             /Contents 4 0 R >>"
        ),
    );
    let content = format!("{paint} 0 0 200 200 re f\n");
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in &off[1..=4] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// Render page 0 at 72 dpi and return the centre pixel as RGBA.
fn centre_pixel(pdf: Vec<u8>) -> [u8; 4] {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    let (w, h) = (img.width as usize, img.height as usize);
    let at = (h / 2 * w + w / 2) * 4;
    [img.data[at], img.data[at + 1], img.data[at + 2], img.data[at + 3]]
}

/// The reachable case: a one-operand `g` under a four-component override.
/// Before the fix this indexed past the end of the operand slice and
/// aborted the process; `render_page` returning at all is the assertion.
#[test]
fn gray_operand_under_cmyk_default_space_renders() {
    let px = centre_pixel(override_pdf("/DefaultGray", "[/DeviceCMYK]", "0.5 g"));
    assert_eq!(
        &px[..3],
        &[128, 128, 128],
        "a single operand should degrade to first-component gray, got {px:?}"
    );
    assert_eq!(px[3], 255, "fill must be opaque");
}

/// Same shape against the three-component projection.
#[test]
fn gray_operand_under_rgb_default_space_renders() {
    let px = centre_pixel(override_pdf("/DefaultGray", "[/DeviceRGB]", "0.25 g"));
    assert_eq!(
        &px[..3],
        &[64, 64, 64],
        "a single operand should degrade to first-component gray, got {px:?}"
    );
}

/// Three operands reaching the four-component projection — the same
/// precondition, one component short rather than three.
#[test]
fn rgb_operand_under_cmyk_default_space_renders() {
    let px = centre_pixel(override_pdf("/DefaultRGB", "[/DeviceCMYK]", "0.75 0.1 0.1 rg"));
    assert_eq!(
        &px[..3],
        &[191, 191, 191],
        "three operands against a CMYK projection should degrade to gray, got {px:?}"
    );
}

/// The full-arity path must be untouched by the degradation: an override
/// that matches its operator still projects every component.
#[test]
fn matching_operand_count_projects_all_components() {
    let px = centre_pixel(override_pdf("/DefaultRGB", "[/DeviceRGB]", "1 0 0 rg"));
    assert_eq!(
        &px[..3],
        &[255, 0, 0],
        "a matching override must still project all three components, got {px:?}"
    );
}
