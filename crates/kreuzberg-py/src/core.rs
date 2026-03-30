//! Core extraction functions
//!
//! Provides both synchronous and asynchronous extraction functions for Python.

use crate::config::{ExtractionConfig, FileExtractionConfig};
use crate::error::to_py_err;
use crate::types::ExtractionResult;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use std::path::PathBuf;

type BatchBytesItem = (Vec<u8>, String, Option<kreuzberg::FileExtractionConfig>);

/// Build file items from separate paths and optional per-file configs.
fn build_file_items(
    path_strings: Vec<String>,
    file_configs: Option<Vec<Option<FileExtractionConfig>>>,
) -> PyResult<Vec<(PathBuf, Option<kreuzberg::FileExtractionConfig>)>> {
    if let Some(ref configs) = file_configs
        && configs.len() != path_strings.len()
    {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "file_configs length ({}) must match paths length ({})",
            configs.len(),
            path_strings.len()
        )));
    }

    let items = match file_configs {
        Some(configs) => path_strings
            .into_iter()
            .zip(configs)
            .map(|(p, fc)| (PathBuf::from(p), fc.map(Into::into)))
            .collect(),
        None => path_strings.into_iter().map(|p| (PathBuf::from(p), None)).collect(),
    };
    Ok(items)
}

/// Build bytes items from separate data/mime lists and optional per-item configs.
fn build_bytes_items(
    data_list: Vec<Vec<u8>>,
    mime_types: Vec<String>,
    file_configs: Option<Vec<Option<FileExtractionConfig>>>,
) -> PyResult<Vec<BatchBytesItem>> {
    if let Some(ref configs) = file_configs
        && configs.len() != data_list.len()
    {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "file_configs length ({}) must match data_list length ({})",
            configs.len(),
            data_list.len()
        )));
    }

    let items = match file_configs {
        Some(configs) => data_list
            .into_iter()
            .zip(mime_types)
            .zip(configs)
            .map(|((data, mime), fc)| (data, mime, fc.map(Into::into)))
            .collect(),
        None => data_list
            .into_iter()
            .zip(mime_types)
            .map(|(data, mime)| (data, mime, None))
            .collect(),
    };
    Ok(items)
}

/// Map an OutputFormat enum to a string.
fn output_format_to_str(fmt: &kreuzberg::core::config::formats::OutputFormat) -> String {
    match fmt {
        kreuzberg::core::config::formats::OutputFormat::Plain => "plain".to_string(),
        kreuzberg::core::config::formats::OutputFormat::Markdown => "markdown".to_string(),
        kreuzberg::core::config::formats::OutputFormat::Djot => "djot".to_string(),
        kreuzberg::core::config::formats::OutputFormat::Html => "html".to_string(),
        kreuzberg::core::config::formats::OutputFormat::Json => "json".to_string(),
        kreuzberg::core::config::formats::OutputFormat::Structured => "structured".to_string(),
        kreuzberg::core::config::formats::OutputFormat::Custom(name) => name.clone(),
    }
}

fn result_format_to_str(fmt: &kreuzberg::types::OutputFormat) -> String {
    match fmt {
        kreuzberg::types::OutputFormat::Unified => "unified".to_string(),
        kreuzberg::types::OutputFormat::ElementBased => "element_based".to_string(),
    }
}

/// Extract format strings from ExtractionConfig before it's consumed.
fn extract_format_strings(config: &ExtractionConfig) -> (Option<String>, Option<String>) {
    (
        Some(output_format_to_str(&config.inner.output_format)),
        Some(result_format_to_str(&config.inner.result_format)),
    )
}

/// Collect per-item format strings, using file_config overrides where present.
/// Returns an iterator that yields (output_format, result_format) for each result.
fn collect_per_item_formats(
    config: &ExtractionConfig,
    file_configs: &Option<Vec<Option<FileExtractionConfig>>>,
) -> PerItemFormats {
    let default_output = output_format_to_str(&config.inner.output_format);
    let default_result = result_format_to_str(&config.inner.result_format);

    match file_configs {
        Some(configs) => {
            let formats: Vec<_> = configs
                .iter()
                .map(|fc| {
                    let output = fc
                        .as_ref()
                        .and_then(|c| c.inner.output_format.as_ref())
                        .map(output_format_to_str)
                        .unwrap_or_else(|| default_output.clone());
                    let result = fc
                        .as_ref()
                        .and_then(|c| c.inner.result_format.as_ref())
                        .map(result_format_to_str)
                        .unwrap_or_else(|| default_result.clone());
                    (output, result)
                })
                .collect();
            PerItemFormats::Explicit(formats)
        }
        None => PerItemFormats::Default(default_output, default_result),
    }
}

