#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::get_test_documents_dir;
use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractionConfig, OutputFormat, PdfBackend, PdfConfig, Table};

const DOT_PLACEHOLDER: &str = "................";

async fn extract_budget_page(page: usize, digest: &str) -> Vec<Table> {
    let path = get_test_documents_dir().join(format!(
        "pdf/regression_pages/budget-fy1997-historical-tables-p{page}.pdf"
    ));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "required public corpus page {}: {error}; run test_documents/scripts/fetch_corpus.py",
            path.display()
        )
    });
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        digest,
        "public source PDF identity"
    );
    let config = ExtractionConfig {
        use_cache: false,
        disable_ocr: true,
        output_format: OutputFormat::Markdown,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let response = xberg::extract(ExtractInput::from_bytes(bytes, "application/pdf", None), &config)
        .await
        .unwrap();
    assert!(response.errors.is_empty());
    assert_eq!(response.results.len(), 1);
    let document = response.results.into_iter().next().unwrap();
    assert!(!document.content.is_empty());
    document.tables
}

fn assert_source_row(tables: &[Table], label: &str, values: &[&str]) {
    let matches: Vec<_> = tables
        .iter()
        .flat_map(|table| &table.cells)
        .filter(|row| row.first().is_some_and(|cell| cell.starts_with(label)))
        .collect();
    assert_eq!(matches.len(), 1, "one physical source row for {label}");
    assert_eq!(
        matches[0].len(),
        values.len() + 1,
        "source column count for {label}: {:?}",
        matches[0]
    );
    assert_eq!(
        matches[0][1..],
        values.iter().map(|value| value.to_string()).collect::<Vec<_>>(),
        "exact ordered source amounts and placeholders for {label}"
    );
}

#[tokio::test]
async fn should_preserve_ten_source_columns_in_the_1984_to_1986_budget() {
    let tables = extract_budget_page(190, "79855f73beedeaf6acac434337a85525eb8db9627d352344fb621244349ed49d").await;
    assert_eq!(tables.len(), 6, "existing section inventory remains intact");
    assert_source_row(
        &tables,
        "Hospital and medical care for veterans",
        &["8,604", "8,545", "59", "9,264", "9,180", "84", "9,634", "9,553", "81"],
    );
    assert_source_row(
        &tables,
        "Medicare: hospital insurance",
        &[
            "41,476",
            "41,476",
            DOT_PLACEHOLDER,
            "47,841",
            "47,841",
            DOT_PLACEHOLDER,
            "49,018",
            "49,018",
            DOT_PLACEHOLDER,
        ],
    );
    assert_source_row(
        &tables,
        "Medicaid",
        &[
            "20,061",
            DOT_PLACEHOLDER,
            "20,061",
            "22,655",
            DOT_PLACEHOLDER,
            "22,655",
            "24,995",
            DOT_PLACEHOLDER,
            "24,995",
        ],
    );
}

#[tokio::test]
async fn should_preserve_seven_source_columns_in_the_1987_to_1988_budget() {
    let tables = extract_budget_page(191, "4193e7b4abaa7e1e36a7b1c29f8b181aa4638eca91a4ccb96f5e3bd77aed2f74").await;
    assert_eq!(tables.len(), 4, "existing section inventory remains intact");
    assert_source_row(
        &tables,
        "Hospital and medical care for veterans",
        &["10,061", "9,967", "94", "10,714", "10,608", "106"],
    );
    assert_source_row(
        &tables,
        "Medicare: hospital insurance",
        &["49,976", "49,976", DOT_PLACEHOLDER, "52,022", "52,022", DOT_PLACEHOLDER],
    );
    assert_source_row(
        &tables,
        "Medicaid",
        &["27,435", DOT_PLACEHOLDER, "27,435", "30,462", DOT_PLACEHOLDER, "30,462"],
    );
}
