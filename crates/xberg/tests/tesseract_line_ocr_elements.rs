//! Tesseract OCR elements at the default granularity. `OcrElementConfig::min_level` defaults to
//! `Line`, so a Tesseract caller who opts in without naming a level must still get elements.

#![cfg(all(feature = "ocr", feature = "pdf"))]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::types::{OcrElementConfig, OcrElementLevel};

const TABLE_IMAGE: &[u8] = include_bytes!("fixtures/ocr/right_aligned_amounts_scan.png");
const SCANNED_PDF: &[u8] = include_bytes!("fixtures/ocr/scanned_hello.pdf");

fn tesseract_config_requesting_elements(element_config: OcrElementConfig) -> ExtractionConfig {
    ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            element_config: Some(element_config),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn default_granularity() -> OcrElementConfig {
    OcrElementConfig {
        include_elements: true,
        ..Default::default()
    }
}

#[test]
fn tesseract_image_elements_at_the_default_level_are_lines() {
    let document = extract_bytes_document_blocking(
        TABLE_IMAGE,
        "image/png",
        &tesseract_config_requesting_elements(default_granularity()),
    )
    .expect("OCR of the image must succeed");

    let elements = document
        .ocr_elements
        .expect("a caller who opts in at the default level must get elements");
    assert!(elements.iter().all(|element| element.level == OcrElementLevel::Line));
    assert!(
        elements
            .iter()
            .any(|element| element.text.split_whitespace().count() > 1)
    );
}

#[test]
fn tesseract_image_word_elements_come_with_their_lines() {
    let document = extract_bytes_document_blocking(
        TABLE_IMAGE,
        "image/png",
        &tesseract_config_requesting_elements(OcrElementConfig {
            include_elements: true,
            min_level: OcrElementLevel::Word,
            ..Default::default()
        }),
    )
    .expect("OCR of the image must succeed");

    let elements = document.ocr_elements.expect("word-level elements must be returned");
    let words = elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .map(|element| element.text.as_str())
        .collect::<Vec<_>>();
    let line_words = elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Line)
        .flat_map(|element| element.text.split(' '))
        .collect::<Vec<_>>();
    assert!(!words.is_empty());
    assert_eq!(line_words, words, "the lines hold exactly the words, in reading order");
}

#[test]
fn tesseract_pdf_elements_at_the_default_level_are_lines() {
    let document = extract_bytes_document_blocking(
        SCANNED_PDF,
        "application/pdf",
        &tesseract_config_requesting_elements(default_granularity()),
    )
    .expect("forced OCR of the scanned PDF must succeed");

    let elements = document
        .ocr_elements
        .expect("a caller who opts in at the default level must get elements");
    assert!(elements.iter().all(|element| element.level == OcrElementLevel::Line));
    assert!(elements.iter().all(|element| element.page_number == 1));
}
