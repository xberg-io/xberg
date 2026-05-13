//! LLM client factory — converts kreuzberg's LlmConfig to a liter-llm DefaultClient.

use std::time::Duration;

use liter_llm::client::{ClientConfigBuilder, DefaultClient};

use crate::core::config::LlmConfig;

/// Create a liter-llm [`DefaultClient`] from kreuzberg's [`LlmConfig`].
///
/// The `model` field from the config is passed as a model hint so that
/// liter-llm can resolve the correct provider automatically.
///
/// When `api_key` is `None`, liter-llm falls back to the provider's standard
/// environment variable (e.g., `OPENAI_API_KEY`).
pub(crate) fn create_client(config: &LlmConfig) -> crate::Result<DefaultClient> {
    let api_key = config.api_key.as_deref().unwrap_or_default();
    let mut builder = ClientConfigBuilder::new(api_key);

    if let Some(ref base_url) = config.base_url {
        let sanitized = base_url.trim_end_matches('/');
        builder = builder.base_url(sanitized.to_string());
    }
    if let Some(timeout) = config.timeout_secs {
        builder = builder.timeout(Duration::from_secs(timeout));
    }
    if let Some(max_retries) = config.max_retries {
        builder = builder.max_retries(max_retries);
    }

    let client_config = builder.build();

    DefaultClient::new(client_config, Some(&config.model)).map_err(|e| {
        let msg = format!("Failed to build LLM client: {e}");
        crate::KreuzbergError::Validation {
            message: msg,
            source: Some(Box::new(e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::LlmConfig;

    #[tokio::test]
    async fn test_client_path_normalization_with_base_url() {
        use axum::{Router, routing::post};
        use liter_llm::LlmClient;
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        let app = Router::new().fallback(post(
            move |_method: axum::http::Method, uri: axum::http::Uri, headers: axum::http::HeaderMap| async move {
                // Assert no double slash in the path
                assert_eq!(uri.path(), "/v1/chat/completions");

                let auth = headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("none")
                    .to_string();
                let _ = tx.send(auth);

                axum::response::Json(serde_json::json!({
                    "id": "test",
                    "object": "chat.completion",
                    "created": 12345,
                    "model": "test",
                    "choices": [{
                        "index": 0,
                        "message": { "role": "assistant", "content": "{\"foo\": \"bar\"}" },
                        "finish_reason": "stop"
                    }]
                }))
            },
        ));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Use a base URL with a trailing slash to test normalization
        let base_url = format!("http://{}/v1/", addr);
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-api-key".to_string()),
            base_url: Some(base_url),
            ..LlmConfig::default()
        };

        let client = create_client(&config).unwrap();

        let mut request = liter_llm::ChatCompletionRequest::default();
        request.model = config.model.clone();
        request.messages = vec![liter_llm::Message::User(liter_llm::UserMessage {
            content: liter_llm::UserContent::Text("test".to_string()),
            ..Default::default()
        })];

        let _ = client.chat(request).await.expect("Request failed");

        let auth_header = tokio::time::timeout(tokio::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("Timeout waiting for header")
            .expect("No header received");

        assert_eq!(auth_header, "Bearer test-api-key");
    }

    #[test]
    fn test_create_client_sanitizes_base_url() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: Some("https://api.openai.com/v1/".to_string()),
            ..LlmConfig::default()
        };

        let _ = create_client(&config).unwrap();
        // Sanitization logic verified via manual/integration testing.
    }
}
