//! PyO3 wrappers for Kreuzberg validation functions.
//!
//! Exposes validation functions from kreuzberg core through Python-friendly interfaces.
//! All validation logic is implemented in Rust core and wrapped here for Python.

use kreuzberg::core::config_validation::{
    validate_binarization_method as validate_binarization_method_core,
    validate_chunking_params as validate_chunking_params_core, validate_confidence as validate_confidence_core,
    validate_dpi as validate_dpi_core, validate_language_code as validate_language_code_core,
    validate_ocr_backend as validate_ocr_backend_core, validate_output_format as validate_output_format_core,
    validate_tesseract_oem as validate_tesseract_oem_core, validate_tesseract_psm as validate_tesseract_psm_core,
    validate_token_reduction_level as validate_token_reduction_level_core,
};
use pyo3::prelude::*;

/// Validate a binarization method.
///
/// Args:
///     method (str): The binarization method to validate (e.g., "otsu", "adaptive", "sauvola")
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_binarization_method(method: &str) -> PyResult<bool> {
    Ok(validate_binarization_method_core(method).is_ok())
}

/// Validate an OCR backend.
///
/// Args:
///     backend (str): The OCR backend to validate (e.g., "tesseract", "easyocr", "paddleocr")
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_ocr_backend(backend: &str) -> PyResult<bool> {
    Ok(validate_ocr_backend_core(backend).is_ok())
}

/// Validate a language code (ISO 639-1 or 639-3 format).
///
/// Args:
///     code (str): The language code to validate (e.g., "en", "eng", "de", "deu")
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_language_code(code: &str) -> PyResult<bool> {
    Ok(validate_language_code_core(code).is_ok())
}

/// Validate a token reduction level.
///
/// Args:
///     level (str): The token reduction level to validate (e.g., "off", "light", "moderate", "aggressive", "maximum")
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_token_reduction_level(level: &str) -> PyResult<bool> {
    Ok(validate_token_reduction_level_core(level).is_ok())
}

/// Validate a Tesseract Page Segmentation Mode (PSM) value.
///
/// Args:
///     psm (int): The PSM value to validate (valid range: 0-13)
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_tesseract_psm(psm: i32) -> PyResult<bool> {
    Ok(validate_tesseract_psm_core(psm).is_ok())
}

/// Validate a Tesseract OCR Engine Mode (OEM) value.
///
/// Args:
///     oem (int): The OEM value to validate (valid range: 0-3)
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_tesseract_oem(oem: i32) -> PyResult<bool> {
    Ok(validate_tesseract_oem_core(oem).is_ok())
}

/// Validate a Tesseract output format string.
///
/// Args:
///     format (str): The output format to validate (e.g., "text", "markdown")
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_output_format(format: &str) -> PyResult<bool> {
    Ok(validate_output_format_core(format).is_ok())
}

/// Validate a confidence threshold value.
///
/// Args:
///     confidence (float): The confidence threshold to validate (valid range: 0.0-1.0)
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_confidence(confidence: f64) -> PyResult<bool> {
    Ok(validate_confidence_core(confidence).is_ok())
}

/// Validate a DPI (dots per inch) value.
///
/// Args:
///     dpi (int): The DPI value to validate (must be positive, typically 72-600)
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_dpi(dpi: i32) -> PyResult<bool> {
    Ok(validate_dpi_core(dpi).is_ok())
}

/// Validate chunking parameters.
///
/// Args:
///     max_chars (int): Maximum characters per chunk (must be > 0)
///     max_overlap (int): Maximum overlap between chunks (must be < max_chars)
///
/// Returns:
///     bool: True if valid, False if invalid
#[pyfunction]
pub fn validate_chunking_params(max_chars: usize, max_overlap: usize) -> PyResult<bool> {
    Ok(validate_chunking_params_core(max_chars, max_overlap).is_ok())
}

/// Get list of valid binarization methods.
///
/// Returns:
///     list[str]: List of valid binarization method names
#[pyfunction]
pub fn get_valid_binarization_methods() -> PyResult<Vec<String>> {
    Ok(vec!["otsu".to_string(), "adaptive".to_string(), "sauvola".to_string()])
}

/// Get list of valid language codes.
///
/// Returns:
///     list[str]: List of valid language codes (ISO 639-1 and 639-3)
#[pyfunction]
pub fn get_valid_language_codes() -> PyResult<Vec<String>> {
    Ok(vec![
        "en", "de", "fr", "es", "it", "pt", "nl", "pl", "ru", "zh", "ja", "ko", "bg", "cs", "da", "el", "et", "fi",
        "hu", "lt", "lv", "ro", "sk", "sl", "sv", "uk", "ar", "hi", "th", "tr", "vi", "eng", "deu", "fra", "spa",
        "ita", "por", "nld", "pol", "rus", "zho", "jpn", "kor", "ces", "dan", "ell", "est", "fin", "hun", "lit", "lav",
        "ron", "slk", "slv", "swe", "tur",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect())
}

/// Get list of valid OCR backends.
///
/// Returns:
///     list[str]: List of valid OCR backend names
#[pyfunction]
pub fn get_valid_ocr_backends() -> PyResult<Vec<String>> {
    Ok(vec![
        "tesseract".to_string(),
        "easyocr".to_string(),
        "paddleocr".to_string(),
    ])
}

/// Get list of valid token reduction levels.
///
/// Returns:
///     list[str]: List of valid token reduction level names
#[pyfunction]
pub fn get_valid_token_reduction_levels() -> PyResult<Vec<String>> {
    Ok(vec![
        "off".to_string(),
        "light".to_string(),
        "moderate".to_string(),
        "aggressive".to_string(),
        "maximum".to_string(),
    ])
}
