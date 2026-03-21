//! Command modules for Kreuzberg CLI
//!
//! This module organizes the CLI commands into focused submodules:
//! - `extract` - Document extraction commands
//! - `cache` - Cache management operations
//! - `server` - API and MCP server commands
//! - `config` - Configuration loading and discovery
//! - `embed` - Embedding generation commands
//! - `chunk` - Text chunking commands

use anyhow::{Context, Result};
use std::io::Read;

pub mod cache;
pub mod chunk;
pub mod config;
#[cfg(feature = "embeddings")]
pub mod embed;
pub mod extract;
#[cfg(any(feature = "api", feature = "mcp"))]
pub mod server;

// Re-export command functions for convenience
pub use cache::{clear_command, manifest_command, stats_command, warm_command};
pub use chunk::chunk_command;
pub use config::load_config;
#[cfg(feature = "embeddings")]
pub use embed::embed_command;
pub use extract::{apply_extraction_overrides, batch_command, extract_command};
#[cfg(feature = "mcp")]
pub use server::mcp_command;
#[cfg(feature = "api")]
pub use server::serve_command;

/// Read text from stdin, trimming whitespace.
pub fn read_stdin() -> Result<String> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("Failed to read from stdin")?;
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        anyhow::bail!("No input received from stdin. Provide text via --text or pipe it to stdin.");
    }
    Ok(trimmed)
}
