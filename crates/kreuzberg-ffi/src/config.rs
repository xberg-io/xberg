//! Centralized FFI configuration parsing module.
//!
//! This module consolidates all configuration parsing logic that was previously
//! duplicated across all language bindings (Python, TypeScript, Ruby, Java, Go, C#).
//!
//! Instead of each binding reimplementing config parsing from JSON, they now
//! call the FFI functions provided here, ensuring:
//! - Single source of truth for validation rules
//! - Consistent behavior across all languages
//! - Elimination of drift/inconsistencies
//! - Better performance (no JSON round-trips in language bindings)

use crate::{clear_last_error, set_last_error};
use kreuzberg::core::config::ExtractionConfig;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

type FfiResult<T> = std::result::Result<T, String>;

/// Parse an ExtractionConfig from a JSON string.
///
/// This is the primary FFI entry point for all language bindings to parse
/// configuration from JSON. Replaces the need for each binding to implement
/// its own JSON parsing logic.
///
/// # Arguments
///
/// * `json_config` - Null-terminated C string containing JSON configuration
///
/// # Returns
///
/// A pointer to an ExtractionConfig struct that MUST be freed with
/// `kreuzberg_config_free`, or NULL on error (check kreuzberg_last_error).
///
/// # Safety
///
/// - `json_config` must be a valid null-terminated C string
/// - The returned pointer must be freed with `kreuzberg_config_free`
/// - Returns NULL if parsing fails (error available via `kreuzberg_last_error`)
///
/// # Example (C)
///
/// ```c
/// const char* config_json = "{\"use_cache\": true, \"ocr\": {\"backend\": \"tesseract\"}}";
/// ExtractionConfig* config = kreuzberg_config_from_json(config_json);
/// if (config == NULL) {
///     printf("Error: %s\n", kreuzberg_last_error());
///     return 1;
/// }
///
/// // Use config...
/// // char* result = kreuzberg_extract_file_with_config("doc.pdf", config);
///
/// kreuzberg_config_free(config);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_config_from_json(json_config: *const c_char) -> *mut ExtractionConfig {
    if json_config.is_null() {
        set_last_error("Config JSON cannot be NULL".to_string());
        return ptr::null_mut();
    }

    clear_last_error();

    let json_str = match unsafe { CStr::from_ptr(json_config) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in config JSON: {}", e));
            return ptr::null_mut();
        }
    };

    match parse_extraction_config_from_json(json_str) {
        Ok(config) => Box::into_raw(Box::new(config)),
        Err(e) => {
            set_last_error(e);
            ptr::null_mut()
        }
    }
}

/// Free an ExtractionConfig allocated by kreuzberg_config_from_json or similar.
///
/// # Safety
///
/// - `config` must be a pointer previously returned by a config creation function
/// - `config` can be NULL (no-op)
/// - `config` must not be used after this call
///
/// # Example (C)
///
/// ```c
/// ExtractionConfig* config = kreuzberg_config_from_json("{...}");
/// if (config != NULL) {
///     // Use config...
///     kreuzberg_config_free(config);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_config_free(config: *mut ExtractionConfig) {
    if !config.is_null() {
        let _ = unsafe { Box::from_raw(config) };
    }
}

