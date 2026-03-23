//! Python OCR backend bridge with GIL management.
//!
//! Provides a Rust wrapper that makes Python OCR backends usable from Rust by implementing
//! the `OcrBackend` trait and managing the FFI boundary with proper GIL handling.

use async_trait::async_trait;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};
use std::path::Path;
use std::sync::Arc;

use kreuzberg::core::config::OcrConfig;
use kreuzberg::plugins::registry::get_ocr_backend_registry;
use kreuzberg::plugins::{OcrBackend, OcrBackendType, Plugin};
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result};

use super::common::validate_plugin_object;

/// Wrapper that makes a Python OCR backend usable from Rust.
///
/// This struct implements the Rust `OcrBackend` trait by forwarding calls
/// to a Python object via PyO3, bridging the FFI boundary with proper
/// GIL management and type conversions.
pub struct PythonOcrBackend {
    /// Python object implementing the OCR backend protocol
    python_obj: Py<PyAny>,
    /// Cached backend name (to avoid repeated GIL acquisition)
    name: String,
    /// Cached supported languages
    supported_languages: Vec<String>,
    /// Cached async flag for process_image method
    process_image_is_async: bool,
    /// Cached async flag for process_document method
    process_document_is_async: bool,
}

impl PythonOcrBackend {
    /// Create a new Python OCR backend wrapper.
    ///
    /// # Arguments
    ///
    /// * `py` - Python GIL token
    /// * `python_obj` - Python object implementing the backend protocol
    ///
    /// # Returns
    ///
    /// A new `PythonOcrBackend` or an error if the Python object is invalid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Python object doesn't have required methods
    /// - Method calls fail during initialization
    pub fn new(py: Python<'_>, python_obj: Py<PyAny>) -> PyResult<Self> {
        let obj = python_obj.bind(py);

        validate_plugin_object(obj, "OCR backend", &["name", "supported_languages", "process_image"])?;

        let name: String = obj.call_method0("name")?.extract()?;
        if name.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "OCR backend name cannot be empty",
            ));
        }

        let supported_languages: Vec<String> = obj.call_method0("supported_languages")?.extract()?;
        if supported_languages.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "OCR backend must support at least one language",
            ));
        }

        let process_image_is_async = obj
            .getattr("process_image")
            .and_then(|m| m.hasattr("__await__"))
            .unwrap_or(false);

        let process_document_is_async = obj
            .getattr("process_document")
            .and_then(|m| m.hasattr("__await__"))
            .unwrap_or(false);

        Ok(Self {
            python_obj,
            name,
            supported_languages,
            process_image_is_async,
            process_document_is_async,
        })
    }
}

impl Plugin for PythonOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        Python::attach(|py| {
            self.python_obj
                .bind(py)
                .getattr("version")
                .and_then(|v| v.call0())
                .and_then(|v| v.extract::<String>())
                .unwrap_or_else(|_| "1.0.0".to_string())
        })
    }

    fn initialize(&self) -> Result<()> {
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);
            if obj.hasattr("initialize")? {
                obj.call_method0("initialize")?;
            }
            Ok(())
        })
        .map_err(|e: PyErr| KreuzbergError::Plugin {
            message: format!("Failed to initialize Python OCR backend '{}': {}", self.name, e),
            plugin_name: self.name.clone(),
        })
    }

    fn shutdown(&self) -> Result<()> {
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);
            if obj.hasattr("shutdown")? {
                obj.call_method0("shutdown")?;
            }
            Ok(())
        })
        .map_err(|e: PyErr| KreuzbergError::Plugin {
            message: format!("Failed to shutdown Python OCR backend '{}': {}", self.name, e),
            plugin_name: self.name.clone(),
        })
    }
}

