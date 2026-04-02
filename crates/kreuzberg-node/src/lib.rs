#![deny(clippy::all)]

mod worker_pool;
mod worker_pool_api;

// Re-export worker pool APIs
pub use worker_pool_api::{
    JsWorkerPool, WorkerPoolStats, batch_extract_files_in_worker, close_worker_pool, create_worker_pool,
    extract_file_in_worker, get_worker_pool_stats,
};

// Module declarations
mod batch;
mod config;
mod embedding_presets;
mod error_handling;
mod extraction;
mod metadata;
mod plugins;
mod result;
mod validation;

// Re-export all public items from modules
pub use error_handling::{ErrorClassification, classify_error, get_error_code_description, get_error_code_name};

pub use config::{
    JsChunkingConfig, JsEmbeddingConfig, JsEmbeddingModelType, JsExtractionConfig, JsFileExtractionConfig,
    JsHierarchyConfig, JsHtmlOptions, JsHtmlPreprocessingOptions, JsImageExtractionConfig, JsKeywordConfig,
    JsLanguageDetectionConfig, JsOcrConfig, JsPageConfig, JsPdfConfig, JsPostProcessorConfig, JsRakeParams,
    JsTesseractConfig, JsTokenReductionConfig, JsYakeParams, discover_extraction_config,
    load_extraction_config_from_file,
};

pub use result::{
    JsChunk, JsChunkMetadata, JsExtractedImage, JsExtractedKeyword, JsExtractionResult, JsHierarchicalBlock,
    JsPageContent, JsPageHierarchy, JsProcessingWarning, JsTable,
};

pub use extraction::{
    JsPdfPageIterator, PdfPageResult, extract_bytes, extract_bytes_sync, extract_file, extract_file_sync,
    iterate_pdf_pages, iterate_pdf_pages_sync, pdf_page_count, render_pdf_page, render_pdf_page_sync,
};

pub use batch::{batch_extract_bytes, batch_extract_bytes_sync, batch_extract_files, batch_extract_files_sync};

pub use validation::{
    config_get_field_internal, config_merge_internal, config_validate_and_normalize, get_extensions_for_mime,
    get_last_error_code, get_last_panic_context, get_valid_binarization_methods, get_valid_language_codes,
    get_valid_ocr_backends, get_valid_token_reduction_levels, validate_binarization_method, validate_chunking_params,
    validate_confidence, validate_dpi, validate_language_code, validate_mime_type, validate_ocr_backend,
    validate_output_format, validate_tesseract_oem, validate_tesseract_psm, validate_token_reduction_level,
};

pub use metadata::{
    clear_document_extractors, detect_mime_type_from_bytes, detect_mime_type_from_path, list_document_extractors,
    unregister_document_extractor,
};

pub use embedding_presets::{EmbeddingPreset, embed, embed_sync, get_embedding_preset, list_embedding_presets};

pub use plugins::{
    clear_ocr_backends, clear_post_processors, clear_validators, list_ocr_backends, list_post_processors,
    list_validators, register_ocr_backend, register_post_processor, register_validator, unregister_ocr_backend,
    unregister_post_processor, unregister_validator,
};

// Core imports for utilities and FFI types
use ahash::AHashSet;
use kreuzberg::{ExtractionConfig, ExtractionResult as RustExtractionResult, KNOWN_FORMATS};
use once_cell::sync::Lazy;
use std::ffi::{CStr, c_char};

static KNOWN_FORMAT_FIELDS: Lazy<AHashSet<&'static str>> = Lazy::new(|| KNOWN_FORMATS.iter().copied().collect());

#[ctor::ctor]
fn setup_onnx_runtime_path() {
    kreuzberg::ort_discovery::ensure_ort_available();
}

#[allow(unused_extern_crates)]
extern crate kreuzberg_ffi;

/// Metadata field structure returned from FFI
/// Mirrors CMetadataField from kreuzberg-ffi/src/result.rs
#[repr(C)]
pub struct CMetadataField {
    name: *const c_char,
    json_value: *mut c_char,
    is_null: i32,
}

