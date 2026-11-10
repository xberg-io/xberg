#![cfg(all(
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "pdf",
    feature = "bundle-tessdata-eng"
))]

mod helpers;

use xberg::types::TesseractConfig;
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const PICTURE_14PX_SEED3: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/14px_seed3.png");
const PICTURE_14PX_SEED3_ROWS: &str = include_str!("fixtures/ocr_dot_leader_2503/14px_seed3.tsv");
const PICTURE_18PX_SEED9: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/18px_seed9.png");
const PICTURE_18PX_SEED9_ROWS: &str = include_str!("fixtures/ocr_dot_leader_2503/18px_seed9.tsv");

fn config(psm: Option<i32>, output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format,
        ocr: Some(OcrConfig {
            tesseract_config: psm.map(|psm| TesseractConfig {
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

fn recorded_psm(result: &ExtractedDocument) -> String {
    result
        .metadata
        .additional
        .get("psm")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("OCR must record its page segmentation mode"))
        .to_string()
}

fn occurrences(content: &str, value: &str) -> usize {
    content
        .split(|c: char| c.is_whitespace() || c == '|')
        .filter(|word| *word == value)
        .count()
}

/// Reads the picture and returns what is wrong with the output; empty when every row printed once.
/// Tesseract misreads a few cells ("§=©777,980"), so presence is a floor (9 of 12 labels, 18 of 24
/// amounts, each exactly once) while no label or amount may appear more than once.
fn problems(picture: &[u8], rows: &str, psm: Option<i32>, format: OutputFormat) -> Vec<String> {
    let result = helpers::extract_bytes_document_blocking(picture, "image/png", &config(psm, format.clone())).unwrap();
    let mut found = Vec::new();
    // The default route reads sparse text first and re-reads the table picture as one block.
    if recorded_psm(&result) != "6" {
        found.push(format!("read with psm {} instead of 6", recorded_psm(&result)));
    }
    let labels: Vec<&str> = rows.lines().map(|row| row.split('\t').next().unwrap()).collect();
    let amounts: Vec<&str> = rows.lines().flat_map(|row| row.split('\t').skip(2)).collect();
    let mut labels_once = 0;
    for label in &labels {
        match result.content.matches(label).count() {
            0 => {}
            1 => labels_once += 1,
            n => found.push(format!("label {label:?} appears {n} times")),
        }
    }
    let mut amounts_once = 0;
    for amount in &amounts {
        match occurrences(&result.content, amount) {
            0 => {}
            1 => amounts_once += 1,
            n => found.push(format!("amount {amount:?} appears {n} times")),
        }
    }
    if labels_once < 9 {
        found.push(format!("only {labels_once} of {} labels appear once", labels.len()));
    }
    if amounts_once < 18 {
        found.push(format!("only {amounts_once} of {} amounts appear once", amounts.len()));
    }
    found
}

fn assert_each_row_once_everywhere(picture: &[u8], rows: &str) {
    let mut failures = Vec::new();
    for psm in [None, Some(6)] {
        for format in [OutputFormat::Plain, OutputFormat::Markdown] {
            for problem in problems(picture, rows, psm, format.clone()) {
                failures.push(format!("psm {psm:?} {format:?}: {problem}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_14px_leader_table_picture_prints_each_row_once() {
    assert_each_row_once_everywhere(PICTURE_14PX_SEED3, PICTURE_14PX_SEED3_ROWS);
}

#[test]
fn an_18px_leader_table_picture_prints_each_row_once() {
    assert_each_row_once_everywhere(PICTURE_18PX_SEED9, PICTURE_18PX_SEED9_ROWS);
}
