use super::*;
use image::ImageEncoder;

#[test]
fn selected_page_retains_only_the_rgb_pixels_consumed_by_ocr() {
    let document = xberg_native_pdf::PdfDocument::from_bytes(
        include_bytes!("../../../../../tests/fixtures/ocr/rotated_financial_table.pdf").to_vec(),
    )
    .unwrap();
    let limits = crate::extractors::security::SecurityLimits::default();
    let rotations = crate::pdf::render::get_page_rotations(&document, 1);
    let (rendered, _) = crate::pdf::render::render_pdf_ocr_page_with_dpi(&document, 0, None, &limits).unwrap();
    let (png, width, height) = crate::pdf::render::normalize_rendered_page_for_ocr_with_security_limits(
        rendered.data,
        rendered.width,
        rendered.height,
        rotations[0],
        &limits,
    )
    .unwrap();
    let original = crate::extraction::image_decode::decode_standard_image_with_security_limits(&png, &limits).unwrap();
    assert!(original.to_rgba8().pixels().all(|pixel| pixel[3] == 255));
    let expected_rgb = original.to_rgb8();
    let ((index, retained), _) = render_one_selected_page(&document, &rotations, 0, &limits, None, None).unwrap();
    assert_eq!(index, 0);
    assert_eq!((retained.width(), retained.height()), (width, height));
    assert_eq!(retained.to_rgb8(), expected_rgb);
    assert_eq!(retained.color(), image::ColorType::Rgb8);
    assert_eq!(retained.as_bytes(), expected_rgb.as_raw());
    assert_eq!(
        encode_for_backend(&retained, &limits),
        encode_for_backend(&original, &limits)
    );
}

fn encode_for_backend(image: &image::DynamicImage, limits: &crate::extractors::security::SecurityLimits) -> Vec<u8> {
    let rgb = clone_rgb_for_png_encode(image, limits).unwrap();
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ColorType::Rgb8.into())
        .unwrap();
    png
}
