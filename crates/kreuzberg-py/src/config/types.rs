//! Configuration type bindings
//!
//! Provides Python-friendly wrappers around the Rust configuration structs.
//! All types support both construction and field access from Python.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::html_options::parse_html_options_dict;
use crate::keywords::KeywordConfig;

/// Main extraction configuration.
///
/// Controls all aspects of document extraction including OCR, PDF rendering,
/// chunking, caching, and post-processing.
///
/// Example:
///     >>> from kreuzberg import ExtractionConfig, OcrConfig
///     >>> config = ExtractionConfig(
///     ...     ocr=OcrConfig(language="eng"),
///     ...     use_cache=True
///     ... )
#[pyclass(name = "ExtractionConfig", module = "kreuzberg", from_py_object)]
#[derive(Default)]
pub struct ExtractionConfig {
    pub inner: kreuzberg::ExtractionConfig,
    pub html_options_dict: Option<Py<PyDict>>,
}

impl Clone for ExtractionConfig {
    fn clone(&self) -> Self {
        let html_options_dict = Python::attach(|py| self.html_options_dict.as_ref().map(|dict| dict.clone_ref(py)));
        Self {
            inner: self.inner.clone(),
            html_options_dict,
        }
    }
}

#[pymethods]
impl ExtractionConfig {
    #[new]
    #[pyo3(signature = (
        use_cache=None,
        enable_quality_processing=None,
        ocr=None,
        force_ocr=None,
        chunking=None,
        images=None,
        pdf_options=None,
        token_reduction=None,
        language_detection=None,
        keywords=None,
        postprocessor=None,
        html_options=None,
        max_concurrent_extractions=None,
        pages=None,
        result_format=None,
        output_format=None,
        include_document_structure=None,
        layout=None,
        acceleration=None,
        email=None,
        concurrency=None,
        cache_namespace=None,
        cache_ttl_secs=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        use_cache: Option<bool>,
        enable_quality_processing: Option<bool>,
        ocr: Option<OcrConfig>,
        force_ocr: Option<bool>,
        chunking: Option<ChunkingConfig>,
        images: Option<ImageExtractionConfig>,
        pdf_options: Option<PdfConfig>,
        token_reduction: Option<TokenReductionConfig>,
        language_detection: Option<LanguageDetectionConfig>,
        keywords: Option<KeywordConfig>,
        postprocessor: Option<PostProcessorConfig>,
        html_options: Option<Bound<'_, PyDict>>,
        max_concurrent_extractions: Option<usize>,
        pages: Option<PageConfig>,
        result_format: Option<String>,
        output_format: Option<String>,
        include_document_structure: Option<bool>,
        layout: Option<LayoutDetectionConfig>,
        acceleration: Option<AccelerationConfig>,
        email: Option<EmailConfig>,
        concurrency: Option<ConcurrencyConfig>,
        cache_namespace: Option<String>,
        cache_ttl_secs: Option<u64>,
    ) -> PyResult<Self> {
        let (html_options_inner, html_options_dict) = parse_html_options_dict(html_options)?;
        Ok(Self {
            inner: kreuzberg::ExtractionConfig {
                use_cache: use_cache.unwrap_or(true),
                enable_quality_processing: enable_quality_processing.unwrap_or(true),
                ocr: ocr.map(Into::into),
                force_ocr: force_ocr.unwrap_or(false),
                chunking: chunking.map(Into::into),
                images: images.map(Into::into),
                pdf_options: pdf_options.map(Into::into),
                token_reduction: token_reduction.map(Into::into),
                language_detection: language_detection.map(Into::into),
                keywords: keywords.map(Into::into),
                postprocessor: postprocessor.map(Into::into),
                html_options: html_options_inner,
                max_concurrent_extractions,
                pages: pages.map(Into::into),
                include_document_structure: include_document_structure.unwrap_or(false),
                result_format: if let Some(rf) = result_format {
                    match rf.to_lowercase().as_str() {
                        "unified" => kreuzberg::types::OutputFormat::Unified,
                        "element_based" | "element-based" => kreuzberg::types::OutputFormat::ElementBased,
                        other => {
                            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Invalid result_format: {}. Must be 'unified' or 'element_based'",
                                other
                            )));
                        }
                    }
                } else {
                    kreuzberg::types::OutputFormat::Unified
                },
                output_format: if let Some(of) = output_format {
                    match of.to_lowercase().as_str() {
                        "plain" | "text" => kreuzberg::core::config::formats::OutputFormat::Plain,
                        "markdown" | "md" => kreuzberg::core::config::formats::OutputFormat::Markdown,
                        "djot" => kreuzberg::core::config::formats::OutputFormat::Djot,
                        "html" => kreuzberg::core::config::formats::OutputFormat::Html,
                        "structured" | "json" => kreuzberg::core::config::formats::OutputFormat::Structured,
                        other => {
                            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Invalid output_format: {}. Must be 'plain', 'markdown', 'djot', 'html', or 'structured'",
                                other
                            )));
                        }
                    }
                } else {
                    kreuzberg::core::config::formats::OutputFormat::Plain
                },
                security_limits: None,
                layout: layout.map(Into::into),
                acceleration: acceleration.map(Into::into),
                cache_namespace,
                cache_ttl_secs,
                email: email.map(Into::into),
                concurrency: concurrency.map(Into::into),
            },
            html_options_dict,
        })
    }

    #[getter]
    fn use_cache(&self) -> bool {
        self.inner.use_cache
    }

    #[setter]
    fn set_use_cache(&mut self, value: bool) {
        self.inner.use_cache = value;
    }

    #[getter]
    fn enable_quality_processing(&self) -> bool {
        self.inner.enable_quality_processing
    }

    #[setter]
    fn set_enable_quality_processing(&mut self, value: bool) {
        self.inner.enable_quality_processing = value;
    }

    #[getter]
    fn ocr(&self) -> Option<OcrConfig> {
        self.inner.ocr.clone().map(Into::into)
    }

    #[setter]
    fn set_ocr(&mut self, value: Option<OcrConfig>) {
        self.inner.ocr = value.map(Into::into);
    }

    #[getter]
    fn force_ocr(&self) -> bool {
        self.inner.force_ocr
    }

    #[setter]
    fn set_force_ocr(&mut self, value: bool) {
        self.inner.force_ocr = value;
    }

    #[getter]
    fn include_document_structure(&self) -> bool {
        self.inner.include_document_structure
    }

    #[setter]
    fn set_include_document_structure(&mut self, value: bool) {
        self.inner.include_document_structure = value;
    }

    #[getter]
    fn chunking(&self) -> Option<ChunkingConfig> {
        self.inner.chunking.clone().map(Into::into)
    }

    #[setter]
    fn set_chunking(&mut self, value: Option<ChunkingConfig>) {
        self.inner.chunking = value.map(Into::into);
    }

    #[getter]
    fn images(&self) -> Option<ImageExtractionConfig> {
        self.inner.images.clone().map(Into::into)
    }

    #[setter]
    fn set_images(&mut self, value: Option<ImageExtractionConfig>) {
        self.inner.images = value.map(Into::into);
    }

    #[getter]
    fn pdf_options(&self) -> Option<PdfConfig> {
        self.inner.pdf_options.clone().map(Into::into)
    }

    #[setter]
    fn set_pdf_options(&mut self, value: Option<PdfConfig>) {
        self.inner.pdf_options = value.map(Into::into);
    }

    #[getter]
    fn token_reduction(&self) -> Option<TokenReductionConfig> {
        self.inner.token_reduction.clone().map(Into::into)
    }

    #[setter]
    fn set_token_reduction(&mut self, value: Option<TokenReductionConfig>) {
        self.inner.token_reduction = value.map(Into::into);
    }

    #[getter]
    fn language_detection(&self) -> Option<LanguageDetectionConfig> {
        self.inner.language_detection.clone().map(Into::into)
    }

    #[setter]
    fn set_language_detection(&mut self, value: Option<LanguageDetectionConfig>) {
        self.inner.language_detection = value.map(Into::into);
    }

    #[getter]
    fn keywords(&self) -> Option<KeywordConfig> {
        self.inner.keywords.clone().map(Into::into)
    }

    #[setter]
    fn set_keywords(&mut self, value: Option<KeywordConfig>) {
        self.inner.keywords = value.map(Into::into);
    }

    #[getter]
    fn postprocessor(&self) -> Option<PostProcessorConfig> {
        self.inner.postprocessor.clone().map(Into::into)
    }

    #[setter]
    fn set_postprocessor(&mut self, value: Option<PostProcessorConfig>) {
        self.inner.postprocessor = value.map(Into::into);
    }

    #[getter]
    fn max_concurrent_extractions(&self) -> Option<usize> {
        self.inner.max_concurrent_extractions
    }

    #[setter]
    fn set_max_concurrent_extractions(&mut self, value: Option<usize>) {
        self.inner.max_concurrent_extractions = value;
    }

    #[getter]
    fn html_options<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyDict>> {
        self.html_options_dict.as_ref().map(|dict| dict.bind(py).clone())
    }

    #[setter]
    fn set_html_options(&mut self, value: Option<Bound<'_, PyDict>>) -> PyResult<()> {
        let (parsed, stored) = parse_html_options_dict(value)?;
        self.inner.html_options = parsed;
        self.html_options_dict = stored;
        Ok(())
    }

    #[getter]
    fn pages(&self) -> Option<PageConfig> {
        self.inner.pages.clone().map(Into::into)
    }

    #[setter]
    fn set_pages(&mut self, value: Option<PageConfig>) {
        self.inner.pages = value.map(Into::into);
    }

    #[getter]
    fn result_format(&self) -> String {
        match self.inner.result_format {
            kreuzberg::types::OutputFormat::Unified => "unified".to_string(),
            kreuzberg::types::OutputFormat::ElementBased => "element_based".to_string(),
        }
    }

    #[setter]
    fn set_result_format(&mut self, value: String) {
        self.inner.result_format = match value.to_lowercase().as_str() {
            "unified" => kreuzberg::types::OutputFormat::Unified,
            "element_based" | "element-based" => kreuzberg::types::OutputFormat::ElementBased,
            _ => kreuzberg::types::OutputFormat::Unified, // Default on invalid
        };
    }

    #[getter]
    fn output_format(&self) -> String {
        match self.inner.output_format {
            kreuzberg::core::config::formats::OutputFormat::Plain => "plain".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Markdown => "markdown".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Djot => "djot".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Html => "html".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Structured => "structured".to_string(),
        }
    }

    #[setter]
    fn set_output_format(&mut self, value: String) {
        self.inner.output_format = match value.to_lowercase().as_str() {
            "plain" => kreuzberg::core::config::formats::OutputFormat::Plain,
            "markdown" => kreuzberg::core::config::formats::OutputFormat::Markdown,
            "djot" => kreuzberg::core::config::formats::OutputFormat::Djot,
            "html" => kreuzberg::core::config::formats::OutputFormat::Html,
            "structured" | "json" => kreuzberg::core::config::formats::OutputFormat::Structured,
            _ => kreuzberg::core::config::formats::OutputFormat::Plain, // Default on invalid
        };
    }

    #[getter]
    fn layout(&self) -> Option<LayoutDetectionConfig> {
        self.inner.layout.clone().map(Into::into)
    }

    #[setter]
    fn set_layout(&mut self, value: Option<LayoutDetectionConfig>) {
        self.inner.layout = value.map(Into::into);
    }

    #[getter]
    fn acceleration(&self) -> Option<AccelerationConfig> {
        self.inner.acceleration.clone().map(Into::into)
    }

    #[setter]
    fn set_acceleration(&mut self, value: Option<AccelerationConfig>) {
        self.inner.acceleration = value.map(Into::into);
    }

    #[getter]
    fn email(&self) -> Option<EmailConfig> {
        self.inner.email.clone().map(Into::into)
    }

    #[setter]
    fn set_email(&mut self, value: Option<EmailConfig>) {
        self.inner.email = value.map(Into::into);
    }

    #[getter]
    fn concurrency(&self) -> Option<ConcurrencyConfig> {
        self.inner.concurrency.clone().map(Into::into)
    }

    #[setter]
    fn set_concurrency(&mut self, value: Option<ConcurrencyConfig>) {
        self.inner.concurrency = value.map(Into::into);
    }

    #[getter]
    fn cache_namespace(&self) -> Option<String> {
        self.inner.cache_namespace.clone()
    }

    #[setter]
    fn set_cache_namespace(&mut self, value: Option<String>) {
        self.inner.cache_namespace = value;
    }

    #[getter]
    fn cache_ttl_secs(&self) -> Option<u64> {
        self.inner.cache_ttl_secs
    }

    #[setter]
    fn set_cache_ttl_secs(&mut self, value: Option<u64>) {
        self.inner.cache_ttl_secs = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "ExtractionConfig(use_cache={}, enable_quality_processing={}, ocr={}, force_ocr={})",
            self.inner.use_cache,
            self.inner.enable_quality_processing,
            if self.inner.ocr.is_some() { "Some(...)" } else { "None" },
            self.inner.force_ocr
        )
    }

    /// Load configuration from a file, auto-detecting format by extension.
    ///
    /// Supported formats:
    /// - `.toml` - TOML format
    /// - `.yaml` - YAML format
    /// - `.json` - JSON format
    ///
    /// Args:
    ///     path: Path to the configuration file (str or Path)
    ///
    /// Returns:
    ///     ExtractionConfig: Loaded configuration
    ///
    /// Raises:
    ///     ValidationError: If file doesn't exist, extension is not supported,
    ///                      or file content is invalid for the detected format
    ///
    /// Example:
    ///     >>> from kreuzberg import ExtractionConfig
    ///     >>> # Auto-detects TOML format
    ///     >>> config = ExtractionConfig.from_file("kreuzberg.toml")
    ///     >>> # Auto-detects YAML format
    ///     >>> config = ExtractionConfig.from_file("kreuzberg.yaml")
    ///     >>> # Works with pathlib.Path objects
    ///     >>> from pathlib import Path
    ///     >>> config = ExtractionConfig.from_file(Path("kreuzberg.toml"))
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        let config = kreuzberg::ExtractionConfig::from_file(path)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to load config: {}", e)))?;
        Ok(Self {
            inner: config,
            html_options_dict: None,
        })
    }

    /// Discover and load configuration from current or parent directories.
    ///
    /// Searches for a configuration file (kreuzberg.toml, kreuzberg.yaml, or kreuzberg.yml)
    /// in the current working directory and parent directories. Returns the first found
    /// configuration or a default configuration if none is found.
    ///
    /// Returns:
    ///     ExtractionConfig: Loaded configuration or default config
    ///
    /// Example:
    ///     >>> from kreuzberg import ExtractionConfig
    ///     >>> # Searches current and parent directories for config
    ///     >>> config = ExtractionConfig.discover()
    ///     >>> # Always returns a config (default if none found)
    ///     >>> assert config is not None
    #[staticmethod]
    fn discover() -> PyResult<Self> {
        let config = kreuzberg::ExtractionConfig::discover()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to discover config: {}", e)))?
            .unwrap_or_default();
        Ok(Self {
            inner: config,
            html_options_dict: None,
        })
    }
}

