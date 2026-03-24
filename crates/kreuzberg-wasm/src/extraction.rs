//! Document extraction functionality for WASM
//!
//! This module provides functions for extracting content from various document formats
//! in WebAssembly environments. Supports both synchronous and asynchronous extraction
//! from byte arrays and web-accessible files.

use crate::errors::convert_error;
use crate::types::{parse_config, result_to_js_value, results_to_js_value};
use js_sys::Uint8Array;
use kreuzberg::{
    FileExtractionConfig, batch_extract_bytes_sync, extract_bytes, extract_bytes_sync, utils::camel_to_snake,
};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

type BatchItem = (Vec<u8>, String, Option<FileExtractionConfig>);
use web_sys::{File, FileReader};

/// Extract content from a byte array (synchronous).
///
/// Extracts text, tables, images, and metadata from a document represented as bytes.
/// This is a synchronous, blocking operation suitable for smaller documents or when
/// async execution is not available.
///
/// # JavaScript Parameters
///
/// * `data: Uint8Array` - The document bytes to extract
/// * `mimeType: string` - MIME type of the data (e.g., "application/pdf", "image/png")
/// * `config?: object` - Optional extraction configuration
///
/// # Returns
///
/// `object` - ExtractionResult with extracted content and metadata
///
/// # Throws
///
/// Throws an error if data is malformed or MIME type is unsupported.
///
/// # Example
///
/// ```javascript
/// import { extractBytesSync } from '@kreuzberg/wasm';
/// import { readFileSync } from 'fs';
///
/// const buffer = readFileSync('document.pdf');
/// const data = new Uint8Array(buffer);
/// const result = extractBytesSync(data, 'application/pdf', null);
/// console.log(result.content);
/// ```
#[wasm_bindgen(js_name = extractBytesSync)]
pub fn extract_bytes_sync_wasm(
    data: Uint8Array,
    mime_type: String,
    config: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    let extraction_config = parse_config(config)?;
    let bytes = data.to_vec();

    extract_bytes_sync(&bytes, &mime_type, &extraction_config)
        .map_err(convert_error)
        .and_then(|result| result_to_js_value(&result))
}

/// Extract content from a byte array (asynchronous).
///
/// Asynchronously extracts text, tables, images, and metadata from a document.
/// Non-blocking alternative to `extractBytesSync` suitable for large documents
/// or browser environments.
///
/// # JavaScript Parameters
///
/// * `data: Uint8Array` - The document bytes to extract
/// * `mimeType: string` - MIME type of the data (e.g., "application/pdf")
/// * `config?: object` - Optional extraction configuration
///
/// # Returns
///
/// `Promise<object>` - Promise resolving to ExtractionResult
///
/// # Throws
///
/// Rejects if data is malformed or MIME type is unsupported.
///
/// # Example
///
/// ```javascript
/// import { extractBytes } from '@kreuzberg/wasm';
///
/// // Fetch from URL
/// const response = await fetch('document.pdf');
/// const arrayBuffer = await response.arrayBuffer();
/// const data = new Uint8Array(arrayBuffer);
///
/// const result = await extractBytes(data, 'application/pdf', null);
/// console.log(result.content.substring(0, 100));
/// ```
#[wasm_bindgen(js_name = extractBytes)]
pub fn extract_bytes_wasm(data: Uint8Array, mime_type: String, config: Option<JsValue>) -> js_sys::Promise {
    let bytes = data.to_vec();

    wasm_bindgen_futures::future_to_promise(async move {
        let extraction_config = parse_config(config)?;
        let result = extract_bytes(&bytes, &mime_type, &extraction_config)
            .await
            .map_err(convert_error)?;

        result_to_js_value(&result)
    })
}

