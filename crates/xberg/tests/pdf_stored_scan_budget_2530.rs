#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime", paddle_ocr))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::core::config::OcrQualityThresholds;
use xberg::types::{OcrBoundingGeometry, OcrElementConfig, OcrElementLevel, TesseractConfig};
use xberg::{ExtractionConfig, OcrConfig, PageConfig};

#[test]
fn certified_stored_scan_recovers_source_words_under_the_default_ocr_budget() {
    let fixture = helpers::get_test_file_path("pdf_scanned/docling_scanned.pdf");
    let bytes =
        std::fs::read(&fixture).unwrap_or_else(|error| panic!("required source {}: {error}", fixture.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "874bada67b7bf700d42df25265823799955ae37ba3ec0e283747d1f2cd5d94f7"
    );
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            element_config: Some(OcrElementConfig {
                include_elements: true,
                min_level: OcrElementLevel::Word,
                ..Default::default()
            }),
            quality_thresholds: Some(OcrQualityThresholds {
                pipeline_min_quality: 0.0,
                ..Default::default()
            }),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let run = |config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(&bytes, "application/pdf", config).unwrap()
    };
    let result = run(&config);
    let pages = result.pages.as_deref().expect("requested page output");
    assert_eq!(pages.len(), 9);
    // These literals are independently present on the matching pages of pdf/docling.pdf.
    // The constrained 146-DPI read loses them before the compressed-source retry.
    for (number, anchors) in [
        (7, ["categories", "commercial", "repositories"]),
        (8, ["birgit", "learning", "yolov5x"]),
    ] {
        let page = &pages[number - 1];
        assert_eq!(page.page_number as usize, number);
        assert_eq!(page.ocr_confidence.as_ref().unwrap().backend, "tesseract");
        let folded = page.content.to_ascii_lowercase();
        let words = folded.split(|ch: char| !ch.is_ascii_alphanumeric()).collect::<Vec<_>>();
        for anchor in anchors {
            assert!(words.contains(&anchor), "page {number} lost source word {anchor:?}");
        }
        let dpi = page.image_preprocessing.as_ref().expect("actual source normalization");
        assert_eq!(dpi.original_dimensions.width, 1700);
        assert_eq!(dpi.original_dimensions.height, 2200);
        assert_eq!(dpi.original_dpi.horizontal, 96.0);
        assert_eq!(dpi.final_dpi, 179);
    }
    assert!(!result.tables.iter().any(|table| matches!(table.page_number, 7 | 8)));
    assert!(result.tables.iter().any(|table| table.page_number == 9));
    let page_nine = pages[8].image_preprocessing.as_ref().unwrap();
    assert_eq!(page_nine.original_dimensions.width, 2586);
    assert_eq!(page_nine.original_dimensions.height, 3346);
    assert!(
        result.processing_warnings.iter().any(|warning| {
            warning.source == "ocr" && warning.message.contains("Page 7") && warning.message.contains("68%")
        }),
        "the original discarded or low-confidence read remains observable"
    );
    assert_birgit_render_geometry(&result);

    let mut explicit = config;
    explicit.ocr.as_mut().unwrap().tesseract_config = Some(TesseractConfig::default());
    let control = run(&explicit);
    let control_pages = control.pages.as_ref().unwrap();
    assert_eq!(
        pages[8].content, control_pages[8].content,
        "retain the existing page-9 table and text byte for byte"
    );
    for index in [0, 1, 2, 3, 4, 5] {
        assert_eq!(
            pages[index].content, control_pages[index].content,
            "do not retry ordinarily readable pages"
        );
    }
    assert_eq!(
        control_pages[6]
            .image_preprocessing
            .as_ref()
            .unwrap()
            .original_dimensions
            .width,
        2586,
        "explicit Tesseract options stay on the rendered route"
    );
}

fn assert_birgit_render_geometry(result: &xberg::ExtractedDocument) {
    let word = result
        .ocr_elements
        .as_ref()
        .unwrap()
        .iter()
        .find(|word| {
            word.page_number == 8 && word.level == OcrElementLevel::Word && word.text.eq_ignore_ascii_case("birgit")
        })
        .expect("requested source word geometry");
    let OcrBoundingGeometry::Rectangle { left, top, .. } = word.geometry else {
        panic!("Tesseract must return a rectangle");
    };
    // The same visible word is at (208.205, 228.895) on the sibling 612 × 792pt native page.
    let expected_x: f64 = 208.205 / 612.0 * 2586.0;
    let expected_y: f64 = 228.895 / 792.0 * 3346.0;
    assert!(
        (f64::from(left) - expected_x).abs() < 40.0,
        "word left {left} expected {expected_x}"
    );
    assert!(
        (f64::from(top) - expected_y).abs() < 40.0,
        "word top {top} expected {expected_y}"
    );
}
