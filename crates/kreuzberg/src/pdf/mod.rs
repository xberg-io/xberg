//! PDF document processing utilities.
//!
//! This module provides PDF processing via pdf_oxide (pure Rust).
//! Used internally by the PDF extractor plugin.
//!
//! # Features
//!
//! - **Text extraction**: Extract text content from PDFs using `pdf_oxide`
//! - **Metadata extraction**: Parse PDF metadata (title, author, creation date, etc.)
//! - **Image extraction**: Extract embedded images from PDF pages
//! - **Error handling**: Comprehensive PDF-specific error types
#[cfg(feature = "pdf")]
pub mod bookmarks;
#[cfg(feature = "pdf")]
pub mod embedded_files;
#[cfg(feature = "pdf")]
pub mod error;
#[cfg(feature = "pdf")]
pub mod hierarchy;
#[cfg(feature = "pdf")]
pub mod metadata;
#[cfg(feature = "pdf")]
pub(crate) mod oxide;
#[cfg(feature = "pdf")]
pub(crate) mod oxide_text;
#[cfg(feature = "pdf")]
pub mod render;
#[cfg(feature = "pdf")]
pub mod structure;
#[cfg(feature = "pdf")]
pub mod table_reconstruct;
#[cfg(feature = "pdf")]
pub(crate) mod text;

#[cfg(feature = "pdf")]
pub use crate::core::config::HierarchyConfig;
#[cfg(feature = "pdf")]
pub use error::PdfError;
