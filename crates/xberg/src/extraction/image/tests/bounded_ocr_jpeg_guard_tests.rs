use crate::XbergError;
use crate::extraction::image::load_image_for_ocr;
use crate::extractors::security::SecurityLimits;

const WIDTH: u16 = 512;
const HEIGHT: u16 = 256;
const BUDGET: usize = 128 * 1024;

fn jpeg(process: oxiarc_jpeg::EncodeProcess) -> Vec<u8> {
    let options = oxiarc_jpeg::EncodeOptions {
        process,
        ..Default::default()
    };
    let pixels = vec![180; usize::from(WIDTH) * usize::from(HEIGHT) * 3];
    let bytes = oxiarc_jpeg::encode_to_vec_with_options(&pixels, WIDTH, HEIGHT, oxiarc_jpeg::InputColor::Rgb, &options)
        .expect("encode a small, real JPEG");
    assert_valid(&bytes);
    bytes
}

fn assert_valid(bytes: &[u8]) {
    let image = image::load_from_memory(bytes).expect("independent JPEG decoder accepts the fixture");
    assert_eq!(image.width(), u32::from(WIDTH));
    assert_eq!(image.height(), u32::from(HEIGHT));
    assert!(bytes.len() < BUDGET);
}

fn error(bytes: &[u8]) -> XbergError {
    load_image_for_ocr(
        bytes,
        &SecurityLimits {
            max_content_size: BUDGET,
            ..Default::default()
        },
    )
    .expect_err("unsupported or over-budget reduced decoding must fail")
}

#[test]
fn progressive_jpeg_is_rejected_before_reduced_coefficient_allocation() {
    let bytes = jpeg(oxiarc_jpeg::EncodeProcess::Progressive);
    assert!(bytes.windows(2).any(|pair| pair == [0xff, 0xc2]));
    let error = error(&bytes);
    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("baseline JPEG"), "{error}");
}

#[test]
fn too_many_metadata_segments_are_rejected_before_reduced_decode() {
    const EXTRA_SEGMENTS: usize = 129;
    let original = jpeg(oxiarc_jpeg::EncodeProcess::Sequential);
    let mut bytes = original[..2].to_vec();
    for _ in 0..EXTRA_SEGMENTS {
        bytes.extend_from_slice(&[0xff, 0xfe, 0, 2]);
    }
    bytes.extend_from_slice(&original[2..]);
    assert_valid(&bytes);
    let error = error(&bytes);
    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("JPEG header segment limit"), "{error}");
}

#[test]
fn retained_metadata_copies_count_toward_the_reduced_decode_budget() {
    const PAYLOAD_LEN: usize = 48 * 1024;
    let original = jpeg(oxiarc_jpeg::EncodeProcess::Sequential);
    let mut bytes = original[..2].to_vec();
    for _ in 0..2 {
        bytes.extend_from_slice(&[0xff, 0xfe]);
        bytes.extend_from_slice(&u16::try_from(PAYLOAD_LEN + 2).unwrap().to_be_bytes());
        bytes.resize(bytes.len() + PAYLOAD_LEN, b'm');
    }
    bytes.extend_from_slice(&original[2..]);
    assert_valid(&bytes);
    assert!(2 * 2 * PAYLOAD_LEN > BUDGET);
    let error = error(&bytes);
    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(
        error.to_string().contains("security_limits.max_content_size"),
        "{error}"
    );
}
