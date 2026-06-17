//! Vision LLM request/response adapter over `liter-llm`.
//!
//! Builds a multimodal `ChatCompletionRequest` (system prompt + optional user text +
//! per-page PNG images) with a `ResponseFormat::JsonSchema` constraint and sends it
//! via the shared [`crate::llm::client::create_client`] seam.  The parsed JSON plus
//! captured [`crate::types::LlmUsage`] are returned as [`VisionResponse`].
//!
//! `liter_llm` types are intentionally kept off the public surface; callers only see
//! the types defined in this module.

use base64::Engine as _;
use liter_llm::{
    ChatCompletionRequest, LlmClient, Message, UserContent, UserMessage,
    types::{ContentPart, ImageDetail, ImageUrl, JsonSchemaFormat, ResponseFormat, SystemMessage},
};

use super::PageImage;
use crate::types::LlmUsage;

// ── Request / Response ───────────────────────────────────────────────────────

/// Caller-supplied parameters for a single vision LLM call.
#[derive(Debug, Clone)]
pub struct VisionRequest {
    /// System-level instruction for the model.
    pub system_prompt: String,
    /// Optional leading user-turn text (e.g. a short document excerpt).
    pub user_text: Option<String>,
    /// Rendered document pages to include as inline PNG data URIs.
    pub images: Vec<PageImage>,
    /// JSON Schema the model must conform to (passed via `response_format`).
    pub response_schema: serde_json::Value,
    /// Schema name label (sent to providers that distinguish multiple schemas).
    pub response_schema_name: String,
    /// Maximum completion tokens requested.
    pub max_output_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
    /// Model identifier (e.g. `"openai/gpt-4o"`).
    pub model: String,
}

/// Parsed result of a vision LLM call.
#[derive(Debug, Clone)]
pub struct VisionResponse {
    /// The model's structured JSON output (already parsed from the message string).
    pub content: serde_json::Value,
    /// Token usage and cost metadata.
    pub usage: LlmUsage,
}

// ── Public entry point ───────────────────────────────────────────────────────

