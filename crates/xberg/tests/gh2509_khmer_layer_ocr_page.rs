//! GH#2509: a PDF page that goes to OCR on its own, in a document whose text layer is Khmer,
//! keeps its Khmer text and the English words and numbers that a read with `eng` alone keeps.
//!
//! The fixture is built from invented text. Page 1 is a text-only page whose layer is Khmer.
//! Page 2 is one picture, with no text layer, of a Khmer line above an English line with numbers.

#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const FIXTURE: &[u8] = include_bytes!("fixtures/ocr/gh2509_khmer_layer_picture_page.pdf");

fn khmer_letters(text: &str) -> usize {
    text.chars().filter(|c| ('\u{1780}'..='\u{17FF}').contains(c)).count()
}

fn khmer_model_installed() -> bool {
    let dirs = [
        std::env::var("TESSDATA_PREFIX").unwrap_or_default(),
        "/usr/share/tesseract-ocr/5/tessdata".to_string(),
        "/usr/share/tesseract-ocr/tessdata".to_string(),
        "/opt/homebrew/share/tessdata".to_string(),
    ];
    dirs.iter()
        .any(|dir| !dir.is_empty() && std::path::Path::new(dir).join("khm.traineddata").exists())
}

fn picture_page_text(output_format: OutputFormat) -> String {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        output_format,
        use_cache: false,
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(FIXTURE, "application/pdf", &config).expect("extraction");
    let pages = document.pages.as_ref().expect("pages");
    let picture_page = pages.iter().find(|page| page.page_number == 2).expect("page 2");
    picture_page.content.clone()
}

#[test]
fn a_picture_page_of_a_khmer_document_keeps_its_khmer_and_its_english_words_and_numbers() {
    if !khmer_model_installed() {
        println!("skipping: khm.traineddata is not installed");
        return;
    }
    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let text = picture_page_text(output_format.clone());
        assert!(
            khmer_letters(&text) >= 10,
            "{output_format:?}: the picture page lost its Khmer text: {} Khmer letters in {text:?}",
            khmer_letters(&text)
        );
        for kept in ["Invoice", "1042", "total", "385"] {
            assert!(
                text.contains(kept),
                "{output_format:?}: the picture page lost {kept:?}: {text:?}"
            );
        }
    }
}
