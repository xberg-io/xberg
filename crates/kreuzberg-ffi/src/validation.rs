//! Configuration validation FFI module.
//!
//! Exposes validation functions from `kreuzberg::core::config_validation` through C FFI,
//! allowing all language bindings (Python, Java, TypeScript, Ruby, Go, C#) to validate
//! configuration values without duplicating validation logic.
//!
//! # Return Values
//!
//! All validator functions return:
//! - `1` if the value is valid
//! - `0` if the value is invalid (with error message set via `set_last_error()`)
//!
//! # String Functions
//!
//! List functions return JSON-encoded strings as `*mut c_char` that MUST be freed by the caller
//! using `kreuzberg_free_string()`.
//!
//! # Examples (in C)
//!
//! ```c
//! // Validate a string parameter
//! if (kreuzberg_validate_binarization_method("otsu") == 1) {
//!     printf("Valid!\n");
//! } else {
//!     printf("Error: %s\n", kreuzberg_get_last_error_message());
//! }
//!
//! // Get valid options
//! char* methods = kreuzberg_get_valid_binarization_methods();
//! // Use it...
//! kreuzberg_free_string(methods);
//! ```

use std::ffi::CStr;
use std::os::raw::c_char;

use kreuzberg::core::config_validation::{
    validate_binarization_method, validate_chunking_params, validate_confidence, validate_dpi, validate_language_code,
    validate_ocr_backend, validate_output_format, validate_tesseract_oem, validate_tesseract_psm,
    validate_token_reduction_level,
};

use crate::set_last_error;

const VALID_BINARIZATION_METHODS: &[&str] = &["otsu", "adaptive", "sauvola"];
const VALID_TOKEN_REDUCTION_LEVELS: &[&str] = &["off", "light", "moderate", "aggressive", "maximum"];
const VALID_OCR_BACKENDS: &[&str] = &["tesseract", "easyocr", "paddleocr"];
const VALID_LANGUAGE_CODES: &[&str] = &[
    "en", "de", "fr", "es", "it", "pt", "nl", "pl", "ru", "zh", "ja", "ko", "bg", "cs", "da", "el", "et", "fi", "hu",
    "lt", "lv", "ro", "sk", "sl", "sv", "uk", "ar", "hi", "th", "tr", "vi", "eng", "deu", "fra", "spa", "ita", "por",
    "nld", "pol", "rus", "zho", "jpn", "kor", "ces", "dan", "ell", "est", "fin", "hun", "lit", "lav", "ron", "slk",
    "slv", "swe", "tur",
];

/// Validates a binarization method string.
///
/// # Arguments
///
/// * `method` - C string containing the binarization method (e.g., "otsu", "adaptive", "sauvola")
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # Safety
///
/// * `method` must be a valid pointer to a null-terminated UTF-8 string
/// * `method` cannot be NULL
/// * The string must be valid for the duration of the call
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_binarization_method(const char* method);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_validate_binarization_method(method: *const c_char) -> i32 {
    if method.is_null() {
        set_last_error("method cannot be NULL".to_string());
        return 0;
    }

    let method_str = match unsafe { CStr::from_ptr(method) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_last_error("Invalid UTF-8 in method".to_string());
            return 0;
        }
    };

    match validate_binarization_method(method_str) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Validates an OCR backend string.
///
/// # Arguments
///
/// * `backend` - C string containing the OCR backend (e.g., "tesseract", "easyocr", "paddleocr")
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # Safety
///
/// * `backend` must be a valid pointer to a null-terminated UTF-8 string
/// * `backend` cannot be NULL
/// * The string must be valid for the duration of the call
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_ocr_backend(const char* backend);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_validate_ocr_backend(backend: *const c_char) -> i32 {
    if backend.is_null() {
        set_last_error("backend cannot be NULL".to_string());
        return 0;
    }

    let backend_str = match unsafe { CStr::from_ptr(backend) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_last_error("Invalid UTF-8 in backend".to_string());
            return 0;
        }
    };

    match validate_ocr_backend(backend_str) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Validates a language code (ISO 639-1 or 639-3 format).
///
/// Accepts both 2-letter codes (e.g., "en", "de") and 3-letter codes (e.g., "eng", "deu").
///
/// # Arguments
///
/// * `code` - C string containing the language code
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # Safety
///
/// * `code` must be a valid pointer to a null-terminated UTF-8 string
/// * `code` cannot be NULL
/// * The string must be valid for the duration of the call
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_language_code(const char* code);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_validate_language_code(code: *const c_char) -> i32 {
    if code.is_null() {
        set_last_error("code cannot be NULL".to_string());
        return 0;
    }

    let code_str = match unsafe { CStr::from_ptr(code) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_last_error("Invalid UTF-8 in code".to_string());
            return 0;
        }
    };

    match validate_language_code(code_str) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Validates a token reduction level string.
