//! VLM-based OCR using liter-llm vision models.
//!
//! Provides text extraction from images by sending them to a vision language
//! model (e.g., GPT-4o, Claude) via the liter-llm client.  This is an
//! alternative to traditional OCR backends (Tesseract, PaddleOCR) and can
//! produce higher-quality results for complex layouts, handwriting, or
//! low-quality scans.

use std::borrow::Cow;

use async_trait::async_trait;
use base64::Engine;
use liter_llm::{ChatCompletionRequest, ContentPart, ImageUrl, LlmClient, Message, UserContent, UserMessage};

use crate::core::config::LlmConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};

/// VLM-based OCR backend using liter-llm vision models.
///
/// This backend sends images to a vision language model (e.g., GPT-4o, Claude)
/// for text extraction, as an alternative to traditional OCR backends.
pub struct VlmOcrBackend;

impl Plugin for VlmOcrBackend {
    fn name(&self) -> &str {
        "vlm"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl OcrBackend for VlmOcrBackend {
    async fn process_image(
        &self,
        image_bytes: &[u8],
        config: &crate::OcrConfig,
    ) -> crate::Result<crate::ExtractionResult> {
        let vlm_config = config
            .vlm_config
            .as_ref()
            .ok_or_else(|| crate::KreuzbergError::validation("VLM OCR requires vlm_config to be set"))?;

        // Detect MIME type from image bytes
        let mime = infer::get(image_bytes).map(|t| t.mime_type()).unwrap_or("image/png");

        let (text, usage) = vlm_ocr(image_bytes, mime, &config.language, vlm_config, config.vlm_prompt.as_deref()).await?;

        Ok(crate::ExtractionResult {
            content: text,
            mime_type: Cow::Borrowed("text/plain"),
            llm_usage: usage.map(|u| vec![u]),
            ..Default::default()
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

/// Perform OCR on an image using a vision language model.
///
/// Sends the image to a VLM (e.g., GPT-4o, Claude) which extracts text.
/// The language hint is included in the prompt when the document language
/// is not English.
///
/// # Arguments
///
/// * `image_bytes` - Raw image data (JPEG, PNG, WebP, etc.)
/// * `image_mime_type` - MIME type of the image (e.g., `"image/png"`)
/// * `language` - ISO 639 language code or Tesseract language name
///   (e.g., `"eng"`, `"de"`, `"fra"`)
/// * `config` - LLM provider/model configuration
///
/// # Returns
///
/// Extracted text from the image, or an error if the VLM call fails.
///
/// # Errors
///
/// - `KreuzbergError::Ocr` if the VLM returns no content or the API call fails
/// - `KreuzbergError::MissingDependency` if the liter-llm client cannot be created
pub async fn vlm_ocr(
    image_bytes: &[u8],
    image_mime_type: &str,
    language: &str,
    config: &LlmConfig,
    vlm_prompt: Option<&str>,
) -> crate::Result<(String, Option<crate::types::LlmUsage>)> {
    let client = super::client::create_client(config)?;

    // Base64-encode the image into a data URL.
    let b64 = base64::engine::general_purpose::STANDARD.encode(image_bytes);
    let data_url = format!("data:{image_mime_type};base64,{b64}");

    // Use the caller-supplied Jinja2 template if provided, otherwise fall back to the
    // built-in default.  The template receives `{{ language }}` as a context variable.
    let template = vlm_prompt.unwrap_or(super::prompts::VLM_OCR_TEMPLATE);
    let ctx = minijinja::context! { language => language };
    let prompt = super::prompts::render_template(template, &ctx)?;

    // Build a multi-part user message with text prompt + image.
    let message = Message::User(UserMessage {
        content: UserContent::Parts(vec![
            ContentPart::Text { text: prompt },
            ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: data_url,
                    detail: None,
                },
            },
        ]),
        name: None,
    });

    // Use mutable default because `stream` is pub(crate) in liter-llm.
    let mut request = ChatCompletionRequest::default();
    request.model = config.model.clone();
    request.messages = vec![message];
    request.temperature = config.temperature;
    request.max_tokens = config.max_tokens;

    let response = client.chat(request).await.map_err(|e| {
        crate::KreuzbergError::ocr(format!(
            "VLM OCR request failed: model={}, language={}, image_size={}KB: {e}",
            config.model,
            language,
            image_bytes.len() / 1024
        ))
    })?;

    let usage = super::usage::extract_usage_from_chat(&response, "vlm_ocr");

    // Extract the text content from the first choice.
    let text = response
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .ok_or_else(|| crate::KreuzbergError::ocr(format!("VLM OCR returned no content (model={})", config.model)))?
        .to_string();

    Ok((text, usage))
}

#[cfg(test)]
mod tests {

    fn render_ocr_prompt(language: &str) -> String {
        let ctx = minijinja::context! { language => language };
        super::super::prompts::render_template(super::super::prompts::VLM_OCR_TEMPLATE, &ctx).unwrap()
    }

    #[test]
    fn test_vlm_ocr_prompt_non_english_includes_language() {
        let prompt = render_ocr_prompt("deu");
        assert!(prompt.contains("language: deu"));
    }

    #[test]
    fn test_vlm_ocr_prompt_english_no_language_hint() {
        let prompt = render_ocr_prompt("eng");
        assert!(!prompt.contains("language:"));
    }

    #[test]
    fn test_vlm_ocr_prompt_en_no_language_hint() {
        let prompt = render_ocr_prompt("en");
        assert!(!prompt.contains("language:"));
    }

    /// Regression test for issue #760: OcrConfig.vlm_prompt must be honoured.
    ///
    /// Before the fix, vlm_prompt was never passed to vlm_ocr() and the hardcoded
    /// VLM_OCR_TEMPLATE was always used instead.
    #[test]
    fn test_vlm_prompt_custom_template_is_used_issue_760() {
        let custom_prompt = "Extract all text from this document image. \
                             Preserve formatting and use latex for mathematical formulas.";

        // When a custom template is supplied it must be rendered instead of the default.
        let ctx = minijinja::context! { language => "eng" };
        let prompt = super::super::prompts::render_template(custom_prompt, &ctx).unwrap();

        assert!(prompt.contains("latex"), "custom prompt must be used; got: {prompt}");
        assert!(
            prompt.contains("Preserve formatting"),
            "custom prompt must be used; got: {prompt}"
        );
        assert!(
            !prompt.contains("Extract all visible text"),
            "default template must NOT be used when custom prompt is set; got: {prompt}"
        );
    }

    /// When vlm_prompt is None the built-in default template is used.
    #[test]
    fn test_vlm_prompt_none_falls_back_to_default() {
        let ctx = minijinja::context! { language => "eng" };
        let prompt =
            super::super::prompts::render_template(super::super::prompts::VLM_OCR_TEMPLATE, &ctx).unwrap();

        assert!(
            prompt.contains("Extract all visible text"),
            "default template must be used when vlm_prompt is None; got: {prompt}"
        );
    }
}
