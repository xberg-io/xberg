//! DocumentExtractor plugin system FFI bindings
//!
//! Provides FFI functions for registering, managing, and executing custom document extractors
//! from C/Java/other FFI languages.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;

use async_trait::async_trait;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::Plugin;
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result};

use crate::helpers::{clear_last_error, set_last_error};
use crate::{ffi_panic_guard, ffi_panic_guard_bool};

/// Type alias for the DocumentExtractor callback function.
///
/// # Parameters
///
/// - `content`: Pointer to document bytes (valid only during the call)
/// - `content_len`: Length of the content in bytes
/// - `mime_type`: Null-terminated MIME type string
/// - `config_json`: Null-terminated JSON configuration string
///
/// # Returns
///
/// Null-terminated JSON string containing the ExtractionResult
/// (must be freed by Rust via kreuzberg_free_string), or NULL on error.
///
/// # Safety
///
/// The callback must:
/// - Not store the content, mime_type, or config_json pointers (only valid during the call)
/// - Return a valid null-terminated UTF-8 JSON string or NULL on error
/// - The returned string must be freeable by kreuzberg_free_string
type DocumentExtractorCallback = unsafe extern "C" fn(
    content: *const u8,
    content_len: usize,
    mime_type: *const c_char,
    config_json: *const c_char,
) -> *mut c_char;

/// FFI wrapper for custom DocumentExtractors registered from Java/C.
///
/// This struct wraps a C function pointer and implements the DocumentExtractor trait,
/// allowing custom extraction implementations from FFI languages to be registered
/// and used within the Rust extraction pipeline.
struct FfiDocumentExtractor {
    name: String,
    callback: DocumentExtractorCallback,
    supported_types_static: Vec<&'static str>,
    priority: i32,
}

impl FfiDocumentExtractor {
    fn new(name: String, callback: DocumentExtractorCallback, supported_types: Vec<String>, priority: i32) -> Self {
        let supported_types_static: Vec<&'static str> = supported_types
            .iter()
            .map(|s| {
                let leaked: &'static str = Box::leak(s.clone().into_boxed_str());
                leaked
            })
            .collect();

        Self {
            name,
            callback,
            supported_types_static,
            priority,
        }
    }
}

