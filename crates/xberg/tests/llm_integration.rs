//! Live integration tests for liter-llm features.
//!
//! These tests hit real provider APIs and require API keys in the workspace
//! `.env` (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`). Each
//! test skips gracefully when its required key is missing.
//!
//! Run with:
//!
//! ```text
//! cargo test -p xberg --features "liter-llm,pdf" --test llm_integration -- --nocapture --test-threads=1
//! ```
//!
//! `--test-threads=1` keeps concurrent provider calls below rate limits.
//!
//! All tests exercise the **public** extraction surface (`extract_file` +
//! `ExtractionConfig`), matching how downstream callers (xberg-enterprise,
//! xberg-py) invoke the engine.

#![cfg(feature = "liter-llm")]

use serde_json::json;
use xberg::core::config::{ExtractionConfig, LlmConfig, OcrConfig, StructuredExtractionConfig, VlmFallbackPolicy};

const MEMO_PDF: &str = "../../test_documents/pdf/fake_memo.pdf";
const HELLO_PNG: &str = "../../test_documents/images/test_hello_world.png";
const SCANNED_PDF: &str = "../../test_documents/pdf_scanned/nougat_001_scanned.pdf";

macro_rules! require_env {
    ($var:expr) => {
        match std::env::var($var) {
            Ok(val) if !val.is_empty() => val,
            _ => {
                eprintln!("SKIP: {} not set, skipping live integration test", $var);
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

fn memo_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "title": { "type": "string" },
            "date": { "type": "string" },
            "summary": { "type": "string" }
        },
        "required": ["title", "date", "summary"],
        "additionalProperties": false
    })
}

// ----------------------------------------------------------------------------
// VLM OCR — via public extract_file with `OcrConfig { backend: "vlm", ... }`.
// ----------------------------------------------------------------------------

async fn run_vlm_ocr(model: &str, api_key: String) {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "vlm".to_string(),
            language: vec!["eng".to_string()],
            vlm_config: Some(llm(model, api_key)),
            ..Default::default()
        }),
        force_ocr: true,
        ..Default::default()
    };
    let result = xberg::extract_file(HELLO_PNG, None, &config)
        .await
        .expect("VLM OCR extraction failed");
    assert!(
        result.content.to_lowercase().contains("hello"),
        "expected 'hello' in OCR result, got: {}",
        result.content
    );
}

#[tokio::test]
async fn test_vlm_ocr_openai() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    run_vlm_ocr("openai/gpt-4o-mini", api_key).await;
}

#[tokio::test]
async fn test_vlm_ocr_anthropic() {
    init();
    let api_key = require_env!("ANTHROPIC_API_KEY");
    run_vlm_ocr("anthropic/claude-haiku-4-5-20251001", api_key).await;
}

#[tokio::test]
async fn test_vlm_ocr_gemini() {
    init();
    let api_key = require_env!("GEMINI_API_KEY");
    run_vlm_ocr("gemini/gemini-2.5-flash", api_key).await;
}

// ----------------------------------------------------------------------------
// Structured extraction — via public extract_file with
// `ExtractionConfig { structured_extraction: Some(..), .. }`.
// ----------------------------------------------------------------------------

async fn run_structured(model: &str, api_key: String, strict: bool) {
    let config = ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            schema: memo_schema(),
            schema_name: "memo_data".to_string(),
            schema_description: Some("Extract memo metadata".to_string()),
            strict,
            prompt: None,
            llm: llm(model, api_key),
        }),
        ..Default::default()
    };
    let result = xberg::extract_file(MEMO_PDF, None, &config)
        .await
        .expect("structured extraction failed");
    let output = result
        .structured_output
        .expect("expected structured_output to be populated");
    assert!(output.is_object(), "expected JSON object, got: {output}");
    assert!(output.get("title").is_some(), "expected 'title' in result: {output}");
    let usage = result
        .llm_usage
        .expect("expected llm_usage populated for a structured-extraction run");
    assert!(!usage.is_empty(), "llm_usage was Some but empty");
    assert!(
        usage.iter().any(|u| u.source == "structured_extraction"),
        "expected at least one usage entry with source=structured_extraction, got {:?}",
        usage.iter().map(|u| u.source.as_str()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_structured_extraction_openai() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    run_structured("openai/gpt-4o-mini", api_key, true).await;
}

#[tokio::test]
async fn test_structured_extraction_anthropic() {
    init();
    let api_key = require_env!("ANTHROPIC_API_KEY");
    run_structured("anthropic/claude-haiku-4-5-20251001", api_key, false).await;
}

#[tokio::test]
async fn test_structured_extraction_gemini() {
    init();
    let api_key = require_env!("GEMINI_API_KEY");
    run_structured("gemini/gemini-2.5-flash", api_key, false).await;
}

#[tokio::test]
async fn test_structured_extraction_custom_prompt() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    let config = ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            schema: json!({
                "type": "object",
                "properties": {
                    "word_count": { "type": "integer" },
                    "language": { "type": "string" }
                },
                "required": ["word_count", "language"],
                "additionalProperties": false
            }),
            schema_name: "doc_stats".to_string(),
            schema_description: None,
            strict: true,
            prompt: Some(
                "Analyze this document and return statistics.\n\n\
                 Document:\n{{ content }}\n\n\
                 Return JSON with word_count and language."
                    .to_string(),
            ),
            llm: llm("openai/gpt-4o-mini", api_key),
        }),
        ..Default::default()
    };
    let result = xberg::extract_file(MEMO_PDF, None, &config)
        .await
        .expect("structured extraction with custom prompt failed");
    let output = result.structured_output.expect("structured_output missing");
    assert!(output.is_object(), "expected JSON object: {output}");
    assert!(output.get("word_count").is_some(), "missing word_count");
    assert!(output.get("language").is_some(), "missing language");
}

