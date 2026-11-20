#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

//! A six-column dot-leader table at 150 dpi keeps every row in HTML output (#2503).
//!
//! The 150 dpi scan route proves the table's rows against the source words, then claims the
//! page lines they cover. A row whose leader the table cell does not carry was left as page
//! text beside the table, so HTML emitted that row twice: once as a paragraph and once as a
//! table row. The claim now runs on the document the proof returns, with the leader artifacts
//! the table does not keep ignored.

mod helpers;

use sha2::{Digest, Sha256};
use xberg::types::TesseractConfig;
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const SIX_COLUMN_PDF: &[u8] = include_bytes!("fixtures/ocr_six_column_rows_2503/six-column-150dpi.pdf");
const SIX_COLUMN_ROWS: &str = include_str!("fixtures/ocr_six_column_rows_2503/six-column-150dpi.tsv");
const NO_LEADERS: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/clean_table.png");
const HEADER: [&str; 6] = ["Item", "Count", "Budget", "Change", "Actual", "Variance"];

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Html,
        ocr: Some(OcrConfig {
            tesseract_config: Some(TesseractConfig {
                psm: Some(6),
                ..Default::default()
            }),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn extract(bytes: &[u8], mime_type: &str, config: &ExtractionConfig) -> ExtractedDocument {
    helpers::extract_bytes_document_blocking(bytes, mime_type, config).unwrap()
}

fn pinned_six_column_pdf() -> Vec<u8> {
    assert_eq!(
        hex::encode(Sha256::digest(SIX_COLUMN_PDF)),
        "98fa2e76586868873167a0b5137743f43323c84dca9f843176315aa76fb287b1"
    );
    SIX_COLUMN_PDF.to_vec()
}

/// The `<tr>` of one row, with its cells in order.
fn row_html(cells: &[String], header: bool) -> String {
    let tag = if header { "th" } else { "td" };
    let cells = cells
        .iter()
        .map(|cell| format!("<{tag}>{cell}</{tag}>\n"))
        .collect::<String>();
    format!("<tr>\n{cells}</tr>")
}

fn assert_each_row_once(content: &str, cells: &[Vec<String>]) {
    assert_eq!(content.matches("<tr>").count(), cells.len(), "table rows: {content}");
    for (index, row) in cells.iter().enumerate() {
        assert_eq!(
            content.matches(&row_html(row, index == 0)).count(),
            1,
            "row {index} must appear exactly once in HTML: {content}"
        );
    }
}

#[test]
fn six_column_dot_leader_scan_keeps_every_row_in_html() {
    let result = extract(&pinned_six_column_pdf(), "application/pdf", &config());
    assert!(result.metadata.ocr_used);
    assert_eq!(result.metadata.additional["psm"], "6");
    let [table] = result.tables.as_slice() else {
        panic!("expected one table, got {}", result.tables.len());
    };
    assert_eq!(
        table.cells[0],
        HEADER.iter().map(|cell| cell.to_string()).collect::<Vec<_>>()
    );
    assert_eq!(table.cells.len(), 13);
    assert_eq!(SIX_COLUMN_ROWS.lines().count(), 12);

    assert_each_row_once(&result.content, &table.cells);
    assert_eq!(result.content.matches("<table>").count(), 1);
    assert_eq!(result.content.matches("<th>").count(), HEADER.len());
    assert_eq!(result.content.matches("<td>").count(), 12 * HEADER.len());
    assert!(
        !result.content.contains("<p>"),
        "no table row may be emitted beside the table it also carries: {}",
        result.content
    );

    let [page] = result.pages.as_deref().expect("requested page output") else {
        panic!("expected one source page");
    };
    assert_eq!(page.tables[0].cells, table.cells);
    assert_each_row_once(&page.content, &table.cells);
    assert!(!page.content.contains("<p>"), "{}", page.content);
}

#[test]
fn control_table_without_leaders_keeps_its_rows_unchanged() {
    let result = extract(NO_LEADERS, "image/png", &config());
    let [table] = result.tables.as_slice() else {
        panic!(
            "the control must still be detected as one table, got {}",
            result.tables.len()
        );
    };
    assert_eq!(table.cells.len(), 13);
    assert_eq!(table.cells[0].len(), 4);
    assert!(
        table.cells.iter().flatten().all(|cell| !cell.contains("..")),
        "the control has no leaders: {table:?}"
    );
    assert_each_row_once(&result.content, &table.cells);
    assert!(!result.content.contains("<p>"), "{}", result.content);
}
