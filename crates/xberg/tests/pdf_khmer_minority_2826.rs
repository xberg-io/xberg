#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use xberg::core::config::PageConfig;
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const FRAGMENTED: &[u8] = include_bytes!("fixtures/pdf/khmer-minority-2826/fragmented.pdf");

fn khmer(text: &str) -> String {
    text.chars()
        .filter(|ch| ('\u{1780}'..='\u{17ff}').contains(ch))
        .collect()
}

fn automatic_english() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: "tesseract".into(),
            language: vec!["eng".into()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn automatic_english_ocr_keeps_visible_minority_khmer_on_a_fragmented_page() {
    let config = automatic_english();
    let native_config = ExtractionConfig {
        disable_ocr: true,
        ocr: None,
        ..config.clone()
    };
    let native = helpers::extract_bytes_document_blocking(FRAGMENTED, "application/pdf", &native_config).unwrap();
    let expected = khmer(&native.content);
    assert_eq!(expected.chars().count(), 222);
    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let config = ExtractionConfig {
            output_format,
            ..config.clone()
        };
        let actual = helpers::extract_bytes_document_blocking(FRAGMENTED, "application/pdf", &config).unwrap();
        assert_eq!(khmer(&actual.content), expected);
        let pages = actual.pages.as_ref().unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(khmer(&pages[0].content), expected);
    }
}

#[test]
fn automatic_english_ocr_keeps_clean_khmer_and_rejects_hidden_and_wrong_mapped_khmer() {
    let config = automatic_english();
    for (pdf, native_count, expected) in [
        (
            include_bytes!("fixtures/pdf/khmer-minority-2826/clean.pdf").as_slice(),
            222,
            222,
        ),
        (
            include_bytes!("fixtures/pdf/khmer-minority-2826/hidden.pdf").as_slice(),
            222,
            0,
        ),
        (
            include_bytes!("fixtures/pdf/khmer-minority-2826/wrong-map.pdf").as_slice(),
            303,
            0,
        ),
    ] {
        let native = helpers::extract_bytes_document_blocking(
            pdf,
            "application/pdf",
            &ExtractionConfig {
                disable_ocr: true,
                ocr: None,
                ..config.clone()
            },
        )
        .unwrap();
        assert_eq!(khmer(&native.content).chars().count(), native_count);
        let read = helpers::extract_bytes_document_blocking(pdf, "application/pdf", &config).unwrap();
        assert_eq!(khmer(&read.content).chars().count(), expected);
        let pages = read.pages.as_ref().unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(khmer(&pages[0].content).chars().count(), expected);
        assert!(read.content.chars().any(|ch| ch.is_ascii_alphabetic()));
    }
}
