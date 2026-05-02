//! Kreuzberg - High-Performance Document Intelligence Library
//!
//! Kreuzberg is a Rust-first document extraction library with language-agnostic plugin support.
//! It provides fast, accurate extraction from PDFs, images, Office documents, emails, and more.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use kreuzberg::{extract_file_sync, ExtractionConfig};
//!
//! # fn main() -> kreuzberg::Result<()> {
//! // Extract content from a file
//! let config = ExtractionConfig::default();
//! let result = extract_file_sync("document.pdf", None, &config)?;
//! println!("Extracted: {}", result.content);
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! - **Core Module** (`core`): Main extraction orchestration, MIME detection, config loading
//! - **Plugin System**: Language-agnostic plugin architecture
//! - **Extractors**: Format-specific extraction (PDF, images, Office docs, email, etc.)
//! - **OCR**: Multiple OCR backend support (Tesseract, EasyOCR, PaddleOCR)
//!
//! # Features
//!
//! - Fast parallel processing with async/await
//! - Priority-based extractor selection
//! - Comprehensive MIME type detection (118+ file extensions)
//! - Configurable caching and quality processing
//! - Cross-language plugin support (Python, Node.js planned)

#![deny(unsafe_code)]

pub mod cache;
pub(crate) mod cache_dir;
pub mod cancellation;
pub mod core;
pub mod error;
pub mod extraction;
pub mod extractors;
#[cfg(feature = "layout-detection")]
pub mod model_cache;
pub mod plugins;
pub mod rendering;
pub mod telemetry;
pub mod text;
pub mod types;
pub mod utils;

#[cfg(any(feature = "ocr", feature = "pdf", feature = "paddle-ocr"))]
pub mod table_core;

#[cfg(feature = "tower-service")]
pub mod service;

#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "chunking")]
pub mod chunking;

#[cfg(all(feature = "liter-llm", not(target_os = "windows"), not(target_arch = "wasm32")))]
pub mod llm;

#[cfg(feature = "embeddings")]
pub mod embeddings;

#[cfg(feature = "ocr")]
pub mod image;

#[cfg(feature = "language-detection")]
pub mod language_detection;

// Note: `image` module (DPI, resize, preprocessing) requires full `ocr` feature
// due to fast_image_resize dependency. The `ocr` module requires tokio and native
// deps (JP2, JBIG2), so it stays `ocr`-only. WASM OCR uses the JS bridge instead.

#[cfg(feature = "stopwords")]
pub mod stopwords;

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
pub mod keywords;

#[cfg(feature = "ocr")]
pub mod ocr;

#[cfg(any(
    feature = "paddle-ocr",
    feature = "embeddings",
    feature = "layout-detection",
    feature = "auto-rotate"
))]
pub mod ort_discovery;

#[cfg(any(feature = "paddle-ocr", feature = "layout-detection", feature = "auto-rotate"))]
pub(crate) mod model_download;

#[cfg(feature = "paddle-ocr")]
pub mod paddle_ocr;

#[cfg(feature = "auto-rotate")]
pub mod doc_orientation;

#[cfg(feature = "layout-detection")]
pub mod layout;

#[cfg(feature = "pdf")]
pub mod pdf;

// ── Error, Result, and all types ─────────────────────────────────────────────
// NOTE: `CancellationToken` is intentionally NOT re-exported here.
// It is an `Arc<AtomicBool>` wrapper that does not cross FFI cleanly.
// Internal callers and FFI shims should reach it via `kreuzberg::cancellation::CancellationToken`.
pub use error::{KreuzbergError, Result};
pub use types::*;

// ── Extraction — public API (8 functions) ────────────────────────────────────
#[cfg(feature = "tokio-runtime")]
pub use core::extractor::{batch_extract_bytes, batch_extract_files};
pub use core::extractor::{extract_bytes, extract_file};

pub use core::extractor::{batch_extract_bytes_sync, extract_bytes_sync};

#[cfg(feature = "tokio-runtime")]
pub use core::extractor::{batch_extract_files_sync, extract_file_sync};

// ── Extraction config types ───────────────────────────────────────────────────
pub use core::config::{
    AccelerationConfig, BatchBytesItem, BatchFileItem, ChunkSizing, ChunkerType, ChunkingConfig, ContentFilterConfig,
    EmailConfig, EmbeddingConfig, EmbeddingModelType, ExecutionProviderType, ExtractionConfig, FileExtractionConfig,
    ImageExtractionConfig, LanguageDetectionConfig, LlmConfig, OcrConfig, OutputFormat, PageConfig,
    PostProcessorConfig, StructuredExtractionConfig, TokenReductionOptions,
};

