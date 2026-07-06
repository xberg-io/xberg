//! Regression tests for xberg-io/xberg#1223: merged HTML table cells must keep
//! their column alignment. A cell under a `rowspan` must not shift left into the
//! spanning column.

#![cfg(feature = "html")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::ExtractionConfig;

fn table_cells(html: &[u8]) -> Vec<Vec<String>> {
    let doc = extract_bytes_document_blocking(html, "text/html", &ExtractionConfig::default())
        .expect("extraction must succeed");
    doc.tables.first().map(|t| t.cells.clone()).unwrap_or_default()
}

/// A `rowspan=2` in the first column must leave that column empty on the second
/// row, so the following cells stay aligned with their headers.
#[test]
fn rowspan_keeps_following_cells_aligned() {
    let html = br#"<table>
<tr><th>Group</th><th>Name</th><th>Score</th></tr>
<tr><td rowspan="2">Alpha</td><td>Alice</td><td>10</td></tr>
<tr><td>Bob</td><td>20</td></tr>
</table>"#;
    let cells = table_cells(html);
    assert!(cells.len() >= 3, "expected 3 rows, got: {cells:?}");
    // Row 2 (the second data row) must read ["", "Bob", "20"] — Bob in the Name
    // column, not shifted into Group.
    let last = cells.last().unwrap();
    assert_eq!(last.len(), 3, "row must have 3 columns: {last:?}");
    assert_eq!(
        last[0], "",
        "the rowspan column must be empty on the spanned row: {last:?}"
    );
    assert_eq!(last[1], "Bob", "Bob must stay in the Name column: {last:?}");
    assert_eq!(last[2], "20", "20 must stay in the Score column: {last:?}");
}

/// A `colspan` header keeps the grid width and the data row stays aligned.
#[test]
fn colspan_header_preserves_width() {
    let html = br#"<table>
<tr><th colspan="2">Fuse</th><th>Circuit</th></tr>
<tr><td>101</td><td>40A</td><td>Blower</td></tr>
</table>"#;
    let cells = table_cells(html);
    let data = cells.last().unwrap();
    assert_eq!(
        data,
        &vec!["101".to_string(), "40A".to_string(), "Blower".to_string()],
        "data row: {data:?}"
    );
}
