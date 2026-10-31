use crate::extraction::image::load_image_for_ocr;
use crate::extractors::security::SecurityLimits;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 256;
const BUDGET: usize = 128 * 1024;

fn jpeg_with_frame_marker(marker: u8) -> Vec<u8> {
    let pixels = image::RgbImage::from_pixel(WIDTH, HEIGHT, image::Rgb([180, 180, 180]));
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90)
        .encode_image(&pixels)
        .unwrap();
    let sof = bytes.windows(2).position(|pair| pair == [0xff, 0xc0]).unwrap();
    bytes[sof + 1] = marker;
    bytes
}

fn message(bytes: &[u8]) -> String {
    load_image_for_ocr(
        bytes,
        &SecurityLimits {
            max_content_size: BUDGET,
            ..Default::default()
        },
    )
    .expect_err("an oversized JPEG that no reduced decode can take must fail")
    .to_string()
}

fn assert_names_the_limit(message: &str) {
    let full_peak = WIDTH as usize * HEIGHT as usize * 3;
    assert!(
        message.contains(&format!("Image dimensions {WIDTH}x{HEIGHT} require")),
        "{message}"
    );
    assert!(
        message.contains(&format!("max_content_size ({BUDGET} bytes)")),
        "{message}"
    );
    let required: usize = message
        .split(" require ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(required > full_peak, "{message}");
}

#[test]
fn progressive_jpeg_reports_the_size_limit() {
    let message = message(&jpeg_with_frame_marker(0xc2));
    assert_names_the_limit(&message);
    assert!(!message.contains("frame"), "{message}");
}

#[test]
fn lossless_jpeg_reports_the_size_limit() {
    let message = message(&jpeg_with_frame_marker(0xc3));
    assert_names_the_limit(&message);
}