/// Per-item format info that can be either explicit per-item or a repeated default.
enum PerItemFormats {
    Explicit(Vec<(String, String)>),
    Default(String, String),
}

impl PerItemFormats {
    fn get(&self, index: usize) -> (&str, &str) {
        match self {
            PerItemFormats::Explicit(formats) => formats
                .get(index)
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .unwrap_or_else(|| {
                    formats
                        .last()
                        .map(|(a, b)| (a.as_str(), b.as_str()))
                        .unwrap_or(("plain", "unified"))
                }),
            PerItemFormats::Default(output, result) => (output.as_str(), result.as_str()),
        }
    }
}

/// Extract a path string from Python input (str, pathlib.Path, or bytes).
///
/// Supports:
/// - `str`: Direct string paths
/// - `pathlib.Path`: Extracts via `__fspath__()` protocol
/// - `bytes`: UTF-8 decoded path bytes (Unix paths)
fn extract_path_string(path: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = path.extract::<String>() {
        return Ok(s);
    }

    if let Ok(fspath) = path.call_method0("__fspath__")
        && let Ok(s) = fspath.extract::<String>()
    {
        return Ok(s);
    }

    if let Ok(b) = path.extract::<Vec<u8>>() {
        if let Ok(s) = String::from_utf8(b) {
            return Ok(s);
        }
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Path bytes must be valid UTF-8",
        ));
    }

    Err(pyo3::exceptions::PyTypeError::new_err(
        "Path must be a string, pathlib.Path, or bytes",
    ))
}

/// Extract content from a file (synchronous).
///
/// Args:
///     path: Path to the file to extract (str or pathlib.Path)
///     mime_type: Optional MIME type hint (auto-detected if None)
///     config: Extraction configuration
///
/// Returns:
///     ExtractionResult with content, metadata, and tables
///
/// Raises:
///     ValueError: Invalid configuration or unsupported format
///     IOError: File access errors
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> from kreuzberg import extract_file_sync, ExtractionConfig
///     >>> result = extract_file_sync("document.pdf", None, ExtractionConfig())
///     >>> print(result.content)
///     >>> # Also works with pathlib.Path
///     >>> from pathlib import Path
///     >>> result = extract_file_sync(Path("document.pdf"), None, ExtractionConfig())
#[pyfunction]
#[pyo3(signature = (path, mime_type=None, config=ExtractionConfig::default()))]
pub fn extract_file_sync(
    py: Python,
    path: &Bound<'_, PyAny>,
    mime_type: Option<String>,
    config: ExtractionConfig,
) -> PyResult<ExtractionResult> {
    let path_str = extract_path_string(path)?;
    let (output_fmt, result_fmt) = extract_format_strings(&config);
    let rust_config = config.into();

    // Release GIL during sync extraction - OSError/RuntimeError must bubble up ~keep
    let result = Python::detach(py, || {
        kreuzberg::extract_file_sync(&path_str, mime_type.as_deref(), &rust_config)
    })
    .map_err(to_py_err)?;

    ExtractionResult::from_rust(result, py, output_fmt.as_deref(), result_fmt.as_deref())
}

/// Extract content from bytes (synchronous).
///
/// Args:
///     data: Bytes to extract (bytes or bytearray)
///     mime_type: MIME type of the data
///     config: Extraction configuration
///
/// Returns:
///     ExtractionResult with content, metadata, and tables
///
/// Raises:
///     ValueError: Invalid configuration or unsupported format
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> from kreuzberg import extract_bytes_sync, ExtractionConfig
///     >>> with open("document.pdf", "rb") as f:
///     ...     data = f.read()
///     >>> result = extract_bytes_sync(data, "application/pdf", ExtractionConfig())
///     >>> print(result.content)
#[pyfunction]
#[pyo3(signature = (data, mime_type, config=ExtractionConfig::default()))]
pub fn extract_bytes_sync(
    py: Python,
    data: Vec<u8>,
    mime_type: String,
    config: ExtractionConfig,
) -> PyResult<ExtractionResult> {
    let (output_fmt, result_fmt) = extract_format_strings(&config);
    let rust_config = config.into();

    // Release GIL during extraction and result conversion - OSError/RuntimeError must bubble up ~keep
    let result =
        Python::detach(py, || kreuzberg::extract_bytes_sync(&data, &mime_type, &rust_config)).map_err(to_py_err)?;

    ExtractionResult::from_rust(result, py, output_fmt.as_deref(), result_fmt.as_deref())
}

