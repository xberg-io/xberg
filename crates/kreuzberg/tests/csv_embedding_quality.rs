//! Tests verifying CSV extraction produces embedding-friendly output.
//!
//! Header-value pairs preserve semantic associations between column names
//! and cell values, improving vector search quality over flat space-separated output.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;

/// Header-value pairs should be explicit in the content.
#[tokio::test]
async fn test_csv_preserves_header_value_association() {
    let config = ExtractionConfig::default();
    let csv = b"Name,Age,City\nAlice,30,NYC\nBob,25,LA\n";

    let result = extract_bytes(csv, "text/csv", &config).await.unwrap();

    assert!(result.content.contains("Name: Alice"));
    assert!(result.content.contains("Age: 30"));
    assert!(result.content.contains("City: NYC"));
    assert!(result.content.contains("Name: Bob"));
    assert!(result.content.contains("Age: 25"));
    assert!(result.content.contains("City: LA"));
}

/// Rows should be labeled and separated by blank lines for chunker-friendly splitting.
#[tokio::test]
async fn test_csv_row_grouping() {
    let config = ExtractionConfig::default();
    let csv = b"Name,Score\nAlice,95\nBob,88\nCarol,72\n";

    let result = extract_bytes(csv, "text/csv", &config).await.unwrap();

    assert!(result.content.contains("Row 1:"));
    assert!(result.content.contains("Row 2:"));
    assert!(result.content.contains("Row 3:"));
    assert!(
        result.content.contains("\n\nRow 2:"),
        "Rows should be separated by blank lines"
    );
}

/// Empty cells should be omitted to avoid noisy `Header:` lines.
#[tokio::test]
async fn test_csv_skips_empty_values() {
    let config = ExtractionConfig::default();
    let csv = b"Name,Age,City\nAlice,,NYC\nBob,25,LA\n";

    let result = extract_bytes(csv, "text/csv", &config).await.unwrap();

    assert!(result.content.contains("Name: Alice"));
    assert!(result.content.contains("City: NYC"));
    // Row 1 (Alice) should not have an Age line since it's empty
    let row1 = result.content.split("\n\n").next().unwrap_or("");
    assert!(!row1.contains("Age:"), "Empty cells should be skipped");
}

/// The tables field should still contain the full parsed structure.
#[tokio::test]
async fn test_csv_tables_field_unchanged() {
    let config = ExtractionConfig::default();
    let csv = b"Name,Age\nAlice,30\nBob,25\n";

    let result = extract_bytes(csv, "text/csv", &config).await.unwrap();

    assert_eq!(result.tables.len(), 1);
    assert_eq!(result.tables[0].cells.len(), 3);
    assert_eq!(result.tables[0].cells[0], vec!["Name", "Age"]);
    assert!(!result.tables[0].markdown.is_empty());
}

/// Rows shorter than the header should not panic; extra headers are skipped.
#[tokio::test]
async fn test_csv_short_row_no_panic() {
    let config = ExtractionConfig::default();
    let csv = b"Name,Age,City\nAlice,30\nBob,25,LA\n";

    let result = extract_bytes(csv, "text/csv", &config).await.unwrap();

    assert!(result.content.contains("Name: Alice"));
    assert!(result.content.contains("Age: 30"));
    // Alice's row has no City — should not appear
    let row1 = result.content.split("\n\n").next().unwrap_or("");
    assert!(!row1.contains("City:"));
}

/// Rows where all cells are empty should be omitted entirely.
#[tokio::test]
async fn test_csv_all_empty_data_rows() {
    let config = ExtractionConfig::default();
    let csv = b"Name,Age\n,,\nAlice,30\n";

    let result = extract_bytes(csv, "text/csv", &config).await.unwrap();

    // First row is all empty — should be skipped, Alice should be Row 1
    assert!(result.content.contains("Name: Alice"));
}

/// When no header is detected, should fall back to space-separated output.
#[tokio::test]
async fn test_csv_no_header_fallback() {
    let config = ExtractionConfig::default();
    // All text, no numbers — detect_header returns false
    let csv = b"Alice,NYC,Engineer\nBob,LA,Designer\n";

    let result = extract_bytes(csv, "text/csv", &config).await.unwrap();

    assert!(
        !result.content.contains("Row 1:"),
        "No header detected — should not label rows"
    );
    assert!(result.content.contains("Alice"));
    assert!(result.content.contains("Bob"));
}

/// Header-only CSV (no data rows) should still produce output.
#[tokio::test]
async fn test_csv_header_only() {
    let config = ExtractionConfig::default();
    let csv = b"Name,Age,City\n";

    let result = extract_bytes(csv, "text/csv", &config).await.unwrap();

    assert!(!result.content.is_empty());
    assert!(result.content.contains("Name"));
}

/// Real CSV file with header-value pairing.
#[tokio::test]
async fn test_csv_real_file_header_value() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/csv/data_table.csv");
    if !path.exists() {
        return;
    }
    let content = std::fs::read(&path).unwrap();
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "text/csv", &config).await.unwrap();

    assert!(result.content.contains("Name: Alice Johnson"));
    assert!(result.content.contains("Department: Engineering"));
    assert!(result.content.contains("Row 1:"));
}
