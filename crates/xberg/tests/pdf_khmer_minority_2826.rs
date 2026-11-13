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
                            .any(|word| word.trim_matches(|ch: char| ch.is_ascii_punctuation()) == "id"),
                        "{:?}: {content}",
                        config.output_format
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

#[cfg(any(feature = "paddle-ocr-tract", feature = "paddle-ocr-ort"))]
#[test]
fn paddle_enabled_pdf_keeps_khmer_from_its_detached_tesseract_read() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use tracing_subscriber::layer::SubscriberExt as _;

    struct DetachedRecovery(Arc<AtomicBool>);
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for DetachedRecovery {
        fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
            struct Message(bool);
            impl tracing::field::Visit for Message {
                fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                    if field.name() == "message" {
                        self.0 = format!("{value:?}") == "Detached Tesseract read recovered Khmer";
                    }
                }
            }
            let mut message = Message(false);
            event.record(&mut message);
            if message.0 {
                self.0.store(true, Ordering::Relaxed);
            }
        }
    }

    let detached = Arc::new(AtomicBool::new(false));
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(DetachedRecovery(detached.clone())))
        .expect("this exact test owns the tracing subscriber");
    let config = automatic_english();
    let source = include_bytes!("fixtures/pdf/khmer-minority-2826/ordered.pdf");
    let result = helpers::extract_bytes_document_blocking(source, "application/pdf", &config).unwrap();
    assert!(
        detached.load(Ordering::Relaxed),
        "expected recovery in the detached Tesseract read"
    );
    assert_eq!(
        result.pages.as_ref().unwrap()[0]
            .ocr_confidence
            .as_ref()
            .unwrap()
            .backend,
        "tesseract"
    );
    for content in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert_eq!(khmer(content).chars().count(), 222);
        let first = content.find(is_khmer).unwrap();
        let last = content.rfind(is_khmer).unwrap();
        assert!(content[..first].contains("auditorium"));
        assert!(content[last..].contains("reception"));
    }
}

#[test]
fn reporter_page_keeps_khmer_and_garden_club_under_automatic_english_ocr() {
    let pdf = include_bytes!("fixtures/pdf/khmer-minority-2826/khmer-in-english-page.pdf");
    let phrase = "The garden club meets every Monday";
    let native = helpers::extract_bytes_document_blocking(
        pdf,
        "application/pdf",
        &ExtractionConfig {
            disable_ocr: true,
            ocr: None,
            ..automatic_english()
        },
    )
    .unwrap();
    let source = khmer(&native.content);
    assert_eq!(source.chars().count(), 222);
    assert!(native.content.contains(phrase));
    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let config = ExtractionConfig {
            output_format,
            ..automatic_english()
        };
        let result = helpers::extract_bytes_document_blocking(pdf, "application/pdf", &config).unwrap();
        let pages = result.pages.as_ref().unwrap();
        assert_eq!(pages.len(), 1);
        for (surface, content) in [("document", &result.content), ("page", &pages[0].content)] {
            assert_eq!(
                khmer(content).chars().count(),
                222,
                "{:?} {surface} {:?}",
                config.output_format,
                result.extraction_method
            );
            assert_eq!(
                khmer(content),
                source,
                "{:?} {surface} source order",
                config.output_format
            );
            assert!(
                content
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .contains(phrase),
                "{:?} {surface}: {content}",
                config.output_format
            );
        }
    }
}