/// Batch extract content from multiple files (synchronous).
///
/// MIME types are auto-detected for each file.
///
/// Args:
///     paths: List of file paths to extract (str, pathlib.Path, or bytes)
///     config: Extraction configuration
///     file_configs: Optional list of per-file extraction config overrides
///
/// Returns:
///     List of ExtractionResult objects (one per file)
///
/// Raises:
///     ValueError: Invalid configuration or file_configs length mismatch
///     IOError: File access errors
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> from kreuzberg import batch_extract_files_sync, ExtractionConfig
///     >>> paths = ["doc1.pdf", "doc2.docx"]
///     >>> results = batch_extract_files_sync(paths, ExtractionConfig())
///     >>> for result in results:
///     ...     print(result.content)
///     >>> # Also works with pathlib.Path
///     >>> from pathlib import Path
///     >>> paths = [Path("doc1.pdf"), Path("doc2.docx")]
///     >>> results = batch_extract_files_sync(paths, ExtractionConfig())
#[pyfunction]
#[pyo3(signature = (paths, config=ExtractionConfig::default(), file_configs=None))]
pub fn batch_extract_files_sync(
    py: Python,
    paths: &Bound<'_, PyList>,
    config: ExtractionConfig,
    file_configs: Option<Vec<Option<FileExtractionConfig>>>,
) -> PyResult<Py<PyList>> {
    let path_strings: PyResult<Vec<String>> = paths.iter().map(|p| extract_path_string(&p)).collect();
    let path_strings = path_strings?;

    let per_item_formats = collect_per_item_formats(&config, &file_configs);
    let items = build_file_items(path_strings, file_configs)?;

    let rust_config = config.into();

    // Release GIL during sync batch extraction - OSError/RuntimeError must bubble up ~keep
    let results = Python::detach(py, || kreuzberg::batch_extract_file_sync(items, &rust_config)).map_err(to_py_err)?;

    let converted: PyResult<Vec<_>> = results
        .into_iter()
        .enumerate()
        .map(|(i, result)| {
            let (output_fmt, result_fmt) = per_item_formats.get(i);
            ExtractionResult::from_rust(result, py, Some(output_fmt), Some(result_fmt))
        })
        .collect();
    let list = PyList::new(py, converted?)?;
    Ok(list.unbind())
}

/// Batch extract content from multiple byte arrays (synchronous).
///
/// Args:
///     data_list: List of bytes objects to extract
///     mime_types: List of MIME types (one per data object)
///     config: Extraction configuration
///     file_configs: Optional list of per-item extraction config overrides
///
/// Returns:
///     List of ExtractionResult objects (one per data object)
///
/// Raises:
///     ValueError: Invalid configuration or list length mismatch
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> from kreuzberg import batch_extract_bytes_sync, ExtractionConfig
///     >>> data_list = [open("doc1.pdf", "rb").read(), open("doc2.pdf", "rb").read()]
///     >>> mime_types = ["application/pdf", "application/pdf"]
///     >>> results = batch_extract_bytes_sync(data_list, mime_types, ExtractionConfig())
#[pyfunction]
#[pyo3(signature = (data_list, mime_types, config=ExtractionConfig::default(), file_configs=None))]
pub fn batch_extract_bytes_sync(
    py: Python,
    data_list: Vec<Vec<u8>>,
    mime_types: Vec<String>,
    config: ExtractionConfig,
    file_configs: Option<Vec<Option<FileExtractionConfig>>>,
) -> PyResult<Py<PyList>> {
    if data_list.len() != mime_types.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "data_list and mime_types must have the same length (got {} and {})",
            data_list.len(),
            mime_types.len()
        )));
    }

    let per_item_formats = collect_per_item_formats(&config, &file_configs);
    let items = build_bytes_items(data_list, mime_types, file_configs)?;

    let rust_config = config.into();

    // Release GIL during sync batch extraction - OSError/RuntimeError must bubble up ~keep
    let results = Python::detach(py, || kreuzberg::batch_extract_bytes_sync(items, &rust_config)).map_err(to_py_err)?;

    let converted: PyResult<Vec<_>> = results
        .into_iter()
        .enumerate()
        .map(|(i, result)| {
            let (output_fmt, result_fmt) = per_item_formats.get(i);
            ExtractionResult::from_rust(result, py, Some(output_fmt), Some(result_fmt))
        })
        .collect();
    let list = PyList::new(py, converted?)?;
    Ok(list.unbind())
}

