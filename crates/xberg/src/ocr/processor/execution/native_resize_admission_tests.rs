use super::{SecurityLimits, TesseractConfig, prepare_encoded_ocr_image};
use crate::types::ImagePreprocessingConfig;

#[test]
fn known_dpi_downscale_is_admitted_for_its_final_native_dimensions() {
    let dimension = 512;
    let rgb = vec![180; dimension as usize * dimension as usize * 3];
    let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, dimension, dimension);
    let result = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits {
            max_content_size: 2 * 1024 * 1024,
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
    .expect("the full decoded RGB and resize fit, and only the final171px raster reaches native Pix");
    assert_eq!((result.width, result.height, result.source_dpi), (171, 171, 200));
    assert!(!result.image_preprocessing.expect("actual normalization").skipped_resize);
}
