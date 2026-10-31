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

#[test]
fn restart_intervals_cannot_enable_unaccounted_parallel_reduced_buffers() {
    let options = oxiarc_jpeg::EncodeOptions {
        restart_interval: oxiarc_jpeg::RestartInterval::McuRows(1),
        ..Default::default()
    };
    let pixels = vec![180; usize::from(WIDTH) * usize::from(HEIGHT) * 3];
    let bytes = oxiarc_jpeg::encode_to_vec_with_options(&pixels, WIDTH, HEIGHT, oxiarc_jpeg::InputColor::Rgb, &options)
        .unwrap();
    assert_valid(&bytes);
    assert!(bytes.windows(2).any(|marker| marker == [0xff, 0xdd]));
    assert!(bytes.windows(2).any(|marker| marker == [0xff, 0xd0]));
    let error = load_image_for_ocr(
        &bytes,
        &SecurityLimits {
            max_content_size: BUDGET,
            ..Default::default()
        },
    )
    .err()
    .expect("reduced decoding must reject restart intervals before feature-unified Rayon can allocate bands");
    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("restart intervals"), "{error}");
}

fn insert_segment(bytes: &[u8], at: usize, segment: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len() + segment.len());
    output.extend_from_slice(&bytes[..at]);
    output.extend_from_slice(segment);
    output.extend_from_slice(&bytes[at..]);
    output
}

fn relocated_adobe_rgb_jpeg() -> (Vec<u8>, Vec<u8>) {
    const ADOBE_RGB: &[u8] = &[0xff, 0xee, 0, 14, b'A', b'd', b'o', b'b', b'e', 0, 100, 0, 0, 0, 0, 0];
    let options = oxiarc_jpeg::EncodeOptions {
        jpeg_color_space: Some(oxiarc_jpeg::ColorSpace::Rgb),
        component_ids: oxiarc_jpeg::ComponentIds::Sequential,
        write_jfif: oxiarc_jpeg::MarkerPolicy::Never,
        write_adobe: oxiarc_jpeg::MarkerPolicy::Never,
        ..Default::default()
    };
    let pixels = [200, 100, 50].repeat(usize::from(WIDTH) * usize::from(HEIGHT));
    let original =
        oxiarc_jpeg::encode_to_vec_with_options(&pixels, WIDTH, HEIGHT, oxiarc_jpeg::InputColor::Rgb, &options)
            .unwrap();
    assert!(!original.windows(2).any(|marker| marker == [0xff, 0xee]));
    assert!(!original.windows(2).any(|marker| marker == [0xff, 0xe0]));
    let frame = original.windows(2).position(|marker| marker == [0xff, 0xc0]).unwrap();
    let frame_length = usize::from(u16::from_be_bytes([original[frame + 2], original[frame + 3]]));
    let early = insert_segment(&original, frame, ADOBE_RGB);
    let late = insert_segment(&original, frame + 2 + frame_length, ADOBE_RGB);
    (early, late)
}

#[test]
fn late_adobe_color_metadata_is_rejected_before_reduced_decoding() {
    let (early, late) = relocated_adobe_rgb_jpeg();
    assert_valid(&early);
    assert_valid(&late);
    let expected = image::load_from_memory(&early).unwrap().into_rgb8();
    let moved = image::load_from_memory(&late).unwrap().into_rgb8();
    for (actual, expected) in expected.get_pixel(0, 0).0.into_iter().zip([200, 100, 50]) {
        assert!(
            actual.abs_diff(expected) <= 4,
            "independent full decode must retain the encoded RGB color"
        );
    }
    assert!(
        expected.as_raw() == moved.as_raw(),
        "moving only APP14 must preserve independently decoded RGB"
    );
    let limits = SecurityLimits {
        max_content_size: BUDGET,
        ..Default::default()
    };
    let reduced = load_image_for_ocr(&early, &limits).unwrap().into_rgb8();
    for (actual, expected) in reduced.get_pixel(0, 0).0.into_iter().zip([200, 100, 50]) {
        assert!(actual.abs_diff(expected) <= 4);
    }
    let fitting = load_image_for_ocr(
        &late,
        &SecurityLimits {
            max_content_size: 1024 * 1024,
            ..Default::default()
        },
    )
    .unwrap()
    .into_rgb8();
    assert!(
        fitting.as_raw() == expected.as_raw(),
        "the fitting-image path keeps its color behavior"
    );
    let error = load_image_for_ocr(&late, &limits)
        .err()
        .expect("late color signalling must not reach the decoder's frozen pre-scan color state");
    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("color marker after frame"), "{error}");
}
