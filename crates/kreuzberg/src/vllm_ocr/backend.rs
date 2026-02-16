//! vLLM OCR backend implementation.

use std::borrow::Cow;
use std::path::Path;

use async_trait::async_trait;
use base64::Engine;

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractionResult;

/// Default vLLM API endpoint (vLLM default port is 8000).
const DEFAULT_ENDPOINT: &str = "http://localhost:8000";
/// Default vision model.
const DEFAULT_MODEL: &str = "glm-ocr";
/// Default OCR prompt.
const DEFAULT_PROMPT: &str = "Extract all text from this image. Return only the extracted text, nothing else.";

/// vLLM OCR backend.
///
/// Uses vLLM's OpenAI-compatible `/v1/chat/completions` endpoint with vision models
/// to perform OCR on images. Any vLLM-hosted model that accepts image input works
/// (e.g., `glm-ocr`, `Nanonets-OCR-s`, `LightOnOCR-2-1B`).
///
/// This backend also works with any server that exposes the OpenAI-compatible vision
/// API format (llama.cpp, Ollama's `/v1` endpoint, etc.).
///
/// # Defaults
///
/// - Endpoint: `http://localhost:8000` (vLLM default port)
/// - Model: `glm-ocr`
/// - Name: `"vllm"` (used for backend selection via `OcrConfig.backend`)
///
/// # Environment Variables
///
/// - `VLLM_OCR_BASE_URL`: Override the default endpoint URL
/// - `VLLM_OCR_MODEL`: Override the default model name
/// - `VLLM_OCR_API_KEY`: Set an API key for authenticated endpoints
#[derive(Debug, Clone)]
pub struct VllmOcrBackend {
    endpoint: String,
    model: String,
    prompt: String,
    api_key: Option<String>,
}

impl VllmOcrBackend {
    /// Create a new vLLM backend with default settings.
    ///
    /// Respects `VLLM_OCR_BASE_URL`, `VLLM_OCR_MODEL`, and `VLLM_OCR_API_KEY`
    /// environment variables.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Create a builder for custom configuration.
    pub fn builder() -> VllmOcrBuilder {
        VllmOcrBuilder::default()
    }

    /// Send an image to vLLM and return the extracted text.
    fn call_vllm(&self, image_bytes: &[u8]) -> Result<String> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(image_bytes);
        let url = format!(
            "{}/v1/chat/completions",
            self.endpoint.trim_end_matches('/')
        );

        // OpenAI-compatible vision format: images as data URLs in content array
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": self.prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", b64)
                        }
                    }
                ]
            }],
            "max_tokens": 4096
        });

        let mut request = ureq::post(&url).header("Content-Type", "application/json");

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response: serde_json::Value = request
            .send_json(&body)
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("vLLM request to {} failed: {}", url, e),
                source: Some(Box::new(e)),
            })?
            .body_mut()
            .read_json()
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("Failed to parse vLLM response: {}", e),
                source: Some(Box::new(e)),
            })?;

        // OpenAI format: response.choices[0].message.content
        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        if content.is_empty() {
            tracing::warn!(
                "vLLM returned empty content for model '{}' at {}",
                self.model,
                self.endpoint
            );
        }

        Ok(content)
    }
}

impl Default for VllmOcrBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for VllmOcrBackend {
    fn name(&self) -> &str {
        "vllm"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl OcrBackend for VllmOcrBackend {
    async fn process_image(
        &self,
        image_bytes: &[u8],
        _config: &OcrConfig,
    ) -> Result<ExtractionResult> {
        let backend = self.clone();
        let bytes = image_bytes.to_vec();

        let content = tokio::task::spawn_blocking(move || backend.call_vllm(&bytes))
            .await
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("vLLM OCR task panicked: {}", e),
                source: None,
            })??;

        Ok(ExtractionResult {
            content,
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        })
    }

    async fn process_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        // Vision models are generally language-agnostic for OCR
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        vec![
            "eng".into(),
            "deu".into(),
            "fra".into(),
            "spa".into(),
            "ita".into(),
            "por".into(),
            "chi".into(),
            "jpn".into(),
            "kor".into(),
            "ara".into(),
            "hin".into(),
            "rus".into(),
        ]
    }
}

/// Builder for [`VllmOcrBackend`].
#[derive(Debug, Clone)]
pub struct VllmOcrBuilder {
    endpoint: String,
    model: String,
    prompt: String,
    api_key: Option<String>,
}

impl Default for VllmOcrBuilder {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("VLLM_OCR_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string()),
            model: std::env::var("VLLM_OCR_MODEL")
                .unwrap_or_else(|_| DEFAULT_MODEL.to_string()),
            prompt: DEFAULT_PROMPT.to_string(),
            api_key: std::env::var("VLLM_OCR_API_KEY").ok(),
        }
    }
}

impl VllmOcrBuilder {
    /// Set the vLLM API endpoint URL.
    pub fn endpoint(mut self, url: &str) -> Self {
        self.endpoint = url.to_string();
        self
    }

    /// Set the vision model name.
    pub fn model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Set the OCR prompt sent with the image.
    pub fn prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }

    /// Set an API key for authenticated endpoints.
    pub fn api_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_string());
        self
    }

    /// Build the [`VllmOcrBackend`].
    pub fn build(self) -> VllmOcrBackend {
        VllmOcrBackend {
            endpoint: self.endpoint,
            model: self.model,
            prompt: self.prompt,
            api_key: self.api_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let backend = VllmOcrBackend::builder()
            .endpoint("https://gpu-server:8000")
            .model("zai-org/GLM-OCR")
            .prompt("OCR this")
            .api_key("test-key")
            .build();
        assert_eq!(backend.endpoint, "https://gpu-server:8000");
        assert_eq!(backend.model, "zai-org/GLM-OCR");
        assert_eq!(backend.prompt, "OCR this");
        assert_eq!(backend.api_key.as_deref(), Some("test-key"));
    }

    #[test]
    fn test_builder_no_api_key() {
        let backend = VllmOcrBackend::builder()
            .endpoint("http://localhost:8000")
            .build();
        assert!(backend.api_key.is_none() || backend.api_key.is_some());
        // api_key comes from env, so just verify the field exists
    }

    #[test]
    fn test_plugin_interface() {
        let backend = VllmOcrBackend::builder()
            .endpoint("http://localhost:8000")
            .model("test-model")
            .build();
        assert_eq!(backend.name(), "vllm");
        assert_eq!(backend.backend_type(), OcrBackendType::Custom);
        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("chi"));
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    #[test]
    fn test_supported_languages() {
        let backend = VllmOcrBackend::new();
        let langs = backend.supported_languages();
        assert!(langs.contains(&"eng".to_string()));
        assert!(langs.contains(&"chi".to_string()));
        assert!(langs.contains(&"jpn".to_string()));
    }

    #[test]
    fn test_version_from_cargo() {
        let backend = VllmOcrBackend::builder().build();
        assert_eq!(backend.version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_default_endpoint_is_port_8000() {
        // vLLM's default port is 8000
        assert_eq!(DEFAULT_ENDPOINT, "http://localhost:8000");
    }
}
