use crate::error_handling::convert_error;
use crate::{kreuzberg_free_string, kreuzberg_last_error_code};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::ffi::{CStr, c_char};

use crate::get_panic_context;
use crate::kreuzberg_config_free;
use crate::kreuzberg_config_from_json;
use crate::kreuzberg_config_get_field;
use crate::kreuzberg_config_merge;
use crate::kreuzberg_config_to_json;
use crate::kreuzberg_get_valid_binarization_methods;
use crate::kreuzberg_get_valid_language_codes;
use crate::kreuzberg_get_valid_ocr_backends;
use crate::kreuzberg_get_valid_token_reduction_levels;
use crate::kreuzberg_validate_binarization_method;
use crate::kreuzberg_validate_chunking_params;
use crate::kreuzberg_validate_confidence;
use crate::kreuzberg_validate_dpi;
use crate::kreuzberg_validate_language_code;
use crate::kreuzberg_validate_ocr_backend;
use crate::kreuzberg_validate_output_format;
use crate::kreuzberg_validate_tesseract_oem;
use crate::kreuzberg_validate_tesseract_psm;
use crate::kreuzberg_validate_token_reduction_level;

/// Validate that a MIME type is supported by Kreuzberg.
///
/// Checks if a MIME type is in the list of supported formats. Note that any
/// `image/*` MIME type is automatically considered valid.
///
/// # Parameters
///
/// * `mime_type` - The MIME type to validate (string)
///
/// # Returns
///
/// The validated MIME type (may be normalized).
///
/// # Errors
///
/// Throws an error if the MIME type is not supported.
#[napi]
pub fn validate_mime_type(mime_type: String) -> Result<String> {
    kreuzberg::core::mime::validate_mime_type(&mime_type).map_err(convert_error)
}

/// Get file extensions for a given MIME type.
///
/// Returns an array of file extensions commonly associated with the specified
/// MIME type. For example, 'application/pdf' returns ['pdf'].
///
/// # Parameters
///
/// * `mime_type` - The MIME type to look up (e.g., 'application/pdf', 'image/jpeg')
///
/// # Returns
///
/// Array of file extensions (without leading dots).
///
/// # Errors
///
/// Throws an error if the MIME type is not recognized or supported.
///
/// # Example
///
/// ```typescript
/// import { getExtensionsForMime } from 'kreuzberg';
///
/// // Get extensions for PDF
/// const pdfExts = getExtensionsForMime('application/pdf');
/// console.log(pdfExts); // ['pdf']
///
/// // Get extensions for JPEG
/// const jpegExts = getExtensionsForMime('image/jpeg');
/// console.log(jpegExts); // ['jpg', 'jpeg']
/// ```
#[napi]
#[allow(dead_code)]
pub fn get_extensions_for_mime(mime_type: String) -> Result<Vec<String>> {
    kreuzberg::core::mime::get_extensions_for_mime(&mime_type).map_err(convert_error)
}

/// Embedding preset configuration for TypeScript bindings.
///
/// Contains all settings for a specific embedding model preset.
#[napi(object)]
#[allow(dead_code)]
pub struct EmbeddingPreset {
    /// Name of the preset (e.g., "fast", "balanced", "quality", "multilingual")
    pub name: String,
    /// Recommended chunk size in characters
    pub chunk_size: u32,
    /// Recommended overlap in characters
    pub overlap: u32,
    /// Model identifier (e.g., "AllMiniLML6V2Q", "BGEBaseENV15")
    pub model_name: String,
    /// Embedding vector dimensions
    pub dimensions: u32,
    /// Human-readable description of the preset
    pub description: String,
}

