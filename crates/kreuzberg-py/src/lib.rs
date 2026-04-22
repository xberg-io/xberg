//! Kreuzberg PyO3 Bindings v4
//!
//! This module exposes the Rust core extraction API to Python with both
//! synchronous and asynchronous variants.
//!
//! # Architecture
//!
//! - All extraction logic is in the Rust core (crates/kreuzberg)
//! - Python is a thin wrapper that adds language-specific features
//! - Zero duplication of core functionality
//! - Modern PyO3 0.26 patterns throughout

#![deny(unsafe_code)]

use once_cell::sync::OnceCell;
use pyo3::prelude::*;
use pyo3_async_runtimes::TaskLocals;

mod cancellation;
mod config;
mod conversion;
mod core;
mod error;
mod ffi;
mod helpers;
mod html_options;
mod keywords;
mod plugins; // Now a module directory with submodules
mod types;
mod validation;

/// Global Python event loop task locals for async handlers
/// Initialized once at startup to avoid ~55µs overhead per call
static TASK_LOCALS: OnceCell<TaskLocals> = OnceCell::new();

/// Initialize Python event loop for async plugin callbacks
///
/// This should be called once after Python initialization to set up
/// the event loop that will be used for all async Python plugin calls.
/// Avoids ~55µs overhead of creating event loops per call.
///
/// Based on spikard's high-performance async patterns.
#[pyfunction]
fn init_async_runtime() -> PyResult<()> {
    Python::attach(|py| {
        let asyncio = py.import("asyncio")?;
        let event_loop = asyncio.call_method0("new_event_loop")?;
        asyncio.call_method1("set_event_loop", (event_loop.clone(),))?;

        TASK_LOCALS.get_or_init(|| TaskLocals::new(event_loop));

        Ok(())
    })
}

