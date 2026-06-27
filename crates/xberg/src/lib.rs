//! Xberg - High-Performance Document Intelligence Library
//!
//! Xberg is a Rust-first document extraction library with language-agnostic plugin support.
//! It provides fast, accurate extraction from PDFs, images, Office documents, emails, and more.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use xberg::{extract, ExtractInput, ExtractionConfig};
//!
//! # async fn run() -> xberg::Result<()> {
//! let config = ExtractionConfig::default();
//! let output = extract(ExtractInput::uri("document.pdf"), &config).await?;
//! println!("Extracted: {}", output.results[0].content);
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
/// Format-specific document extraction implementations and office metadata types.
pub mod extraction;
pub mod extractors;
#[cfg(all(
    feature = "layout-detection",
    any(feature = "pdf", feature = "ocr", feature = "ocr-wasm")
))]
pub mod model_cache;
pub mod plugins;
pub mod rendering;
pub mod telemetry;
/// Text post-processing: NER, summarisation, redaction, token reduction, and translation.
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

#[cfg(feature = "diff")]
pub mod diff;

// TODO(wasm-llm): `liter-llm` stays in no-ORT/wasm target presets because the
// dependency supports hosted HTTP providers on wasm. The runtime module remains
// disabled until the wasm request/runtime integration is wired and tested.
#[cfg(all(feature = "liter-llm", not(target_arch = "wasm32")))]
pub mod llm;

#[cfg(feature = "embedding-presets")]
pub mod embeddings;

#[cfg(any(feature = "reranker-presets", feature = "reranker"))]
pub mod reranking;

#[cfg(feature = "ocr")]
/// Image preprocessing and DPI utilities for OCR pipelines.
pub mod image;

#[cfg(feature = "language-detection")]
pub mod language_detection;

// Note: `image` module (DPI, resize, preprocessing) requires full `ocr` feature
// due to fast_image_resize dependency. The `ocr` module is available with either
// `ocr` or `ocr-wasm` feature; WASM OCR uses lighter-weight FFI calls via tesseract-wasm.

#[cfg(feature = "stopwords")]
pub mod stopwords;

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
pub mod keywords;

#[cfg(feature = "enrichment")]
pub mod enrichment;

#[cfg(feature = "heuristics")]
pub mod heuristics;

#[cfg(feature = "heuristics")]
pub use heuristics::{
    BoundaryReason, ChunkInfo, ChunkPlan, ChunkingDecision, ChunkingReason, ConfidenceSignals, ConfidenceWeights,
    DocumentBoundary, DocumentMetadata, HeuristicsConfig, HeuristicsError, MultidocInput, MultidocThresholds,
    NoChunkingReason, PageRange, PageSignals, SchemaCompliance, StructuredCallMode, StructuredInput,
    StructuredThresholds, UserChunkConfig, analyze_document, analyze_with_user_chunks,
    boundaries_from_extraction_result, calculate_chunk_plan, calculate_plan_from_overrides, check_format_limits,
    choose_call_mode, detect_boundaries, score_confidence,
};

#[cfg(feature = "presets")]
pub mod presets;

// Native HTTP (liter-llm) + PDF rendering are required, so the structured orchestrator is excluded
// on wasm32. Public entry points (`extract_structured`/`split_and_extract`) are re-exported here in
// the orchestrator wave.
#[cfg(all(feature = "structured", not(target_arch = "wasm32")))]
pub mod structured;

#[cfg(all(feature = "structured", not(target_arch = "wasm32")))]
pub use structured::{
    CacheKey, CitationEnvelope, CitationSource, CitedField, MokaVisionCache, PageImage, PresetSpec, StructuredError,
    StructuredOptions, StructuredOutput, VisionCallCache, VisionConfig, extract_structured, extract_structured_sync,
    split_and_extract, split_and_extract_sync,
};

#[cfg(all(feature = "structured", not(target_arch = "wasm32")))]
pub use structured::bindings::{extract_structured_json, split_and_extract_json};

