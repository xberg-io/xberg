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
pub mod model_cache;
pub mod panic_context;
pub mod plugins;
pub mod rendering;
pub mod telemetry;
pub mod text;
pub mod types;
pub mod utils;

pub mod table_core;

#[cfg(feature = "tower-service")]
pub mod service;

#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "chunking")]
pub mod chunking;

#[cfg(feature = "liter-llm")]
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

pub use cancellation::CancellationToken;
pub use error::{KreuzbergError, Result};
pub use types::*;

#[cfg(feature = "tokio-runtime")]
pub use core::extractor::{batch_extract_bytes, batch_extract_file};
pub use core::extractor::{extract_bytes, extract_file};

pub use core::extractor::{batch_extract_bytes_sync, extract_bytes_sync};

#[cfg(feature = "tokio-runtime")]
pub use core::extractor::{batch_extract_file_sync, extract_file_sync};

pub use core::config::{
    AccelerationConfig, ChunkSizing, ChunkerType, ChunkingConfig, ContentFilterConfig, EmailConfig, EmbeddingConfig,
    EmbeddingModelType, ExecutionProviderType, ExtractionConfig, FileExtractionConfig, ImageExtractionConfig,
    LanguageDetectionConfig, LlmConfig, OcrConfig, OutputFormat, PageConfig, PostProcessorConfig,
    StructuredExtractionConfig, TokenReductionConfig,
};

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

#[cfg(feature = "tree-sitter")]
pub use core::config::{CodeContentMode, TreeSitterConfig, TreeSitterProcessConfig};
#[cfg(feature = "tree-sitter")]
pub use tree_sitter_language_pack::{
    ChunkContext, CodeChunk, CommentInfo, CommentKind, Diagnostic, DiagnosticSeverity, DocstringFormat, DocstringInfo,
    ExportInfo, ExportKind, FileMetrics, ImportInfo, ProcessConfig, ProcessResult, Span, StructureItem, StructureKind,
    SymbolInfo, SymbolKind, process as process_code,
};

pub use core::mime::{
    DOCX_MIME_TYPE, EXCEL_MIME_TYPE, HTML_MIME_TYPE, JSON_MIME_TYPE, MARKDOWN_MIME_TYPE, PDF_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE, POWER_POINT_MIME_TYPE, SupportedFormat, XML_MIME_TYPE, detect_mime_type,
    detect_mime_type_from_bytes, detect_or_validate, get_extensions_for_mime, list_supported_formats,
    validate_mime_type,
};

pub use core::formats::{KNOWN_FORMATS, is_valid_format_field};

pub use plugins::registry::{
    get_document_extractor_registry, get_ocr_backend_registry, get_post_processor_registry, get_renderer_registry,
    get_validator_registry,
};

#[cfg(feature = "embeddings")]
pub use embeddings::{
    EMBEDDING_PRESETS, EmbeddingPreset, download_model, embed_texts, get_preset, list_presets, warm_model,
};

#[cfg(all(feature = "embeddings", feature = "tokio-runtime"))]
pub use embeddings::embed_texts_async;

/// Serialize an [`ExtractionResult`] to TOON (Token-Oriented Object Notation).
///
/// TOON is a token-efficient alternative to JSON for LLM prompts.
/// Losslessly convertible to/from JSON but uses fewer tokens.
pub fn serialize_to_toon(result: &ExtractionResult) -> Result<String> {
    serde_toon::to_string(result).map_err(|e| KreuzbergError::serialization(format!("TOON serialization failed: {e}")))
}

/// Serialize an [`ExtractionResult`] to pretty-printed JSON.
pub fn serialize_to_json(result: &ExtractionResult) -> Result<String> {
    serde_json::to_string_pretty(result)
        .map_err(|e| KreuzbergError::serialization(format!("JSON serialization failed: {e}")))
}