/// OCR configuration.
///
/// Example:
///     >>> from kreuzberg import OcrConfig
///     >>> config = OcrConfig(backend="tesseract", language="eng")
#[pyclass(name = "OcrConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct OcrConfig {
    pub inner: kreuzberg::OcrConfig,
}

#[pymethods]
impl OcrConfig {
    #[new]
    #[pyo3(signature = (backend=None, language=None, tesseract_config=None, paddle_ocr_config=None, element_config=None))]
    fn new(
        py: Python<'_>,
        backend: Option<String>,
        language: Option<String>,
        tesseract_config: Option<TesseractConfig>,
        paddle_ocr_config: Option<Bound<'_, pyo3::types::PyAny>>,
        element_config: Option<Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<Self> {
        let paddle_ocr_json = if let Some(obj) = paddle_ocr_config {
            let json_mod = py.import("json")?;
            let json_str: String = json_mod.call_method1("dumps", (&obj,))?.extract()?;
            Some(
                serde_json::from_str(&json_str)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid paddle_ocr_config: {e}")))?,
            )
        } else {
            None
        };
        let element_cfg = if let Some(obj) = element_config {
            let json_mod = py.import("json")?;
            let json_str: String = json_mod.call_method1("dumps", (&obj,))?.extract()?;
            Some(
                serde_json::from_str(&json_str)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid element_config: {e}")))?,
            )
        } else {
            None
        };
        Ok(Self {
            inner: kreuzberg::OcrConfig {
                backend: backend.unwrap_or_else(|| "tesseract".to_string()),
                language: language.unwrap_or_else(|| "eng".to_string()),
                tesseract_config: tesseract_config.map(Into::into),
                output_format: None,
                paddle_ocr_config: paddle_ocr_json,
                element_config: element_cfg,
                quality_thresholds: None,
                pipeline: None,
                auto_rotate: false,
            },
        })
    }

    #[getter]
    fn backend(&self) -> String {
        self.inner.backend.clone()
    }

    #[setter]
    fn set_backend(&mut self, value: String) {
        self.inner.backend = value;
    }

    #[getter]
    fn language(&self) -> String {
        self.inner.language.clone()
    }

    #[setter]
    fn set_language(&mut self, value: String) {
        self.inner.language = value;
    }

    #[getter]
    fn tesseract_config(&self) -> Option<TesseractConfig> {
        self.inner.tesseract_config.clone().map(Into::into)
    }

    #[setter]
    fn set_tesseract_config(&mut self, value: Option<TesseractConfig>) {
        self.inner.tesseract_config = value.map(Into::into);
    }

    fn __repr__(&self) -> String {
        format!(
            "OcrConfig(backend='{}', language='{}', tesseract_config={})",
            self.inner.backend,
            self.inner.language,
            if self.inner.tesseract_config.is_some() {
                "Some(...)"
            } else {
                "None"
            }
        )
    }
}

/// Embedding model type.
///
/// Specifies which model to use for embedding generation.
///
/// Available presets:
///     - "fast": AllMiniLML6V2Q (384 dimensions) - Quick prototyping, low-latency
///     - "balanced": BGEBaseENV15 (768 dimensions) - General-purpose RAG
///     - "quality": BGELargeENV15 (1024 dimensions) - High-quality embeddings
///     - "multilingual": MultilingualE5Base (768 dimensions) - Multi-language support
///
/// Example:
///     >>> from kreuzberg import EmbeddingModelType
///     >>> # Use a preset
///     >>> model = EmbeddingModelType.preset("balanced")
///     >>> # Use a custom model
///     >>> model = EmbeddingModelType.custom("my-model", 512)
#[pyclass(name = "EmbeddingModelType", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct EmbeddingModelType {
    pub inner: kreuzberg::EmbeddingModelType,
}

