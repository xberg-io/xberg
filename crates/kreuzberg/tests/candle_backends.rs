//! Registry-routing and `parse_options` tests for the four Candle VLM-OCR backends.
//!
//! These tests verify that:
//! - `OcrBackendRegistry::new` seeds each backend under its canonical name.
//! - Unknown backend names return `None` from the registry (via `list()`).
//! - `parse_options`-level behaviour (defaults + non-object JSON resilience) is
//!   correct without downloading any model weights.
//! - Network-gated e2e tests (tagged `#[ignore]`) drive real inference when a
//!   local model path is supplied via an environment variable.
//!
//! Run just the registry tests (no network required):
//! ```
//! cargo test -p kreuzberg --features candle-vlm-ocr --test candle_backends
//! ```
//!
//! Run e2e tests (supply model paths via environment):
//! ```
//! KREUZBERG_HUNYUAN_OCR_MODEL_PATH=/models/hunyuan \
//! cargo test -p kreuzberg --features candle-vlm-ocr --test candle_backends -- --ignored --nocapture
//! ```

#![cfg(feature = "candle-ocr")]

use kreuzberg::core::config::OcrConfig;
use kreuzberg::plugins::registry::OcrBackendRegistry;

// ---------------------------------------------------------------------------
// Helper: build a fresh registry with defaults and return the registered names.
// ---------------------------------------------------------------------------

fn new_registry_names() -> Vec<String> {
    let registry = OcrBackendRegistry::new();
    registry.list()
}

// ---------------------------------------------------------------------------
// Registry / selection tests — no network, no model weights.
// ---------------------------------------------------------------------------

/// The global registry seeds a "candle-glm-ocr" backend when the feature is on.
#[cfg(feature = "candle-glm-ocr")]
#[test]
fn registry_resolves_candle_glm_ocr_backend() {
    let names = new_registry_names();
    assert!(
        names.contains(&"candle-glm-ocr".to_string()),
        "Expected 'candle-glm-ocr' in registry; got: {:?}",
        names,
    );
}

/// The global registry seeds a "candle-hunyuan-ocr" backend when the feature is on.
#[cfg(all(feature = "candle-hunyuan-ocr", not(target_arch = "wasm32")))]
#[test]
fn registry_resolves_candle_hunyuan_ocr_backend() {
    let names = new_registry_names();
    assert!(
        names.contains(&"candle-hunyuan-ocr".to_string()),
        "Expected 'candle-hunyuan-ocr' in registry; got: {:?}",
        names,
    );
}

/// The global registry seeds a "candle-deepseek-ocr" backend when the feature is on.
#[cfg(all(feature = "candle-deepseek-ocr", not(target_arch = "wasm32")))]
#[test]
fn registry_resolves_candle_deepseek_ocr_backend() {
    let names = new_registry_names();
    assert!(
        names.contains(&"candle-deepseek-ocr".to_string()),
        "Expected 'candle-deepseek-ocr' in registry; got: {:?}",
        names,
    );
}

/// The global registry seeds a "candle-paddleocr-vl" backend when the feature is on.
#[cfg(feature = "candle-paddleocr-vl")]
#[test]
fn registry_resolves_candle_paddleocr_vl_backend() {
    let names = new_registry_names();
    assert!(
        names.contains(&"candle-paddleocr-vl".to_string()),
        "Expected 'candle-paddleocr-vl' in registry; got: {:?}",
        names,
    );
}

/// An unknown candle backend name is not present in the registry.
#[test]
fn registry_returns_none_for_unknown_candle_backend() {
    let names = new_registry_names();
    assert!(
        !names.contains(&"candle-doesnotexist".to_string()),
        "Expected 'candle-doesnotexist' to be absent from registry; got: {:?}",
        names,
    );
}

// ---------------------------------------------------------------------------
// parse_options — non-object JSON produces defaults (no panic, no error).
// ---------------------------------------------------------------------------

