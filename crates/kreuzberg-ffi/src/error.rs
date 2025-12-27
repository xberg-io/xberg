//! Centralized error codes for Kreuzberg FFI bindings.
//!
//! This module defines the authoritative error codes used across all language bindings
//! (Python, Ruby, Go, Java, TypeScript, C#). All bindings should reference these codes
//! rather than maintaining separate definitions.
//!
//! # Error Code Mapping
//!
//! Each variant maps to a specific error type encountered during document extraction:
//!
//! - **Validation (0)**: Input validation errors (invalid config, parameters)
//! - **Parsing (1)**: Document format errors (corrupt files, unsupported features)
//! - **Ocr (2)**: OCR processing failures (backend errors, image issues)
//! - **MissingDependency (3)**: Required system dependencies not found (tesseract, pandoc)
//! - **Io (4)**: File system and I/O errors (permissions, disk full)
//! - **Plugin (5)**: Plugin registration/execution errors
//! - **UnsupportedFormat (6)**: Unsupported MIME type or file format
//! - **Internal (7)**: Internal library errors (should rarely occur)
//!
//! # Usage in Bindings
//!
//! **Python** (kreuzberg/exceptions.py):
//! ```python
//! class ErrorCode(IntEnum):
//!     VALIDATION = 0
//!     PARSING = 1
//!     OCR = 2
//!     MISSING_DEPENDENCY = 3
//!     IO = 4
//!     PLUGIN = 5
//!     UNSUPPORTED_FORMAT = 6
//!     INTERNAL = 7
//! ```
//!
//! **Ruby** (packages/ruby/lib/kreuzberg.rb):
//! ```ruby
//! module Kreuzberg
//!   class ErrorCode
//!     VALIDATION = 0
//!     PARSING = 1
//!     # ... etc
//!   end
//! end
//! ```
//!
//! **Go** (packages/go/v4/errors.go):
//! ```go
//! const (
//!     ValidationError int32 = 0
//!     ParsingError int32 = 1
//!     // ... etc
//! )
//! ```
//!
//! # FFI Exports
//!
//! This module exports FFI-safe functions for binding libraries to query error codes:
//!
//! - `kreuzberg_error_code_validation()` -> 0
//! - `kreuzberg_error_code_parsing()` -> 1
//! - `kreuzberg_error_code_ocr()` -> 2
//! - `kreuzberg_error_code_missing_dependency()` -> 3
//! - `kreuzberg_error_code_io()` -> 4
//! - `kreuzberg_error_code_plugin()` -> 5
//! - `kreuzberg_error_code_unsupported_format()` -> 6
//! - `kreuzberg_error_code_internal()` -> 7
//! - `kreuzberg_error_code_count()` -> 8
//! - `kreuzberg_error_code_name(code: u32)` -> *const c_char (error name)
//!
//! # Thread Safety
//!
//! All functions are thread-safe and have no runtime overhead (compile-time constants).

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

#[cfg(test)]
use std::ffi::CStr;

/// Centralized error codes for all Kreuzberg bindings.
///
/// These codes are the single source of truth for error classification across
/// all language bindings. Do not introduce new error codes without updating
/// this enum and regenerating bindings.
///
/// # Repr and Stability
///
/// - Uses `#[repr(u32)]` for C ABI compatibility
/// - Error codes are guaranteed stable (0-7, never changing)
/// - Can be safely cast to `int32_t` in C/C++ code
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Input validation error (invalid config, parameters, paths)
    Validation = 0,
    /// Document parsing error (corrupt files, unsupported format features)
    Parsing = 1,
    /// OCR processing error (backend failures, image quality issues)
    Ocr = 2,
    /// Missing system dependency (tesseract not found, pandoc not installed)
    MissingDependency = 3,
    /// File system I/O error (permissions, disk full, file not found)
    Io = 4,
    /// Plugin registration or execution error
    Plugin = 5,
    /// Unsupported MIME type or file format
    UnsupportedFormat = 6,
    /// Internal library error (indicates a bug, should rarely occur)
    Internal = 7,
}