/// Internal bindings module for Kreuzberg
#[pymodule]
fn _internal_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // If ORT_DYLIB_PATH isn't set, try to discover the library provided by the
    // `onnxruntime` or `onnxruntime-gpu` pip packages.
    if std::env::var("ORT_DYLIB_PATH").is_err()
        && let Ok(onnx) = m.py().import("onnxruntime")
        && let Ok(paths) = onnx.getattr("__path__")
        && let Ok(paths_vec) = paths.extract::<Vec<String>>()
        && let Some(base_dir) = paths_vec.first()
    {
        let mut dylib_path = std::path::PathBuf::from(base_dir);
        dylib_path.push("capi");

        #[cfg(target_os = "windows")]
        dylib_path.push("onnxruntime.dll");
        #[cfg(target_os = "macos")]
        dylib_path.push("libonnxruntime.dylib");
        #[cfg(target_os = "linux")]
        dylib_path.push("libonnxruntime.so");

        if dylib_path.exists() {
            // SAFETY: Single-threaded during module initialization.
            #[allow(unsafe_code)]
            unsafe {
                std::env::set_var("ORT_DYLIB_PATH", dylib_path.to_string_lossy().as_ref());
            }
        }
    }

    kreuzberg::ort_discovery::ensure_ort_available();

    m.add("ValidationError", m.py().get_type::<error::ValidationError>())?;
    m.add("ParsingError", m.py().get_type::<error::ParsingError>())?;
    m.add("OCRError", m.py().get_type::<error::OCRError>())?;
    m.add(
        "MissingDependencyError",
        m.py().get_type::<error::MissingDependencyError>(),
    )?;
    m.add(
        "ExtractionTimeoutError",
        m.py().get_type::<error::ExtractionTimeoutError>(),
    )?;
    m.add(
        "ExtractionCancelledError",
        m.py().get_type::<error::ExtractionCancelledError>(),
    )?;
    m.add_class::<cancellation::PyCancellationToken>()?;

    m.add_class::<config::ExtractionConfig>()?;
    m.add_class::<config::OcrConfig>()?;
    m.add_class::<config::PdfConfig>()?;
    m.add_class::<config::PageConfig>()?;
    m.add_class::<config::ChunkingConfig>()?;
    m.add_class::<config::EmbeddingConfig>()?;
    m.add_class::<config::EmbeddingModelType>()?;
    m.add_class::<config::LanguageDetectionConfig>()?;
    m.add_class::<config::TokenReductionConfig>()?;
    m.add_class::<config::ImageExtractionConfig>()?;
    m.add_class::<config::PostProcessorConfig>()?;
    m.add_class::<config::TesseractConfig>()?;
    m.add_class::<config::ImagePreprocessingConfig>()?;
    m.add_class::<config::PyPaddleOcrConfig>()?;
    m.add_class::<config::HierarchyConfig>()?;
    m.add_class::<config::LayoutDetectionConfig>()?;
    m.add_class::<config::AccelerationConfig>()?;
    m.add_class::<config::EmailConfig>()?;
    m.add_class::<config::ConcurrencyConfig>()?;
    m.add_class::<config::ContentFilterConfig>()?;
    m.add_class::<config::TreeSitterProcessConfig>()?;
    m.add_class::<config::TreeSitterConfig>()?;
    m.add_class::<config::FileExtractionConfig>()?;
    m.add_class::<config::PyLlmConfig>()?;
    m.add_class::<config::PyStructuredExtractionConfig>()?;
    m.add_class::<config::HtmlOutputConfig>()?;

    m.add_class::<keywords::YakeParams>()?;
    m.add_class::<keywords::RakeParams>()?;
    m.add_class::<keywords::KeywordAlgorithm>()?;
    m.add_class::<keywords::KeywordConfig>()?;

    m.add_class::<types::ExtractionResult>()?;
    m.add_class::<types::PyChunk>()?;
    m.add_class::<types::ExtractedTable>()?;
    m.add_class::<types::PyExtractedKeyword>()?;
    m.add_class::<types::PyProcessingWarning>()?;
    m.add_class::<types::PyLlmUsage>()?;
    m.add_class::<types::PyPdfAnnotation>()?;

    m.add_function(wrap_pyfunction!(core::extract_file_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::extract_bytes_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_files_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_bytes_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::extract_file, m)?)?;
    m.add_function(wrap_pyfunction!(core::extract_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_files, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_bytes, m)?)?;

    m.add_function(wrap_pyfunction!(core::embed_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::embed, m)?)?;

    m.add_function(wrap_pyfunction!(core::render_pdf_page_impl, m)?)?;

    m.add_class::<core::PyPdfPageIterator>()?;

    m.add_function(wrap_pyfunction!(plugins::register_ocr_backend, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::unregister_ocr_backend, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::list_ocr_backends, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::clear_ocr_backends, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::register_post_processor, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::unregister_post_processor, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::clear_post_processors, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::list_post_processors, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::register_validator, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::unregister_validator, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::clear_validators, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::list_validators, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::list_document_extractors, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::unregister_document_extractor, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::clear_document_extractors, m)?)?;

    m.add_function(wrap_pyfunction!(init_async_runtime, m)?)?;

    m.add_class::<EmbeddingPreset>()?;
    m.add_function(wrap_pyfunction!(list_embedding_presets, m)?)?;
    m.add_function(wrap_pyfunction!(get_embedding_preset, m)?)?;

    m.add_function(wrap_pyfunction!(detect_mime_type_from_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(detect_mime_type_from_path, m)?)?;
    m.add_function(wrap_pyfunction!(validate_mime_type, m)?)?;
    m.add_function(wrap_pyfunction!(get_extensions_for_mime, m)?)?;
    m.add_function(wrap_pyfunction!(get_last_error_code, m)?)?;
    m.add_function(wrap_pyfunction!(get_last_panic_context, m)?)?;

    m.add_function(wrap_pyfunction!(validation::validate_binarization_method, m)?)?;
    m.add_function(wrap_pyfunction!(validation::validate_ocr_backend, m)?)?;
    m.add_function(wrap_pyfunction!(validation::validate_language_code, m)?)?;
    m.add_function(wrap_pyfunction!(validation::validate_token_reduction_level, m)?)?;
    m.add_function(wrap_pyfunction!(validation::validate_tesseract_psm, m)?)?;
    m.add_function(wrap_pyfunction!(validation::validate_tesseract_oem, m)?)?;
    m.add_function(wrap_pyfunction!(validation::validate_output_format, m)?)?;
    m.add_function(wrap_pyfunction!(validation::validate_confidence, m)?)?;
    m.add_function(wrap_pyfunction!(validation::validate_dpi, m)?)?;
    m.add_function(wrap_pyfunction!(validation::validate_chunking_params, m)?)?;
    m.add_function(wrap_pyfunction!(validation::get_valid_binarization_methods, m)?)?;
    m.add_function(wrap_pyfunction!(validation::get_valid_language_codes, m)?)?;
    m.add_function(wrap_pyfunction!(validation::get_valid_ocr_backends, m)?)?;
    m.add_function(wrap_pyfunction!(validation::get_valid_token_reduction_levels, m)?)?;

    m.add_function(wrap_pyfunction!(helpers::config_to_json, m)?)?;
    m.add_function(wrap_pyfunction!(helpers::config_get_field, m)?)?;
    m.add_function(wrap_pyfunction!(helpers::config_merge, m)?)?;
    m.add_function(wrap_pyfunction!(helpers::_discover_extraction_config_impl, m)?)?;
    m.add_function(wrap_pyfunction!(helpers::_load_extraction_config_from_file_impl, m)?)?;

    m.add_function(wrap_pyfunction!(ffi::get_error_details, m)?)?;
    m.add_function(wrap_pyfunction!(ffi::classify_error, m)?)?;
    m.add_function(wrap_pyfunction!(ffi::error_code_name, m)?)?;

    Ok(())
}

