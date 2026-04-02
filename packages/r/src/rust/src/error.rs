//! Error mapping from kreuzberg errors to R errors
//!
//! Provides two functions:
//! - `kreuzberg_error`: Maps KreuzbergError variants to typed R errors with [ErrorType] prefixes
//! - `to_r_error`: Generic error mapping for non-kreuzberg errors (serde, tokio, etc.)

use kreuzberg::KreuzbergError;

/// Convert a KreuzbergError to an extendr error with a typed prefix.
///
/// The prefix format `[ErrorType]` is parsed by R-side `check_native_result()`
/// to create typed R conditions (e.g., `ValidationError`, `ParsingError`).
pub fn kreuzberg_error(err: KreuzbergError) -> extendr_api::Error {
    match err {
        KreuzbergError::Validation { message, .. } => {
            extendr_api::Error::Other(format!("[ValidationError] {}", message))
        }
        KreuzbergError::Parsing { message, .. } => {
            extendr_api::Error::Other(format!("[ParsingError] {}", message))
        }
        KreuzbergError::Ocr { message, .. } => {
            extendr_api::Error::Other(format!("[OCRError] {}", message))
        }
        KreuzbergError::Io(e) => {
            extendr_api::Error::Other(format!("[IOError] {}", e))
        }
        KreuzbergError::MissingDependency(msg) => {
            extendr_api::Error::Other(format!("[MissingDependencyError] {}", msg))
        }
        KreuzbergError::UnsupportedFormat(msg) => {
            extendr_api::Error::Other(format!("[UnsupportedFormatError] {}", msg))
        }
        KreuzbergError::Plugin { message, plugin_name } => {
            extendr_api::Error::Other(format!("[PluginError] {}: {}", plugin_name, message))
        }
        KreuzbergError::ImageProcessing { message, .. } => {
            extendr_api::Error::Other(format!("[ImageProcessingError] {}", message))
        }
        KreuzbergError::Serialization { message, .. } => {
            extendr_api::Error::Other(format!("[SerializationError] {}", message))
        }
        KreuzbergError::Embedding { message, .. } => {
            extendr_api::Error::Other(format!("[EmbeddingError] {}", message))
        }
        other => extendr_api::Error::Other(other.to_string()),
    }
}

/// Convert a generic error to an extendr error string.
///
/// Use this for non-kreuzberg errors (serde_json, tokio, etc.)
/// where typed error classes are not needed.
pub fn to_r_error<E: std::fmt::Display>(err: E) -> extendr_api::Error {
    extendr_api::Error::Other(err.to_string())
}
