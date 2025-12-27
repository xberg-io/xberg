#![deny(clippy::all)]

use base64::Engine;
use html_to_markdown_rs::options::{
    CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle,
    PreprocessingOptions as HtmlPreprocessingOptions, PreprocessingPreset, WhitespaceMode,
};
use kreuzberg::keywords::{
    KeywordAlgorithm as RustKeywordAlgorithm, KeywordConfig as RustKeywordConfig, RakeParams as RustRakeParams,
    YakeParams as RustYakeParams,
};
use kreuzberg::plugins::registry::{get_post_processor_registry, get_validator_registry};
use kreuzberg::{
    Chunk as RustChunk, ChunkMetadata as RustChunkMetadata, ChunkingConfig as RustChunkingConfig,
    EmbeddingConfig as RustEmbeddingConfig, EmbeddingModelType as RustEmbeddingModelType, ExtractionConfig,
    ExtractionResult as RustExtractionResult, ImageExtractionConfig as RustImageExtractionConfig, KNOWN_FORMATS,
    LanguageDetectionConfig as RustLanguageDetectionConfig, OcrConfig as RustOcrConfig, PdfConfig as RustPdfConfig,
    PostProcessorConfig as RustPostProcessorConfig, TesseractConfig as RustTesseractConfig,
    TokenReductionConfig as RustTokenReductionConfig,
};
use lazy_static::lazy_static;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::ffi::{CStr, c_char};

static KNOWN_FORMAT_FIELDS: Lazy<HashSet<&'static str>> = Lazy::new(|| KNOWN_FORMATS.iter().copied().collect());

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
    pub fn kreuzberg_validate_chunking_params(max_chars: usize, max_overlap: usize) -> i32;

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

    /// Get the total count of valid error codes (8)
    pub fn kreuzberg_error_code_count() -> u32;
}

lazy_static! {
    static ref WORKER_POOL: tokio::runtime::Runtime = {
        let worker_count = std::env::var("KREUZBERG_WORKER_THREADS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or_else(num_cpus::get);

        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_count)
            .enable_all()
            .build()
            .expect("Failed to create Tokio worker thread pool")
    };
}

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

/// Converts KreuzbergError to NAPI Error with specific error codes.
///
/// This function maps Kreuzberg error variants to appropriate NAPI status codes,
/// preserving error semantics for JavaScript/TypeScript callers:
///
/// - `Io` → GenericFailure (system-level I/O errors)
/// - `Parsing` → InvalidArg (malformed documents, corrupt files)
/// - `Ocr` → GenericFailure (OCR processing failures)
/// - `Validation` → InvalidArg (invalid configuration or parameters)
/// - `Cache` → GenericFailure (non-fatal cache errors)
/// - `ImageProcessing` → GenericFailure (image manipulation errors)
/// - `Serialization` → InvalidArg (JSON/MessagePack errors)
/// - `MissingDependency` → GenericFailure (missing system dependencies)
/// - `Plugin` → GenericFailure (plugin-specific errors)
/// - `LockPoisoned` → GenericFailure (lock poisoning, should not happen)
/// - `UnsupportedFormat` → InvalidArg (unsupported MIME types)
/// - `Other` → GenericFailure (catch-all)
///
/// # Usage
///
/// ```rust,ignore
/// kreuzberg::extract_file_sync(&path, None, &config)
///     .map_err(convert_error)
///     .and_then(JsExtractionResult::try_from)
/// ```
#[inline]
fn convert_error(err: kreuzberg::KreuzbergError) -> napi::Error {
    use kreuzberg::KreuzbergError;

    match err {
        KreuzbergError::Io(e) => Error::new(Status::GenericFailure, format!("IO error: {}", e)),

        KreuzbergError::Parsing { message, .. } => {
            Error::new(Status::InvalidArg, format!("Parsing error: {}", message))
        }

        KreuzbergError::Ocr { message, .. } => Error::new(Status::GenericFailure, format!("OCR error: {}", message)),

        KreuzbergError::Validation { message, .. } => {
            Error::new(Status::InvalidArg, format!("Validation error: {}", message))
        }

        KreuzbergError::Cache { message, .. } => {
            Error::new(Status::GenericFailure, format!("Cache error: {}", message))
        }

        KreuzbergError::ImageProcessing { message, .. } => {
            Error::new(Status::GenericFailure, format!("Image processing error: {}", message))
        }

        KreuzbergError::Serialization { message, .. } => {
            Error::new(Status::InvalidArg, format!("Serialization error: {}", message))
        }

        KreuzbergError::MissingDependency(dep) => {
            Error::new(Status::GenericFailure, format!("Missing dependency: {}", dep))
        }

        KreuzbergError::Plugin { message, plugin_name } => Error::new(
            Status::GenericFailure,
            format!("Plugin error in '{}': {}", plugin_name, message),
        ),

        KreuzbergError::LockPoisoned(msg) => Error::new(Status::GenericFailure, format!("Lock poisoned: {}", msg)),

        KreuzbergError::UnsupportedFormat(format) => {
            Error::new(Status::InvalidArg, format!("Unsupported format: {}", format))
        }

        KreuzbergError::Other(msg) => Error::new(Status::GenericFailure, msg),
    }
}

/// Validates that a JavaScript object has all required properties before plugin registration.
///
/// This helper function checks if a plugin object has all required methods and provides
/// clear error messages if any are missing. This improves developer experience by
/// catching configuration errors early with actionable error messages.
///
/// # Arguments
///
/// * `obj` - The JavaScript object to validate
/// * `plugin_type` - Type of plugin (for error messages, e.g., "PostProcessor")
/// * `required_methods` - Slice of required method names
///
/// # Returns
///
/// Returns `Ok(())` if all required methods exist, or an error with details about
/// which methods are missing.
///
/// # Example
///
/// ```rust,ignore
/// validate_plugin_object(&processor, "PostProcessor", &["name", "process"])?;
/// ```
fn validate_plugin_object(obj: &Object, plugin_type: &str, required_methods: &[&str]) -> Result<()> {
    let mut missing_methods = Vec::new();

    for method_name in required_methods {
        if !obj.has_named_property(method_name)? {
            missing_methods.push(*method_name);
        }
    }

    if !missing_methods.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "{} is missing required methods: {}. Please ensure your plugin implements all required methods.",
                plugin_type,
                missing_methods.join(", ")
            ),
        ));
    }

    Ok(())
}

/// Returns the human-readable name for an error code.
///
/// Maps to FFI function kreuzberg_error_code_name().
///
/// # Arguments
///
/// * `code` - Numeric error code (0-7)
///
/// # Returns
///
/// A string containing the error code name (e.g., "validation", "ocr", "unknown")
///
/// # Examples
///
/// ```typescript
/// const name = getErrorCodeName(0);  // returns "validation"
/// const name = getErrorCodeName(2);  // returns "ocr"
/// const name = getErrorCodeName(99); // returns "unknown"
/// ```
#[napi]
pub fn get_error_code_name(code: u32) -> String {
    unsafe {
        let ptr = kreuzberg_error_code_name(code);
        if ptr.is_null() {
            "unknown".to_string()
        } else {
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown").to_string()
        }
    }
}

/// Returns the description for an error code.
///
/// Maps to FFI function kreuzberg_error_code_description().
///
/// # Arguments
///
/// * `code` - Numeric error code (0-7)
///
/// # Returns
///
/// A string containing a brief description of the error
///
/// # Examples
///
/// ```typescript
/// const desc = getErrorCodeDescription(0);  // returns "Input validation error"
/// const desc = getErrorCodeDescription(4);  // returns "File system I/O error"
/// const desc = getErrorCodeDescription(99); // returns "Unknown error code"
/// ```
#[napi]
pub fn get_error_code_description(code: u32) -> String {
    unsafe {
        let ptr = kreuzberg_error_code_description(code);
        if ptr.is_null() {
            "Unknown error code".to_string()
        } else {
            CStr::from_ptr(ptr).to_str().unwrap_or("Unknown error code").to_string()
        }
    }
}

/// Classifies an error message string into an error code category.
///
/// This function analyzes the error message content and returns the most likely
/// error code (0-7) based on keyword patterns. Used to programmatically classify
/// errors for handling purposes.
///
/// # Arguments
///
/// * `error_message` - The error message string to classify
///
/// # Returns
///
/// An object with:
/// - `code`: The numeric error code (0-7)
/// - `name`: The error code name string
/// - `description`: Brief description of the error type
/// - `confidence`: Confidence score (0.0-1.0) of the classification
///
/// # Classification Rules
///
/// - **Validation (0)**: Keywords: invalid, validation, invalid_argument, schema, required, unexpected field
/// - **Parsing (1)**: Keywords: parsing, parse_error, corrupted, malformed, invalid format, decode, encoding
/// - **Ocr (2)**: Keywords: ocr, optical, character, recognition, tesseract, language, model
/// - **MissingDependency (3)**: Keywords: not found, not installed, missing, dependency, require, unavailable
/// - **Io (4)**: Keywords: io, file, disk, read, write, permission, access, path
/// - **Plugin (5)**: Keywords: plugin, register, extension, handler, processor
/// - **UnsupportedFormat (6)**: Keywords: unsupported, format, mime, type, codec
/// - **Internal (7)**: Keywords: internal, bug, panic, unexpected, invariant
///
/// # Examples
///
/// ```typescript
/// const result = classifyError("PDF file is corrupted");
/// // Returns: { code: 1, name: "parsing", confidence: 0.95 }
///
/// const result = classifyError("Tesseract not found");
/// // Returns: { code: 3, name: "missing_dependency", confidence: 0.9 }
/// ```
#[napi(object)]
pub struct ErrorClassification {
    pub code: u32,
    pub name: String,
    pub description: String,
    pub confidence: f64,
}

#[napi]
pub fn classify_error(error_message: String) -> ErrorClassification {
    let lower = error_message.to_lowercase();

    let (code, confidence) = if lower.contains("validation")
        || lower.contains("invalid_argument")
        || lower.contains("schema")
        || lower.contains("required")
        || lower.contains("unexpected field")
    {
        (0u32, 0.9)
    } else if lower.contains("parsing")
        || lower.contains("parse_error")
        || lower.contains("corrupted")
        || lower.contains("malformed")
        || lower.contains("invalid format")
        || lower.contains("decode")
        || lower.contains("encoding")
    {
        (1u32, 0.85)
    } else if lower.contains("ocr")
        || lower.contains("optical")
        || lower.contains("character")
        || lower.contains("recognition")
        || lower.contains("tesseract")
        || lower.contains("language")
        || lower.contains("model")
    {
        (2u32, 0.88)
    } else if lower.contains("not found")
        || lower.contains("not installed")
        || lower.contains("missing")
        || lower.contains("dependency")
        || lower.contains("require")
        || lower.contains("unavailable")
    {
        (3u32, 0.92)
    } else if lower.contains("io")
        || lower.contains("file")
        || lower.contains("disk")
        || lower.contains("read")
        || lower.contains("write")
        || lower.contains("permission")
        || lower.contains("access")
        || lower.contains("path")
    {
        (4u32, 0.87)
    } else if lower.contains("plugin")
        || lower.contains("register")
        || lower.contains("extension")
        || lower.contains("handler")
        || lower.contains("processor")
    {
        (5u32, 0.84)
    } else if lower.contains("unsupported")
        || lower.contains("format")
        || lower.contains("mime")
        || lower.contains("type")
        || lower.contains("codec")
    {
        (6u32, 0.83)
    } else if lower.contains("internal")
        || lower.contains("bug")
        || lower.contains("panic")
        || lower.contains("unexpected")
        || lower.contains("invariant")
    {
        (7u32, 0.86)
    } else {
        (7u32, 0.1)
    };

    let name = get_error_code_name(code);
    let description = get_error_code_description(code);

    ErrorClassification {
        code,
        name,
        description,
        confidence,
    }
}

#[napi(object)]
pub struct JsOcrConfig {
    pub backend: String,
    pub language: Option<String>,
    pub tesseract_config: Option<JsTesseractConfig>,
}

impl From<JsOcrConfig> for RustOcrConfig {
    fn from(val: JsOcrConfig) -> Self {
        RustOcrConfig {
            backend: val.backend,
            language: val.language.unwrap_or_else(|| "eng".to_string()),
            tesseract_config: val.tesseract_config.map(Into::into),
        }
    }
}

#[napi(object)]
pub struct JsTesseractConfig {
    pub psm: Option<i32>,
    pub enable_table_detection: Option<bool>,
    pub tessedit_char_whitelist: Option<String>,
}

impl From<JsTesseractConfig> for RustTesseractConfig {
    fn from(val: JsTesseractConfig) -> Self {
        let mut config = RustTesseractConfig::default();
        if let Some(psm) = val.psm {
            config.psm = psm;
        }
        if let Some(enabled) = val.enable_table_detection {
            config.enable_table_detection = enabled;
        }
        if let Some(whitelist) = val.tessedit_char_whitelist {
            config.tessedit_char_whitelist = whitelist;
        }
        config
    }
}