#[async_trait]
impl OcrBackend for PythonOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        let image_bytes = image_bytes.to_vec();
        let language = config.language.clone();
        let backend_name = self.name.clone();

        if self.process_image_is_async {
            let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

            let result = Python::attach(|py| {
                let obj = python_obj.bind(py);
                let py_bytes = PyBytes::new(py, &image_bytes);

                let coroutine = obj
                    .call_method1("process_image", (py_bytes, language.as_str()))
                    .map_err(|e| KreuzbergError::Ocr {
                        message: format!(
                            "Python OCR backend '{}' failed during process_image: {}",
                            backend_name, e
                        ),
                        source: Some(Box::new(e)),
                    })?;

                if !coroutine.hasattr("__await__").unwrap_or(false) {
                    return Err(KreuzbergError::Ocr {
                        message: format!(
                            "Python OCR backend '{}' process_image marked as async but did not return a coroutine",
                            backend_name
                        ),
                        source: None,
                    });
                }

                pyo3_async_runtimes::tokio::into_future(coroutine).map_err(|e| KreuzbergError::Ocr {
                    message: format!(
                        "Failed to convert Python coroutine to Rust future for OCR backend '{}': {}",
                        backend_name, e
                    ),
                    source: Some(Box::new(e)),
                })
            })?
            .await
            .map_err(|e: PyErr| KreuzbergError::Ocr {
                message: format!(
                    "Python OCR backend '{}' async process_image failed: {}",
                    backend_name, e
                ),
                source: Some(Box::new(e)),
            })?;

            Python::attach(|py| dict_to_extraction_result(py, result.bind(py)))
        } else {
            let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

            tokio::task::spawn_blocking(move || {
                Python::attach(|py| {
                    let obj = python_obj.bind(py);

                    let py_bytes = PyBytes::new(py, &image_bytes);

                    let result = obj
                        .call_method1("process_image", (py_bytes, language.as_str()))
                        .map_err(|e| KreuzbergError::Ocr {
                            message: format!(
                                "Python OCR backend '{}' failed during process_image: {}",
                                backend_name, e
                            ),
                            source: Some(Box::new(e)),
                        })?;

                    dict_to_extraction_result(py, &result)
                })
            })
            .await
            .map_err(|e| KreuzbergError::Ocr {
                message: format!("Failed to spawn blocking task for Python OCR backend: {}", e),
                source: Some(Box::new(e)),
            })?
        }
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        // If hasattr fails due to GIL error, log and fall back to process_image ~keep
        let backend_name = self.name.clone();
        let has_process_image_file = Python::attach(|py| {
            self.python_obj
                .bind(py)
                .hasattr("process_image_file")
                .map_err(|e| {
                    tracing::debug!(
                        "WARNING: OCR backend '{}': Failed to check for process_image_file method due to GIL error ({}), falling back to process_image",
                        backend_name, e
                    );
                    e
                })
                .unwrap_or(false)
        });

        if has_process_image_file {
            let path_str = path.to_string_lossy().to_string();
            let language = config.language.clone();
            let backend_name = self.name.clone();

            let is_async = Python::attach(|py| {
                let obj = self.python_obj.bind(py);
                obj.getattr("process_image_file")
                    .and_then(|method| method.hasattr("__await__"))
                    .unwrap_or(false)
            });

            if is_async {
                let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

                let result = Python::attach(|py| {
                    let obj = python_obj.bind(py);
                    let py_path = PyString::new(py, &path_str);

                    let coroutine = obj
                        .call_method1("process_image_file", (py_path, language.as_str()))
                        .map_err(|e| KreuzbergError::Ocr {
                            message: format!(
                                "Python OCR backend '{}' failed during process_image_file: {}",
                                backend_name, e
                            ),
                            source: Some(Box::new(e)),
                        })?;

                    if !coroutine.hasattr("__await__").unwrap_or(false) {
                        return Err(KreuzbergError::Ocr {
                            message: format!(
                                "Python OCR backend '{}' process_image_file marked as async but did not return a coroutine",
                                backend_name
                            ),
                            source: None,
                        });
                    }

                    pyo3_async_runtimes::tokio::into_future(coroutine).map_err(|e| KreuzbergError::Ocr {
                        message: format!(
                            "Failed to convert Python coroutine to Rust future for OCR backend '{}': {}",
                            backend_name, e
                        ),
                        source: Some(Box::new(e)),
                    })
                })?
                .await
                .map_err(|e: PyErr| KreuzbergError::Ocr {
                    message: format!("Python OCR backend '{}' async process_image_file failed: {}", backend_name, e),
                    source: Some(Box::new(e)),
                })?;

                Python::attach(|py| dict_to_extraction_result(py, result.bind(py)))
            } else {
                let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

                tokio::task::spawn_blocking(move || {
                    Python::attach(|py| {
                        let obj = python_obj.bind(py);
                        let py_path = PyString::new(py, &path_str);

                        let result = obj
                            .call_method1("process_image_file", (py_path, language.as_str()))
                            .map_err(|e| KreuzbergError::Ocr {
                                message: format!(
                                    "Python OCR backend '{}' failed during process_image_file: {}",
                                    backend_name, e
                                ),
                                source: Some(Box::new(e)),
                            })?;

                        dict_to_extraction_result(py, &result)
                    })
                })
                .await
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("Failed to spawn blocking task for Python OCR backend: {}", e),
                    source: Some(Box::new(e)),
                })?
            }
        } else {
            use kreuzberg::core::io;
            let bytes = io::read_file_async(path).await?;
            self.process_image(&bytes, config).await
        }
    }

    fn supports_document_processing(&self) -> bool {
        Python::attach(|py| {
            self.python_obj
                .bind(py)
                .call_method0("supports_document_processing")
                .and_then(|result| result.extract::<bool>())
                .unwrap_or(false)
        })
    }

    async fn process_document(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let path_str = path.to_string_lossy().to_string();
        let language = config.language.clone();
        let backend_name = self.name.clone();

        if self.process_document_is_async {
            let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

            let result = Python::attach(|py| {
                let obj = python_obj.bind(py);
                let py_path = PyString::new(py, &path_str);

                let coroutine = obj
                    .call_method1("process_document", (py_path, language.as_str()))
                    .map_err(|e| KreuzbergError::Ocr {
                        message: format!(
                            "Python OCR backend '{}' failed during process_document: {}",
                            backend_name, e
                        ),
                        source: Some(Box::new(e)),
                    })?;

                if !coroutine.hasattr("__await__").unwrap_or(false) {
                    return Err(KreuzbergError::Ocr {
                        message: format!(
                            "Python OCR backend '{}' process_document marked as async but did not return a coroutine",
                            backend_name
                        ),
                        source: None,
                    });
                }

                pyo3_async_runtimes::tokio::into_future(coroutine).map_err(|e| KreuzbergError::Ocr {
                    message: format!(
                        "Failed to convert Python coroutine to Rust future for OCR backend '{}': {}",
                        backend_name, e
                    ),
                    source: Some(Box::new(e)),
                })
            })?
            .await
            .map_err(|e: PyErr| KreuzbergError::Ocr {
                message: format!(
                    "Python OCR backend '{}' async process_document failed: {}",
                    backend_name, e
                ),
                source: Some(Box::new(e)),
            })?;

            Python::attach(|py| dict_to_extraction_result(py, result.bind(py)))
        } else {
            let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

            tokio::task::spawn_blocking(move || {
                Python::attach(|py| {
                    let obj = python_obj.bind(py);
                    let py_path = PyString::new(py, &path_str);

                    let result = obj
                        .call_method1("process_document", (py_path, language.as_str()))
                        .map_err(|e| KreuzbergError::Ocr {
                            message: format!(
                                "Python OCR backend '{}' failed during process_document: {}",
                                backend_name, e
                            ),
                            source: Some(Box::new(e)),
                        })?;

                    dict_to_extraction_result(py, &result)
                })
            })
            .await
            .map_err(|e| KreuzbergError::Ocr {
                message: format!("Failed to spawn blocking task for Python OCR backend: {}", e),
                source: Some(Box::new(e)),
            })?
        }
    }

    fn supports_language(&self, lang: &str) -> bool {
        self.supported_languages.iter().any(|l| l == lang)
    }

    fn backend_type(&self) -> OcrBackendType {
        match self.name.as_str() {
            "easyocr" => OcrBackendType::EasyOCR,
            "paddleocr" | "paddle" => OcrBackendType::PaddleOCR,
            _ => OcrBackendType::Custom,
        }
    }

    fn supported_languages(&self) -> Vec<String> {
        self.supported_languages.clone()
    }
}

