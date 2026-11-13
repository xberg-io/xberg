use std::path::Path;

use crate::core::config::{OcrQualityThresholds, PageConfig};
use crate::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat};

fn count(text: &str, target: &str) -> usize {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| *token == target)
        .count()
}

async fn read(path: &Path, format: OutputFormat, prune: bool) -> ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: format,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: std::env::var_os("XBERG_2943_TESSDATA").map(Into::into),
            quality_thresholds: Some(OcrQualityThresholds {
                prune_symbol_noise_lines: prune,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let bytes = std::fs::read(path).expect("read the qualified public PDF");
    Box::pin(crate::core::extractor::extract_bytes(
        &bytes,
        "application/pdf",
        &config,
    ))
    .await
    .expect("extract the qualified public PDF")
}

#[tokio::test]
#[ignore = "requires the qualified public PDFs in XBERG_2943_EVIDENCE"]
async fn qualified_pcb_texture_is_pruned_without_losing_the_faint_word() {
    let directory = std::env::var_os("XBERG_2943_EVIDENCE").expect("set XBERG_2943_EVIDENCE");
    let variants = [
        (concat!("sa", "de-y665-a65.pdf"), concat!("Sa", "de")),
        ("nina-y665-a65.pdf", "Nina"),
    ];
    for (file, word) in variants {
        for format in [OutputFormat::Plain, OutputFormat::Markdown] {
            let path = Path::new(&directory).join(file);
            let before = super::REPLAY.scope(None, read(&path, format.clone(), true)).await;
            let (after, removals) = super::REPLAY
                .scope(Some(Default::default()), async {
                    let after = read(&path, format.clone(), true).await;
                    let removals = super::REPLAY.with(|replay| std::mem::take(&mut *replay.as_ref().unwrap().lock()));
                    (after, removals)
                })
                .await;
            assert!(before.ocr_elements.as_ref().is_none_or(Vec::is_empty));
            assert!(after.ocr_elements.as_ref().is_none_or(Vec::is_empty));
            let before_pages = before.pages.as_ref().unwrap();
            let after_pages = after.pages.as_ref().unwrap();
            assert_eq!(before_pages.len(), after_pages.len());
            for (before_page, after_page) in before_pages.iter().zip(after_pages) {
                assert_eq!(before_page.page_number, after_page.page_number);
                if before_page.page_number != 3 {
                    assert_eq!(
                        before_page.content, after_page.content,
                        "{file} {format:?} page {}",
                        before_page.page_number
                    );
                    continue;
                }
                assert_eq!(count(&before_page.content, word), 1, "{file}");
                assert_eq!(count(&after_page.content, word), 1, "{file}");
                for anchor in ["ceeces", "ccccee"] {
                    assert_eq!(count(&before_page.content, anchor), 1, "{file}");
                    assert_eq!(count(&after_page.content, anchor), 0, "{file} {format:?} {anchor}");
                    assert!(removals.iter().any(|token| token == anchor));
                }
                let mut intended = removals.clone();
                let expected: Vec<_> = before_page
                    .content
                    .split_whitespace()
                    .filter(|token| {
                        let Some(index) = intended.iter().position(|removed| removed == token) else {
                            return true;
                        };
                        intended.swap_remove(index);
                        false
                    })
                    .collect();
                assert!(
                    intended.is_empty(),
                    "{file} {format:?}: missing intended removals {intended:?}"
                );
                let actual: Vec<_> = after_page.content.split_whitespace().collect();
                assert_eq!(expected.len(), actual.len(), "{file} {format:?}: token count");
                if let Some((index, (expected, actual))) = expected
                    .iter()
                    .zip(&actual)
                    .enumerate()
                    .find(|(_, (left, right))| left != right)
                {
                    panic!("{file} {format:?}: token {index}, expected {expected:?}, got {actual:?}");
                }
                eprintln!("{file} {format:?}: {} intended removals", removals.len());
            }
        }
    }
}

#[tokio::test]
#[ignore = "requires the public PDFs in XBERG_2943_SOURCE_CONTROLS"]
async fn original_photo_and_recognized_same_pad_print_controls() {
    let directory = std::env::var_os("XBERG_2943_SOURCE_CONTROLS").expect("set XBERG_2943_SOURCE_CONTROLS");
    let word = concat!("Sa", "de");
    for (file, page, expected_word) in [
        ("photo-title.pdf", 1, word),
        ("preflight.pdf", 3, word),
        ("heldout.pdf", 3, "Nina"),
    ] {
        for format in [OutputFormat::Plain, OutputFormat::Markdown] {
            let path = Path::new(&directory).join(file);
            let control = super::REPLAY.scope(None, read(&path, format.clone(), true)).await;
            let actual = read(&path, format.clone(), true).await;
            let page_text = |result: ExtractedDocument| {
                result
                    .pages
                    .unwrap()
                    .into_iter()
                    .find(|p| p.page_number == page)
                    .unwrap()
                    .content
            };
            let before = page_text(control);
            let after = page_text(actual);
            assert_eq!(count(&before, expected_word), 1, "{file} {format:?} control");
            assert_eq!(count(&after, expected_word), 1, "{file} {format:?} candidate");
            if page == 1 {
                assert_eq!(after, before, "{file} {format:?}");
                assert_eq!(after.matches("SECTION TITLE").count(), 1);
            }
            eprintln!("{file} {format:?}: {expected_word}=1 in control and candidate");
        }
    }
}