/// Embedding model type configuration for Node.js bindings.
///
/// This struct represents different embedding model sources:
/// - `preset`: Use a named preset (e.g., "balanced", "fast", "quality", "multilingual")
/// - `fastembed`: Use a FastEmbed model with custom dimensions
/// - `custom`: Use a custom ONNX model
#[napi(object)]
pub struct JsEmbeddingModelType {
    /// Type of model: "preset", "fastembed", or "custom"
    pub model_type: String,
    /// For preset: preset name; for fastembed/custom: model ID
    pub value: String,
    /// Number of dimensions (only for fastembed/custom)
    pub dimensions: Option<u32>,
}

impl From<JsEmbeddingModelType> for RustEmbeddingModelType {
    fn from(val: JsEmbeddingModelType) -> Self {
        match val.model_type.as_str() {
            "preset" => RustEmbeddingModelType::Preset { name: val.value },
            "fastembed" => RustEmbeddingModelType::FastEmbed {
                model: val.value,
                dimensions: val.dimensions.unwrap_or(768) as usize,
            },
            "custom" => RustEmbeddingModelType::Custom {
                model_id: val.value,
                dimensions: val.dimensions.unwrap_or(512) as usize,
            },
            _ => RustEmbeddingModelType::Preset {
                name: "balanced".to_string(),
            },
        }
    }
}

/// Embedding generation configuration for Node.js bindings.
#[napi(object)]
pub struct JsEmbeddingConfig {
    /// Embedding model configuration
    pub model: Option<JsEmbeddingModelType>,
    /// Whether to normalize embeddings (L2 normalization)
    pub normalize: Option<bool>,
    /// Batch size for embedding generation
    pub batch_size: Option<u32>,
    /// Whether to show download progress for models
    pub show_download_progress: Option<bool>,
    /// Custom cache directory for model storage
    pub cache_dir: Option<String>,
}

impl From<JsEmbeddingConfig> for RustEmbeddingConfig {
    fn from(val: JsEmbeddingConfig) -> Self {
        RustEmbeddingConfig {
            model: val.model.map(Into::into).unwrap_or(RustEmbeddingModelType::Preset {
                name: "balanced".to_string(),
            }),
            normalize: val.normalize.unwrap_or(true),
            batch_size: val.batch_size.unwrap_or(32) as usize,
            show_download_progress: val.show_download_progress.unwrap_or(false),
            cache_dir: val.cache_dir.map(std::path::PathBuf::from),
        }
    }
}

#[napi(object)]
pub struct JsChunkingConfig {
    pub max_chars: Option<u32>,
    pub max_overlap: Option<u32>,
    /// Optional embedding configuration for generating embeddings
    pub embedding: Option<JsEmbeddingConfig>,
    /// Optional preset name for chunking parameters
    pub preset: Option<String>,
}

impl From<JsChunkingConfig> for RustChunkingConfig {
    fn from(val: JsChunkingConfig) -> Self {
        RustChunkingConfig {
            max_chars: val.max_chars.unwrap_or(1000) as usize,
            max_overlap: val.max_overlap.unwrap_or(200) as usize,
            embedding: val.embedding.map(Into::into),
            preset: val.preset,
        }
    }
}

#[napi(object)]
pub struct JsLanguageDetectionConfig {
    pub enabled: Option<bool>,
    pub min_confidence: Option<f64>,
    pub detect_multiple: Option<bool>,
}

impl From<JsLanguageDetectionConfig> for RustLanguageDetectionConfig {
    fn from(val: JsLanguageDetectionConfig) -> Self {
        RustLanguageDetectionConfig {
            enabled: val.enabled.unwrap_or(true),
            min_confidence: val.min_confidence.unwrap_or(0.8),
            detect_multiple: val.detect_multiple.unwrap_or(false),
        }
    }
}

#[napi(object)]
pub struct JsTokenReductionConfig {
    pub mode: Option<String>,
    pub preserve_important_words: Option<bool>,
}

impl From<JsTokenReductionConfig> for RustTokenReductionConfig {
    fn from(val: JsTokenReductionConfig) -> Self {
        RustTokenReductionConfig {
            mode: val.mode.unwrap_or_else(|| "off".to_string()),
            preserve_important_words: val.preserve_important_words.unwrap_or(true),
        }
    }
}

#[napi(object)]
pub struct JsPdfConfig {
    pub extract_images: Option<bool>,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: Option<bool>,
}

impl From<JsPdfConfig> for RustPdfConfig {
    fn from(val: JsPdfConfig) -> Self {
        RustPdfConfig {
            extract_images: val.extract_images.unwrap_or(false),
            passwords: val.passwords,
            extract_metadata: val.extract_metadata.unwrap_or(true),
        }
    }
}

#[napi(object)]
pub struct JsImageExtractionConfig {
    pub extract_images: Option<bool>,
    pub target_dpi: Option<i32>,
    pub max_image_dimension: Option<i32>,
    pub auto_adjust_dpi: Option<bool>,
    pub min_dpi: Option<i32>,
    pub max_dpi: Option<i32>,
}

impl From<JsImageExtractionConfig> for RustImageExtractionConfig {
    fn from(val: JsImageExtractionConfig) -> Self {
        RustImageExtractionConfig {
            extract_images: val.extract_images.unwrap_or(true),
            target_dpi: val.target_dpi.unwrap_or(300),
            max_image_dimension: val.max_image_dimension.unwrap_or(4096),
            auto_adjust_dpi: val.auto_adjust_dpi.unwrap_or(true),
            min_dpi: val.min_dpi.unwrap_or(72),
            max_dpi: val.max_dpi.unwrap_or(600),
        }
    }
}

#[napi(object)]
pub struct JsPostProcessorConfig {
    pub enabled: Option<bool>,
    pub enabled_processors: Option<Vec<String>>,
    pub disabled_processors: Option<Vec<String>>,
}

impl From<JsPostProcessorConfig> for RustPostProcessorConfig {
    fn from(val: JsPostProcessorConfig) -> Self {
        let enabled_set = val
            .enabled_processors
            .as_ref()
            .map(|procs| procs.iter().cloned().collect());
        let disabled_set = val
            .disabled_processors
            .as_ref()
            .map(|procs| procs.iter().cloned().collect());

        RustPostProcessorConfig {
            enabled: val.enabled.unwrap_or(true),
            enabled_processors: val.enabled_processors,
            disabled_processors: val.disabled_processors,
            enabled_set,
            disabled_set,
        }
    }
}

#[napi(object)]
#[derive(Clone)]
pub struct JsHtmlPreprocessingOptions {
    pub enabled: Option<bool>,
    pub preset: Option<String>,
    pub remove_navigation: Option<bool>,
    pub remove_forms: Option<bool>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsHtmlOptions {
    pub heading_style: Option<String>,
    pub list_indent_type: Option<String>,
    pub list_indent_width: Option<u32>,
    pub bullets: Option<String>,
    pub strong_em_symbol: Option<String>,
    pub escape_asterisks: Option<bool>,
    pub escape_underscores: Option<bool>,
    pub escape_misc: Option<bool>,
    pub escape_ascii: Option<bool>,
    pub code_language: Option<String>,
    pub autolinks: Option<bool>,
    pub default_title: Option<bool>,
    pub br_in_tables: Option<bool>,
    pub hocr_spatial_tables: Option<bool>,
    pub highlight_style: Option<String>,
    pub extract_metadata: Option<bool>,
    pub whitespace_mode: Option<String>,
    pub strip_newlines: Option<bool>,
    pub wrap: Option<bool>,
    pub wrap_width: Option<u32>,
    pub convert_as_inline: Option<bool>,
    pub sub_symbol: Option<String>,
    pub sup_symbol: Option<String>,
    pub newline_style: Option<String>,
    pub code_block_style: Option<String>,
    pub keep_inline_images_in: Option<Vec<String>>,
    pub encoding: Option<String>,
    pub debug: Option<bool>,
    pub strip_tags: Option<Vec<String>>,
    pub preserve_tags: Option<Vec<String>>,
    pub preprocessing: Option<JsHtmlPreprocessingOptions>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsYakeParams {
    pub window_size: Option<u32>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsRakeParams {
    pub min_word_length: Option<u32>,
    pub max_words_per_phrase: Option<u32>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsKeywordConfig {
    pub algorithm: Option<String>,
    pub max_keywords: Option<u32>,
    pub min_score: Option<f64>,
    #[napi(ts_type = "[number, number] | undefined")]
    pub ngram_range: Option<Vec<u32>>,
    pub language: Option<String>,
    pub yake_params: Option<JsYakeParams>,
    pub rake_params: Option<JsRakeParams>,
}

impl TryFrom<JsHtmlOptions> for ConversionOptions {
    type Error = Error;

    fn try_from(options: JsHtmlOptions) -> Result<Self> {
        let mut opts = ConversionOptions::default();

        if let Some(style) = options.heading_style {
            opts.heading_style = parse_heading_style(&style)?;
        }
        if let Some(indent) = options.list_indent_type {
            opts.list_indent_type = parse_list_indent_type(&indent)?;
        }
        if let Some(width) = options.list_indent_width {
            opts.list_indent_width = width as usize;
        }
        if let Some(bullets) = options.bullets {
            opts.bullets = bullets;
        }
        if let Some(symbol) = options.strong_em_symbol {
            let mut chars = symbol.chars();
            let first = chars.next().ok_or_else(|| {
                Error::new(
                    Status::InvalidArg,
                    "htmlOptions.strongEmSymbol must contain at least one character",
                )
            })?;
            opts.strong_em_symbol = first;
        }
        if let Some(value) = options.escape_asterisks {
            opts.escape_asterisks = value;
        }
        if let Some(value) = options.escape_underscores {
            opts.escape_underscores = value;
        }
        if let Some(value) = options.escape_misc {
            opts.escape_misc = value;
        }
        if let Some(value) = options.escape_ascii {
            opts.escape_ascii = value;
        }
        if let Some(language) = options.code_language {
            opts.code_language = language;
        }
        if let Some(value) = options.autolinks {
            opts.autolinks = value;
        }
        if let Some(value) = options.default_title {
            opts.default_title = value;
        }
        if let Some(value) = options.br_in_tables {
            opts.br_in_tables = value;
        }
        if let Some(value) = options.hocr_spatial_tables {
            opts.hocr_spatial_tables = value;
        }
        if let Some(style) = options.highlight_style {
            opts.highlight_style = parse_highlight_style(&style)?;
        }
        if let Some(value) = options.extract_metadata {
            opts.extract_metadata = value;
        }
        if let Some(mode) = options.whitespace_mode {
            opts.whitespace_mode = parse_whitespace_mode(&mode)?;
        }
        if let Some(value) = options.strip_newlines {
            opts.strip_newlines = value;
        }
        if let Some(value) = options.wrap {
            opts.wrap = value;
        }
        if let Some(width) = options.wrap_width {
            opts.wrap_width = width as usize;
        }
        if let Some(value) = options.convert_as_inline {
            opts.convert_as_inline = value;
        }
        if let Some(symbol) = options.sub_symbol {
            opts.sub_symbol = symbol;
        }
        if let Some(symbol) = options.sup_symbol {
            opts.sup_symbol = symbol;
        }
        if let Some(style) = options.newline_style {
            opts.newline_style = parse_newline_style(&style)?;
        }
        if let Some(style) = options.code_block_style {
            opts.code_block_style = parse_code_block_style(&style)?;
        }
        if let Some(tags) = options.keep_inline_images_in {
            opts.keep_inline_images_in = tags;
        }
        if let Some(encoding) = options.encoding {
            opts.encoding = encoding;
        }
        if let Some(debug) = options.debug {
            opts.debug = debug;
        }
        if let Some(tags) = options.strip_tags {
            opts.strip_tags = tags;
        }
        if let Some(tags) = options.preserve_tags {
            opts.preserve_tags = tags;
        }
        if let Some(pre) = options.preprocessing {
            let mut preprocessing = opts.preprocessing.clone();
            if let Some(enabled) = pre.enabled {
                preprocessing.enabled = enabled;
            }
            if let Some(preset) = pre.preset {
                preprocessing.preset = parse_preprocessing_preset(&preset)?;
            }
            if let Some(remove_navigation) = pre.remove_navigation {
                preprocessing.remove_navigation = remove_navigation;
            }
            if let Some(remove_forms) = pre.remove_forms {
                preprocessing.remove_forms = remove_forms;
            }
            opts.preprocessing = preprocessing;
        }

        Ok(opts)
    }
}

impl From<&HtmlPreprocessingOptions> for JsHtmlPreprocessingOptions {
    fn from(opts: &HtmlPreprocessingOptions) -> Self {
        Self {
            enabled: Some(opts.enabled),
            preset: Some(preprocessing_preset_to_string(opts.preset).to_string()),
            remove_navigation: Some(opts.remove_navigation),
            remove_forms: Some(opts.remove_forms),
        }
    }
}

impl From<&ConversionOptions> for JsHtmlOptions {
    fn from(opts: &ConversionOptions) -> Self {
        Self {
            heading_style: Some(heading_style_to_string(opts.heading_style).to_string()),
            list_indent_type: Some(list_indent_type_to_string(opts.list_indent_type).to_string()),
            list_indent_width: Some(opts.list_indent_width as u32),
            bullets: Some(opts.bullets.clone()),
            strong_em_symbol: Some(opts.strong_em_symbol.to_string()),
            escape_asterisks: Some(opts.escape_asterisks),
            escape_underscores: Some(opts.escape_underscores),
            escape_misc: Some(opts.escape_misc),
            escape_ascii: Some(opts.escape_ascii),
            code_language: Some(opts.code_language.clone()),
            autolinks: Some(opts.autolinks),
            default_title: Some(opts.default_title),
            br_in_tables: Some(opts.br_in_tables),
            hocr_spatial_tables: Some(opts.hocr_spatial_tables),
            highlight_style: Some(highlight_style_to_string(opts.highlight_style).to_string()),
            extract_metadata: Some(opts.extract_metadata),
            whitespace_mode: Some(whitespace_mode_to_string(opts.whitespace_mode).to_string()),
            strip_newlines: Some(opts.strip_newlines),
            wrap: Some(opts.wrap),
            wrap_width: Some(opts.wrap_width as u32),
            convert_as_inline: Some(opts.convert_as_inline),
            sub_symbol: Some(opts.sub_symbol.clone()),
            sup_symbol: Some(opts.sup_symbol.clone()),
            newline_style: Some(newline_style_to_string(opts.newline_style).to_string()),
            code_block_style: Some(code_block_style_to_string(opts.code_block_style).to_string()),
            keep_inline_images_in: Some(opts.keep_inline_images_in.clone()),
            encoding: Some(opts.encoding.clone()),
            debug: Some(opts.debug),
            strip_tags: Some(opts.strip_tags.clone()),
            preserve_tags: Some(opts.preserve_tags.clone()),
            preprocessing: Some(JsHtmlPreprocessingOptions::from(&opts.preprocessing)),
        }
    }
}

impl TryFrom<JsKeywordConfig> for RustKeywordConfig {
    type Error = Error;

    fn try_from(config: JsKeywordConfig) -> Result<Self> {
        let mut keywords = RustKeywordConfig::default();

        if let Some(max) = config.max_keywords {
            keywords.max_keywords = max as usize;
        }
        if let Some(score) = config.min_score {
            keywords.min_score = score as f32;
        }
        if let Some(range) = config.ngram_range {
            if range.len() != 2 {
                return Err(Error::new(
                    Status::InvalidArg,
                    "keywords.ngramRange must contain exactly two elements",
                ));
            }
            keywords.ngram_range = (range[0] as usize, range[1] as usize);
        }
        if let Some(language) = config.language {
            keywords.language = Some(language);
        }
        if let Some(algorithm) = config.algorithm {
            keywords.algorithm = parse_keyword_algorithm(&algorithm)?;
        }
        if let Some(yake) = config.yake_params {
            let mut params = RustYakeParams::default();
            if let Some(window) = yake.window_size {
                params.window_size = window as usize;
            }
            keywords.yake_params = Some(params);
        }
        if let Some(rake) = config.rake_params {
            let mut params = RustRakeParams::default();
            if let Some(min_len) = rake.min_word_length {
                params.min_word_length = min_len as usize;
            }
            if let Some(max_words) = rake.max_words_per_phrase {
                params.max_words_per_phrase = max_words as usize;
            }
            keywords.rake_params = Some(params);
        }

        Ok(keywords)
    }
}

impl From<RustKeywordConfig> for JsKeywordConfig {
    fn from(config: RustKeywordConfig) -> Self {
        Self {
            algorithm: Some(keyword_algorithm_to_string(config.algorithm).to_string()),
            max_keywords: Some(config.max_keywords as u32),
            min_score: Some(config.min_score as f64),
            ngram_range: Some(vec![config.ngram_range.0 as u32, config.ngram_range.1 as u32]),
            language: config.language,
            yake_params: config.yake_params.map(|params| JsYakeParams {
                window_size: Some(params.window_size as u32),
            }),
            rake_params: config.rake_params.map(|params| JsRakeParams {
                min_word_length: Some(params.min_word_length as u32),
                max_words_per_phrase: Some(params.max_words_per_phrase as u32),
            }),
        }
    }
}

fn parse_heading_style(value: &str) -> Result<HeadingStyle> {
    match value.to_lowercase().as_str() {
        "atx" => Ok(HeadingStyle::Atx),
        "underlined" => Ok(HeadingStyle::Underlined),
        "atx_closed" | "atx-closed" => Ok(HeadingStyle::AtxClosed),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.headingStyle '{}'", other),
        )),
    }
}

fn heading_style_to_string(style: HeadingStyle) -> &'static str {
    match style {
        HeadingStyle::Atx => "atx",
        HeadingStyle::Underlined => "underlined",
        HeadingStyle::AtxClosed => "atx_closed",
    }
}

fn parse_list_indent_type(value: &str) -> Result<ListIndentType> {
    match value.to_lowercase().as_str() {
        "spaces" => Ok(ListIndentType::Spaces),
        "tabs" => Ok(ListIndentType::Tabs),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.listIndentType '{}'", other),
        )),
    }
}

