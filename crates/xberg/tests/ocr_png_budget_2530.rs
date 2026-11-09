#![cfg(all(feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::ExtractionConfig;
use xberg::core::config::OcrConfig;

/// Native bytes per pixel of the 8-bit contrast route: three 8-bit planes and a 32-bit accumulator, measured
/// at 7.0 by `reduced_jpeg_memory_tests::contrast::gray_route_peak_stays_within_its_model`.
const GRAY_CONTRAST_BYTES_PER_PIXEL: usize = 7;
/// One byte below what the gray contrast route needs for the 600x800 sales report at 300 DPI (2500x3333
/// pixels), so a request for exactly that raster is refused and an engine-selected one must shrink.
const BELOW_GRAY_300_DPI: usize = GRAY_CONTRAST_BYTES_PER_PIXEL * 2500 * 3333 - 1;

const EXPECTED: &str = "Docling bundles PDF document conversion to JSON and Markdown in an easy self contained package";

#[test]
fn budgeted_sparse_retry_preserves_the_rotated_png_sentence() {
    let path = helpers::get_test_documents_dir().join("images/ocr_test_rotated_90.png");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "9a67a012fe80a9a07c639a9a59ddb98feed64f4604d4d7dacd698e0a7d642004"
    );
    let result = helpers::extract_bytes_document_blocking(
        &bytes,
        "image/png",
        &ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        },
    )
    .expect("OCR succeeds with the default workspace");
    assert_eq!(
        result.content.split_whitespace().collect::<Vec<_>>().join(" "),
        EXPECTED
    );
    assert_eq!(
        result.metadata.additional.get("psm").and_then(|psm| psm.as_str()),
        Some("3")
    );
    let dpi = result
        .metadata
        .image_preprocessing
        .expect("actual contrast retry normalization");
    assert_eq!(dpi.target_dpi, 300);
    // These pages are gray, so the 8-bit contrast route (about 7 bytes per pixel) fits the whole 300 DPI
    // upscale in the default workspace. `*_shrinks_under_a_lower_limit` keeps the shrink covered. ~keep
    assert_eq!(dpi.final_dpi, 300, "reported DPI: {dpi:?}");
    assert_eq!(dpi.calculated_dpi, Some(dpi.final_dpi));
}

#[test]
fn budgeted_sparse_retry_recovers_the_densityless_rotated_sales_report() {
    let path = helpers::get_test_documents_dir().join("images/complex_document_rotated_90.png");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "2d500ac3b438c7e97cdf847ace1d3bf77ec5838a5bfe87ae018610f34ea31c60"
    );
    let result = helpers::extract_bytes_document_blocking(
        &bytes,
        "image/png",
        &ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        },
    )
    .expect("OCR succeeds with the default workspace");
    let text = result.content.split_whitespace().collect::<Vec<_>>().join(" ");
    for expected in [
        "Sales Report 2024",
        "This report contains quarterly sales data for our products.",
        "Q1 Sales: Q2 Sales:",
        "Product Units Revenue Product Units Revenue",
        "Prepared by: John Doe",
        "Date: 2024-03-15",
        "Department: Finance",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in {text:?}");
    }
    assert_eq!(text.matches("Widget ").count(), 6);
    assert_eq!(
        result.metadata.additional.get("psm").and_then(|psm| psm.as_str()),
        Some("3")
    );
    let dpi = result
        .metadata
        .image_preprocessing
        .expect("contrast retry normalization");
    assert_eq!(dpi.target_dpi, 300);
    assert_eq!(dpi.final_dpi, 300, "reported DPI: {dpi:?}");
    assert_eq!(dpi.calculated_dpi, Some(dpi.final_dpi));
}

#[test]
fn explicit_contrast_psm_does_not_use_the_implicit_sparse_retry_dpi_policy() {
    let path = helpers::get_test_documents_dir().join("images/complex_document_rotated_90.png");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    let error = helpers::extract_bytes_document_blocking(
        &bytes,
        "image/png",
        &ExtractionConfig {
            use_cache: false,
            security_limits: Some(xberg::SecurityLimits {
                max_content_size: BELOW_GRAY_300_DPI,
                ..Default::default()
            }),
            ocr: Some(OcrConfig {
                tesseract_config: Some(xberg::types::TesseractConfig {
                    psm: Some(3),
                    preprocessing: Some(xberg::types::ImagePreprocessingConfig {
                        deskew: false,
                        contrast_enhance: true,
                        binarization_method: "none".to_owned(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .expect_err("an explicit contrast policy still requires its requested raster budget");
    assert!(
        error.to_string().contains("OCR native image workspace exceeds"),
        "{error}"
    );
}

#[test]
fn budgeted_sparse_retry_shrinks_its_upscale_under_a_lower_limit() {
    let path = helpers::get_test_documents_dir().join("images/complex_document_rotated_90.png");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    let result = helpers::extract_bytes_document_blocking(
        &bytes,
        "image/png",
        &ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig::default()),
            security_limits: Some(xberg::SecurityLimits {
                max_content_size: BELOW_GRAY_300_DPI,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .expect("an engine-selected upscale shrinks instead of failing");
    let dpi = result
        .metadata
        .image_preprocessing
        .expect("contrast retry normalization");
    assert!(
        (150..300).contains(&dpi.final_dpi),
        "the 300 DPI raster does not fit, so the retry must shrink: {dpi:?}"
    );
    assert_eq!(dpi.calculated_dpi, Some(dpi.final_dpi));
}
