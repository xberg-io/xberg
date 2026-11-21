//! Regression test for a ruled table whose rules are painted bars
//! (xberg-io/xberg#3352).
//!
//! ParseBench record `000999c16d74b494e344` is a 38-row rate table. Its row
//! rules are 1 pt filled rectangles rather than stroked segments, so counting
//! only stroked lines left the table rule-less; the columnar-prose guard then
//! dropped it because its numeric columns are all the same width. The fixture
//! reproduces that shape: painted horizontal bars, no strokes, five columns of
//! uniform-width numbers.

#![cfg(feature = "pdf")]

mod helpers;

use flate2::read::GzDecoder;
use helpers::extract_bytes_document_blocking;
use std::io::Read;
use xberg::{ExtractedDocument, ExtractionConfig};

const FIXTURE: &[u8] = include_bytes!("fixtures/pdf_ruled_rate_table_3352/painted-rules.pdf.gz");

/// Painted rules around prose and across a chart must not become table
/// evidence: the box's two spanning rules surround justified prose, and the
/// axis ticks are shorter than the region that carries them.
const NON_TABLES: &[u8] = include_bytes!("fixtures/pdf_ruled_non_tables_3352/boxed-prose-and-axis.pdf.gz");

fn extract_fixture(fixture: &[u8]) -> ExtractedDocument {
    let mut pdf = Vec::new();
    GzDecoder::new(fixture).read_to_end(&mut pdf).unwrap();
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        ..Default::default()
    };
    extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap()
}

fn extract() -> ExtractedDocument {
    extract_fixture(FIXTURE)
}

#[test]
fn table_with_painted_rules_is_detected() {
    let document = extract();

    let [table] = document.tables.as_slice() else {
        panic!(
            "expected one ruled rate table, got {}: {}",
            document.tables.len(),
            document.content
        );
    };

    assert!(
        table.cells.len() >= 30,
        "expected the 31 rate rows to survive, got {}",
        table.cells.len()
    );
    let first = &table.cells[0];
    assert!(
        first.iter().any(|cell| cell.contains("Age")),
        "header must name the age column, got {first:?}"
    );
    let last_row = table.cells.last().unwrap();
    assert_eq!(last_row.first().map(String::as_str), Some("95"), "{last_row:?}");
}

#[test]
fn painted_rules_around_prose_or_axis_ticks_are_not_a_table() {
    let document = extract_fixture(NON_TABLES);

    assert!(
        document.tables.is_empty(),
        "painted non-table rules were admitted: {}",
        document.content
    );
}