fn list_indent_type_to_string(value: ListIndentType) -> &'static str {
    match value {
        ListIndentType::Spaces => "spaces",
        ListIndentType::Tabs => "tabs",
    }
}

fn parse_highlight_style(value: &str) -> Result<HighlightStyle> {
    match value.to_lowercase().as_str() {
        "double_equal" | "==" | "double-equal" => Ok(HighlightStyle::DoubleEqual),
        "html" => Ok(HighlightStyle::Html),
        "bold" => Ok(HighlightStyle::Bold),
        "none" => Ok(HighlightStyle::None),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.highlightStyle '{}'", other),
        )),
    }
}

fn highlight_style_to_string(style: HighlightStyle) -> &'static str {
    match style {
        HighlightStyle::DoubleEqual => "double_equal",
        HighlightStyle::Html => "html",
        HighlightStyle::Bold => "bold",
        HighlightStyle::None => "none",
    }
}

fn parse_whitespace_mode(value: &str) -> Result<WhitespaceMode> {
    match value.to_lowercase().as_str() {
        "normalized" => Ok(WhitespaceMode::Normalized),
        "strict" => Ok(WhitespaceMode::Strict),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.whitespaceMode '{}'", other),
        )),
    }
}

fn whitespace_mode_to_string(mode: WhitespaceMode) -> &'static str {
    match mode {
        WhitespaceMode::Normalized => "normalized",
        WhitespaceMode::Strict => "strict",
    }
}

fn parse_newline_style(value: &str) -> Result<NewlineStyle> {
    match value.to_lowercase().as_str() {
        "spaces" => Ok(NewlineStyle::Spaces),
        "backslash" => Ok(NewlineStyle::Backslash),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.newlineStyle '{}'", other),
        )),
    }
}

fn newline_style_to_string(value: NewlineStyle) -> &'static str {
    match value {
        NewlineStyle::Spaces => "spaces",
        NewlineStyle::Backslash => "backslash",
    }
}

fn parse_code_block_style(value: &str) -> Result<CodeBlockStyle> {
    match value.to_lowercase().as_str() {
        "indented" => Ok(CodeBlockStyle::Indented),
        "backticks" => Ok(CodeBlockStyle::Backticks),
        "tildes" => Ok(CodeBlockStyle::Tildes),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.codeBlockStyle '{}'", other),
        )),
    }
}

fn code_block_style_to_string(value: CodeBlockStyle) -> &'static str {
    match value {
        CodeBlockStyle::Indented => "indented",
        CodeBlockStyle::Backticks => "backticks",
        CodeBlockStyle::Tildes => "tildes",
    }
}

fn parse_preprocessing_preset(value: &str) -> Result<PreprocessingPreset> {
    match value.to_lowercase().as_str() {
        "minimal" => Ok(PreprocessingPreset::Minimal),
        "standard" => Ok(PreprocessingPreset::Standard),
        "aggressive" => Ok(PreprocessingPreset::Aggressive),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.preprocessing.preset '{}'", other),
        )),
    }
}

fn preprocessing_preset_to_string(preset: PreprocessingPreset) -> &'static str {
    match preset {
        PreprocessingPreset::Minimal => "minimal",
        PreprocessingPreset::Standard => "standard",
        PreprocessingPreset::Aggressive => "aggressive",
    }
}

fn parse_keyword_algorithm(value: &str) -> Result<RustKeywordAlgorithm> {
    match value.to_lowercase().as_str() {
        "yake" => Ok(RustKeywordAlgorithm::Yake),
        "rake" => Ok(RustKeywordAlgorithm::Rake),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid keywords.algorithm '{}'. Expected 'yake' or 'rake'", other),
        )),
    }
}

fn keyword_algorithm_to_string(algo: RustKeywordAlgorithm) -> &'static str {
    match algo {
        RustKeywordAlgorithm::Yake => "yake",
        RustKeywordAlgorithm::Rake => "rake",
    }
}

#[napi(object)]
pub struct JsPageConfig {
    pub extract_pages: Option<bool>,
    pub insert_page_markers: Option<bool>,
    pub marker_format: Option<String>,
}

#[napi(object)]
pub struct JsExtractionConfig {
    pub use_cache: Option<bool>,
    pub enable_quality_processing: Option<bool>,
    pub ocr: Option<JsOcrConfig>,
    pub force_ocr: Option<bool>,
    pub chunking: Option<JsChunkingConfig>,
    pub images: Option<JsImageExtractionConfig>,
    pub pdf_options: Option<JsPdfConfig>,
    pub token_reduction: Option<JsTokenReductionConfig>,
    pub language_detection: Option<JsLanguageDetectionConfig>,
    pub postprocessor: Option<JsPostProcessorConfig>,
    pub keywords: Option<JsKeywordConfig>,
    pub html_options: Option<JsHtmlOptions>,
    pub max_concurrent_extractions: Option<u32>,
    pub pages: Option<JsPageConfig>,
}

impl TryFrom<JsPageConfig> for kreuzberg::core::config::PageConfig {
    type Error = Error;

    fn try_from(val: JsPageConfig) -> Result<Self> {
        Ok(kreuzberg::core::config::PageConfig {
            extract_pages: val.extract_pages.unwrap_or(false),
            insert_page_markers: val.insert_page_markers.unwrap_or(false),
            marker_format: val
                .marker_format
                .unwrap_or_else(|| "\n\n<!-- PAGE {page_num} -->\n\n".to_string()),
        })
    }
}

impl From<kreuzberg::core::config::PageConfig> for JsPageConfig {
    fn from(config: kreuzberg::core::config::PageConfig) -> Self {
        Self {
            extract_pages: Some(config.extract_pages),
            insert_page_markers: Some(config.insert_page_markers),
            marker_format: Some(config.marker_format),
        }
    }
}

impl TryFrom<JsExtractionConfig> for ExtractionConfig {
    type Error = Error;

    fn try_from(val: JsExtractionConfig) -> Result<Self> {
        let html_options = match val.html_options {
            Some(options) => Some(ConversionOptions::try_from(options)?),
            None => None,
        };

        let keywords = match val.keywords {
            Some(config) => Some(RustKeywordConfig::try_from(config)?),
            None => None,
        };

        Ok(ExtractionConfig {
            use_cache: val.use_cache.unwrap_or(true),
            enable_quality_processing: val.enable_quality_processing.unwrap_or(true),
            ocr: val.ocr.map(Into::into),
            force_ocr: val.force_ocr.unwrap_or(false),
            chunking: val.chunking.map(Into::into),
            images: val.images.map(Into::into),
            pdf_options: val.pdf_options.map(Into::into),
            token_reduction: val.token_reduction.map(Into::into),
            language_detection: val.language_detection.map(Into::into),
            keywords,
            postprocessor: val.postprocessor.map(Into::into),
            html_options,
            max_concurrent_extractions: val.max_concurrent_extractions.map(|v| v as usize),
            pages: val.pages.map(|p| p.try_into()).transpose()?,
        })
    }
}

impl TryFrom<ExtractionConfig> for JsExtractionConfig {
    type Error = napi::Error;

