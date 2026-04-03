//! OCR Backend Plugin FFI
//!
//! Provides FFI bindings for registering and managing custom OCR backends.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;

use async_trait::async_trait;
use kreuzberg::core::config::OcrConfig;
use kreuzberg::plugins::registry::get_ocr_backend_registry;
use kreuzberg::plugins::{OcrBackend, Plugin};
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result};

use crate::helpers::{clear_last_error, set_last_error};
use crate::memory::kreuzberg_free_string;
// Macros are exported at the crate root due to #[macro_export]
use crate::{ffi_panic_guard, ffi_panic_guard_bool, ffi_panic_guard_i32};

/// Type alias for the OCR backend callback function.
///
/// # Parameters
///
/// - `image_bytes`: Raw image bytes
/// - `image_length`: Length of image data in bytes
/// - `config_json`: JSON-encoded OcrConfig (null-terminated string)
///
/// # Returns
///
/// Null-terminated string containing extracted text (must be freed by Rust via kreuzberg_free_string),
/// or NULL on error.
///
/// # Safety
///
/// The callback must:
/// - Not store the image_bytes pointer (it's only valid for the duration of the call)
/// - Return a valid null-terminated UTF-8 string allocated by the caller
/// - Return NULL on error (error message should be retrievable separately)
type OcrBackendCallback =
    unsafe extern "C" fn(image_bytes: *const u8, image_length: usize, config_json: *const c_char) -> *mut c_char;

fn parse_languages_from_json(languages_json: *const c_char) -> std::result::Result<Option<Vec<String>>, String> {
    if languages_json.is_null() {
        return Ok(None);
    }

    let raw = unsafe { CStr::from_ptr(languages_json) }
        .to_str()
        .map_err(|e| format!("Invalid UTF-8 in languages JSON: {}", e))?;

    if raw.trim().is_empty() {
        return Ok(None);
    }

    let langs: Vec<String> = serde_json::from_str(raw).map_err(|e| format!("Failed to parse languages JSON: {}", e))?;

    if langs.is_empty() {
        return Ok(None);
    }

    let normalized = langs
        .into_iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        return Ok(None);
    }

    Ok(Some(normalized))
}

/// FFI wrapper for custom OCR backends registered from Java/C.
///
/// This struct wraps a C function pointer and implements the OcrBackend trait,
/// allowing custom OCR implementations from FFI languages to be registered
/// and used within the Rust extraction pipeline.
struct FfiOcrBackend {
    name: String,
    callback: OcrBackendCallback,
    supported_languages: Option<Vec<String>>,
}

impl FfiOcrBackend {
    fn new(name: String, callback: OcrBackendCallback, supported_languages: Option<Vec<String>>) -> Self {
        Self {
            name,
            callback,
            supported_languages,
        }
    }
}

impl Plugin for FfiOcrBackend {
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
impl OcrBackend for FfiOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        let config_json = serde_json::to_string(config).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize OCR config: {}", e),
            source: Some(Box::new(e)),
        })?;

        let callback = self.callback;
        let image_data = image_bytes.to_vec();
        let config_json_owned = config_json.clone();

        let result_text = tokio::task::spawn_blocking(move || {
            let config_cstring = CString::new(config_json_owned).map_err(|e| KreuzbergError::Validation {
                message: format!("Failed to create C string from config JSON: {}", e),
                source: Some(Box::new(e)),
            })?;

            let result_ptr = unsafe { callback(image_data.as_ptr(), image_data.len(), config_cstring.as_ptr()) };

            if result_ptr.is_null() {
                return Err(KreuzbergError::Ocr {
                    message: "OCR backend returned NULL (operation failed)".to_string(),
                    source: None,
                });
            }

            let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
            let text = result_cstr
                .to_str()
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("OCR backend returned invalid UTF-8: {}", e),
                    source: Some(Box::new(e)),
                })?
                .to_string();

            unsafe { kreuzberg_free_string(result_ptr) };

            Ok(text)
        })
        .await
        .map_err(|e| KreuzbergError::Ocr {
            message: format!("OCR backend task panicked: {}", e),
            source: Some(Box::new(e)),
        })??;

        Ok(ExtractionResult {
            content: result_text,
            mime_type: std::borrow::Cow::Borrowed("text/plain"),
            metadata: kreuzberg::types::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: None,
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: vec![],
            annotations: None,
            uris: None,
            code_intelligence: None,
            formatted_content: None,
            children: None,
            ocr_internal_document: None,
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        match &self.supported_languages {
            Some(langs) => langs.iter().any(|candidate| candidate.eq_ignore_ascii_case(_lang)),
            None => true,
        }
    }

    fn backend_type(&self) -> kreuzberg::plugins::OcrBackendType {
        kreuzberg::plugins::OcrBackendType::Custom
    }
}

