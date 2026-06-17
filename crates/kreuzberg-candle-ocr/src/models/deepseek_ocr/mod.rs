//! DeepSeek-OCR vision-language OCR model.
//!
//! Implements the DeepSeek-OCR architecture combining SAM vision encoder,
//! ViT or Qwen2 vision transformer, CLIP projection, and language decoder
//! for multimodal optical character recognition.
//!
//! # Feature Flag
//!
//! This module is only available with the `deepseek-ocr` feature enabled.

pub mod config;
pub mod engine;
pub mod model;
pub mod processor;
pub mod utils;

pub use config::DeepseekOCRConfig;
pub use engine::DeepseekOCREngine;
pub use model::DeepseekOCRModel;
pub use processor::DeepseekOCRProcessor;