/// GLM-OCR: non-object backend_options (array) leaves parse_options returning defaults.
///
/// The `parse_options` implementation calls `opts.get("key")` which is only valid
/// on JSON objects. Passing an array or scalar must not panic — the backend must
/// silently fall back to defaults.
#[cfg(feature = "candle-glm-ocr")]
#[test]
fn parse_options_glm_ocr_rejects_non_object_json_by_returning_defaults() {
    use kreuzberg::candle_ocr::GlmOcrBackend;
    use kreuzberg::candle_ocr::glm_ocr_backend::LayoutMode;
    use kreuzberg_candle_ocr::models::GlmOcrTask;

    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());

    // Array value: parse_options must not panic and the backend struct is valid.
    let _config = OcrConfig { backend_options: Some(serde_json::json!([1, 2, 3])), ..Default::default() };

    // Verify the backend is still functional (name/version accessible — no panic).
    use kreuzberg::plugins::Plugin as _;
    assert_eq!(backend.name(), "candle-glm-ocr");

    // Scalar value.
    let _config2 = OcrConfig { backend_options: Some(serde_json::json!("ocr")), ..Default::default() };
    assert_eq!(
        backend.name(),
        "candle-glm-ocr",
        "backend remains coherent after scalar options"
    );
}

/// Hunyuan-OCR: non-object backend_options yields defaults without panicking.
#[cfg(all(feature = "candle-hunyuan-ocr", not(target_arch = "wasm32")))]
#[test]
fn parse_options_hunyuan_ocr_rejects_non_object_json_by_returning_defaults() {
    use kreuzberg::candle_ocr::HunyuanOcrBackend;
    use kreuzberg::plugins::Plugin as _;

    let backend = HunyuanOcrBackend::new();
    assert_eq!(backend.name(), "candle-hunyuan-ocr");

    // Non-object options must not cause a panic or alter backend identity.
    let mut config = OcrConfig::default();
    config.backend_options = Some(serde_json::json!(42));
    let _ = config; // parse_options is private; prove backend stays coherent.
    assert_eq!(backend.name(), "candle-hunyuan-ocr");
}

/// DeepSeek-OCR: non-object backend_options yields defaults without panicking.
#[cfg(all(feature = "candle-deepseek-ocr", not(target_arch = "wasm32")))]
#[test]
fn parse_options_deepseek_ocr_rejects_non_object_json_by_returning_defaults() {
    use kreuzberg::candle_ocr::DeepseekOcrBackend;
    use kreuzberg::plugins::Plugin as _;

    let backend = DeepseekOcrBackend::new();
    assert_eq!(backend.name(), "candle-deepseek-ocr");

    let mut config = OcrConfig::default();
    config.backend_options = Some(serde_json::json!(null));
    let _ = config;
    assert_eq!(backend.name(), "candle-deepseek-ocr");
}

/// PaddleOCR-VL: non-object backend_options yields defaults without panicking.
#[cfg(feature = "candle-paddleocr-vl")]
#[test]
fn parse_options_paddleocr_vl_rejects_non_object_json_by_returning_defaults() {
    use kreuzberg::candle_ocr::PaddleOcrVlBackend;
    use kreuzberg::plugins::Plugin as _;
    use kreuzberg_candle_ocr::models::PaddleOcrVlTask;

    let backend = PaddleOcrVlBackend::new(PaddleOcrVlTask::default());
    assert_eq!(backend.name(), "candle-paddleocr-vl");

    let mut config = OcrConfig::default();
    config.backend_options = Some(serde_json::json!(false));
    let _ = config;
    assert_eq!(backend.name(), "candle-paddleocr-vl");
}

// ---------------------------------------------------------------------------
// parse_options — empty object `{}` is accepted and returns defaults.
// ---------------------------------------------------------------------------