/// Extract content from a web File or Blob (asynchronous).
///
/// Extracts content from a web File (from `<input type="file">`) or Blob object
/// using the FileReader API. Only available in browser environments (FileReader API limitation).
/// For server-side environments, use `extractBytes` with file data converted to Uint8Array.
///
/// # JavaScript Parameters
///
/// * `file: File | Blob` - The file or blob to extract
/// * `mimeType?: string` - Optional MIME type hint (auto-detected if omitted)
/// * `config?: object` - Optional extraction configuration
///
/// # Returns
///
/// `Promise<object>` - Promise resolving to ExtractionResult
///
/// # Throws
///
/// Rejects if file cannot be read or is malformed.
///
/// # Example
///
/// ```javascript
/// import { extractFile } from '@kreuzberg/wasm';
///
/// // From file input
/// const fileInput = document.getElementById('file-input');
/// const file = fileInput.files[0];
///
/// const result = await extractFile(file, null, null);
/// console.log(`Extracted ${result.content.length} characters`);
/// ```
#[wasm_bindgen(js_name = extractFile)]
pub fn extract_file_wasm(file: &web_sys::File, mime_type: Option<String>, config: Option<JsValue>) -> js_sys::Promise {
    // `file` is borrowed so it must be cloned to move into the async block;
    // `mime_type` and `config` are owned values that can be moved directly.
    let file_clone = file.clone();

    wasm_bindgen_futures::future_to_promise(async move {
        let bytes = read_file_as_array_buffer(&file_clone)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to read file: {}", e)))?;

        let extraction_config = parse_config(config)?;
        let mime = mime_type.unwrap_or_else(|| file_clone.type_());

        let result = extract_bytes(&bytes, &mime, &extraction_config)
            .await
            .map_err(convert_error)?;

        result_to_js_value(&result)
    })
}

/// Batch extract from multiple byte arrays (synchronous).
///
/// Processes multiple document byte arrays in parallel. All documents use the
/// same extraction configuration unless per-file configs are provided.
///
/// # JavaScript Parameters
///
/// * `dataList: Uint8Array[]` - Array of document bytes
/// * `mimeTypes: string[]` - Array of MIME types (must match dataList length)
/// * `config?: object` - Optional extraction configuration (applied to all)
/// * `fileConfigs?: (object | null)[]` - Optional per-file config overrides (must match dataList length if provided)
///
/// # Returns
///
/// `object[]` - Array of ExtractionResults in the same order as inputs
///
/// # Throws
///
/// Throws if dataList and mimeTypes lengths don't match, or if fileConfigs
/// is provided and its length doesn't match dataList.
///
/// # Example
///
/// ```javascript
/// import { batchExtractBytesSync } from '@kreuzberg/wasm';
///
/// const buffers = [buffer1, buffer2, buffer3];
/// const mimeTypes = ['application/pdf', 'text/plain', 'image/png'];
/// const results = batchExtractBytesSync(buffers, mimeTypes, null);
///
/// results.forEach((result, i) => {
///   console.log(`Document ${i}: ${result.content.substring(0, 50)}...`);
/// });
///
/// // With per-file configs:
/// const fileConfigs = [{ ocrConfig: { language: 'eng' } }, null, null];
/// const results2 = batchExtractBytesSync(buffers, mimeTypes, null, fileConfigs);
/// ```
#[wasm_bindgen(js_name = batchExtractBytesSync)]
pub fn batch_extract_bytes_sync_wasm(
    data_list: Vec<Uint8Array>,
    mime_types: Vec<String>,
    config: Option<JsValue>,
    file_configs: Option<Vec<JsValue>>,
) -> Result<JsValue, JsValue> {
    if data_list.len() != mime_types.len() {
        return Err(JsValue::from_str("data_list and mime_types must have the same length"));
    }

    let extraction_config = parse_config(config)?;
    let items = build_batch_items(data_list, mime_types, file_configs)?;

    let results = batch_extract_bytes_sync(items, &extraction_config).map_err(convert_error)?;

    results_to_js_value(&results)
}