/// Extract content from a file (asynchronous).
///
/// Args:
///     path: Path to the file to extract (str or pathlib.Path)
///     mime_type: Optional MIME type hint (auto-detected if None)
///     config: Extraction configuration
///
/// Returns:
///     ExtractionResult with content, metadata, and tables
///
/// Raises:
///     ValueError: Invalid configuration or unsupported format
///     IOError: File access errors
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> import asyncio
///     >>> from kreuzberg import extract_file, ExtractionConfig
///     >>> async def main():
///     ...     result = await extract_file("document.pdf", None, ExtractionConfig())
///     ...     print(result.content)
///     >>> asyncio.run(main())
///     >>> # Also works with pathlib.Path
///     >>> from pathlib import Path
///     >>> async def main():
///     ...     result = await extract_file(Path("document.pdf"))
#[pyfunction]
#[pyo3(signature = (path, mime_type=None, config=ExtractionConfig::default()))]
pub fn extract_file<'py>(
    py: Python<'py>,
    path: &Bound<'py, PyAny>,
    mime_type: Option<String>,
    config: ExtractionConfig,
) -> PyResult<Bound<'py, PyAny>> {
    let path_str = extract_path_string(path)?;
    let (output_fmt, result_fmt) = extract_format_strings(&config);
    let rust_config: kreuzberg::ExtractionConfig = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let result = kreuzberg::extract_file(&path_str, mime_type.as_deref(), &rust_config)
            .await
            .map_err(to_py_err)?;
        Python::attach(|py| ExtractionResult::from_rust(result, py, output_fmt.as_deref(), result_fmt.as_deref()))
    })
}

/// Extract content from bytes (asynchronous).
///
/// Args:
///     data: Bytes to extract (bytes or bytearray)
///     mime_type: MIME type of the data
///     config: Extraction configuration
///
/// Returns:
///     ExtractionResult with content, metadata, and tables
///
/// Raises:
///     ValueError: Invalid configuration or unsupported format
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> import asyncio
///     >>> from kreuzberg import extract_bytes, ExtractionConfig
///     >>> async def main():
///     ...     with open("document.pdf", "rb") as f:
///     ...         data = f.read()
///     ...     result = await extract_bytes(data, "application/pdf", ExtractionConfig())
///     ...     print(result.content)
///     >>> asyncio.run(main())
#[pyfunction]
#[pyo3(signature = (data, mime_type, config=ExtractionConfig::default()))]
pub fn extract_bytes<'py>(
    py: Python<'py>,
    data: Vec<u8>,
    mime_type: String,
    config: ExtractionConfig,
) -> PyResult<Bound<'py, PyAny>> {
    let (output_fmt, result_fmt) = extract_format_strings(&config);
    let rust_config: kreuzberg::ExtractionConfig = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let result = kreuzberg::extract_bytes(&data, &mime_type, &rust_config)
            .await
            .map_err(to_py_err)?;
        Python::attach(|py| ExtractionResult::from_rust(result, py, output_fmt.as_deref(), result_fmt.as_deref()))
    })
}

