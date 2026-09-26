//! GH#1781: a `/JPXDecode` image dictionary with no `/ColorSpace` (and no
//! `/BitsPerComponent`) must still decode, per ISO 32000-1 Table 89, which
//! allows both to be omitted because the JPEG 2000 codestream carries its
//! own colour space and bit depth.
//!
//! Always runs, matching `test_jpx_decode.rs`: JPEG 2000 support is
//! unconditional, decoded by the non-optional `hayro-jpeg2000` dependency.

use xberg_native_pdf::document::PdfDocument;

#[test]
fn extract_jpx_image_with_no_colorspace_dictionary_entry() {
    let doc =
        PdfDocument::open("tests/fixtures/jpx/jpx_no_colorspace.pdf").expect("open JPX-without-/ColorSpace repro");
    let images = doc
        .extract_images(0)
        .expect("a JPXDecode image with no /ColorSpace must still be decoded, per ISO 32000-1 Table 89");
    assert_eq!(images.len(), 1, "expected exactly one image on the page");

    let png = images[0].to_png_bytes().expect("encode the extracted JPX image as PNG");
    assert!(
        png.len() > 8 && &png[1..4] == b"PNG",
        "extracted JPX image did not encode to a valid PNG"
    );
}

/// Scope control: the same missing-/ColorSpace omission is only valid for
/// JPXDecode (ISO 32000-1 Table 89). A FlateDecode image with no
/// /ColorSpace must still be rejected and dropped, not decoded with a
/// guessed colour space.
#[test]
fn flate_image_with_no_colorspace_dictionary_entry_is_not_extracted() {
    let doc = PdfDocument::open("tests/fixtures/jpx/gh1781_flate_missing_colorspace_control.pdf")
        .expect("open FlateDecode-without-/ColorSpace control fixture");
    let images = doc
        .extract_images(0)
        .expect("extract_images must not error out entirely");
    assert!(
        images.is_empty(),
        "a non-JPXDecode image with no /ColorSpace must remain rejected, not decoded"
    );
}
