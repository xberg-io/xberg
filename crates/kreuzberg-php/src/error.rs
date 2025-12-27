//! Error conversion from Rust to PHP exceptions
//!
//! Converts `KreuzbergError` from the Rust core into appropriate PHP exceptions.

use ext_php_rs::prelude::*;

/// Convert Rust KreuzbergError to PHP exception.
///
/// Maps all error variants to PHP's standard Exception with descriptive messages
/// that include the error type prefix for categorization.
pub fn to_php_exception(error: kreuzberg::KreuzbergError) -> PhpException {
    use kreuzberg::KreuzbergError;

    let message = format_error_message(&error);

    match error {
        KreuzbergError::Validation { .. } => PhpException::default(format!("[Validation] {}", message)),
        KreuzbergError::UnsupportedFormat(_) => PhpException::default(format!("[UnsupportedFormat] {}", message)),
        KreuzbergError::Parsing { .. } => PhpException::default(format!("[Parsing] {}", message)),
        KreuzbergError::Io(_) => PhpException::default(format!("[IO] {}", message)),
        KreuzbergError::Ocr { .. } => PhpException::default(format!("[OCR] {}", message)),
        KreuzbergError::Plugin { .. } => PhpException::default(format!("[Plugin] {}", message)),
        KreuzbergError::LockPoisoned(_) => PhpException::default(format!("[LockPoisoned] {}", message)),
        KreuzbergError::Cache { .. } => PhpException::default(format!("[Cache] {}", message)),
        KreuzbergError::ImageProcessing { .. } => PhpException::default(format!("[ImageProcessing] {}", message)),
        KreuzbergError::Serialization { .. } => PhpException::default(format!("[Serialization] {}", message)),
        KreuzbergError::MissingDependency(_) => PhpException::default(format!("[MissingDependency] {}", message)),
        KreuzbergError::Other(_) => PhpException::default(format!("[Other] {}", message)),
    }
}

/// Format error message with source chain.
fn format_error_message(error: &kreuzberg::KreuzbergError) -> String {
    use kreuzberg::KreuzbergError;

    match error {
        KreuzbergError::Validation { message, source } => {
            if let Some(src) = source {
                format!("{}: {}", message, src)
            } else {
                message.clone()
            }
        }
        KreuzbergError::UnsupportedFormat(msg) => msg.clone(),
        KreuzbergError::Parsing { message, source } => {
            if let Some(src) = source {
                format!("{}: {}", message, src)
            } else {
                message.clone()
            }
        }
        KreuzbergError::Io(e) => e.to_string(),
        KreuzbergError::Ocr { message, source } => {
            if let Some(src) = source {
                format!("{}: {}", message, src)
            } else {
                message.clone()
            }
        }
        KreuzbergError::Plugin { message, plugin_name } => {
            format!("Plugin error in '{}': {}", plugin_name, message)
        }
        KreuzbergError::LockPoisoned(msg) => msg.clone(),
        KreuzbergError::Cache { message, source } => {
            if let Some(src) = source {
                format!("{}: {}", message, src)
            } else {
                message.clone()
            }
        }
        KreuzbergError::ImageProcessing { message, source } => {
            if let Some(src) = source {
                format!("{}: {}", message, src)
            } else {
                message.clone()
            }
        }
        KreuzbergError::Serialization { message, source } => {
            if let Some(src) = source {
                format!("{}: {}", message, src)
            } else {
                message.clone()
            }
        }
        KreuzbergError::MissingDependency(msg) => msg.clone(),
        KreuzbergError::Other(msg) => msg.clone(),
    }
}

/// Error classification result.
///
/// Contains the classified error code, name, description, and confidence.
///
/// # Properties
///
/// - `code` (int): The numeric error code (0-7)
/// - `name` (string): The error code name
/// - `description` (string): Brief description of the error type
/// - `confidence` (float): Confidence score (0.0-1.0) of the classification
#[php_class]
#[derive(Clone)]
pub struct ErrorClassification {
    pub code: i64,
    pub name: String,
    pub description: String,
    pub confidence: f64,
}

#[php_impl]
impl ErrorClassification {}

