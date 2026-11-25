//! Plain-text and Markdown output of a native PDF read in one reading order.

#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use std::collections::HashMap;
use xberg::core::config::{ExtractionConfig, OutputFormat};

const RULED_TABLE_PDF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test_documents/pdf/tiny.pdf");

const LEFT_COLUMN: [&str; 8] = [
    "Rivers carry fresh water from the mountains",
    "down through the valleys toward the sea",
    "where farmers draw it into narrow canals",
    "that feed the orchards along each bank",
    "during the long and very dry summers",
    "before the autumn rains return again",
    "and the ground recovers its lost moisture",
    "for another season of careful planting",
];

const RIGHT_COLUMN: [&str; 8] = [
    "Harbours along the coast receive the ships",
    "that bring timber and grain from abroad",
    "while fishing boats leave before the dawn",
    "and return with their nets full of catch",
    "which merchants sell in the busy markets",
    "to families who arrive from nearby towns",
    "carrying baskets woven from dried reeds",
    "then walk home along the dusty roads",
];

const TABLE_ROWS: [[&str; 3]; 4] = [
    ["Name", "Celsius", "Fahrenheit"],
    ["Water Freezing Point", "0", "32"],
    ["Water Boiling Point", "100", "212"],
    ["Body Temperature", "37", "98.6"],
];

fn text_run(x: f32, y: f32, text: &str) -> String {
    format!("BT /F1 11 Tf 1 0 0 1 {x:.1} {y:.1} Tm ({text}) Tj ET\n")
}

/// One page: two balanced prose columns above a three-column table, one span per word in
/// the table as a word-per-`Tj` producer writes it.
fn two_column_and_table_pdf() -> Vec<u8> {
    let mut content = String::new();
    for (row, (left, right)) in LEFT_COLUMN.iter().zip(RIGHT_COLUMN).enumerate() {
        let y = 720.0 - row as f32 * 14.0;
        content.push_str(&text_run(60.0, y, left));
        content.push_str(&text_run(320.0, y, right));
    }
    content.push_str(&text_run(72.0, 500.0, "Table 1. Selected Numbers"));
    for (row, cells) in TABLE_ROWS.iter().enumerate() {
        let y = 470.0 - row as f32 * 23.0;
        for (cell, x) in cells.iter().zip([77.0_f32, 233.0, 389.0]) {
            let mut x = x;
            for word in cell.split_whitespace() {
                content.push_str(&text_run(x, y, word));
                x += word.len() as f32 * 6.2 + 3.1;
            }
        }
    }

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_owned(),
        format!("<< /Length {} >>\nstream\n{content}endstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn extract(pdf: &[u8], output_format: OutputFormat) -> String {
    let config = ExtractionConfig {
        output_format,
        disable_ocr: true,
        use_cache: false,
        ..ExtractionConfig::default()
    };
    extract_bytes_document_blocking(pdf, "application/pdf", &config)
        .expect("native PDF extraction must succeed")
        .content
}

/// Lowercased words with markup and punctuation stripped, so a Markdown table row and the
/// same row in plain text compare equal.
fn words(text: &str) -> Vec<String> {
    text.split(|character: char| character.is_whitespace() || character == '|')
        .map(|word| {
            word.chars()
                .filter(|character| character.is_alphanumeric() || *character == '.')
                .collect::<String>()
                .trim_matches('.')
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

/// Words that occur exactly once in both texts, in `text`'s order. Comparing these between
/// two outputs compares their reading order without depending on how each output format
/// joins lines or renders a table.
fn shared_unique_words(text: &[String], other: &[String]) -> Vec<String> {
    fn count(words: &[String]) -> HashMap<&str, usize> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for word in words {
            *counts.entry(word.as_str()).or_default() += 1;
        }
        counts
    }
    let (text_counts, other_counts) = (count(text), count(other));
    text.iter()
        .filter(|word| text_counts[word.as_str()] == 1 && other_counts.get(word.as_str()) == Some(&1))
        .cloned()
        .collect()
}

fn assert_plain_and_markdown_share_order(pdf: &[u8]) -> String {
    let plain = extract(pdf, OutputFormat::Plain);
    let markdown = extract(pdf, OutputFormat::Markdown);
    let (plain_words, markdown_words) = (words(&plain), words(&markdown));
    let plain_order = shared_unique_words(&plain_words, &markdown_words);
    let markdown_order = shared_unique_words(&markdown_words, &plain_words);

    assert!(
        plain_order.len() >= 20,
        "too few shared words to compare order ({}):\nplain: {plain}\nmarkdown: {markdown}",
        plain_order.len()
    );
    assert_eq!(
        plain_order, markdown_order,
        "plain and markdown must read the page in one order\nplain: {plain}\nmarkdown: {markdown}"
    );
    plain
}

#[test]
fn plain_text_reads_a_ruled_table_row_by_row_like_markdown() {
    let pdf = std::fs::read(RULED_TABLE_PDF).expect("tiny.pdf corpus fixture must exist");
    let plain = assert_plain_and_markdown_share_order(&pdf);

    let expected_rows = TABLE_ROWS.iter().flatten().copied().collect::<Vec<_>>().join(" ");
    let normalized = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized.contains(&expected_rows),
        "plain text must read the table row by row, expected {expected_rows:?} in:\n{normalized}"
    );
}

#[test]
fn plain_and_markdown_share_order_on_a_two_column_page_with_a_table() {
    let plain = assert_plain_and_markdown_share_order(&two_column_and_table_pdf());

    let normalized = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    let left_end = normalized
        .find("careful planting")
        .unwrap_or_else(|| panic!("left column missing:\n{normalized}"));
    let right_start = normalized
        .find("Harbours along the coast")
        .unwrap_or_else(|| panic!("right column missing:\n{normalized}"));
    assert!(
        left_end < right_start,
        "the left column must be read before the right column:\n{normalized}"
    );
}
