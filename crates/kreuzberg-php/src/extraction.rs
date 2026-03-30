//! Core extraction functions
//!
//! Provides extraction functions for PHP.

use ahash::AHashMap;
use ext_php_rs::binary_slice::BinarySlice;
use ext_php_rs::prelude::*;
use std::borrow::Cow;

use crate::config::{parse_config_from_json, parse_file_config_from_json};
use crate::error::to_php_exception;
use crate::types::ExtractionResult;

/// Extract the extract_tables flag from config JSON.
/// Defaults to true if not specified.
pub(crate) fn should_extract_tables(config_json: &Option<String>) -> PhpResult<bool> {
    if let Some(json) = config_json {
        match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => {
                // Look for extract_tables field in the JSON config
                Ok(value.get("extract_tables").and_then(|v| v.as_bool()).unwrap_or(true))
            }
            Err(_) => {
                // If JSON parsing fails, default to true (extract tables)
                Ok(true)
            }
        }
    } else {
        Ok(true)
    }
}

/// Extract content from a file.
///
/// # Parameters
///
/// - `path` (string): Path to the file to extract
/// - `mime_type` (string|null): Optional MIME type hint (auto-detected if null)
/// - `config_json` (string|null): JSON-encoded extraction configuration (uses defaults if null)
///
/// # Returns
///
/// ExtractionResult with content, metadata, and tables
///
/// # Throws
///
/// - ValidationException: Invalid configuration or unsupported format
/// - ParsingException: Document parsing failed
/// - OcrException: OCR processing failed
/// - Exception: File access errors or other runtime errors
///
/// # Example
///
/// ```php
/// // Simple extraction with defaults
/// $result = kreuzberg_extract_file("document.pdf");
/// echo $result->content;
///
/// // With custom configuration
/// $config = new \Kreuzberg\Config\ExtractionConfig(
///     useCache: false,
///     ocr: new \Kreuzberg\Config\OcrConfig(language: "deu")
/// );
/// $result = kreuzberg_extract_file("german.pdf", null, $config->toJson());
/// ```
#[php_function]
pub fn kreuzberg_extract_file(
    path: String,
    mime_type: Option<String>,
    config_json: Option<String>,
) -> PhpResult<ExtractionResult> {
    let rust_config = match &config_json {
        Some(json) => parse_config_from_json(json).map_err(PhpException::from)?,
        None => Default::default(),
    };

    // Check if tables should be extracted from config
    let extract_tables = should_extract_tables(&config_json)?;

    let result = kreuzberg::extract_file_sync(&path, mime_type.as_deref(), &rust_config).map_err(to_php_exception)?;

    ExtractionResult::from_rust_with_config(result, extract_tables)
}