impl Plugin for FfiDocumentExtractor {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "ffi-1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl kreuzberg::plugins::DocumentExtractor for FfiDocumentExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<kreuzberg::types::internal::InternalDocument> {
        let config_json = serde_json::to_string(config).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize ExtractionConfig: {}", e),
            source: Some(Box::new(e)),
        })?;

        let callback = self.callback;
        let extractor_name = self.name.clone();
        let extractor_name_error = self.name.clone();
        let extractor_name_parse = self.name.clone();
        let content_vec = content.to_vec();
        let mime_type_owned = mime_type.to_string();
        let config_json_owned = config_json.clone();

        let result_json = tokio::task::spawn_blocking(move || {
            let mime_cstr = match CString::new(mime_type_owned.clone()) {
                Ok(s) => s,
                Err(e) => {
                    return Err(KreuzbergError::Validation {
                        message: format!("Invalid MIME type for extractor '{}': {}", extractor_name, e),
                        source: Some(Box::new(e)),
                    });
                }
            };

            let config_cstr = match CString::new(config_json_owned.clone()) {
                Ok(s) => s,
                Err(e) => {
                    return Err(KreuzbergError::Validation {
                        message: format!("Invalid config JSON for extractor '{}': {}", extractor_name, e),
                        source: Some(Box::new(e)),
                    });
                }
            };

            let result_ptr = unsafe {
                callback(
                    content_vec.as_ptr(),
                    content_vec.len(),
                    mime_cstr.as_ptr(),
                    config_cstr.as_ptr(),
                )
            };

            if result_ptr.is_null() {
                return Err(KreuzbergError::Parsing {
                    message: format!("DocumentExtractor '{}' returned NULL (callback failed)", extractor_name),
                    source: None,
                });
            }

            let result_cstr = unsafe { CString::from_raw(result_ptr) };
            let result_str = result_cstr.to_str().map_err(|e| KreuzbergError::Validation {
                message: format!("Invalid UTF-8 in result from extractor '{}': {}", extractor_name, e),
                source: Some(Box::new(e)),
            })?;

            Ok(result_str.to_string())
        })
        .await
        .map_err(|e| {
            KreuzbergError::Other(format!(
                "Task join error in extractor '{}': {}",
                extractor_name_error, e
            ))
        })??;

        let extraction_result: ExtractionResult =
            serde_json::from_str(&result_json).map_err(|e| KreuzbergError::Parsing {
                message: format!(
                    "Failed to deserialize ExtractionResult from extractor '{}': {}",
                    extractor_name_parse, e
                ),
                source: Some(Box::new(e)),
            })?;

        // Convert ExtractionResult from FFI callback to InternalDocument.
        // FFI callers return ExtractionResult JSON; we convert to the internal format.
        let mut doc = kreuzberg::types::internal::InternalDocument::new("ffi");
        doc.metadata = extraction_result.metadata;
        doc.tables = extraction_result.tables;
        doc.mime_type = extraction_result.mime_type;
        if let Some(images) = extraction_result.images {
            doc.images = images;
        }
        if let Some(annotations) = extraction_result.annotations {
            doc.annotations = Some(annotations);
        }
        // Push content as a paragraph element
        if !extraction_result.content.is_empty() {
            use kreuzberg::types::internal::{ElementKind, InternalElement};
            let elem = InternalElement::text(ElementKind::Paragraph, &extraction_result.content, 0);
            doc.push_element(elem);
        }
        Ok(doc)
    }

    async fn extract_file(
        &self,
        path: &std::path::Path,
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<kreuzberg::types::internal::InternalDocument> {
        let content = tokio::fs::read(path).await.map_err(KreuzbergError::Io)?;
        self.extract_bytes(&content, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &self.supported_types_static
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// Register a custom DocumentExtractor via FFI callback.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `callback` must be a valid function pointer that:
///   - Does not store the content, mime_type, or config_json pointers
///   - Returns a null-terminated UTF-8 JSON string or NULL on error
///   - The returned string must be freeable by kreuzberg_free_string
/// - `mime_types` must be a valid null-terminated C string containing comma-separated MIME types
/// - `priority` determines the order of selection (higher priority preferred)
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// char* my_extractor(const uint8_t* content, size_t len, const char* mime_type, const char* config) {
///     // Extract content from bytes, return JSON ExtractionResult
///     return strdup("{\"content\":\"extracted text\",\"mime_type\":\"text/plain\",\"metadata\":{}}");
/// }
///
/// bool success = kreuzberg_register_document_extractor(
///     "my-extractor",
///     my_extractor,
///     "application/x-custom,text/x-custom",
///     100
/// );
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to register: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_document_extractor(
    name: *const c_char,
    callback: DocumentExtractorCallback,
    mime_types: *const c_char,
    priority: i32,
) -> bool {
    ffi_panic_guard_bool!("kreuzberg_register_document_extractor", {
        clear_last_error();

        if name.is_null() {
            set_last_error("DocumentExtractor name cannot be NULL".to_string());
            return false;
        }

        // SAFETY: C callers may pass NULL for function pointer parameters.
        // Bare fn types in Rust are guaranteed non-null, so the compiler may
        // optimize away a direct `== 0` check. `black_box` prevents this by
        // hiding the value from the optimizer.
        if core::hint::black_box(callback as usize) == 0 {
            set_last_error("DocumentExtractor callback cannot be NULL".to_string());
            return false;
        }

        if mime_types.is_null() {
            set_last_error("MIME types cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in DocumentExtractor name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("Plugin name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("Plugin name cannot contain whitespace".to_string());
            return false;
        }

        let mime_types_str = match unsafe { CStr::from_ptr(mime_types) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in MIME types: {}", e));
                return false;
            }
        };

        let supported_types: Vec<String> = mime_types_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if supported_types.is_empty() {
            set_last_error("At least one MIME type must be specified".to_string());
            return false;
        }

        let extractor = Arc::new(FfiDocumentExtractor::new(
            name_str.to_string(),
            callback,
            supported_types,
            priority,
        ));

        let registry = kreuzberg::plugins::registry::get_document_extractor_registry();
        let mut registry_guard = registry.write();

        match registry_guard.register(extractor) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to register DocumentExtractor: {}", e));
                false
            }
        }
    })
}

/// Unregister a DocumentExtractor by name.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_unregister_document_extractor("my-extractor");
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to unregister: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_unregister_document_extractor(name: *const c_char) -> bool {
    ffi_panic_guard_bool!("kreuzberg_unregister_document_extractor", {
        clear_last_error();

        if name.is_null() {
            set_last_error("DocumentExtractor name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in DocumentExtractor name: {}", e));
                return false;
            }
        };

        let registry = kreuzberg::plugins::registry::get_document_extractor_registry();
        let mut registry_guard = registry.write();

        match registry_guard.remove(name_str) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to remove DocumentExtractor: {}", e));
                false
            }
        }
    })
}

/// List all registered DocumentExtractors as a JSON array of names.
///
/// # Safety
///
/// - Returned string must be freed with `kreuzberg_free_string`.
/// - Returns NULL on error (check `kreuzberg_last_error`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_document_extractors() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_document_extractors", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_document_extractor_registry();
        let registry_guard = registry.read();

        match serde_json::to_string(&registry_guard.list()) {
            Ok(json) => match CString::new(json) {
                Ok(cstr) => cstr.into_raw(),
                Err(e) => {
                    set_last_error(format!("Failed to create C string: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(format!("Failed to serialize DocumentExtractor list: {}", e));
                ptr::null_mut()
            }
        }
    })
}

/// Clear all registered DocumentExtractors.
///
/// # Safety
///
/// - Removes all registered extractors. Subsequent extractions will use only built-in extractors.
/// - Returns true on success, false on error.
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_clear_document_extractors();
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to clear document extractors: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clear_document_extractors() -> bool {
    ffi_panic_guard_bool!("kreuzberg_clear_document_extractors", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_document_extractor_registry();
        let mut registry_guard = registry.write();

        *registry_guard = Default::default();
        true
    })
}
