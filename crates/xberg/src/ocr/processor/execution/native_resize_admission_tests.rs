use super::{SecurityLimits, TesseractConfig, prepare_encoded_ocr_image};
use crate::types::ImagePreprocessingConfig;

fn prepare_contrast_downscale(limit: usize) -> crate::Result<super::PreparedOcrImage> {
    let dimension = 512;
    let rgb = vec![180; dimension as usize * dimension as usize * 3];
    let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, dimension, dimension);
    prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits {
            max_content_size: limit,
            ..Default::default()
        },
        &TesseractConfig {
            source_dpi: Some(600.0),
            preprocessing: Some(ImagePreprocessingConfig {
                target_dpi: 200,
                binarization_method: "none".into(),
                contrast_enhance: true,
                deskew: false,
                ..Default::default()
            }),
            ..Default::default()
        },
        Some(&crate::core::config::ImageExtractionConfig {
            auto_adjust_dpi: false,
            ..Default::default()
        }),
        false,
    )
}

#[test]
fn known_dpi_downscale_is_admitted_for_its_final_native_dimensions() {
    let result = prepare_contrast_downscale(2 * 1024 * 1024)
        .expect("the full decoded RGB and resize fit, and only the final171px raster reaches native Pix");
    assert_eq!((result.width, result.height, result.source_dpi), (171, 171, 200));
    assert!(!result.image_preprocessing.expect("actual normalization").skipped_resize);
}

#[test]
fn fir_workspace_refuses_before_resize_while_the_final_native_phase_would_fit() {
    let error = prepare_contrast_downscale(1024 * 1024)
        .err()
        .expect("source, destination and intermediate RGB exceed1MiB")
        .to_string();
    assert!(error.contains("DPI resize workspace"), "{error}");
    assert!(error.contains("max_content_size"), "{error}");
    assert!(error.contains("target_dpi"), "{error}");
}
