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

#[tokio::test]
async fn should_keep_seven_columns_where_source_rules_start_at_fractional_row_edges() {
    let tables = extract_budget_page(191, "4193e7b4abaa7e1e36a7b1c29f8b181aa4638eca91a4ccb96f5e3bd77aed2f74").await;
    assert_eq!(
        tables.iter().map(|table| table.cells.len()).collect::<Vec<_>>(),
        [7, 3, 6, 4]
    );
    let food = &tables[2..3];
    for (label, values) in [
        (
            "Housing assistance",
            ["12,626", "5,226", "7,400", "13,841", "5,190", "8,651"],
        ),
        (
            "Food stamp program",
            ["12,407", "10,448", "1,959", "13,145", "11,152", "1,993"],
        ),
        ("Child nutrition", ["4,060", "138", "3,922", "4,305", "146", "4,158"]),
        ("Supplemental feeding", ["1,670", "*", "1,669", "1,804", "3", "1,801"]),
        ("Commodity donations", ["725", "10", "716", "745", "9", "736"]),
    ] {
        assert_source_row(food, label, &values);
    }
    assert_eq!(
        food[0].cells[1],
        ["Food and nutrition assistance:", "", "", "", "", "", ""]
    );
}

#[tokio::test]
async fn fractional_rules_refine_housing_cells_without_repartitioning_the_sibling_section() {
    let tables = extract_budget_page(190, "79855f73beedeaf6acac434337a85525eb8db9627d352344fb621244349ed49d").await;
    assert_eq!(tables.len(), 6);
    assert_source_row(
        &tables[3..4],
        "Student assistance",
        &["7,036 6,966", "70 7,800", "7,724 76", "8,036 7,954", "82"],
    );
    for (label, values) in [
        (
            "Housing assistance",
            [
                "9,932", "4,163", "5,769", "11,200", "4,783", "6,417", "12,279", "4,836", "7,443",
            ],
        ),
        (
            "Food stamp program",
            [
                "12,375", "10,829", "1,547", "12,526", "10,815", "1,710", "12,443", "10,651", "1,792",
            ],
        ),
        (
            "Child nutrition",
            [
                "3,552", "192", "3,361", "3,681", "201", "3,480", "3,835", "157", "3,678",
            ],
        ),
        (
            "Supplemental feeding",
            ["1,367", "2", "1,365", "1,495", "*", "1,495", "1,578", "*", "1,577"],
        ),
        (
            "Commodity donations",
            ["677", "9", "668", "756", "9", "747", "668", "9", "660"],
        ),
    ] {
        assert_source_row(&tables[4..5], label, &values);
    }
}
