#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use std::path::Path;

use xberg::core::config::{OcrQualityThresholds, PageConfig};
use xberg::types::OcrElementConfig;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, extract};

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
            element_config: Some(OcrElementConfig {
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
    let mut batch = extract(
        ExtractInput::from_bytes(
            std::fs::read(path).expect("read the qualified public PDF"),
            "application/pdf",
            None,
        ),
        &config,
    )
    .await
    .expect("extract the qualified public PDF");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    assert_eq!(batch.results.len(), 1);
    batch.results.remove(0)
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
            let before = read(&path, format.clone(), false).await;
            let after = read(&path, format, true).await;
            let page = |result: &ExtractedDocument| {
                result
                    .pages
                    .as_ref()
                    .expect("requested pages")
                    .iter()
                    .position(|p| p.page_number == 3)
                    .expect("PCB page")
            };
            let before_text = &before.pages.as_ref().unwrap()[page(&before)].content;
            let after_text = &after.pages.as_ref().unwrap()[page(&after)].content;
            assert_eq!(count(before_text, word), 1, "{file}");
            assert_eq!(count(after_text, word), 1, "{file}");
            for false_word in ["ceeces", "ccccee"] {
                assert_eq!(count(before_text, false_word), 1, "{file} {false_word}");
                assert_eq!(count(after_text, false_word), 0, "{file} {false_word}");
            }
            assert!(
                after.processing_warnings.iter().any(|warning| {
                    warning.message.starts_with("Page 3: removed ")
                        && warning.message.contains("repeated background texture")
                }),
                "{file}"
            );
        }
    }
}
