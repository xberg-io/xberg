//! Regression test for xberg-io/xberg#1223 / #1212: DOCX table cells after a
//! horizontal merge must keep their column position. A cell following a
//! gridSpan=2 must land in column 2, not shift to column 1.

#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::ExtractionConfig;

const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

#[test]
fn horizontal_merge_keeps_column_alignment() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/office/merged_cells.docx");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipping: fixture not present at {path:?}");
        return;
    };
    let doc = extract_bytes_document_blocking(&bytes, DOCX_MIME, &ExtractionConfig::default())
        .expect("extraction must succeed");
    let table = doc.tables.first().expect("a table must be extracted");

    // Header row merges columns 0-1 into "Fuse"; the "Circuit" header must be in
    // the last column, and the data rows must line up under it. The key check:
    // the "Circuit"/data association survives — 101|40A|Blower and 102|50A|Cooling
    // stay as intact rows.
    let flat: Vec<String> = table.cells.iter().flatten().cloned().collect();
    let joined = flat.join(" | ");
    assert!(joined.contains("Fuse"), "merged header present: {joined}");
    assert!(joined.contains("Circuit"), "circuit header present: {joined}");

    // Find the data row with "101" and assert 40A + Blower are its neighbors in
    // order (association intact, not shifted).
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
