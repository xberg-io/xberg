#![cfg(feature = "candle-glm-ocr")]

//! End-to-end integration test for `GlmOcrBackend` through the `OcrBackend` trait.
//!
//! Constructs the backend directly via its public constructor and drives
//! `process_image` through the trait surface, verifying that the full wiring
//! from `kreuzberg::candle_ocr::GlmOcrBackend` down to `kreuzberg-candle-ocr`
//! produces coherent output.
//!
//! Run with:
//! `cargo test -p kreuzberg --features candle-glm-ocr --test glm_ocr_backend -- --ignored --nocapture`

use kreuzberg::candle_ocr::{GlmOcrBackend, glm_ocr_backend::LayoutMode};
use kreuzberg::core::config::OcrConfig;
use kreuzberg::plugins::OcrBackend;
use kreuzberg_candle_ocr::models::GlmOcrTask;

/// End-to-end test driving `GlmOcrBackend` through the `OcrBackend` trait.
///
/// Downloads ~3GB of GLM-OCR model weights on first run (cached in
/// ~/.cache/huggingface). Subsequent runs use cached weights.
#[tokio::test]
#[ignore = "downloads ~3GB of GLM-OCR weights from HuggingFace Hub"]
async fn glm_ocr_backend_process_image_returns_hello_world() {
    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    // Construct the backend via its public constructor.
    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::WholePage);

    let config = OcrConfig::default();

    eprintln!("Calling GlmOcrBackend::process_image through OcrBackend trait...");
    let result = backend
        .process_image(image_bytes, &config)
        .await
        .expect("GlmOcrBackend::process_image should succeed");

    eprintln!("process_image returned successfully");
    eprintln!("Content length: {} chars", result.content.len());
    eprintln!("MIME type: {}", result.mime_type);
    eprintln!("Content:\n{}", result.content);

    assert!(!result.content.is_empty(), "Extracted content should not be empty");

    let lower = result.content.to_lowercase();
    assert!(
        lower.contains("hello") || lower.contains("world"),
        "Expected content to contain \"hello\" or \"world\"; got {:?}",
        result.content
    );

    let run = longest_repeated_ngram_run(&result.content, 3);
    assert!(
        run < 5,
        "Detected degenerate-repeat output (longest 3-gram run = {}): {}...",
        run,
        &result.content[..200.min(result.content.len())]
    );

    eprintln!("\n✓ GlmOcrBackend end-to-end test passed!");
}

/// Count the longest run of identical consecutive N-grams in `text`. Catches
/// degenerate generations where a model loops on the same phrase indefinitely.
fn longest_repeated_ngram_run(text: &str, n: usize) -> usize {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() < n * 2 {
        return 0;
    }
    let mut max_run = 0usize;
    for start in 0..tokens.len() - n + 1 {
        let pattern = &tokens[start..start + n];
        let mut run = 1usize;
        let mut next = start + n;
        while next + n <= tokens.len() && &tokens[next..next + n] == pattern {
            run += 1;
            next += n;
        }
        max_run = max_run.max(run);
    }
    max_run
}
