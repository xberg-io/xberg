//! PaddleOCR backend using ONNX Runtime.
//!
//! This module provides a PaddleOCR implementation that uses ONNX Runtime
//! for inference, enabling high-quality OCR without Python dependencies.
//!
//! # Features
//!
//! - PP-OCRv4/v5 model support
//! - Excellent CJK (Chinese, Japanese, Korean) recognition
//! - Pure Rust implementation via `paddle-ocr-rs`
//! - Shared ONNX Runtime with embeddings feature
//!
//! # Model Files
//!
//! PaddleOCR requires three model files:
//! - Detection model (`*_det_*.onnx`)
//! - Classification model (`*_cls_*.onnx`)
//! - Recognition model (`*_rec_*.onnx`)
//!
//! Models are auto-downloaded on first use to `~/.cache/kreuzberg/paddle-ocr/`.
//!
//! # Example
//!
//! ```rust,ignore
//! use kreuzberg::ocr::paddle::PaddleOcrBackend;
//! use kreuzberg::plugins::OcrBackend;
//! use kreuzberg::OcrConfig;
//!
//! let backend = PaddleOcrBackend::new()?;
//! let config = OcrConfig {
//!     language: "ch".to_string(),
//!     ..Default::default()
//! };
//!
//! let result = backend.process_image(&image_bytes, &config).await?;
//! println!("Extracted: {}", result.content);
//! ```

mod backend;
mod config;
mod model_manager;

pub use backend::PaddleOcrBackend;
pub use config::{PaddleLanguage, PaddleOcrConfig};
pub use model_manager::{CacheStats, ModelManager, ModelPaths};

/// Supported languages for PaddleOCR.
///
/// PaddleOCR supports 14 optimized language models.
pub const SUPPORTED_LANGUAGES: &[&str] = &[
    "ch",          // Chinese (Simplified)
    "en",          // English
    "french",      // French
    "german",      // German
    "korean",      // Korean
    "japan",       // Japanese
    "chinese_cht", // Chinese (Traditional)
    "ta",          // Tamil
    "te",          // Telugu
    "ka",          // Kannada
    "latin",       // Latin script languages
    "arabic",      // Arabic
    "cyrillic",    // Cyrillic script languages
    "devanagari",  // Devanagari script languages
];

/// Check if a language code is supported by PaddleOCR.
pub fn is_language_supported(lang: &str) -> bool {
    SUPPORTED_LANGUAGES.contains(&lang)
}

/// Map Kreuzberg language codes to PaddleOCR language codes.
pub fn map_language_code(kreuzberg_code: &str) -> Option<&'static str> {
    match kreuzberg_code {
        // Direct mappings
        "ch" | "chi_sim" | "zho" | "zh" | "chinese" => Some("ch"),
        "en" | "eng" | "english" => Some("en"),
        "fr" | "fra" | "french" => Some("french"),
        "de" | "deu" | "german" => Some("german"),
        "ko" | "kor" | "korean" => Some("korean"),
        "ja" | "jpn" | "japanese" => Some("japan"),
        "chi_tra" | "zh_tw" | "zh_hant" => Some("chinese_cht"),
        "ta" | "tam" | "tamil" => Some("ta"),
        "te" | "tel" | "telugu" => Some("te"),
        "ka" | "kan" | "kannada" => Some("ka"),
        "ar" | "ara" | "arabic" => Some("arabic"),
        "ru" | "rus" | "russian" => Some("cyrillic"),
        "hi" | "hin" | "hindi" => Some("devanagari"),
        // Latin script fallback for European languages
        "es" | "spa" | "spanish" | "it" | "ita" | "italian" | "pt" | "por" | "portuguese" | "nl" | "nld" | "dutch"
        | "pl" | "pol" | "polish" => Some("latin"),
        _ => None,
    }
}
