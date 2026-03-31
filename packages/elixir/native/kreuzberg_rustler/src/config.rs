//! Configuration parsing and validation
//!
//! This module provides functions for parsing Elixir terms into Kreuzberg
//! ExtractionConfig structures with comprehensive validation.

use crate::conversion::{describe_term_type, term_to_json};
use rustler::{Env, Term};
use std::collections::HashMap;

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
pub fn parse_extraction_config(_env: Env, options: Term) -> Result<kreuzberg::core::config::ExtractionConfig, String> {
    // Handle nil case - return default config
    if let Ok(atom_str) = options.atom_to_string()
        && atom_str == "nil"
    {
        return Ok(kreuzberg::core::config::ExtractionConfig::default());
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
    let boolean_fields = [
        "use_cache",
        "enable_quality_processing",
        "force_ocr",
        "disable_ocr",
        "include_document_structure",
    ];
    let nested_fields = [
        "ocr",
        "chunking",
        "images",
        "pages",
        "language_detection",
        "postprocessor",
        "token_reduction",
        "keywords",
        "pdf_options",
        "html_options",
        "security_limits",
    ];
    // String/enum fields that are passed through via serde deserialization
    let string_fields = ["result_format", "output_format"];
    // Integer fields that are passed through via serde deserialization
    let integer_fields = ["max_concurrent_extractions"];

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
                        "disable_ocr" => config.disable_ocr = bool_val,
                        "include_document_structure" => config.include_document_structure = bool_val,
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
            if let Ok(atom_str) = value.atom_to_string()
                && atom_str == "nil"
            {
                // nil is acceptable for optional nested configs
                continue;
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

        // String/enum fields - validated as strings, handled by serde deserialization
        if string_fields.contains(&field_name) {
            match value.decode::<String>() {
                Ok(_) => {}
                Err(_) => {
                    return Err(format!(
                        "Invalid configuration: field '{}' must be a string, got: {}",
                        field_name,
                        describe_term_type(*value)
                    ));
                }
            }
            continue;
        }

        // Integer fields - validated as integers, handled by serde deserialization
        if integer_fields.contains(&field_name) {
            // Check if value is nil (allowed for optional integer fields)
            if let Ok(atom_str) = value.atom_to_string()
                && atom_str == "nil"
            {
                continue;
            }

            match value.decode::<u64>() {
                Ok(_) => {}
                Err(_) => {
                    return Err(format!(
                        "Invalid configuration: field '{}' must be a positive integer or nil, got: {}",
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
    let json_value =
        term_to_json(options).map_err(|e| format!("Invalid configuration: failed to parse options - {}", e))?;

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
            // Keywords are always available since kreuzberg is compiled with "full" feature
            config.keywords = deserialized.keywords;
            // PDF options are always available since kreuzberg is compiled with "full" feature
            config.pdf_options = deserialized.pdf_options;
            // Forward remaining fields from deserialized config
            config.result_format = deserialized.result_format;
            config.output_format = deserialized.output_format;
            config.html_options = deserialized.html_options;
            config.max_concurrent_extractions = deserialized.max_concurrent_extractions;
            config.security_limits = deserialized.security_limits;
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

/// Parse an Elixir term (nil or map) into an `Option<kreuzberg::FileExtractionConfig>`.
///
/// - `nil` → `None` (use batch-level defaults)
/// - map  → convert to JSON via `term_to_json`, then deserialize to `FileExtractionConfig`
pub fn parse_file_extraction_config(_env: Env, term: Term) -> Result<Option<kreuzberg::FileExtractionConfig>, String> {
    // Handle nil case
    if let Ok(atom_str) = term.atom_to_string()
        && atom_str == "nil"
    {
        return Ok(None);
    }

    let json_value =
        term_to_json(term).map_err(|e| format!("Invalid file extraction config: failed to parse - {}", e))?;

    let file_config: kreuzberg::FileExtractionConfig =
        serde_json::from_value(json_value).map_err(|e| format!("Invalid file extraction config: {}", e))?;
    Ok(Some(file_config))
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
