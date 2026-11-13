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
        assert!(actual.content.contains("library"));
        let pages = actual.pages.as_ref().unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(khmer(&pages[0].content), expected);
        assert!(pages[0].content.contains("library"));
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
        for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
            let config = ExtractionConfig {
                output_format,
                ..config.clone()
            };
            let read = helpers::extract_bytes_document_blocking(pdf, "application/pdf", &config).unwrap();
            assert_eq!(
                khmer(&read.content).chars().count(),
                expected,
                "{:?}",
                config.output_format
            );
            let pages = read.pages.as_ref().unwrap();
            assert_eq!(pages.len(), 1);
            assert_eq!(
                khmer(&pages[0].content).chars().count(),
                expected,
                "{:?}",
                config.output_format
            );
            assert!(read.content.chars().any(|ch| ch.is_ascii_alphabetic()));
        }
    }
}

#[test]
fn mixed_ocr_keeps_khmer_between_english_and_rejects_unrelated_ink() {
    const ORDERED: &[u8] = include_bytes!("fixtures/pdf/khmer-minority-2826/ordered.pdf");
    const OVERLAP: &[u8] = include_bytes!("fixtures/pdf/khmer-minority-2826/overlap.pdf");
    const MIXED_MAP: &[u8] = include_bytes!("fixtures/pdf/khmer-minority-2826/mixed-map.pdf");
    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let config = ExtractionConfig {
            output_format,
            ..automatic_english()
        };
        for (name, pdf, expected) in [
            ("ordered", ORDERED, 222),
            ("overlap", OVERLAP, 0),
            ("mixed-map", MIXED_MAP, 222),
        ] {
            let result = helpers::extract_bytes_document_blocking(pdf, "application/pdf", &config).unwrap();
            for content in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
                assert_eq!(
                    khmer(content).chars().count(),
                    expected,
                    "{name} {:?}",
                    config.output_format
                );
                if name == "mixed-map" {
                    assert!(
                        content
                            .split_whitespace()
                            .any(|word| word.trim_matches(|ch: char| ch.is_ascii_punctuation()) == "ID")
                    );
                }
                if expected != 0 {
                    let first = content.find(is_khmer).unwrap();
                    let last = content.rfind(is_khmer).unwrap();
                    assert!(content[..first].contains("auditorium"), "{content}");
                    assert!(content[last..].contains("reception"), "{content}");
                }
            }
        }
    }
}

fn is_khmer(ch: char) -> bool {
    ('\u{1780}'..='\u{17ff}').contains(&ch)
}
