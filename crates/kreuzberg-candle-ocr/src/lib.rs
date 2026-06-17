//! # kreuzberg-candle-ocr
//!
//! Candle-based VLM OCR engines for Kreuzberg. Pure-Rust transformer OCR.
//!
//! ## Per-model sub-features
//!
//! - `trocr` — Microsoft TrOCR (printed and handwritten variants). **Line-level
//!   only**: TrOCR is trained to recognise a single line of text per image and
//!   produces poor output on full-page documents. Combine with a text-detection
//!   step (e.g. PaddleOCR's text detector) that crops text regions before
//!   handing each crop to TrOCR.
//! - `paddleocr-vl` — PaddleOCR-VL 0.9B vision-language model. Full-page
//!   multi-task: OCR, tables, formulas, charts. Emits markdown directly.
//! - `glm-ocr` — Z.ai GLM-OCR 0.9B vision-language model (CogViT + GLM-4 +
//!   Multi-Token Prediction). Full-page multi-task: OCR, tables, formulas,
//!   charts, key-information extraction. Emits markdown directly.
//!
//! ## Device acceleration
//!
//! Pass-through features to candle: `cuda`, `metal`, `mkl`, `accelerate`.

#![allow(clippy::too_many_arguments)]

pub mod device;
pub mod error;
pub mod models;
pub(crate) mod vendor;

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