#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
pub mod ocr;

#[cfg(any(
    feature = "paddle-ocr",
    feature = "embeddings",
    feature = "reranker",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "transcription"
))]
pub mod ort_discovery;

#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "ner-onnx"
))]
pub(crate) mod model_download;

#[cfg(any(feature = "paddle-ocr", feature = "paddle-ocr-types"))]
pub mod paddle_ocr;

#[cfg(feature = "candle-ocr")]
pub mod candle_ocr;

#[cfg(feature = "auto-rotate-types")]
pub mod doc_orientation;

#[cfg(feature = "layout-types")]
pub mod layout;

#[cfg(feature = "pdf")]
pub mod pdf;

// Transcription (audio/video STT) — decode + inference pipeline; config types live under core::config.
#[cfg(feature = "transcription")]
pub mod transcription;

#[cfg(feature = "captioning")]
pub mod captioning;

// ── Error, Result, and all types ─────────────────────────────────────────────
// NOTE: `CancellationToken` is intentionally NOT re-exported here.
// It is an `Arc<AtomicBool>` wrapper that does not cross FFI cleanly.
// Internal callers and FFI shims should reach it via `xberg::cancellation::CancellationToken`.
pub use error::{Result, XbergError};
pub use types::*;

// Office metadata types are nested under `extraction::office_metadata::*` but
// alef-backend-dart's mirror declarations resolve names against the crate
// root (`#[frb(mirror(CoreProperties))]` → `xberg::CoreProperties`).
// Re-export at the root for path resolution; the canonical module path
// remains valid via `extraction::office_metadata`.
#[cfg(feature = "office")]
pub use extraction::office_metadata::{CoreProperties, DocxAppProperties};

// ── Extraction — public API ──────────────────────────────────────────────────
pub use core::extractor::{extract, extract_batch};
#[cfg(feature = "tokio-runtime")]
pub use core::extractor::{extract_batch_sync, extract_sync};

// ── Extraction config types ───────────────────────────────────────────────────
pub use core::config::{
    AccelerationConfig, CallMode, CaptioningConfig, ChunkSizing, ChunkerType, ChunkingConfig, ContentFilterConfig,
    EmailConfig, EmbeddingConfig, EmbeddingModelType, ExecutionProviderType, ExtractInput, ExtractInputKind,
    ExtractionConfig, ExtractionErrorItem, ExtractionOutput, ExtractionSummary, FileExtractionConfig,
    ImageExtractionConfig, LanguageDetectionConfig, LlmConfig, MergeMode, NerBackendKind, NerConfig, OcrConfig,
    OutputFormat, PageClassificationConfig, PageConfig, PostProcessorConfig, RedactionConfig, RedactionPattern,
    RedactionTerm, RerankerConfig, RerankerModelType, StructuredExtractionConfig, SummarizationConfig,
    TableChunkingMode, TokenReductionOptions, TranslationConfig, UrlExtractionConfig, UrlExtractionMode,
};
#[cfg(feature = "transcription-types")]
pub use core::config::{TranscriptionConfig, WhisperModel};
pub use extractors::security::SecurityLimits;

// ── Presets — format + registry + resolver ───────────────────────────────────
#[cfg(feature = "presets")]
pub use presets::{
    LoadError, MetaSchema, Preset, PresetCategory, PresetSample, PresetSummary, Registry, ResolveError, ResolvedPreset,
    resolve,
};
// `CallMode` and `MergeMode` are re-exported unconditionally from `core::config` above —
// they live in `core::config::llm` (always compiled), not behind the `presets` feature.

#[cfg(feature = "quality")]
pub use text::{ReductionLevel, TokenReductionConfig};

