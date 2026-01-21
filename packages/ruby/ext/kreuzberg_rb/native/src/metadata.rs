//! Metadata handling and document format detection
//!
//! Provides utilities for MIME type detection, format validation, and extension mapping.

use crate::error_handling::runtime_error;
use magnus::Error;

/// Detect MIME type from bytes
pub fn detect_mime_type_from_bytes(bytes: String) -> Result<String, Error> {
    let bytes_vec = bytes.into_bytes();
    kreuzberg::core::mime::detect_mime_type_from_bytes(&bytes_vec)
        .map_err(|e| runtime_error(format!("Failed to detect MIME type: {}", e)))
}

/// Detect MIME type from file path
pub fn detect_mime_type_from_path_native(path: String) -> Result<String, Error> {
    kreuzberg::core::mime::detect_mime_type(&path, true)
        .map_err(|e| runtime_error(format!("Failed to detect MIME type from path: {}", e)))
}

/// Validate MIME type
pub fn validate_mime_type_native(mime_type: String) -> Result<String, Error> {
    if kreuzberg::core::mime::validate_mime_type(&mime_type).is_ok() {
        Ok(mime_type)
    } else {
        Err(runtime_error(format!("Unsupported MIME type: {}", mime_type)))
    }
}

/// Get file extensions for a given MIME type
pub fn get_extensions_for_mime_native(mime_type: String) -> Result<Vec<String>, Error> {
    kreuzberg::core::mime::get_extensions_for_mime(&mime_type)
        .map_err(|e| runtime_error(format!("Failed to get extensions: {}", e)))
}
