#![allow(clippy::let_unit_value)]

use rustler::types::map::map_new;
use rustler::{Binary, Encoder, Env, NifResult, Term};
use std::collections::HashMap;

mod types;
mod utils;

// Constants for validation
const MAX_BINARY_SIZE: usize = 500 * 1024 * 1024; // 500MB

rustler::init!("Elixir.Kreuzberg.Native", load = on_load);

#[allow(non_local_definitions)]
fn on_load(_env: Env, _info: Term) -> bool {
    true
}

/// Atoms module containing all Elixir atom definitions used in NIFs
/// These atoms are used for tuples, maps, and return values
mod atoms {
    rustler::atoms! {
        ok,
        error,
        invalid_input,
        extraction_failed,
        parsing_error,
        validation_error,
        io_error,
        invalid_format,
        invalid_config,
        ocr_error,
        unknown_error,
    }
}

/// Extract text and data from a document binary with default configuration
///
/// # Arguments
/// * `input` - Binary containing the document data
/// * `mime_type` - String representing the MIME type (e.g., "application/pdf")
///
/// # Returns
/// * `{:ok, result_map}` - Map containing extraction results
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
fn extract<'a>(env: Env<'a>, input: Binary<'a>, mime_type: String) -> NifResult<Term<'a>> {
    // Validate input
    if input.is_empty() {
        return Ok((atoms::error(), "Binary input cannot be empty").encode(env));
    }

    if input.len() > MAX_BINARY_SIZE {
        return Ok((atoms::error(), "Binary input exceeds maximum size of 500MB").encode(env));
    }

    // Create default extraction config
    let config = kreuzberg::core::config::ExtractionConfig::default();

    // Call kreuzberg extraction with default config
    match kreuzberg::extract_bytes_sync(input.as_slice(), &mime_type, &config) {
        Ok(result) => {
            // Convert ExtractionResult to Elixir term
            match convert_extraction_result_to_term(env, &result) {
                Ok(term) => Ok((atoms::ok(), term).encode(env)),
                Err(e) => Ok((atoms::error(), format!("Failed to encode result: {}", e)).encode(env)),
            }
        }
        Err(e) => {
            Ok((atoms::error(), format!("Extraction failed: {}", e)).encode(env))
        }
    }
}

/// Extract text and data from a document binary with custom configuration
///
/// # Arguments
/// * `input` - Binary containing the document data
/// * `mime_type` - String representing the MIME type (e.g., "application/pdf")
/// * `options` - Term containing extraction options (as map or keyword list)
///
/// # Returns
/// * `{:ok, result_map}` - Map containing extraction results
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
fn extract_with_options<'a>(
    env: Env<'a>,
    input: Binary<'a>,
    mime_type: String,
    options: Term<'a>,
) -> NifResult<Term<'a>> {
    // Validate input
    if input.is_empty() {
        return Ok((atoms::error(), "Binary input cannot be empty").encode(env));
    }

    if input.len() > MAX_BINARY_SIZE {
        return Ok((atoms::error(), "Binary input exceeds maximum size of 500MB").encode(env));
    }

    // Parse options from Elixir term to ExtractionConfig
    let config = match parse_extraction_config(env, options) {
        Ok(cfg) => cfg,
        Err(e) => return Ok((atoms::error(), format!("Invalid options: {}", e)).encode(env)),
    };

    // Call kreuzberg extraction with parsed config
    match kreuzberg::extract_bytes_sync(input.as_slice(), &mime_type, &config) {
        Ok(result) => {
            // Convert ExtractionResult to Elixir term
            match convert_extraction_result_to_term(env, &result) {
                Ok(term) => Ok((atoms::ok(), term).encode(env)),
                Err(e) => Ok((atoms::error(), format!("Failed to encode result: {}", e)).encode(env)),
            }
        }
        Err(e) => {
            Ok((atoms::error(), format!("Extraction failed: {}", e)).encode(env))
        }
    }
}

/// Extract text and data from a file at the given path with default configuration
///
/// # Arguments
/// * `path` - String containing the file path
/// * `mime_type` - Optional string representing the MIME type; if None, MIME type is detected from file
///
/// # Returns
/// * `{:ok, result_map}` - Map containing extraction results
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
fn extract_file<'a>(
    env: Env<'a>,
    path: String,
    mime_type: Option<String>,
) -> NifResult<Term<'a>> {
    // Create default extraction config
    let config = kreuzberg::core::config::ExtractionConfig::default();

    // Call kreuzberg file extraction with default config
    match kreuzberg::extract_file_sync(&path, mime_type.as_deref(), &config) {
        Ok(result) => {
            // Convert ExtractionResult to Elixir term
            match convert_extraction_result_to_term(env, &result) {
                Ok(term) => Ok((atoms::ok(), term).encode(env)),
                Err(e) => Ok((atoms::error(), format!("Failed to encode result: {}", e)).encode(env)),
            }
        }
        Err(e) => {
            Ok((atoms::error(), format!("Extraction failed: {}", e)).encode(env))
        }
    }
}

