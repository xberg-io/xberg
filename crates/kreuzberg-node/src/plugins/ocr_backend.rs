use async_trait::async_trait;
use base64::Engine;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use std::sync::Arc;

use kreuzberg::plugins::OcrBackend as RustOcrBackend;
use kreuzberg::plugins::OcrBackendType;
use kreuzberg::plugins::Plugin;
use kreuzberg::plugins::registry::get_ocr_backend_registry;

use crate::error_handling::convert_error;

/// Wrapper that makes a JavaScript OCR backend usable from Rust.
///
/// The process_image_fn is an async JavaScript function that:
/// - Takes: (String, String) - Base64 encoded image bytes and language
/// - Returns: Promise<String> - JSON-serialized ExtractionResult
///
/// Type parameters:
/// - Input: (String, String)
/// - Return: Promise<String>
/// - CallJsBackArgs: Vec<(String, String)> (because build_callback returns vec![value])
/// - ErrorStatus: napi::Status
/// - CalleeHandled: false (default with build_callback)
type ProcessImageFn =
    Arc<ThreadsafeFunction<(String, String), Promise<String>, Vec<(String, String)>, napi::Status, false>>;

struct JsOcrBackend {
    name: String,
    supported_languages: Vec<String>,
    process_image_fn: ProcessImageFn,
}

unsafe impl Send for JsOcrBackend {}
unsafe impl Sync for JsOcrBackend {}

impl Plugin for JsOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        Ok(())
    }

    fn shutdown(&self) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        Ok(())
    }
}

#[async_trait]
impl RustOcrBackend for JsOcrBackend {
    async fn process_image(
        &self,
        image_bytes: &[u8],
        config: &kreuzberg::OcrConfig,
    ) -> std::result::Result<kreuzberg::ExtractionResult, kreuzberg::KreuzbergError> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
        let language = config.language.clone();
        let backend_name = self.name.clone();

        let output_json = self
            .process_image_fn
            .call_async((encoded, language))
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!("JavaScript OCR backend '{}' failed: {}", backend_name, e),
                source: Some(Box::new(e)),
            })?
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!("JavaScript OCR backend '{}' failed: {}", backend_name, e),
                source: Some(Box::new(e)),
            })?;

        let wire_result: serde_json::Value =
            serde_json::from_str(&output_json).map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!(
                    "Failed to deserialize JSON result from JavaScript OCR backend '{}': {}",
                    backend_name, e
                ),
                source: Some(Box::new(e)),
            })?;

        let content = wire_result
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kreuzberg::KreuzbergError::Ocr {
                message: format!(
                    "JavaScript OCR backend '{}' result missing 'content' field",
                    backend_name
                ),
                source: None,
            })?
            .to_string();

        let mime_type = wire_result
            .get("mime_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text/plain")
            .to_string();

        let metadata = wire_result
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let metadata: kreuzberg::types::Metadata =
            serde_json::from_value(metadata).map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!(
                    "Failed to parse metadata from JavaScript OCR backend '{}': {}",
                    backend_name, e
                ),
                source: Some(Box::new(e)),
            })?;

        let tables = wire_result
            .get("tables")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| serde_json::from_value::<kreuzberg::Table>(t.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(kreuzberg::ExtractionResult {
            content,
            mime_type: std::borrow::Cow::Owned(mime_type),
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
            elements: None,
            djot_content: None,
            ocr_elements: None,
            document: None,
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: vec![],
            annotations: None,
        })
    }

    async fn process_image_file(
        &self,
        path: &std::path::Path,
        config: &kreuzberg::OcrConfig,
    ) -> std::result::Result<kreuzberg::ExtractionResult, kreuzberg::KreuzbergError> {
        use kreuzberg::core::io;
        let bytes = io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, lang: &str) -> bool {
        self.supported_languages.iter().any(|l| l == lang)
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        self.supported_languages.clone()
    }
}