// `ner-llm` is liter-llm (HTTP) + pure-Rust `ner`; it has no ORT dependency, so it
// is enabled on the x86_64 Android emulator via `android-target`. However, the
// `text::ner::llm` module itself is gated out on android x86_64 (upstream linkage
// constraint), so the re-export must carry the same exclusion to avoid E0432.
// The stub below covers both the no-ner-llm case and the android-x86_64+ner-llm case.
#[cfg(all(
    feature = "ner-llm",
    not(target_arch = "wasm32"),
    not(all(target_os = "android", target_arch = "x86_64"))
))]
pub use text::ner::llm::LlmBackend;

// Re-export the NerBackend trait at crate root so consumers (e.g. the alef-generated
// JNI shim) can call trait methods like `detect` / `detect_with_custom` on
// `&LlmBackend` after a simple `use core_crate::*;`.
#[cfg(feature = "ner-llm")]
pub use text::ner::NerBackend;

// Stub for every config where the real `text::ner::llm` module is absent:
//   (a) `ner-llm` feature is off entirely, OR
//   (b) `ner-llm` is on but we are on android x86_64 (module gated out there).
// This ensures `LlmBackend` is always in scope for alef-generated bindings.
#[cfg(any(not(feature = "ner-llm"), all(target_os = "android", target_arch = "x86_64")))]
#[derive(Clone, Debug)]
pub struct LlmBackend {
    _config: LlmConfig,
}

#[cfg(any(not(feature = "ner-llm"), all(target_os = "android", target_arch = "x86_64")))]
impl LlmBackend {
    pub fn new(config: LlmConfig) -> Self {
        Self { _config: config }
    }

    pub async fn detect(&self, _text: &str, _categories: &[crate::EntityCategory]) -> Result<Vec<crate::Entity>> {
        Err(crate::XbergError::Other(
            "ner-llm feature not available on this target".into(),
        ))
    }

    pub async fn detect_with_custom(
        &self,
        _text: &str,
        _categories: &[crate::EntityCategory],
        _custom_labels: &[String],
    ) -> Result<Vec<crate::Entity>> {
        Err(crate::XbergError::Other(
            "ner-llm feature not available on this target".into(),
        ))
    }
}

// GlineBackend (GLiNER ONNX NER) and RegionKind (per-region VLM extraction) are
// re-exported here so alef-generated bindings can refer to them as `xberg::GlineBackend`
// and `xberg::RegionKind` without internal module path exposure.
#[cfg(feature = "ner-onnx")]
pub use text::ner::gline::GlineBackend;

// Stub for every config that drops ner-onnx so alef-generated bindings keep
// compiling (Windows, all Android arches, all iOS arches, WASM). Previously
// narrowed to (Windows OR wasm32 OR android x86_64); broadened to match the
// iOS/Android-wide target gates added to the binding crates.
#[cfg(not(feature = "ner-onnx"))]
#[derive(Clone, Debug)]
pub struct GlineBackend {
    pub repo_id: String,
    pub model_path: std::path::PathBuf,
    pub tokenizer_path: std::path::PathBuf,
}

#[cfg(not(feature = "ner-onnx"))]
impl GlineBackend {
    pub fn new(_repo_id: Option<&str>) -> Result<Self> {
        Err(crate::XbergError::Other(
            "ner-onnx feature not available on this target".into(),
        ))
    }

    pub async fn detect(&self, _text: &str, _categories: &[crate::EntityCategory]) -> Result<Vec<crate::Entity>> {
        Err(crate::XbergError::Other(
            "ner-onnx feature not available on this target".into(),
        ))
    }

    pub async fn detect_with_custom(
        &self,
        _text: &str,
        _categories: &[crate::EntityCategory],
        _custom_labels: &[String],
    ) -> Result<Vec<crate::Entity>> {
        Err(crate::XbergError::Other(
            "ner-onnx feature not available on this target".into(),
        ))
    }
}

#[cfg(all(feature = "liter-llm", not(target_arch = "wasm32")))]
pub use llm::region_extractor::RegionKind;