#[pymethods]
impl EmbeddingModelType {
    /// Create a model type from a preset name.
    #[staticmethod]
    fn preset(name: String) -> Self {
        Self {
            inner: kreuzberg::EmbeddingModelType::Preset { name },
        }
    }

    /// Create a custom ONNX model type.
    #[staticmethod]
    fn custom(model_id: String, dimensions: usize) -> Self {
        Self {
            inner: kreuzberg::EmbeddingModelType::Custom { model_id, dimensions },
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            kreuzberg::EmbeddingModelType::Preset { name } => format!("EmbeddingModelType.preset('{}')", name),
            kreuzberg::EmbeddingModelType::Custom { model_id, dimensions } => {
                format!("EmbeddingModelType.custom('{}', {})", model_id, dimensions)
            }
        }
    }
}

/// Embedding configuration.
///
/// Controls embedding generation for text chunks.
///
/// Attributes:
///     model (EmbeddingModelType): Model to use (default: preset("balanced"))
///     normalize (bool): Normalize embeddings to unit length (default: True)
///     batch_size (int): Batch size for embedding generation (default: 32)
///     show_download_progress (bool): Show model download progress (default: False)
///     cache_dir (str | None): Custom cache directory for models (default: None)
///
/// Example:
///     >>> from kreuzberg import EmbeddingConfig, EmbeddingModelType
///     >>> config = EmbeddingConfig(
///     ...     model=EmbeddingModelType.preset("balanced"),
///     ...     normalize=True,
///     ...     batch_size=32
///     ... )
#[pyclass(name = "EmbeddingConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct EmbeddingConfig {
    pub inner: kreuzberg::EmbeddingConfig,
}

#[pymethods]
impl EmbeddingConfig {
    #[new]
    #[pyo3(signature = (model=None, normalize=None, batch_size=None, show_download_progress=None, cache_dir=None))]
    fn new(
        model: Option<EmbeddingModelType>,
        normalize: Option<bool>,
        batch_size: Option<usize>,
        show_download_progress: Option<bool>,
        cache_dir: Option<String>,
    ) -> Self {
        Self {
            inner: kreuzberg::EmbeddingConfig {
                model: model.map(Into::into).unwrap_or(kreuzberg::EmbeddingModelType::Preset {
                    name: "balanced".to_string(),
                }),
                normalize: normalize.unwrap_or(true),
                batch_size: batch_size.unwrap_or(32),
                show_download_progress: show_download_progress.unwrap_or(false),
                cache_dir: cache_dir.map(std::path::PathBuf::from),
            },
        }
    }

    #[getter]
    fn normalize(&self) -> bool {
        self.inner.normalize
    }

    #[setter]
    fn set_normalize(&mut self, value: bool) {
        self.inner.normalize = value;
    }

    #[getter]
    fn batch_size(&self) -> usize {
        self.inner.batch_size
    }

    #[setter]
    fn set_batch_size(&mut self, value: usize) {
        self.inner.batch_size = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "EmbeddingConfig(normalize={}, batch_size={})",
            self.inner.normalize, self.inner.batch_size
        )
    }
}

/// Chunking configuration.
///
/// Controls how text is split into chunks with optional embedding generation.
///
/// Attributes:
///     max_chars (int): Maximum characters per chunk (default: 1000)
///     max_overlap (int): Overlap between chunks in characters (default: 200, must be < max_chars)
///     embedding (EmbeddingConfig | None): Embedding configuration (default: None)
///     preset (str | None): Chunking preset to use (default: None)
///
/// Important:
///     The max_overlap must be less than max_chars, otherwise a validation error will be raised.
///
/// Example:
///     >>> from kreuzberg import ChunkingConfig, EmbeddingConfig, EmbeddingModelType
///     >>> # Basic chunking without embeddings
///     >>> basic = ChunkingConfig(max_chars=1000, max_overlap=200)
///     >>>
///     >>> # Chunking with embeddings
///     >>> embedding = EmbeddingConfig(
///     ...     model=EmbeddingModelType.preset("fast"),
///     ...     normalize=True
///     ... )
///     >>> config = ChunkingConfig(
///     ...     max_chars=2000,
///     ...     max_overlap=400,
///     ...     embedding=embedding
///     ... )
#[pyclass(name = "ChunkingConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct ChunkingConfig {
    pub inner: kreuzberg::ChunkingConfig,
}

#[pymethods]
impl ChunkingConfig {
    #[new]
    #[pyo3(signature = (max_chars=None, max_overlap=None, embedding=None, preset=None, chunker_type=None, sizing_type=None, sizing_model=None, sizing_cache_dir=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        max_chars: Option<usize>,
        max_overlap: Option<usize>,
        embedding: Option<EmbeddingConfig>,
        preset: Option<String>,
        chunker_type: Option<String>,
        sizing_type: Option<String>,
        sizing_model: Option<String>,
        sizing_cache_dir: Option<String>,
    ) -> Self {
        let ct = match chunker_type.as_deref() {
            Some("markdown") => kreuzberg::ChunkerType::Markdown,
            _ => kreuzberg::ChunkerType::Text,
        };
        let sizing = Self::resolve_sizing(sizing_type, sizing_model, sizing_cache_dir);
        Self {
            inner: kreuzberg::ChunkingConfig {
                max_characters: max_chars.unwrap_or(1000),
                overlap: max_overlap.unwrap_or(200),
                trim: true,
                chunker_type: ct,
                embedding: embedding.map(Into::into),
                preset,
                sizing,
            },
        }
    }

    #[getter]
    fn max_chars(&self) -> usize {
        self.inner.max_characters
    }

    #[setter]
    fn set_max_chars(&mut self, value: usize) {
        self.inner.max_characters = value;
    }

    #[getter]
    fn max_overlap(&self) -> usize {
        self.inner.overlap
    }

    #[setter]
    fn set_max_overlap(&mut self, value: usize) {
        self.inner.overlap = value;
    }

    #[getter]
    fn embedding(&self) -> Option<EmbeddingConfig> {
        self.inner.embedding.clone().map(Into::into)
    }

    #[setter]
    fn set_embedding(&mut self, value: Option<EmbeddingConfig>) {
        self.inner.embedding = value.map(Into::into);
    }

    #[getter]
    fn preset(&self) -> Option<String> {
        self.inner.preset.clone()
    }

    #[setter]
    fn set_preset(&mut self, value: Option<String>) {
        self.inner.preset = value;
    }

    #[getter]
    fn sizing_type(&self) -> String {
        match &self.inner.sizing {
            kreuzberg::ChunkSizing::Characters => "characters".to_string(),
            kreuzberg::ChunkSizing::Tokenizer { .. } => "tokenizer".to_string(),
        }
    }

    #[getter]
    fn sizing_model(&self) -> Option<String> {
        match &self.inner.sizing {
            kreuzberg::ChunkSizing::Tokenizer { model, .. } => Some(model.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ChunkingConfig(max_chars={}, max_overlap={}, embedding={}, preset={})",
            self.inner.max_characters,
            self.inner.overlap,
            if self.inner.embedding.is_some() { "..." } else { "None" },
            self.inner
                .preset
                .as_ref()
                .map(|s| format!("'{}'", s))
                .unwrap_or_else(|| "None".to_string())
        )
    }
}

impl ChunkingConfig {
    fn resolve_sizing(
        sizing_type: Option<String>,
        sizing_model: Option<String>,
        sizing_cache_dir: Option<String>,
    ) -> kreuzberg::ChunkSizing {
        match sizing_type.as_deref() {
            Some("tokenizer") => kreuzberg::ChunkSizing::Tokenizer {
                model: sizing_model.unwrap_or_else(|| "Xenova/gpt-4o".to_string()),
                cache_dir: sizing_cache_dir.map(std::path::PathBuf::from),
            },
            _ => kreuzberg::ChunkSizing::Characters,
        }
    }
}

/// Image extraction configuration.
///
/// Example:
///     >>> from kreuzberg import ImageExtractionConfig
///     >>> config = ImageExtractionConfig(target_dpi=300, max_image_dimension=4096)
#[pyclass(name = "ImageExtractionConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct ImageExtractionConfig {
    pub inner: kreuzberg::ImageExtractionConfig,
}

