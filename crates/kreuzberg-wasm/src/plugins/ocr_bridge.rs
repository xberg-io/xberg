//! OCR backend wrapper implementation for WASM bindings
//!
//! This module provides the WASM bridge for custom OCR backend plugins that
//! can process images and extract text content.

#[allow(unused_imports)]
use super::{JsPluginValue, MakeSend, acquire_read_lock, acquire_write_lock};
#[allow(unused_imports)]
use async_trait::async_trait;
#[allow(unused_imports)]
use js_sys::{Promise, Reflect};
use kreuzberg::plugins::{OcrBackend, OcrBackendType, Plugin};
#[allow(unused_imports)]
use kreuzberg::{ExtractionResult, KreuzbergError, OcrConfig};
use std::borrow::Cow;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
#[allow(unused_imports)]
use wasm_bindgen_futures::JsFuture;

/// Wrapper that makes a JavaScript OcrBackend object usable from Rust.
///
/// # Thread Safety
///
/// This wrapper contains a JsValue which is NOT Send/Sync. Plugin callbacks
/// MUST be invoked only on the main JavaScript thread. The type system
/// enforces this by preventing the wrapper from being moved across threads.
struct JsOcrBackendWrapper {
    name: String,
    #[allow(dead_code)]
    js_obj: JsPluginValue,
    #[allow(dead_code)]
    supported_languages: Vec<String>,
}

impl JsOcrBackendWrapper {
    /// Create a new wrapper from a JS object
    ///
    /// # Safety
    ///
    /// This wrapper must only be accessed from the main JavaScript thread.
    /// Do not pass this to Web Workers or rayon tasks.
    fn new(js_obj: JsValue, name: String, supported_languages: Vec<String>) -> Self {
        Self {
            js_obj: JsPluginValue(js_obj),
            name,
            supported_languages,
        }
    }
}

impl Plugin for JsOcrBackendWrapper {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> kreuzberg::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> kreuzberg::Result<()> {
        Ok(())
    }
}