// Stub for targets without liter-llm (WASM) so alef-generated FFI bindings compile.
#[cfg(not(all(feature = "liter-llm", not(target_arch = "wasm32"))))]
/// Per-region VLM extraction type stub.
///
/// Identifies the semantic kind of a document region for VLM-based extraction.
/// This stub is emitted on targets where the `liter-llm` feature is unavailable
/// (WASM, Android x86_64 emulator) so that alef-generated bindings compile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RegionKind {
    /// A figure or illustration region.
    Figure,
    /// A data-dense table region.
    DenseTable,
    /// A region with complex multi-column or mixed layout.
    ComplexLayout,
    /// A figure or table caption.
    Caption,
}

#[cfg(not(all(feature = "liter-llm", not(target_arch = "wasm32"))))]
impl RegionKind {
    /// Returns an empty default prompt string for this stub implementation.
    pub fn default_prompt(self) -> &'static str {
        ""
    }
}

// Public NER API: detect_entities function and backend types.
#[cfg(feature = "ner")]
pub use text::ner::detect_entities;

// Public classification API: classify_document function and existing classify_text.
#[cfg(feature = "classification")]
pub use text::classification::classify_document;

#[cfg(feature = "redaction")]
pub use text::redaction::strategy::TokenCounter;

#[cfg(feature = "api-types")]
pub use core::server_config::ServerConfig;

#[cfg(feature = "pdf")]
pub use core::config::{HierarchyConfig, PdfConfig};

#[cfg(feature = "html")]
pub use core::config::{HtmlOutputConfig, HtmlTheme};
#[cfg(feature = "html")]
pub use rendering::StyledHtmlRenderer;

#[cfg(feature = "paddle-ocr-types")]
pub use paddle_ocr::{ModelPaths, PaddleLanguage, PaddleOcrConfig};

#[cfg(feature = "paddle-ocr")]
pub use paddle_ocr::{ModelCacheStats, ModelManager, ModelManifestEntry, PaddleOcrBackend};

// Re-export canonical CacheStats (generic extraction cache statistics) at the crate root.
// This supersedes the orphan `types::formats::CacheStats` which has been removed.
pub use cache::CacheStats;

#[cfg(feature = "layout-types")]
pub use core::config::{LayoutDetectionConfig, TableModel};

#[cfg(feature = "layout-types")]
pub use layout::types::{BBox, DetectionResult, LayoutClass, LayoutDetection};

#[cfg(feature = "layout-types")]
pub use layout::types::RecognizedTable;
#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
pub use ocr::types::PSMMode;

pub use core::config::{OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds, VlmFallbackPolicy};

#[cfg(feature = "auto-rotate-types")]
pub use doc_orientation::OrientationResult;

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
pub use keywords::{Keyword, KeywordAlgorithm, KeywordConfig};

#[cfg(feature = "keywords-rake")]
pub use keywords::RakeParams;

#[cfg(feature = "keywords-yake")]
pub use keywords::YakeParams;

#[cfg(feature = "markdown-footnotes")]
pub use text::markdown_footnotes::{
    Citation, FootnoteAnchor, FootnoteConfig, FootnoteDefinition, find_footnote_anchors, find_inference_markers,
    find_unmarked_claims, parse_citations, parse_footnote_definitions, verify_excerpt,
};

// DiffLine and CellChange are canonical in types::revisions (unconditional)
// and surfaced at the crate root via `pub use types::*` above.
// The diff feature adds algorithm types on top.
#[cfg(feature = "diff")]
pub use diff::{DiffHunk, DiffOptions, EmbeddedChanges, EmbeddedDiff, ExtractionDiff, TableDiff, compare};

#[cfg(feature = "tree-sitter")]
pub use core::config::{CodeContentMode, TreeSitterConfig, TreeSitterProcessConfig};
#[cfg(feature = "tree-sitter")]
pub use tree_sitter_language_pack::{
    ChunkContext, CodeChunk, CommentInfo, CommentKind, Diagnostic, DiagnosticSeverity, DocstringFormat, DocstringInfo,
    ExportInfo, ExportKind, FileMetrics, ImportInfo, ProcessConfig, ProcessResult, Span, StructureItem, StructureKind,
    SymbolInfo, SymbolKind, process as process_code,
};