/// Validate a JSON config string without parsing it.
///
/// This function checks if a JSON config string is valid and would parse correctly,
/// without allocating the full ExtractionConfig structure. Useful for validation
/// before committing to parsing.
///
/// # Arguments
///
/// * `json_config` - Null-terminated C string containing JSON configuration
///
/// # Returns
///
/// - 1 if valid (would parse successfully)
/// - 0 if invalid (check `kreuzberg_last_error` for details)
///
/// # Safety
///
/// - `json_config` must be a valid null-terminated C string
///
/// # Example (C)
///
/// ```c
/// const char* config_json = "{\"use_cache\": true}";
/// if (kreuzberg_config_is_valid(config_json)) {
///     ExtractionConfig* config = kreuzberg_config_from_json(config_json);
///     // Use config...
///     kreuzberg_config_free(config);
/// } else {
///     printf("Invalid config: %s\n", kreuzberg_last_error());
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_config_is_valid(json_config: *const c_char) -> i32 {
    if json_config.is_null() {
        set_last_error("Config JSON cannot be NULL".to_string());
        return 0;
    }

    clear_last_error();

    let json_str = match unsafe { CStr::from_ptr(json_config) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in config JSON: {}", e));
            return 0;
        }
    };

    match parse_extraction_config_from_json(json_str) {
        Ok(_) => 1,
        Err(e) => {
            set_last_error(e);
            0
        }
    }
}

/// Serialize an ExtractionConfig to JSON string.
///
/// Converts an ExtractionConfig structure to its JSON representation, allowing
/// bindings to serialize configs without reimplementing serialization logic.
///
/// # Arguments
///
/// * `config` - Pointer to an ExtractionConfig structure
///
/// # Returns
///
/// A pointer to a C string containing JSON that MUST be freed with `kreuzberg_free_string`.
/// Returns NULL on error (check `kreuzberg_last_error`).
///
/// # Safety
///
/// - `config` must be a valid pointer to an ExtractionConfig
/// - `config` cannot be NULL
/// - The returned pointer must be freed with `kreuzberg_free_string`
///
/// # Example (C)
///
/// ```c
/// ExtractionConfig* config = kreuzberg_config_from_json("{\"use_cache\": true}");
/// if (config != NULL) {
///     char* json = kreuzberg_config_to_json(config);
///     if (json != NULL) {
///         printf("Serialized: %s\n", json);
///         kreuzberg_free_string(json);
///     }
///     kreuzberg_config_free(config);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_config_to_json(config: *const ExtractionConfig) -> *mut c_char {
    if config.is_null() {
        set_last_error("Config cannot be NULL".to_string());
        return ptr::null_mut();
    }

    clear_last_error();

    match serde_json::to_string(unsafe { &*config }) {
        Ok(json) => match std::ffi::CString::new(json) {
            Ok(c_string) => c_string.into_raw(),
            Err(e) => {
                set_last_error(format!("Failed to convert JSON to C string: {}", e));
                ptr::null_mut()
            }
        },
        Err(e) => {
            set_last_error(format!("Failed to serialize config to JSON: {}", e));
            ptr::null_mut()
        }
    }
}

