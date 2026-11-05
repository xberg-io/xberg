#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use sha2::{Digest, Sha256};
use xberg::{ExtractionConfig, OutputFormat, PdfConfig};

#[test]
fn source_headers_and_separate_table_records_reach_the_public_result() {
    let bytes = include_bytes!("../../../fixtures/repros/table-snapshot-consistency/fixture/wildomar-p17-repro.pdf");
    assert_eq!(
        hex::encode(Sha256::digest(bytes)),
        "d21fb4bd4af939c647cd07adc4afe0344b06029879f1f347a81f0aee63776bbe"
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        pdf_options: Some(PdfConfig {
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(bytes, "application/pdf", &config).unwrap();
    assert_eq!(result.tables.len(), 2, "the source page contains two physical tables");
    let upper = result
        .tables
        .iter()
        .find(|table| {
            table.cells.first().is_some_and(|row| {
                row.iter()
                    .map(String::as_str)
                    .eq(["Tests", "Test Method", "Requirements"])
            })
        })
        .expect("source header row survives");
    assert_eq!(
        upper.columns.as_deref(),
        Some(["Tests".into(), "Test Method".into(), "Requirements".into()].as_slice())
    );
    assert_eq!(upper.cells.len(), 5);
    assert!(upper.cells.iter().all(|row| row.len() == 3));
    assert_eq!(upper.cells[2], ["Sand Equivalent", "ASTM D2419", "60 Minimum"]);
    assert_eq!(upper.cells[4], ["Durability", "CTM 229", "55 Minimum"]);
    let lower = result.tables.iter().find(|table| !std::ptr::eq(*table, upper)).unwrap();
    assert!(lower.cells.iter().all(|row| row.len() == 3));
    assert!(
        lower
            .cells
            .iter()
            .any(|row| row == &["No. 8 (2.36 mm)", "60 - 90", "90 - 100"])
    );
    assert!(
        lower
            .cells
            .iter()
            .any(|row| row == &["No. 100 (150 µm)", "5 - 20", "15 - 30"])
    );
}