#[allow(improper_ctypes)]
unsafe extern "C" {
    /// Get the last error code from FFI.
    ///
    /// Maps to kreuzberg_last_error_code() in the FFI library.
    /// This is thread-safe and always safe to call.
    pub fn kreuzberg_last_error_code() -> i32;

    /// Get the last panic context as JSON from FFI.
    ///
    /// Maps to kreuzberg_last_panic_context() in the FFI library.
    /// Returns NULL if no panic context is available.
    /// The returned string must be freed with kreuzberg_free_string().
    pub fn kreuzberg_last_panic_context() -> *const c_char;

    /// Free a string allocated by FFI.
    ///
    /// Maps to kreuzberg_free_string() in the FFI library.
    pub fn kreuzberg_free_string(ptr: *mut c_char);

    pub fn kreuzberg_validate_binarization_method(method: *const c_char) -> i32;
    pub fn kreuzberg_validate_ocr_backend(backend: *const c_char) -> i32;
    pub fn kreuzberg_validate_language_code(code: *const c_char) -> i32;
    pub fn kreuzberg_validate_token_reduction_level(level: *const c_char) -> i32;
    pub fn kreuzberg_validate_tesseract_psm(psm: i32) -> i32;
    pub fn kreuzberg_validate_tesseract_oem(oem: i32) -> i32;
    pub fn kreuzberg_validate_output_format(format: *const c_char) -> i32;
    pub fn kreuzberg_validate_confidence(confidence: f64) -> i32;
    pub fn kreuzberg_validate_dpi(dpi: i32) -> i32;
    pub fn kreuzberg_validate_chunking_params(max_characters: usize, overlap: usize) -> i32;

    pub fn kreuzberg_get_valid_binarization_methods() -> *mut c_char;
    pub fn kreuzberg_get_valid_language_codes() -> *mut c_char;
    pub fn kreuzberg_get_valid_ocr_backends() -> *mut c_char;
    pub fn kreuzberg_get_valid_token_reduction_levels() -> *mut c_char;

    pub fn kreuzberg_config_from_json(json_config: *const c_char) -> *mut ExtractionConfig;
    pub fn kreuzberg_config_free(config: *mut ExtractionConfig);
    pub fn kreuzberg_config_to_json(config: *const ExtractionConfig) -> *mut c_char;
    pub fn kreuzberg_config_get_field(config: *const ExtractionConfig, field_name: *const c_char) -> *mut c_char;
    pub fn kreuzberg_config_merge(base: *mut ExtractionConfig, override_config: *const ExtractionConfig) -> i32;

    pub fn kreuzberg_result_get_page_count(result: *const RustExtractionResult) -> i32;
    pub fn kreuzberg_result_get_chunk_count(result: *const RustExtractionResult) -> i32;
    pub fn kreuzberg_result_get_detected_language(result: *const RustExtractionResult) -> *mut c_char;
    pub fn kreuzberg_result_get_metadata_field(
        result: *const RustExtractionResult,
        field_name: *const c_char,
    ) -> CMetadataField;

    /// Get the name of an error code as a C string.
    /// Returns pointer to static string valid for program lifetime.
    /// Example: kreuzberg_error_code_name(0) -> "validation"
    pub fn kreuzberg_error_code_name(code: u32) -> *const c_char;

    /// Get the description of an error code as a C string.
    /// Returns pointer to static string valid for program lifetime.
    /// Example: kreuzberg_error_code_description(0) -> "Input validation error"
    pub fn kreuzberg_error_code_description(code: u32) -> *const c_char;

    /// Get the validation error code constant (0)
    pub fn kreuzberg_error_code_validation() -> u32;

    /// Get the parsing error code constant (1)
    pub fn kreuzberg_error_code_parsing() -> u32;

    /// Get the OCR error code constant (2)
    pub fn kreuzberg_error_code_ocr() -> u32;

    /// Get the missing dependency error code constant (3)
    pub fn kreuzberg_error_code_missing_dependency() -> u32;

    /// Get the I/O error code constant (4)
    pub fn kreuzberg_error_code_io() -> u32;

    /// Get the plugin error code constant (5)
    pub fn kreuzberg_error_code_plugin() -> u32;

    /// Get the unsupported format error code constant (6)
    pub fn kreuzberg_error_code_unsupported_format() -> u32;

    /// Get the internal error code constant (7)
    pub fn kreuzberg_error_code_internal() -> u32;

    /// Get the embedding error code constant (8)
    pub fn kreuzberg_error_code_embedding() -> u32;

    /// Get the total count of valid error codes (9)
    pub fn kreuzberg_error_code_count() -> u32;
}

static WORKER_POOL: std::sync::LazyLock<tokio::runtime::Runtime> = std::sync::LazyLock::new(|| {
    let worker_count = std::env::var("KREUZBERG_WORKER_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_else(num_cpus::get);

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_count)
        .enable_all()
        .build()
        .expect("Failed to create Tokio worker thread pool")
});

/// Helper function to retrieve panic context from FFI.
///
/// Calls kreuzberg_last_panic_context() and parses the JSON response into a panic context object.
/// Returns None if no panic context is available or if parsing fails.
#[inline]
fn get_panic_context() -> Option<serde_json::Value> {
    unsafe {
        let ptr = kreuzberg_last_panic_context();
        if ptr.is_null() {
            return None;
        }

        let c_str = CStr::from_ptr(ptr);
        if let Ok(json_str) = c_str.to_str() {
            let result = serde_json::from_str(json_str).ok();
            kreuzberg_free_string(ptr as *mut c_char);
            return result;
        }

        kreuzberg_free_string(ptr as *mut c_char);
        None
    }
}