/// List all available embedding preset names.
///
/// Returns an array of preset names that can be used with `getEmbeddingPreset`.
///
/// # Returns
///
/// Array of 4 preset names: ["fast", "balanced", "quality", "multilingual"]
///
/// # Example
///
/// ```typescript
/// import { listEmbeddingPresets } from 'kreuzberg';
///
/// const presets = listEmbeddingPresets();
/// console.log(presets); // ['fast', 'balanced', 'quality', 'multilingual']
/// ```
#[napi(js_name = "listEmbeddingPresets")]
#[allow(dead_code)]
pub fn list_embedding_presets() -> Vec<String> {
    kreuzberg::embeddings::list_presets()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

/// Get a specific embedding preset by name.
///
/// Returns a preset configuration object, or null if the preset name is not found.
///
/// # Arguments
///
/// * `name` - The preset name (case-sensitive)
///
/// # Returns
///
/// An `EmbeddingPreset` object with the following properties:
/// - `name`: string - Preset name
/// - `chunkSize`: number - Recommended chunk size in characters
/// - `overlap`: number - Recommended overlap in characters
/// - `modelName`: string - Model identifier
/// - `dimensions`: number - Embedding vector dimensions
/// - `description`: string - Human-readable description
///
/// Returns `null` if preset name is not found.
///
/// # Example
///
/// ```typescript
/// import { getEmbeddingPreset } from 'kreuzberg';
///
/// const preset = getEmbeddingPreset('balanced');
/// if (preset) {
///   console.log(`Model: ${preset.modelName}, Dims: ${preset.dimensions}`);
///   // Model: BGEBaseENV15, Dims: 768
/// }
/// ```
#[napi(js_name = "getEmbeddingPreset")]
#[allow(dead_code)]
pub fn get_embedding_preset(name: String) -> Option<EmbeddingPreset> {
    let preset = kreuzberg::embeddings::get_preset(&name)?;

    let model_name = format!("{:?}", preset.model);

    Some(EmbeddingPreset {
        name: preset.name.to_string(),
        chunk_size: preset.chunk_size as u32,
        overlap: preset.overlap as u32,
        model_name,
        dimensions: preset.dimensions as u32,
        description: preset.description.to_string(),
    })
}

/// Get the error code for the last FFI error.
///
/// Returns the FFI error code as an integer. Error codes are:
/// - 0: Success (no error)
/// - 1: GenericError
/// - 2: Panic
/// - 3: InvalidArgument
/// - 4: IoError
/// - 5: ParsingError
/// - 6: OcrError
/// - 7: MissingDependency
///
/// This is useful for programmatic error handling and distinguishing
/// between different types of failures in native code.
///
/// # Returns
///
/// The integer error code.
///
/// # Example
///
/// ```typescript
/// import { extractFile, getLastErrorCode, ErrorCode } from '@kreuzberg/node';
///
/// try {
///   const result = await extractFile('document.pdf');
/// } catch (error) {
///   const code = getLastErrorCode();
///   if (code === ErrorCode.Panic) {
///     console.error('Native code panic detected');
///   }
/// }
/// ```
#[napi(js_name = "getLastErrorCode")]
pub fn get_last_error_code() -> i32 {
    unsafe { kreuzberg_last_error_code() }
}

/// Get panic context information if the last error was a panic.
///
/// Returns detailed information about a panic in native code, or null
/// if the last error was not a panic.
///
/// # Returns
///
/// A `PanicContext` object with:
/// - `file`: string - Source file where panic occurred
/// - `line`: number - Line number
/// - `function`: string - Function name
/// - `message`: string - Panic message
/// - `timestamp_secs`: number - Unix timestamp (seconds since epoch)
///
/// Returns `null` if no panic context is available.
///
/// # Example
///
/// ```typescript
/// import { extractFile, getLastPanicContext } from '@kreuzberg/node';
///
/// try {
///   const result = await extractFile('document.pdf');
/// } catch (error) {
///   const context = getLastPanicContext();
///   if (context) {
///     console.error(`Panic at ${context.file}:${context.line}`);
///     console.error(`In function: ${context.function}`);
///     console.error(`Message: ${context.message}`);
///   }
/// }
/// ```
#[napi(js_name = "getLastPanicContext")]
pub fn get_last_panic_context() -> Option<serde_json::Value> {
    get_panic_context()
}

/// Validates a binarization method string.
///
/// Valid methods: "otsu", "adaptive", "sauvola"
///
/// # Arguments
///
/// * `method` - The binarization method to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateBinarizationMethod } from '@kreuzberg/node';
///
/// if (validateBinarizationMethod('otsu')) {
///   console.log('Valid method');
/// } else {
///   console.log('Invalid method');
/// }
/// ```
#[napi(js_name = "validateBinarizationMethod")]
pub fn validate_binarization_method(method: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(method.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in binarization method: {}", method),
        )
    })?;

    let result = unsafe { kreuzberg_validate_binarization_method(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates an OCR backend string.
///
/// Valid backends: "tesseract", "easyocr", "paddleocr", "paddle-ocr", "rapidocr", "rapid-ocr", "rapidpaddle", "rapid-paddle"
///
/// # Arguments
///
/// * `backend` - The OCR backend to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateOcrBackend } from '@kreuzberg/node';
///
/// if (validateOcrBackend('tesseract')) {
///   console.log('Valid backend');
/// }
/// ```
#[napi(js_name = "validateOcrBackend")]
pub fn validate_ocr_backend(backend: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(backend.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in OCR backend: {}", backend),
        )
    })?;

    let result = unsafe { kreuzberg_validate_ocr_backend(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates a language code (ISO 639-1 or 639-3 format).
///
/// Accepts both 2-letter codes (e.g., "en", "de") and 3-letter codes (e.g., "eng", "deu").
///
/// # Arguments
///
/// * `code` - The language code to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateLanguageCode } from '@kreuzberg/node';
///
/// if (validateLanguageCode('en')) {
///   console.log('Valid language code');
/// }
/// ```
#[napi(js_name = "validateLanguageCode")]
pub fn validate_language_code(code: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(code.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in language code: {}", code),
        )
    })?;

    let result = unsafe { kreuzberg_validate_language_code(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates a token reduction level string.
///
/// Valid levels: "off", "light", "moderate", "aggressive", "maximum"
///
/// # Arguments
///
/// * `level` - The token reduction level to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateTokenReductionLevel } from '@kreuzberg/node';
///
/// if (validateTokenReductionLevel('moderate')) {
///   console.log('Valid token reduction level');
/// }
/// ```
#[napi(js_name = "validateTokenReductionLevel")]
pub fn validate_token_reduction_level(level: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(level.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in token reduction level: {}", level),
        )
    })?;

    let result = unsafe { kreuzberg_validate_token_reduction_level(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates a Tesseract Page Segmentation Mode (PSM) value.
///
/// Valid range: 0-13
///
/// # Arguments
///
/// * `psm` - The PSM value to validate
///
/// # Returns
///
/// `true` if valid (0-13), `false` otherwise.
///
/// # Example
///
/// ```typescript
/// import { validateTesseractPsm } from '@kreuzberg/node';
///
/// if (validateTesseractPsm(3)) {
///   console.log('Valid PSM');
/// }
/// ```
#[napi(js_name = "validateTesseractPsm")]
pub fn validate_tesseract_psm(psm: i32) -> bool {
    unsafe { kreuzberg_validate_tesseract_psm(psm) == 1 }
}

/// Validates a Tesseract OCR Engine Mode (OEM) value.
///
/// Valid range: 0-3
///
/// # Arguments
///
/// * `oem` - The OEM value to validate
///
/// # Returns
///
/// `true` if valid (0-3), `false` otherwise.
///
/// # Example
///
/// ```typescript
/// import { validateTesseractOem } from '@kreuzberg/node';
///
/// if (validateTesseractOem(1)) {
///   console.log('Valid OEM');
/// }
/// ```
#[napi(js_name = "validateTesseractOem")]
pub fn validate_tesseract_oem(oem: i32) -> bool {
    unsafe { kreuzberg_validate_tesseract_oem(oem) == 1 }
}

/// Validates a tesseract output format string.
///
/// Valid formats: "text", "markdown"
///
/// # Arguments
///
/// * `format` - The output format to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateOutputFormat } from '@kreuzberg/node';
///
/// if (validateOutputFormat('markdown')) {
///   console.log('Valid output format');
/// }
/// ```
#[napi(js_name = "validateOutputFormat")]
pub fn validate_output_format(format: String) -> Result<bool> {
    let c_str = std::ffi::CString::new(format.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in output format: {}", format),
        )
    })?;

    let result = unsafe { kreuzberg_validate_output_format(c_str.as_ptr()) };
    Ok(result == 1)
}

/// Validates a confidence threshold value.
///
/// Valid range: 0.0 to 1.0 (inclusive)
///
/// # Arguments
///
/// * `confidence` - The confidence threshold to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateConfidence } from '@kreuzberg/node';
///
/// if (validateConfidence(0.75)) {
///   console.log('Valid confidence threshold');
/// }
/// ```
#[napi(js_name = "validateConfidence")]
pub fn validate_confidence(confidence: f64) -> bool {
    unsafe { kreuzberg_validate_confidence(confidence) == 1 }
}

/// Validates a DPI (dots per inch) value.
///
/// Valid range: 1-2400
///
/// # Arguments
///
/// * `dpi` - The DPI value to validate
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateDpi } from '@kreuzberg/node';
///
/// if (validateDpi(300)) {
///   console.log('Valid DPI');
/// }
/// ```
#[napi(js_name = "validateDpi")]
pub fn validate_dpi(dpi: i32) -> bool {
    unsafe { kreuzberg_validate_dpi(dpi) == 1 }
}

/// Validates chunking parameters.
///
/// Checks that `maxChars > 0` and `maxOverlap < maxChars`.
///
/// # Arguments
///
/// * `max_chars` - Maximum characters per chunk
/// * `max_overlap` - Maximum overlap between chunks
///
/// # Returns
///
/// `true` if valid, `false` if invalid.
///
/// # Example
///
/// ```typescript
/// import { validateChunkingParams } from '@kreuzberg/node';
///
/// if (validateChunkingParams(1000, 200)) {
///   console.log('Valid chunking parameters');
/// }
/// ```
#[napi(js_name = "validateChunkingParams")]
pub fn validate_chunking_params(max_characters: u32, overlap: u32) -> bool {
    unsafe { kreuzberg_validate_chunking_params(max_characters as usize, overlap as usize) == 1 }
}

/// Get valid binarization methods.
///
/// Returns a list of all valid binarization method values.
///
/// # Returns
///
/// Array of valid binarization methods: ["otsu", "adaptive", "sauvola"]
///
/// # Example
///
/// ```typescript
/// import { getValidBinarizationMethods } from '@kreuzberg/node';
///
/// const methods = getValidBinarizationMethods();
/// console.log(methods); // ['otsu', 'adaptive', 'sauvola']
/// ```
#[napi(js_name = "getValidBinarizationMethods")]
pub fn get_valid_binarization_methods() -> Result<Vec<String>> {
    let json_str = unsafe {
        let ptr = kreuzberg_get_valid_binarization_methods();
        if ptr.is_null() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Failed to get valid binarization methods",
            ));
        }

        let c_str = CStr::from_ptr(ptr);
        let result = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in binarization methods"))?
            .to_string();

        kreuzberg_free_string(ptr as *mut c_char);
        result
    };

    let parsed: Vec<String> = serde_json::from_str(&json_str).map_err(|_| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse binarization methods JSON",
        )
    })?;

    Ok(parsed)
}

