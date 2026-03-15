//! Configuration loading and management.
//!
//! This module provides utilities for loading extraction configuration from various
//! sources (TOML, YAML, JSON) and discovering configuration files in the project hierarchy.

pub mod acceleration;
pub mod email;
pub mod extraction;
pub mod formats;
pub mod layout;
pub mod ocr;
pub mod page;
pub mod pdf;
pub mod processing;

// Re-export main types for backward compatibility
pub use acceleration::{AccelerationConfig, ExecutionProviderType};
pub use email::EmailConfig;
pub use extraction::{
    ExtractionConfig, FileExtractionConfig, ImageExtractionConfig, LanguageDetectionConfig, TokenReductionConfig,
};
pub use formats::OutputFormat;
#[cfg(feature = "layout-detection")]
pub use layout::LayoutDetectionConfig;
pub use ocr::{OcrConfig, OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds};
pub use page::PageConfig;
#[cfg(feature = "pdf")]
pub use pdf::{HierarchyConfig, PdfConfig};
pub use processing::{
    ChunkSizing, ChunkerType, ChunkingConfig, EmbeddingConfig, EmbeddingModelType, PostProcessorConfig,
};