/// Batch extract content from multiple files (asynchronous).
///
/// MIME types are auto-detected for each file.
///
/// Args:
///     paths: List of file paths to extract (str, pathlib.Path, or bytes)
///     config: Extraction configuration
///     file_configs: Optional list of per-file extraction config overrides
///
/// Returns:
///     List of ExtractionResult objects (one per file)
///
/// Raises:
///     ValueError: Invalid configuration or file_configs length mismatch
///     IOError: File access errors
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> import asyncio
///     >>> from kreuzberg import batch_extract_files, ExtractionConfig
///     >>> async def main():
///     ...     paths = ["doc1.pdf", "doc2.docx"]
///     ...     results = await batch_extract_files(paths, ExtractionConfig())
///     ...     for result in results:
///     ...         print(result.content)
///     >>> asyncio.run(main())
///     >>> # Also works with pathlib.Path
///     >>> from pathlib import Path
///     >>> async def main():
///     ...     paths = [Path("doc1.pdf"), Path("doc2.docx")]
///     ...     results = await batch_extract_files(paths, ExtractionConfig())
#[pyfunction]
#[pyo3(signature = (paths, config=ExtractionConfig::default(), file_configs=None))]
pub fn batch_extract_files<'py>(
    py: Python<'py>,
    paths: &Bound<'py, PyList>,
    config: ExtractionConfig,
    file_configs: Option<Vec<Option<FileExtractionConfig>>>,
) -> PyResult<Bound<'py, PyAny>> {
    let path_strings: PyResult<Vec<String>> = paths.iter().map(|p| extract_path_string(&p)).collect();
    let path_strings = path_strings?;

    let per_item_formats = collect_per_item_formats(&config, &file_configs);
    let items = build_file_items(path_strings, file_configs)?;

    let rust_config: kreuzberg::ExtractionConfig = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let results = kreuzberg::batch_extract_file(items, &rust_config)
            .await
            .map_err(to_py_err)?;

        Python::attach(|py| {
            let converted: PyResult<Vec<_>> = results
                .into_iter()
                .enumerate()
                .map(|(i, result)| {
                    let (output_fmt, result_fmt) = per_item_formats.get(i);
                    ExtractionResult::from_rust(result, py, Some(output_fmt), Some(result_fmt))
                })
                .collect();
            let list = PyList::new(py, converted?)?;
            Ok(list.unbind())
        })
    })
}

/// Batch extract content from multiple byte arrays (asynchronous).
///
/// Args:
///     data_list: List of bytes objects to extract
///     mime_types: List of MIME types (one per data object)
///     config: Extraction configuration
///     file_configs: Optional list of per-item extraction config overrides
///
/// Returns:
///     List of ExtractionResult objects (one per data object)
///
/// Raises:
///     ValueError: Invalid configuration or list length mismatch
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> import asyncio
///     >>> from kreuzberg import batch_extract_bytes, ExtractionConfig
///     >>> async def main():
///     ...     data_list = [open("doc1.pdf", "rb").read(), open("doc2.pdf", "rb").read()]
///     ...     mime_types = ["application/pdf", "application/pdf"]
///     ...     results = await batch_extract_bytes(data_list, mime_types, ExtractionConfig())
///     >>> asyncio.run(main())
#[pyfunction]
#[pyo3(signature = (data_list, mime_types, config=ExtractionConfig::default(), file_configs=None))]
pub fn batch_extract_bytes<'py>(
    py: Python<'py>,
    data_list: Vec<Vec<u8>>,
    mime_types: Vec<String>,
    config: ExtractionConfig,
    file_configs: Option<Vec<Option<FileExtractionConfig>>>,
) -> PyResult<Bound<'py, PyAny>> {
    if data_list.len() != mime_types.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "data_list and mime_types must have the same length (got {} and {})",
            data_list.len(),
            mime_types.len()
        )));
    }

    let per_item_formats = collect_per_item_formats(&config, &file_configs);
    let items = build_bytes_items(data_list, mime_types, file_configs)?;

    let rust_config: kreuzberg::ExtractionConfig = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let results = kreuzberg::batch_extract_bytes(items, &rust_config)
            .await
            .map_err(to_py_err)?;

        Python::attach(|py| {
            let converted: PyResult<Vec<_>> = results
                .into_iter()
                .enumerate()
                .map(|(i, result)| {
                    let (output_fmt, result_fmt) = per_item_formats.get(i);
                    ExtractionResult::from_rust(result, py, Some(output_fmt), Some(result_fmt))
                })
                .collect();
            let list = PyList::new(py, converted?)?;
            Ok(list.unbind())
        })
    })
}