/// Get valid language codes.
///
/// Returns a list of all valid language codes in ISO 639-1 and 639-3 formats.
///
/// # Returns
///
/// Array of valid language codes (both 2-letter and 3-letter codes)
///
/// # Example
///
/// ```typescript
/// import { getValidLanguageCodes } from '@kreuzberg/node';
///
/// const codes = getValidLanguageCodes();
/// console.log(codes); // ['en', 'de', 'fr', ..., 'eng', 'deu', 'fra', ...]
/// ```
#[napi(js_name = "getValidLanguageCodes")]
pub fn get_valid_language_codes() -> Result<Vec<String>> {
    let json_str = unsafe {
        let ptr = kreuzberg_get_valid_language_codes();
        if ptr.is_null() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Failed to get valid language codes",
            ));
        }

        let c_str = CStr::from_ptr(ptr);
        let result = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in language codes"))?
            .to_string();

        kreuzberg_free_string(ptr as *mut c_char);
        result
    };

    let parsed: Vec<String> = serde_json::from_str(&json_str)
        .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Failed to parse language codes JSON"))?;

    Ok(parsed)
}

/// Get valid OCR backends.
///
/// Returns a list of all valid OCR backend values.
///
/// # Returns
///
/// Array of valid OCR backends: ["tesseract", "easyocr", "paddleocr", "paddle-ocr", "rapidocr", "rapid-ocr", "rapidpaddle", "rapid-paddle"]
///
/// # Example
///
/// ```typescript
/// import { getValidOcrBackends } from '@kreuzberg/node';
///
/// const backends = getValidOcrBackends();
/// console.log(backends); // ['tesseract', 'easyocr', 'paddleocr', 'paddle-ocr', 'rapidocr', 'rapid-ocr', 'rapidpaddle', 'rapid-paddle']
/// ```
#[napi(js_name = "getValidOcrBackends")]
pub fn get_valid_ocr_backends() -> Result<Vec<String>> {
    let json_str = unsafe {
        let ptr = kreuzberg_get_valid_ocr_backends();
        if ptr.is_null() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Failed to get valid OCR backends",
            ));
        }

        let c_str = CStr::from_ptr(ptr);
        let result = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in OCR backends"))?
            .to_string();

        kreuzberg_free_string(ptr as *mut c_char);
        result
    };

    let parsed: Vec<String> = serde_json::from_str(&json_str)
        .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Failed to parse OCR backends JSON"))?;

    Ok(parsed)
}

