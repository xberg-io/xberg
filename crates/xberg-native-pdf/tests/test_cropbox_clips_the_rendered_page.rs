//! A page is rendered to its `/CropBox`, not its `/MediaBox`.
//!
//! ISO 32000-1:2008 Table 30 on `/CropBox` (`docs/spec/pdf.md:5761`):
//!
//! > A rectangle … that shall define the visible region of default user space.
//! > When the page is displayed or printed, its contents **shall be clipped
//! > (cropped) to this rectangle** … Default value: the value of MediaBox.
//!
//! The crop box was parsed onto `PageInfo` and then never consulted by either
//! renderer, so a cropped scan rendered at full media size, showing the margins
//! the file asked to have cropped away. §14.11.2 also takes the crop box as its
//! intersection with the media box, so an oversized one does not enlarge the
//! page.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// A 200x200 page with content in two places: a black square at the bottom
/// left (inside any crop box used here) and another at the top right (outside
/// a bottom-left crop).
fn page_pdf(crop: &str) -> Vec<u8> {
    let content = b"0 0 0 rg 10 10 40 40 re f\n0 0 0 rg 150 150 40 40 re f\n";

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
        &format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] {crop} /Contents 4 0 R >>"),
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
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

/// `(width, height, dark_pixel_count)` of the rendered page.
fn render_stats(pdf: Vec<u8>) -> (u32, u32, usize) {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.honor_crop_box = true;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    let dark = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|px| px[0] < 128 && px[1] < 128 && px[2] < 128)
        .count();
    (img.width, img.height, dark)
}

/// No crop box: the whole medium, both squares.
#[test]
fn without_a_crop_box_the_page_is_the_media_box() {
    let (w, h, dark) = render_stats(page_pdf(""));
    assert_eq!((w, h), (200, 200));
    assert!(dark > 2000, "both squares should paint, got {dark} dark pixels");
}

/// The reported symptom: a crop box must set the rendered page size.
#[test]
fn test_crop_box_sets_the_rendered_page_size() {
    let (w, h, _) = render_stats(page_pdf("/CropBox [0 0 100 100]"));
    assert_eq!(
        (w, h),
        (100, 100),
        "the page should render at its crop box, not its media box"
    );
}

/// And content outside the crop box must not appear.
#[test]
fn content_outside_the_crop_box_is_cropped_away() {
    let (_, _, cropped_dark) = render_stats(page_pdf("/CropBox [0 0 100 100]"));
    let (_, _, full_dark) = render_stats(page_pdf(""));
    assert!(
        cropped_dark < full_dark,
        "cropping should remove the top-right square ({cropped_dark} vs {full_dark})"
    );
    assert!(
        cropped_dark > 500,
        "the bottom-left square is inside the crop and must survive, got {cropped_dark}"
    );
}

/// An offset crop box moves the origin as well as the size.
#[test]
fn test_offset_crop_box_moves_the_origin() {
    let (w, h, dark) = render_stats(page_pdf("/CropBox [100 100 200 200]"));
    assert_eq!((w, h), (100, 100));
    assert!(
        dark > 500,
        "the top-right square is inside this crop and must survive, got {dark}"
    );
}

/// §14.11.2: the crop box is intersected with the media box, so an oversized
/// one does not enlarge the page.
#[test]
fn test_crop_box_larger_than_the_media_box_is_clamped() {
    let (w, h, _) = render_stats(page_pdf("/CropBox [-500 -500 900 900]"));
    assert_eq!((w, h), (200, 200), "an oversized crop box must clamp to the medium");
}

/// A crop box disjoint from the medium describes nothing; fall back to the
/// media box rather than rendering an empty page.
#[test]
fn test_disjoint_crop_box_falls_back_to_the_media_box() {
    let (w, h, _) = render_stats(page_pdf("/CropBox [900 900 1000 1000]"));
    assert_eq!((w, h), (200, 200));
}