/// Convert Python dict to Rust ExtractionResult.
///
/// Expected dict format:
/// ```python
/// {
///     "content": "extracted text",
///     "metadata": {"width": 800, "height": 600},
///     "tables": []  # Optional
/// }
/// ```
fn dict_to_extraction_result(_py: Python<'_>, dict: &Bound<'_, PyAny>) -> Result<ExtractionResult> {
    let content: String = match dict.get_item("content") {
        Ok(val) if !val.is_none() => val.extract().map_err(|e| KreuzbergError::Validation {
            message: format!("Python OCR result 'content' must be a string: {}", e),
            source: None,
        })?,
        Ok(_) => {
            return Err(KreuzbergError::Validation {
                message: "Python OCR result 'content' field is None".to_string(),
                source: None,
            });
        }
        Err(e) => {
            return Err(KreuzbergError::Validation {
                message: format!("Python OCR result missing 'content' field: {}", e),
                source: None,
            });
        }
    };

    let additional = match dict.get_item("metadata") {
        Ok(m) if !m.is_none() => extract_metadata(&m)?,
        _ => ahash::AHashMap::new(),
    };

    let tables = match dict.get_item("tables") {
        Ok(t) if !t.is_none() => extract_tables(&t)?,
        _ => vec![],
    };

    Ok(ExtractionResult {
        content,
        mime_type: std::borrow::Cow::Borrowed("text/plain"),
        metadata: kreuzberg::types::Metadata {
            additional,
            ..Default::default()
        },
        tables,
        detected_languages: None,
        chunks: None,
        images: None,
        pages: None,
        elements: None,
        document: None,
        djot_content: None,
        ocr_elements: None,
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: vec![],
        annotations: None,
    })
}