/// Get valid token reduction levels.
///
/// Returns a list of all valid token reduction level values.
///
/// # Returns
///
/// Array of valid levels: ["off", "light", "moderate", "aggressive", "maximum"]
///
/// # Example
///
/// ```typescript
/// import { getValidTokenReductionLevels } from '@kreuzberg/node';
///
/// const levels = getValidTokenReductionLevels();
/// console.log(levels); // ['off', 'light', 'moderate', 'aggressive', 'maximum']
/// ```
#[napi(js_name = "getValidTokenReductionLevels")]
pub fn get_valid_token_reduction_levels() -> Result<Vec<String>> {
    let json_str = unsafe {
        let ptr = kreuzberg_get_valid_token_reduction_levels();
        if ptr.is_null() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Failed to get valid token reduction levels",
            ));
        }

        let c_str = CStr::from_ptr(ptr);
        let result = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in token reduction levels"))?
            .to_string();

        kreuzberg_free_string(ptr as *mut c_char);
        result
    };

    let parsed: Vec<String> = serde_json::from_str(&json_str).map_err(|_| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse token reduction levels JSON",
        )
    })?;

    Ok(parsed)
}

/// Validate and normalize an ExtractionConfig JSON string via FFI.
///
/// This validates the JSON and returns a normalized version, using the shared
/// FFI layer to ensure consistent validation across all language bindings.
///
/// # Arguments
///
/// * `json_str` - A JSON string containing the configuration
///
/// # Returns
///
/// The normalized JSON string representation of the config, or error
#[napi(js_name = "configValidateAndNormalize")]
pub fn config_validate_and_normalize(json_str: String) -> Result<String> {
    let c_str = std::ffi::CString::new(json_str.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in config JSON: {}", json_str),
        )
    })?;

    let config_ptr = unsafe { kreuzberg_config_from_json(c_str.as_ptr()) };

    if config_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse config from JSON",
        ));
    }

    let json_ptr = unsafe { kreuzberg_config_to_json(config_ptr) };
    unsafe {
        kreuzberg_config_free(config_ptr);
    }

    if json_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to serialize parsed config to JSON",
        ));
    }

    let result = unsafe {
        let c_str = CStr::from_ptr(json_ptr);
        let json_str = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in JSON"))?
            .to_string();

        kreuzberg_free_string(json_ptr as *mut c_char);
        json_str
    };

    Ok(result)
}

