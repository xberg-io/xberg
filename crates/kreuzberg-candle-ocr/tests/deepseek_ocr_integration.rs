//! Network-gated integration smoke test for the DeepSeek-OCR backend.
//!
//! # Model
//!
//! HuggingFace model id: `deepseek-ai/DeepSeek-OCR`
//!
//! Disk footprint: ~2.7 GB (safetensors weights + tokenizer).
//!
//! ## Runtime expectations
//!
//! - CUDA is preferred; CPU inference is supported but slow (~minutes per image).
//! - BF16 weights are the production target on CUDA/Metal; F32 is used here for
//!   broad CPU portability.
//!
//! ## Running this test
//!
//! ```sh
//! KREUZBERG_DEEPSEEK_OCR_MODEL_PATH=/path/to/deepseek-ocr \
//!   cargo test -p kreuzberg-candle-ocr --features deepseek-ocr \
//!              --test deepseek_ocr_integration -- --ignored --nocapture
//! ```
//!
//! If the environment variable is unset the test prints a message and skips
//! without failing.
//!
//! ## Preprocessing note
//!
//! The Phase 5 `process_image` implementation uses placeholder zero-tensors for
//! `image_crop` and `images_spatial_crop` (flagged in the Phase 5 commit).  The
//! model still runs a forward pass via the zero-crop branch, producing output
//! from the global-features-only path.  The degenerate-repeat guard catches the
//! most common failure mode (nucleus-sampling collapse).  Phase 6 benchmark
//! gating will measure whether the placeholder path degrades extraction quality.

#![cfg(feature = "deepseek-ocr")]

use kreuzberg_candle_ocr::models::deepseek_ocr::DeepseekOCREngine;

/// Longest run of a repeated n-gram in `text`.
///
/// Returns the maximum number of consecutive identical n-grams found.
/// A healthy OCR output over a small fixture should never repeat the same
/// 3-gram more than 4 times consecutively.
fn longest_repeated_ngram_run(text: &str, n: usize) -> usize {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() < n * 2 {
        return 0;
    }
    let mut max_run = 0usize;
    for start in 0..tokens.len().saturating_sub(n - 1) {
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

/// Network-gated smoke test: load real DeepSeek-OCR weights and run inference
/// on a fixture image that contains recognizable text.
///
/// Skips gracefully when `KREUZBERG_DEEPSEEK_OCR_MODEL_PATH` is not set.
/// Run with `--ignored` to exercise this test.
#[test]
#[ignore = "requires DeepSeek-OCR weights from HuggingFace; set KREUZBERG_DEEPSEEK_OCR_MODEL_PATH and run with --ignored"]
fn deepseek_ocr_extracts_text_from_sample_image() {
    // Resolve model path from environment — skip gracefully if absent.
    let model_path = match std::env::var("KREUZBERG_DEEPSEEK_OCR_MODEL_PATH") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            println!(
                "KREUZBERG_DEEPSEEK_OCR_MODEL_PATH not set — skipping integration test.\n\
                 Download the model from huggingface.co/deepseek-ai/DeepSeek-OCR (~2.7 GB)\n\
                 and set the env var to its directory path."
            );
            return;
        }
    };

    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    eprintln!("Loading DeepSeek-OCR engine from: {model_path}");

    // Version 2 is the primary release (SAM + Qwen2 vision tower).
    let mut engine = DeepseekOCREngine::init(
        &model_path,
        candle_core::Device::Cpu,
        candle_core::DType::F32,
        2, // version
    )
    .expect("DeepseekOCREngine::init must succeed with a valid model path");

    eprintln!("Engine loaded. Running inference on fixture image …");

    let output = engine
        .process_image(image_bytes, None)
        .expect("process_image must not error on a valid PNG");

    eprintln!("Output ({} chars):\n{output}", output.len());

    // ── Assertion 1: non-empty output ─────────────────────────────────────────
    assert!(!output.is_empty(), "DeepSeek-OCR output must not be empty");
    assert!(
        output.len() > 3,
        "DeepSeek-OCR output must contain more than 3 characters, got {:?}",
        output
    );

    // ── Assertion 2: degenerate-repeat guard ──────────────────────────────────
    // A longest 3-gram run >= 5 is a strong signal of nucleus-sampling collapse.
    let repeat_run = longest_repeated_ngram_run(&output, 3);
    assert!(
        repeat_run < 5,
        "Detected degenerate-repeat output (longest 3-gram run = {repeat_run}): {}…",
        &output[..200.min(output.len())]
    );

    eprintln!("✓ DeepSeek-OCR smoke test passed (repeat_run={repeat_run})");
}
