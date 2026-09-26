//! JPEG 2000 (`/JPXDecode`) extraction test.
//!
//! Always runs: JPEG 2000 support is unconditional, decoded by the non-optional
//! `hayro-jpeg2000` dependency. (Unit coverage of the decoder itself lives in
//! `src/decoders/jpx.rs`.)

use xberg_native_pdf::document::PdfDocument;

#[test]
fn extract_jpx_image_from_pdf() {
    let doc = PdfDocument::open("tests/fixtures/jpx/jpx_minimal.pdf").expect("open JPX repro");
    let images = doc.extract_images(0).expect("extract page-0 images");
    assert!(!images.is_empty(), "no images extracted from the JPX page");

    let png = images[0].to_png_bytes().expect("encode the extracted JPX image as PNG");
    assert!(
        png.len() > 8 && &png[1..4] == b"PNG",
        "extracted JPX image did not encode to a valid PNG"
    );
}
