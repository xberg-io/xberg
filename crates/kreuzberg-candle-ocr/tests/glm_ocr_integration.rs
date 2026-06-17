#![cfg(feature = "glm-ocr")]

use kreuzberg_candle_ocr::DevicePreference;
use kreuzberg_candle_ocr::models::glm_ocr::{GlmOcrEngine, GlmOcrTask};

/// Network-gated smoke test for GLM-OCR end-to-end inference.
///
/// Downloads ~3GB of model weights on first run (cached in ~/.cache/huggingface).
/// Subsequent runs use cached weights. Marked with #[ignore] so it only runs on
/// `cargo test -- --ignored --nocapture`.
#[test]
#[ignore = "downloads 3GB of GLM-OCR weights from HuggingFace Hub"]
fn glm_ocr_smoke_ocr_on_fixture() {
    // Use a fixture image with recognizable text
    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    // Resolve device and dtype
    let device = DevicePreference::Auto.select().expect("Failed to select device");

    // F32 for portability — candle 0.10 lacks BF16 matmul on some kernels.
    // BF16 path is the production target on Metal; revisit after smoke is green.
    let dtype = kreuzberg_candle_ocr::DType::F32;

    // Construct the engine (this downloads weights on first run)
    eprintln!("Constructing GLM-OCR engine (downloading weights if needed)...");
    let engine = GlmOcrEngine::new(GlmOcrTask::Ocr, device, dtype).expect("Failed to construct GLM-OCR engine");

    eprintln!("Engine constructed. Running inference on test image...");

    // Run inference
    let output = engine.process_image(image_bytes).expect("Failed to process image");

    eprintln!("Inference completed successfully!");
    eprintln!("Output content length: {} chars", output.content.len());
    eprintln!("Is structured markdown: {}", output.is_structured_markdown);
    eprintln!("Output text:\n{}", output.content);

    assert!(!output.content.is_empty(), "Output text should not be empty");
    assert!(
        output.content.len() > 5,
        "Output text should have more than 5 characters"
    );
    assert!(
        output.is_structured_markdown,
        "GLM-OCR output should be marked as structured markdown"
    );

    // The fixture renders the words "hello" and "world". A working pipeline
    // should recover at least one. Catches degenerate-repeat outputs that pass
    // weaker length-only assertions.
    let lower = output.content.to_lowercase();
    assert!(
        lower.contains("hello") || lower.contains("world"),
        "Expected output to contain \"hello\" or \"world\"; got {:?}",
        output.content
    );

    // Degenerate-repeat detector: catches the "binder title of binder title of…"
    // failure mode where nucleus sampling collapses into a repeating loop.
    // A healthy OCR output over a small fixture should never repeat the same
    // 3-gram more than 4 times consecutively.
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

    assert!(
        longest_repeated_ngram_run(&output.content, 3) < 5,
        "Detected degenerate-repeat output: {}...",
        &output.content[..200.min(output.content.len())]
    );

    eprintln!("\n✓ Smoke test passed!");
}
