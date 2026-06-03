//! # kreuzberg-candle-ocr
//!
//! Candle-based VLM OCR engines for Kreuzberg. Pure-Rust transformer OCR.
//!
//! ## Per-model sub-features
//!
//! - `trocr` — Microsoft TrOCR (printed and handwritten variants)
//! - `paddleocr-vl` — PaddleOCR-VL 0.9B (multi-task: OCR, tables, formulas, charts)
//!
//! ## Device acceleration
//!
//! Pass-through features to candle: `cuda`, `metal`, `mkl`, `accelerate`.

#![allow(clippy::too_many_arguments)]

pub mod device;
pub mod error;
pub mod models;

pub use device::DevicePreference;
pub use error::{CandleOcrError, Result};

#[cfg(not(target_arch = "wasm32"))]
pub use candle_core::DType;

/// Identifier for the model emitted by a [`CandleEngine`]. Used by the
/// backend layer to record telemetry and pick decoding hyperparameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    Trocr,
    PaddleOcrVl,
    GotOcr,
    GlmOcr,
}

/// Output produced by a candle OCR engine for a single image.
#[derive(Debug, Clone)]
pub struct CandleOcrOutput {
    /// Recognised content. For VLM backends this is markdown; for TrOCR it is plain text.
    pub content: String,
    /// True if `content` is markdown (and the extraction pipeline should skip
    /// layout-reconstruction stages).
    pub is_structured_markdown: bool,
    /// Optional model-emitted confidence in `[0.0, 1.0]`. `None` if the model
    /// does not expose token-level confidences.
    pub confidence: Option<f32>,
}