    fn try_from(val: ExtractionConfig) -> Result<Self> {
        Ok(JsExtractionConfig {
            use_cache: Some(val.use_cache),
            enable_quality_processing: Some(val.enable_quality_processing),
            ocr: val.ocr.map(|ocr| JsOcrConfig {
                backend: ocr.backend,
                language: Some(ocr.language),
                tesseract_config: ocr.tesseract_config.map(|tc| JsTesseractConfig {
                    psm: Some(tc.psm),
                    enable_table_detection: Some(tc.enable_table_detection),
                    tessedit_char_whitelist: if tc.tessedit_char_whitelist.is_empty() {
                        None
                    } else {
                        Some(tc.tessedit_char_whitelist)
                    },
                }),
            }),
            force_ocr: Some(val.force_ocr),
            chunking: val.chunking.map(|chunk| JsChunkingConfig {
                max_chars: Some(chunk.max_chars as u32),
                max_overlap: Some(chunk.max_overlap as u32),
                embedding: chunk.embedding.map(|emb| JsEmbeddingConfig {
                    model: Some(JsEmbeddingModelType {
                        model_type: match emb.model {
                            RustEmbeddingModelType::Preset { .. } => "preset".to_string(),
                            RustEmbeddingModelType::FastEmbed { .. } => "fastembed".to_string(),
                            RustEmbeddingModelType::Custom { .. } => "custom".to_string(),
                        },
                        value: match &emb.model {
                            RustEmbeddingModelType::Preset { name } => name.clone(),
                            RustEmbeddingModelType::FastEmbed { model, .. } => model.clone(),
                            RustEmbeddingModelType::Custom { model_id, .. } => model_id.clone(),
                        },
                        dimensions: match emb.model {
                            RustEmbeddingModelType::FastEmbed { dimensions, .. } => Some(dimensions as u32),
                            RustEmbeddingModelType::Custom { dimensions, .. } => Some(dimensions as u32),
                            _ => None,
                        },
                    }),
                    normalize: Some(emb.normalize),
                    batch_size: Some(emb.batch_size as u32),
                    show_download_progress: Some(emb.show_download_progress),
                    cache_dir: emb.cache_dir.and_then(|p| p.to_str().map(String::from)),
                }),
                preset: chunk.preset,
            }),
            images: val.images.map(|img| JsImageExtractionConfig {
                extract_images: Some(img.extract_images),
                target_dpi: Some(img.target_dpi),
                max_image_dimension: Some(img.max_image_dimension),
                auto_adjust_dpi: Some(img.auto_adjust_dpi),
                min_dpi: Some(img.min_dpi),
                max_dpi: Some(img.max_dpi),
            }),
            pdf_options: val.pdf_options.map(|pdf| JsPdfConfig {
                extract_images: Some(pdf.extract_images),
                passwords: pdf.passwords,
                extract_metadata: Some(pdf.extract_metadata),
            }),
            token_reduction: val.token_reduction.map(|tr| JsTokenReductionConfig {
                mode: Some(tr.mode),
                preserve_important_words: Some(tr.preserve_important_words),
            }),
            language_detection: val.language_detection.map(|ld| JsLanguageDetectionConfig {
                enabled: Some(ld.enabled),
                min_confidence: Some(ld.min_confidence),
                detect_multiple: Some(ld.detect_multiple),
            }),
            postprocessor: val.postprocessor.map(|pp| JsPostProcessorConfig {
                enabled: Some(pp.enabled),
                enabled_processors: pp.enabled_processors,
                disabled_processors: pp.disabled_processors,
            }),
            keywords: val.keywords.map(JsKeywordConfig::from),
            html_options: val.html_options.as_ref().map(JsHtmlOptions::from),
            max_concurrent_extractions: val.max_concurrent_extractions.map(|v| v as u32),
            pages: val.pages.map(JsPageConfig::from),
        })
    }
}

/// Load extraction configuration from a file.
///
/// Automatically detects the file format based on extension:
/// - `.toml` - TOML format
/// - `.yaml` - YAML format
/// - `.json` - JSON format
///
/// # Parameters
///
/// * `file_path` - Path to the configuration file (absolute or relative)
///
/// # Returns
///
/// `JsExtractionConfig` object with loaded configuration.
///
/// # Errors
///
/// Throws an error if:
/// - File does not exist or is not accessible
/// - File content is not valid TOML/YAML/JSON
/// - Configuration structure is invalid
///
/// # Example
///
/// ```typescript
/// import { loadExtractionConfigFromFile } from 'kreuzberg';
///
/// // Load from TOML file
/// const config = loadExtractionConfigFromFile('kreuzberg.toml');
///
/// // Load from YAML file
/// const config2 = loadExtractionConfigFromFile('./config.yaml');
///
/// // Use with extraction
/// const result = await extractFile('document.pdf', null, config);
/// ```
#[napi(js_name = "loadExtractionConfigFromFile")]
pub fn load_extraction_config_from_file(file_path: String) -> Result<JsExtractionConfig> {
    let path = std::path::Path::new(&file_path);

    let ext = path.extension().and_then(|e| e.to_str()).ok_or_else(|| {
        Error::new(
            Status::InvalidArg,
            "File path must have an extension (.toml, .yaml, .json)",
        )
    })?;

    let rust_config = match ext.to_lowercase().as_str() {
        "toml" => ExtractionConfig::from_toml_file(path).map_err(convert_error)?,
        "yaml" | "yml" => ExtractionConfig::from_yaml_file(path).map_err(convert_error)?,
        "json" => ExtractionConfig::from_json_file(path).map_err(convert_error)?,
        _ => {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Unsupported file extension: '{}'. Supported: .toml, .yaml, .json", ext),
            ));
        }
    };

    JsExtractionConfig::try_from(rust_config)
}

/// Discover and load extraction configuration from current or parent directories.
///
/// Searches for a `kreuzberg.toml` file starting from the current working directory
/// and traversing up the directory tree. Returns the first configuration file found.
///
/// # Returns
///
/// `JsExtractionConfig` object if a configuration file is found, or `null` if no
/// configuration file exists in the current or parent directories.
///
/// # Example
///
/// ```typescript
/// import { ExtractionConfig } from 'kreuzberg';
///
/// // Try to find config in current or parent directories
/// const config = ExtractionConfig.discover();
/// if (config) {
///   console.log('Found configuration');
///   // Use config for extraction
/// } else {
///   console.log('No configuration file found, using defaults');
/// }
/// ```
#[napi(js_name = "discoverExtractionConfig")]
pub fn discover_extraction_config() -> Result<Option<JsExtractionConfig>> {
    let rust_config = ExtractionConfig::discover().map_err(convert_error)?;

    match rust_config {
        Some(config) => Ok(Some(JsExtractionConfig::try_from(config)?)),
        None => Ok(None),
    }
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsTable {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: u32,
}

#[napi(object)]
pub struct JsExtractedImage {
    pub data: Buffer,
    pub format: String,
    pub image_index: u32,
    pub page_number: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub colorspace: Option<String>,
    pub bits_per_component: Option<u32>,
    pub is_mask: bool,
    pub description: Option<String>,
    #[napi(ts_type = "JsExtractionResult | undefined")]
    pub ocr_result: Option<serde_json::Value>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct JsChunkMetadata {
    pub byte_start: u32,
    pub byte_end: u32,
    pub token_count: Option<u32>,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub first_page: Option<u32>,
    pub last_page: Option<u32>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct JsChunk {
    pub content: String,
    #[napi(ts_type = "number[] | undefined")]
    pub embedding: Option<Vec<f64>>,
    pub metadata: JsChunkMetadata,
}

fn usize_to_u32(value: usize, field: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("{} exceeds supported range (must fit in u32)", field),
        )
    })
}