/// Extract content from bytes.
///
/// # Parameters
///
/// - `data` (string): Binary data to extract (bytes)
/// - `mime_type` (string): MIME type of the data
/// - `config_json` (string|null): JSON-encoded extraction configuration (uses defaults if null)
///
/// # Returns
///
/// ExtractionResult with content, metadata, and tables
///
/// # Throws
///
/// - ValidationException: Invalid configuration or unsupported format
/// - ParsingException: Document parsing failed
/// - OcrException: OCR processing failed
/// - Exception: Runtime errors
///
/// # Example
///
/// ```php
/// $data = file_get_contents("document.pdf");
/// $result = kreuzberg_extract_bytes($data, "application/pdf");
/// echo $result->content;
///
/// // With custom configuration
/// $config = new \Kreuzberg\Config\ExtractionConfig(forceOcr: true);
/// $result = kreuzberg_extract_bytes($data, "application/pdf", $config->toJson());
/// ```
#[php_function]
pub fn kreuzberg_extract_bytes(
    data: BinarySlice<u8>,
    mime_type: String,
    config_json: Option<String>,
) -> PhpResult<ExtractionResult> {
    let rust_config = match &config_json {
        Some(json) => parse_config_from_json(json).map_err(PhpException::from)?,
        None => Default::default(),
    };

    if crate::plugins::has_custom_extractor(&mime_type) {
        match crate::plugins::call_custom_extractor(&mime_type, data.as_ref()) {
            Ok(result_zval) => {
                if let Some(result_array) = result_zval.array() {
                    let content = result_array
                        .get("content")
                        .and_then(|v| v.str())
                        .ok_or_else(|| PhpException::default("Custom extractor result missing 'content'".to_string()))?
                        .to_string();

                    let metadata = if let Some(meta_val) = result_array.get("metadata") {
                        if let Some(meta_arr) = meta_val.array() {
                            let mut additional = ahash::AHashMap::new();
                            for (key, val) in meta_arr.iter() {
                                let key_str = format!("{}", key);
                                if let Ok(json_val) = crate::types::php_zval_to_json_value(val) {
                                    additional.insert(std::borrow::Cow::Owned(key_str), json_val);
                                }
                            }
                            kreuzberg::types::Metadata {
                                additional,
                                ..Default::default()
                            }
                        } else {
                            Default::default()
                        }
                    } else {
                        Default::default()
                    };

                    let tables = if let Some(tables_val) = result_array.get("tables") {
                        if let Some(tables_arr) = tables_val.array() {
                            tables_arr
                                .iter()
                                .filter_map(|(_, t)| t.array().and_then(|a| crate::types::php_array_to_table(a).ok()))
                                .collect()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };

                    let rust_result = kreuzberg::types::ExtractionResult {
                        content,
                        mime_type: std::borrow::Cow::Owned(mime_type.clone()),
                        metadata,
                        tables,
                        detected_languages: None,
                        chunks: None,
                        images: None,
                        pages: None,
                        elements: None,
                        djot_content: None,
                        ocr_elements: None,
                        document: None,
                        extracted_keywords: None,
                        quality_score: None,
                        processing_warnings: vec![],
                        annotations: None,
                        children: None,
                        formatted_content: None,
                    };

                    return ExtractionResult::from_rust(rust_result);
                }
            }
            Err(e) => {
                eprintln!("Custom extractor failed for '{}': {:?}", mime_type, e);
            }
        }
    }

    // Check if tables should be extracted from config
    let extract_tables = should_extract_tables(&config_json)?;

    let result = kreuzberg::extract_bytes_sync(data.as_ref(), &mime_type, &rust_config).map_err(to_php_exception)?;

    ExtractionResult::from_rust_with_config(result, extract_tables)
}

/// Batch extract content from multiple files.
///
/// MIME types are auto-detected for each file.
///
/// # Parameters
///
/// - `paths` (array): Array of file paths (strings)
/// - `config_json` (string|null): JSON-encoded extraction configuration (uses defaults if null)
/// - `file_configs_json` (array|null): Array of JSON-encoded per-file configs (string|null per element)
///
/// # Returns
///
/// Array of ExtractionResult objects (one per file)
///
/// # Throws
///
/// - ValidationException: Invalid configuration or list length mismatch
/// - ParsingException: Document parsing failed
/// - Exception: File access errors or runtime errors
///
/// # Example
///
/// ```php
/// $paths = ["doc1.pdf", "doc2.docx", "doc3.txt"];
/// $results = kreuzberg_batch_extract_files($paths);
///
/// foreach ($results as $i => $result) {
///     echo "Document {$i}: {$result->mime_type}\n";
///     echo substr($result->content, 0, 100) . "...\n";
/// }
///
/// // With custom configuration
/// $config = new \Kreuzberg\Config\ExtractionConfig(useCache: false);
/// $results = kreuzberg_batch_extract_files($paths, $config->toJson());
///
/// // With per-file config overrides
/// $file_configs = ['{"force_ocr": true}', null, null];
/// $results = kreuzberg_batch_extract_files($paths, null, $file_configs);
/// ```
#[php_function]
pub fn kreuzberg_batch_extract_files(
    paths: Vec<String>,
    config_json: Option<String>,
    file_configs_json: Option<Vec<Option<String>>>,
) -> PhpResult<Vec<ExtractionResult>> {
    let rust_config = match &config_json {
        Some(json) => parse_config_from_json(json).map_err(PhpException::from)?,
        None => Default::default(),
    };

    // Check if tables should be extracted from config
    let extract_tables = should_extract_tables(&config_json)?;

    let items: Vec<(std::path::PathBuf, Option<kreuzberg::FileExtractionConfig>)> = match file_configs_json {
        Some(fc_list) => {
            if paths.len() != fc_list.len() {
                return Err(format!(
                    "paths and file_configs_json must have the same length (got {} and {})",
                    paths.len(),
                    fc_list.len()
                )
                .into());
            }
            paths
                .into_iter()
                .zip(fc_list)
                .map(|(path, fc_json)| {
                    let fc = parse_file_config_from_json(&fc_json).map_err(PhpException::from)?;
                    Ok((std::path::PathBuf::from(path), fc))
                })
                .collect::<PhpResult<Vec<_>>>()?
        }
        None => paths.into_iter().map(|p| (std::path::PathBuf::from(p), None)).collect(),
    };

    let results = kreuzberg::batch_extract_file_sync(items, &rust_config).map_err(to_php_exception)?;

    results
        .into_iter()
        .map(|r| ExtractionResult::from_rust_with_config(r, extract_tables))
        .collect()
}

/// Batch extract content from multiple byte arrays.
///
/// # Parameters
///
/// - `data_list` (array): Array of binary data (bytes)
/// - `mime_types` (array): Array of MIME types (one per data element)
/// - `config_json` (string|null): JSON-encoded extraction configuration (uses defaults if null)
/// - `file_configs_json` (array|null): Array of JSON-encoded per-file configs (string|null per element)
///
/// # Returns
///
/// Array of ExtractionResult objects (one per data element)
///
/// # Throws
///
/// - ValidationException: Invalid configuration or list length mismatch
/// - ParsingException: Document parsing failed
/// - Exception: Runtime errors
///
/// # Example
///
/// ```php
/// $data1 = file_get_contents("doc1.pdf");
/// $data2 = file_get_contents("doc2.pdf");
/// $data_list = [$data1, $data2];
/// $mime_types = ["application/pdf", "application/pdf"];
///
/// $results = kreuzberg_batch_extract_bytes($data_list, $mime_types);
///
/// // With per-file config overrides
/// $file_configs = ['{"force_ocr": true}', null];
/// $results = kreuzberg_batch_extract_bytes($data_list, $mime_types, null, $file_configs);
/// ```
#[php_function]
pub fn kreuzberg_batch_extract_bytes(
    data_list: Vec<BinarySlice<u8>>,
    mime_types: Vec<String>,
    config_json: Option<String>,
    file_configs_json: Option<Vec<Option<String>>>,
) -> PhpResult<Vec<ExtractionResult>> {
    if data_list.len() != mime_types.len() {
        return Err(format!(
            "data_list and mime_types must have the same length (got {} and {})",
            data_list.len(),
            mime_types.len()
        )
        .into());
    }

    let rust_config = match &config_json {
        Some(json) => parse_config_from_json(json).map_err(PhpException::from)?,
        None => Default::default(),
    };

    // Check if tables should be extracted from config
    let extract_tables = should_extract_tables(&config_json)?;

    let items: Vec<(Vec<u8>, String, Option<kreuzberg::FileExtractionConfig>)> = match file_configs_json {
        Some(fc_list) => {
            if data_list.len() != fc_list.len() {
                return Err(format!(
                    "data_list and file_configs_json must have the same length (got {} and {})",
                    data_list.len(),
                    fc_list.len()
                )
                .into());
            }
            data_list
                .into_iter()
                .zip(mime_types)
                .zip(fc_list)
                .map(|((binary_slice, mime), fc_json)| {
                    let fc = parse_file_config_from_json(&fc_json).map_err(PhpException::from)?;
                    let bytes: &[u8] = binary_slice.as_ref();
                    Ok((bytes.to_vec(), mime, fc))
                })
                .collect::<PhpResult<Vec<_>>>()?
        }
        None => data_list
            .into_iter()
            .zip(mime_types)
            .map(|(binary_slice, mime)| {
                let bytes: &[u8] = binary_slice.as_ref();
                (bytes.to_vec(), mime, None)
            })
            .collect(),
    };

    let results = kreuzberg::batch_extract_bytes_sync(items, &rust_config).map_err(to_php_exception)?;

    results
        .into_iter()
        .map(|r| ExtractionResult::from_rust_with_config(r, extract_tables))
        .collect()
}

/// Detect MIME type from file bytes.
///
/// # Parameters
///
/// - `data` (string): File content as bytes
///
/// # Returns
///
/// Detected MIME type string (e.g., "application/pdf", "image/png")
///
/// # Example
///
/// ```php
/// $data = file_get_contents("unknown_file");
/// $mime_type = kreuzberg_detect_mime_type_from_bytes($data);
/// echo "Detected type: $mime_type\n";
/// ```
#[php_function]
pub fn kreuzberg_detect_mime_type_from_bytes(data: BinarySlice<u8>) -> PhpResult<String> {
    kreuzberg::detect_mime_type_from_bytes(data.as_ref())
        .map_err(|e| format!("Failed to detect MIME type: {}", e).into())
}

/// Detect MIME type from file bytes (alias for kreuzberg_detect_mime_type_from_bytes).
///
/// # Parameters
///
/// - `data` (string): File content as bytes
///
/// # Returns
///
/// Detected MIME type string (e.g., "application/pdf", "image/png")
///
/// # Example
///
/// ```php
/// $data = file_get_contents("unknown_file");
/// $mime_type = kreuzberg_detect_mime_type($data);
/// echo "Detected type: $mime_type\n";
/// ```
#[php_function]
pub fn kreuzberg_detect_mime_type(data: BinarySlice<u8>) -> PhpResult<String> {
    kreuzberg::detect_mime_type_from_bytes(data.as_ref())
        .map_err(|e| format!("Failed to detect MIME type: {}", e).into())
}

/// Detect MIME type from file path.
///
/// # Parameters
///
/// - `path` (string): Path to the file
///
/// # Returns
///
/// Detected MIME type string (e.g., "application/pdf", "text/plain")
///
/// # Example
///
/// ```php
/// $mime_type = kreuzberg_detect_mime_type_from_path("document.pdf");
/// echo "File type: $mime_type\n";
/// ```
#[php_function]
pub fn kreuzberg_detect_mime_type_from_path(path: String) -> PhpResult<String> {
    kreuzberg::detect_mime_type(&path, true).map_err(|e| format!("Failed to detect MIME type: {}", e).into())
}

/// Validate and normalize a MIME type.
///
/// # Parameters
///
/// - `mime_type` (string): MIME type to validate
///
/// # Returns
///
/// Normalized MIME type string
///
/// # Throws
///
/// - Exception: If the MIME type is not supported
///
/// # Example
///
/// ```php
/// try {
///     $normalized = kreuzberg_validate_mime_type("application/pdf");
///     echo "Valid MIME type: $normalized\n";
/// } catch (Exception $e) {
///     echo "Invalid MIME type: {$e->getMessage()}\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_validate_mime_type(mime_type: String) -> PhpResult<String> {
    kreuzberg::validate_mime_type(&mime_type).map_err(|e| format!("Invalid MIME type: {}", e).into())
}

