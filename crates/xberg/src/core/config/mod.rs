//! Configuration loading and management.
//!
//! This module provides utilities for loading extraction configuration from various
//! sources (TOML, YAML, JSON) and discovering configuration files in the project hierarchy.

pub mod acceleration;
pub mod captioning;
pub mod classification;
pub mod concurrency;
pub mod content_filter;
pub mod email;
pub mod extraction;
pub mod formats;
#[cfg(feature = "html")]
pub mod html_output;
pub mod layout;
pub mod llm;
pub mod merge;
pub mod ner;
pub mod ocr;
pub mod page;
pub mod pdf;
pub mod processing;
pub mod redaction;
pub mod reranker;
pub mod summarization;
pub mod transcription;
pub mod translation;
#[cfg(feature = "tree-sitter")]
pub mod tree_sitter;

// Re-export main types for backward compatibility
pub use acceleration::{AccelerationConfig, ExecutionProviderType};
pub use concurrency::ConcurrencyConfig;
pub use content_filter::ContentFilterConfig;
pub use email::EmailConfig;
pub(crate) use extraction::{BatchBytesItem, BatchFileItem};
pub use extraction::{
    ExtractInput, ExtractInputKind, ExtractionConfig, ExtractionErrorItem, ExtractionOutput, ExtractionSummary,
    FileExtractionConfig, ImageExtractionConfig, LanguageDetectionConfig, TokenReductionOptions, UrlExtractionConfig,
    UrlExtractionMode,
};
pub use formats::OutputFormat;
#[cfg(feature = "html")]
pub use html_output::{HtmlOutputConfig, HtmlTheme};
#[cfg(feature = "layout-types")]
pub use layout::{LayoutDetectionConfig, TableModel};
pub use llm::{CallMode, LlmConfig, MergeMode, StructuredExtractionConfig};
pub use ocr::{OcrConfig, OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds, VlmFallbackPolicy};
pub use page::PageConfig;
#[cfg(feature = "pdf")]
pub use pdf::{HierarchyConfig, PdfConfig};
pub use processing::{
    ChunkSizing, ChunkerType, ChunkingConfig, EmbeddingConfig, EmbeddingModelType, PostProcessorConfig,
    TableChunkingMode,
};
pub use reranker::{RerankerConfig, RerankerModelType};
#[cfg(feature = "tree-sitter")]
pub use tree_sitter::{CodeContentMode, TreeSitterConfig, TreeSitterProcessConfig};

// OSS v5 follow-up feature configs.
pub use captioning::CaptioningConfig;
pub use classification::PageClassificationConfig;
pub use ner::{NerBackendKind, NerConfig};
pub use redaction::{RedactionConfig, RedactionPattern, RedactionTerm};
pub use summarization::SummarizationConfig;
pub use translation::TranslationConfig;

#[cfg(feature = "transcription-types")]
pub use transcription::{TranscriptionConfig, WhisperModel};
