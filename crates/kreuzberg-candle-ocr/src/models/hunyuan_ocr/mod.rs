//! Hunyuan-OCR backend (vendored from jhqxxx/aha).

pub mod config;
pub mod engine;
pub mod model;
pub mod processor;

pub use config::{HunYuanVLConfig, HunyuanOCRGenerationConfig, HunyuanOCRPreprocessorConfig};
pub use engine::HunyuanOCREngine;
pub use model::HunyuanVLModel;
pub use processor::{HunyuanData, HunyuanVLProcessor};
