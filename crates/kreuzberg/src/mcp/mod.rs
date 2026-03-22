//! Model Context Protocol (MCP) server implementation.
//!
//! Provides an MCP server that exposes Kreuzberg's document extraction
//! capabilities as MCP tools for integration with AI assistants.
//!
//! # Features
//!
//! - **extract_file**: Extract content from a file by path
//! - **extract_bytes**: Extract content from base64-encoded bytes
//! - **batch_extract_files**: Extract content from multiple files in parallel
//! - **detect_mime_type**: Detect MIME type of a file
//! - **cache_stats**: Get cache statistics
//! - **cache_clear**: Clear the cache
//! - **cache_manifest**: Get model manifest with checksums
//! - **cache_warm**: Download model files for offline use
//! - **get_version**: Get Kreuzberg version info
//! - **chunk_text**: Split text into chunks
//! - **embed_text**: Generate vector embeddings (requires `embeddings` feature)
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::mcp::start_mcp_server;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     start_mcp_server().await?;
//!     Ok(())
//! }
//! ```

mod errors;
mod format;
mod params;
mod server;

// Re-export public API for backward compatibility
pub use server::{KreuzbergMcp, start_mcp_server, start_mcp_server_with_config};

#[cfg(feature = "mcp-http")]
pub use server::{start_mcp_server_http, start_mcp_server_http_with_config};

pub use params::{
    BatchExtractFilesParams, CacheWarmParams, ChunkTextParams, DetectMimeTypeParams, EmbedTextParams,
    ExtractBytesParams, ExtractFileParams,
};

#[doc(hidden)]
pub use errors::map_kreuzberg_error_to_mcp;