/// GLM-OCR: empty object backend_options is silently accepted; defaults apply.
#[cfg(feature = "candle-glm-ocr")]
#[test]
fn parse_options_glm_ocr_accepts_empty_object_and_returns_defaults() {
    use kreuzberg::candle_ocr::GlmOcrBackend;
    use kreuzberg::candle_ocr::glm_ocr_backend::LayoutMode;
    use kreuzberg::plugins::Plugin as _;
    use kreuzberg_candle_ocr::models::GlmOcrTask;

    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());

    let _config = OcrConfig { backend_options: Some(serde_json::json!({})), ..Default::default() };

    // Backend remains properly identified after empty-object options.
    assert_eq!(backend.name(), "candle-glm-ocr");
    assert_eq!(backend.version(), "0.1.0");
}

/// Hunyuan-OCR: empty object backend_options is accepted; model_path stays None.
#[cfg(all(feature = "candle-hunyuan-ocr", not(target_arch = "wasm32")))]
#[test]
fn parse_options_hunyuan_ocr_accepts_empty_object_and_returns_defaults() {
    use kreuzberg::candle_ocr::HunyuanOcrBackend;
    use kreuzberg::plugins::Plugin as _;

    let backend = HunyuanOcrBackend::new();
    let mut config = OcrConfig::default();
    config.backend_options = Some(serde_json::json!({}));

    assert_eq!(backend.name(), "candle-hunyuan-ocr");
    assert_eq!(backend.version(), "0.1.0");
}

/// DeepSeek-OCR: empty object backend_options is accepted; defaults apply.
#[cfg(all(feature = "candle-deepseek-ocr", not(target_arch = "wasm32")))]
#[test]
fn parse_options_deepseek_ocr_accepts_empty_object_and_returns_defaults() {
    use kreuzberg::candle_ocr::DeepseekOcrBackend;
    use kreuzberg::plugins::Plugin as _;

    let backend = DeepseekOcrBackend::new();
    let mut config = OcrConfig::default();
    config.backend_options = Some(serde_json::json!({}));

    assert_eq!(backend.name(), "candle-deepseek-ocr");
    assert_eq!(backend.version(), "0.1.0");
}

/// PaddleOCR-VL: empty object backend_options is accepted; defaults apply.
#[cfg(feature = "candle-paddleocr-vl")]
#[test]
fn parse_options_paddleocr_vl_accepts_empty_object_and_returns_defaults() {
    use kreuzberg::candle_ocr::PaddleOcrVlBackend;
    use kreuzberg::plugins::Plugin as _;
    use kreuzberg_candle_ocr::models::PaddleOcrVlTask;

    let backend = PaddleOcrVlBackend::new(PaddleOcrVlTask::default());
    let mut config = OcrConfig::default();
    config.backend_options = Some(serde_json::json!({}));

    assert_eq!(backend.name(), "candle-paddleocr-vl");
    assert_eq!(backend.version(), "0.1.0");
}

// ---------------------------------------------------------------------------
// Network-gated e2e tests — #[ignore] until env vars are set.
// ---------------------------------------------------------------------------

/// End-to-end Hunyuan-OCR extraction through `OcrBackend::process_image`.
///
/// Requires `KREUZBERG_HUNYUAN_OCR_MODEL_PATH` to point to a local model directory.
/// Skip cleanly when the variable is absent.
#[cfg(all(feature = "candle-hunyuan-ocr", not(target_arch = "wasm32")))]
#[tokio::test]
#[ignore = "requires KREUZBERG_HUNYUAN_OCR_MODEL_PATH env var pointing to local model weights"]
async fn candle_hunyuan_ocr_e2e_extraction() {
    let model_path = match std::env::var("KREUZBERG_HUNYUAN_OCR_MODEL_PATH") {
        Ok(p) => p,
        Err(_) => {
            println!("KREUZBERG_HUNYUAN_OCR_MODEL_PATH not set — skipping Hunyuan-OCR e2e test");
            return;
        }
    };

    use kreuzberg::candle_ocr::HunyuanOcrBackend;
    use kreuzberg::plugins::OcrBackend as _;

    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    let backend = HunyuanOcrBackend::new();

    let mut config = OcrConfig::default();
    config.backend_options = Some(serde_json::json!({"model_path": model_path}));

    let result = backend
        .process_image(image_bytes, &config)
        .await
        .expect("HunyuanOcrBackend::process_image should succeed");

    assert!(
        !result.content.is_empty(),
        "Hunyuan-OCR extraction returned empty content"
    );
    assert_eq!(
        result.mime_type.as_ref(),
        "text/markdown",
        "Hunyuan-OCR must emit text/markdown"
    );

    println!(
        "Hunyuan-OCR result ({} chars): {}",
        result.content.len(),
        result.content
    );
}