#[pymethods]
impl ImageExtractionConfig {
    #[new]
    #[pyo3(signature = (
        extract_images=None,
        target_dpi=None,
        max_image_dimension=None,
        inject_placeholders=None,
        auto_adjust_dpi=None,
        min_dpi=None,
        max_dpi=None
    ))]
    fn new(
        extract_images: Option<bool>,
        target_dpi: Option<i32>,
        max_image_dimension: Option<i32>,
        inject_placeholders: Option<bool>,
        auto_adjust_dpi: Option<bool>,
        min_dpi: Option<i32>,
        max_dpi: Option<i32>,
    ) -> Self {
        Self {
            inner: kreuzberg::ImageExtractionConfig {
                extract_images: extract_images.unwrap_or(true),
                target_dpi: target_dpi.unwrap_or(300),
                max_image_dimension: max_image_dimension.unwrap_or(4096),
                inject_placeholders: inject_placeholders.unwrap_or(true),
                auto_adjust_dpi: auto_adjust_dpi.unwrap_or(true),
                min_dpi: min_dpi.unwrap_or(72),
                max_dpi: max_dpi.unwrap_or(600),
            },
        }
    }

    #[getter]
    fn extract_images(&self) -> bool {
        self.inner.extract_images
    }

    #[setter]
    fn set_extract_images(&mut self, value: bool) {
        self.inner.extract_images = value;
    }

    #[getter]
    fn target_dpi(&self) -> i32 {
        self.inner.target_dpi
    }

    #[setter]
    fn set_target_dpi(&mut self, value: i32) {
        self.inner.target_dpi = value;
    }

    #[getter]
    fn max_image_dimension(&self) -> i32 {
        self.inner.max_image_dimension
    }

    #[setter]
    fn set_max_image_dimension(&mut self, value: i32) {
        self.inner.max_image_dimension = value;
    }

    #[getter]
    fn inject_placeholders(&self) -> bool {
        self.inner.inject_placeholders
    }

    #[setter]
    fn set_inject_placeholders(&mut self, value: bool) {
        self.inner.inject_placeholders = value;
    }

    #[getter]
    fn auto_adjust_dpi(&self) -> bool {
        self.inner.auto_adjust_dpi
    }

    #[setter]
    fn set_auto_adjust_dpi(&mut self, value: bool) {
        self.inner.auto_adjust_dpi = value;
    }

    #[getter]
    fn min_dpi(&self) -> i32 {
        self.inner.min_dpi
    }

    #[setter]
    fn set_min_dpi(&mut self, value: i32) {
        self.inner.min_dpi = value;
    }

    #[getter]
    fn max_dpi(&self) -> i32 {
        self.inner.max_dpi
    }

    #[setter]
    fn set_max_dpi(&mut self, value: i32) {
        self.inner.max_dpi = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "ImageExtractionConfig(extract_images={}, target_dpi={}, max_image_dimension={})",
            self.inner.extract_images, self.inner.target_dpi, self.inner.max_image_dimension
        )
    }
}

/// PDF-specific configuration.
///
/// Example:
///     >>> from kreuzberg import PdfConfig
///     >>> config = PdfConfig(extract_images=True, passwords=["pass1", "pass2"])
#[pyclass(name = "PdfConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct PdfConfig {
    pub inner: kreuzberg::PdfConfig,
}

#[pymethods]
impl PdfConfig {
    #[new]
    #[pyo3(signature = (extract_images=None, passwords=None, extract_metadata=None, hierarchy=None, extract_annotations=None, top_margin_fraction=None, bottom_margin_fraction=None, allow_single_column_tables=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        extract_images: Option<bool>,
        passwords: Option<Vec<String>>,
        extract_metadata: Option<bool>,
        hierarchy: Option<HierarchyConfig>,
        extract_annotations: Option<bool>,
        top_margin_fraction: Option<f32>,
        bottom_margin_fraction: Option<f32>,
        allow_single_column_tables: Option<bool>,
    ) -> Self {
        Self {
            inner: kreuzberg::PdfConfig {
                extract_images: extract_images.unwrap_or(false),
                passwords,
                extract_metadata: extract_metadata.unwrap_or(true),
                hierarchy: hierarchy.map(|h| h.inner),
                extract_annotations: extract_annotations.unwrap_or(false),
                top_margin_fraction,
                bottom_margin_fraction,
                allow_single_column_tables: allow_single_column_tables.unwrap_or(false),
            },
        }
    }

    #[getter]
    fn extract_images(&self) -> bool {
        self.inner.extract_images
    }

    #[setter]
    fn set_extract_images(&mut self, value: bool) {
        self.inner.extract_images = value;
    }

    #[getter]
    fn passwords(&self) -> Option<Vec<String>> {
        self.inner.passwords.clone()
    }

    #[setter]
    fn set_passwords(&mut self, value: Option<Vec<String>>) {
        self.inner.passwords = value;
    }

    #[getter]
    fn extract_metadata(&self) -> bool {
        self.inner.extract_metadata
    }

    #[setter]
    fn set_extract_metadata(&mut self, value: bool) {
        self.inner.extract_metadata = value;
    }

    #[getter]
    fn hierarchy(&self) -> Option<HierarchyConfig> {
        self.inner.hierarchy.clone().map(Into::into)
    }

    #[setter]
    fn set_hierarchy(&mut self, value: Option<HierarchyConfig>) {
        self.inner.hierarchy = value.map(|h| h.inner);
    }

    #[getter]
    fn extract_annotations(&self) -> bool {
        self.inner.extract_annotations
    }

    #[setter]
    fn set_extract_annotations(&mut self, value: bool) {
        self.inner.extract_annotations = value;
    }

    #[getter]
    fn top_margin_fraction(&self) -> Option<f32> {
        self.inner.top_margin_fraction
    }

    #[setter]
    fn set_top_margin_fraction(&mut self, value: Option<f32>) {
        self.inner.top_margin_fraction = value;
    }

    #[getter]
    fn bottom_margin_fraction(&self) -> Option<f32> {
        self.inner.bottom_margin_fraction
    }

    #[setter]
    fn set_bottom_margin_fraction(&mut self, value: Option<f32>) {
        self.inner.bottom_margin_fraction = value;
    }

    #[getter]
    fn allow_single_column_tables(&self) -> bool {
        self.inner.allow_single_column_tables
    }

    #[setter]
    fn set_allow_single_column_tables(&mut self, value: bool) {
        self.inner.allow_single_column_tables = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "PdfConfig(extract_images={}, extract_metadata={}, passwords={})",
            self.inner.extract_images,
            self.inner.extract_metadata,
            if self.inner.passwords.is_some() {
                "Some([...])"
            } else {
                "None"
            }
        )
    }
}

/// Token reduction configuration.
///
/// Example:
///     >>> from kreuzberg import TokenReductionConfig
///     >>> config = TokenReductionConfig(mode="aggressive", preserve_important_words=True)
#[pyclass(name = "TokenReductionConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct TokenReductionConfig {
    pub inner: kreuzberg::TokenReductionConfig,
}

#[pymethods]
impl TokenReductionConfig {
    #[new]
    #[pyo3(signature = (mode=None, preserve_important_words=None))]
    fn new(mode: Option<String>, preserve_important_words: Option<bool>) -> Self {
        Self {
            inner: kreuzberg::TokenReductionConfig {
                mode: mode.unwrap_or_else(|| "off".to_string()),
                preserve_important_words: preserve_important_words.unwrap_or(true),
            },
        }
    }

    #[getter]
    fn mode(&self) -> String {
        self.inner.mode.clone()
    }

    #[setter]
    fn set_mode(&mut self, value: String) {
        self.inner.mode = value;
    }

    #[getter]
    fn preserve_important_words(&self) -> bool {
        self.inner.preserve_important_words
    }

    #[setter]
    fn set_preserve_important_words(&mut self, value: bool) {
        self.inner.preserve_important_words = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "TokenReductionConfig(mode='{}', preserve_important_words={})",
            self.inner.mode, self.inner.preserve_important_words
        )
    }
}

/// Language detection configuration.
///
/// Example:
///     >>> from kreuzberg import LanguageDetectionConfig
///     >>> config = LanguageDetectionConfig(enabled=True, min_confidence=0.9)
#[pyclass(name = "LanguageDetectionConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct LanguageDetectionConfig {
    pub inner: kreuzberg::LanguageDetectionConfig,
}

#[pymethods]
impl LanguageDetectionConfig {
    #[new]
    #[pyo3(signature = (enabled=None, min_confidence=None, detect_multiple=None))]
    fn new(enabled: Option<bool>, min_confidence: Option<f64>, detect_multiple: Option<bool>) -> Self {
        Self {
            inner: kreuzberg::LanguageDetectionConfig {
                enabled: enabled.unwrap_or(true),
                min_confidence: min_confidence.unwrap_or(0.8),
                detect_multiple: detect_multiple.unwrap_or(false),
            },
        }
    }

    #[getter]
    fn enabled(&self) -> bool {
        self.inner.enabled
    }

    #[setter]
    fn set_enabled(&mut self, value: bool) {
        self.inner.enabled = value;
    }

    #[getter]
    fn min_confidence(&self) -> f64 {
        self.inner.min_confidence
    }

    #[setter]
    fn set_min_confidence(&mut self, value: f64) {
        self.inner.min_confidence = value;
    }

    #[getter]
    fn detect_multiple(&self) -> bool {
        self.inner.detect_multiple
    }

    #[setter]
    fn set_detect_multiple(&mut self, value: bool) {
        self.inner.detect_multiple = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "LanguageDetectionConfig(enabled={}, min_confidence={}, detect_multiple={})",
            self.inner.enabled, self.inner.min_confidence, self.inner.detect_multiple
        )
    }
}

/// Post-processor configuration.
///
/// Example:
///     >>> from kreuzberg import PostProcessorConfig
///     >>> config = PostProcessorConfig(enabled=True, enabled_processors=["entity_extraction"])
#[pyclass(name = "PostProcessorConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct PostProcessorConfig {
    pub inner: kreuzberg::PostProcessorConfig,
}

#[pymethods]
impl PostProcessorConfig {
    #[new]
    #[pyo3(signature = (enabled=None, enabled_processors=None, disabled_processors=None))]
    fn new(
        enabled: Option<bool>,
        enabled_processors: Option<Vec<String>>,
        disabled_processors: Option<Vec<String>>,
    ) -> Self {
        let enabled_set = enabled_processors.as_ref().map(|procs| procs.iter().cloned().collect());
        let disabled_set = disabled_processors
            .as_ref()
            .map(|procs| procs.iter().cloned().collect());

        Self {
            inner: kreuzberg::PostProcessorConfig {
                enabled: enabled.unwrap_or(true),
                enabled_processors,
                disabled_processors,
                enabled_set,
                disabled_set,
            },
        }
    }

