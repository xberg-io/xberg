#![cfg(feature = "glm-ocr")]

use kreuzberg_candle_ocr::DevicePreference;
use kreuzberg_candle_ocr::models::glm_ocr::{GlmOcrEngine, GlmOcrTask};

/// Degenerate-repeat detector shared across GLM-OCR tests.
///
/// Catches the "binder title of binder title of…" failure mode where nucleus
/// sampling collapses into a repeating loop. Returns the length of the longest
/// consecutive run of an identical N-gram in `text`.
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

/// Smoke test for the `process_image_with_task` dispatch path.
///
/// Constructs a `GlmOcrEngine` and calls `process_image_with_task` directly,
/// exercising the paired-mode dispatch layer independently from the whole-page
/// `process_image` convenience wrapper.
///
/// Full multi-region paired test pending fixture addition; this test exercises
/// process_image_with_task explicitly so the paired-mode dispatch path is
/// covered by unit-level invocation.
///
/// Downloads ~3GB of model weights on first run (cached in ~/.cache/huggingface).
/// Subsequent runs use cached weights.
/// Run with: `cargo test -p kreuzberg-candle-ocr --features glm-ocr --test glm_ocr_paired_pipeline -- --ignored --nocapture`
#[test]
#[ignore = "downloads ~3GB of GLM-OCR weights from HuggingFace Hub"]
fn glm_ocr_paired_pipeline_smoke_via_process_image_with_task() {
    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    let device = DevicePreference::Auto.select().expect("Failed to select device");
    let dtype = kreuzberg_candle_ocr::DType::F32;

    eprintln!("Constructing GLM-OCR engine (downloading weights if needed)...");
    let engine = GlmOcrEngine::new(GlmOcrTask::Ocr, device, dtype).expect("Failed to construct GLM-OCR engine");

    eprintln!("Engine constructed. Running process_image_with_task on test image...");

    // Call process_image_with_task explicitly — this is the dispatch entry-point
    // used by the paired-mode backend for each cropped region.
    let output = engine
        .process_image_with_task(image_bytes, GlmOcrTask::Ocr)
        .expect("Failed to process image with task");

    eprintln!("process_image_with_task completed successfully!");
    eprintln!("Output content length: {} chars", output.content.len());
    eprintln!("Is structured markdown: {}", output.is_structured_markdown);
    eprintln!("Output text:\n{}", output.content);

    assert!(!output.content.is_empty(), "Output should not be empty");

    let lower = output.content.to_lowercase();
    assert!(
        lower.contains("hello") || lower.contains("world"),
        "Expected output to contain \"hello\" or \"world\"; got {:?}",
        output.content
    );

    assert!(
        longest_repeated_ngram_run(&output.content, 3) < 5,
        "Detected degenerate-repeat output: {}...",
        &output.content[..200.min(output.content.len())]
    );

    eprintln!("\n✓ Paired pipeline smoke test passed!");
}
