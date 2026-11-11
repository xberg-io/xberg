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

const REMOVED: &str =
    "Tesseract removed 1 OCR line(s) because their dictionary-checkable words were mostly not real words";

/// The removed-line warning reads the judged count minus the kept count, so it says 1 only when the
/// kept line reached the count. The counts are internal: no warning names the kept line, and neither
/// count reaches the public metadata.
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
        .filter(|w| w.source == "tesseract")
        .map(|w| w.message.as_ref())
        .collect();
    assert!(document.content.contains("2024"), "{:?}", document.content);
    assert_eq!(warnings, [REMOVED], "for {:?}", document.content);
    for key in ["dictionary_filtered_line_count", "dictionary_kept_numeral_line_count"] {
        assert!(
            !document.metadata.additional.contains_key(key),
            "{key} leaked into the public metadata: {:?}",
            document.metadata.additional
        );
    }
}