#[cfg(not(test))]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl OcrBackend for JsOcrBackendWrapper {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);

        let promise = {
            let process_fn = Reflect::get(&self.js_obj.0, &JsValue::from_str("processImage"))
                .map_err(|_| KreuzbergError::Ocr {
                    message: format!("OCR backend '{}' missing 'processImage' method", self.name),
                    source: None,
                })?
                .dyn_into::<js_sys::Function>()
                .map_err(|_| KreuzbergError::Ocr {
                    message: format!("OCR backend '{}' processImage is not a function", self.name),
                    source: None,
                })?;

            let language = config.language.clone();
            let promise_val = process_fn
                .call2(
                    &self.js_obj.0,
                    &JsValue::from_str(&encoded),
                    &JsValue::from_str(&language),
                )
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("OCR backend '{}' processImage call failed: {:?}", self.name, e),
                    source: None,
                })?;

            Promise::resolve(&promise_val)
        };

        let result_val = MakeSend(JsFuture::from(promise))
            .await
            .map_err(|e| KreuzbergError::Ocr {
                message: format!("OCR backend '{}' promise failed: {:?}", self.name, e),
                source: None,
            })?;

        let json_output = result_val.as_string().ok_or_else(|| KreuzbergError::Ocr {
            message: format!("OCR backend '{}' returned non-string result", self.name),
            source: None,
        })?;

        let result: serde_json::Value = serde_json::from_str(&json_output).map_err(|e| KreuzbergError::Ocr {
            message: format!("Failed to parse OCR result: {}", e),
            source: None,
        })?;

        let content = result
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KreuzbergError::Ocr {
                message: format!("OCR backend '{}' result missing 'content' field", self.name),
                source: None,
            })?
            .to_string();

        let mime_type = result
            .get("mime_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text/plain")
            .to_string();

        let metadata = result
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let metadata: kreuzberg::types::Metadata =
            serde_json::from_value(metadata).map_err(|e| KreuzbergError::Ocr {
                message: format!("Failed to parse OCR metadata: {}", e),
                source: None,
            })?;

        let tables = result
            .get("tables")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| serde_json::from_value::<kreuzberg::Table>(t.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(ExtractionResult {
            content,
            mime_type: Cow::Owned(mime_type),
            metadata,
            tables,
            ..Default::default()
        })
    }

    async fn process_image_file(&self, path: &std::path::Path, config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        use kreuzberg::core::io;
        let bytes = io::read_file_sync(path)?;
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

#[cfg(test)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl OcrBackend for JsOcrBackendWrapper {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        Ok(ExtractionResult {
            mime_type: Cow::Borrowed("image/jpeg"),
            ..Default::default()
        })
    }

    async fn process_image_file(&self, _path: &std::path::Path, _config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        Ok(ExtractionResult {
            mime_type: Cow::Borrowed("image/jpeg"),
            ..Default::default()
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        false
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        vec![]
    }
}

/// Register a custom OCR backend.
///
/// # Arguments
///
/// * `backend` - JavaScript object implementing the OcrBackendProtocol interface:
///   - `name(): string` - Unique backend name
///   - `supportedLanguages(): string[]` - Array of language codes the backend supports
///   - `processImage(imageBase64: string, language: string): Promise<string>` - Process image and return JSON result
///
/// # Returns
///
/// Ok if registration succeeds, Err with description if it fails.
///
/// # Example
///
/// ```javascript
/// registerOcrBackend({
///   name: () => "custom-ocr",
///   supportedLanguages: () => ["en", "es", "fr"],
///   processImage: async (imageBase64, language) => {
///     const buffer = Buffer.from(imageBase64, "base64");
///     // Process image with custom OCR engine
///     const text = await customOcrEngine.recognize(buffer, language);
///     return JSON.stringify({
///       content: text,
///       mime_type: "text/plain",
///       metadata: {}
///     });
///   }
/// });
/// ```
#[wasm_bindgen]
pub fn register_ocr_backend(backend: JsValue) -> Result<(), JsValue> {
    let name_fn =
        Reflect::get(&backend, &JsValue::from_str("name")).map_err(|e| format!("Missing 'name' method: {:?}", e))?;

    let langs_fn = Reflect::get(&backend, &JsValue::from_str("supportedLanguages"))
        .map_err(|e| format!("Missing 'supportedLanguages' method: {:?}", e))?;

    let process_fn = Reflect::get(&backend, &JsValue::from_str("processImage"))
        .map_err(|e| format!("Missing 'processImage' method: {:?}", e))?;

    if !name_fn.is_function() || !langs_fn.is_function() || !process_fn.is_function() {
        return Err(JsValue::from_str(
            "name, supportedLanguages, and processImage must be functions",
        ));
    }

    let name_fn = name_fn
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "Failed to convert name to function")?;
    let name = name_fn
        .call0(&backend)
        .map_err(|e| format!("Failed to call name(): {:?}", e))?
        .as_string()
        .ok_or("name() must return a string")?;

    if name.is_empty() {
        return Err(JsValue::from_str("OCR backend name cannot be empty"));
    }

    let langs_fn = langs_fn
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "Failed to convert supportedLanguages to function")?;
    let langs_val = langs_fn
        .call0(&backend)
        .map_err(|e| format!("Failed to call supportedLanguages(): {:?}", e))?;

    let langs_array = js_sys::Array::from(&langs_val);
    let mut supported_languages = Vec::new();
    for i in 0..langs_array.length() {
        if let Some(lang) = langs_array.get(i).as_string() {
            supported_languages.push(lang);
        }
    }

    if supported_languages.is_empty() {
        return Err(JsValue::from_str("OCR backend must support at least one language"));
    }

    let wrapper = JsOcrBackendWrapper::new(backend, name.clone(), supported_languages);
    let registry = kreuzberg::plugins::registry::get_ocr_backend_registry();
    let mut registry = acquire_write_lock(&registry, "OCR_BACKENDS").map_err(|e| JsValue::from_str(&e))?;

    registry
        .register(Arc::new(wrapper))
        .map_err(|e| JsValue::from_str(&format!("Registration failed: {}", e)))
}

