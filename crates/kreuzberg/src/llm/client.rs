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
pub fn create_client(config: &LlmConfig) -> crate::Result<DefaultClient> {
    let api_key = config.api_key.as_deref().unwrap_or_default();
    let mut builder = ClientConfigBuilder::new(api_key);

    if let Some(ref base_url) = config.base_url {
        builder = builder.base_url(base_url.clone());
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
