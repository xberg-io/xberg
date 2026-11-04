#![cfg(feature = "ocr")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use std::collections::BTreeMap;
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use xberg::types::{ExtractedDocument, ExtractionMethod, TesseractConfig};

// The no-DPI PNG uses the bundled DejaVu Sans at 14 px. Each source token is unique. ~keep
const IMAGE: &[u8] = include_bytes!("fixtures/ocr/sparse_table_page_rows.png");
const ROWS: [[&str; 4]; 12] = [
    ["Rent", "14", "628,382", "(4,476)"],
    ["Transit", "28", "471,293", "(8,319)"],
    ["Supplies", "35", "809,641", "(3,762)"],
    ["Payroll", "46", "275,908", "(7,153)"],
    ["Repairs", "57", "914,725", "(2,684)"],
    ["Mileage", "68", "583,260", "(5,297)"],
    ["Storage", "79", "362,819", "(6,851)"],
    ["Telephone", "82", "736,415", "(9,624)"],
    ["Licenses", "93", "195,734", "(1,963)"],
    ["Courier", "24", "847,592", "(4,732)"],
    ["Training", "36", "529,163", "(8,257)"],
    ["Insurance", "48", "614,857", "(3,918)"],
];

fn extract(bytes: &[u8], output_format: OutputFormat, psm: Option<i32>) -> ExtractedDocument {
    extract_bytes_document_blocking(
        bytes,
        "image/png",
        &ExtractionConfig {
            use_cache: false,
            output_format,
            ocr: Some(OcrConfig {
                tesseract_config: psm.map(|psm| TesseractConfig {
                    psm: Some(psm),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .expect("real Tesseract OCR of the synthetic table")
}

fn assert_table(doc: &ExtractedDocument, expected_psm: &str) {
    assert_eq!(doc.extraction_method, Some(ExtractionMethod::Ocr));
    assert_eq!(
        doc.metadata.additional.get("psm").and_then(serde_json::Value::as_str),
        Some(expected_psm)
    );
    assert_eq!(doc.tables.len(), 1);
    let cells: Vec<Vec<&str>> = doc.tables[0]
        .cells
        .iter()
        .map(|row| row.iter().map(String::as_str).collect())
        .collect();
    assert_eq!(cells, ROWS);
}

fn page_text(doc: &ExtractedDocument) -> &str {
    let pages = doc.pages.as_deref().expect("one standalone OCR page");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    &pages[0].content
}

fn counts<'a>(words: impl Iterator<Item = &'a str>) -> BTreeMap<&'a str, usize> {
    let mut counts = BTreeMap::new();
    for word in words {
        *counts.entry(word).or_default() += 1;
    }
    counts
}

fn assert_table_words(text: &str) {
    assert_eq!(counts(text.split_whitespace()), counts(ROWS.into_iter().flatten()));
}

fn assert_plain_rows(text: &str) {
    let rows = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(rows, ROWS, "each table row must stay on one line");
}

fn assert_pipe_rows(text: &str) {
    let rows = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix('|')?.strip_suffix('|'))
        .map(|line| line.split('|').map(str::trim).collect::<Vec<_>>())
        .filter(|row| !row.iter().all(|cell| *cell == "---"))
        .collect::<Vec<_>>();
    assert_eq!(rows, ROWS, "one formatted line per row");
}

#[test]
fn implicit_sparse_plain_page_uses_the_same_table_rows_as_the_document() {
    let doc = extract(IMAGE, OutputFormat::Plain, None);
    assert_table(&doc, "11");
    assert_plain_rows(&doc.content);
    assert_table_words(page_text(&doc));
    assert_plain_rows(page_text(&doc));
}

#[test]
fn implicit_sparse_djot_keeps_plain_rows_in_the_page_snapshot() {
    let doc = extract(IMAGE, OutputFormat::Djot, None);
    assert_table(&doc, "11");
    assert_pipe_rows(&doc.content);
    assert_table_words(page_text(&doc));
    assert_plain_rows(page_text(&doc));
}

#[test]
fn explicit_sparse_psm_keeps_the_original_page_transcript() {
    let doc = extract(IMAGE, OutputFormat::Plain, Some(11));
    assert_table(&doc, "11");
    assert_plain_rows(&doc.content);
    assert_table_words(page_text(&doc));
    assert_eq!(
        page_text(&doc).lines().filter(|line| !line.trim().is_empty()).count(),
        48
    );
}

#[test]
fn explicit_block_psm_keeps_the_already_grouped_page_rows() {
    let doc = extract(IMAGE, OutputFormat::Plain, Some(6));
    assert_table(&doc, "6");
    assert_plain_rows(page_text(&doc));
}

#[test]
fn markdown_keeps_the_existing_pipe_table_in_both_outputs() {
    let doc = extract(IMAGE, OutputFormat::Markdown, None);
    assert_table(&doc, "11");
    assert_pipe_rows(&doc.content);
    assert_pipe_rows(page_text(&doc));
}

fn with_repeated_caption() -> Vec<u8> {
    use image::ImageEncoder;
    let table = image::load_from_memory(IMAGE).expect("fixture PNG").to_rgb8();
    let mut page = image::RgbImage::from_pixel(table.width(), 575, image::Rgb([255; 3]));
    image::imageops::replace(&mut page, &table, 0, 0);
    let label = image::imageops::crop_imm(&table, 23, 13, 38, 18).to_image();
    let value = image::imageops::crop_imm(&table, 216, 13, 24, 18).to_image();
    image::imageops::replace(&mut page, &label, 25, 520);
    image::imageops::replace(&mut page, &value, 69, 520);
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            page.as_raw(),
            page.width(),
            page.height(),
            image::ExtendedColorType::Rgb8,
        )
        .expect("synthetic mixed-content PNG");
    bytes
}

#[test]
fn accepted_table_does_not_consume_repeated_words_in_the_surrounding_text() {
    let bytes = with_repeated_caption();
    let baseline = extract(&bytes, OutputFormat::Plain, Some(11));
    let implicit = extract(&bytes, OutputFormat::Plain, None);
    for doc in [&baseline, &implicit] {
        assert_table(doc, "11");
        let words = counts(page_text(doc).split_whitespace());
        assert_eq!(words.get("Rent"), Some(&2));
        assert_eq!(words.get("14"), Some(&2));
        assert_eq!(words.values().sum::<usize>(), 50);
    }
    assert_eq!(page_text(&implicit), page_text(&baseline));
}
