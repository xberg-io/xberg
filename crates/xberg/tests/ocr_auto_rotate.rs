//! Regression test: `auto_rotate` must never crash the process.
//!
//! Tesseract's `DetectOrientationScript` corrupts engine memory in the vendored
//! build (SIGABRT/SIGSEGV, uncatchable), so orientation detection runs on the
//! ONNX PP-LCNet classifier instead. This test drives the tesseract backend with
//! `auto_rotate: true` end-to-end; before the fix the whole test process died
//! with a signal.
#![cfg(all(feature = "ocr", feature = "auto-rotate"))]

#[tokio::test]
async fn should_extract_with_auto_rotate_without_crashing() {
    let bytes = std::fs::read("../../test_documents/images/balance_sheet_1.png").expect("fixture must exist");
    let config = xberg::ExtractionConfig {
        ocr: Some(xberg::core::config::OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            auto_rotate: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = xberg::core::config::ExtractInput {
        kind: xberg::core::config::ExtractInputKind::Bytes,
        bytes: Some(bytes),
        mime_type: Some("image/png".to_string()),
        ..Default::default()
    };

    let result = xberg::extract(input, &config).await.expect("extraction must succeed");
    let text = result.results.first().map(|d| d.content.clone()).unwrap_or_default();
    assert!(
        !text.trim().is_empty(),
        "OCR with auto_rotate must still produce text output"
    );
}
