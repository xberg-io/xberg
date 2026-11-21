//! Regression tests for borderless two-column definition grids (xberg-io/xberg#3351).
//!
//! ParseBench record `0580ae4eb20f312e0b8b` carries eight `Factor`/value grids.
//! Their value column holds single letters (`x`) and ends with `=`, so every
//! row reads as a sentence continuation and the whole grid is discarded as
//! prose. The fixture below is the same shape reduced to one grid: detection
//! produced no table before the definition-grid path existed.

#![cfg(feature = "pdf")]

mod helpers;

use flate2::read::GzDecoder;
use helpers::extract_bytes_document_blocking;
use std::io::Read;
use xberg::{ExtractedDocument, ExtractionConfig};

const FIXTURE: &[u8] = include_bytes!("fixtures/pdf_definition_grids_3351/definition-grid.pdf.gz");

const EXPECTED: [[&str; 2]; 10] = [
    ["Factor", ""],
    ["MeanDebt", "XX"],
    ["Credit", "x"],
    ["UWTier", "x"],
    ["FDLFact", "x"],
    ["UnityOne", "1"],
    ["Install", "x"],
    ["ISITier", "x"],
    ["Downpay", "x"],
    ["DebtFact", "="],
];

fn extract() -> ExtractedDocument {
    let mut pdf = Vec::new();
    GzDecoder::new(FIXTURE).read_to_end(&mut pdf).unwrap();
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        ..Default::default()
    };
    extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap()
}

#[test]
fn borderless_definition_grid_is_detected() {
    let document = extract();

    let [table] = document.tables.as_slice() else {
        panic!(
            "expected one definition grid, got {}: {}",
            document.tables.len(),
            document.content
        );
    };

    let expected: Vec<Vec<String>> = EXPECTED
        .iter()
        .map(|row| row.iter().map(|cell| (*cell).to_string()).collect())
        .collect();
    assert_eq!(table.cells, expected);
}