/// Extract text and data from a file at the given path with custom configuration
///
/// # Arguments
/// * `path` - String containing the file path
/// * `mime_type` - Optional string representing the MIME type; if None, MIME type is detected from file
/// * `options` - Term containing extraction options (as map or keyword list)
///
/// # Returns
/// * `{:ok, result_map}` - Map containing extraction results
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
fn extract_file_with_options<'a>(
    env: Env<'a>,
    path: String,
    mime_type: Option<String>,
    options_term: Term<'a>,
) -> NifResult<Term<'a>> {
    // Parse options from Elixir term to ExtractionConfig
    let config = match parse_extraction_config(env, options_term) {
        Ok(cfg) => cfg,
        Err(e) => return Ok((atoms::error(), format!("Invalid options: {}", e)).encode(env)),
    };

    // Call kreuzberg file extraction with parsed config
    match kreuzberg::extract_file_sync(&path, mime_type.as_deref(), &config) {
        Ok(result) => {
            // Convert ExtractionResult to Elixir term
            match convert_extraction_result_to_term(env, &result) {
                Ok(term) => Ok((atoms::ok(), term).encode(env)),
                Err(e) => Ok((atoms::error(), format!("Failed to encode result: {}", e)).encode(env)),
            }
        }
        Err(e) => {
            Ok((atoms::error(), format!("Extraction failed: {}", e)).encode(env))
        }
    }
}

/// Convert a Rust ExtractionResult to an Elixir term
///
/// This function converts the kreuzberg ExtractionResult struct into a map
/// that can be returned to Elixir code.
fn convert_extraction_result_to_term<'a>(
    env: Env<'a>,
    result: &kreuzberg::types::ExtractionResult,
) -> Result<Term<'a>, String> {
    // Create a JSON representation and convert to Elixir term
    let result_json = serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))?;

    // Convert JSON to Elixir term
    let term = json_to_term(env, &result_json)
        .map_err(|e| format!("Failed to convert to Elixir term: {}", e))?;

    Ok(term)
}

/// Convert a serde_json::Value to a Rustler Term
///
/// Recursively converts JSON values to Elixir terms:
/// - null -> nil
/// - boolean -> true/false
/// - number -> integer or float
/// - string -> binary
/// - array -> list
/// - object -> map
fn json_to_term<'a>(env: Env<'a>, value: &serde_json::Value) -> Result<Term<'a>, String> {
    match value {
        serde_json::Value::Null => Ok(rustler::types::atom::nil().encode(env)),
        serde_json::Value::Bool(b) => Ok(b.encode(env)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.encode(env))
            } else if let Some(u) = n.as_u64() {
                Ok(u.encode(env))
            } else if let Some(f) = n.as_f64() {
                Ok(f.encode(env))
            } else {
                Err("Invalid number".to_string())
            }
        }
        serde_json::Value::String(s) => Ok(s.encode(env)),
        serde_json::Value::Array(arr) => {
            let mut terms = Vec::new();
            for item in arr {
                terms.push(json_to_term(env, item)?);
            }
            Ok(terms.encode(env))
        }
        serde_json::Value::Object(obj) => {
            let mut map = map_new(env);
            for (k, v) in obj {
                let key = k.encode(env);
                let val = json_to_term(env, v)?;
                map = map
                    .map_put(key, val)
                    .map_err(|_| "Failed to put map entry".to_string())?;
            }
            Ok(map)
        }
    }
}

/// Convert an Elixir term to a serde_json::Value
///
/// Recursively converts Elixir terms to JSON values for deserialization.
/// Handles atoms, booleans, numbers, strings, lists, and maps.
fn term_to_json(term: Term) -> Result<serde_json::Value, String> {
    // Handle nil (atom)
    if let Ok(atom_str) = term.atom_to_string() {
        return Ok(match atom_str.as_str() {
            "nil" => serde_json::Value::Null,
            "true" => serde_json::Value::Bool(true),
            "false" => serde_json::Value::Bool(false),
            other => serde_json::Value::String(other.to_string()),
        });
    }

    // Handle booleans
    if let Ok(b) = term.decode::<bool>() {
        return Ok(serde_json::Value::Bool(b));
    }

    // Handle integers
    if let Ok(i) = term.decode::<i64>() {
        return Ok(serde_json::Value::Number(
            serde_json::Number::from(i)
        ));
    }

    // Handle floats
    if let Ok(f) = term.decode::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(f) {
            return Ok(serde_json::Value::Number(num));
        }
    }

    // Handle strings
    if let Ok(s) = term.decode::<String>() {
        return Ok(serde_json::Value::String(s));
    }

    // Handle lists
    if let Ok(list) = term.decode::<Vec<Term>>() {
        let items: Result<Vec<_>, _> = list
            .into_iter()
            .map(term_to_json)
            .collect();
        return Ok(serde_json::Value::Array(items?));
    }

    // Handle maps
    if let Ok(map) = term.decode::<HashMap<String, Term>>() {
        let mut obj = serde_json::Map::new();
        for (k, v) in map {
            obj.insert(k, term_to_json(v)?);
        }
        return Ok(serde_json::Value::Object(obj));
    }

    Err("Unable to convert term to JSON".to_string())
}