/// Batch extract from multiple byte arrays (asynchronous).
///
/// Asynchronously processes multiple document byte arrays in parallel.
/// Non-blocking alternative to `batchExtractBytesSync`.
///
/// # JavaScript Parameters
///
/// * `dataList: Uint8Array[]` - Array of document bytes
/// * `mimeTypes: string[]` - Array of MIME types (must match dataList length)
/// * `config?: object` - Optional extraction configuration (applied to all)
/// * `fileConfigs?: (object | null)[]` - Optional per-file config overrides (must match dataList length if provided)
///
/// # Returns
///
/// `Promise<object[]>` - Promise resolving to array of ExtractionResults
///
/// # Throws
///
/// Rejects if dataList and mimeTypes lengths don't match, or if fileConfigs
/// is provided and its length doesn't match dataList.
///
/// # Example
///
/// ```javascript
/// import { batchExtractBytes } from '@kreuzberg/wasm';
///
/// const responses = await Promise.all([
///   fetch('doc1.pdf'),
///   fetch('doc2.docx')
/// ]);
///
/// const buffers = await Promise.all(
///   responses.map(r => r.arrayBuffer().then(b => new Uint8Array(b)))
/// );
///
/// const results = await batchExtractBytes(
///   buffers,
///   ['application/pdf', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'],
///   null
/// );
///
/// // With per-file configs:
/// const fileConfigs = [{ ocrConfig: { language: 'eng' } }, null];
/// const results2 = await batchExtractBytes(buffers, mimeTypes, null, fileConfigs);
/// ```
#[wasm_bindgen(js_name = batchExtractBytes)]
pub fn batch_extract_bytes_wasm(
    data_list: Vec<Uint8Array>,
    mime_types: Vec<String>,
    config: Option<JsValue>,
    file_configs: Option<Vec<JsValue>>,
) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        if data_list.len() != mime_types.len() {
            return Err(JsValue::from_str("data_list and mime_types must have the same length"));
        }

        let extraction_config = Arc::new(parse_config(config)?);
        let items = build_batch_items(data_list, mime_types, file_configs)?;

        let mut results = Vec::with_capacity(items.len());
        for (data, mime, file_config) in &items {
            // When there is a per-file override we must build a new config struct; otherwise
            // we pass a reference to the shared Arc so no full struct clone is needed.
            let effective_config;
            let config_ref = match file_config {
                Some(fc) => {
                    effective_config = extraction_config.with_file_overrides(fc);
                    &effective_config
                }
                None => &*extraction_config,
            };
            let result = extract_bytes(data.as_slice(), mime, config_ref)
                .await
                .map_err(convert_error)?;
            results.push(result);
        }

        results_to_js_value(&results)
    })
}

/// Batch extract from multiple Files or Blobs (asynchronous).
///
/// Processes multiple web File or Blob objects in parallel using the FileReader API.
/// Only available in browser environments (FileReader API limitation).
/// For server-side environments, use `batchExtractBytes` with file data converted to Uint8Array.
///
/// # JavaScript Parameters
///
/// * `files: (File | Blob)[]` - Array of files or blobs to extract
/// * `config?: object` - Optional extraction configuration (applied to all)
///
/// # Returns
///
/// `Promise<object[]>` - Promise resolving to array of ExtractionResults
///
/// # Example
///
/// ```javascript
/// import { batchExtractFiles } from '@kreuzberg/wasm';
///
/// // From file input with multiple files
/// const fileInput = document.getElementById('file-input');
/// const files = Array.from(fileInput.files);
///
/// const results = await batchExtractFiles(files, null);
/// console.log(`Processed ${results.length} files`);
/// ```
#[wasm_bindgen(js_name = batchExtractFiles)]
pub fn batch_extract_files_wasm(files: Vec<File>, config: Option<JsValue>) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        let extraction_config = parse_config(config)?;
        let mut results = Vec::with_capacity(files.len());

        for file in files {
            let bytes = read_file_as_array_buffer(&file)
                .await
                .map_err(|e| JsValue::from_str(&format!("Failed to read file: {}", e)))?;

            let mime = file.type_();
            let result = extract_bytes(&bytes, &mime, &extraction_config)
                .await
                .map_err(convert_error)?;

            results.push(result);
        }

        results_to_js_value(&results)
    })
}

/// Parse a JsValue (null/undefined or object) into an `Option<FileExtractionConfig>`.
fn parse_file_config(value: JsValue) -> Result<Option<FileExtractionConfig>, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }

    let json_value: serde_json::Value = serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse file config: {e}")))?;
    let snake_value = camel_to_snake(json_value);
    let fc: FileExtractionConfig = serde_json::from_value(snake_value)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse file config: {e}")))?;
    Ok(Some(fc))
}

/// Build batch items from parallel arrays, optionally zipping in per-file configs.
///
/// When `file_configs` is `None`, all items get `None` for their file config.
/// When provided, its length must match `data_list`.
fn build_batch_items(
    data_list: Vec<Uint8Array>,
    mime_types: Vec<String>,
    file_configs: Option<Vec<JsValue>>,
) -> Result<Vec<BatchItem>, JsValue> {
    match file_configs {
        Some(configs) => {
            if configs.len() != data_list.len() {
                return Err(JsValue::from_str(&format!(
                    "fileConfigs length ({}) must match dataList length ({})",
                    configs.len(),
                    data_list.len()
                )));
            }
            data_list
                .into_iter()
                .zip(mime_types)
                .zip(configs)
                .map(|((data, mime), fc_js)| {
                    let fc = parse_file_config(fc_js)?;
                    Ok((data.to_vec(), mime, fc))
                })
                .collect::<Result<Vec<_>, JsValue>>()
        }
        None => Ok(data_list
            .into_iter()
            .zip(mime_types)
            .map(|(data, mime)| (data.to_vec(), mime, None))
            .collect()),
    }
}

