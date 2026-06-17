//! PaddleOCR-VL model: Vision-Language model for document parsing.
//!
//! PaddleOCR-VL 1.5 combines a SigLIP vision encoder with ERNIE 4.5 text decoder
//! for multi-task document understanding (OCR, table/formula/chart recognition).
//!
//! Adapted from aha's PaddleOCR-VL 1.5 vendored implementation.

pub mod config;
pub mod engine;
pub mod model;
pub mod processor;

pub use engine::{PaddleOcrVlEngine, PaddleOcrVlTask};
