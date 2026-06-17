//! Serialization and field extraction helpers
//!
//! Utilities for converting ExtractionConfig to JSON and extracting specific fields.

use crate::helpers::set_last_error;
use kreuzberg::core::config::ExtractionConfig;
#[cfg(feature = "embeddings")]
use serde::Serialize;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// SerializableEmbeddingPreset for FFI serialization.
#[cfg(feature = "embeddings")]
#[derive(Serialize)]
pub struct SerializableEmbeddingPreset<'a> {
    pub name: &'a str,
    pub chunk_size: usize,
    pub overlap: usize,
    pub model_name: String,
    pub dimensions: usize,
    pub description: &'a str,
}

/// Serialize an ExtractionConfig to JSON string.
///
/// # Arguments
///
/// * `config` - Reference to an ExtractionConfig
///
/// # Returns
///
/// JSON string on success, or None on error.
pub fn config_to_json_string(config: &ExtractionConfig) -> Option<String> {
    serde_json::to_string(config).ok()
}

/// Convert a JSON value to C string pointer
pub fn json_to_c_string(json: String) -> *mut c_char {
    match CString::new(json) {
        Ok(c_string) => c_string.into_raw(),
        Err(e) => {
            set_last_error(format!("Failed to convert JSON to C string: {}", e));
            ptr::null_mut()
        }
    }
}

/// Extract a specific field from config as JSON string.
///
/// Supports dot notation for nested fields (e.g., "ocr.backend").
///
/// # Arguments
///
/// * `config` - Reference to an ExtractionConfig
/// * `field_path` - Dot-separated field path
///
/// # Returns
///
/// JSON string representation of the field value, or None if not found.
pub fn get_field_as_json(config: &ExtractionConfig, field_path: &str) -> Option<String> {
    let json_value = match serde_json::to_value(config) {
        Ok(val) => val,
        Err(e) => {
            set_last_error(format!("Failed to serialize config: {}", e));
            return None;
        }
    };

    let mut current = &json_value;
    for part in field_path.split('.') {
        if let Some(obj) = current.as_object() {
            match obj.get(part) {
                Some(val) => current = val,
                None => {
                    set_last_error(format!("Field '{}' not found in config", field_path));
                    return None;
                }
            }
        } else {
            set_last_error(format!("Cannot access nested field '{}' in non-object", part));
            return None;
        }
    }

    match serde_json::to_string(current) {
        Ok(json) => Some(json),
        Err(e) => {
            set_last_error(format!("Failed to serialize field value: {}", e));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_to_json_string() {
        let config = ExtractionConfig {
            use_cache: true,
            ..Default::default()
        };
        let json = config_to_json_string(&config);
        assert!(json.is_some());
        assert!(json.unwrap().contains("use_cache"));
    }

    #[test]
    fn test_get_field_as_json() {
        let config = ExtractionConfig {
            use_cache: true,
            ..Default::default()
        };
        let result = get_field_as_json(&config, "use_cache");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "true");
    }
}