/// Extract content from a file (synchronous) - NOT AVAILABLE IN WASM.
///
/// File system operations are not available in WebAssembly environments.
/// Use `extractBytesSync` or `extractBytes` instead.
///
/// # Throws
///
/// Always throws: "File operations are not available in WASM. Use extractBytesSync or extractBytes instead."
#[wasm_bindgen(js_name = extractFileSync)]
pub fn extract_file_sync_wasm() -> Result<JsValue, JsValue> {
    Err(JsValue::from_str(
        "File operations are not available in WASM. Use extractBytesSync or extractBytes instead.",
    ))
}

/// Batch extract from multiple files (synchronous) - NOT AVAILABLE IN WASM.
///
/// File system operations are not available in WebAssembly environments.
/// Use `batchExtractBytesSync` or `batchExtractBytes` instead.
///
/// # Throws
///
/// Always throws: "File operations are not available in WASM. Use batchExtractBytesSync or batchExtractBytes instead."
#[wasm_bindgen(js_name = batchExtractFilesSync)]
pub fn batch_extract_files_sync_wasm() -> Result<JsValue, JsValue> {
    Err(JsValue::from_str(
        "File operations are not available in WASM. Use batchExtractBytesSync or batchExtractBytes instead.",
    ))
}

/// Helper function to read a File/Blob as bytes using FileReader API.
///
/// This is an internal helper that reads web File/Blob objects asynchronously.
async fn read_file_as_array_buffer(file: &web_sys::File) -> Result<Vec<u8>, String> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;

    let reader = FileReader::new().map_err(|_| "Failed to create FileReader".to_string())?;
    let reader = Rc::new(RefCell::new(reader));

    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let reader_clone = reader.clone();

        let onload = {
            let resolve_clone = resolve.clone();
            let reader_inner = reader_clone.clone();

            Closure::once(move |_: JsValue| {
                if let Ok(result) = reader_inner.borrow().result() {
                    let _ = resolve_clone.call1(&JsValue::undefined(), &result);
                }
            })
        };

        let reader_borrow = reader_clone.borrow_mut();
        let _ = reader_borrow.add_event_listener_with_callback("load", onload.as_ref().unchecked_ref());

        drop(reader_borrow);
        onload.forget();
    });

    reader
        .borrow_mut()
        .read_as_array_buffer(file)
        .map_err(|_| "Failed to read file".to_string())?;

    let array_buffer = JsFuture::from(promise)
        .await
        .map_err(|_| "File read failed".to_string())?;

    let arr = Uint8Array::new(&array_buffer);
    Ok(arr.to_vec())
}

/// Render all pages of a PDF to PNG byte buffers (synchronous).
///
/// # JavaScript Parameters
///
/// * `data: Uint8Array` - The PDF document bytes
/// * `dpi?: number` - Optional DPI (default 150)
///
/// # Returns
///
/// `Array<Uint8Array>` - Array of PNG images, one per page.
#[wasm_bindgen(js_name = renderPdfPagesSync)]
pub fn render_pdf_pages_sync_wasm(data: Uint8Array, dpi: Option<i32>) -> Result<JsValue, JsValue> {
    let bytes = data.to_vec();
    let pages = kreuzberg::pdf::render_pdf_to_png_pages(&bytes, dpi, None).map_err(convert_error)?;

    let js_array = js_sys::Array::new_with_length(pages.len() as u32);
    for (i, page) in pages.iter().enumerate() {
        let arr = Uint8Array::new_with_length(page.len() as u32);
        arr.copy_from(page);
        js_array.set(i as u32, arr.into());
    }
    Ok(js_array.into())
}

