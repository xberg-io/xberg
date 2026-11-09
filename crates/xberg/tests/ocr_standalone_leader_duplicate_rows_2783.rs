#![cfg(all(
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "pdf",
    feature = "bundle-tessdata-eng"
))]

mod helpers;

use xberg::types::TesseractConfig;
use xberg::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

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

fn occurrences(content: &str, value: &str) -> usize {
    content
        .split(|c: char| c.is_whitespace() || c == '|')
        .filter(|word| *word == value)
        .count()
}

fn assert_no_value_repeated(picture: &[u8], rows: &str, psm: Option<i32>, format: OutputFormat) {
    let result = helpers::extract_bytes_document_blocking(picture, "image/png", &config(psm, format.clone())).unwrap();
    let values: Vec<&str> = rows.lines().flat_map(|row| row.split('\t').skip(1)).collect();
    for row in rows.lines() {
        for value in row.split('\t').skip(1) {
            if values.iter().filter(|other| **other == value).count() > 1 {
                continue;
            }
            let count = occurrences(&result.content, value);
            assert!(
                count <= 1,
                "value {value:?} of row {row:?} appears {count} times (psm {psm:?}, {format:?}):\n{}",
                result.content
            );
        }
    }
}

fn assert_each_row_once_everywhere(picture: &[u8], rows: &str) {
    for psm in [None, Some(6)] {
        for format in [OutputFormat::Plain, OutputFormat::Markdown] {
            assert_no_value_repeated(picture, rows, psm, format);
        }
    }
}

#[test]
fn a_14px_leader_table_picture_prints_each_row_once() {
    assert_each_row_once_everywhere(PICTURE_14PX_SEED3, PICTURE_14PX_SEED3_ROWS);
}

#[test]
fn an_18px_leader_table_picture_prints_each_row_once() {
    assert_each_row_once_everywhere(PICTURE_18PX_SEED9, PICTURE_18PX_SEED9_ROWS);
}
