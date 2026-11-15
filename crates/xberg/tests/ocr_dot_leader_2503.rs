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

#[test]
fn psm3_publishes_each_source_row_once_with_its_detected_leaders() {
    const BYTES: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/20.png");
    const EXPECTED: &str = include_str!("fixtures/ocr_dot_leader_2503/20.tsv");
    const LEADER_ROWS: [&str; 2] = [
        "Signal timing .......... 64 389,542 143.92",
        "Field lights ............ 90 752,441 799.07",
    ];

    let mut config = source_row_config(3);
    let plain = helpers::extract_bytes_document_blocking(BYTES, "image/png", &config).unwrap();
    let [page] = plain.pages.as_deref().expect("requested page output") else {
        panic!("expected one page");
    };
    assert_eq!(plain.metadata.additional["psm"], "3");
    assert_eq!(EXPECTED.lines().count(), 12);
    assert_rows_once(&plain.content, EXPECTED, "document", false);
    assert_rows_once(&page.content, EXPECTED, "page", false);
    for row in LEADER_ROWS {
        let normalized = |line: &str| line.replace('|', " ").split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(plain.content.lines().filter(|line| normalized(line) == row).count(), 1);
        assert_eq!(page.content.lines().filter(|line| normalized(line) == row).count(), 1);
    }

    config.output_format = OutputFormat::Html;
    let html = helpers::extract_bytes_document_blocking(BYTES, "image/png", &config).unwrap();
    assert_eq!(html.content.matches("<tr>").count(), 13);
    for (amount, leader) in [("389,542", ".........."), ("752,441", "............")] {
        assert_eq!(html.content.matches(amount).count(), 1, "HTML duplicated {amount}");
        let [table] = html.tables.as_slice() else {
            panic!("expected one table");
        };
        assert!(
            table
                .cells
                .iter()
                .any(|row| row.iter().any(|cell| cell == amount) && row.iter().any(|cell| cell.ends_with(leader))),
            "HTML table lost the detected leader for {amount}"
        );
    }
}

fn source_row_config(psm: i32) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            tesseract_config: Some(TesseractConfig {
                psm: Some(psm),
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

fn assert_source_rows_once(bytes: &[u8], expected: &str, known_joined_pool_repairs: bool) {
    let config = source_row_config(6);
    let result = helpers::extract_bytes_document_blocking(bytes, "image/png", &config).unwrap();
    assert_eq!(result.tables.len(), 1);
    assert_eq!(result.tables[0].cells.len(), 13);
    let [page] = result.pages.as_deref().expect("requested page output") else {
        panic!("expected one page");
    };
    assert_eq!(expected.lines().count(), 12);
    assert_rows_once(&result.content, expected, "document", known_joined_pool_repairs);
    assert_rows_once(&page.content, expected, "page", known_joined_pool_repairs);
}

fn assert_rows_once(content: &str, expected: &str, view: &str, known_joined_pool_repairs: bool) {
    for row in expected.lines() {
        let fields: Vec<_> = row.split('\t').collect();
        let matches = content
            .lines()
            .filter(|line| source_row_matches(line, &fields, known_joined_pool_repairs))
            .count();
        assert_eq!(matches, 1, "{view} should emit {row:?} once");
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
