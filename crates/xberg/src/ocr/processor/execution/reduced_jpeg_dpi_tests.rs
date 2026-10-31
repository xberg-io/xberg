use super::{PreparedOcrImage, SecurityLimits, TesseractConfig, prepare_encoded_ocr_image};
use crate::core::config::ImageExtractionConfig;
use crate::types::ImagePreprocessingConfig;

const WIDTH: u16 = 512;
const HEIGHT: u16 = 256;
const REDUCED_BUDGET: usize = 128 * 1024;

fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let pixels = vec![180; usize::from(width) * usize::from(height) * 3];
    let bytes = oxiarc_jpeg::encode_to_vec_with_options(
        &pixels,
        width,
        height,
        oxiarc_jpeg::InputColor::Rgb,
        &Default::default(),
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
    )
}

#[test]
fn should_keep_the_quarter_decode_when_1200_dpi_already_reaches_300_dpi() {
    let prepared = prepare(&jpeg(WIDTH, HEIGHT), Some(1200.0), REDUCED_BUDGET, true).unwrap();
    assert_eq!((prepared.width, prepared.height), (128, 64));
    assert_eq!(prepared.source_dpi, 300);
    assert_eq!(prepared.data.len(), 128 * 64 * 3);
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
