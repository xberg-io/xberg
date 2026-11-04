//! The renderer lists each image it could not paint in `RenderedImage::skipped_images`.
//!
//! A failed image leaves its region blank and the page still renders, so without this list a
//! caller cannot tell a blank page from a page whose only image failed.

mod common;

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, RenderedImage, render_page};

/// One 8x1 page whose only content is one 8-bit grey image with the given filter entry and data.
fn build_pdf_with_gray_image(filter: &str, data: &[u8]) -> Vec<u8> {
    let content: &[u8] = b"q 8 0 0 1 0 0 cm /Im0 Do Q";
    let mut contents = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    contents.extend_from_slice(content);
    contents.extend_from_slice(b"\nendstream");

    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 1 /BitsPerComponent 8 \
         /ColorSpace /DeviceGray {filter} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    image.extend_from_slice(data);
    image.extend_from_slice(b"\nendstream");

    let objects: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 8 1] \
          /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>",
        &contents,
        &image,
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    common::finalize_pdf(pdf, &offsets)
}

fn render(pdf: Vec<u8>) -> RenderedImage {
    let doc = PdfDocument::from_bytes(pdf).expect("open generated PDF");
    render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("render generated PDF")
}

#[test]
fn a_painted_image_leaves_skipped_images_empty() {
    let rendered = render(build_pdf_with_gray_image("", &[0; 8]));
    assert!(rendered.skipped_images.is_empty(), "{:?}", rendered.skipped_images);
}

#[test]
fn an_image_that_fails_to_decode_is_listed_in_skipped_images() {
    let rendered = render(build_pdf_with_gray_image("/Filter /FlateDecode", b"not deflate data"));
    assert_eq!(rendered.skipped_images.len(), 1, "{:?}", rendered.skipped_images);
    assert!(
        rendered.skipped_images[0].starts_with("image XObject: "),
        "{:?}",
        rendered.skipped_images
    );
}
