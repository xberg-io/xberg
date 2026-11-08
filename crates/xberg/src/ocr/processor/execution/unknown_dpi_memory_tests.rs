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

fn densityless_png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(width, height, image::Rgb([245, 245, 245])))
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("encode PNG");
    bytes
}

#[test]
fn engine_selected_policy_fits_a_densityless_png_upscale_to_the_workspace() {
    let bytes = densityless_png(800, 1000);
    assert!(resolve_known_source_dpi(None, &bytes, 800, 1000).is_none());
    let prepared = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits::default(),
        &TesseractConfig::default(),
        None,
        false,
    )
    .expect("an engine-selected policy shrinks its upscale instead of refusing the page");
    assert!(prepared.width >= 800 && prepared.height >= 1000);
    assert!(u64::from(prepared.width) * u64::from(prepared.height) * 8 <= 100 * 1024 * 1024);
}

#[test]
fn caller_selected_policy_keeps_its_requested_densityless_png_upscale() {
    let bytes = densityless_png(800, 1000);
    let error = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits::default(),
        &TesseractConfig {
            preprocessing: Some(Default::default()),
            ..Default::default()
        },
        None,
        false,
    )
    .err()
    .expect("the requested 300 DPI raster does not fit")
    .to_string();
    assert!(error.contains("OCR native image workspace exceeds"), "{error}");
}

fn densityless_jpeg(width: u16, height: u16) -> Vec<u8> {
    let rgb = vec![245; usize::from(width) * usize::from(height) * 3];
    crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, width, height)
}

fn prepare_densityless_jpeg_with_default_policy(
    allow_bounded_inferred_upscale: bool,
) -> crate::Result<super::PreparedOcrImage> {
    let bytes = densityless_jpeg(800, 1000);
    assert!(resolve_known_source_dpi(None, &bytes, 800, 1000).is_none());
    super::encoded_image::prepare_with_dpi_policy(
        &bytes,
        &SecurityLimits::default(),
        &TesseractConfig {
            preprocessing: Some(Default::default()),
            ..Default::default()
        },
        None,
        false,
        allow_bounded_inferred_upscale,
    )
}

#[test]
fn engine_selected_policy_fits_a_densityless_jpeg_upscale_to_the_workspace() {
    let prepared = prepare_densityless_jpeg_with_default_policy(true)
        .expect("an engine-selected policy shrinks its upscale instead of refusing the image");
    assert!(prepared.width > 800 && prepared.height > 1000);
    assert!(u64::from(prepared.width) * u64::from(prepared.height) * 8 <= 100 * 1024 * 1024);
}

#[test]
fn caller_selected_policy_keeps_its_requested_densityless_jpeg_upscale() {
    let error = prepare_densityless_jpeg_with_default_policy(false)
        .err()
        .expect("the requested 300 DPI raster does not fit")
        .to_string();
    assert!(error.contains("native image workspace"), "{error}");
}