/// Generate embeddings from a list of text strings (synchronous).
///
/// Args:
///     texts: List of strings to embed
///     config: Embedding configuration (model, batch size, normalization)
///
/// Returns:
///     list[list[float]]: One embedding vector per input text
///
/// Raises:
///     MissingDependencyError: ONNX Runtime not installed
///     ParsingError: Unknown preset or model download failure
///
/// Example:
///     >>> from kreuzberg import embed_sync, EmbeddingConfig, EmbeddingModelType
///     >>> config = EmbeddingConfig(model=EmbeddingModelType.preset("balanced"))
///     >>> result = embed_sync(["Hello, world!"], config=config)
///     >>> len(result)
///     1
#[pyfunction]
#[pyo3(signature = (texts, config=crate::config::EmbeddingConfig::default()))]
pub fn embed_sync(
    py: Python,
    texts: Vec<String>,
    config: crate::config::EmbeddingConfig,
) -> PyResult<Vec<Vec<f32>>> {
    let rust_config = config.inner;
    Python::detach(py, || {
        kreuzberg::embed_texts(&texts, &rust_config).map_err(crate::error::to_py_err)
    })
}

/// Generate embeddings from a list of text strings (asynchronous).
///
/// Args:
///     texts: List of strings to embed
///     config: Embedding configuration (model, batch size, normalization)
///
/// Returns:
///     Awaitable[list[list[float]]]: One embedding vector per input text
///
/// Raises:
///     MissingDependencyError: ONNX Runtime not installed
///     ParsingError: Unknown preset or model download failure
///
/// Example:
///     >>> import asyncio
///     >>> from kreuzberg import embed, EmbeddingConfig, EmbeddingModelType
///     >>> async def main():
///     ...     config = EmbeddingConfig(model=EmbeddingModelType.preset("balanced"))
///     ...     result = await embed(["Hello, world!"], config=config)
///     ...     print(len(result))  # 1
///     >>> asyncio.run(main())
#[pyfunction]
#[pyo3(signature = (texts, config=crate::config::EmbeddingConfig::default()))]
pub fn embed<'py>(
    py: Python<'py>,
    texts: Vec<String>,
    config: crate::config::EmbeddingConfig,
) -> PyResult<Bound<'py, PyAny>> {
    let rust_config = config.inner;
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        kreuzberg::embed_texts_async(texts, &rust_config)
            .await
            .map_err(crate::error::to_py_err)
    })
}

/// Render a single page of a PDF file to a PNG byte buffer.
///
/// Args:
///     file_path: Path to the PDF file
///     page_index: Zero-based page index
///     dpi: Optional DPI (default 150)
///
/// Returns:
///     bytes: PNG image data
///
/// Raises:
///     RuntimeError: If rendering fails
#[pyfunction]
#[pyo3(signature = (file_path, page_index, dpi=None))]
pub fn render_pdf_page_impl(
    py: Python<'_>,
    file_path: &str,
    page_index: usize,
    dpi: Option<i32>,
) -> PyResult<Py<PyBytes>> {
    let pdf_bytes = std::fs::read(file_path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(format!("Failed to read file: {}", e)))?;
    let page = Python::detach(py, || {
        kreuzberg::pdf::render_pdf_page_to_png(&pdf_bytes, page_index, dpi, None)
    })
    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Python::attach(|py| Ok(PyBytes::new(py, &page).unbind()))
}

/// Lazy page-by-page PDF renderer.
///
/// Opens the PDF once and yields one PNG-encoded page per `.next()` call.
/// Only one rendered page is held in memory at a time. Supports the
/// iterator protocol and the context-manager protocol (`with` statement).
///
/// Args:
///     file_path: Path to the PDF file
///     dpi: Optional DPI (default 150)
///
/// Example:
///     from kreuzberg import PdfPageIterator
///
///     with PdfPageIterator("document.pdf") as pages:
///         for page_index, png_bytes in pages:
///             print(f"page {page_index}: {len(png_bytes)} bytes")
#[pyclass(name = "PdfPageIterator", module = "kreuzberg")]
pub struct PyPdfPageIterator {
    inner: Option<kreuzberg::pdf::PdfPageIterator>,
}