/// Register a custom OCR backend via FFI callback.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `callback` must be a valid function pointer that:
///   - Does not store the image_bytes pointer
///   - Returns a null-terminated UTF-8 string or NULL on error
///   - The returned string must be freeable by kreuzberg_free_string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// char* my_ocr_backend(const uint8_t* image_bytes, size_t image_length, const char* config_json) {
///     // Implement OCR logic here
///     // Return allocated string with result, or NULL on error
///     return strdup("Extracted text");
/// }
///
/// bool success = kreuzberg_register_ocr_backend("my-ocr", my_ocr_backend);
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to register: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_ocr_backend(name: *const c_char, callback: OcrBackendCallback) -> bool {
    ffi_panic_guard_bool!("kreuzberg_register_ocr_backend", {
        clear_last_error();

        if name.is_null() {
            set_last_error("Backend name cannot be NULL".to_string());
            return false;
        }

        // SAFETY: C callers may pass NULL for function pointer parameters.
        // `black_box` prevents the compiler from optimizing away the null check.
        if core::hint::black_box(callback as usize) == 0 {
            set_last_error("OCR backend callback cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in backend name: {}", e));
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

        let backend = Arc::new(FfiOcrBackend::new(name_str.to_string(), callback, None));

        let registry = get_ocr_backend_registry();
        let mut registry_guard = registry.write();

        match registry_guard.register(backend) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to register OCR backend: {}", e));
                false
            }
        }
    })
}

/// Register a custom OCR backend with explicit language support via FFI callback.
///
/// # Safety
///
/// - `languages_json` must be a null-terminated JSON array of language codes or NULL
/// - See `kreuzberg_register_ocr_backend` for additional safety notes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_ocr_backend_with_languages(
    name: *const c_char,
    callback: OcrBackendCallback,
    languages_json: *const c_char,
) -> bool {
    ffi_panic_guard_bool!("kreuzberg_register_ocr_backend_with_languages", {
        clear_last_error();

        if name.is_null() {
            set_last_error("Backend name cannot be NULL".to_string());
            return false;
        }

        // SAFETY: C callers may pass NULL for function pointer parameters.
        // `black_box` prevents the compiler from optimizing away the null check.
        if core::hint::black_box(callback as usize) == 0 {
            set_last_error("OCR backend callback cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in backend name: {}", e));
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

        let supported_languages = match parse_languages_from_json(languages_json) {
            Ok(langs) => langs,
            Err(e) => {
                set_last_error(e);
                return false;
            }
        };

        let backend = Arc::new(FfiOcrBackend::new(name_str.to_string(), callback, supported_languages));

        let registry = get_ocr_backend_registry();
        let mut registry_guard = registry.write();

        match registry_guard.register(backend) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to register OCR backend: {}", e));
                false
            }
        }
    })
}

/// Unregister an OCR backend by name.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_unregister_ocr_backend("custom-ocr");
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to unregister: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_unregister_ocr_backend(name: *const c_char) -> bool {
    ffi_panic_guard_bool!("kreuzberg_unregister_ocr_backend", {
        clear_last_error();

        if name.is_null() {
            set_last_error("OCR backend name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in OCR backend name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("OCR backend name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("OCR backend name cannot contain whitespace".to_string());
            return false;
        }

        match kreuzberg::plugins::unregister_ocr_backend(name_str) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(e.to_string());
                false
            }
        }
    })
}

/// List all registered OCR backends as a JSON array of names.
///
/// # Safety
///
/// - Returned string must be freed with `kreuzberg_free_string`.
/// - Returns NULL on error (check `kreuzberg_last_error`).
///
/// # Example (C)
///
/// ```c
/// char* backends = kreuzberg_list_ocr_backends();
/// if (backends == NULL) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to list backends: %s\n", error);
/// } else {
///     printf("OCR backends: %s\n", backends);
///     kreuzberg_free_string(backends);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_ocr_backends() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_ocr_backends", {
        clear_last_error();

        match kreuzberg::plugins::list_ocr_backends() {
            Ok(backends) => match serde_json::to_string(&backends) {
                Ok(json) => match CString::new(json) {
                    Ok(cstr) => cstr.into_raw(),
                    Err(e) => {
                        set_last_error(format!("Failed to create C string: {}", e));
                        ptr::null_mut()
                    }
                },
                Err(e) => {
                    set_last_error(format!("Failed to serialize OCR backend list: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(e.to_string());
                ptr::null_mut()
            }
        }
    })
}