/// Get a specific field from config as JSON string.
///
/// Retrieves a nested field from the configuration by path and returns its JSON
/// representation. Supports dot notation for nested fields (e.g., "ocr.backend").
///
/// # Arguments
///
/// * `config` - Pointer to an ExtractionConfig structure
/// * `field_name` - Null-terminated C string with field path (e.g., "use_cache", "ocr.backend")
///
/// # Returns
///
/// A pointer to a C string containing the field value as JSON, or NULL if:
/// - The field doesn't exist
/// - An error occurs during serialization
///
/// The returned pointer (if non-NULL) must be freed with `kreuzberg_free_string`.
///
/// # Safety
///
/// - `config` must be a valid pointer to an ExtractionConfig
/// - `field_name` must be a valid null-terminated C string
/// - Neither parameter can be NULL
///
/// # Example (C)
///
/// ```c
/// ExtractionConfig* config = kreuzberg_config_from_json(
///     "{\"use_cache\": true, \"ocr\": {\"backend\": \"tesseract\"}}"
/// );
/// if (config != NULL) {
///     char* use_cache = kreuzberg_config_get_field(config, "use_cache");
///     char* backend = kreuzberg_config_get_field(config, "ocr.backend");
///
///     if (use_cache != NULL) {
///         printf("use_cache: %s\n", use_cache);
///         kreuzberg_free_string(use_cache);
///     }
///
///     if (backend != NULL) {
///         printf("backend: %s\n", backend);
///         kreuzberg_free_string(backend);
///     }
///
///     kreuzberg_config_free(config);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_config_get_field(
    config: *const ExtractionConfig,
    field_name: *const c_char,
) -> *mut c_char {
    if config.is_null() {
        set_last_error("Config cannot be NULL".to_string());
        return ptr::null_mut();
    }

    if field_name.is_null() {
        set_last_error("Field name cannot be NULL".to_string());
        return ptr::null_mut();
    }

    clear_last_error();

    let field_str = match unsafe { CStr::from_ptr(field_name) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in field name: {}", e));
            return ptr::null_mut();
        }
    };

    let json_value = match serde_json::to_value(unsafe { &*config }) {
        Ok(val) => val,
        Err(e) => {
            set_last_error(format!("Failed to serialize config: {}", e));
            return ptr::null_mut();
        }
    };

    let mut current = &json_value;
    for part in field_str.split('.') {
        if let Some(obj) = current.as_object() {
            match obj.get(part) {
                Some(val) => current = val,
                None => {
                    set_last_error(format!("Field '{}' not found in config", field_str));
                    return ptr::null_mut();
                }
            }
        } else {
            set_last_error(format!("Cannot access nested field '{}' in non-object", part));
            return ptr::null_mut();
        }
    }

    match serde_json::to_string(current) {
        Ok(json) => match std::ffi::CString::new(json) {
            Ok(c_string) => c_string.into_raw(),
            Err(e) => {
                set_last_error(format!("Failed to convert field value to C string: {}", e));
                ptr::null_mut()
            }
        },
        Err(e) => {
            set_last_error(format!("Failed to serialize field value: {}", e));
            ptr::null_mut()
        }
    }
}

/// Merge two configs (override takes precedence over base).
///
/// Performs a shallow merge of two ExtractionConfig structures, where fields
/// from `override_config` take precedence over fields in `base`. The `base`
/// config is modified in-place.
///
/// # Arguments
///
/// * `base` - Pointer to the base ExtractionConfig (will be modified)
/// * `override_config` - Pointer to the override ExtractionConfig (read-only)
///
/// # Returns
///
/// - 1 on success
/// - 0 on error (check `kreuzberg_last_error`)
///
/// # Safety
///
/// - `base` must be a valid mutable pointer to an ExtractionConfig
/// - `override_config` must be a valid pointer to an ExtractionConfig
/// - Neither parameter can be NULL
/// - `base` is modified in-place
///
/// # Example (C)
///
/// ```c
/// ExtractionConfig* base = kreuzberg_config_from_json(
///     "{\"use_cache\": true, \"force_ocr\": false}"
/// );
/// ExtractionConfig* override = kreuzberg_config_from_json(
///     "{\"force_ocr\": true}"
/// );
///
/// if (kreuzberg_config_merge(base, override) == 1) {
///     // base now has: use_cache=true, force_ocr=true
///     char* json = kreuzberg_config_to_json(base);
///     printf("Merged config: %s\n", json);
///     kreuzberg_free_string(json);
/// }
///
/// kreuzberg_config_free(base);
/// kreuzberg_config_free(override);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_config_merge(
    base: *mut ExtractionConfig,
    override_config: *const ExtractionConfig,
) -> i32 {
    if base.is_null() {
        set_last_error("Base config cannot be NULL".to_string());
        return 0;
    }

    if override_config.is_null() {
        set_last_error("Override config cannot be NULL".to_string());
        return 0;
    }

    clear_last_error();

    let base_ref = unsafe { &mut *base };
    let override_ref = unsafe { &*override_config };

    base_ref.use_cache = override_ref.use_cache;
    base_ref.enable_quality_processing = override_ref.enable_quality_processing;
    base_ref.force_ocr = override_ref.force_ocr;
    base_ref.max_concurrent_extractions = override_ref.max_concurrent_extractions;

    if override_ref.ocr.is_some() {
        base_ref.ocr = override_ref.ocr.clone();
    }

    if override_ref.chunking.is_some() {
        base_ref.chunking = override_ref.chunking.clone();
    }

    if override_ref.images.is_some() {
        base_ref.images = override_ref.images.clone();
    }

    #[cfg(feature = "pdf")]
    if override_ref.pdf_options.is_some() {
        base_ref.pdf_options = override_ref.pdf_options.clone();
    }

    if override_ref.token_reduction.is_some() {
        base_ref.token_reduction = override_ref.token_reduction.clone();
    }

    if override_ref.language_detection.is_some() {
        base_ref.language_detection = override_ref.language_detection.clone();
    }

    if override_ref.pages.is_some() {
        base_ref.pages = override_ref.pages.clone();
    }

    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    if override_ref.keywords.is_some() {
        base_ref.keywords = override_ref.keywords.clone();
    }

    if override_ref.postprocessor.is_some() {
        base_ref.postprocessor = override_ref.postprocessor.clone();
    }

    if override_ref.html_options.is_some() {
        base_ref.html_options = override_ref.html_options.clone();
    }

    1
}