impl ErrorCode {
    /// Returns the human-readable name for this error code.
    ///
    /// Used for debugging and logging. Names match the enum variant names
    /// in lowercase.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(ErrorCode::Validation.name(), "validation");
    /// assert_eq!(ErrorCode::Ocr.name(), "ocr");
    /// ```
    #[inline]
    pub fn name(self) -> &'static str {
        match self {
            ErrorCode::Validation => "validation",
            ErrorCode::Parsing => "parsing",
            ErrorCode::Ocr => "ocr",
            ErrorCode::MissingDependency => "missing_dependency",
            ErrorCode::Io => "io",
            ErrorCode::Plugin => "plugin",
            ErrorCode::UnsupportedFormat => "unsupported_format",
            ErrorCode::Internal => "internal",
        }
    }

    /// Returns a brief description of the error code.
    ///
    /// Used for user-facing error messages and documentation.
    #[inline]
    pub fn description(self) -> &'static str {
        match self {
            ErrorCode::Validation => "Input validation error",
            ErrorCode::Parsing => "Document parsing error",
            ErrorCode::Ocr => "OCR processing error",
            ErrorCode::MissingDependency => "Missing system dependency",
            ErrorCode::Io => "File system I/O error",
            ErrorCode::Plugin => "Plugin error",
            ErrorCode::UnsupportedFormat => "Unsupported format",
            ErrorCode::Internal => "Internal library error",
        }
    }

    /// Converts from numeric error code to enum variant.
    ///
    /// Returns `None` if the code is outside the valid range [0, 7].
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(ErrorCode::from_code(0), Some(ErrorCode::Validation));
    /// assert_eq!(ErrorCode::from_code(2), Some(ErrorCode::Ocr));
    /// assert_eq!(ErrorCode::from_code(99), None);
    /// ```
    #[inline]
    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(ErrorCode::Validation),
            1 => Some(ErrorCode::Parsing),
            2 => Some(ErrorCode::Ocr),
            3 => Some(ErrorCode::MissingDependency),
            4 => Some(ErrorCode::Io),
            5 => Some(ErrorCode::Plugin),
            6 => Some(ErrorCode::UnsupportedFormat),
            7 => Some(ErrorCode::Internal),
            _ => None,
        }
    }

    /// Checks if a numeric code is valid (within [0, 7]).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert!(ErrorCode::is_valid(0));
    /// assert!(ErrorCode::is_valid(7));
    /// assert!(!ErrorCode::is_valid(8));
    /// ```
    #[inline]
    pub fn is_valid(code: u32) -> bool {
        code <= 7
    }
}

