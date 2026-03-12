//! HTML to Markdown/Djot conversion functions.
//!
//! This module provides HTML to Markdown and Djot conversion using the `html-to-markdown-rs` library.
//! It supports inline image extraction and YAML frontmatter parsing for HTML metadata.
//!
//! # Features
//!
//! - **HTML to Markdown/Djot conversion**: Clean, readable Markdown or Djot output
//! - **Inline image extraction**: Extract base64 and data URI images
//! - **YAML frontmatter**: Parse YAML metadata from Markdown output
//! - **Customizable conversion**: Full access to `html-to-markdown-rs` options
//! - **Output format selection**: Choose between Markdown and Djot formats
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::extraction::html::convert_html_to_markdown;
//! use kreuzberg::core::config::OutputFormat;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let html = r#"<h1>Title</h1><p>This is <strong>bold</strong> text.</p>"#;
//!
//! // Convert to Markdown (default)
//! let markdown = convert_html_to_markdown(html, None, None)?;
//! assert!(markdown.contains("# Title"));
//! assert!(markdown.contains("**bold**"));
//!
//! // Convert to Djot
//! let djot = convert_html_to_markdown(html, None, Some(OutputFormat::Djot))?;
//! assert!(djot.contains("# Title"));
//! assert!(djot.contains("*bold*")); // Djot uses * for strong
//! # Ok(())
//! # }
//! ```

mod converter;
#[allow(dead_code)]
mod image_handling;
#[allow(dead_code)]
mod processor;
mod stack_management;
mod types;

// Public API re-exports
pub use converter::convert_html_to_markdown;
pub use converter::convert_html_to_markdown_with_metadata;
pub use converter::convert_html_to_markdown_with_tables;
#[cfg(feature = "ocr")]
pub(crate) use converter::map_output_format;
pub use types::{
    CodeBlockStyle, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle, PreprocessingOptions,
    PreprocessingPreset, WhitespaceMode,
};
pub use types::{ExtractedInlineImage, HtmlExtractionResult};

// Re-export from html-to-markdown-rs for convenience
pub use html_to_markdown_rs::ConversionOptions;