/// Parse ExtractionConfig from JSON string.
///
/// This is the core parsing logic shared by all FFI functions that deal with
/// JSON configuration. It handles:
/// - JSON deserialization
/// - All validation rules
/// - Type conversions
/// - HTML options parsing (complex nested structure)
///
/// The error messages are user-friendly and include guidance on what went wrong.
fn parse_extraction_config_from_json(json_str: &str) -> FfiResult<ExtractionConfig> {
    use html_to_markdown_rs::options::{
        CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle,
        PreprocessingPreset, WhitespaceMode,
    };

    // ~keep: This function performs the JSON parsing and validation that was

    fn parse_enum<T, F>(value: Option<&serde_json::Value>, parse_fn: F) -> FfiResult<Option<T>>
    where
        F: Fn(&str) -> FfiResult<T>,
    {
        if let Some(raw) = value {
            let text = raw
                .as_str()
                .ok_or_else(|| "Expected string for enum field".to_string())?;
            return parse_fn(text).map(Some);
        }
        Ok(None)
    }

    fn parse_heading_style(value: &str) -> FfiResult<HeadingStyle> {
        match value.to_lowercase().as_str() {
            "atx" => Ok(HeadingStyle::Atx),
            "underlined" => Ok(HeadingStyle::Underlined),
            "atx_closed" => Ok(HeadingStyle::AtxClosed),
            other => Err(format!(
                "Invalid heading_style '{}'. Expected one of: atx, underlined, atx_closed",
                other
            )),
        }
    }

    fn parse_list_indent_type(value: &str) -> FfiResult<ListIndentType> {
        match value.to_lowercase().as_str() {
            "spaces" => Ok(ListIndentType::Spaces),
            "tabs" => Ok(ListIndentType::Tabs),
            other => Err(format!(
                "Invalid list_indent_type '{}'. Expected 'spaces' or 'tabs'",
                other
            )),
        }
    }

    fn parse_highlight_style(value: &str) -> FfiResult<HighlightStyle> {
        match value.to_lowercase().as_str() {
            "double_equal" | "==" | "highlight" => Ok(HighlightStyle::DoubleEqual),
            "html" => Ok(HighlightStyle::Html),
            "bold" => Ok(HighlightStyle::Bold),
            "none" => Ok(HighlightStyle::None),
            other => Err(format!(
                "Invalid highlight_style '{}'. Expected one of: double_equal, html, bold, none",
                other
            )),
        }
    }

    fn parse_whitespace_mode(value: &str) -> FfiResult<WhitespaceMode> {
        match value.to_lowercase().as_str() {
            "normalized" => Ok(WhitespaceMode::Normalized),
            "strict" => Ok(WhitespaceMode::Strict),
            other => Err(format!(
                "Invalid whitespace_mode '{}'. Expected 'normalized' or 'strict'",
                other
            )),
        }
    }

    fn parse_newline_style(value: &str) -> FfiResult<NewlineStyle> {
        match value.to_lowercase().as_str() {
            "spaces" => Ok(NewlineStyle::Spaces),
            "backslash" => Ok(NewlineStyle::Backslash),
            other => Err(format!(
                "Invalid newline_style '{}'. Expected 'spaces' or 'backslash'",
                other
            )),
        }
    }

    fn parse_code_block_style(value: &str) -> FfiResult<CodeBlockStyle> {
        match value.to_lowercase().as_str() {
            "indented" => Ok(CodeBlockStyle::Indented),
            "backticks" => Ok(CodeBlockStyle::Backticks),
            "tildes" => Ok(CodeBlockStyle::Tildes),
            other => Err(format!(
                "Invalid code_block_style '{}'. Expected 'indented', 'backticks', or 'tildes'",
                other
            )),
        }
    }

    #[allow(dead_code)]
    fn parse_preprocessing_preset(value: &str) -> FfiResult<PreprocessingPreset> {
        match value.to_lowercase().as_str() {
            "minimal" => Ok(PreprocessingPreset::Minimal),
            "standard" => Ok(PreprocessingPreset::Standard),
            "aggressive" => Ok(PreprocessingPreset::Aggressive),
            other => Err(format!(
                "Invalid preprocessing.preset '{}'. Expected one of: minimal, standard, aggressive",
                other
            )),
        }
    }

    fn parse_html_options(value: &serde_json::Value) -> FfiResult<ConversionOptions> {
        let mut opts = ConversionOptions::default();
        let obj = value
            .as_object()
            .ok_or_else(|| "html_options must be an object".to_string())?;

        if let Some(val) = obj.get("heading_style") {
            opts.heading_style = parse_enum(Some(val), parse_heading_style)?.unwrap_or(opts.heading_style);
        }

        if let Some(val) = obj.get("list_indent_type") {
            opts.list_indent_type = parse_enum(Some(val), parse_list_indent_type)?.unwrap_or(opts.list_indent_type);
        }

        if let Some(val) = obj.get("list_indent_width") {
            opts.list_indent_width = val
                .as_u64()
                .map(|v| v as usize)
                .ok_or_else(|| "list_indent_width must be an integer".to_string())?;
        }

        if let Some(val) = obj.get("bullets") {
            opts.bullets = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "bullets must be a string".to_string())?;
        }

        if let Some(val) = obj.get("strong_em_symbol") {
            let symbol = val
                .as_str()
                .ok_or_else(|| "strong_em_symbol must be a string".to_string())?;
            let mut chars = symbol.chars();
            opts.strong_em_symbol = chars
                .next()
                .ok_or_else(|| "strong_em_symbol must not be empty".to_string())?;
        }

        if let Some(val) = obj.get("escape_asterisks") {
            opts.escape_asterisks = val
                .as_bool()
                .ok_or_else(|| "escape_asterisks must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("escape_underscores") {
            opts.escape_underscores = val
                .as_bool()
                .ok_or_else(|| "escape_underscores must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("escape_misc") {
            opts.escape_misc = val
                .as_bool()
                .ok_or_else(|| "escape_misc must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("escape_ascii") {
            opts.escape_ascii = val
                .as_bool()
                .ok_or_else(|| "escape_ascii must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("code_language") {
            opts.code_language = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "code_language must be a string".to_string())?;
        }

        if let Some(val) = obj.get("autolinks") {
            opts.autolinks = val.as_bool().ok_or_else(|| "autolinks must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("default_title") {
            opts.default_title = val
                .as_bool()
                .ok_or_else(|| "default_title must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("br_in_tables") {
            opts.br_in_tables = val
                .as_bool()
                .ok_or_else(|| "br_in_tables must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("hocr_spatial_tables") {
            opts.hocr_spatial_tables = val
                .as_bool()
                .ok_or_else(|| "hocr_spatial_tables must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("highlight_style") {
            opts.highlight_style = parse_enum(Some(val), parse_highlight_style)?.unwrap_or(opts.highlight_style);
        }

        if let Some(val) = obj.get("extract_metadata") {
            opts.extract_metadata = val
                .as_bool()
                .ok_or_else(|| "extract_metadata must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("whitespace_mode") {
            opts.whitespace_mode = parse_enum(Some(val), parse_whitespace_mode)?.unwrap_or(opts.whitespace_mode);
        }

        if let Some(val) = obj.get("strip_newlines") {
            opts.strip_newlines = val
                .as_bool()
                .ok_or_else(|| "strip_newlines must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("wrap") {
            opts.wrap = val.as_bool().ok_or_else(|| "wrap must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("wrap_width") {
            opts.wrap_width = val
                .as_u64()
                .map(|v| v as usize)
                .ok_or_else(|| "wrap_width must be an integer".to_string())?;
        }

        if let Some(val) = obj.get("convert_as_inline") {
            opts.convert_as_inline = val
                .as_bool()
                .ok_or_else(|| "convert_as_inline must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("sub_symbol") {
            opts.sub_symbol = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "sub_symbol must be a string".to_string())?;
        }

        if let Some(val) = obj.get("sup_symbol") {
            opts.sup_symbol = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "sup_symbol must be a string".to_string())?;
        }

        if let Some(val) = obj.get("newline_style") {
            opts.newline_style = parse_enum(Some(val), parse_newline_style)?.unwrap_or(opts.newline_style);
        }

        if let Some(val) = obj.get("code_block_style") {
            opts.code_block_style = parse_enum(Some(val), parse_code_block_style)?.unwrap_or(opts.code_block_style);
        }

        if let Some(val) = obj.get("keep_inline_images_in") {
            opts.keep_inline_images_in = val
                .as_array()
                .ok_or_else(|| "keep_inline_images_in must be an array".to_string())?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| "keep_inline_images_in entries must be strings".to_string())
                })
                .collect::<FfiResult<Vec<_>>>()?;
        }

        if let Some(val) = obj.get("encoding") {
            opts.encoding = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "encoding must be a string".to_string())?;
        }

        if let Some(val) = obj.get("debug") {
            opts.debug = val.as_bool().ok_or_else(|| "debug must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("strip_tags") {
            opts.strip_tags = val
                .as_array()
                .ok_or_else(|| "strip_tags must be an array".to_string())?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| "strip_tags entries must be strings".to_string())
                })
                .collect::<FfiResult<Vec<_>>>()?;
        }

        if let Some(val) = obj.get("preserve_tags") {
            opts.preserve_tags = val
                .as_array()
                .ok_or_else(|| "preserve_tags must be an array".to_string())?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| "preserve_tags entries must be strings".to_string())
                })
                .collect::<FfiResult<Vec<_>>>()?;
        }

        Ok(opts)
    }

    let json_value: serde_json::Value = serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut config: ExtractionConfig =
        serde_json::from_value(json_value.clone()).map_err(|e| format!("Invalid configuration structure: {}", e))?;

    if let Some(html_opts_val) = json_value.get("html_options") {
        config.html_options = Some(parse_html_options(html_opts_val)?);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_parse_minimal_config() {
        let json = "{}";
        let result = parse_extraction_config_from_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_config_with_use_cache() {
        let json = r#"{"use_cache": true}"#;
        let result = parse_extraction_config_from_json(json);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.use_cache);
    }

    #[test]
    fn test_parse_config_with_ocr() {
        let json = r#"{"ocr": {"backend": "tesseract", "language": "eng"}}"#;
        let result = parse_extraction_config_from_json(json);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.ocr.is_some());
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.language, "eng");
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "{invalid json}";
        let result = parse_extraction_config_from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_complex_config() {
        let json = r#"{
            "use_cache": true,
            "enable_quality_processing": true,
            "force_ocr": false,
            "ocr": {
                "backend": "tesseract",
                "language": "eng"
            },
            "chunking": {
                "max_chars": 1024,
                "max_overlap": 128
            },
            "max_concurrent_extractions": 4
        }"#;
        let result = parse_extraction_config_from_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_to_json() {
        let json_str = r#"{"use_cache": true}"#;
        let config_ptr = unsafe { kreuzberg_config_from_json(std::ffi::CString::new(json_str).unwrap().as_ptr()) };
        assert!(!config_ptr.is_null());

        let json_out = unsafe { kreuzberg_config_to_json(config_ptr) };
        assert!(!json_out.is_null());

        let out_str = unsafe { CStr::from_ptr(json_out).to_str().unwrap() };
        assert!(out_str.contains("use_cache"));
        assert!(out_str.contains("true"));

        unsafe {
            crate::kreuzberg_free_string(json_out);
            kreuzberg_config_free(config_ptr);
        }
    }

    #[test]
    fn test_config_to_json_null_pointer() {
        let result = unsafe { kreuzberg_config_to_json(ptr::null()) };
        assert!(result.is_null());
    }

    #[test]
    fn test_config_get_field_simple() {
        let json_str = r#"{"use_cache": true}"#;
        let config_ptr = unsafe { kreuzberg_config_from_json(std::ffi::CString::new(json_str).unwrap().as_ptr()) };
        assert!(!config_ptr.is_null());

        let field_name = std::ffi::CString::new("use_cache").unwrap();
        let field_value = unsafe { kreuzberg_config_get_field(config_ptr, field_name.as_ptr()) };
        assert!(!field_value.is_null());

        let value_str = unsafe { CStr::from_ptr(field_value).to_str().unwrap() };
        assert_eq!(value_str, "true");

        unsafe {
            crate::kreuzberg_free_string(field_value);
            kreuzberg_config_free(config_ptr);
        }
    }

    #[test]
    fn test_config_get_field_nested() {
        let json_str = r#"{"ocr": {"backend": "tesseract"}}"#;
        let config_ptr = unsafe { kreuzberg_config_from_json(std::ffi::CString::new(json_str).unwrap().as_ptr()) };
        assert!(!config_ptr.is_null());

        let field_name = std::ffi::CString::new("ocr.backend").unwrap();
        let field_value = unsafe { kreuzberg_config_get_field(config_ptr, field_name.as_ptr()) };
        assert!(!field_value.is_null());

        let value_str = unsafe { CStr::from_ptr(field_value).to_str().unwrap() };
        assert_eq!(value_str, r#""tesseract""#);

        unsafe {
            crate::kreuzberg_free_string(field_value);
            kreuzberg_config_free(config_ptr);
        }
    }

    #[test]
    fn test_config_get_field_missing() {
        let json_str = r#"{"use_cache": true}"#;
        let config_ptr = unsafe { kreuzberg_config_from_json(std::ffi::CString::new(json_str).unwrap().as_ptr()) };
        assert!(!config_ptr.is_null());

        let field_name = std::ffi::CString::new("nonexistent").unwrap();
        let field_value = unsafe { kreuzberg_config_get_field(config_ptr, field_name.as_ptr()) };
        assert!(field_value.is_null());

        unsafe {
            kreuzberg_config_free(config_ptr);
        }
    }

    #[test]
    fn test_config_get_field_null_pointer() {
        let field_name = std::ffi::CString::new("use_cache").unwrap();
        let result = unsafe { kreuzberg_config_get_field(ptr::null(), field_name.as_ptr()) };
        assert!(result.is_null());
    }

    #[test]
    fn test_config_merge() {
        let base_json = r#"{"use_cache": true, "force_ocr": false}"#;
        let override_json = r#"{"force_ocr": true}"#;

        let base_ptr = unsafe { kreuzberg_config_from_json(std::ffi::CString::new(base_json).unwrap().as_ptr()) };
        let override_ptr =
            unsafe { kreuzberg_config_from_json(std::ffi::CString::new(override_json).unwrap().as_ptr()) };

        assert!(!base_ptr.is_null());
        assert!(!override_ptr.is_null());

        let result = unsafe { kreuzberg_config_merge(base_ptr, override_ptr) };
        assert_eq!(result, 1);

        let merged_json = unsafe { kreuzberg_config_to_json(base_ptr) };
        assert!(!merged_json.is_null());

        let merged_str = unsafe { CStr::from_ptr(merged_json).to_str().unwrap() };
        assert!(merged_str.contains("use_cache"));
        assert!(merged_str.contains("force_ocr"));

        unsafe {
            crate::kreuzberg_free_string(merged_json);
            kreuzberg_config_free(base_ptr);
            kreuzberg_config_free(override_ptr);
        }
    }

    #[test]
    fn test_config_merge_null_base() {
        let override_json = r#"{"force_ocr": true}"#;
        let override_ptr =
            unsafe { kreuzberg_config_from_json(std::ffi::CString::new(override_json).unwrap().as_ptr()) };

        let result = unsafe { kreuzberg_config_merge(ptr::null_mut(), override_ptr) };
        assert_eq!(result, 0);

        unsafe {
            kreuzberg_config_free(override_ptr);
        }
    }

    #[test]
    fn test_config_merge_null_override() {
        let base_json = r#"{"use_cache": true}"#;
        let base_ptr = unsafe { kreuzberg_config_from_json(std::ffi::CString::new(base_json).unwrap().as_ptr()) };

        let result = unsafe { kreuzberg_config_merge(base_ptr, ptr::null()) };
        assert_eq!(result, 0);

        unsafe {
            kreuzberg_config_free(base_ptr);
        }
    }

    #[test]
    fn test_config_merge_override_to_default_value() {
        let base_json = r#"{"use_cache": false}"#;
        let override_json = r#"{"use_cache": true}"#;

        let base_ptr = unsafe { kreuzberg_config_from_json(std::ffi::CString::new(base_json).unwrap().as_ptr()) };
        let override_ptr =
            unsafe { kreuzberg_config_from_json(std::ffi::CString::new(override_json).unwrap().as_ptr()) };

        assert!(!base_ptr.is_null());
        assert!(!override_ptr.is_null());

        let base_ref = unsafe { &*base_ptr };
        assert!(!base_ref.use_cache);

        let result = unsafe { kreuzberg_config_merge(base_ptr, override_ptr) };
        assert_eq!(result, 1);

        let base_ref = unsafe { &*base_ptr };
        assert!(base_ref.use_cache, "override to default value should be applied");

        unsafe {
            kreuzberg_config_free(base_ptr);
            kreuzberg_config_free(override_ptr);
        }
    }

    #[test]
    fn test_config_merge_override_force_ocr() {
        let base_json = r#"{"force_ocr": false}"#;
        let override_json = r#"{"force_ocr": true}"#;

        let base_ptr = unsafe { kreuzberg_config_from_json(std::ffi::CString::new(base_json).unwrap().as_ptr()) };
        let override_ptr =
            unsafe { kreuzberg_config_from_json(std::ffi::CString::new(override_json).unwrap().as_ptr()) };

        assert!(!base_ptr.is_null());
        assert!(!override_ptr.is_null());

        let result = unsafe { kreuzberg_config_merge(base_ptr, override_ptr) };
        assert_eq!(result, 1);

        let base_ref = unsafe { &*base_ptr };
        assert!(base_ref.force_ocr);

        unsafe {
            kreuzberg_config_free(base_ptr);
            kreuzberg_config_free(override_ptr);
        }
    }
}
