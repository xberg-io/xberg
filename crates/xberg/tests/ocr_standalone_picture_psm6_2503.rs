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

fn assert_rows_are_own_lines(format: OutputFormat) {
    let is_djot = matches!(format, OutputFormat::Djot);
    let mut rendered = config(None);
    rendered.output_format = format;
    let result = helpers::extract_bytes_document_blocking(PICTURE_11PX, "image/png", &rendered).unwrap();
    assert_eq!(recorded_psm(&result), "6");
    let mut lines: Vec<_> = result.content.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        13,
        "header plus 12 rows, one per line:\n{}",
        result.content
    );
    if is_djot {
        let djot_text;
        // The rendered Djot must round-trip with every leader dot as literal text: an
        // ellipsis event means a bare `...` run was left for the reader to turn into `…`.
        let mut paragraphs: Vec<String> = Vec::new();
        let mut open = false;
        for event in jotdown::Parser::new(&result.content) {
            match event {
                jotdown::Event::Ellipsis => panic!(
                    "a leader dot run was not escaped and parses as an ellipsis:\n{}",
                    result.content
                ),
                jotdown::Event::Softbreak | jotdown::Event::Hardbreak => panic!(
                    "expected each Djot row to be a separate paragraph, got an inline break:\n{}",
                    result.content
                ),
                jotdown::Event::Start(jotdown::Container::Paragraph, _) => {
                    open = true;
                    paragraphs.push(String::new());
                }
                jotdown::Event::End(jotdown::Container::Paragraph) => open = false,
                jotdown::Event::Str(text) if open => paragraphs.last_mut().unwrap().push_str(&text),
                _ => {}
            }
        }
        djot_text = paragraphs.join("\n");
        lines = djot_text.lines().collect();
        assert_eq!(lines.len(), 13, "header plus 12 parsed Djot paragraphs:\n{djot_text}");

        // Djot escapes every leader dot, so every leader run must round-trip whole:
        // each of the 12 rows must sit on exactly one line with a byte-identical
        // label, a non-empty leader and its values intact. The OCR can misread a
        // value comma as a period (the pipeline's glyph-height resample, ~4.2x for
        // this image, is where it loses one), and that is an accuracy loss, not a
        // leader loss, so the tail comparison reads ',' as '.' on both sides; the
        // tolerance is at most one such row, whichever it is, and the normalized
        // comparison itself pins that line to the row: byte-identical label, tail
        // alone differing. The OCR accuracy loss never lowers the bar: all 12
        // leaders are required, not 11 (GH#3009).
        let mut next = 0;
        let mut normalized_only: Option<&str> = None;
        for row in PICTURE_11PX_ROWS.lines() {
            let hits: Vec<(usize, bool)> = lines
                .iter()
                .enumerate()
                .filter_map(|(at, line)| row_hit(line, row, true).map(|leader| (at, leader)))
                .collect();
            assert_eq!(
                hits.len(),
                1,
                "row {row:?} must be on exactly one line:\n{}",
                result.content
            );
            let (at, leader) = hits[0];
            assert!(leader, "row {row:?} lost its leader:\n{}", result.content);
            assert!(at >= next, "row {row:?} is out of order:\n{}", result.content);
            next = at;
            // A hit under the normalized comparison but not the exact one is a row
            // whose comma the OCR misread: `row_hit(.., true)` already pinned that
            // line to the row (byte-identical label, tail alone differing), so the
            // only open question is how many rows needed the tolerance — at most one.
            if row_hit(lines[at], row, false) != Some(true) {
                assert!(
                    normalized_only.is_none(),
                    "at most one row may need the comma normalization, not {row:?} after {normalized_only:?}:\n{}",
                    result.content
                );
                normalized_only = Some(row);
            }
        }
    } else {
        // The plain and Markdown variants tolerate one OCR-quality row.
        let mut kept = 0;
        let mut next = 0;
        for row in PICTURE_11PX_ROWS.lines() {
            let Some(at) = lines.iter().position(|line| row_hit(line, row, false).is_some()) else {
                continue;
            };
            assert!(at >= next, "row {row:?} is out of order:\n{}", result.content);
            kept += 1;
            next = at;
        }
        assert!(kept >= 11, "{kept} of 12 rows kept whole:\n{}", result.content);
    }
}

/// Whether `line` is the fixture row: the label byte-identical, then a leader run, then the
/// values. With `normalize_tail` the values compare with ',' read as '.' on both sides: the
/// OCR can misread a comma as a period, and that is an accuracy loss, not a leader loss.
/// Returns whether a non-empty run of dots separates the label from the values.
fn row_hit(line: &str, row: &str, normalize_tail: bool) -> Option<bool> {
    let fields: Vec<_> = row.split('\t').collect();
    let [label, values @ ..] = fields.as_slice() else {
        return None;
    };
    let flattened = line.replace('|', " ");
    let words: Vec<_> = flattened.split_whitespace().collect();
    let split = words.len().checked_sub(values.len())?;
    let (head, tail) = words.split_at(split);
    let tail_eq = if normalize_tail {
        tail.len() == values.len()
            && tail
                .iter()
                .zip(values.iter())
                .all(|(word, value)| word.replace(',', ".") == value.replace(',', "."))
    } else {
        tail == values
    };
    if !tail_eq {
        return None;
    }
    let joined = head.join(" ");
    let trimmed = joined.trim_end_matches('.').trim_end();
    (trimmed == *label).then_some(trimmed.len() < joined.len())
}

#[test]
fn markdown_keeps_every_row_of_an_adopted_leader_picture_on_its_own_line() {
    assert_rows_are_own_lines(OutputFormat::Markdown);
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

#[test]
fn djot_keeps_every_row_of_an_adopted_11px_leader_picture_on_its_own_line() {
    assert_rows_are_own_lines(OutputFormat::Djot);
}