/// Extract metadata dict from Python object.
fn extract_metadata(
    obj: &Bound<'_, PyAny>,
) -> Result<ahash::AHashMap<std::borrow::Cow<'static, str>, serde_json::Value>> {
    use super::common::python_to_json;
    use pyo3::types::PyDict;

    let dict = obj.cast::<PyDict>().map_err(|_| KreuzbergError::Validation {
        message: "Metadata must be a dict".to_string(),
        source: None,
    })?;

    let mut metadata = ahash::AHashMap::new();
    for (key, value) in dict.iter() {
        let key_str: String = key.extract().map_err(|_| KreuzbergError::Validation {
            message: "Metadata keys must be strings".to_string(),
            source: None,
        })?;

        let json_value = python_to_json(&value)?;
        metadata.insert(std::borrow::Cow::Owned(key_str), json_value);
    }

    Ok(metadata)
}

/// Extract tables from Python object.
///
/// Expected Python format:
/// ```python
/// [
///     {
///         "cells": [["row1col1", "row1col2"], ["row2col1", "row2col2"]],
///         "markdown": "| Header |\n| ------ |\n| Cell   |",
///         "page_number": 1
///     }
/// ]
/// ```
fn extract_tables(obj: &Bound<'_, PyAny>) -> Result<Vec<kreuzberg::types::Table>> {
    use pyo3::types::{PyDict, PyList};

    let list = obj.cast::<PyList>().map_err(|_| KreuzbergError::Validation {
        message: "Tables must be a list".to_string(),
        source: None,
    })?;

    let mut tables = Vec::new();

    for (index, item) in list.iter().enumerate() {
        let table_dict = item.cast::<PyDict>().map_err(|_| KreuzbergError::Validation {
            message: format!("Table at index {} must be a dict", index),
            source: None,
        })?;

        let cells_val = table_dict
            .get_item("cells")
            .map_err(|e| KreuzbergError::Validation {
                message: format!("Table at index {} error getting 'cells': {}", index, e),
                source: None,
            })?
            .ok_or_else(|| KreuzbergError::Validation {
                message: format!("Table at index {} missing 'cells' field", index),
                source: None,
            })?;

        if cells_val.is_none() {
            return Err(KreuzbergError::Validation {
                message: format!("Table at index {} 'cells' field is None", index),
                source: None,
            });
        }

        let cells_list = cells_val.cast::<PyList>().map_err(|_| KreuzbergError::Validation {
            message: format!("Table at index {} 'cells' must be a list", index),
            source: None,
        })?;

        let cells: Vec<Vec<String>> = cells_list
            .iter()
            .enumerate()
            .map(|(row_idx, row_obj)| {
                let row_list = row_obj.cast::<PyList>().map_err(|_| KreuzbergError::Validation {
                    message: format!("Table {} row {} must be a list", index, row_idx),
                    source: None,
                })?;

                row_list
                    .iter()
                    .enumerate()
                    .map(|(col_idx, cell_obj)| {
                        cell_obj.extract::<String>().map_err(|e| KreuzbergError::Validation {
                            message: format!(
                                "Table {} cell [{}, {}] must be a string: {}",
                                index, row_idx, col_idx, e
                            ),
                            source: None,
                        })
                    })
                    .collect::<Result<Vec<String>>>()
            })
            .collect::<Result<Vec<Vec<String>>>>()?;

        let markdown_val = table_dict
            .get_item("markdown")
            .map_err(|e| KreuzbergError::Validation {
                message: format!("Table at index {} error getting 'markdown': {}", index, e),
                source: None,
            })?
            .ok_or_else(|| KreuzbergError::Validation {
                message: format!("Table at index {} missing 'markdown' field", index),
                source: None,
            })?;

        if markdown_val.is_none() {
            return Err(KreuzbergError::Validation {
                message: format!("Table at index {} 'markdown' field is None", index),
                source: None,
            });
        }

        let markdown: String = markdown_val.extract().map_err(|e| KreuzbergError::Validation {
            message: format!("Table at index {} 'markdown' must be a string: {}", index, e),
            source: None,
        })?;

        let page_num_val = table_dict
            .get_item("page_number")
            .map_err(|e| KreuzbergError::Validation {
                message: format!("Table at index {} error getting 'page_number': {}", index, e),
                source: None,
            })?
            .ok_or_else(|| KreuzbergError::Validation {
                message: format!("Table at index {} missing 'page_number' field", index),
                source: None,
            })?;

        if page_num_val.is_none() {
            return Err(KreuzbergError::Validation {
                message: format!("Table at index {} 'page_number' field is None", index),
                source: None,
            });
        }

        let page_number: usize = page_num_val.extract().map_err(|e| KreuzbergError::Validation {
            message: format!("Table at index {} 'page_number' must be an integer: {}", index, e),
            source: None,
        })?;

        tables.push(kreuzberg::types::Table {
            cells,
            markdown,
            page_number,
            bounding_box: None,
        });
    }

    Ok(tables)
}

