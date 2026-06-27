//! Main extraction configuration struct.
//!
//! This module contains the main `ExtractionConfig` struct that aggregates all
//! configuration options for the extraction process.

use serde::{Deserialize, Serialize};

use super::super::acceleration::AccelerationConfig;
use super::super::content_filter::ContentFilterConfig;
use super::super::formats::OutputFormat;
use super::super::ocr::OcrConfig;
use super::super::page::PageConfig;
use super::super::processing::{ChunkingConfig, PostProcessorConfig};
use super::file_config::FileExtractionConfig;
use super::types::{ImageExtractionConfig, LanguageDetectionConfig, TokenReductionOptions, UrlExtractionConfig};

/// Main extraction configuration.
///
/// This struct contains all configuration options for the extraction process.
/// It can be loaded from TOML, YAML, or JSON files, or created programmatically.
///
/// # Example
///
/// ```rust
/// use xberg::core::config::ExtractionConfig;
///
/// // Create with defaults
/// let config = ExtractionConfig::default();
///
/// // Load from TOML file
/// // let config = ExtractionConfig::from_toml_file("xberg.toml")?;
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
    pub force_ocr_pages: Option<Vec<u32>>,

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

    /// Content filtering configuration (None = use extractor defaults).
    ///
    /// Controls whether document "furniture" (headers, footers, watermarks,
    /// repeating text) is included in or stripped from extraction results.
    /// See [`ContentFilterConfig`] for per-field documentation.
    #[serde(default)]
    pub content_filter: Option<ContentFilterConfig>,

    /// Image extraction configuration (None = no image extraction)
    #[serde(default)]
    pub images: Option<ImageExtractionConfig>,

    /// PDF-specific options (None = use defaults)
    #[cfg(feature = "pdf")]
    #[serde(default)]
    pub pdf_options: Option<super::super::pdf::PdfConfig>,

    /// Token reduction configuration (None = no token reduction)
    #[serde(default)]
    pub token_reduction: Option<TokenReductionOptions>,

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
    #[cfg_attr(alef, alef(skip))]
    pub html_options: Option<html_to_markdown_rs::ConversionOptions>,

    /// Styled HTML output configuration.
    ///
    /// When set alongside `output_format = OutputFormat::Html`, the extraction
    /// pipeline uses [`StyledHtmlRenderer`](crate::rendering::StyledHtmlRenderer)
    /// which emits stable `kb-*` CSS class hooks on every structural element
    /// and optionally embeds theme CSS or user-supplied CSS in a `<style>` block.
    ///
    /// When `None`, the existing plain comrak-based HTML renderer is used.
    #[cfg(feature = "html")]
    #[serde(default)]
    pub html_output: Option<crate::core::config::html_output::HtmlOutputConfig>,

    /// Default per-file timeout in seconds for batch extraction.
    ///
    /// When set, each file in a batch will be canceled after this duration
    /// unless overridden by [`FileExtractionConfig::timeout_secs`].
    ///
    /// Defaults to `Some(60)` to prevent pathological files (e.g. deeply
    /// nested archives, documents with millions of cells) from running
    /// indefinitely and exhausting caller resources. Set to `None` to
    /// disable the timeout for trusted input or long-running workloads.
    #[serde(default = "default_extraction_timeout")]
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
    pub result_format: crate::types::ResultFormat,

    /// Security limits for archive extraction.
    ///
    /// Controls maximum archive size, compression ratio, file count, and other
    /// security thresholds to prevent decompression bomb attacks. Also caps
    /// nesting depth, iteration count, entity / token length, total
    /// content size, and table cell count for every extraction path that
    /// ingests user-controlled bytes.
    /// When `None`, default limits are used.
    #[serde(default)]
    pub security_limits: Option<crate::extractors::security::SecurityLimits>,

    /// Maximum uncompressed size in bytes for a single embedded file before
    /// recursive extraction is attempted (default: 50 MiB).
    ///
    /// Applies to embedded objects inside OOXML containers (DOCX, PPTX) and
    /// to email attachments processed via recursive extraction. Files that
    /// exceed this limit are skipped with a `ProcessingWarning` rather than
    /// passed to the extraction pipeline, preventing a single oversized
    /// embedded object from consuming unbounded memory or time.
    ///
    /// Set to `None` to disable the per-embedded-file cap (falls back to
    /// `security_limits.max_archive_size` as the only guard).
    #[serde(default = "default_max_embedded_file_bytes")]
    pub max_embedded_file_bytes: Option<u64>,

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
    /// Requires the `layout-detection` feature to run inference; the field is
    /// present whenever the `layout-types` feature is active (which includes
    /// `layout-detection` as well as the no-ORT target groups).
    #[cfg(feature = "layout-types")]
    #[serde(default)]
    pub layout: Option<super::super::layout::LayoutDetectionConfig>,

    /// Transcription (speech-to-text) configuration for audio/video files.
    ///
    /// When set and `enabled`, files with audio/video MIME types (mp3, mp4,
    /// m4a, wav, webm, etc.) are routed to the Whisper-based transcription
    /// pipeline. The actual heavy dependencies are only active under the
    /// `transcription` feature; the field is visible under `transcription-types`
    /// (including on WASM and Android targets that use the no-ORT preset).
    ///
    /// Default: `None` (transcription disabled). This is an additive,
    /// non-breaking change.
    #[cfg(feature = "transcription-types")]
    #[serde(default)]
    pub transcription: Option<super::super::transcription::TranscriptionConfig>,

    /// Run layout detection on the non-OCR PDF markdown path.
    ///
    /// When `true` and `layout` is `Some(_)`, layout regions inform heading,
    /// table, list, and figure detection in the structure pipeline that would
    /// otherwise rely on font-clustering heuristics alone. Significantly
    /// improves SF1 (structural F1) at the cost of inference latency
    /// (~150-300ms/page CPU, ~20-50ms/page GPU). Default: `false`.
    /// Requires the `layout-detection` feature.
    #[serde(default)]
    pub use_layout_for_markdown: bool,

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
    #[cfg_attr(alef, alef(skip))]
    pub concurrency: Option<super::super::concurrency::ConcurrencyConfig>,

    /// URL ingestion and crawl configuration.
    #[serde(default)]
    pub url: UrlExtractionConfig,

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

    /// Structured extraction via LLM (None = disabled).
    ///
    /// When set, the extracted document content is sent to an LLM with the
    /// provided JSON schema. The structured response is stored in
    /// `ExtractionResult::structured_output`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_extraction: Option<super::super::llm::StructuredExtractionConfig>,

    /// Named-entity recognition configuration. When set, the NER post-processor runs at
    /// the Middle stage and populates `ExtractionResult::entities`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.0.0"))]
    pub ner: Option<super::super::ner::NerConfig>,

    /// Redaction / anonymisation configuration. When set, the redaction post-processor
    /// runs at the Late stage and rewrites every textual field in `ExtractionResult`,
    /// emitting an audit trail in `ExtractionResult::redaction_report`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.0.0"))]
    pub redaction: Option<super::super::redaction::RedactionConfig>,

    /// Summarisation configuration. When set, the summarisation post-processor runs at
    /// the Middle stage and populates `ExtractionResult::summary`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.0.0"))]
    pub summarization: Option<super::super::summarization::SummarizationConfig>,

    /// Translation configuration. When set, the translation post-processor runs at the
    /// Middle stage and populates `ExtractionResult::translation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.0.0"))]
    pub translation: Option<super::super::translation::TranslationConfig>,

    /// Per-page classification configuration. When set, the classification post-processor
    /// runs at the Middle stage and populates `ExtractionResult::page_classifications`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.0.0"))]
    pub page_classification: Option<super::super::classification::PageClassificationConfig>,

    /// VLM captioning configuration for extracted images. When set, the captioning
    /// post-processor runs at the Middle stage and writes a caption into each
    /// `ExtractedImage::caption`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.0.0"))]
    pub captioning: Option<super::super::captioning::CaptioningConfig>,

    /// Enable QR-code detection in extracted images. When `true`, the QR post-processor
    /// runs at the Middle stage and populates `ExtractedImage::qr_codes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.0.0"))]
    pub qr_codes: Option<bool>,

    /// Cancellation token for this extraction (None = no external cancellation).
    ///
    /// Pass a [`crate::cancellation::CancellationToken`] clone here and call its `cancel()`
    /// from another thread / task to abort the extraction in progress. The extractor
    /// checks the token at safe checkpoints (before lock acquisition, between pages,
    /// between batch items) and returns [`crate::error::XbergError::Cancelled`] when set.
    ///
    /// The field is excluded from serialization because `CancellationToken` is a
    /// runtime handle, not a configuration value.
    #[serde(skip)]
    #[cfg_attr(alef, alef(skip))]
    pub cancel_token: Option<crate::cancellation::CancellationToken>,
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
            content_filter: None,
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
            #[cfg(feature = "html")]
            html_output: None,
            extraction_timeout_secs: default_extraction_timeout(),
            max_concurrent_extractions: None,
            security_limits: None,
            max_embedded_file_bytes: default_max_embedded_file_bytes(),
            #[cfg(feature = "layout-types")]
            layout: None,
            #[cfg(feature = "transcription-types")]
            transcription: None,
            use_layout_for_markdown: false,
            result_format: crate::types::ResultFormat::Unified,
            output_format: OutputFormat::Plain,
            include_document_structure: false,
            acceleration: None,
            cache_namespace: None,
            cache_ttl_secs: None,
            email: None,
            concurrency: None,
            url: UrlExtractionConfig::default(),
            max_archive_depth: default_archive_depth(),
            #[cfg(feature = "tree-sitter")]
            tree_sitter: None,
            structured_extraction: None,
            ner: None,
            redaction: None,
            summarization: None,
            translation: None,
            page_classification: None,
            captioning: None,
            qr_codes: None,
            cancel_token: None,
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
    /// ```ignore
    /// use xberg::{ExtractionConfig, FileExtractionConfig};
    ///
    /// let base = ExtractionConfig::default();
    /// let override_config = FileExtractionConfig {
    ///     force_ocr: Some(true),
    ///     ..Default::default()
    /// };
    /// let resolved = base.with_file_overrides(&override_config);
    /// assert!(resolved.force_ocr);
    /// ```
    pub(crate) fn with_file_overrides(&self, overrides: &FileExtractionConfig) -> Self {
        // Destructure to ensure compile-time exhaustiveness: adding a field to
        // FileExtractionConfig without handling it here will produce a compile error.
        let FileExtractionConfig {
            ref enable_quality_processing,
            ref ocr,
            ref force_ocr,
            ref force_ocr_pages,
            ref disable_ocr,
            ref chunking,
            ref content_filter,
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
            #[cfg(feature = "layout-types")]
            ref layout,
            #[cfg(feature = "transcription-types")]
            ref transcription,
            ref timeout_secs,
            #[cfg(feature = "tree-sitter")]
            ref tree_sitter,
            ref structured_extraction,
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
        if let Some(v) = content_filter {
            config.content_filter = Some(v.clone());
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
        #[cfg(feature = "layout-types")]
        if let Some(v) = layout {
            config.layout = Some(v.clone());
        }
        #[cfg(feature = "transcription-types")]
        if let Some(v) = transcription {
            config.transcription = Some(v.clone());
        }
        if let Some(v) = timeout_secs {
            config.extraction_timeout_secs = Some(*v);
        }
        #[cfg(feature = "tree-sitter")]
        if let Some(v) = tree_sitter {
            config.tree_sitter = Some(v.clone());
        }
        if let Some(v) = structured_extraction {
            config.structured_extraction = Some(v.clone());
        }

        config
    }

    /// Normalize configuration for implicit requirements.
    ///
    /// Currently handles:
    /// - Auto-enabling `extract_pages` when `result_format` is `ElementBased`, because
    ///   the element transformation requires per-page data to assign correct page numbers.
    ///   Without this, all elements would incorrectly get `page_number=1`.
    /// - Auto-enabling `extract_pages` when chunking is configured, because the chunker
    ///   needs page boundaries to assign correct page numbers to chunks.
    pub(crate) fn normalized(&self) -> std::borrow::Cow<'_, Self> {
        let needs_pages = |cfg: &Self| -> bool {
            match &cfg.pages {
                Some(page_config) => !page_config.extract_pages,
                None => true,
            }
        };

        let needs_pages_for_elements =
            self.result_format == crate::types::ResultFormat::ElementBased && needs_pages(self);
        let needs_pages_for_chunking = self.chunking.is_some() && needs_pages(self);

        if needs_pages_for_elements || needs_pages_for_chunking {
            let mut config = self.clone();
            let page_config = config.pages.get_or_insert_with(super::super::page::PageConfig::default);
            page_config.extract_pages = true;
            return std::borrow::Cow::Owned(config);
        }
        std::borrow::Cow::Borrowed(self)
    }

    /// Validate the configuration, returning an error if any settings are invalid.
    ///
    /// Checks:
    /// Returns the effective disable-OCR value, accounting for both the top-level
    /// `disable_ocr` flag and the `ocr.enabled` shorthand on [`OcrConfig`].
    ///
    /// Setting `ocr.enabled = false` in configuration is treated as equivalent to
    /// `disable_ocr = true`. This method is the single source of truth for whether
    /// OCR should be skipped.
    pub(crate) fn effective_disable_ocr(&self) -> bool {
        self.disable_ocr || self.ocr.as_ref().is_some_and(|o| !o.enabled)
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
    /// Returns `true` when image binary data should be extracted.
    ///
    /// True when `config.images.extract_images` is set **or** when captioning is
    /// configured — captioning requires image bytes regardless of whether the caller
    /// also requested `images` extraction.
    pub fn needs_image_data(&self) -> bool {
        self.images.as_ref().is_some_and(|i| i.extract_images) || self.captioning.is_some()
    }

    /// Returns `true` when any image processing is needed during extraction.
    ///
    /// # Optimization Impact
    ///
    /// For text-only extractions (no OCR, no image extraction, no captioning), skipping
    /// image decompression can improve CPU utilization by 5-10% by avoiding wasteful
    /// image I/O and processing when results won't be used.
    pub fn needs_image_processing(&self) -> bool {
        let ocr_enabled = !self.effective_disable_ocr() && (self.ocr.is_some() || self.force_ocr);

        #[cfg(feature = "layout-detection")]
        let layout_enabled = self.layout.is_some();
        #[cfg(not(feature = "layout-detection"))]
        let layout_enabled = false;

        ocr_enabled || self.needs_image_data() || layout_enabled
    }
}

fn default_true() -> bool {
    true
}

fn default_archive_depth() -> usize {
    3
}

/// Default per-embedded-file cap: 50 MiB.
///
/// A single embedded object larger than this can consume significant memory
/// when the recursive extractor materialises it. 50 MiB is generous for
/// real-world embedded documents while still bounding worst-case allocation.
fn default_max_embedded_file_bytes() -> Option<u64> {
    Some(50 * 1024 * 1024)
}

/// Default extraction timeout: 60 seconds.
///
/// Pathological files (deeply nested archives, sheets with millions of cells,
/// adversarial PDFs) can otherwise run indefinitely and exhaust caller
/// resources. 60 s is generous for legitimate documents while bounding the
/// worst-case cost of a single untrusted input.
fn default_extraction_timeout() -> Option<u64> {
    Some(60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::OcrConfig;

    #[test]
    fn test_effective_disable_ocr_from_top_level_flag() {
        let config = ExtractionConfig {
            disable_ocr: true,
            ..Default::default()
        };
        assert!(config.effective_disable_ocr());
    }

    #[test]
    fn test_effective_disable_ocr_from_ocr_enabled_false() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(
            config.effective_disable_ocr(),
            "ocr.enabled = false should be treated as disable_ocr = true"
        );
    }

    #[test]
    fn test_effective_disable_ocr_default_is_false() {
        let config = ExtractionConfig::default();
        assert!(!config.effective_disable_ocr());
    }

    #[test]
    fn test_effective_disable_ocr_ocr_enabled_true_does_not_disable() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(!config.effective_disable_ocr());
    }

    #[test]
    fn test_ocr_enabled_false_deserialized_from_json() {
        let json = r#"{"ocr": {"enabled": false}}"#;
        let config: ExtractionConfig = serde_json::from_str(json).unwrap();
        assert!(
            config.effective_disable_ocr(),
            "JSON ocr.enabled=false should disable OCR"
        );
    }

    #[test]
    fn test_ocr_enabled_defaults_to_true() {
        let json = r#"{"ocr": {"backend": "tesseract"}}"#;
        let config: ExtractionConfig = serde_json::from_str(json).unwrap();
        assert!(!config.effective_disable_ocr(), "OCR should be enabled by default");
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_use_layout_for_markdown_defaults_to_false() {
        let config = ExtractionConfig::default();
        assert!(!config.use_layout_for_markdown);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_use_layout_for_markdown_can_be_set_true() {
        let config = ExtractionConfig {
            use_layout_for_markdown: true,
            ..Default::default()
        };
        assert!(config.use_layout_for_markdown);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_use_layout_for_markdown_serde_round_trip() {
        let config = ExtractionConfig {
            use_layout_for_markdown: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ExtractionConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.use_layout_for_markdown);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_use_layout_for_markdown_serde_default_false() {
        // Field absent in JSON → should default to false.
        let json = r#"{}"#;
        let config: ExtractionConfig = serde_json::from_str(json).unwrap();
        assert!(!config.use_layout_for_markdown);
    }

    // --- extraction_timeout_secs defaults ----------------------------------

    #[test]
    fn test_default_extraction_timeout_is_sixty_seconds() {
        let config = ExtractionConfig::default();
        assert_eq!(
            config.extraction_timeout_secs,
            Some(60),
            "default timeout must be Some(60) to prevent unbounded extraction"
        );
    }

    #[test]
    fn test_extraction_timeout_can_be_disabled_by_setting_none() {
        let config = ExtractionConfig {
            extraction_timeout_secs: None,
            ..Default::default()
        };
        assert_eq!(config.extraction_timeout_secs, None);
    }

    #[test]
    fn test_extraction_timeout_serde_round_trip() {
        let config = ExtractionConfig {
            extraction_timeout_secs: Some(120),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ExtractionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.extraction_timeout_secs, Some(120));
    }

    #[test]
    fn test_extraction_timeout_serde_absent_field_defaults_to_sixty() {
        // When the JSON field is absent the serde default function must fire.
        let json = r#"{}"#;
        let config: ExtractionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.extraction_timeout_secs,
            Some(60),
            "absent field must use default_extraction_timeout() -> Some(60)"
        );
    }
}