/// Clear all registered OCR backends.
///
/// # Safety
///
/// - Removes all registered OCR backends. Subsequent extractions will use only built-in backends.
/// - Returns true on success, false on error.
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_clear_ocr_backends();
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to clear OCR backends: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clear_ocr_backends() -> bool {
    ffi_panic_guard_bool!("kreuzberg_clear_ocr_backends", {
        clear_last_error();

        match kreuzberg::plugins::clear_ocr_backends() {
            Ok(()) => true,
            Err(e) => {
                set_last_error(e.to_string());
                false
            }
        }
    })
}

/// Get supported languages for an OCR backend.
///
/// Returns a JSON array of supported language codes for the given backend.
/// Supported backends: "easyocr", "paddleocr", "tesseract"
///
/// # Safety
///
/// - The returned string must be freed with `kreuzberg_free_string`
/// - Returns NULL if backend not found or on error (check `kreuzberg_last_error`)
///
/// # Example (C)
///
/// ```c
/// char* languages = kreuzberg_get_ocr_languages("easyocr");
/// if (languages != NULL) {
///     printf("EasyOCR languages: %s\n", languages);
///     kreuzberg_free_string(languages);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_get_ocr_languages(backend: *const c_char) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_get_ocr_languages", {
        clear_last_error();

        if backend.is_null() {
            set_last_error("Backend name cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let backend_str = match unsafe { CStr::from_ptr(backend) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in backend name: {}", e));
                return ptr::null_mut();
            }
        };

        use kreuzberg::ocr::LanguageRegistry;
        let registry = LanguageRegistry::global();

        match registry.get_supported_languages(backend_str) {
            Some(languages) => match serde_json::to_string(languages) {
                Ok(json) => match CString::new(json) {
                    Ok(cstr) => cstr.into_raw(),
                    Err(e) => {
                        set_last_error(format!("Failed to serialize language list: {}", e));
                        ptr::null_mut()
                    }
                },
                Err(e) => {
                    set_last_error(format!("Failed to serialize language list: {}", e));
                    ptr::null_mut()
                }
            },
            None => {
                set_last_error(format!("Unknown OCR backend: '{}'", backend_str));
                ptr::null_mut()
            }
        }
    })
}

/// Check if a language is supported by an OCR backend.
///
/// Returns 1 (true) if the language is supported, 0 (false) otherwise.
///
/// # Arguments
///
/// * `backend` - Backend name (e.g., "easyocr", "paddleocr", "tesseract")
/// * `language` - Language code to check
///
/// # Returns
///
/// 1 if supported, 0 if not supported or backend not found.
///
/// # Example (C)
///
/// ```c
/// int is_supported = kreuzberg_is_language_supported("easyocr", "en");
/// if (is_supported) {
///     printf("English is supported by EasyOCR\n");
/// }
/// ```
///
/// # Safety
///
/// - `backend` and `language` must be valid pointers to valid UTF-8 C strings.
/// - Both pointers can be checked for NULL; returns 0 if either is NULL.
/// - The C strings must remain valid for the duration of the function call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_is_language_supported(backend: *const c_char, language: *const c_char) -> i32 {
    ffi_panic_guard_i32!("kreuzberg_is_language_supported", {
        clear_last_error();

        if backend.is_null() || language.is_null() {
            set_last_error("Backend and language parameters cannot be NULL".to_string());
            return 0;
        }

        let backend_str = match unsafe { CStr::from_ptr(backend) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in backend name: {}", e));
                return 0;
            }
        };

        let language_str = match unsafe { CStr::from_ptr(language) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in language code: {}", e));
                return 0;
            }
        };

        use kreuzberg::ocr::LanguageRegistry;
        let registry = LanguageRegistry::global();

        if registry.is_language_supported(backend_str, language_str) {
            1
        } else {
            0
        }
    })
}

/// Get list of all registered OCR backends with language support.
///
/// Returns a JSON object mapping backend names to language counts.
/// Example: `{"easyocr": 80, "paddleocr": 14, "tesseract": 100}`
///
/// # Safety
///
/// - The returned string must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
///
/// # Example (C)
///
/// ```c
/// char* backends = kreuzberg_list_ocr_backends_with_languages();
/// if (backends != NULL) {
///     printf("Available backends: %s\n", backends);
///     kreuzberg_free_string(backends);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_ocr_backends_with_languages() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_ocr_backends_with_languages", {
        clear_last_error();

        use kreuzberg::ocr::LanguageRegistry;
        let registry = LanguageRegistry::global();
        let backends = registry.get_backends();

        let mut backend_map = serde_json::json!({});
        for backend in backends {
            let count = registry.get_language_count(&backend);
            backend_map[&backend] = serde_json::json!(count);
        }

        match CString::new(backend_map.to_string()) {
            Ok(cstr) => cstr.into_raw(),
            Err(e) => {
                set_last_error(format!("Failed to serialize backend list: {}", e));
                ptr::null_mut()
            }
        }
    })
}
