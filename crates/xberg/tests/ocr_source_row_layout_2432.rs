#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use xberg::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

#[test]
fn full_scan_preserves_every_numeric_row_in_plain_and_page_output() {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let fixture = include_bytes!("fixtures/ocr_source_row_layout_2432/scan-150-no-leaders.pdf");
    let result = helpers::extract_bytes_document_blocking(fixture, "application/pdf", &config).unwrap();
    let expected = include_str!("fixtures/ocr_source_row_layout_2432/rows.tsv")
        .lines()
        .map(|row| row.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 13);
    let [table] = result.tables.as_slice() else {
        panic!("expected one complete scan table, found {}", result.tables.len());
    };
    assert_eq!(table.cells, expected);
    assert_eq!(table.page_number, 1);
    assert_eq!(
        result.content.trim(),
        expected.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n")
    );
    let pages = result.pages.as_ref().expect("requested page output");
    assert_eq!(pages.len(), 1);
    let page_rows = pages[0]
        .content
        .trim()
        .lines()
        .enumerate()
        .filter(|(index, _)| *index != 1)
        .map(|(_, line)| line.trim_matches('|').split('|').map(str::trim).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(page_rows, expected);
}