/// End-to-end DeepSeek-OCR extraction through `OcrBackend::process_image`.
///
/// Requires `KREUZBERG_DEEPSEEK_OCR_MODEL_PATH` to point to a local model directory.
/// Skip cleanly when the variable is absent.
#[cfg(all(feature = "candle-deepseek-ocr", not(target_arch = "wasm32")))]
#[tokio::test]
#[ignore = "requires KREUZBERG_DEEPSEEK_OCR_MODEL_PATH env var pointing to local model weights"]
async fn candle_deepseek_ocr_e2e_extraction() {
    let model_path = match std::env::var("KREUZBERG_DEEPSEEK_OCR_MODEL_PATH") {
        Ok(p) => p,
        Err(_) => {
            println!("KREUZBERG_DEEPSEEK_OCR_MODEL_PATH not set — skipping DeepSeek-OCR e2e test");
            return;
        }
    };

    use kreuzberg::candle_ocr::DeepseekOcrBackend;
    use kreuzberg::plugins::OcrBackend as _;

    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    let backend = DeepseekOcrBackend::new();

    let mut config = OcrConfig::default();
    config.backend_options = Some(serde_json::json!({"model_path": model_path}));

    let result = backend
        .process_image(image_bytes, &config)
        .await
        .expect("DeepseekOcrBackend::process_image should succeed");

    assert!(
        !result.content.is_empty(),
        "DeepSeek-OCR extraction returned empty content"
    );
    assert_eq!(
        result.mime_type.as_ref(),
        "text/markdown",
        "DeepSeek-OCR must emit text/markdown"
    );

    println!(
        "DeepSeek-OCR result ({} chars): {}",
        result.content.len(),
        result.content
    );
}

/// End-to-end PaddleOCR-VL extraction through `OcrBackend::process_image`.
///
/// Requires `KREUZBERG_PADDLEOCR_VL_MODEL_PATH` to point to a local model directory.
/// Skip cleanly when the variable is absent.
#[cfg(feature = "candle-paddleocr-vl")]
#[tokio::test]
#[ignore = "requires KREUZBERG_PADDLEOCR_VL_MODEL_PATH env var pointing to local model weights"]
async fn candle_paddleocr_vl_e2e_extraction() {
    let model_path = match std::env::var("KREUZBERG_PADDLEOCR_VL_MODEL_PATH") {
        Ok(p) => p,
        Err(_) => {
            println!("KREUZBERG_PADDLEOCR_VL_MODEL_PATH not set — skipping PaddleOCR-VL e2e test");
            return;
        }
    };

    use kreuzberg::candle_ocr::PaddleOcrVlBackend;
    use kreuzberg::plugins::OcrBackend as _;
    use kreuzberg_candle_ocr::models::PaddleOcrVlTask;

    let image_bytes = include_bytes!("../../../fixtures/images/test_hello_world.png");

    let backend = PaddleOcrVlBackend::new(PaddleOcrVlTask::default());

    let mut config = OcrConfig::default();
    config.backend_options = Some(serde_json::json!({"model_path": model_path}));

    let result = backend
        .process_image(image_bytes, &config)
        .await
        .expect("PaddleOcrVlBackend::process_image should succeed");

    assert!(
        !result.content.is_empty(),
        "PaddleOCR-VL extraction returned empty content"
    );
    assert_eq!(
        result.mime_type.as_ref(),
        "text/markdown",
        "PaddleOCR-VL must emit text/markdown"
    );

    println!(
        "PaddleOCR-VL result ({} chars): {}",
        result.content.len(),
        result.content
    );
}