/// Send a single vision LLM call and return the parsed structured output.
///
/// The caller is responsible for constructing an appropriate [`liter_llm::client::DefaultClient`]
/// (typically via [`crate::llm::client::create_client`]) and passing it here so that the
/// transport can be tested with a mock server without touching `LlmConfig`.
///
/// # Errors
///
/// Returns [`super::StructuredError::Vision`] for any transport failure, non-success
/// HTTP status, missing/empty response content, or JSON parse errors.
pub async fn call(
    client: &liter_llm::client::DefaultClient,
    request: VisionRequest,
) -> Result<VisionResponse, super::StructuredError> {
    // ── Build multimodal user-turn content ───────────────────────────────────
    let mut parts: Vec<ContentPart> = Vec::new();

    if let Some(text) = request.user_text {
        parts.push(ContentPart::Text { text });
    }

    for page in &request.images {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&page.png_bytes);
        let data_url = format!("data:image/png;base64,{b64}");
        parts.push(ContentPart::ImageUrl {
            image_url: ImageUrl {
                url: data_url,
                detail: Some(ImageDetail::Auto),
            },
        });
    }

    let user_content = if parts.is_empty() {
        UserContent::Text(String::new())
    } else {
        UserContent::Parts(parts)
    };

    // ── Assemble messages ────────────────────────────────────────────────────
    let messages = vec![
        Message::System(SystemMessage {
            content: request.system_prompt,
            name: None,
        }),
        Message::User(UserMessage {
            content: user_content,
            name: None,
        }),
    ];

    // ── Build ChatCompletionRequest ──────────────────────────────────────────
    let chat_request = ChatCompletionRequest {
        model: request.model.clone(),
        messages,
        max_tokens: Some(request.max_output_tokens as u64),
        temperature: Some(request.temperature as f64),
        response_format: Some(ResponseFormat::JsonSchema {
            json_schema: JsonSchemaFormat {
                name: request.response_schema_name,
                description: None,
                schema: request.response_schema,
                strict: Some(true),
            },
        }),
        ..Default::default()
    };

    // ── Send request ─────────────────────────────────────────────────────────
    let response = client.chat(chat_request).await.map_err(|e| {
        super::StructuredError::Vision(format!(
            "vision call transport error (model={}): {e}",
            request.model
        ))
    })?;

    // ── Extract usage ────────────────────────────────────────────────────────
    let raw_usage = crate::llm::usage::extract_usage_from_chat(&response, "structured_extraction");
    let usage = raw_usage.unwrap_or_else(|| LlmUsage {
        model: response.model.clone(),
        source: "structured_extraction".to_string(),
        input_tokens: None,
        output_tokens: None,
        total_tokens: None,
        estimated_cost: None,
        finish_reason: None,
    });

    // ── Parse content ─────────────────────────────────────────────────────────
    let text = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
        .ok_or_else(|| {
            super::StructuredError::Vision(format!(
                "vision response missing message content (model={}, {} choices)",
                request.model,
                response.choices.len()
            ))
        })?;

    let content: serde_json::Value = serde_json::from_str(text).map_err(|e| {
        super::StructuredError::Vision(format!(
            "vision response did not parse as valid JSON (model={}): {e}",
            request.model
        ))
    })?;

    Ok(VisionResponse { content, usage })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::core::config::LlmConfig;
    use crate::llm::client::create_client;

    fn sample_request() -> VisionRequest {
        VisionRequest {
            system_prompt: "Extract structured data.".into(),
            user_text: Some("Document follows.".into()),
            images: vec![PageImage {
                page_number: 1,
                png_bytes: vec![0x89, 0x50, 0x4e, 0x47], // PNG magic bytes
            }],
            response_schema: json!({"type": "object", "properties": {"foo": {"type": "string"}}}),
            response_schema_name: "foo_schema".into(),
            max_output_tokens: 1024,
            temperature: 0.0,
            model: "openai/gpt-4o".into(),
        }
    }

    fn stub_chat_completion_body(content_str: &str) -> serde_json::Value {
        json!({
            "id": "test-id",
            "object": "chat.completion",
            "created": 0,
            "model": "openai/gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content_str
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })
    }

    /// Happy path: assistant returns a JSON string; `call` parses it and captures usage.
    #[tokio::test]
    async fn call_posts_to_chat_completions_and_parses_json() {
        let server = MockServer::start().await;

        let stub_body = stub_chat_completion_body(r#"{"foo":"bar"}"#);
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&stub_body))
            .mount(&server)
            .await;

        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: Some(server.uri()),
            ..LlmConfig::default()
        };
        let client = create_client(&config).expect("client must build");

        let resp = call(&client, sample_request()).await.expect("call must succeed");

        assert_eq!(resp.content, json!({"foo": "bar"}));
        assert_eq!(resp.usage.total_tokens, Some(15));
        assert_eq!(resp.usage.source, "structured_extraction");
    }

    /// Verify that a 5xx response propagates as `StructuredError::Vision`.
    #[tokio::test]
    async fn call_propagates_5xx_as_vision_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503).set_body_string("upstream unavailable"))
            .mount(&server)
            .await;

        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: Some(server.uri()),
            ..LlmConfig::default()
        };
        let client = create_client(&config).expect("client must build");

        let err = call(&client, sample_request())
            .await
            .expect_err("5xx must produce an error");
        assert!(
            matches!(err, super::super::StructuredError::Vision(_)),
            "expected Vision error, got: {err:?}"
        );
    }

    /// Verify that un-parseable assistant content maps to `StructuredError::Vision`.
    #[tokio::test]
    async fn call_returns_vision_error_on_invalid_json_content() {
        let server = MockServer::start().await;

        let stub_body = stub_chat_completion_body("not-valid-json");
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&stub_body))
            .mount(&server)
            .await;

        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: Some(server.uri()),
            ..LlmConfig::default()
        };
        let client = create_client(&config).expect("client must build");

        let err = call(&client, sample_request())
            .await
            .expect_err("invalid JSON must produce an error");
        assert!(
            matches!(err, super::super::StructuredError::Vision(_)),
            "expected Vision error, got: {err:?}"
        );
    }

    /// Requests with no user_text and no images still succeed when the server returns valid JSON.
    #[tokio::test]
    async fn call_handles_request_with_no_user_text_and_no_images() {
        let server = MockServer::start().await;

        let stub_body = stub_chat_completion_body(r#"{"result":42}"#);
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&stub_body))
            .mount(&server)
            .await;

        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: Some(server.uri()),
            ..LlmConfig::default()
        };
        let client = create_client(&config).expect("client must build");

        let request = VisionRequest {
            user_text: None,
            images: vec![],
            ..sample_request()
        };
        let resp = call(&client, request).await.expect("call must succeed");
        assert_eq!(resp.content, json!({"result": 42}));
    }
}
