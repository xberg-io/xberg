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
//! table row. The claim now runs on the document the proof returns, as a rows-only claim that
//! ignores leader artifacts and never promotes the table to a bounding-box claim.

mod helpers;

use sha2::{Digest, Sha256};
use xberg::types::TesseractConfig;
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const SIX_COLUMN_PDF: &[u8] = include_bytes!("fixtures/ocr_six_column_rows_2503/six-column-150dpi.pdf");
const SIX_COLUMN_PICTURE: &[u8] = include_bytes!("fixtures/ocr_six_column_rows_2503/six-column-150dpi.png");
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

/// The fixture generator's rows, independent of OCR.
fn source_rows() -> Vec<Vec<&'static str>> {
    let rows: Vec<_> = SIX_COLUMN_ROWS.lines().map(|line| line.split('\t').collect()).collect();
    assert_eq!(rows.len(), 12, "fixture ground truth");
    rows
}

/// Whether `line` carries the source row: its label (trailing leader dots trimmed) then its values
/// in order, as `ocr_dot_leader_2503.rs` compares them.
fn text_row_matches(line: &str, source: &[&str]) -> bool {
    let [label, values @ ..] = source else {
        return false;
    };
    let words: Vec<_> = line.split_whitespace().collect();
    let Some(split) = words.len().checked_sub(values.len()) else {
        return false;
    };
    let (actual_label, actual_values) = words.split_at(split);
    let end = actual_label
        .iter()
        .rposition(|word| {
            !word
                .chars()
                .all(|character| matches!(character, '.' | '\u{b7}' | '_' | '\u{2026}' | '='))
        })
        .map_or(0, |at| at + 1);
    let actual_label = actual_label[..end].join(" ");
    actual_values == values && actual_label.trim_end_matches('.').trim_end() == *label
}

/// The row cells of every `<tr>` in `content`, in document order.
fn html_rows(content: &str) -> Vec<Vec<String>> {
    content
        .split("<tr>")
        .skip(1)
        .filter_map(|rest| {
            let row = rest.split("</tr>").next()?;
            let mut cells = Vec::new();
            for tag in ["th", "td"] {
                let open = format!("<{tag}>");
                let close = format!("</{tag}>");
                let mut remaining = row;
                while let Some(start) = remaining.find(&open) {
                    let after = &remaining[start + open.len()..];
                    let end = after.find(&close)?;
                    cells.push(after[..end].to_string());
                    remaining = &after[end + close.len()..];
                }
            }
            Some(cells)
        })
        .collect()
}

/// Every source row exactly once, and no data row the ground truth does not name.
fn assert_source_rows_once(content: &str, view: &str) {
    let rows = html_rows(content);
    assert_eq!(rows.len(), 13, "{view}: header plus twelve source rows:\n{content}");
    assert_eq!(
        rows[0],
        HEADER.iter().map(|cell| cell.to_string()).collect::<Vec<_>>(),
        "{view} header"
    );
    for source in source_rows() {
        let matches = rows[1..]
            .iter()
            .filter(|cells| text_row_matches(&cells.join(" "), &source))
            .count();
        assert_eq!(
            matches, 1,
            "{view}: source row {source:?} must appear exactly once:\n{content}"
        );
    }
    for cells in &rows[1..] {
        assert!(
            source_rows()
                .iter()
                .any(|source| text_row_matches(&cells.join(" "), source)),
            "{view}: data row the source does not name: {cells:?}"
        );
    }
    assert!(!content.contains("<p>"), "{view}: text beside the table:\n{content}");
}

/// Every source row exactly once as a transcript line, for routes whose page text is plain.
fn assert_source_rows_once_as_text(content: &str, view: &str) {
    let lines: Vec<_> = content.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(lines.len(), 13, "{view}: header plus twelve rows:\n{content}");
    for source in source_rows() {
        let matches = lines.iter().filter(|line| text_row_matches(line, &source)).count();
        assert_eq!(
            matches, 1,
            "{view}: source row {source:?} must appear exactly once:\n{content}"
        );
    }
}

fn assert_table_matches_source(result: &ExtractedDocument) {
    let [table] = result.tables.as_slice() else {
        panic!("expected one table, got {}", result.tables.len());
    };
    assert_eq!(table.cells.len(), 13);
    assert_eq!(
        table.cells[0],
        HEADER.iter().map(|cell| cell.to_string()).collect::<Vec<_>>()
    );
    for source in source_rows() {
        let matches = table
            .cells
            .iter()
            .filter(|cells| text_row_matches(&cells.join(" "), &source))
            .count();
        assert_eq!(
            matches, 1,
            "table cell row {source:?} must appear exactly once: {table:?}"
        );
    }
}

#[test]
fn six_column_dot_leader_scan_keeps_every_row_in_html() {
    let result = extract(&pinned_six_column_pdf(), "application/pdf", &config());
    assert!(result.metadata.ocr_used);
    assert_eq!(result.metadata.additional["psm"], "6");
    assert_table_matches_source(&result);

    assert_source_rows_once(&result.content, "document HTML");
    assert_eq!(result.content.matches("<table>").count(), 1);
    assert_eq!(result.content.matches("<th>").count(), HEADER.len());
    assert_eq!(result.content.matches("<td>").count(), 12 * HEADER.len());

    let pages = result.pages.as_deref().expect("requested page output");
    assert_eq!(pages.len(), 1);
    let [table] = result.tables.as_slice() else {
        unreachable!()
    };
    for page in pages {
        assert_eq!(page.tables[0].cells, table.cells);
        assert_source_rows_once(&page.content, &format!("page {} HTML", page.page_number));
    }
}

#[test]
fn six_column_dot_leader_picture_keeps_every_row_in_html() {
    let result = extract(SIX_COLUMN_PICTURE, "image/png", &config());
    assert!(result.metadata.ocr_used);
    assert_table_matches_source(&result);

    assert_source_rows_once(&result.content, "picture document HTML");
    let [page] = result.pages.as_deref().expect("requested page output") else {
        panic!("expected one source page");
    };
    assert_source_rows_once_as_text(&page.content, "picture page text");
}

#[test]
fn no_leader_picture_keeps_its_rows_in_html() {
    let result = extract(NO_LEADERS, "image/png", &config());
    let [table] = result.tables.as_slice() else {
        panic!(
            "the no-leader picture must still be detected as one table, got {}",
            result.tables.len()
        );
    };
    assert_eq!(table.cells.len(), 13);
    assert_eq!(table.cells[0].len(), 4);
    assert!(
        table.cells.iter().flatten().all(|cell| !cell.contains("..")),
        "the no-leader picture has no leader tokens: {table:?}"
    );
    assert_eq!(result.content.matches("<tr>").count(), 13);
    assert!(!result.content.contains("<p>"), "{}", result.content);
}
