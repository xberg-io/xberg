// JNI bindings for kreuzberg library.
// Delegates to FFI (kreuzberg-ffi) for all operations.
#![allow(non_snake_case, unsafe_code, unsafe_attr_outside_unsafe, deprecated, missing_docs)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use jni::JNIEnv;
use jni::errors::{Error as JniError, ThrowRuntimeExAndDefault};
use jni::objects::{JClass, JString};
use jni::strings::JNIString;
use jni::sys::{jboolean, jbyteArray, jint, jlong, jstring};
use std::ffi::{CStr, CString};

// Pull in kreuzberg-ffi by Rust path. The `use` keeps the rlib's
// #[no_mangle] symbols live through static linking — without an explicit
// reference, rlib dead-code elimination drops them and the JNI shim's
// indirect calls would resolve to null at runtime, crashing the JVM on
// the first FFI invocation. Using the typed FFI signatures directly also
// lets the compiler catch any future signature drift in kreuzberg-ffi.
use kreuzberg_ffi::{
    kreuzberg_batch_extract_bytes, kreuzberg_batch_extract_bytes_sync, kreuzberg_batch_extract_files,
    kreuzberg_batch_extract_files_sync, kreuzberg_clear_document_extractor, kreuzberg_clear_embedding_backend,
    kreuzberg_clear_ocr_backend, kreuzberg_clear_post_processor, kreuzberg_clear_renderer, kreuzberg_clear_validator,
    kreuzberg_detect_mime_type, kreuzberg_detect_mime_type_from_bytes, kreuzberg_embedding_preset_free,
    kreuzberg_embedding_preset_to_json, kreuzberg_extract_bytes, kreuzberg_extract_bytes_sync, kreuzberg_extract_file,
    kreuzberg_extract_file_sync, kreuzberg_extraction_config_free, kreuzberg_extraction_config_from_json,
    kreuzberg_extraction_result_free, kreuzberg_extraction_result_to_json, kreuzberg_free_bytes, kreuzberg_free_string,
    kreuzberg_get_embedding_preset, kreuzberg_get_extensions_for_mime, kreuzberg_last_error_code,
    kreuzberg_last_error_context, kreuzberg_list_document_extractors, kreuzberg_list_embedding_backends,
    kreuzberg_list_embedding_presets, kreuzberg_list_ocr_backends, kreuzberg_list_post_processors,
    kreuzberg_list_renderers, kreuzberg_list_validators, kreuzberg_render_pdf_page_to_png,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Decode Base64 string to bytes using the standard base64 engine
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    STANDARD.decode(input).map_err(|e| format!("Invalid Base64: {}", e))
}

/// Helper to throw a KreuzbergBridgeException and return null/0
fn throw_exception<'local>(env: &mut JNIEnv<'local>, message: &str) -> jstring {
    let msg_jni = JNIString::from(message.to_string());
    let exc_class_name = JNIString::from("dev/kreuzberg/KreuzbergBridgeException");
    let _result = env.with_env(|env| -> Result<(), JniError> {
        let exc_class = env.find_class(exc_class_name.as_ref())?;
        env.throw_new(exc_class, msg_jni.as_ref())
    });
    std::ptr::null_mut()
}

/// Helper for void functions that throw
fn throw_exception_void(env: &mut JNIEnv, message: &str) {
    let msg_jni = JNIString::from(message.to_string());
    let exc_class_name = JNIString::from("dev/kreuzberg/KreuzbergBridgeException");
    let _result = env.with_env(|env| -> Result<(), JniError> {
        let exc_class = env.find_class(exc_class_name.as_ref())?;
        env.throw_new(exc_class, msg_jni.as_ref())
    });
}

/// Get FFI error message from last error context
fn get_ffi_error_message() -> String {
    // SAFETY: kreuzberg_last_error_context() returns a valid C string owned by the FFI
    // layer. The pointer is checked for null before dereferencing via CStr::from_ptr.
    unsafe {
        let ptr = kreuzberg_last_error_context();
        if ptr.is_null() {
            return "Unknown error".to_string();
        }
        match CStr::from_ptr(ptr).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => "Failed to decode error message".to_string(),
        }
    }
}

