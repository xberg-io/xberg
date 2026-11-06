#![cfg(all(feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use xberg::types::TesseractConfig;
use xberg::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

#[test]
fn psm6_emits_each_complete_source_table_row_once() {
    assert_source_rows_once(
        include_bytes!("fixtures/ocr_dot_leader_2503/20.png"),
        include_str!("fixtures/ocr_dot_leader_2503/20.tsv"),
        false,
    );
}

#[test]
fn psm6_preserves_rows_with_dots_already_in_their_label_cell() {
    assert_source_rows_once(
        include_bytes!("fixtures/ocr_dot_leader_2503/14.png"),
        include_str!("fixtures/ocr_dot_leader_2503/14.tsv"),
        true,
    );
}

fn assert_source_rows_once(bytes: &[u8], expected: &str, known_joined_pool_repairs: bool) {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
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
    };
    let result = helpers::extract_bytes_document_blocking(bytes, "image/png", &config).unwrap();
    assert_eq!(result.tables.len(), 1);
    assert_eq!(result.tables[0].cells.len(), 13);
    let [page] = result.pages.as_deref().expect("requested page output") else {
        panic!("expected one page");
    };
    assert_eq!(expected.lines().count(), 12);
    for row in expected.lines() {
        let fields: Vec<_> = row.split('\t').collect();
        for (view, content) in [("document", &result.content), ("page", &page.content)] {
            let matches = content
                .lines()
                .filter(|line| source_row_matches(line, &fields, known_joined_pool_repairs))
                .count();
            assert_eq!(matches, 1, "{view} should emit {row:?} once");
        }
    }
}

fn source_row_matches(line: &str, row: &[&str], known_joined_pool_repairs: bool) -> bool {
    let [label, values @ ..] = row else {
        return false;
    };
    let flattened = line.replace('|', " ");
    let words: Vec<_> = flattened.split_whitespace().collect();
    let Some(split) = words.len().checked_sub(values.len()) else {
        return false;
    };
    let (actual_label, actual_values) = words.split_at(split);
    if actual_values != values {
        return false;
    }
    let joined = actual_label.join(" ");
    let actual = joined.trim_end_matches('.').trim_end();
    actual == *label || (known_joined_pool_repairs && *label == "Pool repairs" && actual == "Poolrepairs")
}