    #[getter]
    fn enabled(&self) -> bool {
        self.inner.enabled
    }

    #[setter]
    fn set_enabled(&mut self, value: bool) {
        self.inner.enabled = value;
    }

    #[getter]
    fn enabled_processors(&self) -> Option<Vec<String>> {
        self.inner.enabled_processors.clone()
    }

    #[setter]
    fn set_enabled_processors(&mut self, value: Option<Vec<String>>) {
        self.inner.enabled_processors = value;
    }

    #[getter]
    fn disabled_processors(&self) -> Option<Vec<String>> {
        self.inner.disabled_processors.clone()
    }

    #[setter]
    fn set_disabled_processors(&mut self, value: Option<Vec<String>>) {
        self.inner.disabled_processors = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "PostProcessorConfig(enabled={}, enabled_processors={:?}, disabled_processors={:?})",
            self.inner.enabled, self.inner.enabled_processors, self.inner.disabled_processors
        )
    }
}

/// Layout detection configuration.
///
/// Controls layout detection behavior for PDF extraction using ONNX-based
/// document layout models (YOLO or RT-DETR).
///
/// Example:
///     >>> from kreuzberg import LayoutDetectionConfig
///     >>> config = LayoutDetectionConfig(preset="fast", apply_heuristics=True)
#[pyclass(name = "LayoutDetectionConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct LayoutDetectionConfig {
    pub inner: kreuzberg::core::config::layout::LayoutDetectionConfig,
}

#[pymethods]
impl LayoutDetectionConfig {
    #[new]
    #[pyo3(signature = (preset=None, confidence_threshold=None, apply_heuristics=None))]
    fn new(preset: Option<String>, confidence_threshold: Option<f32>, apply_heuristics: Option<bool>) -> Self {
        Self {
            inner: kreuzberg::core::config::layout::LayoutDetectionConfig {
                preset: preset.unwrap_or_else(|| "fast".to_string()),
                confidence_threshold,
                apply_heuristics: apply_heuristics.unwrap_or(true),
            },
        }
    }

    #[getter]
    fn preset(&self) -> String {
        self.inner.preset.clone()
    }

    #[setter]
    fn set_preset(&mut self, value: String) {
        self.inner.preset = value;
    }

    #[getter]
    fn confidence_threshold(&self) -> Option<f32> {
        self.inner.confidence_threshold
    }

    #[setter]
    fn set_confidence_threshold(&mut self, value: Option<f32>) {
        self.inner.confidence_threshold = value;
    }

    #[getter]
    fn apply_heuristics(&self) -> bool {
        self.inner.apply_heuristics
    }

    #[setter]
    fn set_apply_heuristics(&mut self, value: bool) {
        self.inner.apply_heuristics = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "LayoutDetectionConfig(preset='{}', confidence_threshold={}, apply_heuristics={})",
            self.inner.preset,
            self.inner
                .confidence_threshold
                .map(|v| v.to_string())
                .unwrap_or_else(|| "None".to_string()),
            self.inner.apply_heuristics
        )
    }
}

impl From<LayoutDetectionConfig> for kreuzberg::core::config::layout::LayoutDetectionConfig {
    fn from(config: LayoutDetectionConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::core::config::layout::LayoutDetectionConfig> for LayoutDetectionConfig {
    fn from(config: kreuzberg::core::config::layout::LayoutDetectionConfig) -> Self {
        Self { inner: config }
    }
}

/// Image preprocessing configuration for OCR.
///
/// Controls how images are preprocessed before OCR to improve text recognition.
///
/// Example:
///     >>> from kreuzberg import ImagePreprocessingConfig
///     >>> config = ImagePreprocessingConfig(
///     ...     target_dpi=600,
///     ...     denoise=True,
///     ...     contrast_enhance=True
///     ... )
#[pyclass(name = "ImagePreprocessingConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct ImagePreprocessingConfig {
    pub inner: kreuzberg::types::ImagePreprocessingConfig,
}

#[pymethods]
impl ImagePreprocessingConfig {
    #[new]
    #[pyo3(signature = (
        target_dpi=None,
        auto_rotate=None,
        deskew=None,
        denoise=None,
        contrast_enhance=None,
        binarization_method=None,
        invert_colors=None
    ))]
    fn new(
        target_dpi: Option<i32>,
        auto_rotate: Option<bool>,
        deskew: Option<bool>,
        denoise: Option<bool>,
        contrast_enhance: Option<bool>,
        binarization_method: Option<String>,
        invert_colors: Option<bool>,
    ) -> Self {
        Self {
            inner: kreuzberg::types::ImagePreprocessingConfig {
                target_dpi: target_dpi.unwrap_or(300),
                auto_rotate: auto_rotate.unwrap_or(true),
                deskew: deskew.unwrap_or(true),
                denoise: denoise.unwrap_or(false),
                contrast_enhance: contrast_enhance.unwrap_or(false),
                binarization_method: binarization_method.unwrap_or_else(|| "otsu".to_string()),
                invert_colors: invert_colors.unwrap_or(false),
            },
        }
    }

    #[getter]
    fn target_dpi(&self) -> i32 {
        self.inner.target_dpi
    }

    #[setter]
    fn set_target_dpi(&mut self, value: i32) {
        self.inner.target_dpi = value;
    }

    #[getter]
    fn auto_rotate(&self) -> bool {
        self.inner.auto_rotate
    }

    #[setter]
    fn set_auto_rotate(&mut self, value: bool) {
        self.inner.auto_rotate = value;
    }

    #[getter]
    fn deskew(&self) -> bool {
        self.inner.deskew
    }

    #[setter]
    fn set_deskew(&mut self, value: bool) {
        self.inner.deskew = value;
    }

    #[getter]
    fn denoise(&self) -> bool {
        self.inner.denoise
    }

    #[setter]
    fn set_denoise(&mut self, value: bool) {
        self.inner.denoise = value;
    }

    #[getter]
    fn contrast_enhance(&self) -> bool {
        self.inner.contrast_enhance
    }

    #[setter]
    fn set_contrast_enhance(&mut self, value: bool) {
        self.inner.contrast_enhance = value;
    }

    #[getter]
    fn binarization_method(&self) -> String {
        self.inner.binarization_method.clone()
    }

    #[setter]
    fn set_binarization_method(&mut self, value: String) {
        self.inner.binarization_method = value;
    }

    #[getter]
    fn invert_colors(&self) -> bool {
        self.inner.invert_colors
    }

    #[setter]
    fn set_invert_colors(&mut self, value: bool) {
        self.inner.invert_colors = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "ImagePreprocessingConfig(target_dpi={}, auto_rotate={}, deskew={}, denoise={})",
            self.inner.target_dpi, self.inner.auto_rotate, self.inner.deskew, self.inner.denoise
        )
    }
}

/// Tesseract OCR configuration.
///
/// Provides fine-grained control over Tesseract OCR behavior including
/// page segmentation mode, table detection, and various Tesseract-specific options.
///
/// Example:
///     >>> from kreuzberg import TesseractConfig
///     >>> config = TesseractConfig(
///     ...     language="eng",
///     ...     psm=6,
///     ...     enable_table_detection=True,
///     ...     tessedit_char_whitelist="0123456789"
///     ... )
#[pyclass(name = "TesseractConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct TesseractConfig {
    pub inner: kreuzberg::types::TesseractConfig,
}

