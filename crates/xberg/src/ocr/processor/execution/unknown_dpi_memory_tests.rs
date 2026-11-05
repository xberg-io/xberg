use super::{SecurityLimits, TesseractConfig, prepare_encoded_ocr_image, resolve_known_source_dpi};

#[test]
fn unknown_dpi_jpeg_cannot_upscale_past_its_image_workspace_limit() {
    let dimension = 1000;
    let rgb = vec![245; dimension as usize * dimension as usize * 3];
    let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, dimension, dimension);
    assert!(resolve_known_source_dpi(None, &bytes, dimension.into(), dimension.into()).is_none());
    let result = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits {
            max_content_size: 40 * 1024 * 1024,
            ..Default::default()
        },
        &TesseractConfig {
            known_full_page_scan: true,
            ..Default::default()
        },
        Some(&crate::core::config::ImageExtractionConfig {
            auto_adjust_dpi: false,
            ..Default::default()
        }),
        false,
    );
    let error = match result {
        Ok(prepared) => panic!("unbounded unknown-DPI output {}x{}", prepared.width, prepared.height),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("DPI resize workspace"), "{error}");
    assert!(error.contains("target_dpi"), "{error}");
}