#[pymethods]
impl PyPdfPageIterator {
    #[new]
    #[pyo3(signature = (file_path, dpi=None))]
    fn new(file_path: &str, dpi: Option<i32>) -> PyResult<Self> {
        let iter = kreuzberg::pdf::PdfPageIterator::from_file(file_path, dpi, None)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: Some(iter) })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<(usize, Py<PyBytes>)>> {
        let iter = match self.inner.as_mut() {
            Some(it) => it,
            None => return Ok(None),
        };

        match iter.next() {
            Some(Ok((page_index, png))) => {
                let bytes = PyBytes::new(py, &png).unbind();
                Ok(Some((page_index, bytes)))
            }
            Some(Err(e)) => Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
            None => Ok(None),
        }
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(
        &mut self,
        _exc_type: &Bound<'_, PyAny>,
        _exc_val: &Bound<'_, PyAny>,
        _exc_tb: &Bound<'_, PyAny>,
    ) -> bool {
        self.inner = None;
        false
    }

    fn __len__(&self) -> usize {
        self.inner.as_ref().map_or(0, |it| it.page_count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::{PyBytes, PyString};
    use std::sync::Once;

    fn prepare_python() {
        static INIT: Once = Once::new();
        INIT.call_once(Python::initialize);
    }

    fn with_py<F, R>(f: F) -> R
    where
        F: FnOnce(Python<'_>) -> R,
    {
        prepare_python();
        Python::attach(f)
    }

    #[test]
    fn test_extract_path_string_from_str() {
        with_py(|py| {
            let value = PyString::new(py, "document.txt");
            let result = extract_path_string(&value.into_any()).expect("string path should extract");
            assert_eq!(result, "document.txt");
        });
    }

    #[test]
    fn test_extract_path_string_from_pathlib_path() {
        with_py(|py| -> PyResult<()> {
            let pathlib = py.import("pathlib")?;
            let path_obj = pathlib.getattr("Path")?.call1(("nested/file.md",))?;
            let extracted = extract_path_string(&path_obj)?;
            assert!(
                extracted.ends_with("nested/file.md"),
                "expected path to end with nested/file.md, got {extracted}"
            );
            Ok(())
        })
        .expect("pathlib.Path extraction should succeed");
    }

    #[test]
    fn test_extract_path_string_from_bytes() {
        with_py(|py| {
            let value = PyBytes::new(py, b"ascii.bin");
            let result = extract_path_string(&value.into_any()).expect("bytes path should extract");
            assert_eq!(result, "ascii.bin");
        });
    }

    #[test]
    fn test_extract_path_string_invalid_type() {
        with_py(|py| {
            let value = py
                .eval(pyo3::ffi::c_str!("42"), None, None)
                .expect("should evaluate literal");
            let err = extract_path_string(&value).expect_err("non-path type should fail");
            assert!(err.is_instance_of::<pyo3::exceptions::PyTypeError>(py));
        });
    }

    #[test]
    fn test_extract_bytes_sync_returns_content() {
        with_py(|py| {
            let data = b"hello kreuzberg".to_vec();
            let result = extract_bytes_sync(py, data, "text/plain".to_string(), ExtractionConfig::default())
                .expect("text/plain extraction should succeed");
            assert_eq!(result.mime_type, "text/plain");
            assert!(result.content.contains("hello"));
        });
    }

    #[test]
    fn test_batch_extract_bytes_sync_length_mismatch() {
        with_py(|py| {
            let err = batch_extract_bytes_sync(
                py,
                vec![b"a".to_vec(), b"b".to_vec()],
                vec!["text/plain".to_string()],
                ExtractionConfig::default(),
                None,
            )
            .expect_err("length mismatch should error");
            assert!(err.is_instance_of::<pyo3::exceptions::PyValueError>(py));
        });
    }

    #[test]
    fn test_batch_extract_bytes_sync_returns_list() {
        with_py(|py| {
            let data = vec![b"first".to_vec(), b"second".to_vec()];
            let mimes = vec!["text/plain".to_string(), "text/plain".to_string()];
            let list = batch_extract_bytes_sync(py, data, mimes, ExtractionConfig::default(), None)
                .expect("batch extraction should succeed");
            assert_eq!(list.bind(py).len(), 2);
        });
    }
}
