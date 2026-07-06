//! Network-gated integration smoke test for Hunyuan-OCR.
//!
//! ## Model
//!
//! **HuggingFace model ID**: `tencent/Hunyuan-OCR`
//!
//! ## Hardware requirements
//!
//! The full model is approximately 13-15 GB on disk and requires at least 16 GB
//! of GPU VRAM (BF16) or equivalent CPU RAM (F32, much slower).
//!
//! ## How to run
//!
//! 1. Download the model weights from HuggingFace Hub:
//!    ```sh
//!    huggingface-cli download tencent/Hunyuan-OCR --local-dir /path/to/model
//!    ```
//! 2. Set the model path and run the test:
//!    ```sh
//!    XBERG_HUNYUAN_OCR_MODEL_PATH=/path/to/model \
//!        cargo test -p xberg-candle-ocr --features hunyuan-ocr \
//!                   --test hunyuan_ocr_integration -- --ignored --nocapture
//!    ```

#![cfg(feature = "hunyuan-ocr")]

use xberg_candle_ocr::models::hunyuan_ocr::HunyuanOCREngine;

/// Degenerate-repeat detector.
///
/// A healthy OCR output over a small fixture should never repeat the same
/// 3-gram more than 4 times consecutively.  This catches the "word word word…"
/// failure mode where nucleus sampling collapses into a repeating loop.
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

/// Real-weights smoke test for the Hunyuan-OCR engine.
///
/// Guarded by `#[ignore]` — only runs when `--ignored` is passed.
/// Skips gracefully when `XBERG_HUNYUAN_OCR_MODEL_PATH` is unset.
#[test]
#[ignore = "requires Hunyuan-OCR weights from HuggingFace; run with --ignored"]
fn hunyuan_ocr_extracts_text_from_sample_image() {
    let model_path = match std::env::var("XBERG_HUNYUAN_OCR_MODEL_PATH") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            println!(
                "XBERG_HUNYUAN_OCR_MODEL_PATH is not set — skipping integration test.\n\
                 Set it to the local path of the tencent/Hunyuan-OCR model to run."
            );
            return;
        }
    };

    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    eprintln!("Constructing Hunyuan-OCR engine from {}…", model_path);
    // Run the reference-correctness path on CPU in F32, like the DeepSeek-OCR e2e test.
    // The checkpoint is bfloat16, but candle has no BF16 matmul on this backend, so
    // leaving dtype unset (which defaults to the config's BF16) makes every matmul fail.
    let device = candle_core::Device::Cpu;
    let mut engine = HunyuanOCREngine::init(&model_path, Some(&device), Some(candle_core::DType::F32))
        .expect("HunyuanOCREngine::init should succeed with a valid model directory");

    eprintln!("Engine ready. Running inference on test fixture…");
    let output = engine
        .process_image(image_bytes)
        .expect("process_image should succeed on a valid PNG");

    eprintln!("Output content ({} chars):", output.content.len());
    eprintln!("{}", output.content);

    assert!(
        !output.content.is_empty(),
        "OCR output must not be empty for a non-blank image"
    );
    assert!(
        output.content.len() > 2,
        "OCR output is suspiciously short ({} chars); expected more than 2 chars",
        output.content.len()
    );

    // The fixture renders "hello world".  A working pipeline should recover at
    // least one of the words.
    let lower = output.content.to_lowercase();
    assert!(
        lower.contains("hello") || lower.contains("world"),
        "Expected output to contain \"hello\" or \"world\"; got {:?}",
        output.content
    );

    // Degenerate-repeat guard: no 3-gram should repeat 5+ times consecutively.
    let repeat_run = longest_repeated_ngram_run(&output.content, 3);
    assert!(
        repeat_run < 5,
        "Detected degenerate-repeat output (longest 3-gram run = {}): {}…",
        repeat_run,
        &output.content[..200.min(output.content.len())]
    );

    eprintln!("Hunyuan-OCR smoke test passed.");
}
