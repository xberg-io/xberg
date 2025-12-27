//! Validation functions for PHP bindings
//!
//! Provides validation functions for configuration parameters.

use ext_php_rs::prelude::*;

/// Validate a binarization method.
///
/// # Parameters
///
/// - `method` (string): The binarization method to validate
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Valid Methods
///
/// - "otsu"
/// - "adaptive"
/// - "sauvola"
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_binarization_method("otsu")) {
///     echo "Valid method\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_binarization_method(method: String) -> bool {
    kreuzberg::core::config_validation::validate_binarization_method(&method).is_ok()
}

/// Validate an OCR backend.
///
/// # Parameters
///
/// - `backend` (string): The OCR backend to validate
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Valid Backends
///
/// - "tesseract"
/// - "easyocr"
/// - "paddleocr"
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_ocr_backend("tesseract")) {
///     echo "Valid backend\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_ocr_backend(backend: String) -> bool {
    kreuzberg::core::config_validation::validate_ocr_backend(&backend).is_ok()
}

/// Validate a language code (ISO 639-1 or 639-3 format).
///
/// # Parameters
///
/// - `code` (string): The language code to validate
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_language_code("eng")) {
///     echo "Valid language code\n";
/// }
/// if (kreuzberg_validate_language_code("en")) {
///     echo "Also valid (2-letter code)\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_language_code(code: String) -> bool {
    kreuzberg::core::config_validation::validate_language_code(&code).is_ok()
}

/// Validate a token reduction level.
///
/// # Parameters
///
/// - `level` (string): The token reduction level to validate
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Valid Levels
///
/// - "off"
/// - "light"
/// - "moderate"
/// - "aggressive"
/// - "maximum"
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_token_reduction_level("moderate")) {
///     echo "Valid level\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_token_reduction_level(level: String) -> bool {
    kreuzberg::core::config_validation::validate_token_reduction_level(&level).is_ok()
}

/// Validate a Tesseract Page Segmentation Mode (PSM) value.
///
/// # Parameters
///
/// - `psm` (int): The PSM value to validate (valid range: 0-13)
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_tesseract_psm(3)) {
///     echo "Valid PSM\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_tesseract_psm(psm: i64) -> bool {
    kreuzberg::core::config_validation::validate_tesseract_psm(psm as i32).is_ok()
}

/// Validate a Tesseract OCR Engine Mode (OEM) value.
///
/// # Parameters
///
/// - `oem` (int): The OEM value to validate (valid range: 0-3)
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_tesseract_oem(3)) {
///     echo "Valid OEM\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_tesseract_oem(oem: i64) -> bool {
    kreuzberg::core::config_validation::validate_tesseract_oem(oem as i32).is_ok()
}

/// Validate a Tesseract output format string.
///
/// # Parameters
///
/// - `format` (string): The output format to validate
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Valid Formats
///
/// - "text"
/// - "markdown"
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_output_format("markdown")) {
///     echo "Valid format\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_output_format(format: String) -> bool {
    kreuzberg::core::config_validation::validate_output_format(&format).is_ok()
}

/// Validate a confidence threshold value.
///
/// # Parameters
///
/// - `confidence` (float): The confidence threshold to validate (valid range: 0.0-1.0)
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_confidence(0.8)) {
///     echo "Valid confidence\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_confidence(confidence: f64) -> bool {
    kreuzberg::core::config_validation::validate_confidence(confidence).is_ok()
}

/// Validate a DPI (dots per inch) value.
///
/// # Parameters
///
/// - `dpi` (int): The DPI value to validate (must be positive, typically 72-600)
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_dpi(300)) {
///     echo "Valid DPI\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_dpi(dpi: i64) -> bool {
    kreuzberg::core::config_validation::validate_dpi(dpi as i32).is_ok()
}

/// Validate chunking parameters.
///
/// # Parameters
///
/// - `max_chars` (int): Maximum characters per chunk (must be > 0)
/// - `max_overlap` (int): Maximum overlap between chunks (must be < max_chars)
///
/// # Returns
///
/// `true` if valid, `false` otherwise
///
/// # Example
///
/// ```php
/// if (kreuzberg_validate_chunking_params(1000, 200)) {
///     echo "Valid chunking params\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_chunking_params(max_chars: i64, max_overlap: i64) -> bool {
    kreuzberg::core::config_validation::validate_chunking_params(max_chars as usize, max_overlap as usize).is_ok()
}

/// Get list of valid binarization methods.
///
/// # Returns
///
/// Array of valid binarization method names
///
/// # Example
///
/// ```php
/// $methods = kreuzberg_get_valid_binarization_methods();
/// print_r($methods); // ["otsu", "adaptive", "sauvola"]
/// ```
#[php_function]
pub fn kreuzberg_get_valid_binarization_methods() -> Vec<String> {
    vec!["otsu".to_string(), "adaptive".to_string(), "sauvola".to_string()]
}

/// Get list of valid language codes.
///
/// Returns both 2-letter (ISO 639-1) and 3-letter (ISO 639-3) language codes.
///
/// # Returns
///
/// Array of valid language codes
///
/// # Example
///
/// ```php
/// $codes = kreuzberg_get_valid_language_codes();
/// // Includes: "en", "eng", "de", "deu", etc.
/// ```
#[php_function]
pub fn kreuzberg_get_valid_language_codes() -> Vec<String> {
    vec![
        "en", "de", "fr", "es", "it", "pt", "nl", "pl", "ru", "zh", "ja", "ko", "bg", "cs", "da", "el", "et", "fi",
        "hu", "lt", "lv", "ro", "sk", "sl", "sv", "uk", "ar", "hi", "th", "tr", "vi", "eng", "deu", "fra", "spa",
        "ita", "por", "nld", "pol", "rus", "zho", "jpn", "kor", "ces", "dan", "ell", "est", "fin", "hun", "lit", "lav",
        "ron", "slk", "slv", "swe", "tur",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Get list of valid OCR backends.
///
/// # Returns
///
/// Array of valid OCR backend names
///
/// # Example
///
/// ```php
/// $backends = kreuzberg_get_valid_ocr_backends();
/// print_r($backends); // ["tesseract", "easyocr", "paddleocr"]
/// ```
#[php_function]
pub fn kreuzberg_get_valid_ocr_backends() -> Vec<String> {
    vec!["tesseract".to_string(), "easyocr".to_string(), "paddleocr".to_string()]
}

/// Get list of valid token reduction levels.
///
/// # Returns
///
/// Array of valid token reduction level names
///
/// # Example
///
/// ```php
/// $levels = kreuzberg_get_valid_token_reduction_levels();
/// print_r($levels); // ["off", "light", "moderate", "aggressive", "maximum"]
/// ```
#[php_function]
pub fn kreuzberg_get_valid_token_reduction_levels() -> Vec<String> {
    vec![
        "off".to_string(),
        "light".to_string(),
        "moderate".to_string(),
        "aggressive".to_string(),
        "maximum".to_string(),
    ]
}
