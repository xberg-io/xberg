//! GH#2332: an unambiguous table boundary must decide whether the rendered HTML
//! holds one or two tables, instead of splitting a table that every row rule keeps
//! together or merging tables that a blank gutter separates.
//!
//! The fixtures are written by `fixtures/gh2332_table_boundaries/generate.py`.
//!
//! Both cases are owned by the native boundary detector (`pdf/native/table`), not by
//! the structural recognition or the renderers, so they are pinned here ignored until
//! #3348 fixes the boundary decision.

#![cfg(feature = "pdf")]

mod helpers;

use flate2::read::GzDecoder;
use helpers::extract_bytes_document_blocking;
use std::io::Read;
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat};

const SIDE_BY_SIDE: &[u8] = include_bytes!("fixtures/gh2332_table_boundaries/side-by-side-gutter.pdf.gz");
const BLANK_BAND: &[u8] = include_bytes!("fixtures/gh2332_table_boundaries/continuous-grid-blank-band.pdf.gz");

fn extract(compressed: &[u8]) -> ExtractedDocument {
    let mut pdf = Vec::new();
    GzDecoder::new(compressed).read_to_end(&mut pdf).unwrap();
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Html,
        ..Default::default()
    };
    extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap()
}

#[test]
#[ignore = "native boundary detector owns this; tracked in #3348"] // merged into one 7-column table
fn blank_gutter_between_side_by_side_grids_keeps_two_tables() {
    let result = extract(SIDE_BY_SIDE);
    assert_eq!(
        result.tables.len(),
        2,
        "a gutter no rule crosses separates two source tables: {:?}",
        result.tables
    );
    assert_eq!(result.content.matches("<table>").count(), 2, "{}", result.content);
    assert_eq!(result.tables[0].cells[1], ["Iris", "North", "350"]);
    assert_eq!(result.tables[1].cells[1], ["Clock", "Week one", "Ada"]);
}

#[test]
#[ignore = "native boundary detector owns this; tracked in #3348"] // split into two tables
fn continuous_rules_through_a_blank_band_keep_one_table() {
    let result = extract(BLANK_BAND);
    let [table] = result.tables.as_slice() else {
        panic!(
            "the row rules run through the blank band, so this is one source table, got {}",
            result.tables.len()
        );
    };
    assert!(
        table.cells.iter().all(|row| row.len() == 3),
        "the blank band is part of the grid: {:?}",
        table.cells
    );
    let labels: Vec<&str> = table
        .cells
        .iter()
        .flat_map(|row| row.iter())
        .map(String::as_str)
        .filter(|cell| !cell.is_empty())
        .collect();
    assert_eq!(
        labels,
        [
            "Service", "Zone", "Capacity", "Iris", "North", "350", "Hazel", "East", "470", "Linden", "South", "590",
            "Aster", "West", "610"
        ]
    );
    assert_eq!(result.content.matches("<table>").count(), 1, "{}", result.content);
}
