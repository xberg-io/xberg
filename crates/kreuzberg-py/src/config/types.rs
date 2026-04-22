//! Configuration type bindings
//!
//! Provides Python-friendly wrappers around the Rust configuration structs.
//! All types support both construction and field access from Python.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::html_options::parse_html_options_dict;
use crate::keywords::KeywordConfig;
use crate::plugins::json_value_to_py;

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
        disable_ocr=None,
        force_ocr_pages=None,
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
        cache_ttl_secs=None,
        extraction_timeout_secs=None,
        tree_sitter=None,
        structured_extraction=None,
        content_filter=None,
        html_output=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        use_cache: Option<bool>,
        enable_quality_processing: Option<bool>,
        ocr: Option<OcrConfig>,
        force_ocr: Option<bool>,
        disable_ocr: Option<bool>,
        force_ocr_pages: Option<Vec<usize>>,
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
        extraction_timeout_secs: Option<u64>,
        tree_sitter: Option<TreeSitterConfig>,
        structured_extraction: Option<PyStructuredExtractionConfig>,
        content_filter: Option<ContentFilterConfig>,
        html_output: Option<HtmlOutputConfig>,
    ) -> PyResult<Self> {
        let (html_options_inner, html_options_dict) = parse_html_options_dict(html_options)?;
        Ok(Self {
            inner: kreuzberg::ExtractionConfig {
                use_cache: use_cache.unwrap_or(true),
                enable_quality_processing: enable_quality_processing.unwrap_or(true),
                ocr: ocr.map(Into::into),
                force_ocr: force_ocr.unwrap_or(false),
                disable_ocr: disable_ocr.unwrap_or(false),
                force_ocr_pages,
                chunking: chunking.map(Into::into),
                content_filter: content_filter.map(Into::into),
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
                        "json" => kreuzberg::core::config::formats::OutputFormat::Json,
                        "structured" => kreuzberg::core::config::formats::OutputFormat::Structured,
                        other => {
                            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Invalid output_format: {}. Must be 'plain', 'markdown', 'djot', 'html', 'json', or 'structured'",
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
                extraction_timeout_secs,
                cache_namespace,
                cache_ttl_secs,
                email: email.map(Into::into),
                concurrency: concurrency.map(Into::into),
                max_archive_depth: 3,
                tree_sitter: tree_sitter.map(Into::into),
                structured_extraction: structured_extraction.map(|s| s.inner),
                html_output: html_output.map(Into::into),
                cancel_token: None,
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
    fn disable_ocr(&self) -> bool {
        self.inner.disable_ocr
    }

    #[setter]
    fn set_disable_ocr(&mut self, value: bool) {
        self.inner.disable_ocr = value;
    }

    #[getter]
    fn force_ocr_pages(&self) -> Option<Vec<usize>> {
        self.inner.force_ocr_pages.clone()
    }

    #[setter]
    fn set_force_ocr_pages(&mut self, value: Option<Vec<usize>>) {
        self.inner.force_ocr_pages = value;
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
            kreuzberg::core::config::formats::OutputFormat::Json => "json".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Structured => "structured".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Custom(ref name) => name.clone(),
        }
    }

    #[setter]
    fn set_output_format(&mut self, value: String) {
        self.inner.output_format = match value.to_lowercase().as_str() {
            "plain" => kreuzberg::core::config::formats::OutputFormat::Plain,
            "markdown" => kreuzberg::core::config::formats::OutputFormat::Markdown,
            "djot" => kreuzberg::core::config::formats::OutputFormat::Djot,
            "html" => kreuzberg::core::config::formats::OutputFormat::Html,
            "json" => kreuzberg::core::config::formats::OutputFormat::Json,
            "structured" => kreuzberg::core::config::formats::OutputFormat::Structured,
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
    fn tree_sitter(&self) -> Option<TreeSitterConfig> {
        self.inner.tree_sitter.clone().map(Into::into)
    }

    #[setter]
    fn set_tree_sitter(&mut self, value: Option<TreeSitterConfig>) {
        self.inner.tree_sitter = value.map(Into::into);
    }

    #[getter]
    fn content_filter(&self) -> Option<ContentFilterConfig> {
        self.inner.content_filter.clone().map(Into::into)
    }

    #[setter]
    fn set_content_filter(&mut self, value: Option<ContentFilterConfig>) {
        self.inner.content_filter = value.map(Into::into);
    }

    #[getter]
    fn html_output(&self) -> Option<HtmlOutputConfig> {
        self.inner.html_output.clone().map(Into::into)
    }

    #[setter]
    fn set_html_output(&mut self, value: Option<HtmlOutputConfig>) {
        self.inner.html_output = value.map(Into::into);
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

    #[getter]
    fn extraction_timeout_secs(&self) -> Option<u64> {
        self.inner.extraction_timeout_secs
    }

    #[setter]
    fn set_extraction_timeout_secs(&mut self, value: Option<u64>) {
        self.inner.extraction_timeout_secs = value;
    }

    #[getter]
    fn max_archive_depth(&self) -> usize {
        self.inner.max_archive_depth
    }

    #[setter]
    fn set_max_archive_depth(&mut self, value: usize) {
        self.inner.max_archive_depth = value;
    }

    /// Get the security limits for archive extraction.
    ///
    /// Returns:
    ///     Optional dict with keys: max_archive_size, max_compression_ratio,
    ///     max_files_in_archive, max_nesting_depth, max_entity_length,
    ///     max_content_size, max_iterations
    #[getter]
    fn security_limits(&self, py: Python<'_>) -> PyResult<Option<Py<PyDict>>> {
        match &self.inner.security_limits {
            Some(limits) => {
                let dict = PyDict::new(py);
                dict.set_item("max_archive_size", limits.max_archive_size)?;
                dict.set_item("max_compression_ratio", limits.max_compression_ratio)?;
                dict.set_item("max_files_in_archive", limits.max_files_in_archive)?;
                dict.set_item("max_nesting_depth", limits.max_nesting_depth)?;
                dict.set_item("max_entity_length", limits.max_entity_length)?;
                dict.set_item("max_content_size", limits.max_content_size)?;
                dict.set_item("max_iterations", limits.max_iterations)?;
                Ok(Some(dict.unbind()))
            }
            None => Ok(None),
        }
    }

    /// Set the security limits for archive extraction.
    ///
    /// Args:
    ///     value: Optional dict with security limit keys, or None to use defaults.
    #[setter]
    fn set_security_limits(&mut self, value: Option<Bound<'_, PyDict>>) -> PyResult<()> {
        self.inner.security_limits = match value {
            Some(dict) => {
                let mut limits = kreuzberg::extractors::security::SecurityLimits::default();
                if let Some(v) = dict.get_item("max_archive_size")? {
                    limits.max_archive_size = v.extract()?;
                }
                if let Some(v) = dict.get_item("max_compression_ratio")? {
                    limits.max_compression_ratio = v.extract()?;
                }
                if let Some(v) = dict.get_item("max_files_in_archive")? {
                    limits.max_files_in_archive = v.extract()?;
                }
                if let Some(v) = dict.get_item("max_nesting_depth")? {
                    limits.max_nesting_depth = v.extract()?;
                }
                if let Some(v) = dict.get_item("max_entity_length")? {
                    limits.max_entity_length = v.extract()?;
                }
                if let Some(v) = dict.get_item("max_content_size")? {
                    limits.max_content_size = v.extract()?;
                }
                if let Some(v) = dict.get_item("max_iterations")? {
                    limits.max_iterations = v.extract()?;
                }
                Some(limits)
            }
            None => None,
        };
        Ok(())
    }

    #[getter]
    fn structured_extraction(&self) -> Option<PyStructuredExtractionConfig> {
        self.inner
            .structured_extraction
            .clone()
            .map(|s| PyStructuredExtractionConfig { inner: s })
    }

    #[setter]
    fn set_structured_extraction(&mut self, value: Option<PyStructuredExtractionConfig>) {
        self.inner.structured_extraction = value.map(|s| s.inner);
    }

    /// Get the cancellation token attached to this config, if any.
    #[getter]
    fn cancel_token(&self) -> Option<crate::cancellation::PyCancellationToken> {
        self.inner
            .cancel_token
            .clone()
            .map(|t| crate::cancellation::PyCancellationToken { inner: t })
    }

    /// Attach a `CancellationToken` to signal the extraction to stop early.
    ///
    /// Set to `None` to remove any previously attached token.
    #[setter]
    fn set_cancel_token(&mut self, value: Option<crate::cancellation::PyCancellationToken>) {
        self.inner.cancel_token = value.map(|t| t.inner);
    }

    fn __repr__(&self) -> String {
        format!(
            "ExtractionConfig(use_cache={}, enable_quality_processing={}, ocr={}, force_ocr={}, extraction_timeout_secs={:?}, force_ocr_pages={:?})",
            self.inner.use_cache,
            self.inner.enable_quality_processing,
            if self.inner.ocr.is_some() { "Some(...)" } else { "None" },
            self.inner.force_ocr,
            self.inner.extraction_timeout_secs,
            self.inner.force_ocr_pages
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
    #[pyo3(signature = (backend=None, language=None, tesseract_config=None, paddle_ocr_config=None, element_config=None, vlm_config=None, vlm_prompt=None, enabled=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        backend: Option<String>,
        language: Option<String>,
        tesseract_config: Option<TesseractConfig>,
        paddle_ocr_config: Option<Bound<'_, pyo3::types::PyAny>>,
        element_config: Option<Bound<'_, pyo3::types::PyAny>>,
        vlm_config: Option<PyLlmConfig>,
        vlm_prompt: Option<String>,
        enabled: Option<bool>,
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
                enabled: enabled.unwrap_or(true),
                backend: backend.unwrap_or_else(|| "tesseract".to_string()),
                language: language.unwrap_or_else(|| "eng".to_string()),
                tesseract_config: tesseract_config.map(Into::into),
                output_format: None,
                paddle_ocr_config: paddle_ocr_json,
                element_config: element_cfg,
                quality_thresholds: None,
                pipeline: None,
                auto_rotate: false,
                vlm_config: vlm_config.map(|c| c.inner),
                vlm_prompt,
                acceleration: None,
            },
        })
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

    #[getter]
    fn vlm_config(&self) -> Option<PyLlmConfig> {
        self.inner.vlm_config.clone().map(|c| PyLlmConfig { inner: c })
    }

    #[setter]
    fn set_vlm_config(&mut self, value: Option<PyLlmConfig>) {
        self.inner.vlm_config = value.map(|c| c.inner);
    }

    #[getter]
    fn vlm_prompt(&self) -> Option<String> {
        self.inner.vlm_prompt.clone()
    }

    #[setter]
    fn set_vlm_prompt(&mut self, value: Option<String>) {
        self.inner.vlm_prompt = value;
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

    /// Create an LLM provider-hosted embedding model type.
    #[staticmethod]
    fn llm(config: PyLlmConfig) -> Self {
        Self {
            inner: kreuzberg::EmbeddingModelType::Llm { llm: config.inner },
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            kreuzberg::EmbeddingModelType::Preset { name } => format!("EmbeddingModelType.preset('{}')", name),
            kreuzberg::EmbeddingModelType::Custom { model_id, dimensions } => {
                format!("EmbeddingModelType.custom('{}', {})", model_id, dimensions)
            }
            kreuzberg::EmbeddingModelType::Llm { llm } => {
                format!("EmbeddingModelType.llm('{}')", llm.model)
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
    #[pyo3(signature = (model=None, normalize=None, batch_size=None, show_download_progress=None, cache_dir=None, acceleration=None))]
    fn new(
        model: Option<EmbeddingModelType>,
        normalize: Option<bool>,
        batch_size: Option<usize>,
        show_download_progress: Option<bool>,
        cache_dir: Option<String>,
        acceleration: Option<AccelerationConfig>,
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
                acceleration: acceleration.map(Into::into),
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

    #[getter]
    fn acceleration(&self) -> Option<AccelerationConfig> {
        self.inner.acceleration.clone().map(Into::into)
    }

    #[setter]
    fn set_acceleration(&mut self, value: Option<AccelerationConfig>) {
        self.inner.acceleration = value.map(Into::into);
    }

    fn __repr__(&self) -> String {
        format!(
            "EmbeddingConfig(normalize={}, batch_size={})",
            self.inner.normalize, self.inner.batch_size
        )
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            inner: kreuzberg::EmbeddingConfig {
                model: kreuzberg::EmbeddingModelType::Preset {
                    name: "balanced".to_string(),
                },
                normalize: true,
                batch_size: 32,
                show_download_progress: false,
                cache_dir: None,
                acceleration: None,
            },
        }
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
    #[pyo3(signature = (max_chars=None, max_overlap=None, embedding=None, preset=None, chunker_type=None, sizing_type=None, sizing_model=None, sizing_cache_dir=None, prepend_heading_context=None, topic_threshold=None))]
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
        prepend_heading_context: Option<bool>,
        topic_threshold: Option<f32>,
    ) -> Self {
        let ct = match chunker_type.as_deref() {
            Some("markdown") => kreuzberg::ChunkerType::Markdown,
            Some("yaml") => kreuzberg::ChunkerType::Yaml,
            Some("semantic") => kreuzberg::ChunkerType::Semantic,
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
                prepend_heading_context: prepend_heading_context.unwrap_or(false),
                topic_threshold,
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

    #[getter]
    fn prepend_heading_context(&self) -> bool {
        self.inner.prepend_heading_context
    }

    #[getter]
    fn chunker_type(&self) -> String {
        match self.inner.chunker_type {
            kreuzberg::ChunkerType::Text => "text".to_string(),
            kreuzberg::ChunkerType::Markdown => "markdown".to_string(),
            kreuzberg::ChunkerType::Yaml => "yaml".to_string(),
            kreuzberg::ChunkerType::Semantic => "semantic".to_string(),
        }
    }

    #[getter]
    fn topic_threshold(&self) -> Option<f32> {
        self.inner.topic_threshold
    }

    #[setter]
    fn set_topic_threshold(&mut self, value: Option<f32>) {
        self.inner.topic_threshold = value;
    }

    #[getter]
    fn sizing_cache_dir(&self) -> Option<String> {
        match &self.inner.sizing {
            kreuzberg::ChunkSizing::Tokenizer { cache_dir, .. } => {
                cache_dir.as_ref().map(|p| p.to_string_lossy().to_string())
            }
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ChunkingConfig(max_chars={}, max_overlap={}, chunker_type='{}', topic_threshold={}, embedding={}, preset={}, prepend_heading_context={})",
            self.inner.max_characters,
            self.inner.overlap,
            self.chunker_type(),
            self.inner
                .topic_threshold
                .map(|t| format!("{t}"))
                .unwrap_or_else(|| "None".to_string()),
            if self.inner.embedding.is_some() { "..." } else { "None" },
            self.inner
                .preset
                .as_ref()
                .map(|s| format!("'{}'", s))
                .unwrap_or_else(|| "None".to_string()),
            self.inner.prepend_heading_context
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
#[allow(clippy::too_many_arguments)]
impl ImageExtractionConfig {
    #[new]
    #[pyo3(signature = (
        extract_images=None,
        target_dpi=None,
        max_image_dimension=None,
        inject_placeholders=None,
        auto_adjust_dpi=None,
        min_dpi=None,
        max_dpi=None,
        max_images_per_page=None
    ))]
    fn new(
        extract_images: Option<bool>,
        target_dpi: Option<i32>,
        max_image_dimension: Option<i32>,
        inject_placeholders: Option<bool>,
        auto_adjust_dpi: Option<bool>,
        min_dpi: Option<i32>,
        max_dpi: Option<i32>,
        max_images_per_page: Option<u32>,
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
                max_images_per_page,
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

    #[getter]
    fn max_images_per_page(&self) -> Option<u32> {
        self.inner.max_images_per_page
    }

    #[setter]
    fn set_max_images_per_page(&mut self, value: Option<u32>) {
        self.inner.max_images_per_page = value;
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
                backend: kreuzberg::PdfBackend::default(),
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
/// document layout models.
///
/// Example:
///     >>> from kreuzberg import LayoutDetectionConfig
///     >>> config = LayoutDetectionConfig(apply_heuristics=True, table_model="tatr")
#[pyclass(name = "LayoutDetectionConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct LayoutDetectionConfig {
    pub inner: kreuzberg::core::config::layout::LayoutDetectionConfig,
}

/// Parse a table model string into a TableModel enum.
fn parse_table_model(s: &str) -> kreuzberg::core::config::layout::TableModel {
    match s {
        "tatr" => kreuzberg::core::config::layout::TableModel::Tatr,
        "slanet_wired" => kreuzberg::core::config::layout::TableModel::SlanetWired,
        "slanet_wireless" => kreuzberg::core::config::layout::TableModel::SlanetWireless,
        "slanet_plus" => kreuzberg::core::config::layout::TableModel::SlanetPlus,
        "slanet_auto" => kreuzberg::core::config::layout::TableModel::SlanetAuto,
        "disabled" => kreuzberg::core::config::layout::TableModel::Disabled,
        _ => kreuzberg::core::config::layout::TableModel::default(),
    }
}

#[pymethods]
impl LayoutDetectionConfig {
    #[new]
    #[pyo3(signature = (confidence_threshold=None, apply_heuristics=None, table_model=None, acceleration=None))]
    fn new(
        confidence_threshold: Option<f32>,
        apply_heuristics: Option<bool>,
        table_model: Option<String>,
        acceleration: Option<AccelerationConfig>,
    ) -> Self {
        Self {
            inner: kreuzberg::core::config::layout::LayoutDetectionConfig {
                confidence_threshold,
                apply_heuristics: apply_heuristics.unwrap_or(true),
                table_model: table_model.as_deref().map(parse_table_model).unwrap_or_default(),
                acceleration: acceleration.map(Into::into),
            },
        }
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

    #[getter]
    fn table_model(&self) -> String {
        self.inner.table_model.to_string()
    }

    #[setter]
    fn set_table_model(&mut self, value: String) {
        self.inner.table_model = parse_table_model(&value);
    }

    #[getter]
    fn acceleration(&self) -> Option<AccelerationConfig> {
        self.inner.acceleration.clone().map(Into::into)
    }

    #[setter]
    fn set_acceleration(&mut self, value: Option<AccelerationConfig>) {
        self.inner.acceleration = value.map(Into::into);
    }

    fn __repr__(&self) -> String {
        format!(
            "LayoutDetectionConfig(confidence_threshold={}, apply_heuristics={}, table_model='{}')",
            self.inner
                .confidence_threshold
                .map(|v| v.to_string())
                .unwrap_or_else(|| "None".to_string()),
            self.inner.apply_heuristics,
            self.inner.table_model,
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
/// Attributes:
///     extract_pages (bool): Enable page tracking and per-page extraction. Default: False
///     insert_page_markers (bool): Insert page markers into content. Default: False
///     marker_format (str): Marker template containing {page_num}. Default: "\\n\\n<!-- PAGE {page_num} -->\\n\\n"
///
/// Example:
///     >>> from kreuzberg import PageConfig
///     >>> # Default configuration (no page extraction)
///     >>> config = PageConfig()
///     >>> # Enable page extraction and markers
///     >>> config = PageConfig(extract_pages=True, insert_page_markers=True)
///
/// Note:
///     Set `extract_pages=True` when using `result_format="element_based"` to get per-page content extraction.
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

/// Content filtering configuration.
///
/// Controls whether "furniture" content (headers, footers, page numbers,
/// watermarks, repeating text) is included in or stripped from extraction results.
///
/// Example:
///     >>> from kreuzberg import ContentFilterConfig
///     >>> # Include headers and footers
///     >>> config = ContentFilterConfig(include_headers=True, include_footers=True)
///     >>>
///     >>> # Disable repeating-text stripping (default is True)
///     >>> config = ContentFilterConfig(strip_repeating_text=False)
#[pyclass(name = "ContentFilterConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct ContentFilterConfig {
    pub inner: kreuzberg::ContentFilterConfig,
}

#[pymethods]
impl ContentFilterConfig {
    #[new]
    #[pyo3(signature = (
        include_headers=None,
        include_footers=None,
        strip_repeating_text=None,
        include_watermarks=None
    ))]
    fn new(
        include_headers: Option<bool>,
        include_footers: Option<bool>,
        strip_repeating_text: Option<bool>,
        include_watermarks: Option<bool>,
    ) -> Self {
        Self {
            inner: kreuzberg::ContentFilterConfig {
                include_headers: include_headers.unwrap_or(false),
                include_footers: include_footers.unwrap_or(false),
                strip_repeating_text: strip_repeating_text.unwrap_or(true),
                include_watermarks: include_watermarks.unwrap_or(false),
            },
        }
    }

    #[getter]
    fn include_headers(&self) -> bool {
        self.inner.include_headers
    }

    #[setter]
    fn set_include_headers(&mut self, value: bool) {
        self.inner.include_headers = value;
    }

    #[getter]
    fn include_footers(&self) -> bool {
        self.inner.include_footers
    }

    #[setter]
    fn set_include_footers(&mut self, value: bool) {
        self.inner.include_footers = value;
    }

    #[getter]
    fn strip_repeating_text(&self) -> bool {
        self.inner.strip_repeating_text
    }

    #[setter]
    fn set_strip_repeating_text(&mut self, value: bool) {
        self.inner.strip_repeating_text = value;
    }

    #[getter]
    fn include_watermarks(&self) -> bool {
        self.inner.include_watermarks
    }

    #[setter]
    fn set_include_watermarks(&mut self, value: bool) {
        self.inner.include_watermarks = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "ContentFilterConfig(include_headers={}, include_footers={}, strip_repeating_text={}, include_watermarks={})",
            self.inner.include_headers,
            self.inner.include_footers,
            self.inner.strip_repeating_text,
            self.inner.include_watermarks
        )
    }
}

impl From<ContentFilterConfig> for kreuzberg::ContentFilterConfig {
    fn from(val: ContentFilterConfig) -> Self {
        val.inner
    }
}

impl From<kreuzberg::ContentFilterConfig> for ContentFilterConfig {
    fn from(val: kreuzberg::ContentFilterConfig) -> Self {
        Self { inner: val }
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

/// Processing options for tree-sitter code analysis.
///
/// Controls which analysis features are enabled when extracting code files.
///
/// Example:
///     >>> from kreuzberg import TreeSitterProcessConfig
///     >>> config = TreeSitterProcessConfig(structure=True, comments=True, docstrings=True)
#[pyclass(name = "TreeSitterProcessConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct TreeSitterProcessConfig {
    pub inner: kreuzberg::core::config::TreeSitterProcessConfig,
}

#[pymethods]
impl TreeSitterProcessConfig {
    #[new]
    #[pyo3(signature = (
        structure=None,
        imports=None,
        exports=None,
        comments=None,
        docstrings=None,
        symbols=None,
        diagnostics=None,
        chunk_max_size=None,
        content_mode=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        structure: Option<bool>,
        imports: Option<bool>,
        exports: Option<bool>,
        comments: Option<bool>,
        docstrings: Option<bool>,
        symbols: Option<bool>,
        diagnostics: Option<bool>,
        chunk_max_size: Option<usize>,
        content_mode: Option<String>,
    ) -> Self {
        let content_mode = content_mode
            .and_then(|s| match s.as_str() {
                "chunks" => Some(kreuzberg::core::config::CodeContentMode::Chunks),
                "raw" => Some(kreuzberg::core::config::CodeContentMode::Raw),
                "structure" => Some(kreuzberg::core::config::CodeContentMode::Structure),
                _ => None,
            })
            .unwrap_or_default();
        Self {
            inner: kreuzberg::core::config::TreeSitterProcessConfig {
                structure: structure.unwrap_or(true),
                imports: imports.unwrap_or(true),
                exports: exports.unwrap_or(true),
                comments: comments.unwrap_or(false),
                docstrings: docstrings.unwrap_or(false),
                symbols: symbols.unwrap_or(false),
                diagnostics: diagnostics.unwrap_or(false),
                chunk_max_size,
                content_mode,
            },
        }
    }

    #[getter]
    fn structure(&self) -> bool {
        self.inner.structure
    }

    #[setter]
    fn set_structure(&mut self, value: bool) {
        self.inner.structure = value;
    }

    #[getter]
    fn imports(&self) -> bool {
        self.inner.imports
    }

    #[setter]
    fn set_imports(&mut self, value: bool) {
        self.inner.imports = value;
    }

    #[getter]
    fn exports(&self) -> bool {
        self.inner.exports
    }

    #[setter]
    fn set_exports(&mut self, value: bool) {
        self.inner.exports = value;
    }

    #[getter]
    fn comments(&self) -> bool {
        self.inner.comments
    }

    #[setter]
    fn set_comments(&mut self, value: bool) {
        self.inner.comments = value;
    }

    #[getter]
    fn docstrings(&self) -> bool {
        self.inner.docstrings
    }

    #[setter]
    fn set_docstrings(&mut self, value: bool) {
        self.inner.docstrings = value;
    }

    #[getter]
    fn symbols(&self) -> bool {
        self.inner.symbols
    }

    #[setter]
    fn set_symbols(&mut self, value: bool) {
        self.inner.symbols = value;
    }

    #[getter]
    fn diagnostics(&self) -> bool {
        self.inner.diagnostics
    }

    #[setter]
    fn set_diagnostics(&mut self, value: bool) {
        self.inner.diagnostics = value;
    }

    #[getter]
    fn chunk_max_size(&self) -> Option<usize> {
        self.inner.chunk_max_size
    }

    #[setter]
    fn set_chunk_max_size(&mut self, value: Option<usize>) {
        self.inner.chunk_max_size = value;
    }

    #[getter]
    fn content_mode(&self) -> String {
        match self.inner.content_mode {
            kreuzberg::core::config::CodeContentMode::Chunks => "chunks".to_string(),
            kreuzberg::core::config::CodeContentMode::Raw => "raw".to_string(),
            kreuzberg::core::config::CodeContentMode::Structure => "structure".to_string(),
        }
    }

    #[setter]
    fn set_content_mode(&mut self, value: String) {
        self.inner.content_mode = match value.as_str() {
            "raw" => kreuzberg::core::config::CodeContentMode::Raw,
            "structure" => kreuzberg::core::config::CodeContentMode::Structure,
            _ => kreuzberg::core::config::CodeContentMode::Chunks,
        };
    }

    fn __repr__(&self) -> String {
        format!(
            "TreeSitterProcessConfig(structure={}, imports={}, exports={}, comments={}, docstrings={}, symbols={}, diagnostics={}, chunk_max_size={:?}, content_mode={:?})",
            self.inner.structure,
            self.inner.imports,
            self.inner.exports,
            self.inner.comments,
            self.inner.docstrings,
            self.inner.symbols,
            self.inner.diagnostics,
            self.inner.chunk_max_size,
            self.inner.content_mode
        )
    }
}

/// Configuration for tree-sitter language pack integration.
///
/// Controls grammar download behavior and code analysis options.
///
/// Example:
///     >>> from kreuzberg import TreeSitterConfig, TreeSitterProcessConfig
///     >>> config = TreeSitterConfig(
///     ...     languages=["python", "rust"],
///     ...     groups=["web"],
///     ...     process=TreeSitterProcessConfig(comments=True)
///     ... )
#[pyclass(name = "TreeSitterConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct TreeSitterConfig {
    pub inner: kreuzberg::core::config::TreeSitterConfig,
}

#[pymethods]
impl TreeSitterConfig {
    #[new]
    #[pyo3(signature = (cache_dir=None, languages=None, groups=None, process=None, enabled=None))]
    fn new(
        cache_dir: Option<String>,
        languages: Option<Vec<String>>,
        groups: Option<Vec<String>>,
        process: Option<TreeSitterProcessConfig>,
        enabled: Option<bool>,
    ) -> Self {
        Self {
            inner: kreuzberg::core::config::TreeSitterConfig {
                enabled: enabled.unwrap_or(true),
                cache_dir: cache_dir.map(std::path::PathBuf::from),
                languages,
                groups,
                process: process.map(Into::into).unwrap_or_default(),
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
    fn cache_dir(&self) -> Option<String> {
        self.inner.cache_dir.as_ref().map(|p| p.display().to_string())
    }

    #[setter]
    fn set_cache_dir(&mut self, value: Option<String>) {
        self.inner.cache_dir = value.map(std::path::PathBuf::from);
    }

    #[getter]
    fn languages(&self) -> Option<Vec<String>> {
        self.inner.languages.clone()
    }

    #[setter]
    fn set_languages(&mut self, value: Option<Vec<String>>) {
        self.inner.languages = value;
    }

    #[getter]
    fn groups(&self) -> Option<Vec<String>> {
        self.inner.groups.clone()
    }

    #[setter]
    fn set_groups(&mut self, value: Option<Vec<String>>) {
        self.inner.groups = value;
    }

    #[getter]
    fn process(&self) -> TreeSitterProcessConfig {
        TreeSitterProcessConfig {
            inner: self.inner.process.clone(),
        }
    }

    #[setter]
    fn set_process(&mut self, value: TreeSitterProcessConfig) {
        self.inner.process = value.inner;
    }

    fn __repr__(&self) -> String {
        format!(
            "TreeSitterConfig(cache_dir={:?}, languages={:?}, groups={:?}, process=...)",
            self.inner.cache_dir, self.inner.languages, self.inner.groups
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
        disable_ocr=None,
        force_ocr_pages=None,
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
        layout=None,
        timeout_secs=None,
        tree_sitter=None,
        content_filter=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        enable_quality_processing: Option<bool>,
        ocr: Option<OcrConfig>,
        force_ocr: Option<bool>,
        disable_ocr: Option<bool>,
        force_ocr_pages: Option<Vec<usize>>,
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
        timeout_secs: Option<u64>,
        tree_sitter: Option<TreeSitterConfig>,
        content_filter: Option<ContentFilterConfig>,
    ) -> PyResult<Self> {
        let (html_options_inner, html_options_dict) = parse_html_options_dict(html_options)?;
        Ok(Self {
            inner: kreuzberg::FileExtractionConfig {
                enable_quality_processing,
                ocr: ocr.map(Into::into),
                force_ocr,
                disable_ocr,
                force_ocr_pages,
                chunking: chunking.map(Into::into),
                content_filter: content_filter.map(Into::into),
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
                        "json" => kreuzberg::core::config::formats::OutputFormat::Json,
                        "structured" => kreuzberg::core::config::formats::OutputFormat::Structured,
                        other => {
                            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Invalid output_format: {}. Must be 'plain', 'markdown', 'djot', 'html', 'json', or 'structured'",
                                other
                            )));
                        }
                    })
                } else {
                    None
                },
                include_document_structure,
                layout: layout.map(Into::into),
                timeout_secs,
                tree_sitter: tree_sitter.map(Into::into),
                structured_extraction: None,
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
    fn disable_ocr(&self) -> Option<bool> {
        self.inner.disable_ocr
    }

    #[setter]
    fn set_disable_ocr(&mut self, value: Option<bool>) {
        self.inner.disable_ocr = value;
    }

    #[getter]
    fn force_ocr_pages(&self) -> Option<Vec<usize>> {
        self.inner.force_ocr_pages.clone()
    }

    #[setter]
    fn set_force_ocr_pages(&mut self, value: Option<Vec<usize>>) {
        self.inner.force_ocr_pages = value;
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
            kreuzberg::core::config::formats::OutputFormat::Json => "json".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Structured => "structured".to_string(),
            kreuzberg::core::config::formats::OutputFormat::Custom(name) => name.clone(),
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
                "json" => kreuzberg::core::config::formats::OutputFormat::Json,
                "structured" => kreuzberg::core::config::formats::OutputFormat::Structured,
                other => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid output_format: {}. Must be 'plain', 'markdown', 'djot', 'html', 'json', or 'structured'",
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

    #[getter]
    fn timeout_secs(&self) -> Option<u64> {
        self.inner.timeout_secs
    }

    #[setter]
    fn set_timeout_secs(&mut self, value: Option<u64>) {
        self.inner.timeout_secs = value;
    }

    #[getter]
    fn tree_sitter(&self) -> Option<TreeSitterConfig> {
        self.inner.tree_sitter.clone().map(Into::into)
    }

    #[setter]
    fn set_tree_sitter(&mut self, value: Option<TreeSitterConfig>) {
        self.inner.tree_sitter = value.map(Into::into);
    }

    #[getter]
    fn content_filter(&self) -> Option<ContentFilterConfig> {
        self.inner.content_filter.clone().map(Into::into)
    }

    #[setter]
    fn set_content_filter(&mut self, value: Option<ContentFilterConfig>) {
        self.inner.content_filter = value.map(Into::into);
    }

    fn __repr__(&self) -> String {
        format!(
            "FileExtractionConfig(force_ocr={:?}, enable_quality_processing={:?}, include_document_structure={:?}, timeout_secs={:?}, force_ocr_pages={:?})",
            self.inner.force_ocr,
            self.inner.enable_quality_processing,
            self.inner.include_document_structure,
            self.inner.timeout_secs,
            self.inner.force_ocr_pages
        )
    }
}

/// LLM provider/model configuration for liter-llm integration.
///
/// Each feature (VLM OCR, structured extraction) carries its own LlmConfig,
/// allowing different providers per feature.
///
/// Attributes:
///     model (str): Provider/model string using liter-llm routing format
///         (e.g., "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514")
///     api_key (str | None): API key for the provider (falls back to env var if None)
///     base_url (str | None): Custom base URL override for the provider endpoint
///     timeout_secs (int | None): Request timeout in seconds (default: 60)
///     max_retries (int | None): Maximum retry attempts (default: 3)
///     temperature (float | None): Sampling temperature for generation tasks
///     max_tokens (int | None): Maximum tokens to generate
///
/// Example:
///     >>> from kreuzberg import LlmConfig
///     >>> config = LlmConfig(model="openai/gpt-4o", temperature=0.0)
#[pyclass(name = "LlmConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct PyLlmConfig {
    pub inner: kreuzberg::LlmConfig,
}

#[pymethods]
impl PyLlmConfig {
    #[new]
    #[pyo3(signature = (model, api_key=None, base_url=None, timeout_secs=None, max_retries=None, temperature=None, max_tokens=None))]
    fn new(
        model: String,
        api_key: Option<String>,
        base_url: Option<String>,
        timeout_secs: Option<u64>,
        max_retries: Option<u32>,
        temperature: Option<f64>,
        max_tokens: Option<u64>,
    ) -> Self {
        Self {
            inner: kreuzberg::LlmConfig {
                model,
                api_key,
                base_url,
                timeout_secs,
                max_retries,
                temperature,
                max_tokens,
            },
        }
    }

    #[getter]
    fn model(&self) -> String {
        self.inner.model.clone()
    }

    #[setter]
    fn set_model(&mut self, value: String) {
        self.inner.model = value;
    }

    #[getter]
    fn api_key(&self) -> Option<String> {
        self.inner.api_key.clone()
    }

    #[setter]
    fn set_api_key(&mut self, value: Option<String>) {
        self.inner.api_key = value;
    }

    #[getter]
    fn base_url(&self) -> Option<String> {
        self.inner.base_url.clone()
    }

    #[setter]
    fn set_base_url(&mut self, value: Option<String>) {
        self.inner.base_url = value;
    }

    #[getter]
    fn timeout_secs(&self) -> Option<u64> {
        self.inner.timeout_secs
    }

    #[setter]
    fn set_timeout_secs(&mut self, value: Option<u64>) {
        self.inner.timeout_secs = value;
    }

    #[getter]
    fn max_retries(&self) -> Option<u32> {
        self.inner.max_retries
    }

    #[setter]
    fn set_max_retries(&mut self, value: Option<u32>) {
        self.inner.max_retries = value;
    }

    #[getter]
    fn temperature(&self) -> Option<f64> {
        self.inner.temperature
    }

    #[setter]
    fn set_temperature(&mut self, value: Option<f64>) {
        self.inner.temperature = value;
    }

    #[getter]
    fn max_tokens(&self) -> Option<u64> {
        self.inner.max_tokens
    }

    #[setter]
    fn set_max_tokens(&mut self, value: Option<u64>) {
        self.inner.max_tokens = value;
    }

    fn __repr__(&self) -> String {
        format!("LlmConfig(model={:?})", self.inner.model)
    }
}

/// Configuration for LLM-based structured data extraction.
///
/// Sends extracted document content to a VLM with a JSON schema,
/// returning structured data that conforms to the schema.
///
/// Attributes:
///     schema (dict): JSON Schema defining the desired output structure
///     llm (LlmConfig): LLM configuration for the extraction
///     schema_name (str): Schema name passed to the LLM's structured output mode (default: "extraction")
///     schema_description (str | None): Optional schema description for the LLM
///     strict (bool): Enable strict mode -- output must exactly match the schema (default: False)
///     prompt (str | None): Custom extraction prompt template (Jinja2 format)
///
/// Example:
///     >>> from kreuzberg import StructuredExtractionConfig, LlmConfig
///     >>> config = StructuredExtractionConfig(
///     ...     schema={"type": "object", "properties": {"vendor": {"type": "string"}}},
///     ...     llm=LlmConfig(model="openai/gpt-4o"),
///     ...     schema_name="invoice_data",
///     ...     strict=True,
///     ... )
#[pyclass(name = "StructuredExtractionConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct PyStructuredExtractionConfig {
    pub inner: kreuzberg::StructuredExtractionConfig,
}

#[pymethods]
impl PyStructuredExtractionConfig {
    #[new]
    #[pyo3(signature = (schema, llm, schema_name=None, schema_description=None, strict=None, prompt=None))]
    fn new(
        py: Python<'_>,
        schema: Bound<'_, pyo3::types::PyAny>,
        llm: PyLlmConfig,
        schema_name: Option<String>,
        schema_description: Option<String>,
        strict: Option<bool>,
        prompt: Option<String>,
    ) -> PyResult<Self> {
        let json_mod = py.import("json")?;
        let json_str: String = json_mod.call_method1("dumps", (&schema,))?.extract()?;
        let schema_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid schema: {e}")))?;

        Ok(Self {
            inner: kreuzberg::StructuredExtractionConfig {
                schema: schema_value,
                schema_name: schema_name.unwrap_or_else(|| "extraction".to_string()),
                schema_description,
                strict: strict.unwrap_or(false),
                prompt,
                llm: llm.inner,
            },
        })
    }

    #[getter]
    fn schema<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyAny>> {
        json_value_to_py(py, &self.inner.schema)
    }

    #[setter]
    fn set_schema(&mut self, py: Python<'_>, value: Bound<'_, pyo3::types::PyAny>) -> PyResult<()> {
        let json_mod = py.import("json")?;
        let json_str: String = json_mod.call_method1("dumps", (&value,))?.extract()?;
        self.inner.schema = serde_json::from_str(&json_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid schema: {e}")))?;
        Ok(())
    }

    #[getter]
    fn llm(&self) -> PyLlmConfig {
        PyLlmConfig {
            inner: self.inner.llm.clone(),
        }
    }

    #[setter]
    fn set_llm(&mut self, value: PyLlmConfig) {
        self.inner.llm = value.inner;
    }

    #[getter]
    fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }

    #[setter]
    fn set_schema_name(&mut self, value: String) {
        self.inner.schema_name = value;
    }

    #[getter]
    fn schema_description(&self) -> Option<String> {
        self.inner.schema_description.clone()
    }

    #[setter]
    fn set_schema_description(&mut self, value: Option<String>) {
        self.inner.schema_description = value;
    }

    #[getter]
    fn strict(&self) -> bool {
        self.inner.strict
    }

    #[setter]
    fn set_strict(&mut self, value: bool) {
        self.inner.strict = value;
    }

    #[getter]
    fn prompt(&self) -> Option<String> {
        self.inner.prompt.clone()
    }

    #[setter]
    fn set_prompt(&mut self, value: Option<String>) {
        self.inner.prompt = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "StructuredExtractionConfig(schema_name={:?}, strict={}, llm=LlmConfig(model={:?}))",
            self.inner.schema_name, self.inner.strict, self.inner.llm.model
        )
    }
}

/// HTML output configuration for styled HTML rendering.
///
/// Controls how the HTML output format renders documents, including
/// theme selection, custom CSS, and class prefix configuration.
///
/// Example:
///     >>> from kreuzberg import HtmlOutputConfig
///     >>> config = HtmlOutputConfig(theme="github", embed_css=True)
#[pyclass(name = "HtmlOutputConfig", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct HtmlOutputConfig {
    pub inner: kreuzberg::HtmlOutputConfig,
}

#[pymethods]
impl HtmlOutputConfig {
    #[new]
    #[pyo3(signature = (
        theme=None,
        css=None,
        css_file=None,
        class_prefix=None,
        embed_css=None
    ))]
    fn new(
        theme: Option<String>,
        css: Option<String>,
        css_file: Option<String>,
        class_prefix: Option<String>,
        embed_css: Option<bool>,
    ) -> PyResult<Self> {
        let theme = match theme.as_deref().unwrap_or("unstyled") {
            "default" => kreuzberg::HtmlTheme::Default,
            "github" => kreuzberg::HtmlTheme::GitHub,
            "dark" => kreuzberg::HtmlTheme::Dark,
            "light" => kreuzberg::HtmlTheme::Light,
            "unstyled" => kreuzberg::HtmlTheme::Unstyled,
            other => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid theme: '{}'. Must be 'default', 'github', 'dark', 'light', or 'unstyled'",
                    other
                )));
            }
        };

        Ok(Self {
            inner: kreuzberg::HtmlOutputConfig {
                css,
                css_file: css_file.map(std::path::PathBuf::from),
                theme,
                class_prefix: class_prefix.unwrap_or_else(|| "kb-".to_string()),
                embed_css: embed_css.unwrap_or(true),
            },
        })
    }

    #[getter]
    fn theme(&self) -> String {
        match self.inner.theme {
            kreuzberg::HtmlTheme::Default => "default".to_string(),
            kreuzberg::HtmlTheme::GitHub => "github".to_string(),
            kreuzberg::HtmlTheme::Dark => "dark".to_string(),
            kreuzberg::HtmlTheme::Light => "light".to_string(),
            kreuzberg::HtmlTheme::Unstyled => "unstyled".to_string(),
        }
    }

    #[setter]
    fn set_theme(&mut self, value: String) -> PyResult<()> {
        self.inner.theme = match value.as_str() {
            "default" => kreuzberg::HtmlTheme::Default,
            "github" => kreuzberg::HtmlTheme::GitHub,
            "dark" => kreuzberg::HtmlTheme::Dark,
            "light" => kreuzberg::HtmlTheme::Light,
            "unstyled" => kreuzberg::HtmlTheme::Unstyled,
            other => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid theme: '{}'. Must be 'default', 'github', 'dark', 'light', or 'unstyled'",
                    other
                )));
            }
        };
        Ok(())
    }

    #[getter]
    fn css(&self) -> Option<String> {
        self.inner.css.clone()
    }

    #[setter]
    fn set_css(&mut self, value: Option<String>) {
        self.inner.css = value;
    }

    #[getter]
    fn css_file(&self) -> Option<String> {
        self.inner.css_file.as_ref().map(|p| p.to_string_lossy().into_owned())
    }

    #[setter]
    fn set_css_file(&mut self, value: Option<String>) {
        self.inner.css_file = value.map(std::path::PathBuf::from);
    }

    #[getter]
    fn class_prefix(&self) -> String {
        self.inner.class_prefix.clone()
    }

    #[setter]
    fn set_class_prefix(&mut self, value: String) {
        self.inner.class_prefix = value;
    }

    #[getter]
    fn embed_css(&self) -> bool {
        self.inner.embed_css
    }

    #[setter]
    fn set_embed_css(&mut self, value: bool) {
        self.inner.embed_css = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "HtmlOutputConfig(theme='{}', class_prefix='{}', embed_css={})",
            self.theme(),
            self.inner.class_prefix,
            self.inner.embed_css
        )
    }
}

impl From<HtmlOutputConfig> for kreuzberg::HtmlOutputConfig {
    fn from(val: HtmlOutputConfig) -> Self {
        val.inner
    }
}

impl From<kreuzberg::HtmlOutputConfig> for HtmlOutputConfig {
    fn from(val: kreuzberg::HtmlOutputConfig) -> Self {
        Self { inner: val }
    }
}
