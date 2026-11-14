use std::path::Path;

use crate::core::config::{OcrQualityThresholds, PageConfig};
use crate::types::{OcrElementConfig, OcrElementLevel};
use crate::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat};

fn count(text: &str, target: &str) -> usize {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| *token == target)
        .count()
}

async fn read(path: &Path, format: OutputFormat, prune: bool, raw_elements: bool) -> ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: format,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: std::env::var_os("XBERG_2943_TESSDATA").map(Into::into),
            element_config: raw_elements.then(|| OcrElementConfig {
                include_elements: true,
                build_hierarchy: true,
                ..Default::default()
            }),
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
            let before = super::REPLAY
                .scope(None, read(&path, format.clone(), true, false))
                .await;
            let (after, removals) = super::REPLAY
                .scope(Some(Default::default()), async {
                    let after = read(&path, format.clone(), true, false).await;
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
#[ignore = "requires the original public PCB subset in XBERG_2943_ARCHIVE"]
async fn original_pcb_hole_row_is_removed_in_both_formats() {
    let archive = std::env::var_os("XBERG_2943_ARCHIVE").expect("set XBERG_2943_ARCHIVE");
    let path = Path::new(&archive).join("historical-cli.2Fmdbd/poc-selected.pdf");
    let row = "cece' cocece ceeces coecee wecece cceece ccccee cocece eeeece eeecce ececee";
    let expected: Vec<_> = row.split_whitespace().collect();
    let mut failures = Vec::new();
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let before = read(&path, format.clone(), false, true).await;
        let after = read(&path, format.clone(), true, true).await;
        assert_eq!(
            serde_json::to_value(&before.ocr_elements).unwrap(),
            serde_json::to_value(&after.ocr_elements).unwrap(),
            "{format:?}: raw recognitions"
        );
        let before = page_content(&before, 3).replace('’', "'");
        let after = page_content(&after, 3).replace('’', "'");
        let tokens: Vec<_> = before.split_whitespace().collect();
        assert_eq!(
            tokens
                .windows(expected.len())
                .filter(|window| *window == expected)
                .count(),
            1,
            "{format:?}"
        );
        for (index, word) in expected.iter().enumerate() {
            if expected[..index].contains(word) {
                continue;
            }
            let row_count = expected.iter().filter(|item| *item == word).count();
            let before_count = before.split_whitespace().filter(|item| item == word).count();
            let after_count = after.split_whitespace().filter(|item| item == word).count();
            if after_count != before_count - row_count {
                failures.push((format.clone(), *word, before_count - row_count, after_count));
            }
        }
        assert_eq!(
            count(&before, "33"),
            count(&after, "33"),
            "{format:?}: printed page number"
        );
        assert!(count(&after, "33") > 0, "{format:?}: printed page number missing");
        eprintln!("{format:?}: exercised all {} PCB occurrences", expected.len());
    }
    assert!(
        failures.is_empty(),
        "PCB occurrence counts (format, word, expected, actual): {failures:?}"
    );
}

#[test]
#[ignore = "requires XBERG_2943_ORIGINAL_PCB and the verified subset under XBERG_2943_ARCHIVE"]
fn pcb_subset_page_three_matches_full_document_page_thirty_three() {
    use xberg_native_pdf::{
        PdfDocument,
        rendering::{RenderOptions, render_page},
    };
    let archive = std::env::var_os("XBERG_2943_ARCHIVE").expect("set XBERG_2943_ARCHIVE");
    let subset = Path::new(&archive).join("historical-cli.2Fmdbd/poc-selected.pdf");
    let original = std::env::var_os("XBERG_2943_ORIGINAL_PCB").expect("set XBERG_2943_ORIGINAL_PCB");
    let subset = PdfDocument::from_bytes(std::fs::read(subset).unwrap()).unwrap();
    let original = PdfDocument::from_bytes(std::fs::read(original).unwrap()).unwrap();
    let mut options = RenderOptions::default();
    options.dpi = 300;
    let subset_page = render_page(&subset, 2, &options).unwrap();
    let original_page = render_page(&original, 32, &options).unwrap();
    assert_eq!((subset_page.width, subset_page.height), (2550, 3300));
    assert_eq!(subset_page.data, original_page.data);
    assert_ne!(subset_page.data, render_page(&original, 31, &options).unwrap().data);
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
            let control = super::REPLAY
                .scope(None, read(&path, format.clone(), true, false))
                .await;
            let actual = read(&path, format.clone(), true, false).await;
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
            assert_eq!(
                count(&after, expected_word),
                usize::from(page != 1),
                "{file} {format:?} candidate"
            );
            if page == 1 {
                assert_eq!(after.matches("SECTION TITLE").count(), 1);
                assert_only_occurrence_removed(&before, &after, expected_word);
            }
            eprintln!("{file} {format:?}: {expected_word} 1 -> {}", usize::from(page != 1));
        }
    }
}