#[pymethods]
impl TesseractConfig {
    #[new]
    #[pyo3(signature = (
        language=None,
        psm=None,
        output_format=None,
        oem=None,
        min_confidence=None,
        preprocessing=None,
        enable_table_detection=None,
        table_min_confidence=None,
        table_column_threshold=None,
        table_row_threshold_ratio=None,
        use_cache=None,
        classify_use_pre_adapted_templates=None,
        language_model_ngram_on=None,
        tessedit_dont_blkrej_good_wds=None,
        tessedit_dont_rowrej_good_wds=None,
        tessedit_enable_dict_correction=None,
        tessedit_char_whitelist=None,
        tessedit_char_blacklist=None,
        tessedit_use_primary_params_model=None,
        textord_space_size_is_variable=None,
        thresholding_method=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        language: Option<String>,
        psm: Option<i32>,
        output_format: Option<String>,
        oem: Option<i32>,
        min_confidence: Option<f64>,
        preprocessing: Option<ImagePreprocessingConfig>,
        enable_table_detection: Option<bool>,
        table_min_confidence: Option<f64>,
        table_column_threshold: Option<i32>,
        table_row_threshold_ratio: Option<f64>,
        use_cache: Option<bool>,
        classify_use_pre_adapted_templates: Option<bool>,
        language_model_ngram_on: Option<bool>,
        tessedit_dont_blkrej_good_wds: Option<bool>,
        tessedit_dont_rowrej_good_wds: Option<bool>,
        tessedit_enable_dict_correction: Option<bool>,
        tessedit_char_whitelist: Option<String>,
        tessedit_char_blacklist: Option<String>,
        tessedit_use_primary_params_model: Option<bool>,
        textord_space_size_is_variable: Option<bool>,
        thresholding_method: Option<bool>,
    ) -> Self {
        Self {
            inner: kreuzberg::types::TesseractConfig {
                language: language.unwrap_or_else(|| "eng".to_string()),
                psm: psm.unwrap_or(3),
                output_format: output_format.unwrap_or_else(|| "markdown".to_string()),
                oem: oem.unwrap_or(3),
                min_confidence: min_confidence.unwrap_or(0.0),
                preprocessing: preprocessing.map(Into::into),
                enable_table_detection: enable_table_detection.unwrap_or(true),
                table_min_confidence: table_min_confidence.unwrap_or(0.0),
                table_column_threshold: table_column_threshold.unwrap_or(50),
                table_row_threshold_ratio: table_row_threshold_ratio.unwrap_or(0.5),
                use_cache: use_cache.unwrap_or(true),
                classify_use_pre_adapted_templates: classify_use_pre_adapted_templates.unwrap_or(true),
                language_model_ngram_on: language_model_ngram_on.unwrap_or(false),
                tessedit_dont_blkrej_good_wds: tessedit_dont_blkrej_good_wds.unwrap_or(true),
                tessedit_dont_rowrej_good_wds: tessedit_dont_rowrej_good_wds.unwrap_or(true),
                tessedit_enable_dict_correction: tessedit_enable_dict_correction.unwrap_or(true),
                tessedit_char_whitelist: tessedit_char_whitelist.unwrap_or_default(),
                tessedit_char_blacklist: tessedit_char_blacklist.unwrap_or_default(),
                tessedit_use_primary_params_model: tessedit_use_primary_params_model.unwrap_or(true),
                textord_space_size_is_variable: textord_space_size_is_variable.unwrap_or(true),
                thresholding_method: thresholding_method.unwrap_or(false),
            },
        }
    }

    #[getter]
    fn language(&self) -> String {
        self.inner.language.clone()
    }

    #[setter]
    fn set_language(&mut self, value: String) {
        self.inner.language = value;
    }

    #[getter]
    fn psm(&self) -> i32 {
        self.inner.psm
    }

    #[setter]
    fn set_psm(&mut self, value: i32) {
        self.inner.psm = value;
    }

    #[getter]
    fn output_format(&self) -> String {
        self.inner.output_format.clone()
    }

    #[setter]
    fn set_output_format(&mut self, value: String) {
        self.inner.output_format = value;
    }

    #[getter]
    fn oem(&self) -> i32 {
        self.inner.oem
    }

    #[setter]
    fn set_oem(&mut self, value: i32) {
        self.inner.oem = value;
    }

    #[getter]
    fn min_confidence(&self) -> f64 {
        self.inner.min_confidence
    }

    #[setter]
    fn set_min_confidence(&mut self, value: f64) {
        self.inner.min_confidence = value;
    }

    #[getter]
    fn preprocessing(&self) -> Option<ImagePreprocessingConfig> {
        self.inner.preprocessing.clone().map(Into::into)
    }

    #[setter]
    fn set_preprocessing(&mut self, value: Option<ImagePreprocessingConfig>) {
        self.inner.preprocessing = value.map(Into::into);
    }

    #[getter]
    fn enable_table_detection(&self) -> bool {
        self.inner.enable_table_detection
    }

    #[setter]
    fn set_enable_table_detection(&mut self, value: bool) {
        self.inner.enable_table_detection = value;
    }

    #[getter]
    fn table_min_confidence(&self) -> f64 {
        self.inner.table_min_confidence
    }

    #[setter]
    fn set_table_min_confidence(&mut self, value: f64) {
        self.inner.table_min_confidence = value;
    }

    #[getter]
    fn table_column_threshold(&self) -> i32 {
        self.inner.table_column_threshold
    }

    #[setter]
    fn set_table_column_threshold(&mut self, value: i32) {
        self.inner.table_column_threshold = value;
    }

    #[getter]
    fn table_row_threshold_ratio(&self) -> f64 {
        self.inner.table_row_threshold_ratio
    }

    #[setter]
    fn set_table_row_threshold_ratio(&mut self, value: f64) {
        self.inner.table_row_threshold_ratio = value;
    }

    #[getter]
    fn use_cache(&self) -> bool {
        self.inner.use_cache
    }

    #[setter]
    fn set_use_cache(&mut self, value: bool) {
        self.inner.use_cache = value;
    }

    #[getter]
    fn classify_use_pre_adapted_templates(&self) -> bool {
        self.inner.classify_use_pre_adapted_templates
    }

    #[setter]
    fn set_classify_use_pre_adapted_templates(&mut self, value: bool) {
        self.inner.classify_use_pre_adapted_templates = value;
    }

    #[getter]
    fn language_model_ngram_on(&self) -> bool {
        self.inner.language_model_ngram_on
    }

    #[setter]
    fn set_language_model_ngram_on(&mut self, value: bool) {
        self.inner.language_model_ngram_on = value;
    }

    #[getter]
    fn tessedit_dont_blkrej_good_wds(&self) -> bool {
        self.inner.tessedit_dont_blkrej_good_wds
    }

    #[setter]
    fn set_tessedit_dont_blkrej_good_wds(&mut self, value: bool) {
        self.inner.tessedit_dont_blkrej_good_wds = value;
    }

    #[getter]
    fn tessedit_dont_rowrej_good_wds(&self) -> bool {
        self.inner.tessedit_dont_rowrej_good_wds
    }

    #[setter]
    fn set_tessedit_dont_rowrej_good_wds(&mut self, value: bool) {
        self.inner.tessedit_dont_rowrej_good_wds = value;
    }

    #[getter]
    fn tessedit_enable_dict_correction(&self) -> bool {
        self.inner.tessedit_enable_dict_correction
    }

    #[setter]
    fn set_tessedit_enable_dict_correction(&mut self, value: bool) {
        self.inner.tessedit_enable_dict_correction = value;
    }

    #[getter]
    fn tessedit_char_whitelist(&self) -> String {
        self.inner.tessedit_char_whitelist.clone()
    }

    #[setter]
    fn set_tessedit_char_whitelist(&mut self, value: String) {
        self.inner.tessedit_char_whitelist = value;
    }

    #[getter]
    fn tessedit_char_blacklist(&self) -> String {
        self.inner.tessedit_char_blacklist.clone()
    }

    #[setter]
    fn set_tessedit_char_blacklist(&mut self, value: String) {
        self.inner.tessedit_char_blacklist = value;
    }

    #[getter]
    fn tessedit_use_primary_params_model(&self) -> bool {
        self.inner.tessedit_use_primary_params_model
    }

    #[setter]
    fn set_tessedit_use_primary_params_model(&mut self, value: bool) {
        self.inner.tessedit_use_primary_params_model = value;
    }

    #[getter]
    fn textord_space_size_is_variable(&self) -> bool {
        self.inner.textord_space_size_is_variable
    }

    #[setter]
    fn set_textord_space_size_is_variable(&mut self, value: bool) {
        self.inner.textord_space_size_is_variable = value;
    }

    #[getter]
    fn thresholding_method(&self) -> bool {
        self.inner.thresholding_method
    }

    #[setter]
    fn set_thresholding_method(&mut self, value: bool) {
        self.inner.thresholding_method = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "TesseractConfig(language='{}', psm={}, output_format='{}', enable_table_detection={})",
            self.inner.language, self.inner.psm, self.inner.output_format, self.inner.enable_table_detection
        )
    }
}

/// Page extraction and tracking configuration.
///
/// Controls how pages are extracted, tracked, and represented in the extraction results.
///
/// Example:
///     >>> from kreuzberg import PageConfig
///     >>> config = PageConfig(extract_pages=True, insert_page_markers=True)
#[pyclass(name = "PageConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct PageConfig {
    pub inner: kreuzberg::core::config::PageConfig,
}

#[pymethods]
impl PageConfig {
    #[new]
    #[pyo3(signature = (extract_pages=None, insert_page_markers=None, marker_format=None))]
    fn new(extract_pages: Option<bool>, insert_page_markers: Option<bool>, marker_format: Option<String>) -> Self {
        Self {
            inner: kreuzberg::core::config::PageConfig {
                extract_pages: extract_pages.unwrap_or(false),
                insert_page_markers: insert_page_markers.unwrap_or(false),
                marker_format: marker_format.unwrap_or_else(|| "\n\n<!-- PAGE {page_num} -->\n\n".to_string()),
            },
        }
    }

    #[getter]
    fn extract_pages(&self) -> bool {
        self.inner.extract_pages
    }

    #[setter]
    fn set_extract_pages(&mut self, value: bool) {
        self.inner.extract_pages = value;
    }

    #[getter]
    fn insert_page_markers(&self) -> bool {
        self.inner.insert_page_markers
    }

    #[setter]
    fn set_insert_page_markers(&mut self, value: bool) {
        self.inner.insert_page_markers = value;
    }

    #[getter]
    fn marker_format(&self) -> String {
        self.inner.marker_format.clone()
    }

    #[setter]
    fn set_marker_format(&mut self, value: String) {
        self.inner.marker_format = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "PageConfig(extract_pages={}, insert_page_markers={}, marker_format='{}')",
            self.inner.extract_pages, self.inner.insert_page_markers, self.inner.marker_format
        )
    }
}

/// Hardware acceleration configuration for ONNX Runtime.
///
/// Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
/// for inference in layout detection and embedding generation.
///
/// Attributes:
///     provider (str): Execution provider ("auto", "cpu", "coreml", "cuda", "tensorrt"). Default: "auto"
///     device_id (int): GPU device ID for CUDA/TensorRT. Default: 0
///
/// Example:
///     >>> from kreuzberg import AccelerationConfig
///     >>> # Auto-select provider per platform
///     >>> config = AccelerationConfig()
///     >>>
///     >>> # Force CPU-only
///     >>> config = AccelerationConfig(provider="cpu")
///     >>>
///     >>> # Use CUDA on device 1
///     >>> config = AccelerationConfig(provider="cuda", device_id=1)
#[pyclass(name = "AccelerationConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct AccelerationConfig {
    pub inner: kreuzberg::AccelerationConfig,
}

