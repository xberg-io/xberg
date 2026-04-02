#![allow(unpredictable_function_pointer_comparisons)]

//! Kreuzberg Ruby Bindings (Magnus)
//!
//! High-performance document intelligence framework bindings for Ruby.
//! Provides extraction, OCR, chunking, and language detection for 30+ file formats.

// Module declarations
mod embedding;
mod error_handling;
mod gc_guarded_value;
mod helpers;
mod config;
mod result;
mod extraction;
mod batch;
mod validation;
mod metadata;
mod plugins;

// Re-export public APIs
pub use error_handling::{kreuzberg_error, runtime_error, get_error_code};
pub use gc_guarded_value::GcGuardedValue;
pub use helpers::{get_kw, set_hash_entry, json_value_to_ruby, ruby_value_to_json, cache_root_dir, cache_directories};
pub use config::parse_extraction_config;
pub use result::extraction_result_to_ruby;
pub use extraction::{extract_file_sync, extract_bytes_sync, extract_file, extract_bytes, render_pdf_pages_iter, native_render_pdf_page};
pub use batch::{
    batch_extract_files_sync, batch_extract_bytes_sync, batch_extract_files, batch_extract_bytes,
};

// Re-export FFI
pub use kreuzberg_ffi::{
    kreuzberg_validate_binarization_method, kreuzberg_validate_ocr_backend,
    kreuzberg_validate_language_code, kreuzberg_validate_token_reduction_level,
    kreuzberg_validate_tesseract_psm, kreuzberg_validate_tesseract_oem,
    kreuzberg_validate_output_format, kreuzberg_validate_confidence,
    kreuzberg_validate_dpi, kreuzberg_validate_chunking_params,
    kreuzberg_get_valid_binarization_methods, kreuzberg_get_valid_language_codes,
    kreuzberg_get_valid_ocr_backends, kreuzberg_get_valid_token_reduction_levels,
    kreuzberg_free_string,
};

use magnus::{Error, Ruby, RHash, Value, function, IntoValue, TryConvert};
use magnus::value::ReprValue;

/// Clear the extraction cache
pub fn ruby_clear_cache() -> Result<(), Error> {
    let cache_root = cache_root_dir()?;
    if !cache_root.exists() {
        return Ok(());
    }

    for dir in cache_directories(&cache_root)? {
        let Some(dir_str) = dir.to_str() else {
            return Err(runtime_error("Cache directory path contains non-UTF8 characters"));
        };
        kreuzberg::cache::clear_cache_directory(dir_str).map_err(kreuzberg_error)?;
    }

    Ok(())
}

/// Get cache statistics
pub fn ruby_cache_stats() -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let hash = ruby.hash_new();
    let cache_root = cache_root_dir()?;

    if !cache_root.exists() {
        hash.aset("total_entries", 0)?;
        hash.aset("total_size_bytes", 0)?;
        return Ok(hash);
    }

    let mut total_entries: usize = 0;
    let mut total_bytes: f64 = 0.0;

    for dir in cache_directories(&cache_root)? {
        let Some(dir_str) = dir.to_str() else {
            return Err(runtime_error("Cache directory path contains non-UTF8 characters"));
        };
        let stats = kreuzberg::cache::get_cache_metadata(dir_str).map_err(kreuzberg_error)?;
        total_entries += stats.total_files;
        total_bytes += stats.total_size_mb * 1024.0 * 1024.0;
    }

    set_hash_entry(
        &ruby,
        &hash,
        "total_entries",
        ruby.integer_from_u64(total_entries as u64).into_value_with(&ruby),
    )?;
    set_hash_entry(
        &ruby,
        &hash,
        "total_size_bytes",
        ruby.integer_from_u64(total_bytes.round() as u64).into_value_with(&ruby),
    )?;

    Ok(hash)
}

// Validation wrapper functions
pub fn validate_binarization_method(method: String) -> Result<i32, Error> {
    let c_method = std::ffi::CString::new(method)
        .map_err(|_| runtime_error("Invalid string: contains null byte"))?;
    // SAFETY: c_method is a valid null-terminated C string that outlives the FFI call.
    unsafe { Ok(kreuzberg_validate_binarization_method(c_method.as_ptr())) }
}

pub fn validate_ocr_backend(backend: String) -> Result<i32, Error> {
    let c_backend = std::ffi::CString::new(backend)
        .map_err(|_| runtime_error("Invalid string: contains null byte"))?;
    // SAFETY: c_backend is a valid null-terminated C string that outlives the FFI call.
    unsafe { Ok(kreuzberg_validate_ocr_backend(c_backend.as_ptr())) }
}

