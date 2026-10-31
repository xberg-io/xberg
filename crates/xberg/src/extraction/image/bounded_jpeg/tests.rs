use super::{budget, headers};
use crate::extraction::image::{load_image_for_ocr, load_image_for_ocr_with_scale};
use crate::extractors::security::SecurityLimits;

const WIDTH: u16 = 512;
const HEIGHT: u16 = 256;
const LOWER_BUDGET: usize = 128 * 1024;

fn limits(max_content_size: usize) -> SecurityLimits {
    SecurityLimits {
        max_content_size,
        ..Default::default()
    }
}

fn jpeg(progressive: bool) -> Vec<u8> {
    let pixels = vec![180; usize::from(WIDTH) * usize::from(HEIGHT) * 3];
    let options = oxiarc_jpeg::EncodeOptions {
        process: if progressive {
            oxiarc_jpeg::EncodeProcess::Progressive
        } else {
            oxiarc_jpeg::EncodeProcess::Sequential
        },
        ..Default::default()
    };
    oxiarc_jpeg::encode_to_vec_with_options(&pixels, WIDTH, HEIGHT, oxiarc_jpeg::InputColor::Rgb, &options).unwrap()
}

#[test]
fn should_choose_the_finest_scale_at_an_inclusive_working_buffer_boundary() {
    let bytes = jpeg(false);
    let header = headers::inspect(&bytes).unwrap();
    let half = budget::select(&header, bytes.len() as u64, u64::MAX).unwrap();
    assert_eq!((half.width, half.height, half.denominator), (256, 128, 2));
    assert_eq!(half.plane_bytes, 256 * 128 * 3 * 2);
    assert_eq!(half.output_bytes, 256 * 128 * 3);
    let boundary = budget::select(&header, bytes.len() as u64, half.peak_bytes).unwrap();
    assert_eq!(boundary.denominator, 2);
    let below = budget::select(&header, bytes.len() as u64, half.peak_bytes - 1).unwrap();
    assert_eq!(below.denominator, 4);
}

#[test]
fn should_reject_retained_metadata_even_when_input_and_eighth_scale_buffers_fit() {
    const COMMENT_BYTES: usize = 36 * 1024;
    let original = jpeg(false);
    let mut bytes = original[..2].to_vec();
    for _ in 0..2 {
        bytes.extend_from_slice(&[0xff, 0xfe]);
        bytes.extend_from_slice(&u16::try_from(COMMENT_BYTES + 2).unwrap().to_be_bytes());
        bytes.resize(bytes.len() + COMMENT_BYTES, b'm');
    }
    bytes.extend_from_slice(&original[2..]);
    let decoded = image::load_from_memory(&bytes).unwrap();
    assert_eq!(
        (decoded.width(), decoded.height()),
        (u32::from(WIDTH), u32::from(HEIGHT))
    );
    assert!(bytes.len() < LOWER_BUDGET);
    let header = headers::inspect(&bytes).unwrap();
    assert!(budget::select(&header, bytes.len() as u64, LOWER_BUDGET as u64).is_none());
    let mut without_retained_payloads = header;
    without_retained_payloads.metadata_bytes = 0;
    assert!(budget::select(&without_retained_payloads, bytes.len() as u64, LOWER_BUDGET as u64).is_some());
    let error = load_image_for_ocr(&bytes, &limits(LOWER_BUDGET)).unwrap_err();
    assert!(error.to_string().contains("security_limits.max_content_size"));
}

#[test]
fn should_report_original_dimensions_and_the_chosen_nominal_scale() {
    let bytes = jpeg(false);
    let decoded = load_image_for_ocr_with_scale(&bytes, &limits(256 * 1024)).unwrap();
    assert_eq!((decoded.image.width(), decoded.image.height()), (128, 64));
    assert_eq!((decoded.original_width, decoded.original_height), (512, 256));
    assert_eq!(decoded.scale_denominator, 4);
}

#[test]
fn should_leave_fitting_progressive_jpeg_dimensions_and_scale_unchanged() {
    let bytes = jpeg(true);
    assert!(bytes.windows(2).any(|marker| marker == [0xff, 0xc2]));
    let decoded = load_image_for_ocr_with_scale(&bytes, &limits(1024 * 1024)).unwrap();
    assert_eq!((decoded.image.width(), decoded.image.height()), (512, 256));
    assert_eq!((decoded.original_width, decoded.original_height), (512, 256));
    assert_eq!(decoded.scale_denominator, 1);
}

#[test]
fn should_reject_a_second_scan_before_allocating_decoder_planes() {
    let original = jpeg(false);
    let scan = original.windows(2).position(|marker| marker == [0xff, 0xda]).unwrap();
    let mut bytes = original[..original.len() - 2].to_vec();
    bytes.extend_from_slice(&original[scan..]);
    assert_eq!(bytes.windows(2).filter(|marker| *marker == [0xff, 0xda]).count(), 2);
    let error = headers::inspect(&bytes).err().expect("second scan must fail admission");
    assert!(error.to_string().contains("single baseline JPEG scan"));
}
