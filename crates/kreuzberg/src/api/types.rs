//! API request and response types.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tower::util::BoxCloneService;

use crate::{ExtractionConfig, KreuzbergError, service::ExtractionRequest, types::ExtractionResult};

/// API server size limit configuration.
///
/// Controls maximum sizes for request bodies and multipart uploads.
/// Default limits are set to 100 MB to accommodate typical document processing workloads.
///
/// # Default Values
///
/// - `max_request_body_bytes`: 100 MB (104,857,600 bytes)
/// - `max_multipart_field_bytes`: 100 MB (104,857,600 bytes)
///
/// # Configuration via Environment Variables
///
/// You can override the defaults using these environment variables:
///
/// ```bash
/// # Modern approach (in bytes):
/// export KREUZBERG_MAX_REQUEST_BODY_BYTES=104857600     # 100 MB
/// export KREUZBERG_MAX_MULTIPART_FIELD_BYTES=104857600  # 100 MB
/// ```
///
/// # Examples
///
/// ```
/// use kreuzberg::api::ApiSizeLimits;
///
/// // Default limits (100 MB)
/// let limits = ApiSizeLimits::default();
///
/// // Custom limits (5 GB for both)
/// let limits = ApiSizeLimits {
///     max_request_body_bytes: 5 * 1024 * 1024 * 1024,
///     max_multipart_field_bytes: 5 * 1024 * 1024 * 1024,
/// };
///
/// // Very large documents (100 GB total, 50 GB per file)
/// let limits = ApiSizeLimits {
///     max_request_body_bytes: 100 * 1024 * 1024 * 1024,
///     max_multipart_field_bytes: 50 * 1024 * 1024 * 1024,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ApiSizeLimits {
    /// Maximum size of the entire request body in bytes.
    ///
    /// This applies to the total size of all uploaded files and form data
    /// in a single request. Default: 100 MB (104,857,600 bytes).
    pub max_request_body_bytes: usize,

    /// Maximum size of a single multipart field in bytes.
    ///
    /// This applies to individual files in a multipart upload.
    /// Default: 100 MB (104,857,600 bytes).
    pub max_multipart_field_bytes: usize,
}

impl Default for ApiSizeLimits {
    fn default() -> Self {
        Self {
            max_request_body_bytes: 100 * 1024 * 1024,
            max_multipart_field_bytes: 100 * 1024 * 1024,
        }
    }
}

impl ApiSizeLimits {
    /// Create new size limits with custom values.
    ///
    /// # Arguments
    ///
    /// * `max_request_body_bytes` - Maximum total request size in bytes
    /// * `max_multipart_field_bytes` - Maximum individual file size in bytes
    pub fn new(max_request_body_bytes: usize, max_multipart_field_bytes: usize) -> Self {
        Self {
            max_request_body_bytes,
            max_multipart_field_bytes,
        }
    }

    /// Create size limits from MB values (convenience method).
    ///
    /// # Arguments
    ///
    /// * `max_request_body_mb` - Maximum total request size in megabytes
    /// * `max_multipart_field_mb` - Maximum individual file size in megabytes
    ///
    /// # Examples
    ///
    /// ```
    /// use kreuzberg::api::ApiSizeLimits;
    ///
    /// // 50 MB limits
    /// let limits = ApiSizeLimits::from_mb(50, 50);
    /// ```
    pub fn from_mb(max_request_body_mb: usize, max_multipart_field_mb: usize) -> Self {
        Self {
            max_request_body_bytes: max_request_body_mb * 1024 * 1024,
            max_multipart_field_bytes: max_multipart_field_mb * 1024 * 1024,
        }
    }
}

/// Plugin status information in health response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PluginStatus {
    /// Number of registered OCR backends
    pub ocr_backends_count: usize,
    /// Names of registered OCR backends
    pub ocr_backends: Vec<String>,
    /// Number of registered document extractors
    pub extractors_count: usize,
    /// Number of registered post-processors
    pub post_processors_count: usize,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    /// Health status
    #[cfg_attr(feature = "api", schema(example = "healthy"))]
    pub status: String,
    /// API version
    #[cfg_attr(feature = "api", schema(example = "0.8.0"))]
    pub version: String,
    /// Plugin status (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<PluginStatus>,
}

/// Server information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct InfoResponse {
    /// API version
    #[cfg_attr(feature = "api", schema(example = "0.8.0"))]
    pub version: String,
    /// Whether using Rust backend
    pub rust_backend: bool,
}