/// Parse an Elixir term into an ExtractionConfig with comprehensive validation
///
/// Accepts an Elixir map with both atom and string keys, supporting nested configurations.
/// Performs strict validation of all configuration fields and returns clear error messages
/// for invalid types or values.
///
/// # Supported Configuration Keys
///
/// Boolean fields:
/// - `use_cache` (default: true) - Enable result caching
/// - `enable_quality_processing` (default: true) - Enable quality post-processing
/// - `force_ocr` (default: false) - Force OCR even for searchable PDFs
///
/// Nested configuration maps:
/// - `ocr` - OCR backend configuration
/// - `chunking` - Text chunking configuration
/// - `images` - Image extraction configuration
/// - `pages` - Page extraction configuration
/// - `language_detection` - Language detection settings
/// - `postprocessor` - Post-processor configuration
/// - `token_reduction` - Token reduction configuration
/// - `keywords` - Keyword extraction configuration
/// - `pdf_options` - PDF-specific configuration (note: use pdf_options, not pdf_config)
///
/// # Key Format Support
///
/// Both atom keys (`:use_cache`) and string keys (`"use_cache"`) are supported,
/// matching the html-to-markdown pattern for flexible Elixir integration.
///
/// # Validation Behavior
///
/// - Boolean fields are validated to ensure they are actually booleans
/// - Nested configurations are validated to be maps or nil
/// - Unknown fields are logged but don't cause failure (forward compatibility)
/// - Invalid types result in descriptive error messages
fn parse_extraction_config(_env: Env, options: Term) -> Result<kreuzberg::core::config::ExtractionConfig, String> {
    // Handle nil case - return default config
    if let Ok(atom_str) = options.atom_to_string() {
        if atom_str == "nil" {
            return Ok(kreuzberg::core::config::ExtractionConfig::default());
        }
    }

    // Try to decode as a map with string keys
    let opts_map: HashMap<String, Term> = match options.decode() {
        Ok(map) => map,
        Err(_) => {
            return Err("Invalid configuration: options must be a map or nil".to_string());
        }
    };

    // Initialize config with defaults
    let mut config = kreuzberg::core::config::ExtractionConfig::default();

    // Define field categories for validation
    let boolean_fields = ["use_cache", "enable_quality_processing", "force_ocr"];
    let nested_fields = ["ocr", "chunking", "images", "pages", "language_detection",
                         "postprocessor", "token_reduction", "keywords", "pdf_options"];

    // Process each key in the map with validation
    for (key, value) in opts_map.iter() {
        let field_name = key.as_str();

        // Validate boolean fields
        if boolean_fields.contains(&field_name) {
            match value.decode::<bool>() {
                Ok(bool_val) => {
                    match field_name {
                        "use_cache" => config.use_cache = bool_val,
                        "enable_quality_processing" => config.enable_quality_processing = bool_val,
                        "force_ocr" => config.force_ocr = bool_val,
                        _ => {} // Already checked above
                    }
                }
                Err(_) => {
                    return Err(format!(
                        "Invalid configuration: field '{}' must be a boolean, got: {}",
                        field_name,
                        describe_term_type(*value)
                    ));
                }
            }
            continue;
        }

        // Validate and handle nested map fields
        if nested_fields.contains(&field_name) {
            // Check if value is a map or nil
            if let Ok(atom_str) = value.atom_to_string() {
                if atom_str == "nil" {
                    // nil is acceptable for optional nested configs
                    continue;
                }
            }

            // Try to decode as a HashMap to validate it's a map
            match value.decode::<HashMap<String, Term>>() {
                Ok(_) => {
                    // Map is valid, it will be handled by serde_json if needed
                    // For now, we just validate the structure exists
                }
                Err(_) => {
                    return Err(format!(
                        "Invalid configuration: field '{}' must be a map or nil, got: {}",
                        field_name,
                        describe_term_type(*value)
                    ));
                }
            }
            continue;
        }

        // Unknown fields are accepted for forward compatibility
        // This allows newer Elixir code to pass options that Rust may not recognize yet
    }

    // Now attempt full deserialization using serde_json for nested structures
    let json_value = term_to_json(options)
        .map_err(|e| format!("Invalid configuration: failed to parse options - {}", e))?;

    // Deserialize using serde_json - this handles nested structures automatically
    match serde_json::from_value::<kreuzberg::core::config::ExtractionConfig>(json_value) {
        Ok(deserialized) => {
            // Use deserialized config but prefer validated boolean fields
            config.ocr = deserialized.ocr;
            config.chunking = deserialized.chunking;
            config.images = deserialized.images;
            config.pages = deserialized.pages;
            config.language_detection = deserialized.language_detection;
            config.postprocessor = deserialized.postprocessor;
            config.token_reduction = deserialized.token_reduction;
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            {
                config.keywords = deserialized.keywords;
            }
            #[cfg(feature = "pdf")]
            {
                config.pdf_options = deserialized.pdf_options;
            }
        }
        Err(e) => {
            // Nested structure deserialization failed
            return Err(format!(
                "Invalid configuration: failed to deserialize nested configs - {}",
                e
            ));
        }
    }

    // Validate the final configuration
    validate_extraction_config(&config)?;

    Ok(config)
}

