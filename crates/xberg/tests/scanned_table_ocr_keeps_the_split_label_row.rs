//! Regression coverage for xberg-io/xberg#1797.
//!
//! A scanned table page (`shaded_table_scan.pdf`, one JPEG 2000 image, synthetic content)
//! whose row labels span several words. Under the sparse-text segmentation mode, OCR word
//! grouping leaves one word of a label in a header-less column 1 of its own, and the
//! column-sparsity gate then rejected the whole table, so the page came back with no table.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests
#![cfg(all(feature = "ocr", feature = "pdf"))]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::types::TesseractConfig;

const SCANNED_TABLE: &[u8] = include_bytes!("fixtures/ocr/shaded_table_scan.pdf");

#[test]
fn scanned_table_ocr_keeps_the_split_label_row() {
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: Some(11),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(SCANNED_TABLE, "application/pdf", &config)
        .expect("forced OCR of the scanned table must succeed");

    assert!(
        !document.tables.is_empty(),
        "the page must come back with a table; content was:\n{}",
        document.content
    );
    let table = &document.tables[0];
    assert!(
        table.cells.iter().all(|row| row.len() == 7),
        "the table must have a label column and six value columns, got rows: {:?}",
        table.cells
    );
    let label_row = table
        .cells
        .iter()
        .find(|row| row[0] == "CLOSING STOCK BALANCE")
        .unwrap_or_else(|| panic!("the multi-word label must be whole, got rows: {:?}", table.cells));
    let numeric_cells = label_row[1..]
        .iter()
        .filter(|cell| cell.chars().any(|c| c.is_ascii_digit()))
        .count();
    assert!(
        numeric_cells >= 4,
        "the label's values must stay on its row, got {label_row:?}"
    );
}
