use crate::XbergError;
use crate::extraction::image::load_image_for_ocr;
use crate::extraction::image_decode::decode_standard_rgb8_with_security_limits;
use crate::extractors::security::SecurityLimits;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 256;
const FULL_RGB_BYTES: usize = WIDTH as usize * HEIGHT as usize * 3;
const LOWER_BUDGET: usize = 128 * 1024;
const HIGHER_BUDGET: usize = 256 * 1024;
const SOURCE_COLOR: [u8; 3] = [200, 100, 50];

fn baseline_jpeg() -> Vec<u8> {
    let pixels = image::RgbImage::from_pixel(WIDTH, HEIGHT, image::Rgb(SOURCE_COLOR));
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90)
        .encode_image(&pixels)
        .expect("encode a small real baseline JPEG");
    assert!(bytes.windows(2).any(|marker| marker == [0xff, 0xc0]));
    assert!(bytes.len() < LOWER_BUDGET);
    assert!(FULL_RGB_BYTES > HIGHER_BUDGET);
    bytes
}

fn limits(max_content_size: usize) -> SecurityLimits {
    SecurityLimits {
        max_content_size,
        ..Default::default()
    }
}

fn assert_bounded_rgb(image: &image::DynamicImage, encoded_bytes: usize, budget: usize) {
    assert_eq!(image.color(), image::ColorType::Rgb8);
    assert!(image.width() > 0 && image.width() < WIDTH);
    assert!(image.height() > 0 && image.height() < HEIGHT);
    assert_eq!(image.width() * HEIGHT, image.height() * WIDTH, "preserve aspect ratio");
    assert!(image.as_bytes().len() + encoded_bytes <= budget);
    let pixel = image
        .as_rgb8()
        .unwrap()
        .get_pixel(image.width() / 2, image.height() / 2);
    for (actual, expected) in pixel.0.into_iter().zip(SOURCE_COLOR) {
        assert!(
            actual.abs_diff(expected) <= 4,
            "the returned RGB must come from the JPEG"
        );
    }
}

// ~keep GH#2339: this small synthetic JPEG exercises the same full-source budget rejection,
// without allocating the unavailable report's 10007x6343 raster or claiming its text quality.
#[test]
fn oversized_baseline_jpeg_is_reduced_during_ocr_loading_within_the_budget() {
    let bytes = baseline_jpeg();
    let image = load_image_for_ocr(&bytes, &limits(HIGHER_BUDGET))
        .expect("OCR may reduce a baseline JPEG before allocating its over-budget full RGB raster");
    assert_bounded_rgb(&image, bytes.len(), HIGHER_BUDGET);
}

#[test]
fn increasing_the_budget_keeps_a_recoverable_jpeg_usable() {
    let bytes = baseline_jpeg();
    let lower =
        load_image_for_ocr(&bytes, &limits(LOWER_BUDGET)).expect("a reduced baseline JPEG must fit the lower budget");
    let higher = load_image_for_ocr(&bytes, &limits(HIGHER_BUDGET))
        .expect("increasing the budget must not turn a recoverable JPEG into an error");
    assert_bounded_rgb(&lower, bytes.len(), LOWER_BUDGET);
    assert_bounded_rgb(&higher, bytes.len(), HIGHER_BUDGET);
    assert!(higher.width() >= lower.width());
    assert!(higher.height() >= lower.height());
}

#[test]
fn fitting_jpeg_keeps_its_original_dimensions() {
    let bytes = baseline_jpeg();
    let image = load_image_for_ocr(&bytes, &limits(1024 * 1024)).expect("an in-budget JPEG must still decode");
    assert_eq!((image.width(), image.height()), (WIDTH, HEIGHT));
    assert_eq!(image.as_bytes().len(), FULL_RGB_BYTES);
}

#[test]
fn compressed_jpeg_bytes_alone_over_budget_are_rejected() {
    let bytes = baseline_jpeg();
    let error = load_image_for_ocr(&bytes, &limits(bytes.len() - 1))
        .expect_err("downscaling must not bypass the compressed-byte hard stop");
    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("security_limits.max_content_size"));
}

#[test]
fn general_rgb_decoder_stays_strict_for_the_same_oversized_jpeg() {
    let bytes = baseline_jpeg();
    let error = decode_standard_rgb8_with_security_limits(&bytes, &limits(HIGHER_BUDGET))
        .expect_err("ordinary decoding must not silently change source dimensions");
    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("security_limits.max_content_size"));
}
