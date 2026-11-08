#![cfg(all(
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "pdf",
    feature = "bundle-tessdata-eng"
))]

mod helpers;

use xberg::types::TesseractConfig;
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const PICTURE_20PX: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/20.png");
const PICTURE_20PX_ROWS: &str = include_str!("fixtures/ocr_dot_leader_2503/20.tsv");
const TINTED_SCAN: &[u8] = include_bytes!("fixtures/ocr/dot_leader_scan_2503/scan-200-tinted.pdf");

fn config(psm: Option<i32>) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
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

fn row_lines(content: &str, row: &str) -> usize {
    let fields: Vec<_> = row.split('\t').collect();
    let [label, values @ ..] = fields.as_slice() else {
        panic!("empty fixture row");
    };
    content
        .lines()
        .filter(|line| {
            let flattened = line.replace('|', " ");
            let words: Vec<_> = flattened.split_whitespace().collect();
            let Some(split) = words.len().checked_sub(values.len()) else {
                return false;
            };
            let (head, tail) = words.split_at(split);
            tail == values && head.join(" ").trim_end_matches('.').trim_end() == *label
        })
        .count()
}

#[test]
fn default_route_keeps_every_row_of_a_standalone_leader_table_picture() {
    assert_eq!(PICTURE_20PX_ROWS.lines().count(), 12, "fixture truth must hold 12 rows");
    let result = helpers::extract_bytes_document_blocking(PICTURE_20PX, "image/png", &config(None)).unwrap();
    assert_eq!(recorded_psm(&result), "6");
    for row in PICTURE_20PX_ROWS.lines() {
        assert_eq!(row_lines(&result.content, row), 1, "row {row:?} must be on one line");
    }
}

#[test]
fn explicit_caller_psm_wins_over_the_standalone_picture_default() {
    let result = helpers::extract_bytes_document_blocking(PICTURE_20PX, "image/png", &config(Some(4))).unwrap();
    assert_eq!(recorded_psm(&result), "4");
}

#[test]
fn scanned_pdf_pages_are_not_reread_as_a_single_block() {
    let result = helpers::extract_bytes_document_blocking(TINTED_SCAN, "application/pdf", &config(None)).unwrap();
    assert!(result.metadata.ocr_used);
    assert_ne!(
        result
            .metadata
            .additional
            .get("psm")
            .and_then(serde_json::Value::as_str),
        Some("6")
    );
    assert_eq!(result.tables.len(), 1, "the scan route keeps its own table recovery");
}
