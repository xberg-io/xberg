//! OCR (Optical Character Recognition) subsystem.
//!
//! This module provides OCR functionality using Tesseract as the backend.
//! It includes caching, table reconstruction, hOCR parsing, and batch processing.
//!
//! # Features
//!
//! - **Tesseract integration**: Native Tesseract backend via `xberg-tesseract`
//! - **Result caching**: Persistent cache for OCR results using file hashing
//! - **Table reconstruction**: Extract and reconstruct tables from hOCR/TSV output
//! - **hOCR to Markdown**: Convert hOCR format to clean Markdown
//! - **Batch processing**: Process multiple images efficiently
//! - **Language support**: Validate and configure Tesseract languages
//! - **PSM modes**: Support for all Tesseract Page Segmentation Modes
//!
//! # Example
//!
//! Configure OCR through the extraction pipeline:
//!
//! ```no_run
//! use xberg::{extract, ExtractInput, ExtractionConfig, OcrConfig};
//!
//! # async fn run(image_bytes: Vec<u8>) -> xberg::Result<()> {
//! let config = ExtractionConfig {
//!     ocr: Some(OcrConfig::default()),
//!     ..Default::default()
//! };
//! let input = ExtractInput::from_bytes(image_bytes, "image/png", Some("scan.png".into()));
//! let output = extract(input, &config).await?;
//! let document = output
//!     .results
//!     .first()
//!     .ok_or_else(|| xberg::XbergError::Other("OCR produced no document".into()))?;
//! assert!(!document.content.is_empty());
//! # Ok(())
//! # }
//! ```
//!
//! # Optional Feature
//!
//! This module requires the `ocr` feature to be enabled:
//! ```toml
//! [dependencies]
//! xberg = { version = "1.1", features = ["ocr"] }
//! ```
#[cfg(feature = "ocr")]
/// Persistent file-backed cache for OCR results keyed by image hash and config.
pub mod cache;
#[cfg(any(feature = "ocr", paddle_ocr))]
/// Type conversions between internal OCR types and public API types.
pub mod conversion;
/// OCR error types.
pub mod error;
#[cfg(feature = "ocr")]
/// hOCR HTML output parser that extracts word bounding boxes and confidence scores.
pub mod hocr_parser;
#[cfg(all(
    feature = "layout-detection",
    feature = "pdf",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
/// Assembles layout-detection bounding boxes with OCR word spans for region-level extraction.
pub mod layout_assembly;
#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
pub(crate) mod preprocessing;
#[cfg(feature = "ocr")]
/// High-level Tesseract OCR processor with caching and table reconstruction.
pub mod processor;
#[cfg(feature = "ocr")]
pub(crate) mod shaded_rows;
#[cfg(feature = "ocr")]
/// TSV and hOCR table reconstruction utilities.
pub mod table;
#[cfg(feature = "ocr")]
/// Runtime tessdata language pack download utilities.
pub mod tessdata_download;
#[cfg(feature = "ocr")]
/// Tessdata language-pack download and management utilities.
pub mod tessdata_manager;
#[cfg(feature = "ocr")]
/// Native Tesseract backend using `xberg-tesseract` (C FFI).
pub mod tesseract_backend;
#[cfg(all(feature = "ocr-wasm", not(feature = "ocr")))]
/// WebAssembly Tesseract backend using `tesseract-wasm`.
pub mod tesseract_wasm_backend;
/// OCR configuration and result types shared across all backends.
pub mod types;
#[cfg(feature = "ocr")]
/// Utility functions for OCR result hashing and formatting constants.
pub mod utils;
#[cfg(feature = "ocr")]
/// Validation helpers for language codes and Tesseract version constraints.
pub mod validation;

#[cfg(feature = "ocr")]
pub use cache::{OcrCache, OcrCacheStats};
pub use error::OcrError;
#[cfg(feature = "ocr")]
pub use processor::OcrProcessor;
#[cfg(feature = "ocr")]
pub use tessdata_manager::TessdataManager;
#[cfg(feature = "ocr")]
pub use tesseract_backend::TesseractBackend;
#[cfg(all(feature = "ocr-wasm", not(feature = "ocr")))]
pub use tesseract_wasm_backend::TesseractWasmBackend;
pub use types::{BatchItemResult, ExtractionResult, PSMMode, Table, TesseractConfig};
#[cfg(feature = "ocr")]
pub use utils::compute_hash;