/// Returns the validation error code (0).
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_error_code_validation(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_validation() -> u32 {
    ErrorCode::Validation as u32
}

/// Returns the parsing error code (1).
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_error_code_parsing(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_parsing() -> u32 {
    ErrorCode::Parsing as u32
}

/// Returns the OCR error code (2).
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_error_code_ocr(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_ocr() -> u32 {
    ErrorCode::Ocr as u32
}

/// Returns the missing dependency error code (3).
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_error_code_missing_dependency(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_missing_dependency() -> u32 {
    ErrorCode::MissingDependency as u32
}

/// Returns the I/O error code (4).
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_error_code_io(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_io() -> u32 {
    ErrorCode::Io as u32
}

/// Returns the plugin error code (5).
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_error_code_plugin(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_plugin() -> u32 {
    ErrorCode::Plugin as u32
}

/// Returns the unsupported format error code (6).
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_error_code_unsupported_format(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_unsupported_format() -> u32 {
    ErrorCode::UnsupportedFormat as u32
}

/// Returns the internal error code (7).
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_error_code_internal(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_internal() -> u32 {
    ErrorCode::Internal as u32
}

/// Returns the total count of valid error codes.
///
/// Currently 8 error codes (0-7). This helps bindings validate error codes.
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_error_code_count(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_count() -> u32 {
    8
}

/// Returns the name of an error code as a C string.
///
/// # Arguments
///
/// - `code`: Numeric error code (0-7)
///
/// # Returns
///
/// Pointer to a null-terminated C string with the error name (e.g., "validation", "ocr").
/// Returns a pointer to "unknown" if the code is invalid.
///
/// The returned pointer is valid for the lifetime of the program and should not be freed.
///
/// # Examples
///
/// ```c
/// const char* name = kreuzberg_error_code_name(0);
/// printf("%s\n", name);  // prints: validation
/// ```
///
/// # C Signature
///
/// ```c
/// const char* kreuzberg_error_code_name(uint32_t code);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_name(code: u32) -> *const c_char {
    match ErrorCode::from_code(code) {
        Some(err_code) => match err_code {
            ErrorCode::Validation => c"validation".as_ptr(),
            ErrorCode::Parsing => c"parsing".as_ptr(),
            ErrorCode::Ocr => c"ocr".as_ptr(),
            ErrorCode::MissingDependency => c"missing_dependency".as_ptr(),
            ErrorCode::Io => c"io".as_ptr(),
            ErrorCode::Plugin => c"plugin".as_ptr(),
            ErrorCode::UnsupportedFormat => c"unsupported_format".as_ptr(),
            ErrorCode::Internal => c"internal".as_ptr(),
        },
        None => c"unknown".as_ptr(),
    }
}

/// Returns the description of an error code as a C string.
///
/// # Arguments
///
/// - `code`: Numeric error code (0-7)
///
/// # Returns
///
/// Pointer to a null-terminated C string with a description (e.g., "Input validation error").
/// Returns a pointer to "Unknown error code" if the code is invalid.
///
/// The returned pointer is valid for the lifetime of the program and should not be freed.
///
/// # C Signature
///
/// ```c
/// const char* kreuzberg_error_code_description(uint32_t code);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_error_code_description(code: u32) -> *const c_char {
    match ErrorCode::from_code(code) {
        Some(err_code) => match err_code {
            ErrorCode::Validation => c"Input validation error".as_ptr(),
            ErrorCode::Parsing => c"Document parsing error".as_ptr(),
            ErrorCode::Ocr => c"OCR processing error".as_ptr(),
            ErrorCode::MissingDependency => c"Missing system dependency".as_ptr(),
            ErrorCode::Io => c"File system I/O error".as_ptr(),
            ErrorCode::Plugin => c"Plugin error".as_ptr(),
            ErrorCode::UnsupportedFormat => c"Unsupported format".as_ptr(),
            ErrorCode::Internal => c"Internal library error".as_ptr(),
        },
        None => c"Unknown error code".as_ptr(),
    }
}

/// C-compatible structured error details returned by `kreuzberg_get_error_details()`.
///
/// All string fields (message, error_type, source_file, source_function, context_info)
/// are dynamically allocated C strings that MUST be freed using `kreuzberg_free_string()`.
/// Set fields are non-NULL; unset fields are NULL.
#[repr(C)]
pub struct CErrorDetails {
    /// The error message (must be freed with kreuzberg_free_string)
    pub message: *mut c_char,
    /// Numeric error code (0-7 for Kreuzberg errors, 1-7 for panic_shield codes)
    pub error_code: u32,
    /// Human-readable error type name (must be freed with kreuzberg_free_string)
    pub error_type: *mut c_char,
    /// Source file where error occurred (may be NULL)
    pub source_file: *mut c_char,
    /// Source function where error occurred (may be NULL)
    pub source_function: *mut c_char,
    /// Line number in source file (0 if unknown)
    pub source_line: u32,
    /// Additional context information (may be NULL)
    pub context_info: *mut c_char,
    /// 1 if this error originated from a panic, 0 otherwise
    pub is_panic: i32,
}

/// Retrieves detailed error information from the thread-local error storage.
///
/// Returns structured error details including message, code, type, and source location.
/// This function queries the error state captured by FFI functions and provides
/// comprehensive error information for binding implementations.
///
/// # Returns
///
/// A `CErrorDetails` structure with the following characteristics:
/// - All non-NULL string pointers must be freed with `kreuzberg_free_string()`
/// - NULL pointers indicate the field is not available
/// - `error_code` is a numeric code (0-7)
/// - `source_line` is 0 if unknown
/// - `is_panic` is 1 if error originated from a panic, 0 otherwise
///
/// # Thread Safety
///
/// This function is thread-safe. Each thread has its own error storage.
///
/// # Example (C)
///
/// ```c
/// CErrorDetails details = kreuzberg_get_error_details();
/// printf("Error: %s (code=%u, type=%s)\n", details.message, details.error_code, details.error_type);
/// if (details.source_file != NULL) {
///     printf("  at %s:%u in %s\n", details.source_file, details.source_line, details.source_function);
/// }
/// kreuzberg_free_string(details.message);
/// kreuzberg_free_string(details.error_type);
/// if (details.source_file != NULL) kreuzberg_free_string(details.source_file);
/// if (details.source_function != NULL) kreuzberg_free_string(details.source_function);
/// if (details.context_info != NULL) kreuzberg_free_string(details.context_info);
/// ```
///
/// # C Signature
///
/// ```c
/// typedef struct {
///     char* message;
///     uint32_t error_code;
///     char* error_type;
///     char* source_file;
///     char* source_function;
///     uint32_t source_line;
///     char* context_info;
///     int is_panic;
/// } CErrorDetails;
///
/// CErrorDetails kreuzberg_get_error_details(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_get_error_details() -> CErrorDetails {
    use crate::panic_shield;

    let message = panic_shield::get_last_error_message().unwrap_or_else(|| "No error".to_string());
    let error_code_enum = panic_shield::get_last_error_code();
    let error_code = error_code_enum as u32;
    let is_panic = if error_code_enum == panic_shield::ErrorCode::Panic {
        1
    } else {
        0
    };

    let error_type = match error_code_enum {
        panic_shield::ErrorCode::Success => "success".to_string(),
        panic_shield::ErrorCode::GenericError => "generic_error".to_string(),
        panic_shield::ErrorCode::Panic => "panic".to_string(),
        panic_shield::ErrorCode::InvalidArgument => "invalid_argument".to_string(),
        panic_shield::ErrorCode::IoError => "io_error".to_string(),
        panic_shield::ErrorCode::ParsingError => "parsing_error".to_string(),
        panic_shield::ErrorCode::OcrError => "ocr_error".to_string(),
        panic_shield::ErrorCode::MissingDependency => "missing_dependency".to_string(),
    };

    let (source_file, source_function, source_line) = if let Some(ctx) = panic_shield::get_last_panic_context() {
        (Some(ctx.file), Some(ctx.function), ctx.line)
    } else {
        (None, None, 0)
    };

    CErrorDetails {
        message: CString::new(message)
            .map(CString::into_raw)
            .unwrap_or_else(|_| "Error message creation failed".as_ptr() as *mut c_char),
        error_code,
        error_type: CString::new(error_type)
            .map(CString::into_raw)
            .unwrap_or_else(|_| "unknown".as_ptr() as *mut c_char),
        source_file: source_file
            .and_then(|f| CString::new(f).ok())
            .map(CString::into_raw)
            .unwrap_or(ptr::null_mut()),
        source_function: source_function
            .and_then(|f| CString::new(f).ok())
            .map(CString::into_raw)
            .unwrap_or(ptr::null_mut()),
        source_line,
        context_info: ptr::null_mut(),
        is_panic,
    }
}

/// Classifies an error based on the error message string.
///
/// Analyzes an error message and attempts to classify it into one of the standard
/// Kreuzberg error codes (0-7). This is useful for converting error messages from
/// external libraries or system calls into Kreuzberg error categories.
///
/// # Arguments
///
/// - `error_message`: Pointer to a null-terminated C string with the error message
///
/// # Returns
///
/// Numeric error code (0-7) indicating the most likely error classification.
/// Returns 7 (Internal) if the message cannot be reliably classified.
///
/// # Classification Rules
///
/// The classifier looks for common keywords and patterns:
/// - **0 (Validation)**: "invalid", "validation", "parameter", "constraint", "format mismatch"
/// - **1 (Parsing)**: "parse", "parsing", "corrupt", "unexpected", "malformed", "invalid format"
/// - **2 (OCR)**: "ocr", "tesseract", "recognition", "optical"
/// - **3 (MissingDependency)**: "not found", "missing", "dependency", "not installed", "unavailable"
/// - **4 (Io)**: "io", "file", "read", "write", "permission", "access", "disk", "exists"
/// - **5 (Plugin)**: "plugin", "loader", "registry", "extension"
/// - **6 (UnsupportedFormat)**: "unsupported", "unknown format", "MIME type"
///
/// # Thread Safety
///
/// This function is thread-safe and has no side effects.
///
/// # Example (C)
///
/// ```c
/// uint32_t code = kreuzberg_classify_error("Failed to open file: permission denied");
/// if (code == kreuzberg_error_code_io()) {
///     printf("This is an I/O error\n");
/// }
/// ```
///
/// # Safety
///
/// - `error_message` must be a valid null-terminated C string or NULL
/// - `error_message` must remain valid for the duration of the function call
///
/// # C Signature
///
/// ```c
/// uint32_t kreuzberg_classify_error(const char* error_message);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_classify_error(error_message: *const c_char) -> u32 {
    if error_message.is_null() {
        return ErrorCode::Internal as u32;
    }

    let message_str = match unsafe { std::ffi::CStr::from_ptr(error_message) }.to_str() {
        Ok(s) => s,
        Err(_) => return ErrorCode::Internal as u32,
    };

    let lower = message_str.to_lowercase();

    if lower.contains("not found")
        || lower.contains("missing")
        || lower.contains("dependency")
        || lower.contains("not installed")
        || lower.contains("unavailable")
    {
        return ErrorCode::MissingDependency as u32;
    }

    if lower.contains("invalid")
        || lower.contains("validation")
        || lower.contains("parameter")
        || lower.contains("constraint")
        || lower.contains("format mismatch")
    {
        return ErrorCode::Validation as u32;
    }

    if lower.contains("parse")
        || lower.contains("parsing")
        || lower.contains("corrupt")
        || lower.contains("unexpected")
        || lower.contains("malformed")
    {
        return ErrorCode::Parsing as u32;
    }

    if lower.contains("ocr")
        || lower.contains("tesseract")
        || lower.contains("recognition")
        || lower.contains("optical")
    {
        return ErrorCode::Ocr as u32;
    }

    if lower.contains("io")
        || lower.contains("file")
        || lower.contains("read")
        || lower.contains("write")
        || lower.contains("permission")
        || lower.contains("access")
        || lower.contains("disk")
        || lower.contains("exists")
    {
        return ErrorCode::Io as u32;
    }

    if lower.contains("plugin") || lower.contains("loader") || lower.contains("registry") || lower.contains("extension")
    {
        return ErrorCode::Plugin as u32;
    }

    if lower.contains("unsupported") || lower.contains("unknown format") || lower.contains("mime type") {
        return ErrorCode::UnsupportedFormat as u32;
    }

    ErrorCode::Internal as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_values() {
        assert_eq!(ErrorCode::Validation as u32, 0);
        assert_eq!(ErrorCode::Parsing as u32, 1);
        assert_eq!(ErrorCode::Ocr as u32, 2);
        assert_eq!(ErrorCode::MissingDependency as u32, 3);
        assert_eq!(ErrorCode::Io as u32, 4);
        assert_eq!(ErrorCode::Plugin as u32, 5);
        assert_eq!(ErrorCode::UnsupportedFormat as u32, 6);
        assert_eq!(ErrorCode::Internal as u32, 7);
    }

    #[test]
    fn test_error_code_names() {
        assert_eq!(ErrorCode::Validation.name(), "validation");
        assert_eq!(ErrorCode::Parsing.name(), "parsing");
        assert_eq!(ErrorCode::Ocr.name(), "ocr");
        assert_eq!(ErrorCode::MissingDependency.name(), "missing_dependency");
        assert_eq!(ErrorCode::Io.name(), "io");
        assert_eq!(ErrorCode::Plugin.name(), "plugin");
        assert_eq!(ErrorCode::UnsupportedFormat.name(), "unsupported_format");
        assert_eq!(ErrorCode::Internal.name(), "internal");
    }

    #[test]
    fn test_error_code_descriptions() {
        assert_eq!(ErrorCode::Validation.description(), "Input validation error");
        assert_eq!(ErrorCode::Parsing.description(), "Document parsing error");
        assert_eq!(ErrorCode::Ocr.description(), "OCR processing error");
        assert_eq!(ErrorCode::MissingDependency.description(), "Missing system dependency");
        assert_eq!(ErrorCode::Io.description(), "File system I/O error");
        assert_eq!(ErrorCode::Plugin.description(), "Plugin error");
        assert_eq!(ErrorCode::UnsupportedFormat.description(), "Unsupported format");
        assert_eq!(ErrorCode::Internal.description(), "Internal library error");
    }

    #[test]
    fn test_from_code_valid() {
        assert_eq!(ErrorCode::from_code(0), Some(ErrorCode::Validation));
        assert_eq!(ErrorCode::from_code(1), Some(ErrorCode::Parsing));
        assert_eq!(ErrorCode::from_code(2), Some(ErrorCode::Ocr));
        assert_eq!(ErrorCode::from_code(3), Some(ErrorCode::MissingDependency));
        assert_eq!(ErrorCode::from_code(4), Some(ErrorCode::Io));
        assert_eq!(ErrorCode::from_code(5), Some(ErrorCode::Plugin));
        assert_eq!(ErrorCode::from_code(6), Some(ErrorCode::UnsupportedFormat));
        assert_eq!(ErrorCode::from_code(7), Some(ErrorCode::Internal));
    }

    #[test]
    fn test_from_code_invalid() {
        assert_eq!(ErrorCode::from_code(8), None);
        assert_eq!(ErrorCode::from_code(99), None);
        assert_eq!(ErrorCode::from_code(u32::MAX), None);
    }

    #[test]
    fn test_is_valid() {
        for code in 0..=7 {
            assert!(ErrorCode::is_valid(code), "Code {} should be valid", code);
        }

        assert!(!ErrorCode::is_valid(8));
        assert!(!ErrorCode::is_valid(99));
        assert!(!ErrorCode::is_valid(u32::MAX));
    }

    #[test]
    fn test_error_code_count() {
        assert_eq!(kreuzberg_error_code_count(), 8);
    }

    #[test]
    fn test_ffi_error_code_functions() {
        assert_eq!(kreuzberg_error_code_validation(), 0);
        assert_eq!(kreuzberg_error_code_parsing(), 1);
        assert_eq!(kreuzberg_error_code_ocr(), 2);
        assert_eq!(kreuzberg_error_code_missing_dependency(), 3);
        assert_eq!(kreuzberg_error_code_io(), 4);
        assert_eq!(kreuzberg_error_code_plugin(), 5);
        assert_eq!(kreuzberg_error_code_unsupported_format(), 6);
        assert_eq!(kreuzberg_error_code_internal(), 7);
    }

    // #[test]
    // #[test]

    #[test]
    fn test_error_code_round_trip() {
        for code in 0u32..=7 {
            let err = ErrorCode::from_code(code).unwrap();
            assert_eq!(err as u32, code);

            assert!(!err.name().is_empty());
            assert!(!err.description().is_empty());
        }
    }

    #[test]
    fn test_error_code_copy_clone() {
        let err = ErrorCode::Validation;
        let err_copy = err;
        let err_clone = err.clone();

        assert_eq!(err, err_copy);
        assert_eq!(err, err_clone);
    }

    #[test]
    fn test_error_code_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ErrorCode::Validation);
        set.insert(ErrorCode::Parsing);
        set.insert(ErrorCode::Validation);

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ErrorCode::Validation));
        assert!(set.contains(&ErrorCode::Parsing));
    }

    #[test]
    fn test_error_code_debug() {
        let err = ErrorCode::Ocr;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Ocr"));
    }

    #[test]
    fn test_get_error_details_no_error() {
        crate::panic_shield::clear_structured_error();

        let details = kreuzberg_get_error_details();

        let msg = unsafe { CStr::from_ptr(details.message).to_str().unwrap() };
        assert_eq!(msg, "No error");

        assert_eq!(details.error_code, 0);

        unsafe {
            let _ = CString::from_raw(details.message);
            let _ = CString::from_raw(details.error_type);
        }
    }

    #[test]
    fn test_get_error_details_with_error() {
        crate::panic_shield::clear_structured_error();
        let err = crate::panic_shield::StructuredError::from_message(
            "Test error message".to_string(),
            crate::panic_shield::ErrorCode::IoError,
        );
        crate::panic_shield::set_structured_error(err);

        let details = kreuzberg_get_error_details();

        let msg = unsafe { CStr::from_ptr(details.message).to_str().unwrap() };
        assert_eq!(msg, "Test error message");

        let error_type = unsafe { CStr::from_ptr(details.error_type).to_str().unwrap() };
        assert_eq!(error_type, "io_error");

        assert_eq!(details.error_code, crate::panic_shield::ErrorCode::IoError as u32);
        assert_eq!(details.is_panic, 0);

        unsafe {
            let _ = CString::from_raw(details.message);
            let _ = CString::from_raw(details.error_type);
        }
    }

    #[test]
    fn test_classify_error_validation() {
        let msg = CString::new("Invalid parameter").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 0);

        let msg = CString::new("Validation error occurred").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 0);

        let msg = CString::new("Constraint violation").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 0);
    }

    #[test]
    fn test_classify_error_parsing() {
        let msg = CString::new("Parse error in file").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 1);

        let msg = CString::new("Corrupt data detected").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 1);

        let msg = CString::new("Malformed JSON").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 1);
    }

    #[test]
    fn test_classify_error_ocr() {
        let msg = CString::new("OCR processing failed").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 2);

        let msg = CString::new("Tesseract error").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 2);
    }

    #[test]
    fn test_classify_error_missing_dependency() {
        let msg = CString::new("Library not found").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 3);

        let msg = CString::new("Missing dependency: tesseract").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 3);

        let msg = CString::new("Not installed").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 3);
    }

    #[test]
    fn test_classify_error_io() {
        let msg = CString::new("IO error reading file").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 4);

        let msg = CString::new("Permission denied").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 4);

        let msg = CString::new("Disk full").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 4);
    }

    #[test]
    fn test_classify_error_plugin() {
        let msg = CString::new("Plugin loading failed").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 5);

        let msg = CString::new("Registry error").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 5);
    }

    #[test]
    fn test_classify_error_unsupported_format() {
        let msg = CString::new("Unsupported format").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 6);

        let msg = CString::new("Unknown MIME type").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 6);
    }

    #[test]
    fn test_classify_error_internal() {
        let msg = CString::new("Something weird happened").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 7);
    }

    #[test]
    fn test_classify_error_null() {
        assert_eq!(unsafe { kreuzberg_classify_error(std::ptr::null()) }, 7);
    }

    #[test]
    fn test_classify_error_case_insensitive() {
        let msg = CString::new("INVALID PARAMETER").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 0);

        let msg = CString::new("Parse ERROR").unwrap();
        assert_eq!(unsafe { kreuzberg_classify_error(msg.as_ptr()) }, 1);
    }
}
