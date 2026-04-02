//! Text chunking utilities.
//!
//! This module provides text chunking functionality using the `text-splitter` library.
//! It splits long text into smaller chunks while preserving semantic boundaries.
//!
//! # Features
//!
//! - **Smart splitting**: Respects word and sentence boundaries
//! - **Markdown-aware**: Preserves Markdown structure (headings, code blocks, lists)
//! - **Configurable overlap**: Overlap chunks to maintain context
//! - **Unicode support**: Handles CJK characters and emojis correctly
//! - **Batch processing**: Process multiple texts efficiently
//!
//! # Chunker Types
//!
//! - **Text**: Generic text splitter, splits on whitespace and punctuation
//! - **Markdown**: Markdown-aware splitter, preserves formatting and structure
//! - **Yaml**: YAML-aware splitter, creates one chunk per top-level key
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::chunking::{chunk_text, ChunkingConfig, ChunkerType};
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let config = ChunkingConfig {
//!     max_characters: 500,
//!     overlap: 50,
//!     trim: true,
//!     chunker_type: ChunkerType::Text,
//!     ..Default::default()
//! };
//!
//! let long_text = "This is a very long document...".repeat(100);
//! let result = chunk_text(&long_text, &config, None)?;
//!
//! println!("Split into {} chunks", result.chunk_count);
//! for (i, chunk) in result.chunks.iter().enumerate() {
//!     println!("Chunk {}: {} chars", i + 1, chunk.content.len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Use Cases
//!
//! - Splitting documents for LLM context windows
//! - Creating overlapping chunks for semantic search
//! - Processing large documents in batches
//! - Maintaining context across chunk boundaries

use once_cell::sync::Lazy;
use std::sync::Arc;

// Module declarations
pub mod boundaries;
mod builder;
pub mod classifier;
pub mod config;
pub mod core;
mod headings;
pub mod processor;
#[cfg(feature = "chunking-tokenizers")]
mod tokenizer_cache;
pub mod validation;
mod yaml_section;

// Re-export submodule types and functions
pub use boundaries::{calculate_page_range, validate_page_boundaries};
pub use classifier::classify_chunk;
pub use config::{ChunkSizing, ChunkerType, ChunkingConfig, ChunkingResult}; // ChunkingConfig re-exported from core::config::processing
pub use core::{chunk_text, chunk_text_with_heading_source, chunk_text_with_type, chunk_texts_batch};
pub use processor::ChunkingProcessor;
pub use validation::{ADAPTIVE_VALIDATION_THRESHOLD, precompute_utf8_boundaries, validate_utf8_boundaries};

use crate::error::Result;

/// Lazy-initialized flag that ensures chunking processor is registered exactly once.
///
/// This static is accessed on first use to automatically register the
/// chunking processor with the plugin registry.
static PROCESSOR_INITIALIZED: Lazy<Result<()>> = Lazy::new(register_chunking_processor);

/// Ensure the chunking processor is registered.
///
/// This function is called automatically when needed.
/// It's safe to call multiple times - registration only happens once.
pub fn ensure_initialized() -> Result<()> {
    PROCESSOR_INITIALIZED
        .as_ref()
        .map(|_| ())
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to register chunking processor: {}", e),
            plugin_name: "text-chunking".to_string(),
        })
}

/// Register the chunking processor with the global registry.
///
/// This function should be called once at application startup to register
/// the chunking post-processor.
///
/// **Note:** This is called automatically on first use.
/// Explicit calling is optional.
pub fn register_chunking_processor() -> Result<()> {
    let registry = crate::plugins::registry::get_post_processor_registry();
    let mut registry = registry.write();

    registry.register(Arc::new(ChunkingProcessor), 50)?;

    Ok(())
}
