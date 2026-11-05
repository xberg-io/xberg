use super::{PreparedOcrImage, SecurityLimits, TesseractConfig, prepare_encoded_ocr_image};
use crate::core::config::ImageExtractionConfig;
use crate::types::ImagePreprocessingConfig;

const WIDTH: u16 = 512;
const HEIGHT: u16 = 256;
const REDUCED_BUDGET: usize = 128 * 1024;

fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let pixels = vec![180; usize::from(width) * usize::from(height) * 3];
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90)
        .encode(
            &pixels,
            u32::from(width),
            u32::from(height),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    let independent = image::load_from_memory(&bytes).unwrap();
    assert_eq!(
        (independent.width(), independent.height()),
        (u32::from(width), u32::from(height))
    );
    bytes
}

fn prepare(bytes: &[u8], dpi: Option<f64>, budget: usize, preprocess: bool) -> crate::Result<PreparedOcrImage> {
    let config = TesseractConfig {
        source_dpi: dpi,
        preprocessing: preprocess.then(|| ImagePreprocessingConfig {
            target_dpi: 300,
            ..Default::default()
        }),
        ..Default::default()
    };
    let images = ImageExtractionConfig {
        auto_adjust_dpi: false,
        ..Default::default()
    };
    prepare_encoded_ocr_image(
        bytes,
        &SecurityLimits {
            max_content_size: budget,
            ..Default::default()
        },
        &config,
        Some(&images),
        false,
    )
}

#[test]
fn should_keep_the_quarter_decode_when_1200_dpi_already_reaches_300_dpi() {
    let prepared = prepare(&jpeg(WIDTH, HEIGHT), Some(1200.0), REDUCED_BUDGET, true).unwrap();
    assert_eq!((prepared.width, prepared.height), (128, 64));
    assert_eq!(prepared.source_dpi, 300);
    assert_eq!(prepared.data.len(), 128 * 64 * 3);
}

// ~keep At 350 KiB the half decode fits but leaves 600 DPI, which would need a resize; the
// quarter decode meets 300 DPI. A higher limit must not turn the 128 KiB success into an error.
#[test]
fn should_keep_the_quarter_decode_when_a_higher_limit_also_fits_the_half_decode() {
    let prepared = prepare(&jpeg(WIDTH, HEIGHT), Some(1200.0), 350 * 1024, true).unwrap();
    assert_eq!((prepared.width, prepared.height), (128, 64));
    assert_eq!(prepared.source_dpi, 300);
}

#[test]
fn should_preserve_nominal_density_despite_ceil_rounded_reduced_dimensions() {
    let prepared = prepare(&jpeg(513, 257), Some(1200.0), REDUCED_BUDGET, true).unwrap();
    assert_eq!((prepared.width, prepared.height), (129, 65));
    assert_eq!(prepared.source_dpi, 300);
}

#[test]
fn should_report_reduced_density_without_preprocessing() {
    let prepared = prepare(&jpeg(WIDTH, HEIGHT), Some(1200.0), REDUCED_BUDGET, false).unwrap();
    assert_eq!((prepared.width, prepared.height), (128, 64));
    assert_eq!(prepared.source_dpi, 300);
    assert!(!prepared.apply_pix_preprocessing);
}

#[test]
fn should_keep_fitting_decode_density_and_normalization_unchanged() {
    let prepared = prepare(&jpeg(WIDTH, HEIGHT), Some(1200.0), 1024 * 1024, true).unwrap();
    assert_eq!((prepared.width, prepared.height), (128, 64));
    assert_eq!(prepared.source_dpi, 300);
}

#[test]
fn should_keep_unknown_density_unknown_on_the_unpreprocessed_reduced_image() {
    let prepared = prepare(&jpeg(WIDTH, HEIGHT), None, REDUCED_BUDGET, false).unwrap();
    assert_eq!((prepared.width, prepared.height), (128, 64));
    assert_eq!(prepared.source_dpi, 72);
    assert!(prepared.image_preprocessing.is_none());
}

#[test]
fn should_reject_150_dpi_render_normalization_that_would_exceed_the_decode_budget() {
    let result = prepare(&jpeg(WIDTH, HEIGHT), Some(150.0), REDUCED_BUDGET, true);
    let error = result
        .err()
        .expect("150 DPI physical normalization must not escape the caller budget");
    assert!(
        error.to_string().contains("security_limits.max_content_size"),
        "{error}"
    );
}

#[test]
fn should_reject_300_dpi_render_normalization_that_would_restore_over_budget_pixels() {
    let result = prepare(&jpeg(WIDTH, HEIGHT), Some(300.0), REDUCED_BUDGET, true);
    let error = result
        .err()
        .expect("300 DPI physical normalization must not restore an oversized full raster");
    assert!(
        error.to_string().contains("security_limits.max_content_size"),
        "{error}"
    );
}

#[test]
fn should_choose_the_eighth_decode_that_meets_a_150_dpi_target_without_a_resize() {
    let bytes = jpeg(WIDTH, HEIGHT);
    let config = TesseractConfig {
        source_dpi: Some(1200.0),
        preprocessing: Some(ImagePreprocessingConfig {
            target_dpi: 150,
            ..Default::default()
        }),
        ..Default::default()
    };
    let images = ImageExtractionConfig {
        auto_adjust_dpi: false,
        ..Default::default()
    };
    let prepared = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits {
            max_content_size: REDUCED_BUDGET,
            ..Default::default()
        },
        &config,
        Some(&images),
        false,
    )
    .unwrap();
    assert_eq!((prepared.width, prepared.height), (64, 32));
    assert_eq!(prepared.source_dpi, 150);
}

#[test]
fn should_reject_a_resize_even_when_dimension_clamping_returns_the_input_size() {
    let bytes = jpeg(WIDTH, HEIGHT);
    let config = TesseractConfig {
        source_dpi: Some(1200.0),
        preprocessing: Some(ImagePreprocessingConfig {
            target_dpi: 600,
            ..Default::default()
        }),
        ..Default::default()
    };
    let images = ImageExtractionConfig {
        auto_adjust_dpi: false,
        max_image_dimension: 128,
        ..Default::default()
    };
    let dpi = crate::types::ImageDpiConfig {
        target_dpi: 600,
        ..crate::types::ImageDpiConfig::from(&images)
    };
    assert_eq!(
        crate::image::preprocessing::normalized_image_dimensions(128, 64, &dpi, Some(300.0)),
        (128, 64)
    );
    let error = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits {
            max_content_size: REDUCED_BUDGET,
            ..Default::default()
        },
        &config,
        Some(&images),
        false,
    )
    .err()
    .expect("equal final dimensions do not mean the convolution resize is skipped");
    let message = error.to_string();
    assert!(message.contains("cannot be resized to target_dpi 600"), "{message}");
    assert!(
        message.contains("Raise security_limits.max_content_size, or set target_dpi to 300"),
        "{message}"
    );
}

#[test]
fn should_name_the_unknown_source_dpi_and_the_limit_when_a_reduced_image_needs_a_resize() {
    let error = prepare(&jpeg(WIDTH, HEIGHT), None, REDUCED_BUDGET, true)
        .err()
        .expect("an unknown source DPI cannot prove that the reduced image needs no resize")
        .to_string();
    assert!(error.contains("its source DPI is unknown"), "{error}");
    assert!(
        error.ends_with("Raise security_limits.max_content_size"),
        "an unknown source DPI has no other remedy: {error}"
    );
}
