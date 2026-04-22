//! Live integration tests for liter-llm features.
//!
//! These tests require real API keys (loaded from `.env` at the workspace root).
//! Tests skip gracefully when the required key is not set.
//!
//! Run with: `cargo test -p kreuzberg --features "liter-llm,embeddings,pdf" --test llm_integration -- --nocapture --test-threads=1`
//!
//! Use `--test-threads=1` to avoid provider rate limiting when running all tests.

#![cfg(feature = "liter-llm")]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::config::llm::{LlmConfig, StructuredExtractionConfig};
use serde_json::json;

/// Skip a test if the named env var is not set or empty.
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

fn make_llm_config(model: &str, api_key: String) -> LlmConfig {
    LlmConfig {
        model: model.to_string(),
        api_key: Some(api_key),
        base_url: None,
        timeout_secs: Some(120),
        max_retries: Some(2),
        temperature: None,
        max_tokens: None,
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

async fn extract_memo_text() -> String {
    kreuzberg::extract_file(
        "../../test_documents/pdf/fake_memo.pdf",
        None,
        &ExtractionConfig::default(),
    )
    .await
    .expect("Failed to extract fake_memo.pdf")
    .content
}

// ---------------------------------------------------------------------------
// VLM OCR tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_vlm_ocr_openai() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    let config = make_llm_config("openai/gpt-4o-mini", api_key);
    let image_bytes = std::fs::read("../../test_documents/images/test_hello_world.png").unwrap();
    let (result, _usage) = kreuzberg::llm::vlm_ocr::vlm_ocr(&image_bytes, "image/png", "eng", &config, None)
        .await
        .unwrap();
    assert!(!result.is_empty(), "VLM OCR returned empty string");
    assert!(
        result.to_lowercase().contains("hello"),
        "Expected 'hello' in OCR result, got: {result}"
    );
}

#[tokio::test]
async fn test_vlm_ocr_anthropic() {
    init();
    let api_key = require_env!("ANTHROPIC_API_KEY");
    let config = make_llm_config("anthropic/claude-sonnet-4-20250514", api_key);
    let image_bytes = std::fs::read("../../test_documents/images/test_hello_world.png").unwrap();
    let (result, _usage) = kreuzberg::llm::vlm_ocr::vlm_ocr(&image_bytes, "image/png", "eng", &config, None)
        .await
        .unwrap();
    assert!(!result.is_empty(), "VLM OCR returned empty string");
    assert!(
        result.to_lowercase().contains("hello"),
        "Expected 'hello' in OCR result, got: {result}"
    );
}

#[tokio::test]
async fn test_vlm_ocr_gemini() {
    init();
    let api_key = require_env!("GEMINI_API_KEY");
    let config = make_llm_config("gemini/gemini-2.5-flash", api_key);
    let image_bytes = std::fs::read("../../test_documents/images/test_hello_world.png").unwrap();
    let (result, _usage) = kreuzberg::llm::vlm_ocr::vlm_ocr(&image_bytes, "image/png", "eng", &config, None)
        .await
        .unwrap();
    assert!(!result.is_empty(), "VLM OCR returned empty string");
    assert!(
        result.to_lowercase().contains("hello"),
        "Expected 'hello' in OCR result, got: {result}"
    );
}

// ---------------------------------------------------------------------------
// LLM Embedding tests
// ---------------------------------------------------------------------------

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn test_llm_embed_openai() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    let config = kreuzberg::core::config::processing::EmbeddingConfig {
        model: kreuzberg::core::config::processing::EmbeddingModelType::Llm {
            llm: make_llm_config("openai/text-embedding-3-small", api_key),
        },
        normalize: true,
        batch_size: 32,
        show_download_progress: false,
        cache_dir: None,
        acceleration: None,
    };
    let texts = vec!["Hello, world!".to_string(), "Rust is great".to_string()];
    let result = kreuzberg::embed_texts_async(texts, &config).await.unwrap();
    assert_eq!(result.len(), 2, "Expected 2 embeddings");
    assert!(!result[0].is_empty(), "Embedding vector is empty");
    assert!(
        result[0].len() > 100,
        "Embedding dimension too small: {}",
        result[0].len()
    );
}

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn test_llm_embed_mistral() {
    init();
    let api_key = require_env!("MISTRAL_API_KEY");
    let config = kreuzberg::core::config::processing::EmbeddingConfig {
        model: kreuzberg::core::config::processing::EmbeddingModelType::Llm {
            llm: make_llm_config("mistral/mistral-embed", api_key),
        },
        normalize: true,
        batch_size: 32,
        show_download_progress: false,
        cache_dir: None,
        acceleration: None,
    };
    let texts = vec!["Hello, world!".to_string()];
    let result = kreuzberg::embed_texts_async(texts, &config).await.unwrap();
    assert_eq!(result.len(), 1, "Expected 1 embedding");
    assert!(!result[0].is_empty(), "Embedding vector is empty");
}

