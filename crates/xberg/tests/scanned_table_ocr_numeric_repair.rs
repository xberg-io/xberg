//! Coverage for xberg-io/xberg#1789.
//!
//! On `shaded_table_scan.pdf` (a synthetic table page, one 196 dpi JPEG 2000 image) the
//! repaired output carries no bare integer of four or more digits standing alone as a number
//! where the unrepaired output does, and every value the unrepaired output read correctly is
//! still there.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests
#![cfg(all(feature = "ocr", feature = "pdf"))]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::types::TesseractConfig;

const SCANNED_TABLE: &[u8] = include_bytes!("fixtures/ocr/shaded_table_scan.pdf");

fn ocr_text(numeric_repair: bool) -> String {
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                numeric_repair,
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

/// Whitespace-delimited tokens that are bare integers of four or more digits.
fn bare_long_integers(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .filter(|token| token.len() >= 4 && token.chars().all(|c| c.is_ascii_digit()))
        .collect()
}

#[test]
fn numeric_repair_restores_separators_without_losing_correct_values() {
    let plain = ocr_text(false);
    let repaired = ocr_text(true);
    assert!(
        bare_long_integers(&repaired).is_empty(),
        "repaired output must carry no bare long integer, got {:?}",
        bare_long_integers(&repaired)
    );
    for value in ["48,210", "21,406", "3,250", "41,920", "18,402"] {
        if plain.contains(value) {
            assert!(
                repaired.contains(value),
                "{value} was read correctly and must survive the repair"
            );
        }
    }
}
