use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::ffi::CStr;

use super::kreuzberg_error_code_description;
use super::kreuzberg_error_code_name;

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
/// - `Timeout` → GenericFailure (extraction timeout exceeded)
/// - `Other` → GenericFailure (catch-all)
///
/// # Usage
///
/// ```rust,ignore
/// kreuzberg::extract_file_sync(&path, None, &config)
///     .map_err(convert_error)
///     .and_then(JsExtractionResult::try_from)
/// ```
pub(crate) fn convert_error(err: kreuzberg::KreuzbergError) -> napi::Error {
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

        KreuzbergError::Timeout { elapsed_ms, limit_ms } => Error::new(
            Status::GenericFailure,
            format!("Extraction timed out after {}ms (limit: {}ms)", elapsed_ms, limit_ms),
        ),

        KreuzbergError::Embedding { message, .. } => {
            Error::new(Status::GenericFailure, format!("Embedding error: {}", message))
        }

        KreuzbergError::Cancelled => Error::new(Status::Cancelled, "Extraction cancelled"),

        KreuzbergError::Other(msg) => Error::new(Status::GenericFailure, msg),
    }
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
/// - **Embedding (8)**: Keywords: embed, embedding, vector, inference, model
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

    let (code, confidence) = if lower.contains("not found")
        || lower.contains("not installed")
        || lower.contains("missing")
        || lower.contains("dependency")
        || lower.contains("require")
        || lower.contains("unavailable")
    {
        (3u32, 0.92)
    } else if lower.contains("validation")
        || lower.contains("invalid_argument")
        || lower.contains("invalid")
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
    } else if lower.contains("plugin")
        || lower.contains("register")
        || lower.contains("registration")
        || lower.contains("extension")
        || lower.contains("handler")
        || lower.contains("processor")
    {
        (5u32, 0.84)
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
    } else if lower.contains("embed")
        || lower.contains("embedding")
        || lower.contains("vector")
        || lower.contains("inference")
        || lower.contains("model")
    {
        (8u32, 0.89)
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