/// Extraction response (list of results).
pub type ExtractResponse = Vec<ExtractionResult>;

/// Error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    /// Error type name
    #[cfg_attr(feature = "api", schema(example = "ValidationError"))]
    pub error_type: String,
    /// Error message
    #[cfg_attr(feature = "api", schema(example = "Invalid input provided"))]
    pub message: String,
    /// Stack trace (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traceback: Option<String>,
    /// HTTP status code
    #[cfg_attr(feature = "api", schema(example = 400))]
    pub status_code: u16,
}

/// API server state.
///
/// Holds the default extraction configuration loaded from config file
/// (via discovery or explicit path). Per-request configs override these defaults.
#[derive(Clone)]
pub struct ApiState {
    /// Default extraction configuration
    pub default_config: Arc<ExtractionConfig>,
    /// Tower service for extraction requests.
    ///
    /// Wrapped in `Arc<Mutex>` because `BoxCloneService` is `Send` but not `Sync`,
    /// while `ApiState` must be `Clone + Sync` for Axum's state requirement.
    /// The lock is held only long enough to clone the service.
    pub extraction_service: Arc<Mutex<BoxCloneService<ExtractionRequest, ExtractionResult, KreuzbergError>>>,
}

/// Cache statistics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct CacheStatsResponse {
    /// Cache directory path
    #[cfg_attr(feature = "api", schema(example = "/tmp/kreuzberg-cache"))]
    pub directory: String,
    /// Total number of cache files
    pub total_files: usize,
    /// Total cache size in MB
    pub total_size_mb: f64,
    /// Available disk space in MB
    pub available_space_mb: f64,
    /// Age of oldest file in days
    pub oldest_file_age_days: f64,
    /// Age of newest file in days
    pub newest_file_age_days: f64,
}

/// Cache clear response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct CacheClearResponse {
    /// Cache directory path
    #[cfg_attr(feature = "api", schema(example = "/tmp/kreuzberg-cache"))]
    pub directory: String,
    /// Number of files removed
    pub removed_files: usize,
    /// Space freed in MB
    pub freed_mb: f64,
}

/// Embedding request for generating embeddings from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct EmbedRequest {
    /// Text strings to generate embeddings for (at least one non-empty string required)
    #[cfg_attr(feature = "api", schema(min_items = 1))]
    pub texts: Vec<String>,
    /// Optional embedding configuration (model, batch size, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(value_type = Option<Object>))]
    pub config: Option<crate::core::config::EmbeddingConfig>,
}

/// Embedding response containing generated embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct EmbedResponse {
    /// Generated embeddings (one per input text)
    pub embeddings: Vec<Vec<f32>>,
    /// Model used for embedding generation
    #[cfg_attr(feature = "api", schema(example = "all-MiniLM-L6-v2"))]
    pub model: String,
    /// Dimensionality of the embeddings
    pub dimensions: usize,
    /// Number of embeddings generated
    pub count: usize,
}

/// Default chunker type.
fn default_chunker_type() -> String {
    "text".to_string()
}

/// Chunk request with text and configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ChunkRequest {
    /// Text to chunk (must not be empty)
    #[cfg_attr(feature = "api", schema(example = "This is sample text to chunk.", min_length = 1))]
    pub text: String,
    /// Optional chunking configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ChunkingConfigRequest>,
    /// Chunker type (text, markdown, yaml, or semantic)
    #[serde(default = "default_chunker_type")]
    #[cfg_attr(
        feature = "api",
        schema(example = "text", pattern = "^(text|markdown|yaml|semantic)$")
    )]
    pub chunker_type: String,
}

/// Chunking configuration request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ChunkingConfigRequest {
    /// Maximum characters per chunk (must be greater than overlap, default: 2000)
    #[cfg_attr(feature = "api", schema(minimum = 101, example = 2000))]
    pub max_characters: Option<usize>,
    /// Overlap between chunks in characters (must be less than max_characters, default: 100)
    #[cfg_attr(feature = "api", schema(minimum = 0, maximum = 1999, example = 100))]
    pub overlap: Option<usize>,
    /// Whether to trim whitespace
    pub trim: Option<bool>,
    /// Cosine similarity threshold for semantic topic detection (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_threshold: Option<f32>,
}