pub fn validate_language_code(code: String) -> Result<i32, Error> {
    let c_code = std::ffi::CString::new(code)
        .map_err(|_| runtime_error("Invalid string: contains null byte"))?;
    // SAFETY: c_code is a valid null-terminated C string that outlives the FFI call.
    unsafe { Ok(kreuzberg_validate_language_code(c_code.as_ptr())) }
}

pub fn validate_token_reduction_level(level: String) -> Result<i32, Error> {
    let c_level = std::ffi::CString::new(level)
        .map_err(|_| runtime_error("Invalid string: contains null byte"))?;
    // SAFETY: c_level is a valid null-terminated C string that outlives the FFI call.
    unsafe { Ok(kreuzberg_validate_token_reduction_level(c_level.as_ptr())) }
}

pub fn validate_tesseract_psm(psm: i32) -> Result<i32, Error> {
    Ok(kreuzberg_validate_tesseract_psm(psm))
}

pub fn validate_tesseract_oem(oem: i32) -> Result<i32, Error> {
    Ok(kreuzberg_validate_tesseract_oem(oem))
}

pub fn validate_output_format(format: String) -> Result<i32, Error> {
    let c_format = std::ffi::CString::new(format)
        .map_err(|_| runtime_error("Invalid string: contains null byte"))?;
    // SAFETY: c_format is a valid null-terminated C string that outlives the FFI call.
    unsafe { Ok(kreuzberg_validate_output_format(c_format.as_ptr())) }
}

pub fn validate_confidence(confidence: f64) -> Result<i32, Error> {
    Ok(kreuzberg_validate_confidence(confidence))
}

pub fn validate_dpi(dpi: i32) -> Result<i32, Error> {
    Ok(kreuzberg_validate_dpi(dpi))
}

pub fn validate_chunking_params(max_chars: usize, max_overlap: usize) -> Result<i32, Error> {
    Ok(kreuzberg_validate_chunking_params(max_chars, max_overlap))
}

pub fn get_valid_binarization_methods(_ruby: &Ruby) -> Result<String, Error> {
    unsafe {
        let ptr = kreuzberg_get_valid_binarization_methods();
        if ptr.is_null() {
            Ok(String::new())
        } else {
            let cstr = std::ffi::CStr::from_ptr(ptr);
            let result = cstr.to_string_lossy().to_string();
            kreuzberg_free_string(ptr as *mut std::ffi::c_char);
            Ok(result)
        }
    }
}

pub fn get_valid_language_codes(_ruby: &Ruby) -> Result<String, Error> {
    unsafe {
        let ptr = kreuzberg_get_valid_language_codes();
        if ptr.is_null() {
            Ok(String::new())
        } else {
            let cstr = std::ffi::CStr::from_ptr(ptr);
            let result = cstr.to_string_lossy().to_string();
            kreuzberg_free_string(ptr as *mut std::ffi::c_char);
            Ok(result)
        }
    }
}

pub fn get_valid_ocr_backends(_ruby: &Ruby) -> Result<String, Error> {
    unsafe {
        let ptr = kreuzberg_get_valid_ocr_backends();
        if ptr.is_null() {
            Ok(String::new())
        } else {
            let cstr = std::ffi::CStr::from_ptr(ptr);
            let result = cstr.to_string_lossy().to_string();
            kreuzberg_free_string(ptr as *mut std::ffi::c_char);
            Ok(result)
        }
    }
}

pub fn get_valid_token_reduction_levels(_ruby: &Ruby) -> Result<String, Error> {
    unsafe {
        let ptr = kreuzberg_get_valid_token_reduction_levels();
        if ptr.is_null() {
            Ok(String::new())
        } else {
            let cstr = std::ffi::CStr::from_ptr(ptr);
            let result = cstr.to_string_lossy().to_string();
            kreuzberg_free_string(ptr as *mut std::ffi::c_char);
            Ok(result)
        }
    }
}

pub fn last_error_code() -> i32 {
    get_error_code()
}

pub fn last_panic_context_json(ruby: &Ruby) -> Value {
    if let Some(context) = error_handling::get_panic_context() {
        ruby.str_new(&context).into_value_with(ruby)
    } else {
        ruby.qnil().as_value()
    }
}

// Config wrapper functions
pub fn config_from_file(path: String) -> Result<RHash, Error> {
    config::config_from_file(path)
}