fn resolve_config(config: Option<JsExtractionConfig>) -> Result<ExtractionConfig> {
    match config {
        Some(cfg) => ExtractionConfig::try_from(cfg),
        None => Ok(ExtractionConfig::default()),
    }
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsExtractionResult {
    pub content: String,
    pub mime_type: String,
    #[napi(ts_type = "Metadata")]
    pub metadata: serde_json::Value,
    pub tables: Vec<JsTable>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<JsChunk>>,
    #[serde(skip)]
    pub images: Option<Vec<JsExtractedImage>>,
}

impl TryFrom<RustExtractionResult> for JsExtractionResult {
    type Error = napi::Error;

    fn try_from(val: RustExtractionResult) -> Result<Self> {
        let metadata = serde_json::to_value(&val.metadata)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to serialize metadata: {}", e)))?;

        let images = if let Some(imgs) = val.images {
            let mut js_images = Vec::with_capacity(imgs.len());
            for img in imgs {
                let ocr_result = if let Some(ocr) = img.ocr_result {
                    Some(JsExtractionResult::try_from(*ocr).and_then(|js_res| {
                        serde_json::to_value(js_res).map_err(|e| {
                            Error::new(
                                Status::GenericFailure,
                                format!("Failed to serialize OCR result metadata: {}", e),
                            )
                        })
                    })?)
                } else {
                    None
                };

                js_images.push(JsExtractedImage {
                    data: img.data.into(),
                    format: img.format,
                    image_index: img.image_index as u32,
                    page_number: img.page_number.map(|p| p as u32),
                    width: img.width,
                    height: img.height,
                    colorspace: img.colorspace,
                    bits_per_component: img.bits_per_component,
                    is_mask: img.is_mask,
                    description: img.description,
                    ocr_result,
                });
            }
            Some(js_images)
        } else {
            None
        };

        Ok(JsExtractionResult {
            content: val.content,
            mime_type: val.mime_type,
            metadata,
            tables: val
                .tables
                .into_iter()
                .map(|t| JsTable {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number as u32,
                })
                .collect(),
            detected_languages: val.detected_languages,
            chunks: if let Some(chunks) = val.chunks {
                let mut js_chunks = Vec::with_capacity(chunks.len());
                for chunk in chunks {
                    let metadata = JsChunkMetadata {
                        byte_start: usize_to_u32(chunk.metadata.byte_start, "chunks[].metadata.byte_start")?,
                        byte_end: usize_to_u32(chunk.metadata.byte_end, "chunks[].metadata.byte_end")?,
                        token_count: match chunk.metadata.token_count {
                            Some(tokens) => Some(usize_to_u32(tokens, "chunks[].metadata.token_count")?),
                            None => None,
                        },
                        chunk_index: usize_to_u32(chunk.metadata.chunk_index, "chunks[].metadata.chunk_index")?,
                        total_chunks: usize_to_u32(chunk.metadata.total_chunks, "chunks[].metadata.total_chunks")?,
                        first_page: chunk.metadata.first_page.map(|p| p as u32),
                        last_page: chunk.metadata.last_page.map(|p| p as u32),
                    };

                    let embedding = chunk
                        .embedding
                        .map(|values| values.into_iter().map(f64::from).collect());

                    js_chunks.push(JsChunk {
                        content: chunk.content,
                        embedding,
                        metadata,
                    });
                }
                Some(js_chunks)
            } else {
                None
            },
            images,
        })
    }
}

impl TryFrom<JsExtractionResult> for RustExtractionResult {
    type Error = napi::Error;

    fn try_from(val: JsExtractionResult) -> Result<Self> {
        let metadata = {
            let mut metadata_map: std::collections::HashMap<String, serde_json::Value> =
                serde_json::from_value(val.metadata.clone()).map_err(|e| {
                    Error::new(
                        Status::GenericFailure,
                        format!("Failed to parse metadata as map: {}", e),
                    )
                })?;

            let language = metadata_map
                .remove("language")
                .and_then(|v| serde_json::from_value(v).ok());
            let subject = metadata_map
                .remove("subject")
                .and_then(|v| serde_json::from_value(v).ok());
            let image_preprocessing = metadata_map
                .remove("image_preprocessing")
                .and_then(|v| serde_json::from_value(v).ok());
            let json_schema = metadata_map.remove("json_schema");
            let error = metadata_map
                .remove("error")
                .and_then(|v| serde_json::from_value(v).ok());

            let mut format_fields = serde_json::Map::new();
            for key in KNOWN_FORMAT_FIELDS.iter() {
                if let Some(value) = metadata_map.remove(*key) {
                    format_fields.insert(key.to_string(), value);
                }
            }

            let format = if !format_fields.is_empty() {
                serde_json::from_value(serde_json::Value::Object(format_fields)).ok()
            } else {
                None
            };

            let additional = metadata_map;

            kreuzberg::Metadata {
                language,
                subject,
                format,
                image_preprocessing,
                json_schema,
                error,
                additional,
                ..Default::default()
            }
        };

        let images = if let Some(imgs) = val.images {
            let mut rust_images = Vec::with_capacity(imgs.len());
            for img in imgs {
                let ocr_result = if let Some(json) = img.ocr_result {
                    Some(Box::new(
                        serde_json::from_value::<JsExtractionResult>(json)
                            .map_err(|e| {
                                Error::new(
                                    Status::GenericFailure,
                                    format!("Failed to deserialize OCR result: {}", e),
                                )
                            })
                            .and_then(RustExtractionResult::try_from)?,
                    ))
                } else {
                    None
                };

                rust_images.push(kreuzberg::ExtractedImage {
                    data: img.data.to_vec(),
                    format: img.format,
                    image_index: img.image_index as usize,
                    page_number: img.page_number.map(|p| p as usize),
                    width: img.width,
                    height: img.height,
                    colorspace: img.colorspace,
                    bits_per_component: img.bits_per_component,
                    is_mask: img.is_mask,
                    description: img.description,
                    ocr_result,
                });
            }
            Some(rust_images)
        } else {
            None
        };

        let chunks = if let Some(chunks) = val.chunks {
            let mut rust_chunks = Vec::with_capacity(chunks.len());
            for chunk in chunks {
                let embedding = if let Some(values) = chunk.embedding {
                    let mut normalized = Vec::with_capacity(values.len());
                    for (idx, value) in values.into_iter().enumerate() {
                        if !value.is_finite() {
                            return Err(Error::new(
                                Status::InvalidArg,
                                format!("chunks[].embedding[{}] must be a finite number", idx),
                            ));
                        }
                        if value > f32::MAX as f64 || value < -(f32::MAX as f64) {
                            return Err(Error::new(
                                Status::InvalidArg,
                                format!("chunks[].embedding[{}] value {} exceeds f32 range", idx, value),
                            ));
                        }
                        normalized.push(value as f32);
                    }
                    Some(normalized)
                } else {
                    None
                };

                rust_chunks.push(RustChunk {
                    content: chunk.content,
                    embedding,
                    metadata: RustChunkMetadata {
                        byte_start: chunk.metadata.byte_start as usize,
                        byte_end: chunk.metadata.byte_end as usize,
                        token_count: chunk.metadata.token_count.map(|v| v as usize),
                        chunk_index: chunk.metadata.chunk_index as usize,
                        total_chunks: chunk.metadata.total_chunks as usize,
                        first_page: chunk.metadata.first_page.map(|v| v as usize),
                        last_page: chunk.metadata.last_page.map(|v| v as usize),
                    },
                });
            }
            Some(rust_chunks)
        } else {
            None
        };

        Ok(RustExtractionResult {
            content: val.content,
            mime_type: val.mime_type,
            metadata,
            tables: val
                .tables
                .into_iter()
                .map(|t| kreuzberg::Table {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number as usize,
                })
                .collect(),
            detected_languages: val.detected_languages,
            chunks,
            images,
            pages: None,
        })
    }
}

/// Extract content from a file (synchronous).
///
/// Synchronously extracts text, tables, images, and metadata from a document file.
/// Supports 118+ file formats including PDFs, Office documents, images, and more.
///
/// # Parameters
///
/// * `file_path` - Path to the file to extract (absolute or relative)
/// * `mime_type` - Optional MIME type hint (auto-detected if omitted)
/// * `config` - Optional extraction configuration (OCR, chunking, etc.)
///
/// # Returns
///
/// `ExtractionResult` containing:
/// - `content`: Extracted text content
/// - `mimeType`: Detected MIME type
/// - `metadata`: File metadata (author, title, etc.)
/// - `tables`: Extracted tables (if any)
/// - `images`: Extracted images (if configured)
/// - `chunks`: Text chunks (if chunking enabled)
/// - `detectedLanguages`: Detected languages (if enabled)
///
/// # Errors
///
/// Throws an error if:
/// - File does not exist or is not accessible
/// - File format is unsupported
/// - File is corrupted or malformed
/// - OCR processing fails (if enabled)
///
/// # Example
///
/// ```typescript
/// import { extractFileSync, ExtractionConfig } from '@kreuzberg/node';
///
/// // Basic extraction
/// const result = extractFileSync('document.pdf', null, null);
/// console.log(result.content);
///
/// // With MIME type hint
/// const result2 = extractFileSync('file.bin', 'application/pdf', null);
///
/// // With OCR enabled
/// const config: ExtractionConfig = {
///   ocr: {
///     backend: 'tesseract',
///     language: 'eng',
///   }
/// };
/// const result3 = extractFileSync('scanned.pdf', null, config);
/// ```
#[napi]
pub fn extract_file_sync(
    file_path: String,
    mime_type: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;

    kreuzberg::extract_file_sync(&file_path, mime_type.as_deref(), &rust_config)
        .map_err(convert_error)
        .and_then(JsExtractionResult::try_from)
}

/// Extract content from a file (asynchronous).
///
/// Asynchronously extracts text, tables, images, and metadata from a document file.
/// Non-blocking alternative to `extractFileSync` for use in async/await contexts.
///
/// # Parameters
///
/// * `file_path` - Path to the file to extract (absolute or relative)
/// * `mime_type` - Optional MIME type hint (auto-detected if omitted)
/// * `config` - Optional extraction configuration (OCR, chunking, etc.)
///
/// # Returns
///
/// Promise resolving to `ExtractionResult` with extracted content and metadata.
///
/// # Errors
///
/// Rejects if file processing fails (see `extractFileSync` for error conditions).
///
/// # Example
///
/// ```typescript
/// import { extractFile } from '@kreuzberg/node';
///
/// // Async/await usage
/// const result = await extractFile('document.pdf', null, null);
/// console.log(result.content);
///
/// // Promise usage
/// extractFile('report.docx', null, null)
///   .then(result => console.log(result.content))
///   .catch(err => console.error(err));
/// ```
#[napi]
pub async fn extract_file(
    file_path: String,
    mime_type: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;

    let result = WORKER_POOL
        .spawn_blocking(move || kreuzberg::extract_file_sync(&file_path, mime_type.as_deref(), &rust_config))
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    JsExtractionResult::try_from(result)
}

/// Extract content from bytes (synchronous).
///
/// Synchronously extracts content from a byte buffer without requiring a file path.
/// Useful for processing in-memory data, network streams, or database BLOBs.
///
/// # Parameters
///
/// * `data` - Buffer containing the document bytes
/// * `mime_type` - MIME type of the data (e.g., "application/pdf", "image/png")
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// `ExtractionResult` with extracted content and metadata.
///
/// # Errors
///
/// Throws an error if data is malformed or MIME type is unsupported.
///
/// # Example
///
/// ```typescript
/// import { extractBytesSync } from '@kreuzberg/node';
/// import fs from 'fs';
///
/// const buffer = fs.readFileSync('document.pdf');
/// const result = extractBytesSync(buffer, 'application/pdf', null);
/// console.log(result.content);
/// ```
#[napi]
pub fn extract_bytes_sync(
    data: Buffer,
    mime_type: String,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;

    let bytes = data.as_ref();

    kreuzberg::extract_bytes_sync(bytes, &mime_type, &rust_config)
        .map_err(convert_error)
        .and_then(JsExtractionResult::try_from)
}

/// Extract content from bytes (asynchronous).
///
/// Asynchronously extracts content from a byte buffer. Non-blocking alternative
/// to `extractBytesSync` for processing in-memory data.
///
/// # Parameters
///
/// * `data` - Buffer containing the document bytes
/// * `mime_type` - MIME type of the data
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// Promise resolving to `ExtractionResult`.
///
/// # Example
///
/// ```typescript
/// import { extractBytes } from '@kreuzberg/node';
///
/// const response = await fetch('https://example.com/document.pdf');
/// const buffer = Buffer.from(await response.arrayBuffer());
/// const result = await extractBytes(buffer, 'application/pdf', null);
/// ```
#[napi]
pub async fn extract_bytes(
    data: Buffer,
    mime_type: String,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;
    let data_vec = data.to_vec();

    let result = WORKER_POOL
        .spawn_blocking(move || kreuzberg::extract_bytes_sync(&data_vec, &mime_type, &rust_config))
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    JsExtractionResult::try_from(result)
}

/// Batch extract from multiple files (synchronous).
///
/// Synchronously processes multiple files in parallel using Rayon. Significantly
/// faster than sequential processing for large batches.
///
/// # Parameters
///
/// * `paths` - Array of file paths to extract
/// * `config` - Optional extraction configuration (applied to all files)
///
/// # Returns
///
/// Array of `ExtractionResult` in the same order as input paths.
///
/// # Example
///
/// ```typescript
/// import { batchExtractFilesSync } from '@kreuzberg/node';
///
/// const files = ['doc1.pdf', 'doc2.docx', 'doc3.txt'];
/// const results = batchExtractFilesSync(files, null);
/// results.forEach((result, i) => {
///   console.log(`File ${files[i]}: ${result.content.substring(0, 100)}...`);
/// });
/// ```
#[napi]
pub fn batch_extract_files_sync(
    paths: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let rust_config = resolve_config(config)?;

    kreuzberg::batch_extract_file_sync(paths, &rust_config)
        .map_err(convert_error)
        .and_then(|results| results.into_iter().map(JsExtractionResult::try_from).collect())
}

/// Batch extract from multiple files (asynchronous).
///
/// Asynchronously processes multiple files in parallel. Non-blocking alternative
/// to `batchExtractFilesSync` with same performance benefits.
///
/// # Parameters
///
/// * `paths` - Array of file paths to extract
/// * `config` - Optional extraction configuration (applied to all files)
///
/// # Returns
///
/// Promise resolving to array of `ExtractionResult`.
///
/// # Example
///
/// ```typescript
/// import { batchExtractFiles } from '@kreuzberg/node';
///
/// const files = ['report1.pdf', 'report2.pdf', 'report3.pdf'];
/// const results = await batchExtractFiles(files, null);
/// console.log(`Processed ${results.length} files`);
/// ```
#[napi]
pub async fn batch_extract_files(
    paths: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let rust_config = resolve_config(config)?;

    let results = WORKER_POOL
        .spawn_blocking(move || kreuzberg::batch_extract_file_sync(paths, &rust_config))
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    results.into_iter().map(JsExtractionResult::try_from).collect()
}

/// Batch extract from multiple byte arrays (synchronous).
///
/// Synchronously processes multiple in-memory buffers in parallel. Requires
/// corresponding MIME types for each buffer.
///
/// # Parameters
///
/// * `data_list` - Array of buffers to extract
/// * `mime_types` - Array of MIME types (must match data_list length)
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// Array of `ExtractionResult` in the same order as inputs.
///
/// # Errors
///
/// Throws if data_list and mime_types lengths don't match.
///
/// # Example
///
/// ```typescript
/// import { batchExtractBytesSync } from '@kreuzberg/node';
///
/// const buffers = [buffer1, buffer2, buffer3];
/// const mimeTypes = ['application/pdf', 'image/png', 'text/plain'];
/// const results = batchExtractBytesSync(buffers, mimeTypes, null);
/// ```
#[napi]
pub fn batch_extract_bytes_sync(
    data_list: Vec<Buffer>,
    mime_types: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    if data_list.len() != mime_types.len() {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "data_list length ({}) must match mime_types length ({})",
                data_list.len(),
                mime_types.len()
            ),
        ));
    }

    let rust_config = resolve_config(config)?;

    let contents: Vec<(&[u8], &str)> = data_list
        .iter()
        .zip(mime_types.iter())
        .map(|(data, mime)| (data.as_ref(), mime.as_str()))
        .collect();

    kreuzberg::batch_extract_bytes_sync(contents, &rust_config)
        .map_err(convert_error)
        .and_then(|results| results.into_iter().map(JsExtractionResult::try_from).collect())
}

/// Batch extract from multiple byte arrays (asynchronous).
///
/// Asynchronously processes multiple in-memory buffers in parallel. Non-blocking
/// alternative to `batchExtractBytesSync`.
///
/// # Parameters
///
/// * `data_list` - Array of buffers to extract
/// * `mime_types` - Array of MIME types (must match data_list length)
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// Promise resolving to array of `ExtractionResult`.
///
/// # Example
///
/// ```typescript
/// import { batchExtractBytes } from '@kreuzberg/node';
///
/// const responses = await Promise.all([
///   fetch('https://example.com/doc1.pdf'),
///   fetch('https://example.com/doc2.pdf')
/// ]);
/// const buffers = await Promise.all(
///   responses.map(r => r.arrayBuffer().then(b => Buffer.from(b)))
/// );
/// const results = await batchExtractBytes(
///   buffers,
///   ['application/pdf', 'application/pdf'],
///   null
/// );
/// ```
#[napi]
pub async fn batch_extract_bytes(
    data_list: Vec<Buffer>,
    mime_types: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    if data_list.len() != mime_types.len() {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "data_list length ({}) must match mime_types length ({})",
                data_list.len(),
                mime_types.len()
            ),
        ));
    }

    let rust_config = resolve_config(config)?;

    let contents: Vec<(Vec<u8>, String)> = data_list
        .iter()
        .zip(mime_types.iter())
        .map(|(data, mime)| (data.to_vec(), mime.clone()))
        .collect();

    let results = WORKER_POOL
        .spawn_blocking(move || {
            let contents_refs: Vec<(&[u8], &str)> = contents
                .iter()
                .map(|(data, mime)| (data.as_slice(), mime.as_str()))
                .collect();
            kreuzberg::batch_extract_bytes_sync(contents_refs, &rust_config)
        })
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    results.into_iter().map(JsExtractionResult::try_from).collect()
}

use async_trait::async_trait;
use kreuzberg::plugins::{Plugin, PostProcessor as RustPostProcessor, ProcessingStage};
use napi::bindgen_prelude::Promise;
use napi::threadsafe_function::ThreadsafeFunction;
use std::sync::Arc;

/// Wrapper that makes a JavaScript PostProcessor usable from Rust.
///
/// Uses JSON serialization to pass data between Rust and JavaScript due to NAPI limitations
/// with complex object types across ThreadsafeFunction boundaries.
///
/// Wrapper that holds the ThreadsafeFunction to call JavaScript from Rust.
/// The process_fn is an async JavaScript function that:
/// - Takes: String (JSON-serialized ExtractionResult)
/// - Returns: Promise<String> (JSON-serialized ExtractionResult)
///
/// Type parameters:
/// - Input: String
/// - Return: Promise<String>
/// - CallJsBackArgs: Vec<String> (because build_callback returns vec![value])
/// - ErrorStatus: napi::Status
/// - CalleeHandled: false (default with build_callback)
struct JsPostProcessor {
    name: String,
    process_fn: Arc<ThreadsafeFunction<String, Promise<String>, Vec<String>, napi::Status, false>>,
    stage: ProcessingStage,
}

unsafe impl Send for JsPostProcessor {}
unsafe impl Sync for JsPostProcessor {}

