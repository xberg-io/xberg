//! A form XObject's `/BBox` clips what its content stream paints.
//!
//! ISO 32000-1:2008 §8.10.2 step (c) (`docs/spec/pdf.md:15219`): the form's
//! `/BBox`, expressed in form space and transformed by `/Matrix`, is
//! intersected with the current clipping path before the content stream is
//! executed. Table 78 makes `/BBox` a required entry for exactly this reason.
//!
//! It was not applied at all, so a form painting outside its own bounding box
//! bled onto the page.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// A page that paints a white backdrop, then draws a form whose content fills
/// a 200x200 black square while its `/BBox` admits only part of it.
fn form_pdf(bbox: &str, matrix: &str) -> Vec<u8> {
    form_pdf_with_body(bbox, matrix, "0 0 0 rg 0 0 200 200 re f\n")
}

/// The same page with the form's content stream supplied verbatim.
fn form_pdf_with_body(bbox: &str, matrix: &str, body: &str) -> Vec<u8> {
    let form = body.as_bytes();

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 6];
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
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
         /Resources << /XObject << /Fm0 5 0 R >> >> /Contents 4 0 R >>",
    );
    let content = b"1 1 1 rg 0 0 200 200 re f\nq /Fm0 Do Q\n";
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    off[5] = buf.len();
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Form {bbox} {matrix} /Length {} >>\nstream\n",
            form.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(form);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in &off[1..=5] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// Is the pixel at PDF-space `(x, y)` dark?
fn is_dark_at(pdf: &[u8], x: usize, y_pdf: usize) -> bool {
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    let (w, h) = (img.width as usize, img.height as usize);
    // PDF y is bottom-up; the raster is top-down.
    let y = h.saturating_sub(1 + y_pdf);
    let i = (y.min(h - 1) * w + x.min(w - 1)) * 4;
    img.data[i] < 128 && img.data[i + 1] < 128 && img.data[i + 2] < 128
}

/// The reported symptom: content outside the `/BBox` must not paint.
#[test]
fn content_outside_the_bbox_is_clipped_away() {
    // The form fills 0..200 but declares only the lower-left 50x50.
    let pdf = form_pdf("/BBox [0 0 50 50]", "");
    assert!(is_dark_at(&pdf, 25, 25), "inside the /BBox should be painted");
    assert!(
        !is_dark_at(&pdf, 150, 150),
        "outside the /BBox must be clipped away, but it painted"
    );
}

/// A `/BBox` covering the whole form leaves it fully painted — the clip must
/// not shrink content that is legitimately inside.
#[test]
fn test_bbox_covering_the_content_clips_nothing() {
    let pdf = form_pdf("/BBox [0 0 200 200]", "");
    assert!(is_dark_at(&pdf, 25, 25));
    assert!(
        is_dark_at(&pdf, 150, 150),
        "a /BBox that admits everything must not clip"
    );
}

/// The box is in form space, so `/Matrix` moves it with the content.
#[test]
fn test_bbox_is_transformed_by_matrix() {
    // Translate the form by (100, 100); its 50x50 box lands at 100..150.
    let pdf = form_pdf("/BBox [0 0 50 50]", "/Matrix [1 0 0 1 100 100]");
    assert!(
        is_dark_at(&pdf, 125, 125),
        "the translated /BBox region should be painted"
    );
    assert!(
        !is_dark_at(&pdf, 25, 25),
        "the untranslated position must now be outside the box"
    );
}

/// §7.9.5 allows either diagonal, so a reversed box describes the same region.
#[test]
fn test_reversed_bbox_describes_the_same_region() {
    let pdf = form_pdf("/BBox [50 50 0 0]", "");
    assert!(is_dark_at(&pdf, 25, 25), "reversed corners must normalise");
    assert!(!is_dark_at(&pdf, 150, 150));
}

/// A degenerate box cannot be honoured; refusing to clip keeps the content
/// rather than erasing the form entirely.
#[test]
fn test_degenerate_bbox_does_not_erase_the_form() {
    let pdf = form_pdf("/BBox [0 0 0 0]", "");
    assert!(
        is_dark_at(&pdf, 25, 25),
        "a zero-area /BBox should not clip the form away"
    );
}

/// The mechanism that makes this safe, pinned.
///
/// The clip is installed at depth 0 of the nested stream's own clip stack, and
/// `Q` never pops below depth 0, so it holds however unbalanced the form's
/// `q`/`Q` pairs are. Expressing the same clip by wrapping the form's
/// operators in an injected save/restore does not survive this: the stray `Q`
/// below consumes the injected save, and the rest of the form paints unclipped.
///
/// Real content streams carry unbalanced restores often enough that this is
/// the difference between a clip that works and one that silently stops
/// applying partway through a page.
#[test]
fn test_unbalanced_restore_inside_the_form_does_not_drop_the_clip() {
    // A stray `Q` with no matching `q`, then paint outside the declared box.
    let pdf = form_pdf_with_body(
        "/BBox [0 0 50 50]",
        "",
        "0 0 0 rg 0 0 20 20 re f\nQ\n0 0 0 rg 0 0 200 200 re f\n",
    );
    assert!(is_dark_at(&pdf, 10, 10), "content inside the box must still paint");
    assert!(
        !is_dark_at(&pdf, 150, 150),
        "the clip was dropped by an unbalanced restore, so content outside the \
         /BBox painted"
    );
}

/// And an unbalanced `q` must not leave the clip applied to anything after the
/// form returns: the nested stream has its own stack, so the page's clip is
/// untouched whatever the form does to its own.
#[test]
fn test_unbalanced_save_inside_the_form_does_not_leak_its_clip() {
    let pdf = form_pdf_with_body("/BBox [0 0 50 50]", "", "q\n0 0 0 rg 0 0 200 200 re f\n");
    // The page paints a white backdrop, then the form. Nothing after the form
    // is drawn here, so the check is that the page still renders and the box
    // still held.
    assert!(is_dark_at(&pdf, 25, 25));
    assert!(!is_dark_at(&pdf, 150, 150), "the /BBox must still clip");
}
