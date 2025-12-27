//! PyO3 wrappers for FFI error classification functions (Phase 2).
//!
//! Exposes error details retrieval and error message classification from
//! the kreuzberg-ffi crate through Python-friendly interfaces.
//!
//! Functions:
//! - get_error_details() -> dict with error information
//! - classify_error(message: str) -> int (error code)
//! - error_code_name(code: int) -> str

#![allow(unsafe_code)]

use pyo3::prelude::*;
use std::ffi::{CStr, CString};

use kreuzberg_ffi::{kreuzberg_classify_error, kreuzberg_error_code_name, kreuzberg_get_error_details};

/// Error details from kreuzberg-ffi.
///
/// Retrieves detailed error information from the thread-local FFI error storage.
/// Returns a dictionary with the following keys:
/// - "message" (str): Human-readable error message
/// - "error_code" (int): Numeric error code (0-7)
/// - "error_type" (str): Error type name (e.g., "validation", "ocr")
/// - "source_file" (str | None): Source file path if available
/// - "source_function" (str | None): Function name if available
/// - "source_line" (int): Line number (0 if unknown)
/// - "context_info" (str | None): Additional context if available
/// - "is_panic" (bool): Whether error came from a panic
///
/// Returns:
///     dict: Structured error details
#[pyfunction]
pub fn get_error_details(py: Python<'_>) -> PyResult<pyo3::Bound<'_, pyo3::types::PyDict>> {
    let details = kreuzberg_get_error_details();

    let result = pyo3::types::PyDict::new(py);

    unsafe {
        let message = if !details.message.is_null() {
            CStr::from_ptr(details.message).to_string_lossy().into_owned()
        } else {
            String::new()
        };

        let error_type = if !details.error_type.is_null() {
            CStr::from_ptr(details.error_type).to_string_lossy().into_owned()
        } else {
            "unknown".to_string()
        };

        let source_file = if !details.source_file.is_null() {
            Some(CStr::from_ptr(details.source_file).to_string_lossy().into_owned())
        } else {
            None
        };

        let source_function = if !details.source_function.is_null() {
            Some(CStr::from_ptr(details.source_function).to_string_lossy().into_owned())
        } else {
            None
        };

        let context_info = if !details.context_info.is_null() {
            Some(CStr::from_ptr(details.context_info).to_string_lossy().into_owned())
        } else {
            None
        };

        result.set_item("message", message)?;
        result.set_item("error_code", details.error_code)?;
        result.set_item("error_type", error_type)?;
        result.set_item("source_file", source_file)?;
        result.set_item("source_function", source_function)?;
        result.set_item("source_line", details.source_line)?;
        result.set_item("context_info", context_info)?;
        result.set_item("is_panic", details.is_panic != 0)?;

        Ok(result)
    }
}

/// Classify an error based on an error message string.
///
/// Analyzes the error message and returns the most likely Kreuzberg error code.
///
/// Args:
///     message (str): The error message to classify
///
/// Returns:
///     int: Error code (0-7) representing the classification
///
/// Classification:
/// - 0 (Validation): Invalid parameters, constraints, format mismatches
/// - 1 (Parsing): Parse errors, corrupt data, malformed content
/// - 2 (OCR): OCR processing failures
/// - 3 (MissingDependency): Missing libraries or system dependencies
/// - 4 (Io): File I/O, permissions, disk errors
/// - 5 (Plugin): Plugin loading or registry errors
/// - 6 (UnsupportedFormat): Unsupported MIME types or formats
/// - 7 (Internal): Unknown or internal errors
#[pyfunction]
pub fn classify_error(message: &str) -> PyResult<u32> {
    let c_message =
        CString::new(message).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    let code = unsafe { kreuzberg_classify_error(c_message.as_ptr()) };

    Ok(code)
}

/// Get the human-readable name of an error code.
///
/// Args:
///     code (int): Numeric error code (0-7)
///
/// Returns:
///     str: Human-readable error code name (e.g., "validation", "ocr")
///
/// Returns "unknown" for codes outside the valid range.
#[pyfunction]
pub fn error_code_name(code: u32) -> PyResult<String> {
    let name_ptr = kreuzberg_error_code_name(code);

    if name_ptr.is_null() {
        return Ok("unknown".to_string());
    }

    let name = unsafe { CStr::from_ptr(name_ptr).to_string_lossy().into_owned() };

    Ok(name)
}

/// Get the last error code from the FFI layer.
///
/// Returns 0 (Success) - actual error codes are only available through
/// the C FFI library and are thread-local in the FFI layer.
///
/// This function exists for API completeness and future extension
/// when FFI layer integration is available.
pub fn get_last_error_code() -> i32 {
    // TODO: Link to kreuzberg-ffi when available in py bindings
    0
}

/// Stub panic context function.
///
/// Returns None since panic context is not available in the Rust bindings.
/// Panic context is only available through the C FFI library.
///
/// This function exists for API completeness and future extension.
pub fn get_last_panic_context() -> Option<String> {
    // TODO: Link to kreuzberg-ffi when available in py bindings
    None
}
