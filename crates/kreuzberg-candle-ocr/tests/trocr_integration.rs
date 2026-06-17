//! End-to-end integration test for the candle TrOCR engine.
//!
//! Marked `#[ignore]` because it pulls ~1.5 GB of safetensors from
//! HuggingFace Hub and the autoregressive decode on CPU takes ~30 s. Run
//! explicitly with:
//!
//! ```sh
//! KREUZBERG_NETWORK_TESTS=1 cargo test -p kreuzberg-candle-ocr \
//!     --features trocr --test trocr_integration -- --ignored
//! ```
//!
//! Skipped silently when `KREUZBERG_NETWORK_TESTS` is unset, so CI can
//! invoke `cargo test -- --ignored` without unconditionally requiring
//! network and ~1.5 GB of model cache.

#![cfg(all(feature = "trocr", not(target_arch = "wasm32")))]

use std::path::PathBuf;

use candle_core::Device;
use kreuzberg_candle_ocr::models::{TrocrEngine, TrocrVariant};

const ENV_GATE: &str = "KREUZBERG_NETWORK_TESTS";

fn skip_unless_network_tests_opt_in() -> bool {
    match std::env::var(ENV_GATE) {
        Ok(v) if matches!(v.as_str(), "1" | "true" | "yes") => false,
        _ => {
            eprintln!(
                "Skipping TrOCR integration test; set {ENV_GATE}=1 to run (~1.5 GB download \
                 from HuggingFace, ~30 s on CPU)."
            );
            true
        }
    }
}

fn fixture_path(name: &str) -> PathBuf {
    // crates/kreuzberg-candle-ocr/tests/ → workspace root → test_documents/images
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/images")
        .join(name)
}

#[test]
#[ignore = "pulls ~1.5 GB from HF Hub; gated on KREUZBERG_NETWORK_TESTS=1"]
fn trocr_base_printed_recognises_single_line_fixture() {
    if skip_unless_network_tests_opt_in() {
        return;
    }

    let fixture = fixture_path("test_hello_world.png");
    let bytes = std::fs::read(&fixture).unwrap_or_else(|e| {
        panic!("failed to read fixture {}: {e}", fixture.display());
    });

    let engine = TrocrEngine::new(TrocrVariant::BasePrinted, Device::Cpu)
        .expect("TrocrEngine::new should succeed with weights cached in $HOME/.cache/huggingface");

    let output = engine
        .process_image(&bytes)
        .expect("process_image should produce a CandleOcrOutput on a single-line printed fixture");

    let content = output.content.trim();
    assert!(
        !content.is_empty(),
        "TrOCR decoded an empty string for a single-line printed fixture; either the model failed \
         to load, the decode loop is broken, or the fixture is unreadable"
    );
    assert!(
        !output.is_structured_markdown,
        "TrOCR emits plain text, not markdown; is_structured_markdown should be false"
    );
}