/// Register a custom OCR backend
///
/// Registers a JavaScript OCR backend that can process images and extract text.
///
/// # Arguments
///
/// * `backend` - JavaScript object with the following interface:
///   - `name(): string` - Unique backend name
///   - `supportedLanguages(): string[]` - Array of supported ISO 639-2/3 language codes
///   - `processImage(imageBytes: string, language: string): Promise<result>` - Process image and return extraction result
///
/// # Implementation Notes
///
/// Due to NAPI ThreadsafeFunction limitations, the processImage function receives:
/// - `imageBytes` as a Base64 string (first argument)
/// - `language` as string (second argument)
///
/// And must return a Promise resolving to a JSON-serializable object with:
/// ```typescript
/// {
///   content: string,
///   mime_type: string,  // default: "text/plain"
///   metadata: object,   // default: {}
///   tables: array       // default: []
/// }
/// ```
///
/// # Example
///
/// ```typescript
/// import { registerOcrBackend } from '@kreuzberg/node';
///
/// registerOcrBackend({
///   name: () => "my-ocr",
///   supportedLanguages: () => ["eng", "deu", "fra"],
///   processImage: async (imageBytes, language) => {
///     const buffer = Buffer.from(imageBytes, "base64");
///     const text = await myOcrLibrary.process(buffer, language);
///     return {
///       content: text,
///       mime_type: "text/plain",
///       metadata: { confidence: 0.95 },
///       tables: []
///     };
///   }
/// });
/// ```
#[napi]
pub fn register_ocr_backend(_env: Env, backend: Object) -> Result<()> {
    use super::validate_plugin_object;

    validate_plugin_object(&backend, "OCR Backend", &["name", "supportedLanguages", "processImage"])?;

    let name: String = backend.get_named_property::<String>("name").or_else(|_| {
        let name_fn: Function<(), String> = backend.get_named_property("name")?;
        name_fn.call(())
    })?;

    if name.is_empty() {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            "OCR backend name cannot be empty".to_string(),
        ));
    }

    let supported_languages: Vec<String> = backend
        .get_named_property::<Vec<String>>("supportedLanguages")
        .or_else(|_| {
            let supported_languages_fn: Function<(), Vec<String>> = backend.get_named_property("supportedLanguages")?;
            supported_languages_fn.call(())
        })?;

    if supported_languages.is_empty() {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            "OCR backend must support at least one language".to_string(),
        ));
    }

    let process_image_fn: Function<(String, String), Promise<String>> = backend.get_named_property("processImage")?;

    let tsfn = process_image_fn
        .build_threadsafe_function()
        .build_callback(|ctx| Ok(vec![ctx.value]))?;

    let js_ocr_backend = JsOcrBackend {
        name: name.clone(),
        supported_languages,
        process_image_fn: Arc::new(tsfn),
    };

    let arc_backend: Arc<dyn RustOcrBackend> = Arc::new(js_ocr_backend);
    let registry = get_ocr_backend_registry();
    let mut registry = registry.write().map_err(|e| {
        napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to acquire write lock on OCR backend registry: {}", e),
        )
    })?;

    registry.register(arc_backend).map_err(|e| {
        napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to register OCR backend '{}': {}", name, e),
        )
    })?;

    Ok(())
}

/// Unregister an OCR backend by name
#[napi]
pub fn unregister_ocr_backend(name: String) -> Result<()> {
    kreuzberg::plugins::unregister_ocr_backend(&name).map_err(convert_error)
}

/// List all registered OCR backends
#[napi]
pub fn list_ocr_backends() -> Result<Vec<String>> {
    kreuzberg::plugins::list_ocr_backends().map_err(convert_error)
}

/// Clear all registered OCR backends
#[napi]
pub fn clear_ocr_backends() -> Result<()> {
    kreuzberg::plugins::clear_ocr_backends().map_err(convert_error)
}
