//! Extraction NIFs
//!
//! This module provides Native Implemented Functions (NIFs) for document extraction,
//! including single file/bytes extraction and batch operations.

use crate::atoms;
use crate::config::parse_extraction_config;
use crate::conversion::convert_extraction_result_to_term;
use rustler::{Binary, Encoder, Env, NifResult, Term};

// Constants for validation
const MAX_BINARY_SIZE: usize = 500 * 1024 * 1024; // 500MB

/// Extract text and data from a document binary with default configuration
///
/// # Arguments
/// * `input` - Binary containing the document data
/// * `mime_type` - String representing the MIME type (e.g., "application/pdf")
///
/// # Returns
/// * `{:ok, result_map}` - Map containing extraction results
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
pub fn extract<'a>(env: Env<'a>, input: Binary<'a>, mime_type: String) -> NifResult<Term<'a>> {
    // Validate input
    if input.is_empty() {
        return Ok((atoms::error(), "Binary input cannot be empty").encode(env));
    }

    if input.len() > MAX_BINARY_SIZE {
        return Ok((atoms::error(), "Binary input exceeds maximum size of 500MB").encode(env));
    }

    // Create default extraction config
    let config = kreuzberg::core::config::ExtractionConfig::default();

    // Call kreuzberg extraction with default config
    match kreuzberg::extract_bytes_sync(input.as_slice(), &mime_type, &config) {
        Ok(result) => {
            // Convert ExtractionResult to Elixir term
            match convert_extraction_result_to_term(env, &result) {
                Ok(term) => Ok((atoms::ok(), term).encode(env)),
                Err(e) => Ok((atoms::error(), format!("Failed to encode result: {}", e)).encode(env)),
            }
        }
        Err(e) => Ok((atoms::error(), format!("Extraction failed: {}", e)).encode(env)),
    }
}

/// Extract text and data from a document binary with custom configuration
///
/// # Arguments
/// * `input` - Binary containing the document data
/// * `mime_type` - String representing the MIME type (e.g., "application/pdf")
/// * `options` - Term containing extraction options (as map or keyword list)
///
/// # Returns
/// * `{:ok, result_map}` - Map containing extraction results
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
pub fn extract_with_options<'a>(
    env: Env<'a>,
    input: Binary<'a>,
    mime_type: String,
    options: Term<'a>,
) -> NifResult<Term<'a>> {
    // Validate input
    if input.is_empty() {
        return Ok((atoms::error(), "Binary input cannot be empty").encode(env));
    }

    if input.len() > MAX_BINARY_SIZE {
        return Ok((atoms::error(), "Binary input exceeds maximum size of 500MB").encode(env));
    }

    // Parse options from Elixir term to ExtractionConfig
    let config = match parse_extraction_config(env, options) {
        Ok(cfg) => cfg,
        Err(e) => return Ok((atoms::error(), format!("Invalid options: {}", e)).encode(env)),
    };

    // Call kreuzberg extraction with parsed config
    match kreuzberg::extract_bytes_sync(input.as_slice(), &mime_type, &config) {
        Ok(result) => {
            // Convert ExtractionResult to Elixir term
            match convert_extraction_result_to_term(env, &result) {
                Ok(term) => Ok((atoms::ok(), term).encode(env)),
                Err(e) => Ok((atoms::error(), format!("Failed to encode result: {}", e)).encode(env)),
            }
        }
        Err(e) => Ok((atoms::error(), format!("Extraction failed: {}", e)).encode(env)),
    }
}

/// Extract text and data from a file at the given path with default configuration
///
/// # Arguments
/// * `path` - String containing the file path
/// * `mime_type` - Optional string representing the MIME type; if None, MIME type is detected from file
///
/// # Returns
/// * `{:ok, result_map}` - Map containing extraction results
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
pub fn extract_file<'a>(env: Env<'a>, path: String, mime_type: Option<String>) -> NifResult<Term<'a>> {
    // Create default extraction config
    let config = kreuzberg::core::config::ExtractionConfig::default();

    // Call kreuzberg file extraction with default config
    match kreuzberg::extract_file_sync(&path, mime_type.as_deref(), &config) {
        Ok(result) => {
            // Convert ExtractionResult to Elixir term
            match convert_extraction_result_to_term(env, &result) {
                Ok(term) => Ok((atoms::ok(), term).encode(env)),
                Err(e) => Ok((atoms::error(), format!("Failed to encode result: {}", e)).encode(env)),
            }
        }
        Err(e) => Ok((atoms::error(), format!("Extraction failed: {}", e)).encode(env)),
    }
}

