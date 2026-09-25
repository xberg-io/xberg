//! Regression coverage for xberg-io/xberg#1797.
//!
//! A scanned table page (`shaded_table_scan.pdf`, one 196 dpi JPEG 2000 image, synthetic
//! content) whose row labels span several words: OCR word grouping leaves the last word of one
//! label in a header-less column of its own, and the column-sparsity gate then rejected the
//! whole table, so the page came back with no table at all.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests
#![cfg(all(feature = "ocr", feature = "pdf"))]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig};

const SCANNED_TABLE: &[u8] = include_bytes!("fixtures/ocr/shaded_table_scan.pdf");

#[test]
fn scanned_table_ocr_keeps_the_split_label_row() {
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
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
    let label_rows: Vec<&Vec<String>> = table
        .cells
        .iter()
        .filter(|row| row.first().is_some_and(|label| label.contains("BALANCE")))
        .collect();
    assert!(
        !label_rows.is_empty(),
        "a multi-word label row must be in the table, got rows: {:?}",
        table.cells
    );
    for row in label_rows {
        let numeric_cells = row[1..]
            .iter()
            .filter(|cell| cell.chars().any(|c| c.is_ascii_digit()))
            .count();
        assert!(
            numeric_cells >= 4,
            "the label's values must stay on its row, got {row:?}"
        );
    }
}
