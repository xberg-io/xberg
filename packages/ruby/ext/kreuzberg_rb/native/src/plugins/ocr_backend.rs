//! OCR backend plugin registration and management

use crate::error_handling::{kreuzberg_error, runtime_error};
use crate::gc_guarded_value::GcGuardedValue;
use magnus::{Error, Ruby, TryConvert, Value};
use magnus::value::ReprValue;
use kreuzberg::plugins::{
    register_ocr_backend as kz_register_ocr_backend,
    unregister_ocr_backend as kz_unregister_ocr_backend,
    list_ocr_backends as kz_list_ocr_backends,
    clear_ocr_backends as kz_clear_ocr_backends,
    OcrBackend, OcrBackendType, Plugin,
};
use kreuzberg::types::{ExtractionResult, Metadata};
use kreuzberg::{OcrConfig, KreuzbergError};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// Ruby OCR backend wrapper that implements the OcrBackend trait
struct RubyOcrBackend {
    name: String,
    backend: GcGuardedValue,
}

// SAFETY: Ruby's GC is handled by GcGuardedValue, and we ensure all Ruby
// calls happen through proper Magnus/Ruby FFI boundaries
unsafe impl Send for RubyOcrBackend {}
unsafe impl Sync for RubyOcrBackend {}

impl Plugin for RubyOcrBackend {
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

#[async_trait]
impl OcrBackend for RubyOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        let backend_name = self.name.clone();
        let backend = self.backend.value();
        let image_data = image_bytes.to_vec();
        let ocr_config = config.clone();

        tokio::task::block_in_place(|| {
            let ruby = Ruby::get().expect("Ruby not initialized");

            // Convert image bytes to Ruby string (binary)
            let ruby_bytes = ruby.str_from_slice(&image_data);

            // Convert config to Ruby hash
            let config_hash = ruby.hash_new();
            config_hash.aset("backend", ocr_config.backend.as_str())
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("Failed to set backend in config: {}", e),
                    plugin_name: backend_name.clone(),
                })?;
            config_hash.aset("language", ocr_config.language.as_str())
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("Failed to set language in config: {}", e),
                    plugin_name: backend_name.clone(),
                })?;

            // Call Ruby backend's process_image method
            let result: magnus::Value = backend
                .funcall("process_image", (ruby_bytes, config_hash))
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("Ruby OCR backend failed: {}", e),
                    plugin_name: backend_name.clone(),
                })?;

            // Convert result to String
            let content = String::try_convert(result)
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("OCR backend must return a String: {}", e),
                    plugin_name: backend_name.clone(),
                })?;

            Ok(ExtractionResult {
                content,
                mime_type: "text/plain".to_string(),
                metadata: Metadata::default(),
                tables: vec![],
                detected_languages: None,
                chunks: None,
                images: None,
                djot_content: None,
                pages: None,
                elements: None,
            })
        })
    }

    async fn process_file(&self, path: &Path, config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        let bytes = std::fs::read(path)?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        // Ruby backends are assumed to support all languages by default
        // A more sophisticated implementation could call back to Ruby
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

/// Register an OCR backend plugin
pub fn register_ocr_backend(name: String, backend: Value) -> Result<(), Error> {
    let _ruby = Ruby::get().expect("Ruby not initialized");

    // Validate that the backend has the required methods
    if !backend.respond_to("name", true)? {
        return Err(runtime_error("OCR backend must implement #name method"));
    }
    if !backend.respond_to("process_image", true)? {
        return Err(runtime_error("OCR backend must implement #process_image(image_bytes, config) method"));
    }

    let backend_impl = Arc::new(RubyOcrBackend {
        name: name.clone(),
        backend: GcGuardedValue::new(backend),
    });

    kz_register_ocr_backend(backend_impl)
        .map_err(kreuzberg_error)
}

/// Unregister an OCR backend
pub fn unregister_ocr_backend(_name: String) -> Result<(), Error> {
    kz_unregister_ocr_backend(_name.as_str())
        .map_err(kreuzberg_error)
}

/// List registered OCR backends
pub fn list_ocr_backends() -> Result<Vec<String>, Error> {
    kz_list_ocr_backends()
        .map_err(kreuzberg_error)
}

/// Clear all OCR backends
pub fn clear_ocr_backends() -> Result<(), Error> {
    kz_clear_ocr_backends()
        .map_err(kreuzberg_error)
}