impl Plugin for JsPostProcessor {
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
impl RustPostProcessor for JsPostProcessor {
    async fn process(
        &self,
        result: &mut kreuzberg::ExtractionResult,
        _config: &kreuzberg::ExtractionConfig,
    ) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        let js_result =
            JsExtractionResult::try_from(result.clone()).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("Failed to convert result for JavaScript PostProcessor: {}", e),
                plugin_name: self.name.clone(),
            })?;

        let json_string = serde_json::to_string(&js_result).map_err(|e| kreuzberg::KreuzbergError::Plugin {
            message: format!("Failed to serialize result to JSON for JavaScript PostProcessor: {}", e),
            plugin_name: self.name.clone(),
        })?;

        let output_json = self
            .process_fn
            .call_async(json_string)
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("JavaScript PostProcessor '{}' call failed: {}", self.name, e),
                plugin_name: self.name.clone(),
            })?
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("JavaScript PostProcessor '{}' promise failed: {}", self.name, e),
                plugin_name: self.name.clone(),
            })?;

        let updated: JsExtractionResult =
            serde_json::from_str(&output_json).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!(
                    "Failed to deserialize JSON result from JavaScript PostProcessor '{}': {}",
                    self.name, e
                ),
                plugin_name: self.name.clone(),
            })?;

        let rust_result =
            kreuzberg::ExtractionResult::try_from(updated).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("Failed to convert result from JavaScript PostProcessor: {}", e),
                plugin_name: self.name.clone(),
            })?;

        *result = rust_result;

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }
}

/// Register a custom postprocessor
///
/// Registers a JavaScript PostProcessor that will be called after extraction.
///
/// # Arguments
///
/// * `processor` - JavaScript object with the following interface:
///   - `name(): string` - Unique processor name
///   - `process(...args): string` - Process function that receives JSON string as args\[0\]
///   - `processingStage(): "early" | "middle" | "late"` - Optional processing stage
///
/// # Implementation Notes
///
/// Due to NAPI ThreadsafeFunction limitations, the process function receives the extraction
/// result as a JSON string in args\[0\] and must return a JSON string. Use the TypeScript
/// wrapper functions for a cleaner API.
///
/// # Example
///
/// ```typescript
/// import { registerPostProcessor } from '@kreuzberg/node';
///
/// registerPostProcessor({
///   name: () => "word-counter",
///   processingStage: () => "middle",
///   process: (...args) => {
///     const result = JSON.parse(args[0]);
///     const wordCount = result.content.split(/\s+/).length;
///     result.metadata.word_count = wordCount;
///     return JSON.stringify(result);
///   }
/// });
/// ```
#[napi]
pub fn register_post_processor(_env: Env, processor: Object) -> Result<()> {
    validate_plugin_object(&processor, "PostProcessor", &["name", "process"])?;

    let name: String = processor.get_named_property::<String>("name").or_else(|_| {
        let name_fn: Function<(), String> = processor.get_named_property("name")?;
        name_fn.call(())
    })?;

    if name.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "Processor name cannot be empty".to_string(),
        ));
    }

    let stage = if let Ok(stage_str) = processor.get_named_property::<String>("processingStage") {
        match stage_str.to_lowercase().as_str() {
            "early" => ProcessingStage::Early,
            "middle" => ProcessingStage::Middle,
            "late" => ProcessingStage::Late,
            _ => ProcessingStage::Middle,
        }
    } else if let Ok(stage_fn) = processor.get_named_property::<Function<(), String>>("processingStage") {
        let stage_str: String = stage_fn.call(())?;
        match stage_str.to_lowercase().as_str() {
            "early" => ProcessingStage::Early,
            "middle" => ProcessingStage::Middle,
            "late" => ProcessingStage::Late,
            _ => ProcessingStage::Middle,
        }
    } else {
        ProcessingStage::Middle
    };

    let process_fn: Function<String, Promise<String>> = processor.get_named_property("process")?;

    let tsfn = process_fn
        .build_threadsafe_function()
        .build_callback(|ctx| Ok(vec![ctx.value]))?;

    let js_processor = JsPostProcessor {
        name: name.clone(),
        process_fn: Arc::new(tsfn),
        stage,
    };

    let arc_processor: Arc<dyn RustPostProcessor> = Arc::new(js_processor);
    let registry = get_post_processor_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on PostProcessor registry: {}", e),
        )
    })?;

    registry.register(arc_processor, 0).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to register PostProcessor '{}': {}", name, e),
        )
    })?;

    Ok(())
}

/// Unregister a postprocessor by name
#[napi]
pub fn unregister_post_processor(name: String) -> Result<()> {
    let registry = get_post_processor_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on PostProcessor registry: {}", e),
        )
    })?;

    registry.remove(&name).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to unregister PostProcessor '{}': {}", name, e),
        )
    })?;
    Ok(())
}

/// Clear all registered postprocessors
#[napi]
pub fn clear_post_processors() -> Result<()> {
    let registry = get_post_processor_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on PostProcessor registry: {}", e),
        )
    })?;

    *registry = Default::default();
    Ok(())
}

/// List all registered post-processors
#[napi]
pub fn list_post_processors() -> Result<Vec<String>> {
    let registry = get_post_processor_registry();
    let registry = registry.read().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire read lock on PostProcessor registry: {}", e),
        )
    })?;

    Ok(registry.list())
}

use kreuzberg::plugins::Validator as RustValidator;

/// Wrapper that makes a JavaScript Validator usable from Rust.
///
/// Uses JSON serialization to pass data between Rust and JavaScript due to NAPI limitations
/// with complex object types across ThreadsafeFunction boundaries.
///
/// Wrapper that holds the ThreadsafeFunction to call JavaScript from Rust.
/// The validate_fn is an async JavaScript function that:
/// - Takes: String (JSON-serialized ExtractionResult)
/// - Returns: Promise<String> (empty string on success, rejects on validation failure)
///
/// Type parameters:
/// - Input: String
/// - Return: Promise<String>
/// - CallJsBackArgs: Vec<String> (because build_callback returns vec![value])
/// - ErrorStatus: napi::Status
/// - CalleeHandled: false (default with build_callback)
struct JsValidator {
    name: String,
    validate_fn: Arc<ThreadsafeFunction<String, Promise<String>, Vec<String>, napi::Status, false>>,
    priority: i32,
}

unsafe impl Send for JsValidator {}
unsafe impl Sync for JsValidator {}

impl Plugin for JsValidator {
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
impl RustValidator for JsValidator {
    async fn validate(
        &self,
        result: &kreuzberg::ExtractionResult,
        _config: &kreuzberg::ExtractionConfig,
    ) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        let js_result =
            JsExtractionResult::try_from(result.clone()).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("Failed to convert result for JavaScript Validator: {}", e),
                plugin_name: self.name.clone(),
            })?;

        let json_string = serde_json::to_string(&js_result).map_err(|e| kreuzberg::KreuzbergError::Plugin {
            message: format!("Failed to serialize result to JSON for JavaScript Validator: {}", e),
            plugin_name: self.name.clone(),
        })?;

        self.validate_fn
            .call_async(json_string)
            .await
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("ValidationError") || err_msg.contains("validation") {
                    kreuzberg::KreuzbergError::Validation {
                        message: err_msg,
                        source: None,
                    }
                } else {
                    kreuzberg::KreuzbergError::Plugin {
                        message: format!("JavaScript Validator '{}' call failed: {}", self.name, err_msg),
                        plugin_name: self.name.clone(),
                    }
                }
            })?
            .await
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("ValidationError") || err_msg.contains("validation") {
                    kreuzberg::KreuzbergError::Validation {
                        message: err_msg,
                        source: None,
                    }
                } else {
                    kreuzberg::KreuzbergError::Plugin {
                        message: format!("JavaScript Validator '{}' promise failed: {}", self.name, err_msg),
                        plugin_name: self.name.clone(),
                    }
                }
            })?;

        Ok(())
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// Register a custom validator
///
/// Registers a JavaScript Validator that will be called after extraction.
///
/// # Arguments
///
/// * `validator` - JavaScript object with the following interface:
///   - `name(): string` - Unique validator name
///   - `validate(...args): Promise<string>` - Validate function that receives JSON string as args\[0\]
///   - `priority(): number` - Optional priority (defaults to 50, higher runs first)
///
/// # Implementation Notes
///
/// Due to NAPI ThreadsafeFunction limitations, the validate function receives the extraction
/// result as a JSON string in args\[0\]. On success, return an empty string. On validation
/// failure, throw an error (the Promise should reject). Use the TypeScript wrapper functions
/// for a cleaner API.
///
/// # Example
///
/// ```typescript
/// import { registerValidator } from '@kreuzberg/node';
///
/// registerValidator({
///   name: () => "min-length",
///   priority: () => 100,
///   validate: async (...args) => {
///     const result = JSON.parse(args[0]);
///     if (result.content.length < 100) {
///       throw new Error("ValidationError: Content too short");
///     }
///     return ""; // Success - return empty string
///   }
/// });
/// ```
#[napi]
pub fn register_validator(_env: Env, validator: Object) -> Result<()> {
    validate_plugin_object(&validator, "Validator", &["name", "validate"])?;

    let name: String = validator.get_named_property::<String>("name").or_else(|_| {
        let name_fn: Function<(), String> = validator.get_named_property("name")?;
        name_fn.call(())
    })?;

    if name.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "Validator name cannot be empty".to_string(),
        ));
    }

    let priority = if let Ok(priority_val) = validator.get_named_property::<i32>("priority") {
        priority_val
    } else if let Ok(priority_fn) = validator.get_named_property::<Function<(), i32>>("priority") {
        priority_fn.call(())?
    } else {
        50
    };

    let validate_fn: Function<String, Promise<String>> = validator.get_named_property("validate")?;

    let tsfn = validate_fn
        .build_threadsafe_function()
        .build_callback(|ctx| Ok(vec![ctx.value]))?;

    let js_validator = JsValidator {
        name: name.clone(),
        validate_fn: Arc::new(tsfn),
        priority,
    };

    let arc_validator: Arc<dyn RustValidator> = Arc::new(js_validator);
    let registry = get_validator_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on Validator registry: {}", e),
        )
    })?;

    registry.register(arc_validator).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to register Validator '{}': {}", name, e),
        )
    })?;

    Ok(())
}

/// Unregister a validator by name
#[napi]
pub fn unregister_validator(name: String) -> Result<()> {
    let registry = get_validator_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on Validator registry: {}", e),
        )
    })?;

    registry.remove(&name).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to unregister Validator '{}': {}", name, e),
        )
    })?;
    Ok(())
}

/// Clear all registered validators
#[napi]
pub fn clear_validators() -> Result<()> {
    let registry = get_validator_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on Validator registry: {}", e),
        )
    })?;

    *registry = Default::default();
    Ok(())
}

/// List all registered validators
#[napi]
pub fn list_validators() -> Result<Vec<String>> {
    let registry = get_validator_registry();
    let registry = registry.read().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire read lock on Validator registry: {}", e),
        )
    })?;

    Ok(registry.list())
}

use kreuzberg::plugins::registry::get_ocr_backend_registry;
use kreuzberg::plugins::{OcrBackend as RustOcrBackend, OcrBackendType};

/// Wrapper that makes a JavaScript OCR backend usable from Rust.
///
/// Uses JSON serialization to pass data between Rust and JavaScript due to NAPI limitations
/// with complex object types across ThreadsafeFunction boundaries.
///
/// Wrapper that holds the ThreadsafeFunction to call JavaScript from Rust.
/// The processImage_fn is an async JavaScript function that:
/// - Takes: (String, String) - base64-encoded image bytes and language code
/// - Returns: Promise<String> (JSON-serialized ExtractionResult)
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
            mime_type,
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        })
    }

    async fn process_file(
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
    validate_plugin_object(&backend, "OCR Backend", &["name", "supportedLanguages", "processImage"])?;

    let name: String = backend.get_named_property::<String>("name").or_else(|_| {
        let name_fn: Function<(), String> = backend.get_named_property("name")?;
        name_fn.call(())
    })?;

    if name.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
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
        return Err(Error::new(
            Status::InvalidArg,
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
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on OCR backend registry: {}", e),
        )
    })?;

    registry.register(arc_backend).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to register OCR backend '{}': {}", name, e),
        )
    })?;

    Ok(())
}

/// Unregister an OCR backend by name.
///
/// Removes the specified OCR backend from the registry. If the backend doesn't exist,
/// this operation is a no-op (does not throw an error).
///
/// # Parameters
///
/// * `name` - Name of the OCR backend to unregister
///
/// # Example
///
/// ```typescript
/// import { unregisterOcrBackend } from 'kreuzberg';
///
/// // Unregister a custom backend
/// unregisterOcrBackend('my-custom-ocr');
/// ```
#[napi]
pub fn unregister_ocr_backend(name: String) -> Result<()> {
    kreuzberg::plugins::unregister_ocr_backend(&name).map_err(convert_error)
}

/// List all registered OCR backends.
///
/// Returns an array of names of all currently registered OCR backends,
/// including built-in backends like "tesseract".
///
/// # Returns
///
/// Array of OCR backend names.
///
/// # Example
///
/// ```typescript
/// import { listOcrBackends } from 'kreuzberg';
///
/// const backends = listOcrBackends();
/// console.log(backends); // ['tesseract', 'my-custom-backend', ...]
/// ```
#[napi]
pub fn list_ocr_backends() -> Result<Vec<String>> {
    kreuzberg::plugins::list_ocr_backends().map_err(convert_error)
}