// ----------------------------------------------------------------------------
// VlmFallbackPolicy — Stage 1A.
// ----------------------------------------------------------------------------

#[tokio::test]
async fn test_vlm_fallback_always_routes_to_vlm() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            vlm_fallback: VlmFallbackPolicy::Always,
            vlm_config: Some(llm("openai/gpt-4o-mini", api_key)),
            ..Default::default()
        }),
        force_ocr: true,
        // Multi-page scanned PDFs sent to VLM require more than the default 60 s.
        extraction_timeout_secs: Some(300),
        ..Default::default()
    };
    let result = xberg::extract_file(SCANNED_PDF, None, &config)
        .await
        .expect("VlmFallbackPolicy::Always extraction failed");
    assert!(
        !result.content.trim().is_empty(),
        "VlmFallbackPolicy::Always produced empty content"
    );
    let usage = result
        .llm_usage
        .expect("expected llm_usage populated when VLM fallback ran");
    assert!(
        usage.iter().any(|u| u.source == "vlm_ocr"),
        "expected vlm_ocr LlmUsage entry, got sources {:?}",
        usage.iter().map(|u| u.source.as_str()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_vlm_fallback_on_low_quality() {
    init();
    // A very-permissive threshold (close to 1.0) forces the classical pass to
    // be judged "low quality" almost always, exercising the fallback path on
    // a scanned PDF that tesseract handles imperfectly.
    let api_key = require_env!("OPENAI_API_KEY");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            vlm_fallback: VlmFallbackPolicy::OnLowQuality {
                quality_threshold: 0.95,
            },
            vlm_config: Some(llm("openai/gpt-4o-mini", api_key)),
            ..Default::default()
        }),
        force_ocr: true,
        // Multi-page scanned PDFs sent to VLM require more than the default 60 s.
        extraction_timeout_secs: Some(300),
        ..Default::default()
    };
    let result = xberg::extract_file(SCANNED_PDF, None, &config)
        .await
        .expect("VlmFallbackPolicy::OnLowQuality extraction failed");
    assert!(
        !result.content.trim().is_empty(),
        "OnLowQuality fallback produced empty content"
    );
    // The fallback may or may not fire depending on tesseract's score; only
    // assert that *some* LLM call ran when content materialised. If the
    // classical pass cleared the threshold, no LLM ran — that's also valid
    // behaviour for this configuration, so this check is best-effort.
    if let Some(usage) = result.llm_usage {
        assert!(
            usage.iter().any(|u| u.source == "vlm_ocr"),
            "llm_usage present but no vlm_ocr source: {:?}",
            usage.iter().map(|u| u.source.as_str()).collect::<Vec<_>>()
        );
    }
}

#[tokio::test]
async fn test_vlm_fallback_disabled_does_not_call_llm() {
    init();
    // Skip if Tesseract is not available in the current feature set (e.g. liter-llm
    // only, without the full `ocr` feature which pulls in the Tesseract backend).
    {
        use xberg::plugins::registry::get_ocr_backend_registry;
        let registry = get_ocr_backend_registry();
        let registry = registry.read();
        if !registry.list().iter().any(|n| n == "tesseract") {
            eprintln!("SKIP: tesseract backend not registered in this feature set");
            return;
        }
    }
    // No API key required — Disabled must not contact any provider.
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            vlm_fallback: VlmFallbackPolicy::Disabled,
            vlm_config: None,
            ..Default::default()
        }),
        force_ocr: true,
        // Tesseract OCR on the scanned-PDF fixture takes ~2 min on the slow Linux arm64 CI
        // runner; the 60 s default introduced alongside vlm_fallback would always trip.
        extraction_timeout_secs: Some(300),
        ..Default::default()
    };
    let result = xberg::extract_file(SCANNED_PDF, None, &config)
        .await
        .expect("Disabled-fallback extraction failed");
    assert!(
        result.llm_usage.as_ref().is_none_or(|u| u.is_empty()),
        "Disabled policy must not produce LLM usage records, got {:?}",
        result.llm_usage
    );
}