/// Embedding preset configuration.
///
/// Contains all settings for a specific embedding model preset including chunk size,
/// overlap, model name, embedding dimensions, and description.
///
/// Attributes:
///     name (str): Name of the preset (e.g., "fast", "balanced", "quality", "multilingual")
///     chunk_size (int): Recommended chunk size in characters
///     overlap (int): Recommended overlap in characters
///     model_name (str): Model identifier
///     dimensions (int): Embedding vector dimensions
///     description (str): Human-readable description of the preset
///
/// Example:
///     >>> from kreuzberg import get_embedding_preset
///     >>> preset = get_embedding_preset("balanced")
///     >>> print(f"Model: {preset.model_name}, Dims: {preset.dimensions}")
///     Model: Xenova/bge-base-en-v1.5, Dims: 768
#[pyclass(name = "EmbeddingPreset", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct EmbeddingPreset {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub chunk_size: usize,
    #[pyo3(get)]
    pub overlap: usize,
    #[pyo3(get)]
    pub model_name: String,
    #[pyo3(get)]
    pub dimensions: usize,
    #[pyo3(get)]
    pub description: String,
}

#[pymethods]
impl EmbeddingPreset {
    fn __repr__(&self) -> String {
        format!(
            "EmbeddingPreset(name='{}', chunk_size={}, overlap={}, model_name='{}', dimensions={}, description='{}')",
            self.name, self.chunk_size, self.overlap, self.model_name, self.dimensions, self.description
        )
    }
}

