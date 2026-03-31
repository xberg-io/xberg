//! Main extraction configuration struct.
//!
//! This module contains the main `ExtractionConfig` struct that aggregates all
//! configuration options for the extraction process.

use serde::{Deserialize, Serialize};

use super::super::acceleration::AccelerationConfig;
use super::super::formats::OutputFormat;
use super::super::ocr::OcrConfig;
use super::super::page::PageConfig;
use super::super::processing::{ChunkingConfig, PostProcessorConfig};
use super::file_config::FileExtractionConfig;
use super::types::{ImageExtractionConfig, LanguageDetectionConfig, TokenReductionConfig};

/// Main extraction configuration.
///
/// This struct contains all configuration options for the extraction process.
/// It can be loaded from TOML, YAML, or JSON files, or created programmatically.
///
/// # Example
///
/// ```rust
/// use kreuzberg::core::config::ExtractionConfig;
///
/// // Create with defaults
/// let config = ExtractionConfig::default();
///
/// // Load from TOML file
/// // let config = ExtractionConfig::from_toml_file("kreuzberg.toml")?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionConfig {
    /// Enable caching of extraction results
    #[serde(default = "default_true")]
    pub use_cache: bool,

    /// Enable quality post-processing
    #[serde(default = "default_true")]
    pub enable_quality_processing: bool,

    /// OCR configuration (None = OCR disabled)
    #[serde(default)]
    pub ocr: Option<OcrConfig>,

    /// Force OCR even for searchable PDFs
    #[serde(default)]
    pub force_ocr: bool,

    /// Force OCR on specific pages only (1-indexed page numbers, must be >= 1).
    ///
    /// When set, only the listed pages are OCR'd regardless of text layer quality.
    /// Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`.
    /// Only applies to PDF documents. Duplicates are automatically deduplicated.
    /// An `ocr` config is recommended for backend/language selection; defaults are used if absent.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_ocr_pages: Option<Vec<usize>>,

    /// Disable OCR entirely, even for images.
    ///
    /// When `true`, OCR is skipped for all document types. Images return metadata
    /// only (dimensions, format, EXIF) without text extraction. PDFs use only
    /// native text extraction without OCR fallback.
    ///
    /// Cannot be `true` simultaneously with `force_ocr`.
    ///
    /// *Added in v4.7.0.*
    #[serde(default)]
    pub disable_ocr: bool,

    /// Text chunking configuration (None = chunking disabled)
    #[serde(default)]
    pub chunking: Option<ChunkingConfig>,

    /// Image extraction configuration (None = no image extraction)
    #[serde(default)]
    pub images: Option<ImageExtractionConfig>,

    /// PDF-specific options (None = use defaults)
    #[cfg(feature = "pdf")]
    #[serde(default)]
    pub pdf_options: Option<super::super::pdf::PdfConfig>,

    /// Token reduction configuration (None = no token reduction)
    #[serde(default)]
    pub token_reduction: Option<TokenReductionConfig>,

    /// Language detection configuration (None = no language detection)
    #[serde(default)]
    pub language_detection: Option<LanguageDetectionConfig>,

    /// Page extraction configuration (None = no page tracking)
    #[serde(default)]
    pub pages: Option<PageConfig>,

    /// Keyword extraction configuration (None = no keyword extraction)
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    #[serde(default)]
    pub keywords: Option<crate::keywords::KeywordConfig>,

    /// Post-processor configuration (None = use defaults)
    #[serde(default)]
    pub postprocessor: Option<PostProcessorConfig>,

    /// HTML to Markdown conversion options (None = use defaults)
    ///
    /// Configure how HTML documents are converted to Markdown, including heading styles,
    /// list formatting, code block styles, and preprocessing options.
    #[cfg(feature = "html")]
    #[serde(default)]
    pub html_options: Option<html_to_markdown_rs::ConversionOptions>,

    /// Default per-file timeout in seconds for batch extraction.
    ///
    /// When set, each file in a batch will be canceled after this duration
    /// unless overridden by [`FileExtractionConfig::timeout_secs`].
    /// `None` means no timeout (unbounded extraction time).
    #[serde(default)]
    pub extraction_timeout_secs: Option<u64>,

    /// Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()).
    ///
    /// Limits parallelism to prevent resource exhaustion when processing
    /// large batches. Defaults to (num_cpus × 1.5).ceil() when not set.
    #[serde(default)]
    pub max_concurrent_extractions: Option<usize>,

    /// Result structure format
    ///
    /// Controls whether results are returned in unified format (default) with all
    /// content in the `content` field, or element-based format with semantic
    /// elements (for Unstructured-compatible output).
    #[serde(default)]
    pub result_format: crate::types::OutputFormat,

    /// Security limits for archive extraction.
    ///
    /// Controls maximum archive size, compression ratio, file count, and other
    /// security thresholds to prevent decompression bomb attacks.
    /// When `None`, default limits are used (500MB archive, 100:1 ratio, 10K files).
    #[cfg(feature = "archives")]
    #[serde(default)]
    pub security_limits: Option<crate::extractors::security::SecurityLimits>,

    /// Content text format (default: Plain).
    ///
    /// Controls the format of the extracted content:
    /// - `Plain`: Raw extracted text (default)
    /// - `Markdown`: Markdown formatted output
    /// - `Djot`: Djot markup format (requires djot feature)
    /// - `Html`: HTML formatted output
    ///
    /// When set to a structured format, extraction results will include
    /// formatted output. The `formatted_content` field may be populated
    /// when format conversion is applied.
    #[serde(default)]
    pub output_format: OutputFormat,

    /// Layout detection configuration (None = layout detection disabled).
    ///
    /// When set, PDF pages and images are analyzed for document structure
    /// (headings, code, formulas, tables, figures, etc.) using RT-DETR models
    /// via ONNX Runtime. For PDFs, layout hints override paragraph classification
    /// in the markdown pipeline. For images, per-region OCR is performed with
    /// markdown formatting based on detected layout classes.
    /// Requires the `layout-detection` feature.
    #[cfg(feature = "layout-detection")]
    #[serde(default)]
    pub layout: Option<super::super::layout::LayoutDetectionConfig>,

    /// Enable structured document tree output.
    ///
    /// When true, populates the `document` field on `ExtractionResult` with a
    /// hierarchical `DocumentStructure` containing heading-driven section nesting,
    /// table grids, content layer classification, and inline annotations.
    ///
    /// Independent of `result_format` — can be combined with Unified or ElementBased.
    #[serde(default)]
    pub include_document_structure: bool,

    /// Hardware acceleration configuration for ONNX Runtime models.
    ///
    /// Controls execution provider selection for layout detection and embedding
    /// models. When `None`, uses platform defaults (CoreML on macOS, CUDA on
    /// Linux, CPU on Windows).
    #[serde(default)]
    pub acceleration: Option<AccelerationConfig>,

    /// Cache namespace for tenant isolation.
    ///
    /// When set, cache entries are stored under `{cache_dir}/{namespace}/`.
    /// Must be alphanumeric, hyphens, or underscores only (max 64 chars).
    /// Different namespaces have isolated cache spaces on the same filesystem.
    #[serde(default)]
    pub cache_namespace: Option<String>,

    /// Per-request cache TTL in seconds.
    ///
    /// Overrides the global `max_age_days` for this specific extraction.
    /// When `0`, caching is completely skipped (no read or write).
    /// When `None`, the global TTL applies.
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,

    /// Email extraction configuration (None = use defaults).
    ///
    /// Currently supports configuring the fallback codepage for MSG files
    /// that do not specify one. See [`crate::core::config::EmailConfig`] for details.
    #[serde(default)]
    pub email: Option<super::super::email::EmailConfig>,

    /// Concurrency limits for constrained environments (None = use defaults).
    ///
    /// Controls Rayon thread pool size, ONNX Runtime intra-op threads, and
    /// (when `max_concurrent_extractions` is unset) the batch concurrency
    /// semaphore. See [`crate::core::config::ConcurrencyConfig`] for details.
    #[serde(default)]
    pub concurrency: Option<super::super::concurrency::ConcurrencyConfig>,

    /// Maximum recursion depth for archive extraction (default: 3).
    /// Set to 0 to disable recursive extraction (legacy behavior).
    #[serde(default = "default_archive_depth")]
    pub max_archive_depth: usize,

    /// Tree-sitter language pack configuration (None = tree-sitter disabled).
    ///
    /// When set, enables code file extraction using tree-sitter parsers.
    /// Controls grammar download behavior and code analysis options.
    #[cfg(feature = "tree-sitter")]
    #[serde(default)]
    pub tree_sitter: Option<super::super::tree_sitter::TreeSitterConfig>,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            use_cache: true,
            enable_quality_processing: true,
            ocr: None,
            force_ocr: false,
            force_ocr_pages: None,
            disable_ocr: false,
            chunking: None,
            images: None,
            #[cfg(feature = "pdf")]
            pdf_options: None,
            token_reduction: None,
            language_detection: None,
            pages: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            keywords: None,
            postprocessor: None,
            #[cfg(feature = "html")]
            html_options: None,
            extraction_timeout_secs: None,
            max_concurrent_extractions: None,
            #[cfg(feature = "archives")]
            security_limits: None,
            #[cfg(feature = "layout-detection")]
            layout: None,
            result_format: crate::types::OutputFormat::Unified,
            output_format: OutputFormat::Plain,
            include_document_structure: false,
            acceleration: None,
            cache_namespace: None,
            cache_ttl_secs: None,
            email: None,
            concurrency: None,
            max_archive_depth: default_archive_depth(),
            #[cfg(feature = "tree-sitter")]
            tree_sitter: None,
        }
    }
}