// ── MIME / Format Info — public API (4 functions + 1 type) ───────────────────
pub use core::mime::{SupportedFormat, detect_mime_type_from_bytes, get_extensions_for_mime, list_supported_formats};

/// Detect the MIME type of a file at the given path.
///
/// Uses the file extension and optionally the file content to determine the MIME type.
/// Set `check_exists` to `true` to verify the file exists before detection.
pub fn detect_mime_type(path: String, check_exists: bool) -> crate::Result<String> {
    core::mime::detect_mime_type(path, check_exists)
}

// ── PDF Rendering ─────────────────────────────────────────────────────────────
#[cfg(feature = "pdf")]
pub use pdf::render::{pdf_page_count, render_pdf_page_to_png};

// ── Plugin Lifecycle — public API ────────────────────────────────────────────
// Alef extracts plugin-lifecycle fns via the `plugins::{trait_snake}::` alias modules
// (see plugins/mod.rs) so they emit with their fully-qualified path. Skip this
// top-level re-export to avoid generating duplicate bindings.
#[cfg_attr(alef, alef(skip))]
pub use plugins::{
    clear_document_extractors, clear_embedding_backends, clear_ocr_backends, clear_post_processors, clear_renderers,
    clear_reranker_backends, clear_validators, list_document_extractors, list_embedding_backends, list_ocr_backends,
    list_post_processors, list_renderers, list_reranker_backends, list_validators, register_document_extractor,
    register_embedding_backend, register_ocr_backend, register_post_processor, register_renderer,
    register_reranker_backend, register_validator, unregister_document_extractor, unregister_embedding_backend,
    unregister_ocr_backend, unregister_post_processor, unregister_renderer, unregister_reranker_backend,
    unregister_validator,
};

// ── Plugin Traits — public API (for plugin implementors) ─────────────────────
// Re-exported at the top level so plugin implementors can write
// `use xberg::DocumentExtractor` without knowledge of internal module paths.
#[cfg_attr(alef, alef(skip))]
pub use plugins::{
    DocumentExtractor, EmbeddingBackend, OcrBackend, OcrBackendType, PostProcessor, ProcessingStage, Renderer,
    RerankerBackend, Validator,
};

// ── Embeddings — public API (4 functions + 1 type, feature-gated) ────────────
#[cfg(feature = "embedding-presets")]
pub use embeddings::EmbeddingPreset;

/// Embed a list of texts using the configured embedding model.
///
/// Returns a 2D vector where each inner vector is the embedding for the corresponding text.
#[cfg(feature = "embeddings")]
pub fn embed_texts(texts: Vec<String>, config: &core::config::EmbeddingConfig) -> crate::Result<Vec<Vec<f32>>> {
    embeddings::embed_texts(&texts, config)
}

/// Stub for builds without the `embeddings` feature — keeps the symbol available
/// on no-ORT targets (Android x86_64 emulator, WASM) so language bindings that
/// mirror the public API compile; the runtime call returns an unsupported error.
#[cfg(all(feature = "embedding-presets", not(feature = "embeddings")))]
pub fn embed_texts(_texts: Vec<String>, _config: &core::config::EmbeddingConfig) -> crate::Result<Vec<Vec<f32>>> {
    Err(XbergError::validation(
        "embed_texts requires the `embeddings` feature, which depends on ONNX Runtime; \
         not available on this target (Android x86_64 emulator or WASM)",
    ))
}

#[cfg(all(feature = "embeddings", feature = "tokio-runtime"))]
pub use embeddings::embed_texts_async;