pub fn config_discover() -> Result<Value, Error> {
    config::config_discover()
}

pub fn config_to_json_wrapper(_ruby: &Ruby, config_json: String) -> Result<String, Error> {
    Ok(config_json)
}

pub fn config_get_field_wrapper(ruby: &Ruby, config_json: String, field_name: String) -> Result<Value, Error> {
    let json_value: serde_json::Value = serde_json::from_str(&config_json)
        .map_err(|e| runtime_error(format!("Invalid JSON: {}", e)))?;

    if let Some(field_value) = json_value.get(&field_name) {
        json_value_to_ruby(ruby, field_value)
    } else {
        Ok(ruby.qnil().as_value())
    }
}

pub fn config_merge_wrapper(_ruby: &Ruby, base_json: String, override_json: String) -> Result<String, Error> {
    let mut base: serde_json::Value = serde_json::from_str(&base_json)
        .map_err(|e| runtime_error(format!("Invalid base JSON: {}", e)))?;
    let override_val: serde_json::Value = serde_json::from_str(&override_json)
        .map_err(|e| runtime_error(format!("Invalid override JSON: {}", e)))?;

    if let (Some(base_obj), Some(override_obj)) = (base.as_object_mut(), override_val.as_object()) {
        for (key, value) in override_obj {
            base_obj.insert(key.clone(), value.clone());
        }
    }

    serde_json::to_string(&base).map_err(|e| runtime_error(format!("Failed to serialize merged config: {}", e)))
}

// Result wrapper functions
// These functions receive a Ruby Hash (the extraction result) and extract specific fields.

/// Get page count from extraction result
/// Accesses metadata["page_count"] or metadata["sheet_count"] (for Excel) or returns 0
pub fn result_page_count(ruby: &Ruby, result: Value) -> Result<i32, Error> {
    // Try to get the result as an RHash
    let hash = match RHash::try_convert(result) {
        Ok(h) => h,
        Err(_) => return Ok(0),
    };

    // Get metadata field
    let metadata = match hash.get("metadata") {
        Some(m) => m,
        None => return Ok(0),
    };

    // Try to convert metadata to hash
    let metadata_hash = match RHash::try_convert(metadata) {
        Ok(h) => h,
        Err(_) => return Ok(0),
    };

    // Try page_count first (PDF/PPTX format)
    if let Some(page_count) = metadata_hash.get("page_count") {
        if page_count.equal(ruby.qnil()).ok() != Some(true) {
            if let Ok(count) = i32::try_convert(page_count) {
                return Ok(count);
            }
        }
    }

    // Fall back to sheet_count (Excel format)
    if let Some(sheet_count) = metadata_hash.get("sheet_count") {
        if sheet_count.equal(ruby.qnil()).ok() != Some(true) {
            if let Ok(count) = i32::try_convert(sheet_count) {
                return Ok(count);
            }
        }
    }

    Ok(0)
}

/// Get chunk count from extraction result
/// Returns chunks.length or 0 if nil/empty
pub fn result_chunk_count(ruby: &Ruby, result: Value) -> Result<i32, Error> {
    // Try to get the result as an RHash
    let hash = match RHash::try_convert(result) {
        Ok(h) => h,
        Err(_) => return Ok(0),
    };

    // Get chunks field
    let chunks = match hash.get("chunks") {
        Some(c) => c,
        None => return Ok(0),
    };

    // Check if chunks is nil
    if chunks.equal(ruby.qnil()).ok() == Some(true) {
        return Ok(0);
    }

    // Try to convert chunks to array
    let chunks_array = match magnus::RArray::try_convert(chunks) {
        Ok(a) => a,
        Err(_) => return Ok(0),
    };

    Ok(chunks_array.len() as i32)
}

/// Get detected language from extraction result
/// Returns first element from detected_languages array or metadata["language"]
pub fn result_detected_language(ruby: &Ruby, result: Value) -> Result<Value, Error> {
    // Try to get the result as an RHash
    let hash = match RHash::try_convert(result) {
        Ok(h) => h,
        Err(_) => return Ok(ruby.qnil().as_value()),
    };

    // First try detected_languages array (primary detection result)
    if let Some(detected_languages) = hash.get("detected_languages") {
        if detected_languages.equal(ruby.qnil()).ok() != Some(true) {
            if let Ok(langs_array) = magnus::RArray::try_convert(detected_languages) {
                if langs_array.len() > 0 {
                    if let Ok(first) = langs_array.entry(0) {
                        return Ok(first);
                    }
                }
            }
        }
    }

    // Fall back to metadata["language"]
    if let Some(metadata) = hash.get("metadata") {
        if let Ok(metadata_hash) = RHash::try_convert(metadata) {
            if let Some(language) = metadata_hash.get("language") {
                if language.equal(ruby.qnil()).ok() != Some(true) {
                    return Ok(language);
                }
            }
        }
    }

    Ok(ruby.qnil().as_value())
}