impl ExtractionConfig {
    /// Create a new `ExtractionConfig` by applying per-file overrides from a
    /// [`FileExtractionConfig`]. Fields that are `Some` in the override replace the
    /// corresponding field in `self`; `None` fields keep the original value.
    ///
    /// Batch-level fields (`max_concurrent_extractions`, `use_cache`, `acceleration`,
    /// `security_limits`) are never affected by overrides.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::{ExtractionConfig, FileExtractionConfig};
    ///
    /// let base = ExtractionConfig::default();
    /// let override_config = FileExtractionConfig {
    ///     force_ocr: Some(true),
    ///     ..Default::default()
    /// };
    /// let resolved = base.with_file_overrides(&override_config);
    /// assert!(resolved.force_ocr);
    /// ```
    pub fn with_file_overrides(&self, overrides: &FileExtractionConfig) -> Self {
        // Destructure to ensure compile-time exhaustiveness: adding a field to
        // FileExtractionConfig without handling it here will produce a compile error.
        let FileExtractionConfig {
            ref enable_quality_processing,
            ref ocr,
            ref force_ocr,
            ref force_ocr_pages,
            ref disable_ocr,
            ref chunking,
            ref images,
            #[cfg(feature = "pdf")]
            ref pdf_options,
            ref token_reduction,
            ref language_detection,
            ref pages,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            ref keywords,
            ref postprocessor,
            #[cfg(feature = "html")]
            ref html_options,
            ref result_format,
            ref output_format,
            ref include_document_structure,
            #[cfg(feature = "layout-detection")]
            ref layout,
            ref timeout_secs,
            #[cfg(feature = "tree-sitter")]
            ref tree_sitter,
        } = *overrides;

        let mut config = self.clone();

        if let Some(v) = enable_quality_processing {
            config.enable_quality_processing = *v;
        }
        if let Some(v) = ocr {
            config.ocr = Some(v.clone());
        }
        if let Some(v) = force_ocr {
            config.force_ocr = *v;
        }
        if let Some(v) = force_ocr_pages {
            config.force_ocr_pages = Some(v.clone());
        }
        if let Some(v) = disable_ocr {
            config.disable_ocr = *v;
        }
        if let Some(v) = chunking {
            config.chunking = Some(v.clone());
        }
        if let Some(v) = images {
            config.images = Some(v.clone());
        }
        #[cfg(feature = "pdf")]
        if let Some(v) = pdf_options {
            config.pdf_options = Some(v.clone());
        }
        if let Some(v) = token_reduction {
            config.token_reduction = Some(v.clone());
        }
        if let Some(v) = language_detection {
            config.language_detection = Some(v.clone());
        }
        if let Some(v) = pages {
            config.pages = Some(v.clone());
        }
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        if let Some(v) = keywords {
            config.keywords = Some(v.clone());
        }
        if let Some(v) = postprocessor {
            config.postprocessor = Some(v.clone());
        }
        #[cfg(feature = "html")]
        if let Some(v) = html_options {
            config.html_options = Some(v.clone());
        }
        if let Some(v) = result_format {
            config.result_format = *v;
        }
        if let Some(v) = output_format {
            config.output_format = v.clone();
        }
        if let Some(v) = include_document_structure {
            config.include_document_structure = *v;
        }
        #[cfg(feature = "layout-detection")]
        if let Some(v) = layout {
            config.layout = Some(v.clone());
        }
        if let Some(v) = timeout_secs {
            config.extraction_timeout_secs = Some(*v);
        }
        #[cfg(feature = "tree-sitter")]
        if let Some(v) = tree_sitter {
            config.tree_sitter = Some(v.clone());
        }

        config
    }