/// Convert JString to Rust String, returning an error message if conversion fails
fn jstring_to_string<'local>(env: &mut JNIEnv<'local>, jstr: &JString<'local>) -> Result<String, String> {
    let s = env
        .with_env(|env| -> Result<String, JniError> { jstr.try_to_string(env) })
        .resolve::<ThrowRuntimeExAndDefault>();
    Ok(s)
}

/// Pointer-to-c_char that's null when the underlying string is empty.
/// The Kotlin wrapper collapses Optional<String> mime params to "" before
/// reaching JNI; the FFI in turn treats null mime as "auto-detect from path".
/// This bridges the two conventions.
fn cstr_ptr_or_null(opt: &Option<CString>) -> *const std::ffi::c_char {
    opt.as_ref().map_or(std::ptr::null(), |c| c.as_ptr())
}

/// Build an `Option<CString>` from a Rust String, returning None when the
/// string is empty so callers can pass a null pointer to the FFI.
fn cstring_or_none(s: String) -> Result<Option<CString>, String> {
    if s.is_empty() {
        Ok(None)
    } else {
        CString::new(s)
            .map(Some)
            .map_err(|e| format!("Invalid C string: {}", e))
    }
}

/// Convert Rust String to jstring, returning null if allocation fails
fn string_to_jstring(env: &mut JNIEnv, s: &str) -> jstring {
    env.with_env(|env| -> Result<jstring, JniError> { env.new_string(s).map(|js| js.into_raw()) })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Convert a C string pointer to JString, reading the result from FFI
fn cstring_ptr_to_jstring(env: &mut JNIEnv, ptr: *mut std::ffi::c_char) -> jstring {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: ptr must be a valid C string returned by FFI
    let c_str = unsafe { CStr::from_ptr(ptr) };
    match c_str.to_str() {
        Ok(s) => string_to_jstring(env, s),
        Err(_) => {
            throw_exception(env, "Invalid UTF-8 in FFI response");
            std::ptr::null_mut()
        }
    }
}

// Extraction Functions
// ============================================================================

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeExtractBytesImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    content: JString<'local>,
    mime_type: JString<'local>,
    config: JString<'local>,
) -> jstring {
    let content_str = match jstring_to_string(&mut env, &content) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let mime_type_str = match jstring_to_string(&mut env, &mime_type) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let config_str = match jstring_to_string(&mut env, &config) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    // Decode content from Base64 (Kotlin wrapper encodes bytes as Base64 to pass through JNI)
    let content_bytes = match base64_decode(&content_str) {
        Ok(bytes) => bytes,
        Err(e) => {
            return throw_exception(&mut env, &format!("Failed to decode Base64 content: {}", e));
        }
    };

    let mime_type_c = match cstring_or_none(mime_type_str) {
        Ok(opt) => opt,
        Err(e) => return throw_exception(&mut env, &format!("Invalid mime_type: {}", e)),
    };

    let config_c = match CString::new(config_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid config: {}", e)),
    };

    // Parse config JSON into ExtractionConfig
    // SAFETY: config_c is a valid C string created from CString::new above.
    // FFI returns null on error; error context is available via kreuzberg_last_error_code/context.
    let config_ptr = unsafe { kreuzberg_extraction_config_from_json(config_c.as_ptr()) };
    if config_ptr.is_null() {
        // Try to get FFI error details
        // SAFETY: kreuzberg_last_error_code and kreuzberg_last_error_context return valid FFI data.
        let error_code = unsafe { kreuzberg_last_error_code() };
        let error_msg = unsafe { CStr::from_ptr(kreuzberg_last_error_context()) }.to_string_lossy();
        return throw_exception(
            &mut env,
            &format!("Failed to parse config JSON (code {}): {}", error_code, error_msg),
        );
    }

    // SAFETY: We have valid pointers from CString and config_ptr; cstr_ptr_or_null
    // hands a null pointer to FFI when the mime was empty so Rust auto-detects.
    let result = unsafe {
        kreuzberg_extract_bytes(
            content_bytes.as_ptr(),
            content_bytes.len(),
            cstr_ptr_or_null(&mime_type_c),
            config_ptr,
        )
    };

    if result.is_null() {
        unsafe {
            kreuzberg_extraction_config_free(config_ptr);
        }
        return throw_exception(&mut env, &format!("Extract bytes failed: {}", get_ffi_error_message()));
    }

    // SAFETY: result is a valid ExtractionResult pointer from FFI; convert to JSON
    let json_ptr = unsafe { kreuzberg_extraction_result_to_json(result) };
    let jstr = cstring_ptr_to_jstring(&mut env, json_ptr);
    // Clean up
    unsafe {
        kreuzberg_extraction_result_free(result);
        kreuzberg_extraction_config_free(config_ptr);
        kreuzberg_free_string(json_ptr);
    }
    jstr
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeExtractFileImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    path: JString<'local>,
    mime_type: JString<'local>,
    config: JString<'local>,
) -> jstring {
    let path_str = match jstring_to_string(&mut env, &path) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let mime_type_str = match jstring_to_string(&mut env, &mime_type) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let config_str = match jstring_to_string(&mut env, &config) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let path_c = match CString::new(path_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid path: {}", e)),
    };

    let mime_type_c = match cstring_or_none(mime_type_str) {
        Ok(opt) => opt,
        Err(e) => return throw_exception(&mut env, &format!("Invalid mime_type: {}", e)),
    };

    let config_c = match CString::new(config_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid config: {}", e)),
    };

    let config_ptr = unsafe { kreuzberg_extraction_config_from_json(config_c.as_ptr()) };
    if config_ptr.is_null() {
        return throw_exception(&mut env, "Failed to parse config JSON");
    }

    // SAFETY: We have valid pointers from CString; mime null = FFI auto-detect.
    let result = unsafe { kreuzberg_extract_file(path_c.as_ptr(), cstr_ptr_or_null(&mime_type_c), config_ptr) };

    if result.is_null() {
        unsafe {
            kreuzberg_extraction_config_free(config_ptr);
        }
        return throw_exception(&mut env, &format!("Extract file failed: {}", get_ffi_error_message()));
    }

    // SAFETY: result is a valid ExtractionResult pointer from FFI
    let json_ptr = unsafe { kreuzberg_extraction_result_to_json(result) };
    let jstr = cstring_ptr_to_jstring(&mut env, json_ptr);
    // Clean up
    unsafe {
        kreuzberg_extraction_result_free(result);
        kreuzberg_extraction_config_free(config_ptr);
        kreuzberg_free_string(json_ptr);
    }
    jstr
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeExtractFileSyncImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    path: JString<'local>,
    mime_type: JString<'local>,
    config: JString<'local>,
) -> jstring {
    let path_str = match jstring_to_string(&mut env, &path) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let mime_type_str = match jstring_to_string(&mut env, &mime_type) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let config_str = match jstring_to_string(&mut env, &config) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let path_c = match CString::new(path_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid path: {}", e)),
    };

    let mime_type_c = match cstring_or_none(mime_type_str) {
        Ok(opt) => opt,
        Err(e) => return throw_exception(&mut env, &format!("Invalid mime_type: {}", e)),
    };

    let config_c = match CString::new(config_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid config: {}", e)),
    };

    let config_ptr = unsafe { kreuzberg_extraction_config_from_json(config_c.as_ptr()) };
    if config_ptr.is_null() {
        return throw_exception(
            &mut env,
            &format!("Failed to parse config JSON: {}", config_c.to_string_lossy()),
        );
    }

    // SAFETY: We have valid pointers from CString; mime null = FFI auto-detect.
    let result = unsafe { kreuzberg_extract_file_sync(path_c.as_ptr(), cstr_ptr_or_null(&mime_type_c), config_ptr) };

    if result.is_null() {
        unsafe {
            kreuzberg_extraction_config_free(config_ptr);
        }
        return throw_exception(
            &mut env,
            &format!("Extract file sync failed: {}", get_ffi_error_message()),
        );
    }

    // SAFETY: result is a valid ExtractionResult pointer from FFI
    let json_ptr = unsafe { kreuzberg_extraction_result_to_json(result) };
    let jstr = cstring_ptr_to_jstring(&mut env, json_ptr);
    // Clean up
    unsafe {
        kreuzberg_extraction_result_free(result);
        kreuzberg_extraction_config_free(config_ptr);
        kreuzberg_free_string(json_ptr);
    }
    jstr
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeExtractBytesSyncImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    content: JString<'local>,
    mime_type: JString<'local>,
    config: JString<'local>,
) -> jstring {
    let content_str = match jstring_to_string(&mut env, &content) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let mime_type_str = match jstring_to_string(&mut env, &mime_type) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let config_str = match jstring_to_string(&mut env, &config) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    // Decode content from Base64 (Kotlin wrapper encodes bytes as Base64 to pass through JNI)
    let content_bytes = match base64_decode(&content_str) {
        Ok(bytes) => bytes,
        Err(e) => return throw_exception(&mut env, &format!("Failed to decode Base64 content: {}", e)),
    };

    let mime_type_c = match cstring_or_none(mime_type_str.clone()) {
        Ok(opt) => opt,
        Err(e) => return throw_exception(&mut env, &format!("Invalid mime_type: {}", e)),
    };

    let config_c = match CString::new(config_str.clone()) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid config (null byte in string): {}", e)),
    };

    let config_ptr = unsafe { kreuzberg_extraction_config_from_json(config_c.as_ptr()) };
    if config_ptr.is_null() {
        return throw_exception(&mut env, &format!("Failed to parse config JSON: '{}'", config_str));
    }

    // SAFETY: We have valid pointers from CString; mime null = FFI auto-detect.
    let result = unsafe {
        kreuzberg_extract_bytes_sync(
            content_bytes.as_ptr(),
            content_bytes.len(),
            cstr_ptr_or_null(&mime_type_c),
            config_ptr,
        )
    };

    if result.is_null() {
        unsafe {
            kreuzberg_extraction_config_free(config_ptr);
        }
        return throw_exception(
            &mut env,
            &format!("Extract bytes sync failed: {}", get_ffi_error_message()),
        );
    }

    // SAFETY: result is a valid ExtractionResult pointer from FFI
    let json_ptr = unsafe { kreuzberg_extraction_result_to_json(result) };
    let jstr = cstring_ptr_to_jstring(&mut env, json_ptr);
    // Clean up
    unsafe {
        kreuzberg_extraction_result_free(result);
        kreuzberg_extraction_config_free(config_ptr);
        kreuzberg_free_string(json_ptr);
    }
    jstr
}