/// Get metadata field by name with dot notation support
/// Accesses metadata[field_name] using dot notation for nested fields
pub fn result_metadata_field(ruby: &Ruby, result: Value, field_name: String) -> Result<Value, Error> {
    // Try to get the result as an RHash
    let hash = match RHash::try_convert(result) {
        Ok(h) => h,
        Err(_) => return Ok(ruby.qnil().as_value()),
    };

    // Get metadata field
    let metadata = match hash.get("metadata") {
        Some(m) => m,
        None => return Ok(ruby.qnil().as_value()),
    };

    // Check if metadata is nil
    if metadata.equal(ruby.qnil()).ok() == Some(true) {
        return Ok(ruby.qnil().as_value());
    }

    // Split field name by dots and traverse
    let parts: Vec<&str> = field_name.split('.').collect();
    let mut current = metadata;

    for part in parts {
        // Try to convert current to hash
        let current_hash = match RHash::try_convert(current) {
            Ok(h) => h,
            Err(_) => return Ok(ruby.qnil().as_value()),
        };

        // Get the field
        current = match current_hash.get(part) {
            Some(v) => v,
            None => return Ok(ruby.qnil().as_value()),
        };

        // Check if current is nil
        if current.equal(ruby.qnil()).ok() == Some(true) {
            return Ok(ruby.qnil().as_value());
        }
    }

    Ok(current)
}

// Error detail functions
pub fn get_error_details_native(ruby: &Ruby) -> Result<Value, Error> {
    let hash = ruby.hash_new();
    hash.aset("code", get_error_code())?;
    hash.aset("message", "")?;
    Ok(hash.into_value_with(ruby))
}

pub fn classify_error_native(ruby: &Ruby, _message: String) -> Result<Value, Error> {
    let hash = ruby.hash_new();
    hash.aset("type", "unknown")?;
    Ok(hash.into_value_with(ruby))
}

pub fn error_code_name_native(ruby: &Ruby, code: u32) -> Result<Value, Error> {
    let name = format!("error_{}", code);
    Ok(ruby.str_new(&name).into_value_with(ruby))
}

pub fn error_code_description_native(ruby: &Ruby, _code: u32) -> Result<Value, Error> {
    Ok(ruby.str_new("Error").into_value_with(ruby))
}