/// Clear all registered OCR backends.
///
/// Removes all OCR backends from the registry, including built-in backends.
/// Use with caution as this will make OCR functionality unavailable until
/// backends are re-registered.
///
/// # Example
///
/// ```typescript
/// import { clearOcrBackends } from 'kreuzberg';
///
/// clearOcrBackends();
/// ```
#[napi]
pub fn clear_ocr_backends() -> Result<()> {
    kreuzberg::plugins::clear_ocr_backends().map_err(convert_error)
}

/// List all registered document extractors.
///
/// Returns an array of names of all currently registered document extractors,
/// including built-in extractors for PDF, Office documents, images, etc.
///
/// # Returns
///
/// Array of document extractor names.
///
/// # Example
///
/// ```typescript
/// import { listDocumentExtractors } from 'kreuzberg';
///
/// const extractors = listDocumentExtractors();
/// console.log(extractors); // ['PDFExtractor', 'ImageExtractor', ...]
/// ```
#[napi]
pub fn list_document_extractors() -> Result<Vec<String>> {
    kreuzberg::plugins::list_extractors().map_err(convert_error)
}

/// Unregister a document extractor by name.
///
/// Removes the specified document extractor from the registry. If the extractor
/// doesn't exist, this operation is a no-op (does not throw an error).
///
/// # Parameters
///
/// * `name` - Name of the document extractor to unregister
///
/// # Example
///
/// ```typescript
/// import { unregisterDocumentExtractor } from 'kreuzberg';
///
/// // Unregister a custom extractor
/// unregisterDocumentExtractor('MyCustomExtractor');
/// ```
#[napi]
pub fn unregister_document_extractor(name: String) -> Result<()> {
    kreuzberg::plugins::unregister_extractor(&name).map_err(convert_error)
}

/// Clear all registered document extractors.
///
/// Removes all document extractors from the registry, including built-in extractors.
/// Use with caution as this will make document extraction unavailable until
/// extractors are re-registered.
///
/// # Example
///
/// ```typescript
/// import { clearDocumentExtractors } from 'kreuzberg';
///
/// clearDocumentExtractors();
/// ```
#[napi]
pub fn clear_document_extractors() -> Result<()> {
    kreuzberg::plugins::clear_extractors().map_err(convert_error)
}

/// Detect MIME type from raw bytes.
///
/// Uses content inspection (magic bytes) to determine MIME type.
/// This is more accurate than extension-based detection but requires
/// reading the file content.
///
/// # Parameters
///
/// * `bytes` - Raw file content as Buffer
///
/// # Returns
///
/// The detected MIME type string.
///
/// # Errors
///
/// Throws an error if MIME type cannot be determined from content.
///
/// # Example
///
/// ```typescript
/// import { detectMimeType } from 'kreuzberg';
/// import * as fs from 'fs';
///
/// // Read file content
/// const content = fs.readFileSync('document.pdf');
///
/// // Detect MIME type from bytes
/// const mimeType = detectMimeType(content);
/// console.log(mimeType); // 'application/pdf'
/// ```
#[napi]
pub fn detect_mime_type_from_bytes(bytes: Buffer) -> Result<String> {
    kreuzberg::core::mime::detect_mime_type_from_bytes(bytes.as_ref()).map_err(convert_error)
}

/// Detect MIME type from a file path.
///
/// Uses file extension to determine MIME type. Falls back to `mime_guess` crate
/// if extension-based detection fails.
///
/// # Parameters
///
/// * `path` - Path to the file (string)
/// * `check_exists` - Whether to verify file existence (default: true)
///
/// # Returns
///
/// The detected MIME type string.
///
/// # Errors
///
/// Throws an error if:
/// - File doesn't exist (when check_exists is true)
/// - MIME type cannot be determined from path/extension
/// - Extension is unknown
///
/// # Example
///
/// ```typescript
/// import { detectMimeTypeFromPath } from 'kreuzberg';
///
/// // Detect from existing file
/// const mimeType = detectMimeTypeFromPath('document.pdf');
/// console.log(mimeType); // 'application/pdf'
///
/// const mimeType2 = detectMimeTypeFromPath('document.docx');
/// console.log(mimeType2); // 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
/// ```
#[napi]
pub fn detect_mime_type_from_path(path: String, check_exists: Option<bool>) -> Result<String> {
    kreuzberg::core::mime::detect_mime_type(&path, check_exists.unwrap_or(true)).map_err(convert_error)
}

/// Validate that a MIME type is supported by Kreuzberg.
///
/// Checks if a MIME type is in the list of supported formats. Note that any
/// `image/*` MIME type is automatically considered valid.
///
/// # Parameters
///
/// * `mime_type` - The MIME type to validate (string)
///
/// # Returns
///
/// The validated MIME type (may be normalized).
///
/// # Errors
///
/// Throws an error if the MIME type is not supported.
///
/// # Example
///
/// ```typescript
/// import { validateMimeType } from 'kreuzberg';
///
/// // Validate supported type
/// const validated = validateMimeType('application/pdf');
/// console.log(validated); // 'application/pdf'
///
/// // Validate custom image type
/// const validated2 = validateMimeType('image/custom-format');
/// console.log(validated2); // 'image/custom-format' (any image/* is valid)
///
/// // Validate unsupported type (throws error)
/// try {
///   validateMimeType('video/mp4');
/// } catch (err) {
///   console.error(err); // Error: Unsupported format: video/mp4
/// }
/// ```
#[napi]
pub fn validate_mime_type(mime_type: String) -> Result<String> {
    kreuzberg::core::mime::validate_mime_type(&mime_type).map_err(convert_error)
}

/// Get file extensions for a given MIME type.
///
/// Returns an array of file extensions commonly associated with the specified
/// MIME type. For example, 'application/pdf' returns ['pdf'].
///
/// # Parameters
///
/// * `mime_type` - The MIME type to look up (e.g., 'application/pdf', 'image/jpeg')
///
/// # Returns
///
/// Array of file extensions (without leading dots).
///
/// # Errors
///
/// Throws an error if the MIME type is not recognized or supported.
///
/// # Example
///
/// ```typescript
/// import { getExtensionsForMime } from 'kreuzberg';
///
/// // Get extensions for PDF
/// const pdfExts = getExtensionsForMime('application/pdf');
/// console.log(pdfExts); // ['pdf']
///
/// // Get extensions for JPEG
/// const jpegExts = getExtensionsForMime('image/jpeg');
/// console.log(jpegExts); // ['jpg', 'jpeg']
/// ```
#[napi]
pub fn get_extensions_for_mime(mime_type: String) -> Result<Vec<String>> {
    kreuzberg::core::mime::get_extensions_for_mime(&mime_type).map_err(convert_error)
}

/// Embedding preset configuration for TypeScript bindings.
///
/// Contains all settings for a specific embedding model preset.
#[napi(object)]
pub struct EmbeddingPreset {
    /// Name of the preset (e.g., "fast", "balanced", "quality", "multilingual")
    pub name: String,
    /// Recommended chunk size in characters
    pub chunk_size: u32,
    /// Recommended overlap in characters
    pub overlap: u32,
    /// Model identifier (e.g., "AllMiniLML6V2Q", "BGEBaseENV15")
    pub model_name: String,
    /// Embedding vector dimensions
    pub dimensions: u32,
    /// Human-readable description of the preset
    pub description: String,
}

/// List all available embedding preset names.
///
/// Returns an array of preset names that can be used with `getEmbeddingPreset`.
///
/// # Returns
///
/// Array of 4 preset names: ["fast", "balanced", "quality", "multilingual"]
///
/// # Example
///
/// ```typescript
/// import { listEmbeddingPresets } from 'kreuzberg';
///
/// const presets = listEmbeddingPresets();
/// console.log(presets); // ['fast', 'balanced', 'quality', 'multilingual']
/// ```
#[napi(js_name = "listEmbeddingPresets")]
pub fn list_embedding_presets() -> Vec<String> {
    kreuzberg::embeddings::list_presets()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

/// Get a specific embedding preset by name.
///
/// Returns a preset configuration object, or null if the preset name is not found.
///
/// # Arguments
///
/// * `name` - The preset name (case-sensitive)
///
/// # Returns
///
/// An `EmbeddingPreset` object with the following properties:
/// - `name`: string - Preset name
/// - `chunkSize`: number - Recommended chunk size in characters
/// - `overlap`: number - Recommended overlap in characters
/// - `modelName`: string - Model identifier
/// - `dimensions`: number - Embedding vector dimensions
/// - `description`: string - Human-readable description
///
/// Returns `null` if preset name is not found.
///
/// # Example
///
/// ```typescript
/// import { getEmbeddingPreset } from 'kreuzberg';
///
/// const preset = getEmbeddingPreset('balanced');
/// if (preset) {
///   console.log(`Model: ${preset.modelName}, Dims: ${preset.dimensions}`);
///   // Model: BGEBaseENV15, Dims: 768
/// }
/// ```
#[napi(js_name = "getEmbeddingPreset")]
pub fn get_embedding_preset(name: String) -> Option<EmbeddingPreset> {
    let preset = kreuzberg::embeddings::get_preset(&name)?;

    let model_name = format!("{:?}", preset.model);

    Some(EmbeddingPreset {
        name: preset.name.to_string(),
        chunk_size: preset.chunk_size as u32,
        overlap: preset.overlap as u32,
        model_name,
        dimensions: preset.dimensions as u32,
        description: preset.description.to_string(),
    })
}

/// Get the error code for the last FFI error.
///
/// Returns the FFI error code as an integer. Error codes are:
/// - 0: Success (no error)
/// - 1: GenericError
/// - 2: Panic
/// - 3: InvalidArgument
/// - 4: IoError
/// - 5: ParsingError
/// - 6: OcrError
/// - 7: MissingDependency
///
/// This is useful for programmatic error handling and distinguishing
/// between different types of failures in native code.
///
/// # Returns
///
/// The integer error code.
///
/// # Example
///
/// ```typescript
/// import { extractFile, getLastErrorCode, ErrorCode } from '@kreuzberg/node';
///
/// try {
///   const result = await extractFile('document.pdf');
/// } catch (error) {
///   const code = getLastErrorCode();
///   if (code === ErrorCode.Panic) {
///     console.error('Native code panic detected');
///   }
/// }
/// ```
#[napi(js_name = "getLastErrorCode")]
pub fn get_last_error_code() -> i32 {
    unsafe { kreuzberg_last_error_code() }
}

/// Get panic context information if the last error was a panic.
///
/// Returns detailed information about a panic in native code, or null
/// if the last error was not a panic.
///
/// # Returns
///
/// A `PanicContext` object with:
/// - `file`: string - Source file where panic occurred
/// - `line`: number - Line number
/// - `function`: string - Function name
/// - `message`: string - Panic message
/// - `timestamp_secs`: number - Unix timestamp (seconds since epoch)
///
/// Returns `null` if no panic context is available.
///
/// # Example
///
/// ```typescript
/// import { extractFile, getLastPanicContext } from '@kreuzberg/node';
///
/// try {
///   const result = await extractFile('document.pdf');
/// } catch (error) {
///   const context = getLastPanicContext();
///   if (context) {
///     console.error(`Panic at ${context.file}:${context.line}`);
///     console.error(`In function: ${context.function}`);
///     console.error(`Message: ${context.message}`);
///   }
/// }
/// ```
#[napi(js_name = "getLastPanicContext")]
pub fn get_last_panic_context() -> Option<serde_json::Value> {
    get_panic_context()
}

/// Validates a binarization method string.
///
/// Valid methods: "otsu", "adaptive", "sauvola"
///
/// # Arguments
///
/// * `method` - The binarization method to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateBinarizationMethod } from '@kreuzberg/node';
///
/// if (validateBinarizationMethod('otsu')) {
///   console.log('Valid method');
/// } else {
///   console.log('Invalid method');
/// }
/// ```
#[napi(js_name = "validateBinarizationMethod")]
pub fn validate_binarization_method(method: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(method.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in binarization method: {}", method),
        )
    })?;

    let result = unsafe { kreuzberg_validate_binarization_method(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates an OCR backend string.
///
/// Valid backends: "tesseract", "easyocr", "paddleocr"
///
/// # Arguments
///
/// * `backend` - The OCR backend to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateOcrBackend } from '@kreuzberg/node';
///
/// if (validateOcrBackend('tesseract')) {
///   console.log('Valid backend');
/// }
/// ```
#[napi(js_name = "validateOcrBackend")]
pub fn validate_ocr_backend(backend: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(backend.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in OCR backend: {}", backend),
        )
    })?;

    let result = unsafe { kreuzberg_validate_ocr_backend(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates a language code (ISO 639-1 or 639-3 format).
///
/// Accepts both 2-letter codes (e.g., "en", "de") and 3-letter codes (e.g., "eng", "deu").
///
/// # Arguments
///
/// * `code` - The language code to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateLanguageCode } from '@kreuzberg/node';
///
/// if (validateLanguageCode('en')) {
///   console.log('Valid language code');
/// }
/// ```
#[napi(js_name = "validateLanguageCode")]
pub fn validate_language_code(code: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(code.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in language code: {}", code),
        )
    })?;

    let result = unsafe { kreuzberg_validate_language_code(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates a token reduction level string.
///
/// Valid levels: "off", "light", "moderate", "aggressive", "maximum"
///
/// # Arguments
///
/// * `level` - The token reduction level to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateTokenReductionLevel } from '@kreuzberg/node';
///
/// if (validateTokenReductionLevel('moderate')) {
///   console.log('Valid token reduction level');
/// }
/// ```
#[napi(js_name = "validateTokenReductionLevel")]
pub fn validate_token_reduction_level(level: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(level.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in token reduction level: {}", level),
        )
    })?;

    let result = unsafe { kreuzberg_validate_token_reduction_level(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates a Tesseract Page Segmentation Mode (PSM) value.
///
/// Valid range: 0-13
///
/// # Arguments
///
/// * `psm` - The PSM value to validate
///
/// # Returns
///
/// `true` if valid (0-13), `false` otherwise.
///
/// # Example
///
/// ```typescript
/// import { validateTesseractPsm } from '@kreuzberg/node';
///
/// if (validateTesseractPsm(3)) {
///   console.log('Valid PSM');
/// }
/// ```
#[napi(js_name = "validateTesseractPsm")]
pub fn validate_tesseract_psm(psm: i32) -> bool {
    unsafe { kreuzberg_validate_tesseract_psm(psm) == 1 }
}

/// Validates a Tesseract OCR Engine Mode (OEM) value.
///
/// Valid range: 0-3
///
/// # Arguments
///
/// * `oem` - The OEM value to validate
///
/// # Returns
///
/// `true` if valid (0-3), `false` otherwise.
///
/// # Example
///
/// ```typescript
/// import { validateTesseractOem } from '@kreuzberg/node';
///
/// if (validateTesseractOem(1)) {
///   console.log('Valid OEM');
/// }
/// ```
#[napi(js_name = "validateTesseractOem")]
pub fn validate_tesseract_oem(oem: i32) -> bool {
    unsafe { kreuzberg_validate_tesseract_oem(oem) == 1 }
}

/// Validates a tesseract output format string.
///
/// Valid formats: "text", "markdown"
///
/// # Arguments
///
/// * `format` - The output format to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateOutputFormat } from '@kreuzberg/node';
///
/// if (validateOutputFormat('markdown')) {
///   console.log('Valid output format');
/// }
/// ```
#[napi(js_name = "validateOutputFormat")]
pub fn validate_output_format(format: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(format.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in output format: {}", format),
        )
    })?;

    let result = unsafe { kreuzberg_validate_output_format(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates a confidence threshold value.
///
/// Valid range: 0.0 to 1.0 (inclusive)
///
/// # Arguments
///
/// * `confidence` - The confidence threshold to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateConfidence } from '@kreuzberg/node';
///
/// if (validateConfidence(0.75)) {
///   console.log('Valid confidence threshold');
/// }
/// ```
#[napi(js_name = "validateConfidence")]
pub fn validate_confidence(confidence: f64) -> bool {
    unsafe { kreuzberg_validate_confidence(confidence) == 1 }
}

/// Validates a DPI (dots per inch) value.
///
/// Valid range: 1-2400
///
/// # Arguments
///
/// * `dpi` - The DPI value to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateDpi } from '@kreuzberg/node';
///
/// if (validateDpi(300)) {
///   console.log('Valid DPI');
/// }
/// ```
#[napi(js_name = "validateDpi")]
pub fn validate_dpi(dpi: i32) -> bool {
    unsafe { kreuzberg_validate_dpi(dpi) == 1 }
}

/// Validates chunking parameters.
///
/// Checks that `maxChars > 0` and `maxOverlap < maxChars`.
///
/// # Arguments
///
/// * `max_chars` - Maximum characters per chunk
/// * `max_overlap` - Maximum overlap between chunks
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateChunkingParams } from '@kreuzberg/node';
///
/// if (validateChunkingParams(1000, 200)) {
///   console.log('Valid chunking parameters');
/// }
/// ```
#[napi(js_name = "validateChunkingParams")]
pub fn validate_chunking_params(max_chars: u32, max_overlap: u32) -> bool {
    unsafe { kreuzberg_validate_chunking_params(max_chars as usize, max_overlap as usize) == 1 }
}

/// Get valid binarization methods.
///
/// Returns a list of all valid binarization method values.
///
/// # Returns
///
/// Array of valid binarization methods: ["otsu", "adaptive", "sauvola"]
///
/// # Example
///
/// ```typescript
/// import { getValidBinarizationMethods } from '@kreuzberg/node';
///
/// const methods = getValidBinarizationMethods();
/// console.log(methods); // ['otsu', 'adaptive', 'sauvola']
/// ```
#[napi(js_name = "getValidBinarizationMethods")]
pub fn get_valid_binarization_methods() -> Result<Vec<String>> {
    let json_str = unsafe {
        let ptr = kreuzberg_get_valid_binarization_methods();
        if ptr.is_null() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Failed to get valid binarization methods",
            ));
        }

        let c_str = CStr::from_ptr(ptr);
        let result = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in binarization methods"))?
            .to_string();

        kreuzberg_free_string(ptr as *mut c_char);
        result
    };

    let parsed: Vec<String> = serde_json::from_str(&json_str).map_err(|_| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse binarization methods JSON",
        )
    })?;

    Ok(parsed)
}