#[tokio::test]
#[ignore = "requires the archived PDFs under XBERG_2943_ARCHIVE"]
async fn document_pruning_preserves_raw_recognitions_and_isolated_print() {
    let archive = std::env::var_os("XBERG_2943_ARCHIVE").expect("set XBERG_2943_ARCHIVE");
    let archive = Path::new(&archive);
    let name = concat!("Sa", "de");
    let cases = [
        ("same-page.kutOmz/sade-y665-a65.pdf", 3, name, Some("ccccee")),
        ("same-page.kutOmz/nina-y665-a65.pdf", 3, "Nina", Some("ccccee")),
        ("historical-cli.2Fmdbd/photo-title.pdf", 1, name, None),
        (
            "photo-available-reference.mRy9U1/native-baked-matched-96.pdf",
            1,
            name,
            None,
        ),
        (
            "photo-available-reference.mRy9U1/native-baked-qualified.pdf",
            1,
            name,
            None,
        ),
    ];
    for (file, page, retained, removed) in cases {
        for format in [OutputFormat::Plain, OutputFormat::Markdown] {
            let path = archive.join(file);
            let before = super::REPLAY.scope(None, read(&path, format.clone(), true, true)).await;
            let after = read(&path, format.clone(), true, true).await;
            assert_eq!(
                serde_json::to_value(&before.ocr_elements).unwrap(),
                serde_json::to_value(&after.ocr_elements).unwrap(),
                "{file} {format:?}"
            );
            let raw = after.ocr_elements.as_ref().expect("requested raw OCR elements");
            let raw_count = |word| {
                raw.iter()
                    .filter(|e| e.level == OcrElementLevel::Line && e.page_number == page && e.text == word)
                    .count()
            };
            assert_eq!(raw_count(retained), 1, "{file} {format:?}");
            assert_eq!(count(page_content(&before, page), retained), 1, "{file} {format:?}");
            let expected = usize::from(file != "historical-cli.2Fmdbd/photo-title.pdf");
            assert_eq!(
                count(page_content(&after, page), retained),
                expected,
                "{file} {format:?}"
            );
            if let Some(word) = removed {
                assert_eq!(raw_count(word), 1, "{file} {format:?}");
                assert_eq!(count(page_content(&before, page), word), 1, "{file} {format:?}");
                assert_eq!(count(page_content(&after, page), word), 0, "{file} {format:?}");
            } else {
                if expected == 1 {
                    assert_eq!(after.content, before.content, "{file} {format:?}");
                } else {
                    assert_only_occurrence_removed(page_content(&before, page), page_content(&after, page), retained);
                    assert_only_occurrence_removed(&before.content, &after.content, retained);
                }
                assert_eq!(after.content.matches("SECTION TITLE").count(), 1);
            }
        }
    }
}

fn assert_only_occurrence_removed(before: &str, after: &str, occurrence: &str) {
    let mut expected: Vec<_> = before.split_whitespace().collect();
    let index = expected
        .iter()
        .position(|token| *token == occurrence)
        .expect("isolated occurrence");
    expected.remove(index);
    assert_eq!(after.split_whitespace().collect::<Vec<_>>(), expected);
}

fn page_content(result: &ExtractedDocument, page: u32) -> &str {
    &result
        .pages
        .as_ref()
        .expect("requested pages")
        .iter()
        .find(|p| p.page_number == page)
        .expect("expected page")
        .content
}

#[tokio::test]
#[ignore = "requires the qualified CAS PDFs in XBERG_2943_PAINT"]
async fn unique_layered_trigrams_survive_noise_pruning() {
    let directory = std::env::var_os("XBERG_2943_PAINT").expect("set XBERG_2943_PAINT");
    for (file, expected) in [("cas-ink-1350.pdf", 1), ("cas-ink-280.pdf", 1), ("photo-title.pdf", 0)] {
        for format in [OutputFormat::Plain, OutputFormat::Markdown] {
            let path = Path::new(&directory).join(file);
            let before = read(&path, format.clone(), false, true).await;
            let after = read(&path, format.clone(), true, true).await;
            assert_eq!(count(page_content(&before, 1), "CAS"), 1, "{file} {format:?}");
            assert_eq!(
                serde_json::to_value(&before.ocr_elements).unwrap(),
                serde_json::to_value(&after.ocr_elements).unwrap(),
                "{file} {format:?}"
            );
            let raw = before.ocr_elements.as_ref().unwrap();
            assert_eq!(
                raw.iter()
                    .filter(|line| line.level == OcrElementLevel::Line && line.text == "CAS")
                    .count(),
                1
            );
            assert!(after.tables.is_empty());
            assert!(after.processing_warnings.iter().any(|warning| {
                warning.message.contains("removed ") && warning.message.contains("symbol-noise line")
            }));
            let content = page_content(&after, 1);
            assert_eq!(content.matches("SECTION TITLE").count(), 1, "{file} {format:?}");
            assert_eq!(count(content, "CAS"), expected, "{file} {format:?}");
            assert_eq!(
                count(&after.content, "CAS"),
                expected,
                "{file} {format:?}: document output"
            );
            if expected == 0 {
                assert_eq!(
                    count(page_content(&before, 1), "Sade"),
                    1,
                    "{file} {format:?} positive OCR control"
                );
                assert_eq!(count(content, "Sade"), 0, "{file} {format:?} certified background");
                assert_eq!(count(&after.content, "Sade"), 0, "{file} {format:?} document output");
            }
            eprintln!("{file} {format:?}: CAS 1 -> {expected}");
        }
    }
}
