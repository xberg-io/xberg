#![cfg(all(
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

//! A line the dictionary filter judges noise and keeps for its numeral still counts as judged noise.
//!
//! `noise.png` is six lines of black Arial 44 on a white canvas: four sentences, one line of random
//! letters that ends in a year, and one line of random letters alone.

use xberg::{ExtractInput, ExtractionConfig, OcrConfig, OutputFormat, extract};

const NOISE_PNG: &[u8] = include_bytes!("fixtures/ocr_numeral_noise_2851/noise.png");

const KEPT: &str = "Tesseract kept 1 OCR line(s) whose dictionary-checkable words were mostly not real words, \
                    because they hold numbers";
const REMOVED: &str =
    "Tesseract removed 1 OCR line(s) because their dictionary-checkable words were mostly not real words";

#[tokio::test]
async fn the_ocr_result_counts_a_line_kept_for_its_numeral_apart_from_a_removed_one() {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Markdown,
        ocr: Some(OcrConfig::default()),
        ..ExtractionConfig::default()
    };
    let mut batch = extract(ExtractInput::from_bytes(NOISE_PNG.to_vec(), "image/png", None), &config)
        .await
        .expect("extract");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    let document = batch.results.remove(0);

    let warnings: Vec<&str> = document
        .processing_warnings
        .iter()
        .map(|w| w.message.as_ref())
        .collect();
    assert!(document.content.contains("2024"), "{:?}", document.content);
    assert!(warnings.contains(&KEPT), "{warnings:?} for {:?}", document.content);
    assert!(warnings.contains(&REMOVED), "{warnings:?} for {:?}", document.content);
}