/// Register a Python OCR backend with the Rust core.
///
/// This function validates the Python backend object, wraps it in a Rust
/// `OcrBackend` implementation, and registers it with the global OCR backend
/// registry. Once registered, the backend can be used by the Rust CLI, API,
/// and MCP server.
///
/// # Arguments
///
/// * `name` - Backend name (must be unique)
/// * `backend` - Python object implementing the OCR backend protocol
///
/// # Required Methods on Python Backend
///
/// The Python backend must implement:
/// - `name() -> str` - Return backend name
/// - `supported_languages() -> list[str]` - Return list of supported language codes
/// - `process_image(image_bytes: bytes, language: str) -> dict` - Process image and return result
///
/// # Optional Methods
///
/// - `process_image_file(path: str, language: str) -> dict` - Custom file processing
/// - `initialize()` - Called when backend is registered
/// - `shutdown()` - Called when backend is unregistered
/// - `version() -> str` - Backend version (defaults to "1.0.0")
///
/// # Example
///
/// ```python
/// from kreuzberg import register_ocr_backend
///
/// class MyOcrBackend:
///     def name(self) -> str:
///         return "my-ocr"
///
///     def supported_languages(self) -> list[str]:
///         return ["eng", "deu", "fra"]
///
///     def process_image(self, image_bytes: bytes, language: str) -> dict:
///         # Process image and extract text
///         return {
///             "content": "extracted text",
///             "metadata": {"confidence": 0.95},
///             "tables": []
///         }
///
/// register_ocr_backend("my-ocr", MyOcrBackend())
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Backend is missing required methods
/// - Backend name is empty or duplicate
/// - Registration fails
#[pyfunction]
pub fn register_ocr_backend(py: Python<'_>, backend: Py<PyAny>) -> PyResult<()> {
    let rust_backend = PythonOcrBackend::new(py, backend)?;
    let backend_name = rust_backend.name().to_string();

    let arc_backend: Arc<dyn OcrBackend> = Arc::new(rust_backend);

    py.detach(|| {
        let registry = get_ocr_backend_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock on OCR registry: {}", e))
        })?;

        registry.register(arc_backend).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to register OCR backend '{}': {}",
                backend_name, e
            ))
        })
    })?;

    Ok(())
}
