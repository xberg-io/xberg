//! Network-gated smoke test for PaddleOCR-VL 1.5 end-to-end inference.
//!
//! **HuggingFace model**: `paddlepaddle/paddleocr-v4` (PaddleOCR-VL 1.5 vision-language variant)
//! **Disk size**: ~700 MB – 2 GB depending on precision variant downloaded.
//! **CPU-feasible**: yes (~60–180 s/page on CPU).
//!
//! Download the model weights locally with:
//! ```sh
//! huggingface-cli download paddlepaddle/paddleocr-v4 \
//!     --local-dir /path/to/model \
//!     --include "config.json" "preprocessor_config.json" "tokenizer.json" "model.safetensors"
//! ```
//!
//! Then run this test with:
//! ```sh
//! KREUZBERG_PADDLEOCR_VL_MODEL_PATH=/path/to/model \
//!     cargo test -p kreuzberg-candle-ocr \
//!     --features paddleocr-vl \
//!     --test paddleocr_vl_15_integration \
//!     -- --ignored
//! ```

#![cfg(all(feature = "paddleocr-vl", not(target_arch = "wasm32")))]

use candle_core::{DType, Device};
use kreuzberg_candle_ocr::models::paddleocr_vl::{PaddleOcrVlEngine, PaddleOcrVlTask};

const ENV_MODEL_PATH: &str = "KREUZBERG_PADDLEOCR_VL_MODEL_PATH";

fn model_path_or_skip() -> Option<String> {
    match std::env::var(ENV_MODEL_PATH) {
        Ok(p) if !p.trim().is_empty() => Some(p),
        _ => {
            println!(
                "Skipping PaddleOCR-VL 1.5 smoke test: set {ENV_MODEL_PATH}=/path/to/model \
                 (~700 MB–2 GB local weights, CPU-feasible) to run."
            );
            None
        }
    }
}

/// Counts the longest run of the same `n`-gram in whitespace-tokenized `text`.
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

/// PaddleOCR-VL 1.5 smoke test: extracts non-empty text from a sample image.
///
/// Gated on `KREUZBERG_PADDLEOCR_VL_MODEL_PATH` pointing to a local model directory.
/// Marked `#[ignore]` so it only runs with `cargo test -- --ignored`.
#[test]
#[ignore = "requires PaddleOCR-VL 1.5 weights from HuggingFace; run with --ignored"]
fn paddleocr_vl_15_extracts_text_from_sample_image() {
    let model_path = match model_path_or_skip() {
        Some(p) => p,
        None => return,
    };

    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    let device = Device::Cpu;
    let dtype = DType::F32;

    println!("Constructing PaddleOCR-VL 1.5 engine from {model_path}...");
    let mut engine = PaddleOcrVlEngine::new(&model_path, PaddleOcrVlTask::Ocr, device, dtype)
        .expect("PaddleOcrVlEngine::new should succeed when model weights are present at model_path");

    println!("Running inference on test_hello_world.png ...");
    let output = engine
        .process_image(image_bytes)
        .expect("process_image should produce a CandleOcrOutput from a valid PNG");

    let content = output.content.trim();

    println!("Output length: {} chars", content.len());
    println!("is_structured_markdown: {}", output.is_structured_markdown);
    println!("Text:\n{content}");

    assert!(
        !content.is_empty(),
        "PaddleOCR-VL 1.5 decoded an empty string; model or decode loop may be broken"
    );
    assert!(
        content.len() > 5,
        "Output is suspiciously short ({} chars); expected recognizable text",
        content.len()
    );
    assert!(
        output.is_structured_markdown,
        "PaddleOCR-VL should set is_structured_markdown = true"
    );

    // Keyword check: the fixture renders "hello" and "world".
    let lower = content.to_lowercase();
    assert!(
        lower.contains("hello") || lower.contains("world"),
        "Expected \"hello\" or \"world\" in output; got: {content:?}"
    );

    // Degenerate-repeat detector: a healthy output should not repeat the same
    // 3-gram more than 4 consecutive times (catches nucleus-collapse failure mode).
    assert!(
        longest_repeated_ngram_run(content, 3) < 5,
        "Detected degenerate-repeat output (3-gram run ≥ 5): {}...",
        &content[..200.min(content.len())]
    );

    println!("Smoke test passed.");
}

/// PaddleOCR-VL 1.5 table task: model accepts `Table` task prompt without panicking.
///
/// Only verifies the engine constructs and produces output; does not assert table structure.
#[test]
#[ignore = "requires PaddleOCR-VL 1.5 weights from HuggingFace; run with --ignored"]
fn paddleocr_vl_15_table_task_produces_output() {
    let model_path = match model_path_or_skip() {
        Some(p) => p,
        None => return,
    };

    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    let mut engine = PaddleOcrVlEngine::new(&model_path, PaddleOcrVlTask::Table, Device::Cpu, DType::F32)
        .expect("PaddleOcrVlEngine::new with Table task should succeed");

    let output = engine
        .process_image(image_bytes)
        .expect("Table task process_image should not error on a valid image");

    // The fixture has no table structure; we only assert the engine doesn't panic or
    // return an error. Non-empty output would be a bonus.
    println!("Table task output ({} chars): {}", output.content.len(), output.content);
}
