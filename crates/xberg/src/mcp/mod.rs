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

mod allowed_hosts;
mod errors;
mod format;
mod params;
pub(crate) mod prompts;
pub(crate) mod resources;
pub(crate) mod schema;
mod server;

pub use allowed_hosts::{MCP_ALLOWED_HOSTS_ENV, read_mcp_allowed_hosts_from_file, resolve_extra_allowed_hosts};

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

pub use params::{CacheWarmParams, DetectMimeTypeParams, ExtractBatchParams, ExtractParams};