#[cfg(all(
    feature = "embedding-presets",
    not(feature = "embeddings"),
    feature = "tokio-runtime"
))]
pub async fn embed_texts_async(
    _texts: Vec<String>,
    _config: &core::config::EmbeddingConfig,
) -> crate::Result<Vec<Vec<f32>>> {
    Err(XbergError::validation(
        "embed_texts_async requires the `embeddings` feature, which depends on ONNX Runtime; \
         not available on this target (Android x86_64 emulator or WASM)",
    ))
}

/// Get an embedding preset by name.
///
/// Returns `None` if no preset with the given name exists. Returns an owned
/// clone so the value is safe to pass across FFI boundaries.
#[cfg(feature = "embedding-presets")]
pub fn get_embedding_preset(name: &str) -> Option<embeddings::EmbeddingPreset> {
    embeddings::get_preset(name)
}

/// List the names of all available embedding presets.
///
/// Returns owned `String`s so the values are safe to pass across FFI boundaries.
#[cfg(feature = "embedding-presets")]
pub fn list_embedding_presets() -> Vec<String> {
    embeddings::list_presets()
}

// ── Embedding-preset stubs for builds without the feature ────────────────────
// The alef-generated xberg-ffi crate references `xberg::EmbeddingPreset`
// unconditionally in FFI return-type positions. Without these stubs, any build
// that omits `embedding-presets` fails to compile xberg-ffi and — if the
// feature was accidentally dropped from a release build — causes a Java
// `UnsatisfiedLinkError` at class-load time (issue #998).  Stubs return
// empty/None so callers degrade gracefully instead of crashing.

/// Stub preset type for builds without the `embedding-presets` feature.
///
/// Field names match the real type so JSON round-trips through
/// `xberg_embedding_preset_from_json` remain schema-compatible. When the
/// feature is absent, `get_embedding_preset` always returns `None`, so the
/// stub is never allocated in practice.
#[cfg(not(feature = "embedding-presets"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingPreset {
    /// Unique preset identifier (e.g. "balanced", "multilingual").
    pub name: String,
    /// Maximum input size in Unicode characters for chunking.
    pub chunk_size: usize,
    /// Overlap in characters between adjacent chunks.
    pub overlap: usize,
    /// HuggingFace repository ID for the model (e.g. "BAAI/bge-small-en-v1.5").
    pub model_repo: String,
    /// Pooling strategy used to aggregate token embeddings (e.g. "mean", "cls").
    pub pooling: String,
    /// ONNX model file name within the repository.
    pub model_file: String,
    /// Number of dimensions in the embedding output vectors.
    pub dimensions: usize,
    /// Human-readable description of the preset's intended use case.
    pub description: String,
}

/// Returns `None` for builds without the `embedding-presets` feature.
#[cfg(not(feature = "embedding-presets"))]
pub fn get_embedding_preset(_name: &str) -> Option<EmbeddingPreset> {
    None
}

/// Returns an empty list for builds without the `embedding-presets` feature.
#[cfg(not(feature = "embedding-presets"))]
pub fn list_embedding_presets() -> Vec<String> {
    Vec::new()
}

// ── Reranking — public API (4 functions + 2 types, feature-gated) ─────────────
/// Re-export `RerankerPreset` when the `reranker-presets` feature is active.
///
/// Since v5.0.0.
#[cfg(feature = "reranker-presets")]
pub use reranking::RerankerPreset;

/// Re-export `RerankedDocument` — needed for stub signatures and result types.
///
/// Since v5.0.0.
#[cfg(any(feature = "reranker-presets", feature = "reranker"))]
pub use reranking::RerankedDocument;

/// Rerank a list of documents by relevance to a query.
///
/// Returns documents sorted descending by score. Applies `top_k` truncation if
/// configured.
///
/// # Errors
///
/// - [`XbergError::Validation`] if `query` is empty or blank.
/// - [`XbergError::MissingDependency`] if ONNX Runtime is not installed (ONNX path).
/// - [`XbergError::Reranking`] if the preset is unknown or model download fails.
///
/// Since v5.0.0.
#[cfg(feature = "reranker")]
pub fn rerank(
    query: String,
    documents: Vec<String>,
    config: &core::config::RerankerConfig,
) -> crate::Result<Vec<reranking::RerankedDocument>> {
    reranking::rerank(query, documents, config)
}

