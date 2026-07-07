//! Regression guard for xberg-io/xberg#1223: a PPTX table cell with a
//! horizontal merge must keep every following cell in its own column. xberg's
//! own DrawingML parser iterates all `<a:tc>` including the `hMerge` placeholder
//! that OOXML keeps for a `gridSpan`, so the grid stays rectangular. This pins
//! that behavior — the same aligned blank-continuation the DOCX/HTML merge fix
//! settled on — so a future naive re-index can't silently shift columns.

#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::ExtractionConfig;

const PPTX_MIME: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";

#[test]
fn grid_span_keeps_column_alignment() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/office/merged_table.pptx");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipping: fixture not present at {path:?}");
        return;
    };
    // PPTX builds structured tables from the markdown round-trip, so the
    // structured `doc.tables` surface is only populated under Markdown output.
    let cfg = ExtractionConfig {
        output_format: xberg::OutputFormat::Markdown,
        ..Default::default()
    };
    let doc = extract_bytes_document_blocking(&bytes, PPTX_MIME, &cfg).expect("extraction must succeed");
    let table = doc.tables.first().expect("a table must be extracted");

    // Header cell "Fuse" spans columns 0-1 (gridSpan=2); "Circuit" must stay in
    // the last column, and the data rows must line up under it.
    let joined = table.cells.iter().flatten().cloned().collect::<Vec<_>>().join(" | ");
    assert!(joined.contains("Fuse"), "merged header present: {joined}");
    assert!(joined.contains("Circuit"), "circuit header present: {joined}");

    // The 101 data row must stay ordered 101, 40A, Blower — not shifted.
    let data_row = table
        .cells
        .iter()
        .find(|r| r.iter().any(|c| c == "101"))
        .expect("row with 101 must exist");
    let row_text = data_row.join(" | ");
    let pos_101 = row_text.find("101").unwrap();
    let pos_40a = row_text.find("40A").expect("40A in same row");
    let pos_blower = row_text.find("Blower").expect("Blower in same row");
    assert!(
        pos_101 < pos_40a && pos_40a < pos_blower,
        "fuse row must stay in order 101, 40A, Blower: {row_text}"
    );
}