#[cfg(feature = "quality")]
pub use text::{ReductionLevel, TokenReductionConfig};

#[cfg(feature = "api")]
pub use core::server_config::ServerConfig;

#[cfg(feature = "pdf")]
pub use core::config::{HierarchyConfig, PdfBackend, PdfConfig};

#[cfg(feature = "html")]
pub use core::config::{HtmlOutputConfig, HtmlTheme};
#[cfg(feature = "html")]
pub use rendering::StyledHtmlRenderer;

#[cfg(feature = "paddle-ocr")]
pub use paddle_ocr::{CacheStats, ModelManager, ModelPaths, PaddleLanguage, PaddleOcrBackend, PaddleOcrConfig};

#[cfg(feature = "layout-detection")]
pub use core::config::{LayoutDetectionConfig, TableModel};

#[cfg(feature = "layout-detection")]
pub use layout::types::{BBox, DetectionResult, LayoutClass, LayoutDetection};

#[cfg(all(feature = "ocr", feature = "layout-detection"))]
pub use ocr::layout_assembly::RecognizedTable;
#[cfg(feature = "ocr")]
pub use ocr::types::PSMMode;

pub use core::config::{OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds};

#[cfg(feature = "auto-rotate")]
pub use doc_orientation::OrientationResult;

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
pub use keywords::{Keyword, KeywordAlgorithm, KeywordConfig};

#[cfg(feature = "keywords-rake")]
pub use keywords::RakeParams;

#[cfg(feature = "keywords-yake")]
pub use keywords::YakeParams;

#[cfg(feature = "tree-sitter")]
pub use core::config::{CodeContentMode, TreeSitterConfig, TreeSitterProcessConfig};
#[cfg(feature = "tree-sitter")]
pub use tree_sitter_language_pack::{
    ChunkContext, CodeChunk, CommentInfo, CommentKind, Diagnostic, DiagnosticSeverity, DocstringFormat, DocstringInfo,
    ExportInfo, ExportKind, FileMetrics, ImportInfo, ProcessConfig, ProcessResult, Span, StructureItem, StructureKind,
    SymbolInfo, SymbolKind, process as process_code,
};

// ── MIME / Format Info — public API (3 functions) ────────────────────────────
pub use core::mime::{SupportedFormat, detect_mime_type_from_bytes, get_extensions_for_mime};

/// Detect the MIME type of a file at the given path.
///
/// Uses the file extension and optionally the file content to determine the MIME type.
/// Set `check_exists` to `true` to verify the file exists before detection.
pub fn detect_mime_type(path: String, check_exists: bool) -> crate::Result<String> {
    core::mime::detect_mime_type(path, check_exists)
}

// ── PDF Rendering — public API (1 function, feature-gated) ───────────────────
#[cfg(feature = "pdf")]
pub use pdf::rendering::render_pdf_page_to_png;

// ── Plugin Lifecycle — public API (13 functions) ─────────────────────────────
pub use plugins::extractor::list_document_extractors;
pub use plugins::list_ocr_backends;
pub use plugins::list_post_processors;
pub use plugins::list_validators;
pub use plugins::{
    clear_ocr_backends, clear_post_processors, clear_validators, register_ocr_backend, register_post_processor,
    register_validator, unregister_ocr_backend, unregister_post_processor, unregister_validator,
};

// ── Embeddings — public API (4 functions + 1 type, feature-gated) ────────────
#[cfg(feature = "embeddings")]
pub use embeddings::EmbeddingPreset;

/// Embed a list of texts using the configured embedding model.
///
/// Returns a 2D vector where each inner vector is the embedding for the corresponding text.
#[cfg(feature = "embeddings")]
pub fn embed_texts(texts: Vec<String>, config: Option<core::config::EmbeddingConfig>) -> crate::Result<Vec<Vec<f32>>> {
    embeddings::embed_texts(&texts, &config.unwrap_or_default())
}

#[cfg(all(feature = "embeddings", feature = "tokio-runtime"))]
pub use embeddings::embed_texts_async;

/// Get an embedding preset by name.
///
/// Returns `None` if no preset with the given name exists. Returns an owned
/// clone so the value is safe to pass across FFI boundaries.
#[cfg(feature = "embeddings")]
pub fn get_embedding_preset(name: &str) -> Option<embeddings::EmbeddingPreset> {
    embeddings::get_preset(name)
}

/// List the names of all available embedding presets.
///
/// Returns owned `String`s so the values are safe to pass across FFI boundaries.
#[cfg(feature = "embeddings")]
pub fn list_embedding_presets() -> Vec<String> {
    embeddings::list_presets()
}