/// Get valid language codes.
///
/// Returns a list of all valid language codes in ISO 639-1 and 639-3 formats.
///
/// # Returns
///
/// Array of valid language codes (both 2-letter and 3-letter codes)
///
/// # Example
///
/// ```typescript
/// import { getValidLanguageCodes } from '@kreuzberg/node';
///
/// const codes = getValidLanguageCodes();
/// console.log(codes); // ['en', 'de', 'fr', ..., 'eng', 'deu', 'fra', ...]
/// ```
#[napi(js_name = "getValidLanguageCodes")]
pub fn get_valid_language_codes() -> Result<Vec<String>> {
    let json_str = unsafe {
        let ptr = kreuzberg_get_valid_language_codes();
        if ptr.is_null() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Failed to get valid language codes",
            ));
        }

        let c_str = CStr::from_ptr(ptr);
        let result = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in language codes"))?
            .to_string();

        kreuzberg_free_string(ptr as *mut c_char);
        result
    };

    let parsed: Vec<String> = serde_json::from_str(&json_str)
        .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Failed to parse language codes JSON"))?;

    Ok(parsed)
}

/// Get valid OCR backends.
///
/// Returns a list of all valid OCR backend values.
///
/// # Returns
///
/// Array of valid OCR backends: ["tesseract", "easyocr", "paddleocr"]
///
/// # Example
///
/// ```typescript
/// import { getValidOcrBackends } from '@kreuzberg/node';
///
/// const backends = getValidOcrBackends();
/// console.log(backends); // ['tesseract', 'easyocr', 'paddleocr']
/// ```
#[napi(js_name = "getValidOcrBackends")]
pub fn get_valid_ocr_backends() -> Result<Vec<String>> {
    let json_str = unsafe {
        let ptr = kreuzberg_get_valid_ocr_backends();
        if ptr.is_null() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Failed to get valid OCR backends",
            ));
        }

        let c_str = CStr::from_ptr(ptr);
        let result = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in OCR backends"))?
            .to_string();

        kreuzberg_free_string(ptr as *mut c_char);
        result
    };

    let parsed: Vec<String> = serde_json::from_str(&json_str)
        .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Failed to parse OCR backends JSON"))?;

    Ok(parsed)
}

/// Get valid token reduction levels.
///
/// Returns a list of all valid token reduction level values.
///
/// # Returns
///
/// Array of valid levels: ["off", "light", "moderate", "aggressive", "maximum"]
///
/// # Example
///
/// ```typescript
/// import { getValidTokenReductionLevels } from '@kreuzberg/node';
///
/// const levels = getValidTokenReductionLevels();
/// console.log(levels); // ['off', 'light', 'moderate', 'aggressive', 'maximum']
/// ```
#[napi(js_name = "getValidTokenReductionLevels")]
pub fn get_valid_token_reduction_levels() -> Result<Vec<String>> {
    let json_str = unsafe {
        let ptr = kreuzberg_get_valid_token_reduction_levels();
        if ptr.is_null() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Failed to get valid token reduction levels",
            ));
        }

        let c_str = CStr::from_ptr(ptr);
        let result = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in token reduction levels"))?
            .to_string();

        kreuzberg_free_string(ptr as *mut c_char);
        result
    };

    let parsed: Vec<String> = serde_json::from_str(&json_str).map_err(|_| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse token reduction levels JSON",
        )
    })?;

    Ok(parsed)
}

/// Validate and normalize an ExtractionConfig JSON string via FFI.
///
/// This validates the JSON and returns a normalized version, using the shared
/// FFI layer to ensure consistent validation across all language bindings.
///
/// # Arguments
///
/// * `json_str` - A JSON string containing the configuration
///
/// # Returns
///
/// The normalized JSON string representation of the config, or error
#[napi(js_name = "configValidateAndNormalize")]
pub fn config_validate_and_normalize(json_str: String) -> Result<String> {
    let c_str = std::ffi::CString::new(json_str.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in config JSON: {}", json_str),
        )
    })?;

    let config_ptr = unsafe { kreuzberg_config_from_json(c_str.as_ptr()) };

    if config_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse config from JSON",
        ));
    }

    let json_ptr = unsafe { kreuzberg_config_to_json(config_ptr) };
    unsafe {
        kreuzberg_config_free(config_ptr);
    }

    if json_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to serialize parsed config to JSON",
        ));
    }

    let result = unsafe {
        let c_str = CStr::from_ptr(json_ptr);
        let json_str = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in JSON"))?
            .to_string();

        kreuzberg_free_string(json_ptr as *mut c_char);
        json_str
    };

    Ok(result)
}

/// Get a specific field from config (represented as JSON string) by name via FFI.
///
/// Retrieves a configuration field by path, supporting nested access with
/// dot notation (e.g., "ocr.backend"). Returns the field value as a JSON string.
///
/// # Arguments
///
/// * `json_str` - A JSON string representation of the configuration
/// * `field_name` - The field path to retrieve (e.g., "useCache", "ocr.backend")
///
/// # Returns
///
/// The field value as a JSON string, or null if not found
#[napi(js_name = "configGetFieldInternal")]
pub fn config_get_field_internal(json_str: String, field_name: String) -> Result<Option<String>> {
    let c_str = std::ffi::CString::new(json_str.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in config JSON: {}", json_str),
        )
    })?;

    let config_ptr = unsafe { kreuzberg_config_from_json(c_str.as_ptr()) };

    if config_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse config from JSON",
        ));
    }

    let c_field_name = std::ffi::CString::new(field_name.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in field name: {}", field_name),
        )
    })?;

    let field_ptr = unsafe { kreuzberg_config_get_field(config_ptr, c_field_name.as_ptr()) };
    unsafe {
        kreuzberg_config_free(config_ptr);
    }

    if field_ptr.is_null() {
        return Ok(None);
    }

    let result = unsafe {
        let c_str = CStr::from_ptr(field_ptr);
        let field_str = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in field value"))?
            .to_string();

        kreuzberg_free_string(field_ptr as *mut c_char);
        field_str
    };

    Ok(Some(result))
}

/// Merge two configs (override takes precedence over base) via FFI.
///
/// Performs a shallow merge where fields from the override config take
/// precedence over fields in the base config.
///
/// # Arguments
///
/// * `base_json` - A JSON string representation of the base ExtractionConfig
/// * `override_json` - A JSON string representation of the override ExtractionConfig
///
/// # Returns
///
/// The merged configuration as a JSON string, or error
#[napi(js_name = "configMergeInternal")]
pub fn config_merge_internal(base_json: String, override_json: String) -> Result<String> {
    let base_c_str = std::ffi::CString::new(base_json.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in base config JSON: {}", base_json),
        )
    })?;

    let override_c_str = std::ffi::CString::new(override_json.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in override config JSON: {}", override_json),
        )
    })?;

    let base_ptr = unsafe { kreuzberg_config_from_json(base_c_str.as_ptr()) };

    if base_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse base config from JSON",
        ));
    }

    let override_ptr = unsafe { kreuzberg_config_from_json(override_c_str.as_ptr()) };

    if override_ptr.is_null() {
        unsafe {
            kreuzberg_config_free(base_ptr);
        }
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse override config from JSON",
        ));
    }

    let merge_result = unsafe { kreuzberg_config_merge(base_ptr, override_ptr) };

    if merge_result == 0 {
        unsafe {
            kreuzberg_config_free(base_ptr);
            kreuzberg_config_free(override_ptr);
        }
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to merge configs",
        ));
    }

    let json_ptr = unsafe { kreuzberg_config_to_json(base_ptr) };

    unsafe {
        kreuzberg_config_free(base_ptr);
        kreuzberg_config_free(override_ptr);
    }

    if json_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to serialize merged config to JSON",
        ));
    }

    let result = unsafe {
        let c_str = CStr::from_ptr(json_ptr);
        let json_str = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in JSON"))?
            .to_string();

        kreuzberg_free_string(json_ptr as *mut c_char);
        json_str
    };

    Ok(result)
}

// #[cfg(all(
// #[global_allocator]