// ============================================================================
// Batch Extraction Functions
// ============================================================================

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeBatchExtractFilesSyncImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    items: JString<'local>,
    config: JString<'local>,
) -> jstring {
    let items_str = match jstring_to_string(&mut env, &items) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let config_str = match jstring_to_string(&mut env, &config) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let items_c = match CString::new(items_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid items: {}", e)),
    };

    let config_c = match CString::new(config_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid config: {}", e)),
    };

    let config_ptr = unsafe { kreuzberg_extraction_config_from_json(config_c.as_ptr()) };
    if config_ptr.is_null() {
        return throw_exception(&mut env, "Failed to parse config JSON");
    }

    // SAFETY: We have valid pointers from CString
    let result_ptr = unsafe { kreuzberg_batch_extract_files_sync(items_c.as_ptr(), config_ptr) };

    if result_ptr.is_null() {
        unsafe {
            kreuzberg_extraction_config_free(config_ptr);
        }
        return throw_exception(
            &mut env,
            &format!("Batch extract files sync failed: {}", get_ffi_error_message()),
        );
    }

    let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
    // Clean up
    unsafe {
        kreuzberg_extraction_config_free(config_ptr);
        kreuzberg_free_string(result_ptr);
    }
    jstr
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeBatchExtractBytesSyncImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    items: JString<'local>,
    config: JString<'local>,
) -> jstring {
    let items_str = match jstring_to_string(&mut env, &items) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let config_str = match jstring_to_string(&mut env, &config) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let items_c = match CString::new(items_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid items: {}", e)),
    };

    let config_c = match CString::new(config_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid config: {}", e)),
    };

    let config_ptr = unsafe { kreuzberg_extraction_config_from_json(config_c.as_ptr()) };
    if config_ptr.is_null() {
        let error_msg = get_ffi_error_message();
        return throw_exception(&mut env, &format!("Failed to parse config JSON: {}", error_msg));
    }

    // SAFETY: We have valid pointers from CString
    let result_ptr = unsafe { kreuzberg_batch_extract_bytes_sync(items_c.as_ptr(), config_ptr) };

    if result_ptr.is_null() {
        let error_msg = get_ffi_error_message();
        unsafe {
            kreuzberg_extraction_config_free(config_ptr);
        }
        let detailed_error = format!("Batch extract bytes sync failed: {}", error_msg);
        return throw_exception(&mut env, &detailed_error);
    }

    let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
    // Clean up
    unsafe {
        kreuzberg_extraction_config_free(config_ptr);
        kreuzberg_free_string(result_ptr);
    }
    jstr
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeBatchExtractFilesImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    items: JString<'local>,
    config: JString<'local>,
) -> jstring {
    let items_str = match jstring_to_string(&mut env, &items) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let config_str = match jstring_to_string(&mut env, &config) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let items_c = match CString::new(items_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid items: {}", e)),
    };

    let config_c = match CString::new(config_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid config: {}", e)),
    };

    let config_ptr = unsafe { kreuzberg_extraction_config_from_json(config_c.as_ptr()) };
    if config_ptr.is_null() {
        return throw_exception(&mut env, "Failed to parse config JSON");
    }

    // SAFETY: We have valid pointers from CString
    let result_ptr = unsafe { kreuzberg_batch_extract_files(items_c.as_ptr(), config_ptr) };

    if result_ptr.is_null() {
        unsafe {
            kreuzberg_extraction_config_free(config_ptr);
        }
        return throw_exception(
            &mut env,
            &format!("Batch extract files failed: {}", get_ffi_error_message()),
        );
    }

    let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
    // Clean up
    unsafe {
        kreuzberg_extraction_config_free(config_ptr);
        kreuzberg_free_string(result_ptr);
    }
    jstr
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeBatchExtractBytesImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    items: JString<'local>,
    config: JString<'local>,
) -> jstring {
    let items_str = match jstring_to_string(&mut env, &items) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let config_str = match jstring_to_string(&mut env, &config) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let items_c = match CString::new(items_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid items: {}", e)),
    };

    let config_c = match CString::new(config_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid config: {}", e)),
    };

    let config_ptr = unsafe { kreuzberg_extraction_config_from_json(config_c.as_ptr()) };
    if config_ptr.is_null() {
        return throw_exception(&mut env, "Failed to parse config JSON");
    }

    // SAFETY: We have valid pointers from CString
    let result_ptr = unsafe { kreuzberg_batch_extract_bytes(items_c.as_ptr(), config_ptr) };

    if result_ptr.is_null() {
        unsafe {
            kreuzberg_extraction_config_free(config_ptr);
        }
        return throw_exception(
            &mut env,
            &format!("Batch extract bytes failed: {}", get_ffi_error_message()),
        );
    }

    let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
    // Clean up
    unsafe {
        kreuzberg_extraction_config_free(config_ptr);
        kreuzberg_free_string(result_ptr);
    }
    jstr
}