/// Render a single page of a PDF to a PNG byte buffer (synchronous).
///
/// # JavaScript Parameters
///
/// * `data: Uint8Array` - The PDF document bytes
/// * `pageIndex: number` - Zero-based page index
/// * `dpi?: number` - Optional DPI (default 150)
///
/// # Returns
///
/// `Uint8Array` - PNG image data.
#[wasm_bindgen(js_name = renderPdfPageSync)]
pub fn render_pdf_page_sync_wasm(data: Uint8Array, page_index: u32, dpi: Option<i32>) -> Result<Uint8Array, JsValue> {
    let bytes = data.to_vec();
    let page = kreuzberg::pdf::render_pdf_page_to_png(&bytes, page_index as usize, dpi, None).map_err(convert_error)?;

    let arr = Uint8Array::new_with_length(page.len() as u32);
    arr.copy_from(&page);
    Ok(arr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    const VALID_PDF_DATA: &[u8] = b"%PDF-1.4\n%test";
    const INVALID_DATA: &[u8] = b"some data";
    const EMPTY_DATA: &[u8] = b"";
    const TEXT_DATA: &[u8] = b"Hello, this is plain text content";

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_valid_pdf_data_returns_result() {
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = None;

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_invalid_mime_type_returns_error() {
        let data = unsafe { Uint8Array::view(INVALID_DATA) };
        let mime_type = "invalid/mime".to_string();
        let config = None;

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_empty_data_returns_error() {
        let data = unsafe { Uint8Array::view(EMPTY_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = None;

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_with_valid_config_returns_result() {
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = Some(JsValue::NULL);

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_text_plain_data_returns_result() {
        let data = unsafe { Uint8Array::view(TEXT_DATA) };
        let mime_type = "text/plain".to_string();
        let config = None;

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_wasm_returns_promise() {
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = None;

        let promise = extract_bytes_wasm(data, mime_type, config);

        assert!(!promise.is_null());
        assert!(!promise.is_undefined());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_wasm_invalid_mime_type_returns_promise() {
        let data = unsafe { Uint8Array::view(INVALID_DATA) };
        let mime_type = "invalid/type".to_string();
        let config = None;

        let promise = extract_bytes_wasm(data, mime_type, config);

        assert!(!promise.is_null());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_wasm_with_config_returns_promise() {
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = Some(JsValue::NULL);

        let promise = extract_bytes_wasm(data, mime_type, config);

        assert!(!promise.is_null());
    }

    #[wasm_bindgen_test]
    fn test_extract_file_sync_wasm_always_returns_error() {
        let result = extract_file_sync_wasm();

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{:?}", err);
        assert!(err_str.contains("not available") || err_str.contains("WASM"));
    }

    #[wasm_bindgen_test]
    fn test_extract_file_sync_wasm_error_message_is_descriptive() {
        let result = extract_file_sync_wasm();

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_files_sync_wasm_always_returns_error() {
        let result = batch_extract_files_sync_wasm();

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_files_sync_wasm_error_mentions_alternative() {
        let result = batch_extract_files_sync_wasm();

        assert!(result.is_err());
    }

    const PDF_DATA_1: &[u8] = b"%PDF-1.4\n%test1";
    const TEXT_CONTENT: &[u8] = b"Plain text content";

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_matching_lengths_returns_result() {
        let data1 = unsafe { Uint8Array::view(PDF_DATA_1) };
        let data2 = unsafe { Uint8Array::view(TEXT_CONTENT) };
        let data_list = vec![data1, data2];
        let mime_types = vec!["application/pdf".to_string(), "text/plain".to_string()];
        let config = None;

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config, None);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_mismatched_lengths_returns_error() {
        let data1 = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data1];
        let mime_types = vec!["application/pdf".to_string(), "text/plain".to_string()];
        let config = None;

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config, None);

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_empty_batch_returns_result() {
        let data_list: Vec<Uint8Array> = vec![];
        let mime_types: Vec<String> = vec![];
        let config = None;

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config, None);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_single_document_returns_result() {
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data];
        let mime_types = vec!["application/pdf".to_string()];
        let config = None;

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config, None);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_with_config_returns_result() {
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data];
        let mime_types = vec!["application/pdf".to_string()];
        let config = Some(JsValue::NULL);

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config, None);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_wasm_returns_promise() {
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data];
        let mime_types = vec!["application/pdf".to_string()];
        let config = None;

        let promise = batch_extract_bytes_wasm(data_list, mime_types, config, None);

        assert!(!promise.is_null());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_wasm_mismatched_lengths_returns_promise() {
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data];
        let mime_types = vec!["application/pdf".to_string(), "text/plain".to_string()];
        let config = None;

        let promise = batch_extract_bytes_wasm(data_list, mime_types, config, None);

        assert!(!promise.is_null());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_wasm_empty_batch_returns_promise() {
        let data_list: Vec<Uint8Array> = vec![];
        let mime_types: Vec<String> = vec![];
        let config = None;

        let promise = batch_extract_bytes_wasm(data_list, mime_types, config, None);

        assert!(!promise.is_null());
    }
}