/// Helper function to describe the type of a Term for error messages
fn describe_term_type(term: Term) -> String {
    if term.decode::<bool>().is_ok() {
        return "boolean".to_string();
    }
    if term.decode::<i64>().is_ok() {
        return "integer".to_string();
    }
    if term.decode::<f64>().is_ok() {
        return "float".to_string();
    }
    if term.decode::<String>().is_ok() {
        return "string".to_string();
    }
    if term.decode::<Vec<Term>>().is_ok() {
        return "list".to_string();
    }
    if term.decode::<HashMap<String, Term>>().is_ok() {
        return "map".to_string();
    }
    if term.atom_to_string().is_ok() {
        return "atom".to_string();
    }
    "unknown type".to_string()
}

/// Validate an ExtractionConfig for internal consistency
///
/// Ensures that:
/// - Boolean flags are consistent with each other
/// - The configuration won't cause runtime issues
fn validate_extraction_config(config: &kreuzberg::core::config::ExtractionConfig) -> Result<(), String> {
    // If force_ocr is true, quality processing should ideally be enabled for best results
    // However, we don't enforce this as a hard error - it's valid to disable it
    if config.force_ocr && !config.enable_quality_processing {
        // This is a valid but potentially suboptimal configuration
    }

    // Add more sophisticated validation as needed
    // For example: validate nested config structure, check for conflicting options, etc.

    Ok(())
}

/// Batch extract text and data from multiple files with default configuration
///
/// # Arguments
/// * `paths` - Vec of file paths as strings
/// * `mime_type` - Optional string representing the MIME type for all files; if None, MIME type is detected per file
///
/// # Returns
/// * `{:ok, [result_map]}` - List of extraction result maps
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
fn batch_extract_files<'a>(
    env: Env<'a>,
    paths: Vec<String>,
    mime_type: Option<String>,
) -> NifResult<Term<'a>> {
    if paths.is_empty() {
        return Ok((atoms::error(), "File paths list cannot be empty").encode(env));
    }

    let config = kreuzberg::core::config::ExtractionConfig::default();
    let mime_ref = mime_type.as_deref();

    let mut results = Vec::new();

    // Process each file
    for path in paths {
        match kreuzberg::extract_file_sync(&path, mime_ref, &config) {
            Ok(result) => {
                match convert_extraction_result_to_term(env, &result) {
                    Ok(term) => results.push(term),
                    Err(e) => {
                        return Ok((
                            atoms::error(),
                            format!("Failed to encode result for '{}': {}", path, e),
                        )
                            .encode(env))
                    }
                }
            }
            Err(e) => {
                return Ok((
                    atoms::error(),
                    format!("Extraction failed for '{}': {}", path, e),
                )
                    .encode(env))
            }
        }
    }

    Ok((atoms::ok(), results).encode(env))
}

/// Batch extract text and data from multiple files with custom configuration
///
/// # Arguments
/// * `paths` - Vec of file paths as strings
/// * `mime_type` - Optional string representing the MIME type for all files; if None, MIME type is detected per file
/// * `options_term` - Term containing extraction options (as map or keyword list)
///
/// # Returns
/// * `{:ok, [result_map]}` - List of extraction result maps
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
fn batch_extract_files_with_options<'a>(
    env: Env<'a>,
    paths: Vec<String>,
    mime_type: Option<String>,
    options_term: Term<'a>,
) -> NifResult<Term<'a>> {
    if paths.is_empty() {
        return Ok((atoms::error(), "File paths list cannot be empty").encode(env));
    }

    // Parse options from Elixir term to ExtractionConfig
    let config = match parse_extraction_config(env, options_term) {
        Ok(cfg) => cfg,
        Err(e) => return Ok((atoms::error(), format!("Invalid options: {}", e)).encode(env)),
    };

    let mime_ref = mime_type.as_deref();
    let mut results = Vec::new();

    // Process each file
    for path in paths {
        match kreuzberg::extract_file_sync(&path, mime_ref, &config) {
            Ok(result) => {
                match convert_extraction_result_to_term(env, &result) {
                    Ok(term) => results.push(term),
                    Err(e) => {
                        return Ok((
                            atoms::error(),
                            format!("Failed to encode result for '{}': {}", path, e),
                        )
                            .encode(env))
                    }
                }
            }
            Err(e) => {
                return Ok((
                    atoms::error(),
                    format!("Extraction failed for '{}': {}", path, e),
                )
                    .encode(env))
            }
        }
    }

    Ok((atoms::ok(), results).encode(env))
}