    /// Normalize configuration for implicit requirements.
    ///
    /// Currently handles:
    /// - Auto-enabling `extract_pages` when `result_format` is `ElementBased`, because
    ///   the element transformation requires per-page data to assign correct page numbers.
    ///   Without this, all elements would incorrectly get `page_number=1`.
    pub fn normalized(&self) -> std::borrow::Cow<'_, Self> {
        if self.result_format == crate::types::OutputFormat::ElementBased {
            let needs_pages = match &self.pages {
                Some(page_config) => !page_config.extract_pages,
                None => true,
            };
            if needs_pages {
                let mut config = self.clone();
                let page_config = config.pages.get_or_insert_with(super::super::page::PageConfig::default);
                page_config.extract_pages = true;
                return std::borrow::Cow::Owned(config);
            }
        }
        std::borrow::Cow::Borrowed(self)
    }

    /// Check if image processing is needed by examining OCR and image extraction settings.
    ///
    /// Returns `true` if either OCR is enabled or image extraction is configured,
    /// indicating that image decompression and processing should occur.
    /// Returns `false` if both are disabled, allowing optimization to skip unnecessary
    /// image decompression for text-only extraction workflows.
    ///
    /// # Optimization Impact
    /// For text-only extractions (no OCR, no image extraction), skipping image
    /// decompression can improve CPU utilization by 5-10% by avoiding wasteful
    /// image I/O and processing when results won't be used.
    pub fn needs_image_processing(&self) -> bool {
        let ocr_enabled = !self.disable_ocr && (self.ocr.is_some() || self.force_ocr);

        let image_extraction_enabled = self.images.as_ref().map(|i| i.extract_images).unwrap_or(false);

        #[cfg(feature = "layout-detection")]
        let layout_enabled = self.layout.is_some();
        #[cfg(not(feature = "layout-detection"))]
        let layout_enabled = false;

        ocr_enabled || image_extraction_enabled || layout_enabled
    }
}

fn default_true() -> bool {
    true
}

fn default_archive_depth() -> usize {
    3
}
