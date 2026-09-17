//! Integration tests for `LlmConfig::max_response_bytes`.
//!
//! Mirrors `structured_pipeline.rs`'s wiremock-stubbed OpenAI-compatible endpoint pattern:
//! the unified public `extract(ExtractInput, &ExtractionConfig)` API drives a structured
//! extraction whose LLM client is bounded by `max_response_bytes`, so no provider API keys
//! or live network calls are required.

use serde_json::json;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use xberg::{ExtractInput, ExtractionConfig, ExtractionResult, LlmConfig, StructuredExtractionConfig, extract};

const PLAIN_TEXT_MIME: &str = "text/plain";
const PLAIN_TEXT_CONTENT: &[u8] = b"Invoice number: INV-001
Vendor: Acme Corp
Total: $42.00
This document contains enough text for the public extraction pipeline.";

/// Small enough that any real chat-completion JSON body exceeds it, but large enough to be an
/// unambiguous, deliberate cap rather than a boundary value.
const TINY_RESPONSE_CAP: usize = 32;

/// Comfortably larger than the stub response body below, so it acts as a true "cap does not
/// interfere" control.
const GENEROUS_RESPONSE_CAP: usize = 1_000_000;

fn invoice_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "invoice_number": { "type": "string" },
            "vendor": { "type": "string" },
            "total": { "type": "string" }
        },
        "required": ["invoice_number", "vendor", "total"],
        "additionalProperties": false
    })
}

fn stub_completion(content: &str) -> serde_json::Value {
    json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 0,
        "model": "openai/gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 20,
            "total_tokens": 120
        }
    })
}

fn structured_config(server_uri: &str, max_response_bytes: usize) -> ExtractionConfig {
    ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            schema: invoice_schema(),
            schema_name: "test_schema".to_string(),
            schema_description: Some("Deterministic test schema".to_string()),
            strict: true,
            prompt: None,
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                base_url: Some(server_uri.to_string()),
                timeout_secs: Some(10),
                max_retries: Some(0),
                max_response_bytes: Some(max_response_bytes),
                ..Default::default()
            },
        }),
        ..Default::default()
    }
}

async fn extract_public_structured(config: &ExtractionConfig) -> ExtractionResult {
    extract(
        ExtractInput::from_bytes(
            PLAIN_TEXT_CONTENT.to_vec(),
            PLAIN_TEXT_MIME,
            Some("invoice.txt".to_string()),
        ),
        config,
    )
    .await
    .expect("public extraction must succeed")
}

/// A response body larger than the configured `max_response_bytes` must surface as a
/// processing warning naming the configured limit, not as truncated structured output — the
/// underlying liter-llm read is aborted mid-stream rather than silently clipped.
#[tokio::test]
async fn public_structured_extraction_reports_response_byte_cap_instead_of_truncating() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_completion(
            r#"{"invoice_number":"INV-001","vendor":"Acme Corp","total":"$42.00"}"#,
        )))
        .mount(&server)
        .await;

    let config = structured_config(&server.uri(), TINY_RESPONSE_CAP);
    let output = extract_public_structured(&config).await;

    assert_eq!(output.summary.inputs, 1);
    assert_eq!(output.summary.results, 1);
    let result = &output.results[0];
    assert!(
        result.structured_output.is_none(),
        "an over-cap response must not populate structured_output, even partially"
    );
    assert!(
        result.processing_warnings.iter().any(|warning| {
            warning.source == "structured_extraction"
                && warning.message.contains("Structured extraction failed")
                && warning
                    .message
                    .contains(&format!("exceeds configured limit of {TINY_RESPONSE_CAP} bytes"))
        }),
        "expected a processing warning naming the {TINY_RESPONSE_CAP}-byte cap, got {:?}",
        result.processing_warnings
    );
}

/// Control: the same response body under a `max_response_bytes` comfortably above its size
/// must extract successfully — the cap must not interfere with an in-bounds response.
#[tokio::test]
async fn public_structured_extraction_succeeds_when_response_is_within_the_byte_cap() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_completion(
            r#"{"invoice_number":"INV-001","vendor":"Acme Corp","total":"$42.00"}"#,
        )))
        .mount(&server)
        .await;

    let config = structured_config(&server.uri(), GENEROUS_RESPONSE_CAP);
    let output = extract_public_structured(&config).await;

    assert_eq!(output.summary.inputs, 1);
    assert_eq!(output.summary.results, 1);
    assert_eq!(output.summary.errors, 0);
    assert!(
        output.errors.is_empty(),
        "unexpected public extraction errors: {:?}",
        output.errors
    );
    let result = &output.results[0];
    assert!(
        result.processing_warnings.is_empty(),
        "unexpected processing warnings: {:?}",
        result.processing_warnings
    );
    let structured_output = result
        .structured_output
        .as_ref()
        .expect("structured_output should be populated when the response is within the cap");
    assert_eq!(structured_output["invoice_number"].as_str(), Some("INV-001"));
    assert_eq!(structured_output["vendor"].as_str(), Some("Acme Corp"));
    assert_eq!(structured_output["total"].as_str(), Some("$42.00"));
}