/// Get file extensions for a MIME type.
///
/// # Parameters
///
/// - `mime_type` (string): MIME type (e.g., "application/pdf")
///
/// # Returns
///
/// Array of file extensions (e.g., ["pdf"])
///
/// # Example
///
/// ```php
/// $extensions = kreuzberg_get_extensions_for_mime("application/pdf");
/// print_r($extensions); // ["pdf"]
/// ```
#[php_function]
pub fn kreuzberg_get_extensions_for_mime(mime_type: String) -> PhpResult<Vec<String>> {
    kreuzberg::get_extensions_for_mime(&mime_type).map_err(|e| format!("Failed to get extensions: {}", e).into())
}

/// Render a single page of a PDF file to a PNG byte string.
///
/// # Parameters
///
/// - `path` (string): Path to the PDF file
/// - `page_index` (int): Zero-based page index
/// - `dpi` (int|null): Optional DPI (default 150)
///
/// # Returns
///
/// Binary string containing PNG image data
///
/// # Throws
///
/// - Exception: If rendering fails
///
/// # Example
///
/// ```php
/// $png = kreuzberg_render_pdf_page("document.pdf", 0);
/// file_put_contents("page_0.png", $png);
/// ```
#[php_function]
pub fn kreuzberg_render_pdf_page(path: String, page_index: i64, dpi: Option<i64>) -> PhpResult<Vec<u8>> {
    if page_index < 0 {
        return Err(PhpException::default(
            "[Validation] page_index must be non-negative".to_string(),
        ));
    }
    let page_index = usize::try_from(page_index)
        .map_err(|_| PhpException::default("[Validation] page_index out of range".to_string()))?;
    let dpi_i32 = dpi
        .map(|d| i32::try_from(d).map_err(|_| PhpException::default("[Validation] dpi out of range".to_string())))
        .transpose()?;
    let pdf_bytes =
        std::fs::read(&path).map_err(|e| PhpException::default(format!("[IO] Failed to read file: {}", e)))?;
    kreuzberg::pdf::render_pdf_page_to_png(&pdf_bytes, page_index, dpi_i32, None)
        .map_err(|e| PhpException::default(format!("[Rendering] {}", e)))
}