/// Get a specific field from config (represented as JSON string) by name via FFI.
///
/// Retrieves a configuration field by path, supporting nested access with
/// dot notation (e.g., "ocr.backend"). Returns the field value as a JSON string.
///
/// # Arguments
///
/// * `json_str` - A JSON string representation of the configuration
/// * `field_name` - The field path to retrieve (e.g., "useCache", "ocr.backend")
///
/// # Returns
///
/// The field value as a JSON string, or null if not found
#[napi(js_name = "configGetFieldInternal")]
pub fn config_get_field_internal(json_str: String, field_name: String) -> Result<Option<String>> {
    let c_str = std::ffi::CString::new(json_str.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in config JSON: {}", json_str),
        )
    })?;

    let config_ptr = unsafe { kreuzberg_config_from_json(c_str.as_ptr()) };

    if config_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse config from JSON",
        ));
    }

    let c_field_name = std::ffi::CString::new(field_name.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in field name: {}", field_name),
        )
    })?;

    let field_ptr = unsafe { kreuzberg_config_get_field(config_ptr, c_field_name.as_ptr()) };
    unsafe {
        kreuzberg_config_free(config_ptr);
    }

    if field_ptr.is_null() {
        return Ok(None);
    }

    let result = unsafe {
        let c_str = CStr::from_ptr(field_ptr);
        let field_str = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in field value"))?
            .to_string();

        kreuzberg_free_string(field_ptr as *mut c_char);
        field_str
    };

    Ok(Some(result))
}