/// Stub for builds without the `reranker` feature — keeps the symbol available
/// on no-ORT targets (Android x86_64 emulator, WASM) so language bindings compile.
///
/// Since v5.0.0.
#[cfg(all(feature = "reranker-presets", not(feature = "reranker")))]
pub fn rerank(
    _query: String,
    _documents: Vec<String>,
    _config: &core::config::RerankerConfig,
) -> crate::Result<Vec<reranking::RerankedDocument>> {
    Err(XbergError::validation(
        "rerank requires the `reranker` feature, which depends on ONNX Runtime; \
         not available on this target (Android x86_64 emulator or WASM)",
    ))
}

#[cfg(all(feature = "reranker", feature = "tokio-runtime"))]
pub use reranking::rerank_async;

/// Stub for builds without the `reranker` feature.
///
/// Since v5.0.0.
#[doc(alias = "rerank")]
#[cfg(all(feature = "reranker-presets", not(feature = "reranker"), feature = "tokio-runtime"))]
pub async fn rerank_async(
    _query: String,
    _documents: Vec<String>,
    _config: &core::config::RerankerConfig,
) -> crate::Result<Vec<reranking::RerankedDocument>> {
    Err(XbergError::validation(
        "rerank_async requires the `reranker` feature, which depends on ONNX Runtime; \
         not available on this target (Android x86_64 emulator or WASM)",
    ))
}

/// Get a reranker preset by name.
///
/// Returns `None` if no preset with the given name exists. Returns an owned
/// clone so the value is safe to pass across FFI boundaries.
///
/// Since v5.0.0.
#[cfg(feature = "reranker-presets")]
pub fn get_reranker_preset(name: &str) -> Option<reranking::RerankerPreset> {
    reranking::get_preset(name)
}

/// List the names of all available reranker presets.
///
/// Returns owned `String`s so the values are safe to pass across FFI boundaries.
///
/// Since v5.0.0.
#[cfg(feature = "reranker-presets")]
pub fn list_reranker_presets() -> Vec<String> {
    reranking::list_presets()
}

// ── Reranker-preset stubs for builds without the feature ─────────────────────
/// Stub preset type for builds without the `reranker-presets` feature.
///
/// Field names match the real type so JSON round-trips remain schema-compatible.
/// When the feature is absent, `get_reranker_preset` always returns `None`, so
/// the stub is never allocated in practice.
///
/// Since v5.0.0.
#[cfg(not(feature = "reranker-presets"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RerankerPreset {
    /// Unique preset identifier (e.g. "balanced", "multilingual").
    pub name: String,
    /// HuggingFace repository ID for the model.
    pub model_repo: String,
    /// ONNX model file name within the repository.
    pub model_file: String,
    /// Maximum token sequence length the model supports.
    pub max_length: usize,
    /// Human-readable description of the preset's intended use case.
    pub description: String,
}

/// Stub result document type for builds without `reranker-presets`.
///
/// Since v5.0.0.
#[cfg(not(feature = "reranker-presets"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RerankedDocument {
    /// Position of this document in the original input slice.
    pub index: usize,
    /// Relevance score in `[0, 1]`.
    pub score: f32,
    /// The document text.
    pub document: String,
}

/// Returns `None` for builds without the `reranker-presets` feature.
///
/// Since v5.0.0.
#[cfg(not(feature = "reranker-presets"))]
pub fn get_reranker_preset(_name: &str) -> Option<RerankerPreset> {
    None
}

/// Returns an empty list for builds without the `reranker-presets` feature.
///
/// Since v5.0.0.
#[cfg(not(feature = "reranker-presets"))]
pub fn list_reranker_presets() -> Vec<String> {
    Vec::new()
}

