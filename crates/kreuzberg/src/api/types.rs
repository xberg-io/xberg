//! API request and response types.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{ExtractionConfig, types::ExtractionResult};

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
///
/// # Legacy approach (in MB, applies to both limits):
/// export KREUZBERG_MAX_UPLOAD_SIZE_MB=100  # 100 MB
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

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Health status
    pub status: String,
    /// API version
    pub version: String,
}

/// Server information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoResponse {
    /// API version
    pub version: String,
    /// Whether using Rust backend
    pub rust_backend: bool,
}

/// Extraction response (list of results).
pub type ExtractResponse = Vec<ExtractionResult>;

/// Error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error type name
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Stack trace (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traceback: Option<String>,
    /// HTTP status code
    pub status_code: u16,
}

/// API server state.
///
/// Holds the default extraction configuration loaded from config file
/// (via discovery or explicit path). Per-request configs override these defaults.
#[derive(Debug, Clone)]
pub struct ApiState {
    /// Default extraction configuration
    pub default_config: Arc<ExtractionConfig>,
}

/// Cache statistics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsResponse {
    /// Cache directory path
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
pub struct CacheClearResponse {
    /// Cache directory path
    pub directory: String,
    /// Number of files removed
    pub removed_files: usize,
    /// Space freed in MB
    pub freed_mb: f64,
}

/// Embedding request for generating embeddings from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    /// Text strings to generate embeddings for
    pub texts: Vec<String>,
    /// Optional embedding configuration (model, batch size, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<crate::core::config::EmbeddingConfig>,
}

/// Embedding response containing generated embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
    /// Generated embeddings (one per input text)
    pub embeddings: Vec<Vec<f32>>,
    /// Model used for embedding generation
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
pub struct ChunkRequest {
    /// Text to chunk
    pub text: String,
    /// Optional chunking configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ChunkingConfigRequest>,
    /// Chunker type (text or markdown)
    #[serde(default = "default_chunker_type")]
    pub chunker_type: String,
}

/// Chunking configuration request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkingConfigRequest {
    /// Maximum characters per chunk
    pub max_characters: Option<usize>,
    /// Overlap between chunks in characters
    pub overlap: Option<usize>,
    /// Whether to trim whitespace
    pub trim: Option<bool>,
}

/// Chunk response with chunks and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub chunker_type: String,
}

/// Individual chunk item with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Chunking configuration response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfigResponse {
    /// Maximum characters per chunk
    pub max_characters: usize,
    /// Overlap between chunks in characters
    pub overlap: usize,
    /// Whether whitespace was trimmed
    pub trim: bool,
    /// Type of chunker used
    pub chunker_type: String,
}
