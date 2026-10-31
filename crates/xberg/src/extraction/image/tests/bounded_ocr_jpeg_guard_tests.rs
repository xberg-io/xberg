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
    assert!(message.contains("Raise security_limits.max_content_size"), "{message}");
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
fn progressive_jpeg_reports_the_size_limit_and_the_progressive_reason() {
    let message = message(&jpeg_with_frame_marker(0xc2));
    assert_names_the_limit(&message);
    assert!(
        message.contains("a progressive JPEG keeps full-resolution coefficients"),
        "{message}"
    );
    assert!(!message.contains("frame"), "{message}");
}

#[test]
fn lossless_jpeg_reports_the_size_limit_and_the_lossless_reason() {
    let message = message(&jpeg_with_frame_marker(0xc3));
    assert_names_the_limit(&message);
    assert!(
        message.contains("lossless, hierarchical or arithmetic-coded JPEG cannot be decoded at a reduced size"),
        "{message}"
    );
}

#[test]
fn hierarchical_jpeg_reports_the_size_limit_and_the_hierarchical_reason() {
    let message = message(&jpeg_with_frame_marker(0xc5));
    assert_names_the_limit(&message);
    assert!(message.contains("hierarchical"), "{message}");
}

#[test]
fn twelve_bit_jpeg_reports_the_size_limit_and_the_sample_precision() {
    let mut bytes = jpeg_with_frame_marker(0xc1);
    let sof = bytes.windows(2).position(|pair| pair == [0xff, 0xc1]).unwrap();
    // The precision byte follows the marker and the two-byte segment length.
    bytes[sof + 4] = 12;
    let message = message(&bytes);
    assert_names_the_limit(&message);
    assert!(
        message.contains("a JPEG with 12-bit samples cannot be decoded at a reduced size"),
        "{message}"
    );
}

#[test]
fn cmyk_jpeg_reports_the_size_limit_and_the_component_count() {
    let pixels = vec![120u8; WIDTH as usize * HEIGHT as usize * 4];
    let mut bytes = Vec::new();
    jpeg_encoder::Encoder::new(&mut bytes, 90)
        .encode(&pixels, WIDTH as u16, HEIGHT as u16, jpeg_encoder::ColorType::Cmyk)
        .unwrap();
    let message = message(&bytes);
    assert_names_the_limit(&message);
    assert!(
        message.contains("a JPEG with 4 color components cannot be decoded at a reduced size"),
        "{message}"
    );
}

#[test]
fn metadata_probe_admits_an_oversized_baseline_jpeg_that_ocr_can_reduce() {
    let bytes = jpeg_with_frame_marker(0xc0);
    let metadata = crate::extraction::image::extract_image_metadata_with_security_limits(
        &bytes,
        &SecurityLimits {
            max_content_size: BUDGET,
            ..Default::default()
        },
    )
    .expect("the metadata probe must not reject a JPEG that the OCR path decodes at a reduced size");
    assert_eq!((metadata.width, metadata.height), (WIDTH, HEIGHT));
    assert_eq!(metadata.format, "JPEG");
}

#[test]
fn metadata_probe_reads_an_oversized_jpeg_from_its_frame_header_without_decoding() {
    // Keep the bytes up to the end of the frame header and drop the tables and the scan, so
    // any decode fails and only a header read succeeds.
    let mut bytes = jpeg_with_frame_marker(0xc0);
    let sof = bytes.windows(2).position(|pair| pair == [0xff, 0xc0]).unwrap();
    let length = usize::from(u16::from_be_bytes([bytes[sof + 2], bytes[sof + 3]]));
    bytes.truncate(sof + 2 + length);
    bytes.extend_from_slice(&[0xff, 0xd9]);
    let metadata = crate::extraction::image::extract_image_metadata_with_security_limits(
        &bytes,
        &SecurityLimits {
            max_content_size: BUDGET,
            ..Default::default()
        },
    )
    .expect("the metadata probe must admit an oversized JPEG from its frame header");
    assert_eq!((metadata.width, metadata.height), (WIDTH, HEIGHT));
}