// ── Captioning — public API (3 functions, feature-gated) ──────────────────────
/// Caption a single image from bytes using a configured LLM.
///
/// # Arguments
///
/// * `image_bytes` - The image data.
/// * `llm_config` - LLM configuration for the VLM call.
/// * `custom_prompt` - Optional custom caption prompt. Uses the default
///   `RegionKind::Caption` prompt when `None`.
///
/// # Returns
///
/// The generated caption text.
///
/// # Errors
///
/// Returns an error if the VLM call fails or if image format detection fails.
///
/// # Example
///
/// ```ignore
/// use xberg::captioning::caption_image;
/// use xberg::LlmConfig;
///
/// # async fn example() -> xberg::Result<()> {
/// let image_bytes = std::fs::read("photo.jpg")?;
/// let config = LlmConfig {
///     model: "openai/gpt-4o-mini".to_string(),
///     ..Default::default()
/// };
/// let caption = caption_image(&image_bytes, &config, None).await?;
/// println!("Caption: {}", caption);
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "captioning", feature = "tokio-runtime"))]
pub use captioning::caption_image;

/// Caption a single image from a file path using a configured LLM.
///
/// # Arguments
///
/// * `path` - Path to the image file.
/// * `llm_config` - LLM configuration for the VLM call.
/// * `custom_prompt` - Optional custom caption prompt. Uses the default
///   `RegionKind::Caption` prompt when `None`.
///
/// # Returns
///
/// The generated caption text.
///
/// # Errors
///
/// Returns an error if the file cannot be read, if image format detection fails,
/// or if the VLM call fails.
///
/// # Example
///
/// ```ignore
/// use xberg::captioning::caption_image_file;
/// use xberg::LlmConfig;
///
/// # async fn example() -> xberg::Result<()> {
/// let config = LlmConfig {
///     model: "openai/gpt-4o-mini".to_string(),
///     ..Default::default()
/// };
/// let caption = caption_image_file("document_page_001.png", &config, None).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "captioning", feature = "tokio-runtime"))]
pub use captioning::caption_image_file;

/// Caption multiple images in a single batch.
///
/// Processes images sequentially (not in parallel). Returns one caption per input image
/// in the same order. If a caption fails, the error is returned immediately without
/// processing remaining images.
///
/// # Arguments
///
/// * `images` - Slice of image byte references to caption.
/// * `llm_config` - LLM configuration for the VLM calls.
/// * `custom_prompt` - Optional custom caption prompt. Uses the default
///   `RegionKind::Caption` prompt when `None`.
///
/// # Returns
///
/// A vector of captions, one per input image, in the same order.
///
/// # Errors
///
/// Returns an error if any VLM call fails.
///
/// # Example
///
/// ```ignore
/// use xberg::captioning::caption_images;
/// use xberg::LlmConfig;
///
/// # async fn example() -> xberg::Result<()> {
/// let image1 = std::fs::read("photo1.jpg")?;
/// let image2 = std::fs::read("photo2.jpg")?;
/// let images = vec![image1.as_ref(), image2.as_ref()];
/// let config = LlmConfig {
///     model: "openai/gpt-4o-mini".to_string(),
///     ..Default::default()
/// };
/// let captions = caption_images(&images, &config, None).await?;
/// assert_eq!(captions.len(), 2);
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "captioning", feature = "tokio-runtime"))]
pub use captioning::caption_images;

// ── Enrichment chokepoint ─────────────────────────────────────────────────────
/// Unified post-extraction enrichment: classification, NER, captioning, and
/// (future) transcription in a single composable call.
pub mod enrich;
pub use enrich::{EnrichedResult, EnrichmentConfig, enrich};

#[cfg(feature = "ner")]
pub use enrich::NerEnrichmentConfig;

#[cfg(feature = "classification")]
pub use enrich::ClassificationEnrichmentConfig;

#[cfg(feature = "captioning")]
pub use enrich::CaptioningEnrichmentConfig;
