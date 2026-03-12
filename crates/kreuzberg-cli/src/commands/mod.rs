//! Command modules for Kreuzberg CLI
//!
//! This module organizes the CLI commands into focused submodules:
//! - `extract` - Document extraction commands
//! - `cache` - Cache management operations
//! - `server` - API and MCP server commands
//! - `config` - Configuration loading and discovery

pub mod cache;
pub mod config;
pub mod extract;
#[cfg(any(feature = "api", feature = "mcp"))]
pub mod server;

// Re-export command functions for convenience
pub use cache::{clear_command, stats_command};
pub use config::load_config;
pub use extract::{apply_extraction_overrides, batch_command, extract_command};
#[cfg(feature = "mcp")]
pub use server::mcp_command;
#[cfg(feature = "api")]
pub use server::serve_command;