// ============================================================================
// MIME Type Detection
// ============================================================================

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeDetectMimeTypeFromBytesImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    content: JString<'local>,
) -> jstring {
    let content_str = match jstring_to_string(&mut env, &content) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let content_bytes = content_str.into_bytes();

    // SAFETY: We have a valid slice from Vec
    let result_ptr = unsafe { kreuzberg_detect_mime_type_from_bytes(content_bytes.as_ptr(), content_bytes.len()) };

    if result_ptr.is_null() {
        throw_exception(&mut env, "Detect MIME type failed")
    } else {
        let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
        // Clean up
        unsafe {
            kreuzberg_free_string(result_ptr);
        }
        jstr
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeDetectMimeTypeImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    path: JString<'local>,
    check_exists: jboolean,
) -> jstring {
    let path_str = match jstring_to_string(&mut env, &path) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let path_c = match CString::new(path_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid path: {}", e)),
    };

    // SAFETY: We have a valid C string
    let result_ptr = unsafe { kreuzberg_detect_mime_type(path_c.as_ptr(), check_exists as i32) };

    if result_ptr.is_null() {
        throw_exception(&mut env, "Detect MIME type failed")
    } else {
        let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
        // Clean up
        unsafe {
            kreuzberg_free_string(result_ptr);
        }
        jstr
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeGetExtensionsForMimeImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    mime_type: JString<'local>,
) -> jstring {
    let mime_type_str = match jstring_to_string(&mut env, &mime_type) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };
    let mime_type_c = match CString::new(mime_type_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid mime_type: {}", e)),
    };
    // SAFETY: mime_type_c is a valid C string; FFI returns null on error.
    let result_ptr = unsafe { kreuzberg_get_extensions_for_mime(mime_type_c.as_ptr()) };
    if result_ptr.is_null() {
        return throw_exception(
            &mut env,
            &format!("Get extensions for MIME failed: {}", get_ffi_error_message()),
        );
    }
    let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
    unsafe { kreuzberg_free_string(result_ptr) };
    jstr
}

// ============================================================================
// Embedding Functions
// ============================================================================

// TODO: nativeEmbedTextsImpl removed — the sync embed_texts function is generic and
// cannot be FFI-exported. Use the async variant (embed_texts_async) instead.
// This was a placeholder that was never fully tested — prefer the Kotlin coroutines
// API for async embedding operations.

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeListEmbeddingPresetsImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    // SAFETY: Function takes no parameters
    let result_ptr = unsafe { kreuzberg_list_embedding_presets() };

    if result_ptr.is_null() {
        throw_exception(&mut env, "List embedding presets failed")
    } else {
        let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
        // Clean up
        unsafe {
            kreuzberg_free_string(result_ptr);
        }
        jstr
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeGetEmbeddingPresetImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    name: JString<'local>,
) -> jstring {
    let name_str = match jstring_to_string(&mut env, &name) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let name_c = match CString::new(name_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid name: {}", e)),
    };

    // SAFETY: We have a valid C string.
    // kreuzberg_get_embedding_preset returns a typed *mut EmbeddingPreset, not a
    // JSON string — serialise it via kreuzberg_embedding_preset_to_json before
    // crossing back to the JVM.
    let preset_ptr = unsafe { kreuzberg_get_embedding_preset(name_c.as_ptr()) };

    if preset_ptr.is_null() {
        // Return null (None in Kotlin)
        std::ptr::null_mut()
    } else {
        let json_ptr = unsafe { kreuzberg_embedding_preset_to_json(preset_ptr) };
        let jstr = cstring_ptr_to_jstring(&mut env, json_ptr);
        unsafe {
            kreuzberg_free_string(json_ptr);
            kreuzberg_embedding_preset_free(preset_ptr);
        }
        jstr
    }
}

// ============================================================================
// List/Clear Backend Functions
// ============================================================================

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeListEmbeddingBackendsImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    // SAFETY: Function takes no parameters
    let result_ptr = unsafe { kreuzberg_list_embedding_backends() };

    if result_ptr.is_null() {
        throw_exception(&mut env, "List embedding backends failed")
    } else {
        let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
        // Clean up
        unsafe {
            kreuzberg_free_string(result_ptr);
        }
        jstr
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeClearEmbeddingBackendsImpl(
    mut env: JNIEnv,
    _class: JClass,
) {
    // SAFETY: Function takes no parameters other than out_error
    let mut err_ptr = std::ptr::null_mut();
    let code = unsafe { kreuzberg_clear_embedding_backend(&mut err_ptr) };

    if code != 0 {
        let msg = if err_ptr.is_null() {
            format!("Clear embedding backends failed with code {}", code)
        } else {
            let c_str = unsafe { CStr::from_ptr(err_ptr) };
            c_str.to_string_lossy().to_string()
        };
        throw_exception_void(&mut env, &msg);
        // Clean up error string
        if !err_ptr.is_null() {
            unsafe {
                kreuzberg_free_string(err_ptr);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeListDocumentExtractorsImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    // SAFETY: Function takes no parameters
    let result_ptr = unsafe { kreuzberg_list_document_extractors() };

    if result_ptr.is_null() {
        throw_exception(&mut env, "List document extractors failed")
    } else {
        let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
        // Clean up
        unsafe {
            kreuzberg_free_string(result_ptr);
        }
        jstr
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeClearDocumentExtractorsImpl(
    mut env: JNIEnv,
    _class: JClass,
) {
    // SAFETY: Function takes no parameters other than out_error
    let mut err_ptr = std::ptr::null_mut();
    let code = unsafe { kreuzberg_clear_document_extractor(&mut err_ptr) };

    if code != 0 {
        let msg = if err_ptr.is_null() {
            format!("Clear document extractors failed with code {}", code)
        } else {
            let c_str = unsafe { CStr::from_ptr(err_ptr) };
            c_str.to_string_lossy().to_string()
        };
        throw_exception_void(&mut env, &msg);
        if !err_ptr.is_null() {
            unsafe {
                kreuzberg_free_string(err_ptr);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeListOcrBackendsImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    // SAFETY: Function takes no parameters
    let result_ptr = unsafe { kreuzberg_list_ocr_backends() };

    if result_ptr.is_null() {
        throw_exception(&mut env, "List OCR backends failed")
    } else {
        let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
        // Clean up
        unsafe {
            kreuzberg_free_string(result_ptr);
        }
        jstr
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeClearOcrBackendsImpl(mut env: JNIEnv, _class: JClass) {
    // SAFETY: Function takes no parameters other than out_error
    let mut err_ptr = std::ptr::null_mut();
    let code = unsafe { kreuzberg_clear_ocr_backend(&mut err_ptr) };

    if code != 0 {
        let msg = if err_ptr.is_null() {
            format!("Clear OCR backends failed with code {}", code)
        } else {
            let c_str = unsafe { CStr::from_ptr(err_ptr) };
            c_str.to_string_lossy().to_string()
        };
        throw_exception_void(&mut env, &msg);
        if !err_ptr.is_null() {
            unsafe {
                kreuzberg_free_string(err_ptr);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeListPostProcessorsImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    // SAFETY: Function takes no parameters
    let result_ptr = unsafe { kreuzberg_list_post_processors() };

    if result_ptr.is_null() {
        throw_exception(&mut env, "List post processors failed")
    } else {
        let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
        // Clean up
        unsafe {
            kreuzberg_free_string(result_ptr);
        }
        jstr
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeClearPostProcessorsImpl(
    mut env: JNIEnv,
    _class: JClass,
) {
    // SAFETY: Function takes no parameters other than out_error
    let mut err_ptr = std::ptr::null_mut();
    let code = unsafe { kreuzberg_clear_post_processor(&mut err_ptr) };

    if code != 0 {
        let msg = if err_ptr.is_null() {
            format!("Clear post processors failed with code {}", code)
        } else {
            let c_str = unsafe { CStr::from_ptr(err_ptr) };
            c_str.to_string_lossy().to_string()
        };
        throw_exception_void(&mut env, &msg);
        if !err_ptr.is_null() {
            unsafe {
                kreuzberg_free_string(err_ptr);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeListRenderersImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    // SAFETY: Function takes no parameters
    let result_ptr = unsafe { kreuzberg_list_renderers() };

    if result_ptr.is_null() {
        throw_exception(&mut env, "List renderers failed")
    } else {
        let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
        // Clean up
        unsafe {
            kreuzberg_free_string(result_ptr);
        }
        jstr
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeClearRenderersImpl(mut env: JNIEnv, _class: JClass) {
    // SAFETY: Function takes no parameters other than out_error
    let mut err_ptr = std::ptr::null_mut();
    let code = unsafe { kreuzberg_clear_renderer(&mut err_ptr) };

    if code != 0 {
        let msg = if err_ptr.is_null() {
            format!("Clear renderers failed with code {}", code)
        } else {
            let c_str = unsafe { CStr::from_ptr(err_ptr) };
            c_str.to_string_lossy().to_string()
        };
        throw_exception_void(&mut env, &msg);
        if !err_ptr.is_null() {
            unsafe {
                kreuzberg_free_string(err_ptr);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeListValidatorsImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    // SAFETY: Function takes no parameters
    let result_ptr = unsafe { kreuzberg_list_validators() };

    if result_ptr.is_null() {
        throw_exception(&mut env, "List validators failed")
    } else {
        let jstr = cstring_ptr_to_jstring(&mut env, result_ptr);
        // Clean up
        unsafe {
            kreuzberg_free_string(result_ptr);
        }
        jstr
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeClearValidatorsImpl(mut env: JNIEnv, _class: JClass) {
    // SAFETY: Function takes no parameters other than out_error
    let mut err_ptr = std::ptr::null_mut();
    let code = unsafe { kreuzberg_clear_validator(&mut err_ptr) };

    if code != 0 {
        let msg = if err_ptr.is_null() {
            format!("Clear validators failed with code {}", code)
        } else {
            let c_str = unsafe { CStr::from_ptr(err_ptr) };
            c_str.to_string_lossy().to_string()
        };
        throw_exception_void(&mut env, &msg);
        if !err_ptr.is_null() {
            unsafe {
                kreuzberg_free_string(err_ptr);
            }
        }
    }
}

// ============================================================================
// PDF Rendering
// ============================================================================

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_kreuzberg_KreuzbergBridge_nativeRenderPdfPageToPngImpl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    pdf_bytes: JString<'local>,
    page_index: jlong,
    dpi: jint,
    password: JString<'local>,
) -> jbyteArray {
    let pdf_bytes_str = match jstring_to_string(&mut env, &pdf_bytes) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    let password_str = match jstring_to_string(&mut env, &password) {
        Ok(s) => s,
        Err(e) => return throw_exception(&mut env, &e),
    };

    // The Kotlin wrapper Base64-encodes the PDF bytes before crossing JNI
    // (matching how nativeExtractBytesImpl marshals binary payloads).
    let pdf_bytes_data = match base64_decode(&pdf_bytes_str) {
        Ok(bytes) => bytes,
        Err(_) => pdf_bytes_str.into_bytes(),
    };

    let password_c = match CString::new(password_str) {
        Ok(cs) => cs,
        Err(e) => return throw_exception(&mut env, &format!("Invalid password: {}", e)),
    };

    // FFI returns -1 on failure or fills the out_ptr/out_len/out_cap triple.
    let mut out_ptr: *mut u8 = std::ptr::null_mut();
    let mut out_len: usize = 0;
    let mut out_cap: usize = 0;
    // SAFETY: input pointers are valid for the duration of this call; out-params
    // point to stack locals that outlive the FFI call.
    let rc = unsafe {
        kreuzberg_render_pdf_page_to_png(
            pdf_bytes_data.as_ptr(),
            pdf_bytes_data.len(),
            page_index as usize,
            dpi,
            password_c.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut out_cap,
        )
    };

    if rc != 0 || out_ptr.is_null() {
        throw_exception(
            &mut env,
            &format!("Render PDF page to PNG failed: {}", get_ffi_error_message()),
        );
        return std::ptr::null_mut();
    }

    // Copy the PNG bytes into a JVM-owned byte[] before releasing the FFI buffer.
    // SAFETY: out_ptr/out_len describe the PNG buffer the FFI populated.
    let png_bytes = unsafe { std::slice::from_raw_parts(out_ptr, out_len) }.to_vec();
    unsafe {
        kreuzberg_free_bytes(out_ptr, out_len, out_cap);
    }

    env.with_env(|env| -> Result<jbyteArray, JniError> {
        env.byte_array_from_slice(&png_bytes).map(|ba| ba.into_raw())
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
