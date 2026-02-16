//! Ollama OCR backend implementation.

use std::borrow::Cow;
use std::path::Path;

use async_trait::async_trait;
use base64::Engine;

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractionResult;

/// Default Ollama API endpoint.
const DEFAULT_ENDPOINT: &str = "http://localhost:11434";
/// Default vision model.
const DEFAULT_MODEL: &str = "glm-ocr";
/// Default OCR prompt.
const DEFAULT_PROMPT: &str = "Extract all text from this image. Return only the extracted text, nothing else.";

/// Ollama OCR backend.
///
/// Uses Ollama's `/api/chat` endpoint with vision models to perform OCR on images.
/// Any model that accepts image input works (e.g., `glm-ocr`, `llava`, `moondream`,
/// `qwen2.5vl`).
///
/// # Defaults
///
/// - Endpoint: `http://localhost:11434`
/// - Model: `glm-ocr`
/// - Name: `"ollama"` (used for backend selection via `OcrConfig.backend`)
///
/// # Environment Variables
///
/// - `OLLAMA_HOST`: Override the default endpoint URL
/// - `OLLAMA_MODEL`: Override the default model name
#[derive(Debug, Clone)]
pub struct OllamaOcrBackend {
    endpoint: String,
    model: String,
    prompt: String,
}

impl OllamaOcrBackend {
    /// Create a new Ollama backend with default settings.
    ///
    /// Respects `OLLAMA_HOST` and `OLLAMA_MODEL` environment variables.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Create a builder for custom configuration.
    pub fn builder() -> OllamaOcrBuilder {
        OllamaOcrBuilder::default()
    }

    /// Send an image to Ollama and return the extracted text.
    fn call_ollama(&self, image_bytes: &[u8]) -> Result<String> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(image_bytes);
        let url = format!("{}/api/chat", self.endpoint.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": self.prompt,
                "images": [b64]
            }],
            "stream": false
        });

        let response: serde_json::Value = ureq::post(&url)
            .header("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("Ollama request to {} failed: {}", url, e),
                source: Some(Box::new(e)),
            })?
            .body_mut()
            .read_json()
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("Failed to parse Ollama response: {}", e),
                source: Some(Box::new(e)),
            })?;

        let content = response["message"]["content"].as_str().unwrap_or("").trim().to_string();

        if content.is_empty() {
            tracing::warn!(
                "Ollama returned empty content for model '{}' at {}",
                self.model,
                self.endpoint
            );
        }

        Ok(content)
    }
}

impl Default for OllamaOcrBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for OllamaOcrBackend {
    fn name(&self) -> &str {
        "ollama"
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
impl OcrBackend for OllamaOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], _config: &OcrConfig) -> Result<ExtractionResult> {
        let backend = self.clone();
        let bytes = image_bytes.to_vec();

        let content = tokio::task::spawn_blocking(move || backend.call_ollama(&bytes))
            .await
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("Ollama OCR task panicked: {}", e),
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

/// Builder for [`OllamaOcrBackend`].
#[derive(Debug, Clone)]
pub struct OllamaOcrBuilder {
    endpoint: String,
    model: String,
    prompt: String,
}

impl Default for OllamaOcrBuilder {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string()),
            model: std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string()),
            prompt: DEFAULT_PROMPT.to_string(),
        }
    }
}

impl OllamaOcrBuilder {
    /// Set the Ollama API endpoint URL.
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

    /// Build the [`OllamaOcrBackend`].
    pub fn build(self) -> OllamaOcrBackend {
        OllamaOcrBackend {
            endpoint: self.endpoint,
            model: self.model,
            prompt: self.prompt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let backend = OllamaOcrBackend::builder()
            .endpoint("https://custom.host:8080")
            .model("llava")
            .prompt("OCR this")
            .build();
        assert_eq!(backend.endpoint, "https://custom.host:8080");
        assert_eq!(backend.model, "llava");
        assert_eq!(backend.prompt, "OCR this");
    }

    #[test]
    fn test_plugin_interface() {
        let backend = OllamaOcrBackend::new();
        assert_eq!(backend.name(), "ollama");
        assert_eq!(backend.backend_type(), OcrBackendType::Custom);
        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("jpn"));
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    #[test]
    fn test_supported_languages() {
        let backend = OllamaOcrBackend::new();
        let langs = backend.supported_languages();
        assert!(langs.contains(&"eng".to_string()));
        assert!(langs.contains(&"chi".to_string()));
    }

    #[test]
    fn test_version_from_cargo() {
        let backend = OllamaOcrBackend::builder().build();
        assert_eq!(backend.version(), env!("CARGO_PKG_VERSION"));
    }
}
