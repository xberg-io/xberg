//! Standalone embedding FFI functions.

use crate::ffi_panic_guard;
use crate::helpers::{clear_last_error, set_last_error, string_to_c_string};
use std::ffi::{CStr, c_char};
use std::ptr;

/// Generate embeddings for a list of texts.
///
/// # Arguments
///
/// * `texts_json` - Null-terminated C string containing a JSON array of strings,
///   e.g. `["hello","world"]`. Must not be NULL.
/// * `config_json` - Null-terminated C string containing a JSON object with
///   `EmbeddingConfig` fields, e.g. `{"model":{"type":"preset","name":"balanced"}}`.
///   May be NULL to use default config.
///
/// # Returns
///
/// A JSON string representing an array of float arrays (one per input text),
/// e.g. `[[0.1,0.2,...],[0.3,0.4,...]]`. Caller **must** free with `kreuzberg_free_string`.
/// Returns NULL on error — check `kreuzberg_last_error` for the message.
///
/// # Safety
///
/// * `texts_json` must be a valid null-terminated UTF-8 C string (not NULL).
/// * `config_json` must be a valid null-terminated UTF-8 C string, or NULL.
/// * The returned pointer must be freed with `kreuzberg_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_embed(texts_json: *const c_char, config_json: *const c_char) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_embed", {
        clear_last_error();

        if texts_json.is_null() {
            set_last_error("texts_json cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let texts_str = match unsafe { CStr::from_ptr(texts_json) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in texts_json: {}", e));
                return ptr::null_mut();
            }
        };

        let texts: Vec<String> = match serde_json::from_str(texts_str) {
            Ok(v) => v,
            Err(e) => {
                set_last_error(format!("Failed to parse texts_json: {}", e));
                return ptr::null_mut();
            }
        };

        let config: kreuzberg::EmbeddingConfig = if config_json.is_null() {
            Default::default()
        } else {
            let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
                Ok(s) => s,
                Err(e) => {
                    set_last_error(format!("Invalid UTF-8 in config_json: {}", e));
                    return ptr::null_mut();
                }
            };
            match serde_json::from_str(config_str) {
                Ok(c) => c,
                Err(e) => {
                    set_last_error(format!("Failed to parse config_json: {}", e));
                    return ptr::null_mut();
                }
            }
        };

        match kreuzberg::embed_texts(&texts, &config) {
            Ok(embeddings) => match serde_json::to_string(&embeddings) {
                Ok(json) => match string_to_c_string(json) {
                    Ok(ptr) => ptr,
                    Err(e) => {
                        set_last_error(e);
                        ptr::null_mut()
                    }
                },
                Err(e) => {
                    set_last_error(format!("Failed to serialize embeddings: {}", e));
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
