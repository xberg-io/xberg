#![cfg(feature = "pdf")]

mod helpers;

use xberg::core::config::{ConcurrencyConfig, ExtractionConfig, OutputFormat, PageConfig};
use xberg::types::ExtractedDocument;

fn extract_markdown(fixture: &str) -> ExtractedDocument {
    let path = helpers::get_test_file_path(fixture);
    assert!(path.is_file(), "public PDF fixture is missing: {}", path.display());
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        concurrency: Some(ConcurrencyConfig {
            max_threads: Some(2),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            insert_page_markers: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    helpers::extract_uri_document_blocking(&path, Some("application/pdf"), &config)
        .expect("public PDF fixture extracts")
}

fn markdown_page(markdown: &str, number: u32) -> &str {
    let marker = format!("<!-- PAGE {number} -->");
    let next_marker = format!("<!-- PAGE {} -->", number + 1);
    let (_, rest) = markdown.split_once(&marker).expect("requested Markdown page marker");
    rest.split_once(&next_marker).expect("next Markdown page marker").0
}

fn is_redfish_take_rate(cell: &str) -> bool {
    let cell = cell.replace("<sup>3</sup>", "").replace('³', "");
    let tokens: Vec<_> = cell.split_whitespace().collect();
    matches!(tokens.as_slice(), ["14.3", "(13.6)"] | ["14.3", "3", "(13.6)"])
}

#[test]
fn redfish_bay_row_carries_its_take_rate_on_page_13() {
    const STOCK: &str = "Redfish Bay/Espiritu Santo Bay";
    let document = extract_markdown("pdf/2026-14578.pdf");
    let mut tables = document
        .tables
        .iter()
        .filter(|table| table.page_number == 13 && table.cells.iter().flatten().any(|cell| cell.contains(STOCK)));
    let table = tables
        .next()
        .expect("page 13 typed table contains the Redfish Bay stock");
    assert!(
        tables.next().is_none(),
        "only one page 13 typed table may contain the Redfish Bay stock"
    );
    assert!(table.cells.iter().all(|row| row.len() == 6), "six columns in every row");
    let header = table.cells.first().expect("stock table header");
    let stock_column = header
        .iter()
        .position(|cell| cell.trim() == "Stock")
        .expect("Stock column");
    let take_column = header
        .iter()
        .position(|cell| cell.split_whitespace().eq(["Take/r-PBR", "(%)"]))
        .expect("Take/r-PBR (%) column");
    let rows: Vec<_> = table
        .cells
        .iter()
        .skip(1)
        .filter(|row| row.get(stock_column).is_some_and(|cell| cell.contains(STOCK)))
        .collect();
    assert_eq!(rows.len(), 1, "expected a single Redfish Bay stock row");
    assert!(rows[0].get(take_column).is_some_and(|cell| is_redfish_take_rate(cell)));

    let markdown = markdown_page(&document.content, 13);
    let mut rows = markdown
        .lines()
        .filter(|line| line.trim_start().starts_with('|'))
        .map(|line| line.trim().trim_matches('|').split('|').collect::<Vec<_>>())
        .filter(|cells| cells.get(stock_column).is_some_and(|cell| cell.contains(STOCK)));
    let row = rows.next().expect("Markdown must contain the Redfish Bay stock row");
    assert!(rows.next().is_none(), "expected a single Redfish Bay Markdown row");
    assert!(row.get(take_column).is_some_and(|cell| is_redfish_take_rate(cell)));
}

#[test]
fn federal_register_preserves_the_rotated_stamp_on_page_8() {
    let document = extract_markdown("vendored/pdfplumber/pdf/federal-register-2020-17221.pdf");
    let markdown = markdown_page(&document.content, 8);
    let count = markdown
        .lines()
        .filter(|line| line.trim() == "jbell on DSKJLSW7X2PROD with PROPOSALS")
        .count();
    assert_eq!(count, 1, "expected one standalone stamp on page 8");
}

#[test]
fn algebra_preserves_the_standalone_folio_on_page_1441() {
    let document = extract_markdown(concat!(
        "pdf/algebra_topology_differential_calculus_and_optimization_theory_",
        "for_computer_science_and_machine_learning_2019_math_deep.pdf"
    ));
    let markdown = markdown_page(&document.content, 1441);
    let count = markdown.lines().filter(|line| line.trim() == "1441").count();
    assert_eq!(count, 1, "expected one standalone printed folio on page 1441");
    for page in [1440, 1442] {
        let printed_number = page.to_string();
        assert!(
            !markdown_page(&document.content, page)
                .lines()
                .any(|line| line.trim() == printed_number),
            "running top folio should stay suppressed on page {page}"
        );
    }
}
