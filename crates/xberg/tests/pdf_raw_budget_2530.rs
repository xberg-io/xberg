#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime", paddle_ocr))]

mod helpers;

use xberg::types::{OcrBoundingGeometry, OcrElementConfig, OcrElementLevel};
use xberg::{ExtractionConfig, OcrConfig, PageConfig, SecurityLimits};

const SCAN: &[u8] = include_bytes!("fixtures/pdf_raw_budget_2530/title-layer-scan.pdf");

#[test]
fn tesseract_reads_a_title_layer_and_scan_after_its_render_exceeds_native_workspace() {
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            element_config: Some(OcrElementConfig {
                include_elements: true,
                min_level: OcrElementLevel::Word,
                ..Default::default()
            }),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        security_limits: Some(SecurityLimits {
            max_content_size: 64 * 1024 * 1024,
            ..Default::default()
        }),
        ..Default::default()
    };
    let run =
        |config: &ExtractionConfig| helpers::extract_bytes_document_blocking(SCAN, "application/pdf", config).unwrap();
    let native = run(&ExtractionConfig {
        disable_ocr: true,
        ..config.clone()
    });
    assert_eq!(native.content.trim(), "Signed Native Source Title");

    let result = run(&config);
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 1);
    let page = &pages[0];
    assert_eq!(
        page.ocr_confidence.as_ref().unwrap().backend.as_str(),
        "tesseract",
        "{:?}",
        result.processing_warnings
    );
    assert!(result.content.contains("Signed Native Source Title"));
    assert!(result.content.contains("Quarterly Accounts Report"));
    let record_numbers: Vec<_> = result
        .content
        .lines()
        .filter_map(|line| {
            line.strip_prefix("Record ")?
                .split_whitespace()
                .next()?
                .parse::<u8>()
                .ok()
        })
        .collect();
    assert_eq!(record_numbers, (1..=30).collect::<Vec<_>>());
    let dpi = page.image_preprocessing.as_ref().expect("bounded OCR scale metadata");
    assert_eq!(dpi.original_dpi.horizontal, 41.0);
    let dimensions = dpi.new_dimensions.expect("smaller raster dimensions");
    assert!(dimensions.width < dpi.original_dimensions.width);
    let effective = 41.0 * dimensions.width as f64 / dpi.original_dimensions.width as f64;
    assert_eq!(dpi.final_dpi, effective.round() as i32);
    assert!(dpi.final_dpi < 41);
    assert_eq!(dpi.calculated_dpi, Some(dpi.final_dpi));
    assert!(dpi.scale_factor < 1.0 && !dpi.skipped_resize);
    let words = result.ocr_elements.as_ref().expect("requested OCR elements");
    let heading = words
        .iter()
        .find(|element| element.level == OcrElementLevel::Word && element.text == "Quarterly")
        .expect("raster heading geometry");
    let OcrBoundingGeometry::Rectangle { left, .. } = heading.geometry else {
        panic!("Tesseract heading must have a rectangle");
    };
    // The source draws the heading 200px into a 4,600px image placed from x=250
    // at a width of 1,600pt. Convert that known position to the 41 DPI render frame.
    let expected_x = (250.0 + 1600.0 * 200.0 / 4600.0) * 41.0 / 72.0;
    assert!(
        (f64::from(left) - expected_x).abs() < 5.0,
        "heading at {left}, expected {expected_x}"
    );
    assert!(result.processing_warnings.iter().any(|warning| {
        warning.source == "pdf-render" && warning.message.contains("fit security_limits.max_content_size")
    }));
}