/// Classify an error message string into an error code category.
///
/// This function analyzes the error message content and returns the most likely
/// error code (0-7) based on keyword patterns.
///
/// # Parameters
///
/// - `error_message` (string): The error message string to classify
///
/// # Returns
///
/// ErrorClassification object with code, name, description, and confidence
///
/// # Error Codes
///
/// - 0: Validation
/// - 1: Parsing
/// - 2: OCR
/// - 3: MissingDependency
/// - 4: IO
/// - 5: Plugin
/// - 6: UnsupportedFormat
/// - 7: Internal
///
/// # Example
///
/// ```php
/// $result = kreuzberg_classify_error("PDF file is corrupted");
/// echo "Error code: {$result->code}\n";  // 1 (Parsing)
/// echo "Name: {$result->name}\n";        // "parsing"
/// echo "Confidence: {$result->confidence}\n"; // 0.85
/// ```
#[php_function]
pub fn kreuzberg_classify_error(error_message: String) -> ErrorClassification {
    let lower = error_message.to_lowercase();

    let (code, confidence) = if lower.contains("validation")
        || lower.contains("invalid_argument")
        || lower.contains("schema")
        || lower.contains("required")
        || lower.contains("unexpected field")
    {
        (0i64, 0.9)
    } else if lower.contains("parsing")
        || lower.contains("parse_error")
        || lower.contains("corrupted")
        || lower.contains("malformed")
        || lower.contains("invalid format")
        || lower.contains("decode")
        || lower.contains("encoding")
    {
        (1i64, 0.85)
    } else if lower.contains("ocr")
        || lower.contains("optical")
        || lower.contains("character")
        || lower.contains("recognition")
        || lower.contains("tesseract")
        || lower.contains("language")
        || lower.contains("model")
    {
        (2i64, 0.88)
    } else if lower.contains("not found")
        || lower.contains("not installed")
        || lower.contains("missing")
        || lower.contains("dependency")
        || lower.contains("require")
        || lower.contains("unavailable")
    {
        (3i64, 0.92)
    } else if lower.contains("io")
        || lower.contains("file")
        || lower.contains("disk")
        || lower.contains("read")
        || lower.contains("write")
        || lower.contains("permission")
        || lower.contains("access")
        || lower.contains("path")
    {
        (4i64, 0.87)
    } else if lower.contains("plugin")
        || lower.contains("register")
        || lower.contains("extension")
        || lower.contains("handler")
        || lower.contains("processor")
    {
        (5i64, 0.84)
    } else if lower.contains("unsupported")
        || lower.contains("format")
        || lower.contains("mime")
        || lower.contains("type")
        || lower.contains("codec")
    {
        (6i64, 0.83)
    } else if lower.contains("internal")
        || lower.contains("bug")
        || lower.contains("panic")
        || lower.contains("unexpected")
        || lower.contains("invariant")
    {
        (7i64, 0.86)
    } else {
        (7i64, 0.1)
    };

    let name = kreuzberg_error_code_name(code as u32);
    let description = kreuzberg_error_code_description(code as u32);

    ErrorClassification {
        code,
        name,
        description,
        confidence,
    }
}

/// Get the human-readable name for an error code.
///
/// # Parameters
///
/// - `code` (int): Numeric error code (0-7)
///
/// # Returns
///
/// String containing the error code name (e.g., "validation", "ocr", "unknown")
///
/// # Example
///
/// ```php
/// $name = kreuzberg_error_code_name(0);  // "validation"
/// $name = kreuzberg_error_code_name(2);  // "ocr"
/// ```
#[php_function]
pub fn kreuzberg_error_code_name(code: u32) -> String {
    match code {
        0 => "validation".to_string(),
        1 => "parsing".to_string(),
        2 => "ocr".to_string(),
        3 => "missing_dependency".to_string(),
        4 => "io".to_string(),
        5 => "plugin".to_string(),
        6 => "unsupported_format".to_string(),
        7 => "internal".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Get the description for an error code.
///
/// # Parameters
///
/// - `code` (int): Numeric error code (0-7)
///
/// # Returns
///
/// String containing a brief description of the error
///
/// # Example
///
/// ```php
/// $desc = kreuzberg_error_code_description(0);  // "Input validation error"
/// $desc = kreuzberg_error_code_description(4);  // "File system I/O error"
/// ```
#[php_function]
pub fn kreuzberg_error_code_description(code: u32) -> String {
    match code {
        0 => "Input validation error".to_string(),
        1 => "Document parsing error".to_string(),
        2 => "OCR processing error".to_string(),
        3 => "Missing dependency error".to_string(),
        4 => "File system I/O error".to_string(),
        5 => "Plugin execution error".to_string(),
        6 => "Unsupported format error".to_string(),
        7 => "Internal error".to_string(),
        _ => "Unknown error code".to_string(),
    }
}