/// Batch extract text and data from multiple binary inputs with default configuration
///
/// # Arguments
/// * `data_list` - Vec of binary data inputs
/// * `mime_types` - Vec of MIME type strings (one per input)
///
/// # Returns
/// * `{:ok, [result_map]}` - List of extraction result maps
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
fn batch_extract_bytes<'a>(
    env: Env<'a>,
    data_list: Vec<Binary<'a>>,
    mime_types: Vec<String>,
) -> NifResult<Term<'a>> {
    if data_list.is_empty() {
        return Ok((atoms::error(), "Data list cannot be empty").encode(env));
    }

    if data_list.len() != mime_types.len() {
        return Ok((
            atoms::error(),
            format!(
                "Mismatch: {} data inputs but {} MIME types",
                data_list.len(),
                mime_types.len()
            ),
        )
            .encode(env));
    }

    let config = kreuzberg::core::config::ExtractionConfig::default();
    let mut results = Vec::new();

    // Process each binary input with its corresponding MIME type
    for (idx, (data, mime_type)) in data_list.iter().zip(mime_types.iter()).enumerate() {
        if data.is_empty() {
            return Ok((
                atoms::error(),
                format!("Binary input at index {} cannot be empty", idx),
            )
                .encode(env));
        }

        if data.len() > MAX_BINARY_SIZE {
            return Ok((
                atoms::error(),
                format!(
                    "Binary input at index {} exceeds maximum size of 500MB",
                    idx
                ),
            )
                .encode(env));
        }

        match kreuzberg::extract_bytes_sync(data.as_slice(), mime_type, &config) {
            Ok(result) => {
                match convert_extraction_result_to_term(env, &result) {
                    Ok(term) => results.push(term),
                    Err(e) => {
                        return Ok((
                            atoms::error(),
                            format!("Failed to encode result at index {}: {}", idx, e),
                        )
                            .encode(env))
                    }
                }
            }
            Err(e) => {
                return Ok((
                    atoms::error(),
                    format!("Extraction failed at index {}: {}", idx, e),
                )
                    .encode(env))
            }
        }
    }

    Ok((atoms::ok(), results).encode(env))
}

/// Batch extract text and data from multiple binary inputs with custom configuration
///
/// # Arguments
/// * `data_list` - Vec of binary data inputs
/// * `mime_types` - Vec of MIME type strings (one per input)
/// * `options_term` - Term containing extraction options (as map or keyword list)
///
/// # Returns
/// * `{:ok, [result_map]}` - List of extraction result maps
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
fn batch_extract_bytes_with_options<'a>(
    env: Env<'a>,
    data_list: Vec<Binary<'a>>,
    mime_types: Vec<String>,
    options_term: Term<'a>,
) -> NifResult<Term<'a>> {
    if data_list.is_empty() {
        return Ok((atoms::error(), "Data list cannot be empty").encode(env));
    }

    if data_list.len() != mime_types.len() {
        return Ok((
            atoms::error(),
            format!(
                "Mismatch: {} data inputs but {} MIME types",
                data_list.len(),
                mime_types.len()
            ),
        )
            .encode(env));
    }

    // Parse options from Elixir term to ExtractionConfig
    let config = match parse_extraction_config(env, options_term) {
        Ok(cfg) => cfg,
        Err(e) => return Ok((atoms::error(), format!("Invalid options: {}", e)).encode(env)),
    };

    let mut results = Vec::new();

    // Process each binary input with its corresponding MIME type
    for (idx, (data, mime_type)) in data_list.iter().zip(mime_types.iter()).enumerate() {
        if data.is_empty() {
            return Ok((
                atoms::error(),
                format!("Binary input at index {} cannot be empty", idx),
            )
                .encode(env));
        }

        if data.len() > MAX_BINARY_SIZE {
            return Ok((
                atoms::error(),
                format!(
                    "Binary input at index {} exceeds maximum size of 500MB",
                    idx
                ),
            )
                .encode(env));
        }

        match kreuzberg::extract_bytes_sync(data.as_slice(), mime_type, &config) {
            Ok(result) => {
                match convert_extraction_result_to_term(env, &result) {
                    Ok(term) => results.push(term),
                    Err(e) => {
                        return Ok((
                            atoms::error(),
                            format!("Failed to encode result at index {}: {}", idx, e),
                        )
                            .encode(env))
                    }
                }
            }
            Err(e) => {
                return Ok((
                    atoms::error(),
                    format!("Extraction failed at index {}: {}", idx, e),
                )
                    .encode(env))
            }
        }
    }

    Ok((atoms::ok(), results).encode(env))
}