/// Extract text and data from a file at the given path with custom configuration
///
/// # Arguments
/// * `path` - String containing the file path
/// * `mime_type` - Optional string representing the MIME type; if None, MIME type is detected from file
/// * `options` - Term containing extraction options (as map or keyword list)
///
/// # Returns
/// * `{:ok, result_map}` - Map containing extraction results
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
pub fn extract_file_with_options<'a>(
    env: Env<'a>,
    path: String,
    mime_type: Option<String>,
    options_term: Term<'a>,
) -> NifResult<Term<'a>> {
    // Parse options from Elixir term to ExtractionConfig
    let config = match parse_extraction_config(env, options_term) {
        Ok(cfg) => cfg,
        Err(e) => return Ok((atoms::error(), format!("Invalid options: {}", e)).encode(env)),
    };

    // Call kreuzberg file extraction with parsed config
    match kreuzberg::extract_file_sync(&path, mime_type.as_deref(), &config) {
        Ok(result) => {
            // Convert ExtractionResult to Elixir term
            match convert_extraction_result_to_term(env, &result) {
                Ok(term) => Ok((atoms::ok(), term).encode(env)),
                Err(e) => Ok((atoms::error(), format!("Failed to encode result: {}", e)).encode(env)),
            }
        }
        Err(e) => Ok((atoms::error(), format!("Extraction failed: {}", e)).encode(env)),
    }
}

/// Render all pages of a PDF binary to PNG byte buffers
///
/// # Arguments
/// * `input` - Binary containing the PDF data
/// * `dpi` - Optional DPI (default 150)
///
/// # Returns
/// * `{:ok, [binary]}` - List of PNG binaries, one per page
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
pub fn render_pdf_pages<'a>(env: Env<'a>, input: Binary<'a>, dpi: Option<i32>) -> NifResult<Term<'a>> {
    if input.is_empty() {
        return Ok((atoms::error(), "Binary input cannot be empty").encode(env));
    }

    match kreuzberg::pdf::render_pdf_to_png_pages(input.as_slice(), dpi, None) {
        Ok(pages) => {
            let binaries: Vec<_> = pages
                .iter()
                .map(|png| {
                    let mut obin = rustler::OwnedBinary::new(png.len()).unwrap();
                    obin.as_mut_slice().copy_from_slice(png);
                    obin.release(env)
                })
                .collect();
            Ok((atoms::ok(), binaries).encode(env))
        }
        Err(e) => Ok((atoms::error(), format!("Rendering failed: {}", e)).encode(env)),
    }
}

/// Render a single page of a PDF binary to a PNG byte buffer
///
/// # Arguments
/// * `input` - Binary containing the PDF data
/// * `page_index` - Zero-based page index
/// * `dpi` - Optional DPI (default 150)
///
/// # Returns
/// * `{:ok, binary}` - PNG binary
/// * `{:error, reason}` - Error tuple with reason string
#[rustler::nif(schedule = "DirtyCpu")]
pub fn render_pdf_page<'a>(
    env: Env<'a>,
    input: Binary<'a>,
    page_index: usize,
    dpi: Option<i32>,
) -> NifResult<Term<'a>> {
    if input.is_empty() {
        return Ok((atoms::error(), "Binary input cannot be empty").encode(env));
    }

    match kreuzberg::pdf::render_pdf_page_to_png(input.as_slice(), page_index, dpi, None) {
        Ok(png) => {
            let mut obin = rustler::OwnedBinary::new(png.len()).unwrap();
            obin.as_mut_slice().copy_from_slice(&png);
            Ok((atoms::ok(), obin.release(env)).encode(env))
        }
        Err(e) => Ok((atoms::error(), format!("Rendering failed: {}", e)).encode(env)),
    }
}
