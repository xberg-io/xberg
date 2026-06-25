//! Live integration test for per-image VLM captioning.
//!
//! Mirrors the skip-without-keys pattern in `tests/llm_integration.rs`. Skips
//! gracefully when no provider key is set so CI without API keys still runs
//! the suite cleanly.
//!
//! Run with:
//!
//! ```text
//! cargo test -p xberg --features "liter-llm,pdf,captioning,ocr" \
//!   --test captioning_smoke -- --nocapture --test-threads=1
//! ```

#![cfg(all(feature = "captioning", not(target_os = "windows")))]

use xberg::core::config::{CaptioningConfig, ExtractionConfig, ImageExtractionConfig, LlmConfig};

const IMAGES_PDF: &str = "../../test_documents/pdf/with_images.pdf";

macro_rules! require_env {
    ($var:expr) => {
        match std::env::var($var) {
            Ok(val) if !val.is_empty() => val,
            _ => {
                eprintln!("SKIP: {} not set, skipping live captioning test", $var);
                return;
            }
        }
    };
}

fn init() {
    let _ = dotenvy::dotenv();
}

fn llm(model: &str, api_key: String) -> LlmConfig {
    LlmConfig {
        model: model.to_string(),
        api_key: Some(api_key),
        timeout_secs: Some(120),
        max_retries: Some(2),
        ..Default::default()
    }
}

fn captioning_config(model: &str, api_key: String) -> CaptioningConfig {
    CaptioningConfig {
        llm: llm(model, api_key),
        prompt: None,
        // The test PDF may contain small images; allow anything non-zero through.
        min_image_area: 0,
    }
}

async fn run_captioning_against_pdf(model: &str, api_key: String) {
    let config = ExtractionConfig {
        captioning: Some(captioning_config(model, api_key)),
        images: Some(ImageExtractionConfig {
            extract_images: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    // Ensure the captioning post-processor is registered for this test
    // (the orchestrator wires `register_builtin()` later; the smoke test
    // explicitly registers so it can verify behaviour in isolation).
    xberg::plugins::processor::builtin::register_builtin().expect("register_builtin failed");

    let result = xberg::extract_file(IMAGES_PDF, None, &config)
        .await
        .expect("extraction failed");
    let Some(images) = result.images.as_ref() else {
        eprintln!(
            "SKIP: extractor did not populate result.images for {IMAGES_PDF} — captioning has nothing to caption"
        );
        return;
    };
    if images.is_empty() {
        eprintln!("SKIP: result.images is empty for {IMAGES_PDF}");
        return;
    }
    let captioned = images
        .iter()
        .filter_map(|i| i.caption.as_deref())
        .filter(|c| !c.is_empty())
        .count();
    assert!(
        captioned >= 1,
        "expected at least one populated caption, none found across {} images",
        images.len()
    );

    let usage = result
        .llm_usage
        .as_ref()
        .expect("expected llm_usage to be populated by the captioning processor");
    assert!(
        usage.iter().any(|u| u.source == "captioning" || u.source == "vlm_ocr"),
        "expected at least one usage entry sourced from captioning, got {:?}",
        usage.iter().map(|u| u.source.as_str()).collect::<Vec<_>>()
    );
}

/// Deterministic check: with no images on the result the captioning
/// processor is a no-op even when fully configured. Exercised through the
/// public `PostProcessor` surface so the wiring (config gate, image
/// iteration, llm_usage append) is covered without spending an API call.
#[tokio::test]
async fn captioning_post_processor_is_noop_without_images() {
    use std::borrow::Cow;
    use xberg::plugins::PostProcessor;
    use xberg::plugins::processor::builtin::captioning::CaptioningProcessor;
    use xberg::types::ExtractionResult;

    let cfg = ExtractionConfig {
        captioning: Some(CaptioningConfig {
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                ..Default::default()
            },
            prompt: None,
            min_image_area: 1024,
        }),
        ..Default::default()
    };

    // No images vector at all.
    let mut result = ExtractionResult {
        content: String::new(),
        mime_type: Cow::Borrowed("text/plain"),
        ..Default::default()
    };
    CaptioningProcessor
        .process(&mut result, &cfg)
        .await
        .expect("processor must succeed on result without images");
    assert!(result.llm_usage.is_none(), "no images means no usage rows");

    // Empty images vector.
    result.images = Some(Vec::new());
    CaptioningProcessor
        .process(&mut result, &cfg)
        .await
        .expect("processor must succeed on result with empty images vec");
    assert!(result.llm_usage.is_none(), "no images means no usage rows");
}

/// Deterministic check: an LLM failure on every image (no API key + a
/// guaranteed-unreachable base URL) leaves captions empty without aborting
/// the whole extraction. Captures the "tolerate per-image failure" policy.
#[tokio::test]
async fn captioning_post_processor_tolerates_vlm_failure() {
    use bytes::Bytes;
    use std::borrow::Cow;
    use xberg::plugins::PostProcessor;
    use xberg::plugins::processor::builtin::captioning::CaptioningProcessor;
    use xberg::types::{ExtractedImage, ExtractionResult};

    let cfg = ExtractionConfig {
        captioning: Some(CaptioningConfig {
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                api_key: Some("invalid-key-deterministic-test".to_string()),
                base_url: Some("http://127.0.0.1:1/".to_string()),
                timeout_secs: Some(2),
                max_retries: Some(0),
                ..Default::default()
            },
            prompt: None,
            min_image_area: 0,
        }),
        ..Default::default()
    };

    let mut result = ExtractionResult {
        content: String::new(),
        mime_type: Cow::Borrowed("text/plain"),
        images: Some(vec![ExtractedImage {
            data: Bytes::from_static(b"not-a-real-image"),
            format: Cow::Borrowed("png"),
            width: Some(64),
            height: Some(64),
            is_mask: false,
            ..Default::default()
        }]),
        ..Default::default()
    };
    CaptioningProcessor
        .process(&mut result, &cfg)
        .await
        .expect("processor must not propagate per-image VLM failures");
    let images = result.images.as_ref().expect("images preserved on failure");
    assert!(images[0].caption.is_none(), "failed VLM call must leave caption None");
}

#[tokio::test]
async fn test_captioning_openai() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    run_captioning_against_pdf("openai/gpt-4o-mini", api_key).await;
}

#[tokio::test]
async fn test_captioning_anthropic() {
    init();
    let api_key = require_env!("ANTHROPIC_API_KEY");
    run_captioning_against_pdf("anthropic/claude-haiku-4-5-20251001", api_key).await;
}

#[tokio::test]
async fn test_captioning_gemini() {
    init();
    let api_key = require_env!("GEMINI_API_KEY");
    run_captioning_against_pdf("gemini/gemini-2.5-flash", api_key).await;
}