/// Chunk response with chunks and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ChunkResponse {
    /// List of chunks
    pub chunks: Vec<ChunkItem>,
    /// Total number of chunks
    pub chunk_count: usize,
    /// Configuration used for chunking
    pub config: ChunkingConfigResponse,
    /// Input text size in bytes
    pub input_size_bytes: usize,
    /// Chunker type used for chunking
    #[cfg_attr(feature = "api", schema(example = "text"))]
    pub chunker_type: String,
}

/// Individual chunk item with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ChunkItem {
    /// Chunk content
    pub content: String,
    /// Byte offset start position
    pub byte_start: usize,
    /// Byte offset end position
    pub byte_end: usize,
    /// Index of this chunk (0-based)
    pub chunk_index: usize,
    /// Total number of chunks
    pub total_chunks: usize,
    /// First page number (optional, for PDF chunking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_page: Option<usize>,
    /// Last page number (optional, for PDF chunking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_page: Option<usize>,
}

/// Version response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct VersionResponse {
    /// Kreuzberg version string
    #[cfg_attr(feature = "api", schema(example = "0.8.0"))]
    pub version: String,
}

/// MIME type detection response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DetectResponse {
    /// Detected MIME type
    #[cfg_attr(feature = "api", schema(example = "application/pdf"))]
    pub mime_type: String,
    /// Original filename (if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Model manifest entry for cache management.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ManifestEntryResponse {
    /// Relative path within the cache directory
    #[cfg_attr(feature = "api", schema(example = "paddle-ocr/det/model.onnx"))]
    pub relative_path: String,
    /// SHA256 checksum of the model file
    pub sha256: String,
    /// Expected file size in bytes
    pub size_bytes: u64,
    /// HuggingFace source URL for downloading
    pub source_url: String,
}

/// Model manifest response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ManifestResponse {
    /// Kreuzberg version
    #[cfg_attr(feature = "api", schema(example = "0.8.0"))]
    pub kreuzberg_version: String,
    /// Total size of all models in bytes
    pub total_size_bytes: u64,
    /// Number of models in the manifest
    pub model_count: usize,
    /// Individual model entries
    pub models: Vec<ManifestEntryResponse>,
}

/// Cache warm request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct WarmRequest {
    /// Download all embedding model presets
    #[serde(default)]
    pub all_embeddings: bool,
    /// Specific embedding model preset to download
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
}

/// Cache warm response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct WarmResponse {
    /// Cache directory used
    pub cache_dir: String,
    /// Models that were downloaded
    pub downloaded: Vec<String>,
    /// Models that were already cached
    pub already_cached: Vec<String>,
}

/// Response from structured extraction endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct StructuredExtractionResponse {
    /// Structured data conforming to the provided JSON schema
    pub structured_output: serde_json::Value,
    /// Extracted document text content
    pub content: String,
    /// Detected MIME type of the input file
    #[cfg_attr(feature = "api", schema(example = "application/pdf"))]
    pub mime_type: String,
}

// ---------------------------------------------------------------------------
// OpenWebUI compatibility types
// ---------------------------------------------------------------------------

/// OpenWebUI "External" engine response format.
///
/// Returned by `PUT /process` for the OpenWebUI external document loader.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct OpenWebDocumentResponse {
    /// Extracted text content
    pub page_content: String,
    /// Document metadata
    pub metadata: OpenWebDocumentMetadata,
}

/// Metadata for the OpenWebUI external document loader response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct OpenWebDocumentMetadata {
    /// Original filename
    #[cfg_attr(feature = "api", schema(example = "document.pdf"))]
    pub source: String,
}

/// OpenWebUI "Docling" engine response format.
///
/// Returned by `POST /v1/convert/file` for docling-serve compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DoclingCompatResponse {
    /// Converted document content
    pub document: DoclingCompatDocument,
    /// Processing status
    #[cfg_attr(feature = "api", schema(example = "success"))]
    pub status: String,
}

/// Document content in the docling-serve response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DoclingCompatDocument {
    /// Markdown content of the converted document
    pub md_content: String,
}

/// Chunking configuration response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ChunkingConfigResponse {
    /// Maximum characters per chunk
    pub max_characters: usize,
    /// Overlap between chunks in characters
    pub overlap: usize,
    /// Whether whitespace was trimmed
    pub trim: bool,
    /// Type of chunker used
    #[cfg_attr(feature = "api", schema(example = "text"))]
    pub chunker_type: String,
    /// Topic threshold used for semantic chunking
    #[cfg_attr(feature = "api", schema(example = "0.75"))]
    pub topic_threshold: Option<f32>,
}