#[pymethods]
impl AccelerationConfig {
    #[new]
    #[pyo3(signature = (provider=None, device_id=None))]
    fn new(provider: Option<String>, device_id: Option<u32>) -> PyResult<Self> {
        let execution_provider = match provider.as_deref() {
            Some("auto") | None => kreuzberg::ExecutionProviderType::Auto,
            Some("cpu") => kreuzberg::ExecutionProviderType::Cpu,
            Some("coreml") => kreuzberg::ExecutionProviderType::CoreMl,
            Some("cuda") => kreuzberg::ExecutionProviderType::Cuda,
            Some("tensorrt") | Some("tensor_rt") => kreuzberg::ExecutionProviderType::TensorRt,
            Some(other) => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid provider: {}. Must be 'auto', 'cpu', 'coreml', 'cuda', or 'tensorrt'",
                    other
                )));
            }
        };

        Ok(Self {
            inner: kreuzberg::AccelerationConfig {
                provider: execution_provider,
                device_id: device_id.unwrap_or(0),
            },
        })
    }

    #[getter]
    fn provider(&self) -> String {
        match self.inner.provider {
            kreuzberg::ExecutionProviderType::Auto => "auto".to_string(),
            kreuzberg::ExecutionProviderType::Cpu => "cpu".to_string(),
            kreuzberg::ExecutionProviderType::CoreMl => "coreml".to_string(),
            kreuzberg::ExecutionProviderType::Cuda => "cuda".to_string(),
            kreuzberg::ExecutionProviderType::TensorRt => "tensorrt".to_string(),
        }
    }

    #[setter]
    fn set_provider(&mut self, value: String) -> PyResult<()> {
        self.inner.provider = match value.to_lowercase().as_str() {
            "auto" => kreuzberg::ExecutionProviderType::Auto,
            "cpu" => kreuzberg::ExecutionProviderType::Cpu,
            "coreml" => kreuzberg::ExecutionProviderType::CoreMl,
            "cuda" => kreuzberg::ExecutionProviderType::Cuda,
            "tensorrt" | "tensor_rt" => kreuzberg::ExecutionProviderType::TensorRt,
            other => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid provider: {}. Must be 'auto', 'cpu', 'coreml', 'cuda', or 'tensorrt'",
                    other
                )));
            }
        };
        Ok(())
    }

    #[getter]
    fn device_id(&self) -> u32 {
        self.inner.device_id
    }

    #[setter]
    fn set_device_id(&mut self, value: u32) {
        self.inner.device_id = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "AccelerationConfig(provider='{}', device_id={})",
            self.provider(),
            self.inner.device_id
        )
    }
}

/// Email extraction configuration.
///
/// Controls behavior specific to MSG email extraction.
///
/// Attributes:
///     msg_fallback_codepage (int | None): Windows codepage number to use when an MSG file
///         contains no codepage property. Defaults to None, which falls back to windows-1252.
///         Common values: 1250 (Central European), 1251 (Cyrillic), 1252 (Western European, default),
///         1253 (Greek), 1254 (Turkish), 1255 (Hebrew), 1256 (Arabic), 932 (Japanese), 936 (Simplified Chinese).
///
/// Example:
///     >>> from kreuzberg import EmailConfig
///     >>> # Use default (windows-1252 fallback)
///     >>> config = EmailConfig()
///     >>>
///     >>> # Force Cyrillic codepage for Russian MSG files
///     >>> config = EmailConfig(msg_fallback_codepage=1251)
#[pyclass(name = "EmailConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct EmailConfig {
    pub inner: kreuzberg::EmailConfig,
}

#[pymethods]
impl EmailConfig {
    #[new]
    #[pyo3(signature = (msg_fallback_codepage=None))]
    fn new(msg_fallback_codepage: Option<u32>) -> Self {
        Self {
            inner: kreuzberg::EmailConfig { msg_fallback_codepage },
        }
    }

    #[getter]
    fn msg_fallback_codepage(&self) -> Option<u32> {
        self.inner.msg_fallback_codepage
    }

    #[setter]
    fn set_msg_fallback_codepage(&mut self, value: Option<u32>) {
        self.inner.msg_fallback_codepage = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "EmailConfig(msg_fallback_codepage={:?})",
            self.inner.msg_fallback_codepage
        )
    }
}

/// Concurrency configuration.
///
/// Controls thread usage for constrained environments.
///
/// Example:
///     >>> from kreuzberg import ConcurrencyConfig
///     >>> config = ConcurrencyConfig(max_threads=2)
#[pyclass(name = "ConcurrencyConfig", module = "kreuzberg", from_py_object)]
#[derive(Default, Clone)]
pub struct ConcurrencyConfig {
    pub inner: kreuzberg::core::config::ConcurrencyConfig,
}

#[pymethods]
impl ConcurrencyConfig {
    #[new]
    #[pyo3(signature = (max_threads=None))]
    fn new(max_threads: Option<usize>) -> Self {
        Self {
            inner: kreuzberg::core::config::ConcurrencyConfig { max_threads },
        }
    }

    #[getter]
    fn max_threads(&self) -> Option<usize> {
        self.inner.max_threads
    }

    #[setter]
    fn set_max_threads(&mut self, value: Option<usize>) {
        self.inner.max_threads = value;
    }

    fn __repr__(&self) -> String {
        format!("ConcurrencyConfig(max_threads={:?})", self.inner.max_threads)
    }
}

impl From<ConcurrencyConfig> for kreuzberg::core::config::ConcurrencyConfig {
    fn from(val: ConcurrencyConfig) -> Self {
        val.inner
    }
}

impl From<kreuzberg::core::config::ConcurrencyConfig> for ConcurrencyConfig {
    fn from(val: kreuzberg::core::config::ConcurrencyConfig) -> Self {
        Self { inner: val }
    }
}

/// Hierarchy extraction configuration.
///
/// Controls document hierarchy detection based on font size clustering.
///
/// Example:
///     >>> from kreuzberg import HierarchyConfig
///     >>> config = HierarchyConfig(enabled=True, k_clusters=6, include_bbox=True)
#[pyclass(name = "HierarchyConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct HierarchyConfig {
    pub inner: kreuzberg::core::config::HierarchyConfig,
}

#[pymethods]
impl HierarchyConfig {
    #[new]
    #[pyo3(signature = (enabled=None, k_clusters=None, include_bbox=None, ocr_coverage_threshold=None))]
    fn new(
        enabled: Option<bool>,
        k_clusters: Option<usize>,
        include_bbox: Option<bool>,
        ocr_coverage_threshold: Option<f32>,
    ) -> Self {
        Self {
            inner: kreuzberg::core::config::HierarchyConfig {
                enabled: enabled.unwrap_or(true),
                k_clusters: k_clusters.unwrap_or(6),
                include_bbox: include_bbox.unwrap_or(true),
                ocr_coverage_threshold,
            },
        }
    }

    #[getter]
    fn enabled(&self) -> bool {
        self.inner.enabled
    }

    #[setter]
    fn set_enabled(&mut self, value: bool) {
        self.inner.enabled = value;
    }

    #[getter]
    fn k_clusters(&self) -> usize {
        self.inner.k_clusters
    }

    #[setter]
    fn set_k_clusters(&mut self, value: usize) {
        self.inner.k_clusters = value;
    }

    #[getter]
    fn include_bbox(&self) -> bool {
        self.inner.include_bbox
    }

    #[setter]
    fn set_include_bbox(&mut self, value: bool) {
        self.inner.include_bbox = value;
    }

    #[getter]
    fn ocr_coverage_threshold(&self) -> Option<f32> {
        self.inner.ocr_coverage_threshold
    }

    #[setter]
    fn set_ocr_coverage_threshold(&mut self, value: Option<f32>) {
        self.inner.ocr_coverage_threshold = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "HierarchyConfig(enabled={}, k_clusters={}, include_bbox={}, ocr_coverage_threshold={:?})",
            self.inner.enabled, self.inner.k_clusters, self.inner.include_bbox, self.inner.ocr_coverage_threshold
        )
    }
}

/// Per-file extraction configuration overrides for batch processing.
///
/// All fields are optional — `None` means "use the batch-level default."
/// Used with `batch_extract_files` and `batch_extract_bytes` via the `file_configs` parameter
/// to allow heterogeneous extraction settings within a single batch.
///
/// Example:
///     >>> from kreuzberg import FileExtractionConfig
///     >>> config = FileExtractionConfig(force_ocr=True)
#[pyclass(name = "FileExtractionConfig", module = "kreuzberg", from_py_object)]
#[derive(Default)]
pub struct FileExtractionConfig {
    pub inner: kreuzberg::FileExtractionConfig,
    pub html_options_dict: Option<Py<PyDict>>,
}

impl Clone for FileExtractionConfig {
    fn clone(&self) -> Self {
        let html_options_dict = Python::attach(|py| self.html_options_dict.as_ref().map(|dict| dict.clone_ref(py)));
        Self {
            inner: self.inner.clone(),
            html_options_dict,
        }
    }
}

