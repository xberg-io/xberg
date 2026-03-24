use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::WORKER_POOL;
use crate::config::JsExtractionConfig;
use crate::error_handling::convert_error;
use crate::result::{JsExtractionResult, resolve_config};

#[napi]
pub fn extract_file_sync(
    file_path: String,
    mime_type: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;

    kreuzberg::extract_file_sync(&file_path, mime_type.as_deref(), &rust_config)
        .map_err(convert_error)
        .and_then(JsExtractionResult::try_from)
}

/// Extract content from a file (asynchronous).
///
/// Asynchronously extracts text, tables, images, and metadata from a document file.
/// Non-blocking alternative to `extractFileSync` for use in async/await contexts.
///
/// # Parameters
///
/// * `file_path` - Path to the file to extract (absolute or relative)
/// * `mime_type` - Optional MIME type hint (auto-detected if omitted)
/// * `config` - Optional extraction configuration (OCR, chunking, etc.)
///
/// # Returns
///
/// Promise resolving to `ExtractionResult` with extracted content and metadata.
///
/// # Errors
///
/// Rejects if file processing fails (see `extractFileSync` for error conditions).
///
/// # Example
///
/// ```typescript
/// import { extractFile } from '@kreuzberg/node';
///
/// // Async/await usage
/// const result = await extractFile('document.pdf', null, null);
/// console.log(result.content);
///
/// // Promise usage
/// extractFile('report.docx', null, null)
///   .then(result => console.log(result.content))
///   .catch(err => console.error(err));
/// ```
#[napi]
pub async fn extract_file(
    file_path: String,
    mime_type: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;

    let result = WORKER_POOL
        .spawn_blocking(move || kreuzberg::extract_file_sync(&file_path, mime_type.as_deref(), &rust_config))
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    JsExtractionResult::try_from(result)
}

/// Extract content from bytes (synchronous).
///
/// Synchronously extracts content from a byte buffer without requiring a file path.
/// Useful for processing in-memory data, network streams, or database BLOBs.
///
/// # Parameters
///
/// * `data` - Buffer containing the document bytes
/// * `mime_type` - MIME type of the data (e.g., "application/pdf", "image/png")
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// `ExtractionResult` with extracted content and metadata.
///
/// # Errors
///
/// Throws an error if data is malformed or MIME type is unsupported.
///
/// # Example
///
/// ```typescript
/// import { extractBytesSync } from '@kreuzberg/node';
/// import fs from 'fs';
///
/// const buffer = fs.readFileSync('document.pdf');
/// const result = extractBytesSync(buffer, 'application/pdf', null);
/// console.log(result.content);
/// ```
#[napi]
pub fn extract_bytes_sync(
    data: Buffer,
    mime_type: String,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;

    let bytes = data.as_ref();

    kreuzberg::extract_bytes_sync(bytes, &mime_type, &rust_config)
        .map_err(convert_error)
        .and_then(JsExtractionResult::try_from)
}

/// Extract content from bytes (asynchronous).
///
/// Asynchronously extracts content from a byte buffer. Non-blocking alternative
/// to `extractBytesSync` for processing in-memory data.
///
/// # Parameters
///
/// * `data` - Buffer containing the document bytes
/// * `mime_type` - MIME type of the data
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// Promise resolving to `ExtractionResult`.
///
/// # Example
///
/// ```typescript
/// import { extractBytes } from '@kreuzberg/node';
///
/// const response = await fetch('https://example.com/document.pdf');
/// const buffer = Buffer.from(await response.arrayBuffer());
/// const result = await extractBytes(buffer, 'application/pdf', null);
/// ```
#[napi]
pub async fn extract_bytes(
    data: Buffer,
    mime_type: String,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;
    let data_vec = data.to_vec();

    let result = WORKER_POOL
        .spawn_blocking(move || kreuzberg::extract_bytes_sync(&data_vec, &mime_type, &rust_config))
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    JsExtractionResult::try_from(result)
}

/// Render all pages of a PDF file to PNG buffers (synchronous).
///
/// # Parameters
///
/// * `file_path` - Path to the PDF file
/// * `dpi` - Optional DPI (default 150)
///
/// # Returns
///
/// Array of Buffer objects, one PNG per page.
#[napi]
pub fn render_pdf_pages_sync(file_path: String, dpi: Option<i32>) -> Result<Vec<Buffer>> {
    let pdf_bytes = std::fs::read(&file_path).map_err(|e| Error::from_reason(format!("Failed to read file: {}", e)))?;
    let pages = kreuzberg::pdf::render_pdf_to_png_pages(&pdf_bytes, dpi, None)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(pages.into_iter().map(|p| Buffer::from(p.as_slice())).collect())
}

/// Render a single page of a PDF file to a PNG buffer (synchronous).
///
/// # Parameters
///
/// * `file_path` - Path to the PDF file
/// * `page_index` - Zero-based page index
/// * `dpi` - Optional DPI (default 150)
///
/// # Returns
///
/// Buffer containing PNG image data.
#[napi]
pub fn render_pdf_page_sync(file_path: String, page_index: u32, dpi: Option<i32>) -> Result<Buffer> {
    let pdf_bytes = std::fs::read(&file_path).map_err(|e| Error::from_reason(format!("Failed to read file: {}", e)))?;
    let page = kreuzberg::pdf::render_pdf_page_to_png(&pdf_bytes, page_index as usize, dpi, None)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(Buffer::from(page.as_slice()))
}

/// Render all pages of a PDF file to PNG buffers (asynchronous).
///
/// # Parameters
///
/// * `file_path` - Path to the PDF file
/// * `dpi` - Optional DPI (default 150)
///
/// # Returns
///
/// Promise resolving to an array of Buffer objects.
#[napi]
pub async fn render_pdf_pages(file_path: String, dpi: Option<i32>) -> Result<Vec<Buffer>> {
    let result = WORKER_POOL
        .spawn_blocking(move || {
            let pdf_bytes =
                std::fs::read(&file_path).map_err(|e| kreuzberg::KreuzbergError::Io(std::sync::Arc::new(e)))?;
            kreuzberg::pdf::render_pdf_to_png_pages(&pdf_bytes, dpi, None)
                .map_err(|e| kreuzberg::KreuzbergError::Other(e.to_string()))
        })
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    Ok(result.into_iter().map(|p| Buffer::from(p.as_slice())).collect())
}