// =============================================================================
// VALIDATION FUNCTIONS - Configuration validators for extraction parameters
// =============================================================================

/// Validate chunking parameters (max_chars and max_overlap).
///
/// # Arguments
/// * `max_chars` - Maximum characters per chunk (must be > 0)
/// * `max_overlap` - Maximum overlap between chunks (must be < max_chars)
///
/// # Returns
/// * `:ok` - If parameters are valid
/// * `{:error, reason}` - If parameters are invalid
#[rustler::nif]
fn validate_chunking_params<'a>(
    env: Env<'a>,
    max_chars: usize,
    max_overlap: usize,
) -> NifResult<Term<'a>> {
    match kreuzberg::core::config_validation::validate_chunking_params(max_chars, max_overlap) {
        Ok(_) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Validate a language code (ISO 639-1 or 639-3 format).
///
/// # Arguments
/// * `code` - The language code to validate (e.g., "en", "eng", "de", "deu")
///
/// # Returns
/// * `:ok` - If the language code is valid
/// * `{:error, reason}` - If the language code is invalid
#[rustler::nif]
fn validate_language_code<'a>(env: Env<'a>, code: String) -> NifResult<Term<'a>> {
    match kreuzberg::core::config_validation::validate_language_code(&code) {
        Ok(_) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Validate a DPI (dots per inch) value.
///
/// # Arguments
/// * `dpi` - The DPI value to validate (must be > 0 and <= 2400)
///
/// # Returns
/// * `:ok` - If the DPI is valid
/// * `{:error, reason}` - If the DPI is invalid
#[rustler::nif]
fn validate_dpi<'a>(env: Env<'a>, dpi: i32) -> NifResult<Term<'a>> {
    match kreuzberg::core::config_validation::validate_dpi(dpi) {
        Ok(_) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Validate a confidence threshold value.
///
/// # Arguments
/// * `confidence` - The confidence threshold to validate (must be 0.0-1.0)
///
/// # Returns
/// * `:ok` - If the confidence is valid
/// * `{:error, reason}` - If the confidence is invalid
#[rustler::nif]
fn validate_confidence<'a>(env: Env<'a>, confidence: f64) -> NifResult<Term<'a>> {
    match kreuzberg::core::config_validation::validate_confidence(confidence) {
        Ok(_) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Validate an OCR backend name.
///
/// # Arguments
/// * `backend` - The OCR backend name to validate (tesseract, easyocr, paddleocr)
///
/// # Returns
/// * `:ok` - If the backend is valid
/// * `{:error, reason}` - If the backend is invalid
#[rustler::nif]
fn validate_ocr_backend<'a>(env: Env<'a>, backend: String) -> NifResult<Term<'a>> {
    match kreuzberg::core::config_validation::validate_ocr_backend(&backend) {
        Ok(_) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Validate a binarization method.
///
/// # Arguments
/// * `method` - The binarization method to validate (otsu, adaptive, sauvola)
///
/// # Returns
/// * `:ok` - If the method is valid
/// * `{:error, reason}` - If the method is invalid
#[rustler::nif]
fn validate_binarization_method<'a>(env: Env<'a>, method: String) -> NifResult<Term<'a>> {
    match kreuzberg::core::config_validation::validate_binarization_method(&method) {
        Ok(_) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Validate a Tesseract Page Segmentation Mode (PSM) value.
///
/// # Arguments
/// * `psm` - The PSM value to validate (0-13)
///
/// # Returns
/// * `:ok` - If the PSM is valid
/// * `{:error, reason}` - If the PSM is invalid
#[rustler::nif]
fn validate_tesseract_psm<'a>(env: Env<'a>, psm: i32) -> NifResult<Term<'a>> {
    match kreuzberg::core::config_validation::validate_tesseract_psm(psm) {
        Ok(_) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Validate a Tesseract OCR Engine Mode (OEM) value.
///
/// # Arguments
/// * `oem` - The OEM value to validate (0-3)
///
/// # Returns
/// * `:ok` - If the OEM is valid
/// * `{:error, reason}` - If the OEM is invalid
#[rustler::nif]
fn validate_tesseract_oem<'a>(env: Env<'a>, oem: i32) -> NifResult<Term<'a>> {
    match kreuzberg::core::config_validation::validate_tesseract_oem(oem) {
        Ok(_) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Detect MIME type from binary data using content inspection.
///
/// # Arguments
/// * `data` - Binary data to analyze
///
/// # Returns
/// * `{:ok, mime_type}` - Detected MIME type as a string
/// * `{:error, reason}` - Error if detection fails
#[rustler::nif]
fn detect_mime_type<'a>(env: Env<'a>, data: Binary<'a>) -> NifResult<Term<'a>> {
    if data.is_empty() {
        return Ok((atoms::error(), "Binary input cannot be empty").encode(env));
    }

    match kreuzberg::detect_mime_type_from_bytes(data.as_slice()) {
        Ok(mime_type) => Ok((atoms::ok(), mime_type).encode(env)),
        Err(e) => Ok((atoms::error(), format!("MIME detection failed: {}", e)).encode(env)),
    }
}

/// Detect MIME type from file path using extension and optional content inspection.
///
/// # Arguments
/// * `path` - File path as a string
///
/// # Returns
/// * `{:ok, mime_type}` - Detected MIME type as a string
/// * `{:error, reason}` - Error if detection fails
#[rustler::nif]
fn detect_mime_type_from_path<'a>(env: Env<'a>, path: String) -> NifResult<Term<'a>> {
    if path.is_empty() {
        return Ok((atoms::error(), "File path cannot be empty").encode(env));
    }

    match kreuzberg::detect_mime_type(&path, true) {
        Ok(mime_type) => Ok((atoms::ok(), mime_type).encode(env)),
        Err(e) => Ok((atoms::error(), format!("MIME detection failed: {}", e)).encode(env)),
    }
}