/// Unregister an OCR backend by name.
///
/// # Arguments
///
/// * `name` - Name of the OCR backend to unregister
///
/// # Returns
///
/// Ok if unregistration succeeds, Err if the backend is not found or other error occurs.
///
/// # Example
///
/// ```javascript
/// unregisterOcrBackend("custom-ocr");
/// ```
#[wasm_bindgen]
pub fn unregister_ocr_backend(name: String) -> Result<(), JsValue> {
    let registry = kreuzberg::plugins::registry::get_ocr_backend_registry();
    let mut registry = acquire_write_lock(&registry, "OCR_BACKENDS").map_err(|e| JsValue::from_str(&e))?;

    registry
        .remove(&name)
        .map_err(|e| JsValue::from_str(&format!("Unregistration failed: {}", e)))
}

/// Clear all registered OCR backends.
///
/// # Returns
///
/// Ok if clearing succeeds, Err if an error occurs.
///
/// # Example
///
/// ```javascript
/// clearOcrBackends();
/// ```
#[wasm_bindgen]
pub fn clear_ocr_backends() -> Result<(), JsValue> {
    let registry = kreuzberg::plugins::registry::get_ocr_backend_registry();
    let mut registry = acquire_write_lock(&registry, "OCR_BACKENDS").map_err(|e| JsValue::from_str(&e))?;

    let names = registry.list();
    for name in names {
        registry
            .remove(&name)
            .map_err(|e| JsValue::from_str(&format!("Failed to remove OCR backend: {}", e)))?;
    }

    Ok(())
}