/// Create a new PDF page iterator, returning an opaque handle as i64.
///
/// # Parameters
///
/// - `path` (string): Path to the PDF file
/// - `dpi` (int|null): Optional DPI (default 150)
///
/// # Returns
///
/// An opaque iterator handle (i64) for use with the other iterator functions.
///
/// # Throws
///
/// - Exception: If the file cannot be read or the PDF is invalid
#[php_function]
pub fn kreuzberg_pdf_page_iterator_new(path: String, dpi: Option<i64>) -> PhpResult<i64> {
    let dpi_i32 = dpi
        .map(|d| i32::try_from(d).map_err(|_| PhpException::default("[Validation] dpi out of range".to_string())))
        .transpose()?;
    let iter = kreuzberg::pdf::PdfPageIterator::from_file(&path, dpi_i32, None)
        .map_err(|e| PhpException::default(format!("[Rendering] {}", e)))?;
    let boxed = Box::new(iter);
    Ok(Box::into_raw(boxed) as i64)
}

/// Advance the iterator and return the next page's PNG bytes, or None when exhausted.
///
/// # Parameters
///
/// - `handle` (int): Opaque iterator handle from `kreuzberg_pdf_page_iterator_new`
///
/// # Returns
///
/// PNG bytes for the next page, or null when all pages have been rendered.
///
/// # Throws
///
/// - Exception: If rendering a page fails
#[php_function]
pub fn kreuzberg_pdf_page_iterator_next(handle: i64) -> PhpResult<Option<Vec<u8>>> {
    if handle == 0 {
        return Err(PhpException::default(
            "[Validation] Invalid iterator handle".to_string(),
        ));
    }
    let iter = unsafe { &mut *(handle as *mut kreuzberg::pdf::PdfPageIterator) };
    match iter.next() {
        Some(Ok((_page_index, png))) => Ok(Some(png)),
        Some(Err(e)) => Err(PhpException::default(format!("[Rendering] {}", e))),
        None => Ok(None),
    }
}

