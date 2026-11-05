use super::{SecurityLimits, TesseractConfig, prepare_encoded_ocr_image, resolve_known_source_dpi};

fn prepare_unknown_dpi_jpeg(limit_mib: usize, target_dpi: i32) -> crate::Result<super::PreparedOcrImage> {
    let dimension = 1000;
    let rgb = vec![245; dimension as usize * dimension as usize * 3];
    let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, dimension, dimension);
    assert!(resolve_known_source_dpi(None, &bytes, dimension.into(), dimension.into()).is_none());
    prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits {
            max_content_size: limit_mib * 1024 * 1024,
            ..Default::default()
        },
        &TesseractConfig {
            known_full_page_scan: true,
            preprocessing: Some(crate::types::ImagePreprocessingConfig {
                target_dpi,
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
fn unknown_dpi_jpeg_cannot_upscale_past_its_image_workspace_limit() {
    let error = match prepare_unknown_dpi_jpeg(40, 300) {
        Ok(prepared) => panic!("unbounded unknown-DPI output {}x{}", prepared.width, prepared.height),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("DPI resize workspace"), "{error}");
    assert!(error.contains("target_dpi"), "{error}");
}

#[test]
fn unknown_dpi_jpeg_fits_at_source_size_under_the_same_limit() {
    let prepared = prepare_unknown_dpi_jpeg(40, 72).expect("source raster and native workspace fit");
    assert_eq!((prepared.width, prepared.height, prepared.source_dpi), (1000, 1000, 72));
}

#[test]
fn unknown_dpi_jpeg_checks_native_budget_even_when_resize_workspace_fits() {
    let error = prepare_unknown_dpi_jpeg(100, 300)
        .err()
        .expect("normalized raster exceeds the native workspace limit")
        .to_string();
    assert!(error.contains("native image workspace after DPI resize"), "{error}");
}