/// List all registered OCR backend names.
///
/// # Returns
///
/// Array of OCR backend names, or Err if an error occurs.
///
/// # Example
///
/// ```javascript
/// const backends = listOcrBackends();
/// console.log(backends); // ["tesseract", "custom-ocr", ...]
/// ```
#[wasm_bindgen]
pub fn list_ocr_backends() -> Result<js_sys::Array, JsValue> {
    let registry = kreuzberg::plugins::registry::get_ocr_backend_registry();
    let registry = acquire_read_lock(&registry, "OCR_BACKENDS").map_err(|e| JsValue::from_str(&e))?;

    let names = registry.list();
    let arr = js_sys::Array::new();
    for name in names {
        arr.push(&JsValue::from_str(&name));
    }

    Ok(arr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn create_mock_ocr_backend(name: &str) -> Result<JsValue, String> {
        let obj = js_sys::Object::new();

        Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", &format!("return '{}'", name)),
        )
        .map_err(|_| "Failed to set name method".to_string())?;

        Reflect::set(
            &obj,
            &JsValue::from_str("supportedLanguages"),
            &js_sys::Function::new_with_args("", "return ['en']"),
        )
        .map_err(|_| "Failed to set supportedLanguages method".to_string())?;

        Reflect::set(
            &obj,
            &JsValue::from_str("processImage"),
            &js_sys::Function::new_with_args(
                "imageBase64,language",
                "return Promise.resolve('{\"content\":\"test\"}')",
            ),
        )
        .map_err(|_| "Failed to set processImage method".to_string())?;

        Ok(JsValue::from(obj))
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_valid_backend_succeeds() {
        clear_ocr_backends().ok();
        let backend = create_mock_ocr_backend("test-backend").expect("Failed to create mock OCR backend");

        let result = register_ocr_backend(backend);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_missing_name_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("supportedLanguages"),
            &js_sys::Function::new_with_args("", "return ['en']"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("processImage"),
            &js_sys::Function::new_with_args("image,lang", "return Promise.resolve('')"),
        )
        .ok();

        let result = register_ocr_backend(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_missing_supported_languages_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return 'test'"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("processImage"),
            &js_sys::Function::new_with_args("image,lang", "return Promise.resolve('')"),
        )
        .ok();

        let result = register_ocr_backend(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_missing_process_image_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return 'test'"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("supportedLanguages"),
            &js_sys::Function::new_with_args("", "return ['en']"),
        )
        .ok();

        let result = register_ocr_backend(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_empty_languages_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return 'test'"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("supportedLanguages"),
            &js_sys::Function::new_with_args("", "return []"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("processImage"),
            &js_sys::Function::new_with_args("image,lang", "return Promise.resolve('')"),
        )
        .ok();

        let result = register_ocr_backend(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_unregister_ocr_backend_registered_backend_succeeds() {
        clear_ocr_backends().ok();
        let backend = create_mock_ocr_backend("test-backend").expect("Failed to create mock OCR backend");
        register_ocr_backend(backend).ok();

        let result = unregister_ocr_backend("test-backend".to_string());

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_unregister_ocr_backend_unregistered_backend_fails() {
        clear_ocr_backends().ok();

        let result = unregister_ocr_backend("nonexistent".to_string());

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_clear_ocr_backends_removes_all() {
        clear_ocr_backends().ok();
        let backend1 = create_mock_ocr_backend("backend1").expect("Failed to create mock OCR backend 1");
        let backend2 = create_mock_ocr_backend("backend2").expect("Failed to create mock OCR backend 2");
        register_ocr_backend(backend1).ok();
        register_ocr_backend(backend2).ok();

        let result = clear_ocr_backends();

        assert!(result.is_ok());
        let list = list_ocr_backends().unwrap_or_else(|_| js_sys::Array::new());
        assert_eq!(list.length(), 0);
    }

    #[wasm_bindgen_test]
    fn test_list_ocr_backends_returns_array() {
        clear_ocr_backends().ok();

        let result = list_ocr_backends();

        assert!(result.is_ok());
        let arr = result.unwrap();
        assert!(arr.is_array());
    }

    #[wasm_bindgen_test]
    fn test_list_ocr_backends_after_register_contains_name() {
        clear_ocr_backends().ok();
        let backend = create_mock_ocr_backend("test-backend").expect("Failed to create mock OCR backend");
        register_ocr_backend(backend).ok();

        let result = list_ocr_backends();

        assert!(result.is_ok());
        let arr = result.unwrap();
        assert!(arr.length() > 0);
    }

    #[wasm_bindgen_test]
    fn test_js_ocr_backend_wrapper_implements_plugin() {
        let backend = create_mock_ocr_backend("test").expect("Failed to create mock OCR backend");
        let wrapper = JsOcrBackendWrapper::new(backend, "test".to_string(), vec!["en".to_string()]);

        assert_eq!(wrapper.name(), "test");
        assert_eq!(wrapper.version(), "1.0.0");
        assert!(wrapper.initialize().is_ok());
        assert!(wrapper.shutdown().is_ok());
        assert_eq!(wrapper.supported_languages().len(), 1);
        assert!(wrapper.supports_language("en"));
        assert!(!wrapper.supports_language("fr"));
    }

    #[wasm_bindgen_test]
    fn test_register_multiple_ocr_backends() {
        clear_ocr_backends().ok();
        let b1 = create_mock_ocr_backend("backend1").expect("Failed to create mock OCR backend 1");
        let b2 = create_mock_ocr_backend("backend2").expect("Failed to create mock OCR backend 2");

        assert!(register_ocr_backend(b1).is_ok());
        assert!(register_ocr_backend(b2).is_ok());

        let list = list_ocr_backends().unwrap();
        assert!(list.length() >= 2);
    }
}