// ---------------------------------------------------------------------------
// Structured Extraction tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_structured_extraction_openai() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    let text = extract_memo_text().await;
    let config = StructuredExtractionConfig {
        schema: memo_schema(),
        schema_name: "memo_data".to_string(),
        schema_description: Some("Extract memo metadata".to_string()),
        strict: true,
        prompt: None,
        llm: make_llm_config("openai/gpt-4o-mini", api_key),
    };
    let (result, _usage) = kreuzberg::llm::structured::extract_structured(&text, &config)
        .await
        .unwrap();
    assert!(result.is_object(), "Expected JSON object, got: {result}");
    assert!(
        result.get("title").is_some(),
        "Expected 'title' field in result: {result}"
    );
}

#[tokio::test]
async fn test_structured_extraction_anthropic() {
    init();
    let api_key = require_env!("ANTHROPIC_API_KEY");
    let text = extract_memo_text().await;
    let config = StructuredExtractionConfig {
        schema: memo_schema(),
        schema_name: "memo_data".to_string(),
        schema_description: None,
        strict: false,
        prompt: None,
        llm: make_llm_config("anthropic/claude-sonnet-4-20250514", api_key),
    };
    let (result, _usage) = kreuzberg::llm::structured::extract_structured(&text, &config)
        .await
        .unwrap();
    assert!(result.is_object(), "Expected JSON object");
}

#[tokio::test]
async fn test_structured_extraction_gemini() {
    init();
    let api_key = require_env!("GEMINI_API_KEY");
    let text = extract_memo_text().await;
    // Schema sanitization now strips additionalProperties automatically for Gemini.
    let config = StructuredExtractionConfig {
        schema: memo_schema(),
        schema_name: "memo_data".to_string(),
        schema_description: None,
        strict: false,
        prompt: None,
        llm: make_llm_config("gemini/gemini-2.5-flash", api_key),
    };
    let (result, _usage) = kreuzberg::llm::structured::extract_structured(&text, &config)
        .await
        .unwrap();
    assert!(result.is_object(), "Expected JSON object");
}

#[tokio::test]
async fn test_structured_extraction_custom_prompt() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    let text = extract_memo_text().await;
    let config = StructuredExtractionConfig {
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
            "Analyze this document and return statistics.\n\nDocument:\n{{ content }}\n\nReturn JSON with word_count and language.".to_string()
        ),
        llm: make_llm_config("openai/gpt-4o-mini", api_key),
    };
    let (result, _usage) = kreuzberg::llm::structured::extract_structured(&text, &config)
        .await
        .unwrap();
    assert!(result.is_object(), "Expected JSON object");
    assert!(result.get("word_count").is_some(), "Missing word_count");
    assert!(result.get("language").is_some(), "Missing language");
}

// ---------------------------------------------------------------------------
// Full pipeline integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_structured_extraction_pipeline() {
    init();
    let api_key = require_env!("OPENAI_API_KEY");
    let config = ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            schema: memo_schema(),
            schema_name: "memo_data".to_string(),
            schema_description: None,
            strict: true,
            prompt: None,
            llm: make_llm_config("openai/gpt-4o-mini", api_key),
        }),
        ..Default::default()
    };
    let result = kreuzberg::extract_file("../../test_documents/pdf/fake_memo.pdf", None, &config)
        .await
        .unwrap();
    assert!(
        result.structured_output.is_some(),
        "Expected structured_output to be populated"
    );
    let output = result.structured_output.unwrap();
    assert!(output.is_object(), "Expected JSON object in structured_output");
}
