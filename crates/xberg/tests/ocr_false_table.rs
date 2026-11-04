//! OCR must not build a table from labels that only sit in scattered positions, and a real
//! table must stay a table. The pages are invented: `make_ocr_layout_fixtures.py` draws them.
#![cfg(feature = "ocr")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use xberg::types::TesseractConfig;

fn config(format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        output_format: format,
        force_ocr: true,
        use_cache: false,
        ..Default::default()
    }
}

fn read_page(name: &str, format: OutputFormat) -> xberg::types::ExtractedDocument {
    let path = format!("{}/tests/data/ocr_layout/{name}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("fixture {path}: {error}"));
    extract_bytes_document_blocking(&bytes, "image/png", &config(format)).expect("OCR extraction")
}

const FORMATS: [OutputFormat; 2] = [OutputFormat::Plain, OutputFormat::Markdown];

fn assert_text_without_table(name: &str, labels: &[&str]) {
    for format in FORMATS {
        let doc = read_page(name, format.clone());
        assert!(
            doc.tables.is_empty(),
            "{name} {format:?}: scattered labels became {} table(s): {:?}",
            doc.tables.len(),
            doc.tables.iter().map(|table| table.cells.clone()).collect::<Vec<_>>()
        );
        assert!(
            !doc.content.contains("| ---"),
            "{name} {format:?}: content holds a table: {}",
            doc.content
        );
        for label in labels {
            assert_eq!(
                doc.content.matches(label).count(),
                1,
                "{name} {format:?}: label {label:?} must appear once in: {}",
                doc.content
            );
        }
    }
}

#[test]
fn scattered_cover_blocks_stay_text() {
    assert_text_without_table(
        "cover_scattered.png",
        &[
            "HARBOR",
            "CITY",
            "Annual",
            "Review",
            "Seventh Edition",
            "Prepared for the council",
            "Office of Planning",
            "Spring Volume Two",
        ],
    );
}

#[test]
fn loose_grid_of_map_labels_stays_text() {
    assert_text_without_table(
        "map_loose_grid.png",
        &[
            "Pine Ridge",
            "Cedar Point",
            "Fox Hollow",
            "Quarry Hill",
            "Salt Marsh",
            "Bell Tower",
        ],
    );
}

// The PDF build drops a two-column OCR grid before the shape rule runs, as the base does. ~keep
#[test]
#[cfg(not(feature = "pdf"))]
fn ledger_with_one_value_per_row_stays_one_table_with_each_cell_once() {
    let cells = ["Debit", "Credit", "100", "200", "300", "400"];
    for format in FORMATS {
        let doc = read_page("ledger_plain.png", format.clone());
        assert_eq!(doc.tables.len(), 1, "ledger {format:?}: expected one table");
        for cell in cells {
            assert_eq!(
                doc.content.matches(cell).count(),
                1,
                "ledger {format:?}: cell {cell:?} must appear once in: {}",
                doc.content
            );
        }
    }
}

#[test]
fn ruled_table_stays_one_table_with_each_cell_once() {
    let cells = [
        "Anvil", "Bucket", "Chisel", "Ladder", "12.50", "50.00", "3.20", "32.00", "5.75", "40.25", "48.00", "96.00",
    ];
    for format in FORMATS {
        let doc = read_page("ruled_table.png", format.clone());
        assert_eq!(doc.tables.len(), 1, "ruled_table {format:?}: expected one table");
        for cell in cells {
            assert_eq!(
                doc.content.matches(cell).count(),
                1,
                "ruled_table {format:?}: cell {cell:?} must appear once in: {}",
                doc.content
            );
        }
    }
}

/// The words of a candidate that the rule keeps as text must reach the markdown. A garbled line
/// sits in a page that has a candidate region; the table path read it, the noise filter dropped it
/// from the page text, and the page printed 36 words where the table print had 41.
#[test]
#[cfg(feature = "pdf")]
fn words_of_a_rejected_candidate_stay_in_the_markdown() {
    const DOCUMENT: &str = "pdf_scanned/rtl_arabic_scanned.pdf";
    if helpers::skip_if_missing(DOCUMENT) {
        return;
    }
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    };
    let path = helpers::get_test_file_path(DOCUMENT);
    let doc = helpers::extract_uri_document_blocking(path.to_str().expect("utf-8 path"), None, &config)
        .expect("OCR extraction");
    let words = doc
        .content
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .count();
    assert!(words >= 41, "the page prints {words} words, the table print had 41");
}
