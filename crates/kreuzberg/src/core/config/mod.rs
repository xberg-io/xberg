//! Configuration loading and management.
//!
//! This module provides utilities for loading extraction configuration from various
//! sources (TOML, YAML, JSON) and discovering configuration files in the project hierarchy.

pub mod extraction;
pub mod formats;
pub mod ocr;
pub mod page;
pub mod pdf;
pub mod processing;

// Re-export main types for backward compatibility
pub use extraction::{ExtractionConfig, ImageExtractionConfig, LanguageDetectionConfig, TokenReductionConfig};
pub use formats::OutputFormat;
pub use ocr::{OcrConfig, canonical_ocr_backend_name};
pub use page::PageConfig;
#[cfg(feature = "pdf")]
pub use pdf::{HierarchyConfig, PdfConfig};
pub use processing::{ChunkerType, ChunkingConfig, EmbeddingConfig, EmbeddingModelType, PostProcessorConfig};
