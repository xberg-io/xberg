#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::get_test_documents_dir;
use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, PdfBackend, PdfConfig};

async fn extract_page(stem: &str, digest: &str) -> ExtractedDocument {
    let path = get_test_documents_dir().join(format!("pdf/regression_pages/{stem}.pdf"));
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
    response.results.into_iter().next().unwrap()
}

#[tokio::test]
async fn should_keep_a_three_column_list_beside_a_section_divider_out_of_a_table() {
    let document = extract_page(
        "irs-p946--2024-p112",
        "9c0ddfabc5630e31e10d992ee9da3a88e73605e76132a3067fd3f67caf60f56a",
    )
    .await;
    assert!(
        document
            .content
            .lines()
            .any(|line| line == "# Tax Publications for Business Taxpayers"),
        "the page heading stays whole:\n{}",
        document.content
    );
    assert!(
        document
            .tables
            .iter()
            .all(|table| !table.markdown.contains("Social Security")),
        "the publication list columns are not a table: {:?}",
        document.tables
    );
}

#[tokio::test]
async fn should_not_repeat_table_rows_whose_cells_a_marker_or_sign_split() {
    let document = extract_page(
        "arxiv-2403.02163v1-p2",
        "5844dbd6a8f7d58cdbd1978618238bfa5247b8c68f7d0506e350f264a1a25898",
    )
    .await;
    let repeated: Vec<&str> = document
        .content
        .lines()
        .filter(|line| !line.starts_with('|'))
        .filter(|line| ["PICCOLO", "KUMC", "SUN", "UAH"].iter().any(|row| line.contains(row)))
        .filter(|line| !line.starts_with("**Table"))
        .collect();
    assert!(
        repeated.is_empty(),
        "table rows were emitted again as prose: {repeated:?}\n{}",
        document.content
    );
}
