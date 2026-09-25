//! Coverage for xberg-io/xberg#1785.
//!
//! `shaded_table_scan.pdf` is a synthetic table page (one 196 dpi JPEG 2000 image) with three
//! row fills: dark bold text on a light grey band, white text on a mid grey band, and white
//! text on a dark band. Tesseract's page-wide threshold drops the light-fill rows whole.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests
#![cfg(all(feature = "ocr", feature = "pdf"))]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::types::{ImagePreprocessingConfig, TesseractConfig};

const SCANNED_TABLE: &[u8] = include_bytes!("fixtures/ocr/shaded_table_scan.pdf");
/// A light-fill row: label and its six values, as drawn.
const LIGHT_FILL_ROW: (&str, [&str; 6]) = (
    "SUBTOTAL FRUIT",
    ["80,532", "82,992", "85,400", "87,881", "90,435", "93,063"],
);

fn light_fill_values_read(normalize_shaded_rows: bool) -> usize {
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                preprocessing: Some(ImagePreprocessingConfig {
                    normalize_shaded_rows,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(SCANNED_TABLE, "application/pdf", &config)
        .expect("forced OCR of the scanned table must succeed");
    let (label, values) = LIGHT_FILL_ROW;
    document
        .content
        .lines()
        .filter(|line| line.contains(label))
        .map(|line| values.iter().filter(|value| line.contains(*value)).count())
        .max()
        .unwrap_or(0)
}

#[test]
fn a_light_fill_row_is_read_when_shaded_rows_are_normalized_and_dropped_when_they_are_not() {
    let without = light_fill_values_read(false);
    let with = light_fill_values_read(true);
    assert_eq!(
        without, 0,
        "off: the page-wide threshold drops the light-fill row whole"
    );
    assert!(with >= 4, "on: the light-fill row's values are read, got {with} of 6");
}