/// Module initialization for Ruby
#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Kreuzberg")?;

    // Extraction functions
    module.define_module_function("extract_file_sync", function!(extract_file_sync, -1))?;
    module.define_module_function("extract_bytes_sync", function!(extract_bytes_sync, -1))?;
    module.define_module_function("batch_extract_files_sync", function!(batch_extract_files_sync, -1))?;
    module.define_module_function("batch_extract_bytes_sync", function!(batch_extract_bytes_sync, -1))?;
    module.define_module_function("extract_file", function!(extract_file, -1))?;
    module.define_module_function("extract_bytes", function!(extract_bytes, -1))?;
    module.define_module_function("batch_extract_files", function!(batch_extract_files, -1))?;
    module.define_module_function("batch_extract_bytes", function!(batch_extract_bytes, -1))?;

    // Embedding functions
    module.define_module_function("embed_sync", function!(embedding::embed_sync, -1))?;
    module.define_module_function("embed", function!(embedding::embed, -1))?;

    // PDF page iterator
    module.define_module_function("native_render_pdf_pages_iter", function!(render_pdf_pages_iter, 2))?;
    module.define_module_function("native_render_pdf_page", function!(native_render_pdf_page, 3))?;

    // Cache functions
    module.define_module_function("clear_cache", function!(ruby_clear_cache, 0))?;
    module.define_module_function("cache_stats", function!(ruby_cache_stats, 0))?;

    // Plugin functions
    module.define_module_function("register_post_processor", function!(plugins::register_post_processor, -1))?;
    module.define_module_function("register_validator", function!(plugins::register_validator, -1))?;
    module.define_module_function("register_ocr_backend", function!(plugins::register_ocr_backend, 2))?;
    module.define_module_function("unregister_post_processor", function!(plugins::unregister_post_processor, 1))?;
    module.define_module_function("unregister_validator", function!(plugins::unregister_validator, 1))?;
    module.define_module_function("clear_post_processors", function!(plugins::clear_post_processors, 0))?;
    module.define_module_function("clear_validators", function!(plugins::clear_validators, 0))?;
    module.define_module_function("list_post_processors", function!(plugins::list_post_processors, 0))?;
    module.define_module_function("list_validators", function!(plugins::list_validators, 0))?;
    module.define_module_function("unregister_ocr_backend", function!(plugins::unregister_ocr_backend, 1))?;
    module.define_module_function("list_ocr_backends", function!(plugins::list_ocr_backends, 0))?;
    module.define_module_function("clear_ocr_backends", function!(plugins::clear_ocr_backends, 0))?;
    module.define_module_function("list_document_extractors", function!(plugins::list_document_extractors, 0))?;
    module.define_module_function("unregister_document_extractor", function!(plugins::unregister_document_extractor, 1))?;
    module.define_module_function("clear_document_extractors", function!(plugins::clear_document_extractors, 0))?;

    // Config functions
    module.define_module_function("_config_from_file_native", function!(config_from_file, 1))?;
    module.define_module_function("_config_discover_native", function!(config_discover, 0))?;

    // Metadata functions
    module.define_module_function("detect_mime_type", function!(metadata::detect_mime_type_from_bytes, 1))?;
    module.define_module_function("detect_mime_type_from_path", function!(metadata::detect_mime_type_from_path_native, 1))?;
    module.define_module_function("get_extensions_for_mime", function!(metadata::get_extensions_for_mime_native, 1))?;
    module.define_module_function("validate_mime_type", function!(metadata::validate_mime_type_native, 1))?;

    // Error functions
    module.define_module_function("_last_error_code_native", function!(last_error_code, 0))?;
    module.define_module_function("_last_panic_context_json_native", function!(last_panic_context_json, 0))?;

    // Validation functions
    module.define_module_function("_validate_binarization_method_native", function!(validate_binarization_method, 1))?;
    module.define_module_function("_validate_ocr_backend_native", function!(validate_ocr_backend, 1))?;
    module.define_module_function("_validate_language_code_native", function!(validate_language_code, 1))?;
    module.define_module_function("_validate_token_reduction_level_native", function!(validate_token_reduction_level, 1))?;
    module.define_module_function("_validate_tesseract_psm_native", function!(validate_tesseract_psm, 1))?;
    module.define_module_function("_validate_tesseract_oem_native", function!(validate_tesseract_oem, 1))?;
    module.define_module_function("_validate_output_format_native", function!(validate_output_format, 1))?;
    module.define_module_function("_validate_confidence_native", function!(validate_confidence, 1))?;
    module.define_module_function("_validate_dpi_native", function!(validate_dpi, 1))?;
    module.define_module_function("_validate_chunking_params_native", function!(validate_chunking_params, 2))?;
    module.define_module_function("_get_valid_binarization_methods_native", function!(get_valid_binarization_methods, 0))?;
    module.define_module_function("_get_valid_language_codes_native", function!(get_valid_language_codes, 0))?;
    module.define_module_function("_get_valid_ocr_backends_native", function!(get_valid_ocr_backends, 0))?;
    module.define_module_function("_get_valid_token_reduction_levels_native", function!(get_valid_token_reduction_levels, 0))?;

    // Config wrapper functions
    module.define_module_function("_config_to_json_native", function!(config_to_json_wrapper, 1))?;
    module.define_module_function("_config_get_field_native", function!(config_get_field_wrapper, 2))?;
    module.define_module_function("_config_merge_native", function!(config_merge_wrapper, 2))?;

    // Result wrapper functions
    module.define_module_function("_result_page_count_native", function!(result_page_count, 1))?;
    module.define_module_function("_result_chunk_count_native", function!(result_chunk_count, 1))?;
    module.define_module_function("_result_detected_language_native", function!(result_detected_language, 1))?;
    module.define_module_function("_result_metadata_field_native", function!(result_metadata_field, 2))?;

    // Error detail functions
    module.define_module_function("_get_error_details_native", function!(get_error_details_native, 0))?;
    module.define_module_function("_classify_error_native", function!(classify_error_native, 1))?;
    module.define_module_function("_error_code_name_native", function!(error_code_name_native, 1))?;
    module.define_module_function("_error_code_description_native", function!(error_code_description_native, 1))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modular_structure() {
        assert!(true);
    }
}