/// List all available embedding preset names.
///
/// Returns an array of preset names that can be used with get_embedding_preset.
/// Available presets: "fast", "balanced", "quality", "multilingual".
///
/// Returns:
///     list[str]: List of 4 preset names
///
/// Example:
///     >>> from kreuzberg import list_embedding_presets
///     >>> presets = list_embedding_presets()
///     >>> print(presets)
///     ['fast', 'balanced', 'quality', 'multilingual']
#[pyfunction]
fn list_embedding_presets() -> Vec<String> {
    kreuzberg::embeddings::list_presets()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

/// Get a specific embedding preset by name.
///
/// Returns a preset configuration object, or None if the preset name is not found.
///
/// Args:
///     name (str): The preset name (case-sensitive)
///
/// Returns:
///     EmbeddingPreset | None: Preset configuration or None if not found
///
/// Available presets:
///     - "fast": Xenova/all-minilm-l6-v2 (384 dimensions) - Quick prototyping, low-latency
///     - "balanced": Xenova/bge-base-en-v1.5 (768 dimensions) - General-purpose RAG
///     - "quality": Xenova/bge-large-en-v1.5 (1024 dimensions) - High-quality embeddings
///     - "multilingual": Xenova/multilingual-e5-base (768 dimensions) - Multi-language support
///
/// Example:
///     >>> from kreuzberg import get_embedding_preset
///     >>> preset = get_embedding_preset("balanced")
///     >>> if preset:
///     ...     print(f"Model: {preset.model_name}, Dims: {preset.dimensions}")
///     ...     # Model: Xenova/bge-base-en-v1.5, Dims: 768
#[pyfunction]
fn get_embedding_preset(name: String) -> Option<EmbeddingPreset> {
    let preset = kreuzberg::embeddings::get_preset(&name)?;

    let model_name = preset.model_repo.to_string();

    Some(EmbeddingPreset {
        name: preset.name.to_string(),
        chunk_size: preset.chunk_size,
        overlap: preset.overlap,
        model_name,
        dimensions: preset.dimensions,
        description: preset.description.to_string(),
    })
}

/// Detect MIME type from file bytes.
///
/// Analyzes the provided bytes to determine the MIME type using magic number detection.
///
/// Args:
///     data (bytes): File content as bytes
///
/// Returns:
///     str: Detected MIME type (e.g., "application/pdf", "image/png")
///
/// Example:
///     >>> from kreuzberg import detect_mime_type_from_bytes
///     >>> pdf_bytes = b"%PDF-1.4\n"
///     >>> mime_type = detect_mime_type_from_bytes(pdf_bytes)
///     >>> assert "pdf" in mime_type.lower()
#[pyfunction]
fn detect_mime_type_from_bytes(data: &[u8]) -> PyResult<String> {
    kreuzberg::detect_mime_type_from_bytes(data).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Detect MIME type from file path.
///
/// Reads the file at the given path and detects its MIME type.
///
/// Args:
///     path (str): Path to the file
///
/// Returns:
///     str: Detected MIME type (e.g., "application/pdf", "text/plain")
///
/// Example:
///     >>> from kreuzberg import detect_mime_type_from_path
///     >>> mime_type = detect_mime_type_from_path("document.pdf")
///     >>> assert "pdf" in mime_type.lower()
#[pyfunction]
fn detect_mime_type_from_path(path: &str) -> PyResult<String> {
    kreuzberg::detect_mime_type(path, true).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Validate and normalize a MIME type.
///
/// Checks if the provided MIME type is supported. Accepts specific supported types
/// and all image/* types.
///
/// Args:
///     mime_type (str): MIME type to validate (e.g., "application/pdf", "image/png")
///
/// Returns:
///     str: Normalized MIME type string
///
/// Raises:
///     RuntimeError: If the MIME type is not supported
///
/// Example:
///     >>> from kreuzberg import validate_mime_type
///     >>> normalized = validate_mime_type("application/pdf")
///     >>> assert normalized == "application/pdf"
///     >>> # Image types are always supported
///     >>> custom_image = validate_mime_type("image/custom")
///     >>> assert custom_image == "image/custom"
#[pyfunction]
fn validate_mime_type(mime_type: &str) -> PyResult<String> {
    kreuzberg::validate_mime_type(mime_type).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Get file extensions for a MIME type.
///
/// Returns a list of common file extensions associated with the given MIME type.
///
/// Args:
///     mime_type (str): MIME type (e.g., "application/pdf", "image/png")
///
/// Returns:
///     list[str]: List of file extensions (e.g., ["pdf"], ["png"])
///
/// Example:
///     >>> from kreuzberg import get_extensions_for_mime
///     >>> extensions = get_extensions_for_mime("application/pdf")
///     >>> assert "pdf" in extensions
#[pyfunction]
fn get_extensions_for_mime(mime_type: &str) -> PyResult<Vec<String>> {
    kreuzberg::get_extensions_for_mime(mime_type).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Get the last error code from the FFI layer.
///
/// Error codes:
///     - 0: Success (no error)
///     - 1: GenericError
///     - 2: Panic
///     - 3: InvalidArgument
///     - 4: IoError
///     - 5: ParsingError
///     - 6: OcrError
///     - 7: MissingDependency
///
/// Returns:
///     int: The error code (0 if no error has occurred)
///
/// Example:
///     >>> from kreuzberg import get_last_error_code
///     >>> code = get_last_error_code()
///     >>> if code == 2:
///     ...     print("A panic occurred")
#[pyfunction]
fn get_last_error_code() -> i32 {
    ffi::get_last_error_code()
}

/// Get panic context information from the last error.
///
/// Returns JSON string with panic context if the last error was a panic,
/// or None if no panic occurred.
///
/// Panic context fields:
///     - file: Source file where panic occurred
///     - line: Line number
///     - function: Function name
///     - message: Panic message
///     - timestamp_secs: Unix timestamp
///
/// Returns:
///     str | None: JSON string with panic context, or None if no panic
///
/// Example:
///     >>> from kreuzberg import get_last_panic_context
///     >>> context = get_last_panic_context()
///     >>> if context:
///     ...     print(f"Panic details: {context}")
#[pyfunction]
fn get_last_panic_context() -> PyResult<Option<String>> {
    Ok(ffi::get_last_panic_context())
}
