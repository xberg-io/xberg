//! Coverage for xberg-io/xberg#1784.
//!
//! `tesseract_config.thresholding_method` used to be a boolean that Tesseract's integer parser
//! read as 0 whatever it held, so every value gave byte-identical text. The three named
//! methods must now reach the engine: on a scanned table page with grey-filled rows that reaches
//! Tesseract in grey (xberg's own binarization off), Otsu and Sauvola read different rows, so
//! their outputs differ.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests
#![cfg(all(feature = "ocr", feature = "pdf"))]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::types::{ImagePreprocessingConfig, TesseractConfig};

const SCANNED_TABLE: &[u8] = include_bytes!("fixtures/ocr/shaded_table_scan.pdf");

fn ocr_text(thresholding_method: &str) -> String {
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                thresholding_method: thresholding_method.to_string(),
                // xberg's default preprocessing binarizes the page with Otsu before Tesseract
                // sees it, which makes every engine method read the same bilevel image; the
                // engine thresholds only a page that reaches it in grey.
                preprocessing: Some(ImagePreprocessingConfig {
                    binarization_method: "none".to_string(),
                    deskew: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    extract_bytes_document_blocking(SCANNED_TABLE, "application/pdf", &config)
        .expect("forced OCR of the scanned table must succeed")
        .content
}

#[test]
fn otsu_and_sauvola_read_a_page_with_grey_filled_rows_differently() {
    let otsu = ocr_text("otsu");
    let sauvola = ocr_text("sauvola");
    assert!(!otsu.trim().is_empty() && !sauvola.trim().is_empty());
    assert_ne!(
        otsu, sauvola,
        "the method must reach the engine; identical text means it did not"
    );
}
