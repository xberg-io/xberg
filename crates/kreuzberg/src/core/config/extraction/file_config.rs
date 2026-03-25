//! Per-file extraction configuration overrides for batch processing.
//!
//! This module contains [`FileExtractionConfig`], a subset of [`super::ExtractionConfig`]
//! where every field is optional. When used with batch extraction functions, each file
//! can specify overrides that are merged with the batch-level default config.
//!
//! Fields that are batch-level concerns (concurrency, caching, acceleration, security)
//! are intentionally excluded and can only be set on the batch-level [`super::ExtractionConfig`].

use serde::{Deserialize, Serialize};

use super::super::formats::OutputFormat;
use super::super::ocr::OcrConfig;
use super::super::page::PageConfig;
use super::super::processing::{ChunkingConfig, PostProcessorConfig};
use super::types::{ImageExtractionConfig, LanguageDetectionConfig, TokenReductionConfig};

/// Per-file extraction configuration overrides for batch processing.
///
/// All fields are `Option<T>` — `None` means "use the batch-level default."
/// This type is used with [`crate::batch_extract_file`] and
/// [`crate::batch_extract_bytes`] to allow heterogeneous
/// extraction settings within a single batch.
///
/// # Excluded Fields
///
/// The following [`super::ExtractionConfig`] fields are batch-level only and
/// cannot be overridden per file:
/// - `max_concurrent_extractions` — controls batch parallelism
/// - `use_cache` — global caching policy
/// - `acceleration` — shared ONNX execution provider
/// - `security_limits` — global archive security policy
///
/// # Example
///
/// ```rust
/// use kreuzberg::FileExtractionConfig;
///
/// // Override just OCR forcing for a specific file
/// let config = FileExtractionConfig {
///     force_ocr: Some(true),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FileExtractionConfig {
    /// Override quality post-processing for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_quality_processing: Option<bool>,

    /// Override OCR configuration for this file (None in the Option = use batch default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr: Option<OcrConfig>,

    /// Override force OCR for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_ocr: Option<bool>,

    /// Override force OCR pages for this file (1-indexed page numbers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_ocr_pages: Option<Vec<usize>>,

    /// Override chunking configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking: Option<ChunkingConfig>,

    /// Override image extraction configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<ImageExtractionConfig>,

    /// Override PDF options for this file.
    #[cfg(feature = "pdf")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_options: Option<super::super::pdf::PdfConfig>,

    /// Override token reduction for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_reduction: Option<TokenReductionConfig>,

    /// Override language detection for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_detection: Option<LanguageDetectionConfig>,

    /// Override page extraction for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<PageConfig>,

    /// Override keyword extraction for this file.
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<crate::keywords::KeywordConfig>,

    /// Override post-processor for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postprocessor: Option<PostProcessorConfig>,

    /// Override HTML conversion options for this file.
    #[cfg(feature = "html")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_options: Option<html_to_markdown_rs::ConversionOptions>,

    /// Override result format for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_format: Option<crate::types::OutputFormat>,

    /// Override output content format for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<OutputFormat>,

    /// Override document structure output for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_document_structure: Option<bool>,

    /// Override layout detection for this file.
    #[cfg(feature = "layout-detection")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<super::super::layout::LayoutDetectionConfig>,

    /// Override per-file extraction timeout in seconds.
    ///
    /// When set, the extraction for this file will be canceled after the
    /// specified duration. A timed-out file produces an error result without
    /// affecting other files in the batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}
