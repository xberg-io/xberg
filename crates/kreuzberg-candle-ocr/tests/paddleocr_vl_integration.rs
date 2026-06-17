//! End-to-end integration test for the candle PaddleOCR-VL engine.
//!
//! Marked `#[ignore]` because it pulls ~1.8 GB of safetensors from
//! HuggingFace Hub and the autoregressive decode on CPU is multi-minute.
//! Run explicitly with:
//!
//! ```sh
//! KREUZBERG_NETWORK_TESTS=1 cargo test -p kreuzberg-candle-ocr \
//!     --features paddleocr-vl --test paddleocr_vl_integration -- --ignored
//! ```
//!
//! Skipped silently when `KREUZBERG_NETWORK_TESTS` is unset.

#![cfg(all(feature = "paddleocr-vl", not(target_arch = "wasm32")))]

use std::path::PathBuf;

use candle_core::{DType, Device};
use kreuzberg_candle_ocr::models::{PaddleOcrVlEngine, PaddleOcrVlTask};

const ENV_GATE: &str = "KREUZBERG_NETWORK_TESTS";

fn skip_unless_network_tests_opt_in() -> bool {
    match std::env::var(ENV_GATE) {
        Ok(v) if matches!(v.as_str(), "1" | "true" | "yes") => false,
        _ => {
            eprintln!(
                "Skipping PaddleOCR-VL integration test; set {ENV_GATE}=1 to run (~1.8 GB \
                 download from HuggingFace, multi-minute decode on CPU)."
            );
            true
        }
    }
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/images")
        .join(name)
}

#[test]
#[ignore = "pulls ~1.8 GB from HF Hub; gated on KREUZBERG_NETWORK_TESTS=1"]
fn paddleocr_vl_ocr_task_recognises_full_page_fixture() {
    if skip_unless_network_tests_opt_in() {
        return;
    }

    let fixture = fixture_path("english_and_korean.png");
    let bytes = std::fs::read(&fixture).unwrap_or_else(|e| {
        panic!("failed to read fixture {}: {e}", fixture.display());
    });

    // Model path resolved from env or HuggingFace cache; the original API required a local path.
    let model_path = std::env::var("KREUZBERG_PADDLEOCR_VL_MODEL_PATH").unwrap_or_default();
    let mut engine = PaddleOcrVlEngine::new(&model_path, PaddleOcrVlTask::Ocr, Device::Cpu, DType::F32)
        .expect("PaddleOcrVlEngine::new should succeed when weights are present at model_path");

    let output = engine
        .process_image(&bytes)
        .expect("process_image should produce a CandleOcrOutput on a full-page document fixture");

    let content = output.content.trim();
    assert!(
        !content.is_empty(),
        "PaddleOCR-VL decoded an empty string for a full-page document; either the model failed \
         to load, the decode loop is broken, or the fixture is unreadable"
    );
    assert!(
        output.is_structured_markdown,
        "PaddleOCR-VL emits markdown; is_structured_markdown should be true"
    );
}
