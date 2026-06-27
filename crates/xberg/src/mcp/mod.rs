//! Model Context Protocol (MCP) server implementation.
//!
//! Provides an MCP server that exposes Xberg's document extraction
//! capabilities as MCP tools for integration with AI assistants.
//!
//! # Features
//!
//! - **extract**: Extract content from bytes, local paths, file URIs, or URLs
//! - **extract_batch**: Extract content from multiple bytes or URI inputs
//! - **detect_mime_type**: Detect MIME type of a file
//! - **cache_stats**: Get cache statistics
//! - **cache_clear**: Clear the cache
//! - **cache_manifest**: Get model manifest with checksums
//! - **cache_warm**: Download model files for offline use
//! - **get_version**: Get Xberg version info
//! - **chunk_text**: Split text into chunks
//! - **embed_text**: Generate vector embeddings (requires `embeddings` feature)
//! - **extract_structured**: Extract structured data via LLM with JSON schema (requires `liter-llm` feature)
//!
//! # Example
//!
//! ```rust,no_run
//! use xberg::mcp::start_mcp_server;
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
pub(crate) mod prompts;
pub(crate) mod resources;
pub(crate) mod schema;
mod server;

// Re-export public API for backward compatibility

#[allow(unused_imports)]
pub use server::start_mcp_server;
#[cfg(feature = "mcp-http")]
#[allow(unused_imports)]
pub use server::start_mcp_server_http;
#[cfg(feature = "mcp-http")]
#[allow(unused_imports)]
pub use server::start_mcp_server_http_with_config;
#[allow(unused_imports)]
pub use server::start_mcp_server_with_config;

pub use params::{
    CacheWarmParams, ChunkTextParams, DetectMimeTypeParams, EmbedTextParams, ExtractBatchParams, ExtractParams,
    ExtractStructuredParams,
};