#[pymethods]
impl FileExtractionConfig {
    #[new]
    #[pyo3(signature = (
        enable_quality_processing=None,
        ocr=None,
        force_ocr=None,
        chunking=None,
        images=None,
        pdf_options=None,
        token_reduction=None,
        language_detection=None,
        pages=None,
        keywords=None,
        postprocessor=None,
        html_options=None,
        result_format=None,
        output_format=None,
        include_document_structure=None,
        layout=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        enable_quality_processing: Option<bool>,
        ocr: Option<OcrConfig>,
        force_ocr: Option<bool>,
        chunking: Option<ChunkingConfig>,
        images: Option<ImageExtractionConfig>,
        pdf_options: Option<PdfConfig>,
        token_reduction: Option<TokenReductionConfig>,
        language_detection: Option<LanguageDetectionConfig>,
        pages: Option<PageConfig>,
        keywords: Option<KeywordConfig>,
        postprocessor: Option<PostProcessorConfig>,
        html_options: Option<Bound<'_, PyDict>>,
        result_format: Option<String>,
        output_format: Option<String>,
        include_document_structure: Option<bool>,
        layout: Option<LayoutDetectionConfig>,
    ) -> PyResult<Self> {
        let (html_options_inner, html_options_dict) = parse_html_options_dict(html_options)?;
        Ok(Self {
            inner: kreuzberg::FileExtractionConfig {
                enable_quality_processing,
                ocr: ocr.map(Into::into),
                force_ocr,
                chunking: chunking.map(Into::into),
                images: images.map(Into::into),
                pdf_options: pdf_options.map(Into::into),
                token_reduction: token_reduction.map(Into::into),
                language_detection: language_detection.map(Into::into),
                pages: pages.map(Into::into),
                keywords: keywords.map(Into::into),
                postprocessor: postprocessor.map(Into::into),
                html_options: html_options_inner,
                result_format: if let Some(rf) = result_format {
                    Some(match rf.to_lowercase().as_str() {
                        "unified" => kreuzberg::types::OutputFormat::Unified,
                        "element_based" | "element-based" => kreuzberg::types::OutputFormat::ElementBased,
                        other => {
                            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Invalid result_format: {}. Must be 'unified' or 'element_based'",
                                other
                            )));
                        }
                    })
                } else {
                    None
                },
                output_format: if let Some(of) = output_format {
                    Some(match of.to_lowercase().as_str() {
                        "plain" | "text" => kreuzberg::core::config::formats::OutputFormat::Plain,
                        "markdown" | "md" => kreuzberg::core::config::formats::OutputFormat::Markdown,
                        "djot" => kreuzberg::core::config::formats::OutputFormat::Djot,
                        "html" => kreuzberg::core::config::formats::OutputFormat::Html,
                        "structured" | "json" => kreuzberg::core::config::formats::OutputFormat::Structured,
                        other => {
                            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Invalid output_format: {}. Must be 'plain', 'markdown', 'djot', 'html', or 'structured'",
                                other
                            )));
                        }
                    })
                } else {
                    None
                },
                include_document_structure,
                layout: layout.map(Into::into),
            },
            html_options_dict,
        })
    }

    #[getter]
    fn enable_quality_processing(&self) -> Option<bool> {
        self.inner.enable_quality_processing
    }

    #[setter]
    fn set_enable_quality_processing(&mut self, value: Option<bool>) {
        self.inner.enable_quality_processing = value;
    }

    #[getter]
    fn ocr(&self) -> Option<OcrConfig> {
        self.inner.ocr.clone().map(Into::into)
    }

    #[setter]
    fn set_ocr(&mut self, value: Option<OcrConfig>) {
        self.inner.ocr = value.map(Into::into);
    }

    #[getter]
    fn force_ocr(&self) -> Option<bool> {
        self.inner.force_ocr
    }

    #[setter]
    fn set_force_ocr(&mut self, value: Option<bool>) {
        self.inner.force_ocr = value;
    }

    #[getter]
    fn chunking(&self) -> Option<ChunkingConfig> {
        self.inner.chunking.clone().map(Into::into)
    }

    #[setter]
    fn set_chunking(&mut self, value: Option<ChunkingConfig>) {
        self.inner.chunking = value.map(Into::into);
    }

    #[getter]
    fn images(&self) -> Option<ImageExtractionConfig> {
        self.inner.images.clone().map(Into::into)
    }

    #[setter]
    fn set_images(&mut self, value: Option<ImageExtractionConfig>) {
        self.inner.images = value.map(Into::into);
    }

    #[getter]
    fn pdf_options(&self) -> Option<PdfConfig> {
        self.inner.pdf_options.clone().map(Into::into)
    }

    #[setter]
    fn set_pdf_options(&mut self, value: Option<PdfConfig>) {
        self.inner.pdf_options = value.map(Into::into);
    }

    #[getter]
    fn token_reduction(&self) -> Option<TokenReductionConfig> {
        self.inner.token_reduction.clone().map(Into::into)
    }

    #[setter]
    fn set_token_reduction(&mut self, value: Option<TokenReductionConfig>) {
        self.inner.token_reduction = value.map(Into::into);
    }

    #[getter]
    fn language_detection(&self) -> Option<LanguageDetectionConfig> {
        self.inner.language_detection.clone().map(Into::into)
    }

    #[setter]
    fn set_language_detection(&mut self, value: Option<LanguageDetectionConfig>) {
        self.inner.language_detection = value.map(Into::into);
    }

    #[getter]
    fn pages(&self) -> Option<PageConfig> {
        self.inner.pages.clone().map(Into::into)
    }

    #[setter]
    fn set_pages(&mut self, value: Option<PageConfig>) {
        self.inner.pages = value.map(Into::into);
    }

    #[getter]
    fn keywords(&self) -> Option<KeywordConfig> {
        self.inner.keywords.clone().map(Into::into)
    }

    #[setter]
    fn set_keywords(&mut self, value: Option<KeywordConfig>) {
        self.inner.keywords = value.map(Into::into);
    }

    #[getter]
    fn postprocessor(&self) -> Option<PostProcessorConfig> {
        self.inner.postprocessor.clone().map(Into::into)
    }

    #[setter]
    fn set_postprocessor(&mut self, value: Option<PostProcessorConfig>) {
        self.inner.postprocessor = value.map(Into::into);
    }

    #[getter]
    fn html_options(&self) -> Option<Py<PyDict>> {
        Python::attach(|py| self.html_options_dict.as_ref().map(|d| d.clone_ref(py)))
    }

    #[setter]
    fn set_html_options(&mut self, value: Option<Bound<'_, PyDict>>) -> PyResult<()> {
        let (html_options_inner, html_options_dict) = parse_html_options_dict(value)?;
        self.inner.html_options = html_options_inner;
        self.html_options_dict = html_options_dict;
        Ok(())
    }

    #[getter]
    fn result_format(&self) -> Option<String> {
        self.inner.result_format.as_ref().map(|rf| match rf {
            kreuzberg::types::OutputFormat::Unified => "unified".to_string(),
            kreuzberg::types::OutputFormat::ElementBased => "element_based".to_string(),
        })
    }

    #[setter]
    fn set_result_format(&mut self, value: Option<String>) -> PyResult<()> {
        self.inner.result_format = if let Some(rf) = value {
            Some(match rf.to_lowercase().as_str() {
                "unified" => kreuzberg::types::OutputFormat::Unified,
                "element_based" | "element-based" => kreuzberg::types::OutputFormat::ElementBased,
                other => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid result_format: {}. Must be 'unified' or 'element_based'",
                        other
                    )));
                }
            })
        } else {
            None
        };
        Ok(())
    }

    #[getter]
    fn output_format(&self) -> Option<String> {
        self.inner.output_format.as_ref().map(|of| match of {
            kreuzberg::core::config::formats::OutputFormat::Plain => "plain".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Markdown => "markdown".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Djot => "djot".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Html => "html".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Structured => "structured".to_string(),
        })
    }

    #[setter]
    fn set_output_format(&mut self, value: Option<String>) -> PyResult<()> {
        self.inner.output_format = if let Some(of) = value {
            Some(match of.to_lowercase().as_str() {
                "plain" | "text" => kreuzberg::core::config::formats::OutputFormat::Plain,
                "markdown" | "md" => kreuzberg::core::config::formats::OutputFormat::Markdown,
                "djot" => kreuzberg::core::config::formats::OutputFormat::Djot,
                "html" => kreuzberg::core::config::formats::OutputFormat::Html,
                "structured" | "json" => kreuzberg::core::config::formats::OutputFormat::Structured,
                other => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid output_format: {}. Must be 'plain', 'markdown', 'djot', 'html', or 'structured'",
                        other
                    )));
                }
            })
        } else {
            None
        };
        Ok(())
    }

    #[getter]
    fn include_document_structure(&self) -> Option<bool> {
        self.inner.include_document_structure
    }

    #[setter]
    fn set_include_document_structure(&mut self, value: Option<bool>) {
        self.inner.include_document_structure = value;
    }

    #[getter]
    fn layout(&self) -> Option<LayoutDetectionConfig> {
        self.inner.layout.clone().map(Into::into)
    }

    #[setter]
    fn set_layout(&mut self, value: Option<LayoutDetectionConfig>) {
        self.inner.layout = value.map(Into::into);
    }

    fn __repr__(&self) -> String {
        format!(
            "FileExtractionConfig(force_ocr={:?}, enable_quality_processing={:?}, include_document_structure={:?})",
            self.inner.force_ocr, self.inner.enable_quality_processing, self.inner.include_document_structure
        )
    }
}
