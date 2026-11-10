#![cfg(all(
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "pdf",
    feature = "bundle-tessdata-eng"
))]

mod helpers;

use xberg::types::TesseractConfig;
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const PICTURE_11PX: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/11.png");
const PICTURE_11PX_ROWS: &str = include_str!("fixtures/ocr_dot_leader_2503/11.tsv");
const TABLE_OF_CONTENTS: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/toc.png");
const TWO_COLUMN_PROSE: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/two_column.png");
const PICTURE_14PX: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/14.png");
const PICTURE_14PX_ROWS: &str = include_str!("fixtures/ocr_dot_leader_2503/14.tsv");
const CLEAN_TABLE: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/clean_table.png");
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

#[test]
fn default_route_rereads_a_standalone_leader_picture_whose_block_read_has_no_table() {
    let result = helpers::extract_bytes_document_blocking(PICTURE_11PX, "image/png", &config(None)).unwrap();
    assert_eq!(recorded_psm(&result), "6");
    let kept = PICTURE_11PX_ROWS
        .lines()
        .filter(|row| row_lines(&result.content, row) == 1)
        .count();
    assert!(kept >= 11, "{kept} of 12 rows kept whole:\n{}", result.content);
}

#[test]
fn explicit_caller_psm_wins_on_a_standalone_leader_picture_that_would_be_reread() {
    for psm in [3, 11] {
        let result = helpers::extract_bytes_document_blocking(PICTURE_11PX, "image/png", &config(Some(psm))).unwrap();
        assert_eq!(recorded_psm(&result), psm.to_string());
    }
}

#[test]
fn a_standalone_prose_picture_keeps_the_default_route() {
    let result = helpers::extract_bytes_document_blocking(TWO_COLUMN_PROSE, "image/png", &config(None)).unwrap();
    assert_eq!(recorded_psm(&result), "11");
}

#[test]
fn a_dot_leader_table_of_contents_is_not_read_again_as_a_table() {
    let result = helpers::extract_bytes_document_blocking(TABLE_OF_CONTENTS, "image/png", &config(None)).unwrap();
    assert_eq!(recorded_psm(&result), "11");
}

#[test]
fn markdown_keeps_every_row_of_an_adopted_leader_picture_on_its_own_line() {
    let mut markdown = config(None);
    markdown.output_format = OutputFormat::Markdown;
    let result = helpers::extract_bytes_document_blocking(PICTURE_11PX, "image/png", &markdown).unwrap();
    assert_eq!(recorded_psm(&result), "6");
    let lines: Vec<_> = result.content.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        13,
        "header plus 12 rows, one per line:\n{}",
        result.content
    );
    let mut kept = 0;
    let mut next = 0;
    for row in PICTURE_11PX_ROWS.lines() {
        let Some(at) = lines.iter().position(|line| row_lines(line, row) == 1) else {
            continue;
        };
        assert!(at >= next, "row {row:?} is out of order:\n{}", result.content);
        kept += 1;
        next = at;
    }
    assert!(kept >= 11, "{kept} of 12 rows kept whole:\n{}", result.content);
}

#[test]
fn plain_keeps_the_rows_of_an_adopted_leader_picture_on_consecutive_lines() {
    let result = helpers::extract_bytes_document_blocking(PICTURE_11PX, "image/png", &config(None)).unwrap();
    assert_eq!(recorded_psm(&result), "6");
    assert!(
        !result.content.contains("\n\n"),
        "no blank line between rows:\n{}",
        result.content
    );
}

/// Rows of the 14 px picture that sit whole on one line; the block read joins "Pool repairs" into
/// "Poolrepairs", which is asserted separately and counted as kept.
fn kept_14px_rows(content: &str) -> usize {
    assert!(content.contains("Poolrepairs"), "the known join is gone:\n{content}");
    let normalized = content.replace("Poolrepairs", "Pool repairs");
    PICTURE_14PX_ROWS
        .lines()
        .filter(|row| row_lines(&normalized, row) == 1)
        .count()
}

#[test]
fn default_route_rereads_a_14px_picture_whose_first_read_holds_a_garbled_table() {
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let mut config = config(None);
        config.output_format = format.clone();
        let result = helpers::extract_bytes_document_blocking(PICTURE_14PX, "image/png", &config).unwrap();
        assert_eq!(recorded_psm(&result), "6", "{format:?}");
        let kept = kept_14px_rows(&result.content);
        assert!(
            kept >= 11,
            "{format:?}: {kept} of 12 rows kept whole:\n{}",
            result.content
        );
    }
}

#[test]
fn explicit_caller_psm_wins_on_a_14px_picture_with_a_garbled_first_read() {
    let result = helpers::extract_bytes_document_blocking(PICTURE_14PX, "image/png", &config(Some(11))).unwrap();
    assert_eq!(recorded_psm(&result), "11");
}

#[test]
fn a_picture_whose_first_read_holds_a_correct_table_keeps_it() {
    let result = helpers::extract_bytes_document_blocking(CLEAN_TABLE, "image/png", &config(None)).unwrap();
    assert_eq!(
        result.tables.len(),
        1,
        "the first read must hold a table:\n{}",
        result.content
    );
    assert_eq!(recorded_psm(&result), "11");
}

fn assert_djot_rows_are_paragraphs(picture: &[u8], rows: &str) {
    let mut djot = config(None);
    djot.output_format = OutputFormat::Djot;
    let result = helpers::extract_bytes_document_blocking(picture, "image/png", &djot).unwrap();
    assert_eq!(recorded_psm(&result), "6");
    let lines: Vec<_> = result.content.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        13,
        "header plus 12 rows, one per paragraph:\n{}",
        result.content
    );
    let mut kept = 0;
    let mut next = 0;
    for row in rows.lines() {
        let Some(at) = lines.iter().position(|line| row_lines(line, row) == 1) else {
            continue;
        };
        assert!(at >= next, "row {row:?} is out of order:\n{}", result.content);
        kept += 1;
        next = at;
    }
    assert!(kept >= 11, "{kept} of 12 rows kept whole:\n{}", result.content);
}

#[test]
fn djot_keeps_every_row_of_an_adopted_11px_leader_picture_on_its_own_line() {
    assert_djot_rows_are_paragraphs(PICTURE_11PX, PICTURE_11PX_ROWS);
}