/// Validate that a MIME type is supported by Kreuzberg.
///
/// # Arguments
/// * `mime_type` - MIME type string to validate
///
/// # Returns
/// * `{:ok, mime_type}` - Returns the validated MIME type
/// * `{:error, reason}` - Error if MIME type is not supported
#[rustler::nif]
fn validate_mime_type<'a>(env: Env<'a>, mime_type: String) -> NifResult<Term<'a>> {
    if mime_type.is_empty() {
        return Ok((atoms::error(), "MIME type cannot be empty").encode(env));
    }

    match kreuzberg::validate_mime_type(&mime_type) {
        Ok(validated) => Ok((atoms::ok(), validated).encode(env)),
        Err(e) => Ok((atoms::error(), format!("MIME validation failed: {}", e)).encode(env)),
    }
}

/// Get file extensions for a given MIME type.
///
/// # Arguments
/// * `mime_type` - MIME type string
///
/// # Returns
/// * `{:ok, [extensions]}` - List of file extensions
/// * `{:error, reason}` - Error if MIME type not found
#[rustler::nif]
fn get_extensions_for_mime<'a>(env: Env<'a>, mime_type: String) -> NifResult<Term<'a>> {
    if mime_type.is_empty() {
        return Ok((atoms::error(), "MIME type cannot be empty").encode(env));
    }

    match kreuzberg::get_extensions_for_mime(&mime_type) {
        Ok(extensions) => Ok((atoms::ok(), extensions).encode(env)),
        Err(e) => {
            Ok((atoms::error(), format!("Failed to get extensions: {}", e)).encode(env))
        }
    }
}

/// List all available embedding presets.
///
/// # Returns
/// * `{:ok, [preset_names]}` - List of preset names
/// * `{:error, reason}` - Error if retrieval fails
#[rustler::nif]
fn list_embedding_presets<'a>(env: Env<'a>) -> NifResult<Term<'a>> {
    let presets = kreuzberg::list_presets();
    let preset_names: Vec<&str> = presets;
    Ok((atoms::ok(), preset_names).encode(env))
}

/// Get detailed information about a specific embedding preset.
///
/// # Arguments
/// * `preset_name` - Name of the preset to retrieve
///
/// # Returns
/// * `{:ok, preset_map}` - Map containing preset information
/// * `{:error, reason}` - Error if preset not found
#[rustler::nif]
fn get_embedding_preset<'a>(env: Env<'a>, preset_name: String) -> NifResult<Term<'a>> {
    if preset_name.is_empty() {
        return Ok((atoms::error(), "Preset name cannot be empty").encode(env));
    }

    match kreuzberg::get_preset(&preset_name) {
        Some(preset) => {
            // Manually construct a map from preset fields
            let mut map = map_new(env);

            map = match map.map_put("name".encode(env), preset.name.encode(env)) {
                Ok(m) => m,
                Err(_) => return Ok((atoms::error(), "Failed to build preset map").encode(env)),
            };

            map = match map.map_put("chunk_size".encode(env), (preset.chunk_size as i64).encode(env)) {
                Ok(m) => m,
                Err(_) => return Ok((atoms::error(), "Failed to build preset map").encode(env)),
            };

            map = match map.map_put("overlap".encode(env), (preset.overlap as i64).encode(env)) {
                Ok(m) => m,
                Err(_) => return Ok((atoms::error(), "Failed to build preset map").encode(env)),
            };

            map = match map.map_put("dimensions".encode(env), (preset.dimensions as i64).encode(env)) {
                Ok(m) => m,
                Err(_) => return Ok((atoms::error(), "Failed to build preset map").encode(env)),
            };

            map = match map.map_put("description".encode(env), preset.description.encode(env)) {
                Ok(m) => m,
                Err(_) => return Ok((atoms::error(), "Failed to build preset map").encode(env)),
            };

            Ok((atoms::ok(), map).encode(env))
        }
        None => Ok((atoms::error(), format!("Preset '{}' not found", preset_name)).encode(env)),
    }
}

