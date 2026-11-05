//! Recovery of a full-page scan whose first OCR pass returns no text.
#![cfg(all(feature = "ocr", feature = "pdf", feature = "auto-rotate-tract"))]

use std::path::Path;

use xberg::{ExtractInput, ExtractionConfig, OcrConfig, extract};

async fn scan_text(ocr: Option<OcrConfig>) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/ocr_test_rotated_270.pdf");
    let bytes = std::fs::read(path).expect("rotated PDF fixture");
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        ocr,
        ..Default::default()
    };
    let input = ExtractInput::from_bytes(bytes, "application/pdf", None);
    let result = extract(input, &config).await.expect("PDF OCR succeeds");
    assert_eq!(result.results.len(), 1);
    result.results[0]
        .content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[tokio::test]
#[ignore = "requires the verified document-orientation model in the local cache and the PDF fixture"]
async fn cached_orientation_recovers_the_empty_270_degree_scan() {
    let expected = "Docling bundles PDF document conversion to JSON and Markdown in an easy self contained package";
    assert_eq!(
        scan_text(Some(OcrConfig::default())).await,
        "",
        "explicit orientation opt-out"
    );
    let implicit = scan_text(None).await;
    assert_eq!(
        scan_text(Some(OcrConfig {
            auto_rotate: true,
            ..Default::default()
        }))
        .await,
        expected,
        "explicit cached orientation control"
    );
    assert_eq!(implicit, expected, "implicit cached orientation recovery");
}

#[tokio::test]
#[ignore = "requires the PDF fixture and a separate process with an isolated empty model cache"]
async fn empty_270_degree_scan_stays_empty_without_a_cached_model() {
    assert_eq!(scan_text(None).await, "");
}