/// Free the PDF page iterator. Safe to call once; do not reuse the handle afterwards.
///
/// # Parameters
///
/// - `handle` (int): Opaque iterator handle from `kreuzberg_pdf_page_iterator_new`
#[php_function]
pub fn kreuzberg_pdf_page_iterator_free(handle: i64) -> PhpResult<()> {
    if handle == 0 {
        return Ok(());
    }
    let _ = unsafe { Box::from_raw(handle as *mut kreuzberg::pdf::PdfPageIterator) };
    Ok(())
}

/// Return the total number of pages in the PDF behind the iterator.
///
/// # Parameters
///
/// - `handle` (int): Opaque iterator handle from `kreuzberg_pdf_page_iterator_new`
///
/// # Returns
///
/// Page count as i64.
#[php_function]
pub fn kreuzberg_pdf_page_iterator_page_count(handle: i64) -> PhpResult<i64> {
    if handle == 0 {
        return Err(PhpException::default(
            "[Validation] Invalid iterator handle".to_string(),
        ));
    }
    let iter = unsafe { &*(handle as *const kreuzberg::pdf::PdfPageIterator) };
    Ok(iter.page_count() as i64)
}

/// Returns all function builders for the extraction module.
pub fn get_function_builders() -> Vec<ext_php_rs::builders::FunctionBuilder<'static>> {
    vec![
        wrap_function!(kreuzberg_extract_file),
        wrap_function!(kreuzberg_extract_bytes),
        wrap_function!(kreuzberg_batch_extract_files),
        wrap_function!(kreuzberg_batch_extract_bytes),
        wrap_function!(kreuzberg_detect_mime_type_from_bytes),
        wrap_function!(kreuzberg_detect_mime_type),
        wrap_function!(kreuzberg_detect_mime_type_from_path),
        wrap_function!(kreuzberg_validate_mime_type),
        wrap_function!(kreuzberg_get_extensions_for_mime),
        wrap_function!(kreuzberg_render_pdf_page),
        wrap_function!(kreuzberg_pdf_page_iterator_new),
        wrap_function!(kreuzberg_pdf_page_iterator_next),
        wrap_function!(kreuzberg_pdf_page_iterator_free),
        wrap_function!(kreuzberg_pdf_page_iterator_page_count),
    ]
}