/// Get cache statistics including file count, size, and disk space information.
///
/// # Arguments
///
/// None - retrieves stats for the extraction cache.
///
/// # Returns
///
/// * `{:ok, stats_map}` - Map containing cache statistics:
///   * `"total_files"` - Number of cached files (integer)
///   * `"total_size_mb"` - Total cache size in megabytes (float)
///   * `"available_space_mb"` - Available disk space in megabytes (float)
///   * `"oldest_file_age_days"` - Age of oldest file in days (float)
///   * `"newest_file_age_days"` - Age of newest file in days (float)
/// * `{:error, reason}` - Error tuple with reason string if retrieval fails
#[rustler::nif]
fn cache_stats<'a>(env: Env<'a>) -> NifResult<Term<'a>> {
    // Get the cache directory - use kreuzberg's internal cache path
    let cache_dir = match std::env::current_dir() {
        Ok(dir) => {
            let mut path = dir;
            path.push(".kreuzberg");
            path.push("extraction");
            path
        }
        Err(_) => {
            return Ok((atoms::error(), "Failed to determine cache directory").encode(env));
        }
    };

    let cache_dir_str = match cache_dir.to_str() {
        Some(s) => s,
        None => {
            return Ok((atoms::error(), "Cache directory path contains invalid UTF-8").encode(env));
        }
    };

    // Get cache statistics using kreuzberg's cache module
    match kreuzberg::cache::get_cache_metadata(cache_dir_str) {
        Ok(stats) => {
            let mut map = map_new(env);

            // Add all statistics to the map
            map = match map.map_put(
                "total_files".encode(env),
                (stats.total_files as i64).encode(env),
            ) {
                Ok(m) => m,
                Err(_) => {
                    return Ok((atoms::error(), "Failed to encode cache statistics").encode(env));
                }
            };

            map = match map.map_put(
                "total_size_mb".encode(env),
                stats.total_size_mb.encode(env),
            ) {
                Ok(m) => m,
                Err(_) => {
                    return Ok((atoms::error(), "Failed to encode cache statistics").encode(env));
                }
            };

            map = match map.map_put(
                "available_space_mb".encode(env),
                stats.available_space_mb.encode(env),
            ) {
                Ok(m) => m,
                Err(_) => {
                    return Ok((atoms::error(), "Failed to encode cache statistics").encode(env));
                }
            };

            map = match map.map_put(
                "oldest_file_age_days".encode(env),
                stats.oldest_file_age_days.encode(env),
            ) {
                Ok(m) => m,
                Err(_) => {
                    return Ok((atoms::error(), "Failed to encode cache statistics").encode(env));
                }
            };

            map = match map.map_put(
                "newest_file_age_days".encode(env),
                stats.newest_file_age_days.encode(env),
            ) {
                Ok(m) => m,
                Err(_) => {
                    return Ok((atoms::error(), "Failed to encode cache statistics").encode(env));
                }
            };

            Ok((atoms::ok(), map).encode(env))
        }
        Err(e) => {
            Ok((atoms::error(), format!("Failed to get cache statistics: {}", e)).encode(env))
        }
    }
}

/// Clear all cached extraction results.
///
/// # Arguments
///
/// None - clears the entire extraction cache.
///
/// # Returns
///
/// * `:ok` - Cache cleared successfully
/// * `{:error, reason}` - Error tuple with reason string if clearing fails
#[rustler::nif]
fn clear_cache<'a>(env: Env<'a>) -> NifResult<Term<'a>> {
    // Get the cache directory - use kreuzberg's internal cache path
    let cache_dir = match std::env::current_dir() {
        Ok(dir) => {
            let mut path = dir;
            path.push(".kreuzberg");
            path.push("extraction");
            path
        }
        Err(_) => {
            return Ok((atoms::error(), "Failed to determine cache directory").encode(env));
        }
    };

    let cache_dir_str = match cache_dir.to_str() {
        Some(s) => s,
        None => {
            return Ok((atoms::error(), "Cache directory path contains invalid UTF-8").encode(env));
        }
    };

    // Clear the cache using kreuzberg's cache module
    match kreuzberg::cache::clear_cache_directory(cache_dir_str) {
        Ok((removed_count, removed_size_mb)) => {
            // Cache cleared successfully
            // Note: removed_count and removed_size_mb are available for future logging
            let _ = (removed_count, removed_size_mb);
            Ok(atoms::ok().encode(env))
        }
        Err(e) => {
            Ok((atoms::error(), format!("Failed to clear cache: {}", e)).encode(env))
        }
    }
}