/// Merge two configs (override takes precedence over base) via FFI.
///
/// Performs a shallow merge where fields from the override config take
/// precedence over fields in the base config.
///
/// # Arguments
///
/// * `base_json` - A JSON string representation of the base ExtractionConfig
/// * `override_json` - A JSON string representation of the override ExtractionConfig
///
/// # Returns
///
/// The merged configuration as a JSON string, or error
#[napi(js_name = "configMergeInternal")]
pub fn config_merge_internal(base_json: String, override_json: String) -> Result<String> {
    let base_c_str = std::ffi::CString::new(base_json.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in base config JSON: {}", base_json),
        )
    })?;

    let override_c_str = std::ffi::CString::new(override_json.clone()).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid UTF-8 in override config JSON: {}", override_json),
        )
    })?;

    let base_ptr = unsafe { kreuzberg_config_from_json(base_c_str.as_ptr()) };

    if base_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse base config from JSON",
        ));
    }

    let override_ptr = unsafe { kreuzberg_config_from_json(override_c_str.as_ptr()) };

    if override_ptr.is_null() {
        unsafe {
            kreuzberg_config_free(base_ptr);
        }
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to parse override config from JSON",
        ));
    }

    let merge_result = unsafe { kreuzberg_config_merge(base_ptr, override_ptr) };

    if merge_result == 0 {
        unsafe {
            kreuzberg_config_free(base_ptr);
            kreuzberg_config_free(override_ptr);
        }
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to merge configs",
        ));
    }

    let json_ptr = unsafe { kreuzberg_config_to_json(base_ptr) };

    unsafe {
        kreuzberg_config_free(base_ptr);
        kreuzberg_config_free(override_ptr);
    }

    if json_ptr.is_null() {
        return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to serialize merged config to JSON",
        ));
    }

    let result = unsafe {
        let c_str = CStr::from_ptr(json_ptr);
        let json_str = c_str
            .to_str()
            .map_err(|_| napi::Error::new(napi::Status::GenericFailure, "Invalid UTF-8 in JSON"))?
            .to_string();

        kreuzberg_free_string(json_ptr as *mut c_char);
        json_str
    };

    Ok(result)
}

// #[cfg(all(
// #[global_allocator]