///
/// # Arguments
///
/// * `level` - C string containing the token reduction level (e.g., "off", "light", "moderate")
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # Safety
///
/// * `level` must be a valid pointer to a null-terminated UTF-8 string
/// * `level` cannot be NULL
/// * The string must be valid for the duration of the call
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_token_reduction_level(const char* level);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_validate_token_reduction_level(level: *const c_char) -> i32 {
    if level.is_null() {
        set_last_error("level cannot be NULL".to_string());
        return 0;
    }

    let level_str = match unsafe { CStr::from_ptr(level) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_last_error("Invalid UTF-8 in level".to_string());
            return 0;
        }
    };

    match validate_token_reduction_level(level_str) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Validates a tesseract Page Segmentation Mode (PSM) value.
///
/// # Arguments
///
/// * `psm` - PSM value (valid range: 0-13)
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_tesseract_psm(int32_t psm);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_validate_tesseract_psm(psm: i32) -> i32 {
    match validate_tesseract_psm(psm) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Validates a tesseract OCR Engine Mode (OEM) value.
///
/// # Arguments
///
/// * `oem` - OEM value (valid range: 0-3)
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_tesseract_oem(int32_t oem);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_validate_tesseract_oem(oem: i32) -> i32 {
    match validate_tesseract_oem(oem) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Validates a tesseract output format string.
///
/// # Arguments
///
/// * `format` - C string containing the output format (e.g., "text", "markdown")
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # Safety
///
/// * `format` must be a valid pointer to a null-terminated UTF-8 string
/// * `format` cannot be NULL
/// * The string must be valid for the duration of the call
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_output_format(const char* format);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_validate_output_format(format: *const c_char) -> i32 {
    if format.is_null() {
        set_last_error("format cannot be NULL".to_string());
        return 0;
    }

    let format_str = match unsafe { CStr::from_ptr(format) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_last_error("Invalid UTF-8 in format".to_string());
            return 0;
        }
    };

    match validate_output_format(format_str) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Validates a confidence threshold value.
///
/// Confidence thresholds must be between 0.0 and 1.0 inclusive.
///
/// # Arguments
///
/// * `confidence` - Confidence threshold value
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_confidence(double confidence);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_validate_confidence(confidence: f64) -> i32 {
    match validate_confidence(confidence) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Validates a DPI (dots per inch) value.
///
/// DPI must be a positive integer, typically 72-600.
///
/// # Arguments
///
/// * `dpi` - DPI value
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_dpi(int32_t dpi);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_validate_dpi(dpi: i32) -> i32 {
    match validate_dpi(dpi) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Validates chunking parameters.
///
/// Checks that `max_chars > 0` and `max_overlap < max_chars`.
///
/// # Arguments
///
/// * `max_chars` - Maximum characters per chunk
/// * `max_overlap` - Maximum overlap between chunks
///
/// # Returns
///
/// - `1` if valid
/// - `0` if invalid (error message available via `kreuzberg_get_last_error_message()`)
///
/// # C Signature
///
/// ```c
/// int32_t kreuzberg_validate_chunking_params(size_t max_chars, size_t max_overlap);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_validate_chunking_params(max_chars: usize, max_overlap: usize) -> i32 {
    match validate_chunking_params(max_chars, max_overlap) {
        Ok(()) => 1,
        Err(e) => {
            set_last_error(e.to_string());
            0
        }
    }
}

/// Returns valid binarization methods as a JSON array string.
///
/// The returned string MUST be freed by the caller using `kreuzberg_free_string()`.
///
/// # Returns
///
/// A pointer to a dynamically allocated C string containing a JSON array of valid methods.
/// Returns NULL if memory allocation fails (error message set via `set_last_error()`).
///
/// # Example
///
/// The returned JSON string looks like: `["otsu","adaptive","sauvola"]`
///
/// # C Signature
///
/// ```c
/// char* kreuzberg_get_valid_binarization_methods(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_get_valid_binarization_methods() -> *mut c_char {
    let json = format!(
        "[{}]",
        VALID_BINARIZATION_METHODS
            .iter()
            .map(|m| format!("\"{}\"", m))
            .collect::<Vec<_>>()
            .join(",")
    );

    match std::ffi::CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(e) => {
            set_last_error(format!("Failed to allocate string: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Returns valid language codes as a JSON array string.
///
/// The returned string MUST be freed by the caller using `kreuzberg_free_string()`.
///
/// # Returns
///
/// A pointer to a dynamically allocated C string containing a JSON array of valid codes.
/// Returns NULL if memory allocation fails (error message set via `set_last_error()`).
///
/// # C Signature
///
/// ```c
/// char* kreuzberg_get_valid_language_codes(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_get_valid_language_codes() -> *mut c_char {
    let json = format!(
        "[{}]",
        VALID_LANGUAGE_CODES
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(",")
    );

    match std::ffi::CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(e) => {
            set_last_error(format!("Failed to allocate string: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Returns valid OCR backends as a JSON array string.
///
/// The returned string MUST be freed by the caller using `kreuzberg_free_string()`.
///
/// # Returns
///
/// A pointer to a dynamically allocated C string containing a JSON array of valid backends.
/// Returns NULL if memory allocation fails (error message set via `set_last_error()`).
///
/// # C Signature
///
/// ```c
/// char* kreuzberg_get_valid_ocr_backends(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_get_valid_ocr_backends() -> *mut c_char {
    let json = format!(
        "[{}]",
        VALID_OCR_BACKENDS
            .iter()
            .map(|b| format!("\"{}\"", b))
            .collect::<Vec<_>>()
            .join(",")
    );

    match std::ffi::CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(e) => {
            set_last_error(format!("Failed to allocate string: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Returns valid token reduction levels as a JSON array string.
///
/// The returned string MUST be freed by the caller using `kreuzberg_free_string()`.
///
/// # Returns
///
/// A pointer to a dynamically allocated C string containing a JSON array of valid levels.
/// Returns NULL if memory allocation fails (error message set via `set_last_error()`).
///
/// # C Signature
///
/// ```c
/// char* kreuzberg_get_valid_token_reduction_levels(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn kreuzberg_get_valid_token_reduction_levels() -> *mut c_char {
    let json = format!(
        "[{}]",
        VALID_TOKEN_REDUCTION_LEVELS
            .iter()
            .map(|l| format!("\"{}\"", l))
            .collect::<Vec<_>>()
            .join(",")
    );

    match std::ffi::CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(e) => {
            set_last_error(format!("Failed to allocate string: {}", e));
            std::ptr::null_mut()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_binarization_method_valid() {
        unsafe {
            assert_eq!(
                kreuzberg_validate_binarization_method(b"otsu\0".as_ptr() as *const c_char),
                1
            );
            assert_eq!(
                kreuzberg_validate_binarization_method(b"adaptive\0".as_ptr() as *const c_char),
                1
            );
            assert_eq!(
                kreuzberg_validate_binarization_method(b"sauvola\0".as_ptr() as *const c_char),
                1
            );
        }
    }

    #[test]
    fn test_validate_binarization_method_invalid() {
        unsafe {
            assert_eq!(
                kreuzberg_validate_binarization_method(b"invalid\0".as_ptr() as *const c_char),
                0
            );
        }
    }

    #[test]
    fn test_validate_binarization_method_null() {
        unsafe {
            assert_eq!(kreuzberg_validate_binarization_method(std::ptr::null()), 0);
        }
    }

    #[test]
    fn test_validate_ocr_backend_valid() {
        unsafe {
            assert_eq!(
                kreuzberg_validate_ocr_backend(b"tesseract\0".as_ptr() as *const c_char),
                1
            );
            assert_eq!(
                kreuzberg_validate_ocr_backend(b"easyocr\0".as_ptr() as *const c_char),
                1
            );
            assert_eq!(
                kreuzberg_validate_ocr_backend(b"paddleocr\0".as_ptr() as *const c_char),
                1
            );
        }
    }

    #[test]
    fn test_validate_ocr_backend_invalid() {
        unsafe {
            assert_eq!(
                kreuzberg_validate_ocr_backend(b"invalid_backend\0".as_ptr() as *const c_char),
                0
            );
        }
    }

    #[test]
    fn test_validate_ocr_backend_null() {
        unsafe {
            assert_eq!(kreuzberg_validate_ocr_backend(std::ptr::null()), 0);
        }
    }

    #[test]
    fn test_validate_language_code_valid_2letter() {
        unsafe {
            assert_eq!(kreuzberg_validate_language_code(b"en\0".as_ptr() as *const c_char), 1);
            assert_eq!(kreuzberg_validate_language_code(b"de\0".as_ptr() as *const c_char), 1);
            assert_eq!(kreuzberg_validate_language_code(b"fr\0".as_ptr() as *const c_char), 1);
        }
    }

    #[test]
    fn test_validate_language_code_valid_3letter() {
        unsafe {
            assert_eq!(kreuzberg_validate_language_code(b"eng\0".as_ptr() as *const c_char), 1);
            assert_eq!(kreuzberg_validate_language_code(b"deu\0".as_ptr() as *const c_char), 1);
            assert_eq!(kreuzberg_validate_language_code(b"fra\0".as_ptr() as *const c_char), 1);
        }
    }

    #[test]
    fn test_validate_language_code_invalid() {
        unsafe {
            assert_eq!(
                kreuzberg_validate_language_code(b"invalid\0".as_ptr() as *const c_char),
                0
            );
            assert_eq!(kreuzberg_validate_language_code(b"xx\0".as_ptr() as *const c_char), 0);
        }
    }

    #[test]
    fn test_validate_language_code_null() {
        unsafe {
            assert_eq!(kreuzberg_validate_language_code(std::ptr::null()), 0);
        }
    }

    #[test]
    fn test_validate_token_reduction_level_valid() {
        unsafe {
            assert_eq!(
                kreuzberg_validate_token_reduction_level(b"off\0".as_ptr() as *const c_char),
                1
            );
            assert_eq!(
                kreuzberg_validate_token_reduction_level(b"light\0".as_ptr() as *const c_char),
                1
            );
            assert_eq!(
                kreuzberg_validate_token_reduction_level(b"moderate\0".as_ptr() as *const c_char),
                1
            );
            assert_eq!(
                kreuzberg_validate_token_reduction_level(b"aggressive\0".as_ptr() as *const c_char),
                1
            );
            assert_eq!(
                kreuzberg_validate_token_reduction_level(b"maximum\0".as_ptr() as *const c_char),
                1
            );
        }
    }

    #[test]
    fn test_validate_token_reduction_level_invalid() {
        unsafe {
            assert_eq!(
                kreuzberg_validate_token_reduction_level(b"extreme\0".as_ptr() as *const c_char),
                0
            );
        }
    }

    #[test]
    fn test_validate_token_reduction_level_null() {
        unsafe {
            assert_eq!(kreuzberg_validate_token_reduction_level(std::ptr::null()), 0);
        }
    }

    #[test]
    fn test_validate_tesseract_psm_valid() {
        for psm in 0..=13 {
            assert_eq!(kreuzberg_validate_tesseract_psm(psm), 1, "PSM {} should be valid", psm);
        }
    }

    #[test]
    fn test_validate_tesseract_psm_invalid() {
        assert_eq!(kreuzberg_validate_tesseract_psm(-1), 0);
        assert_eq!(kreuzberg_validate_tesseract_psm(14), 0);
        assert_eq!(kreuzberg_validate_tesseract_psm(100), 0);
    }

    #[test]
    fn test_validate_tesseract_oem_valid() {
        for oem in 0..=3 {
            assert_eq!(kreuzberg_validate_tesseract_oem(oem), 1, "OEM {} should be valid", oem);
        }
    }

    #[test]
    fn test_validate_tesseract_oem_invalid() {
        assert_eq!(kreuzberg_validate_tesseract_oem(-1), 0);
        assert_eq!(kreuzberg_validate_tesseract_oem(4), 0);
        assert_eq!(kreuzberg_validate_tesseract_oem(10), 0);
    }

    #[test]
    fn test_validate_output_format_valid() {
        unsafe {
            assert_eq!(kreuzberg_validate_output_format(b"text\0".as_ptr() as *const c_char), 1);
            assert_eq!(
                kreuzberg_validate_output_format(b"markdown\0".as_ptr() as *const c_char),
                1
            );
        }
    }

    #[test]
    fn test_validate_output_format_invalid() {
        unsafe {
            assert_eq!(kreuzberg_validate_output_format(b"json\0".as_ptr() as *const c_char), 0);
        }
    }

    #[test]
    fn test_validate_output_format_null() {
        unsafe {
            assert_eq!(kreuzberg_validate_output_format(std::ptr::null()), 0);
        }
    }

    #[test]
    fn test_validate_confidence_valid() {
        assert_eq!(kreuzberg_validate_confidence(0.0), 1);
        assert_eq!(kreuzberg_validate_confidence(0.5), 1);
        assert_eq!(kreuzberg_validate_confidence(1.0), 1);
    }

    #[test]
    fn test_validate_confidence_invalid() {
        assert_eq!(kreuzberg_validate_confidence(-0.1), 0);
        assert_eq!(kreuzberg_validate_confidence(1.1), 0);
        assert_eq!(kreuzberg_validate_confidence(2.0), 0);
    }

    #[test]
    fn test_validate_dpi_valid() {
        assert_eq!(kreuzberg_validate_dpi(72), 1);
        assert_eq!(kreuzberg_validate_dpi(96), 1);
        assert_eq!(kreuzberg_validate_dpi(300), 1);
        assert_eq!(kreuzberg_validate_dpi(600), 1);
    }

    #[test]
    fn test_validate_dpi_invalid() {
        assert_eq!(kreuzberg_validate_dpi(0), 0);
        assert_eq!(kreuzberg_validate_dpi(-1), 0);
        assert_eq!(kreuzberg_validate_dpi(2401), 0);
    }

    #[test]
    fn test_validate_chunking_params_valid() {
        assert_eq!(kreuzberg_validate_chunking_params(1000, 200), 1);
        assert_eq!(kreuzberg_validate_chunking_params(500, 50), 1);
        assert_eq!(kreuzberg_validate_chunking_params(1, 0), 1);
    }

    #[test]
    fn test_validate_chunking_params_invalid_zero_chars() {
        assert_eq!(kreuzberg_validate_chunking_params(0, 100), 0);
    }

    #[test]
    fn test_validate_chunking_params_invalid_overlap() {
        assert_eq!(kreuzberg_validate_chunking_params(100, 100), 0);
        assert_eq!(kreuzberg_validate_chunking_params(100, 150), 0);
    }

    #[test]
    fn test_get_valid_binarization_methods() {
        unsafe {
            let json_ptr = kreuzberg_get_valid_binarization_methods();
            assert!(!json_ptr.is_null(), "Should return non-null pointer");

            let c_str = CStr::from_ptr(json_ptr);
            let json_str = c_str.to_str().expect("Should be valid UTF-8");

            assert!(json_str.contains("otsu"));
            assert!(json_str.contains("adaptive"));
            assert!(json_str.contains("sauvola"));
            assert!(json_str.starts_with('[') && json_str.ends_with(']'));

            let _ = std::ffi::CString::from_raw(json_ptr as *mut c_char);
        }
    }

    #[test]
    fn test_get_valid_language_codes() {
        unsafe {
            let json_ptr = kreuzberg_get_valid_language_codes();
            assert!(!json_ptr.is_null(), "Should return non-null pointer");

            let c_str = CStr::from_ptr(json_ptr);
            let json_str = c_str.to_str().expect("Should be valid UTF-8");

            assert!(json_str.contains("en"));
            assert!(json_str.contains("de"));
            assert!(json_str.contains("eng"));
            assert!(json_str.contains("deu"));
            assert!(json_str.starts_with('[') && json_str.ends_with(']'));

            let _ = std::ffi::CString::from_raw(json_ptr as *mut c_char);
        }
    }

    #[test]
    fn test_get_valid_ocr_backends() {
        unsafe {
            let json_ptr = kreuzberg_get_valid_ocr_backends();
            assert!(!json_ptr.is_null(), "Should return non-null pointer");

            let c_str = CStr::from_ptr(json_ptr);
            let json_str = c_str.to_str().expect("Should be valid UTF-8");

            assert!(json_str.contains("tesseract"));
            assert!(json_str.contains("easyocr"));
            assert!(json_str.contains("paddleocr"));
            assert!(json_str.starts_with('[') && json_str.ends_with(']'));

            let _ = std::ffi::CString::from_raw(json_ptr as *mut c_char);
        }
    }

    #[test]
    fn test_get_valid_token_reduction_levels() {
        unsafe {
            let json_ptr = kreuzberg_get_valid_token_reduction_levels();
            assert!(!json_ptr.is_null(), "Should return non-null pointer");

            let c_str = CStr::from_ptr(json_ptr);
            let json_str = c_str.to_str().expect("Should be valid UTF-8");

            assert!(json_str.contains("off"));
            assert!(json_str.contains("light"));
            assert!(json_str.contains("moderate"));
            assert!(json_str.contains("aggressive"));
            assert!(json_str.contains("maximum"));
            assert!(json_str.starts_with('[') && json_str.ends_with(']'));

            let _ = std::ffi::CString::from_raw(json_ptr as *mut c_char);
        }
    }
}
