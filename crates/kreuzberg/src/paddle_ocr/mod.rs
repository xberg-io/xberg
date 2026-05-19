//! PaddleOCR backend using ONNX Runtime.
//!
//! This module provides a PaddleOCR implementation that uses ONNX Runtime
//! for inference, enabling high-quality OCR without Python dependencies.
//!
//! # Features
//!
//! - PP-OCRv5 model support (server detection, per-family recognition)
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

#[cfg(feature = "paddle-ocr")]
mod backend;
mod config;
mod model_manager;

#[cfg(feature = "paddle-ocr")]
pub use backend::PaddleOcrBackend;
pub use config::{PaddleLanguage, PaddleOcrConfig};
pub use model_manager::{ModelPaths, RecModelPaths, ResolvedRecModel, SharedModelPaths};

#[cfg(feature = "paddle-ocr")]
pub use model_manager::{ModelCacheStats, ModelManager, ModelManifestEntry};

/// Supported languages for PaddleOCR.
///
/// PaddleOCR supports 15+ optimized language models covering 80+ languages
/// via 11 script-family recognition models (all PP-OCRv5).
pub const SUPPORTED_LANGUAGES: &[&str] = &[
    "ch",          // Chinese (Simplified)
    "en",          // English
    "french",      // French
    "german",      // German
    "korean",      // Korean
    "japan",       // Japanese
    "chinese_cht", // Chinese (Traditional)
    "latin",       // Latin script languages
    "cyrillic",    // Cyrillic script languages
    "thai",        // Thai
    "greek",       // Greek
    "arabic",      // Arabic script languages
    "devanagari",  // Hindi, Marathi, Sanskrit, Nepali
    "tamil",       // Tamil
    "telugu",      // Telugu
];

/// Check if a language code is supported by PaddleOCR.
#[allow(dead_code)] // consumed by `paddle-ocr`; types crate ships it for ABI shape
pub(crate) fn is_language_supported(lang: &str) -> bool {
    SUPPORTED_LANGUAGES.contains(&lang)
}

/// Map a PaddleOCR language code to its script family.
///
/// Script families group languages that share a single recognition model.
/// For example, French, German, and Spanish all use the `latin` rec model.
/// Chinese simplified, traditional, and Japanese share the `chinese` rec model.
///
/// # Script Families (11, all PP-OCRv5)
///
/// | Family | Languages |
/// |---|---|
/// | `english` | English |
/// | `chinese` | Chinese (simplified+traditional), Japanese |
/// | `latin` | French, German, Spanish, Italian, 40+ more |
/// | `korean` | Korean |
/// | `eslav` | Russian, Ukrainian, Belarusian |
/// | `thai` | Thai |
/// | `greek` | Greek |
/// | `arabic` | Arabic, Persian, Urdu |
/// | `devanagari` | Hindi, Marathi, Sanskrit, Nepali |
/// | `tamil` | Tamil |
/// | `telugu` | Telugu |
#[allow(dead_code)] // consumed by `paddle-ocr`; types crate ships it for ABI shape
pub(crate) fn language_to_script_family(paddle_lang: &str) -> &'static str {
    match paddle_lang {
        "en" => "english",
        "ch" | "japan" | "chinese_cht" => "chinese",
        "korean" => "korean",
        "french" | "german" | "latin" => "latin",
        "cyrillic" => "eslav",
        "thai" => "thai",
        "greek" => "greek",
        "arabic" => "arabic",
        "devanagari" => "devanagari",
        "tamil" => "tamil",
        "telugu" => "telugu",
        _ => "english",
    }
}

/// Map Kreuzberg language codes to PaddleOCR language codes.
#[allow(dead_code)] // consumed by `paddle-ocr`; types crate ships it for ABI shape
pub(crate) fn map_language_code(kreuzberg_code: &str) -> Option<&'static str> {
    match kreuzberg_code {
        // Direct mappings
        "ch" | "chi_sim" | "zho" | "zh" | "chinese" => Some("ch"),
        "en" | "eng" | "english" => Some("en"),
        "fr" | "fra" | "french" => Some("french"),
        "de" | "deu" | "german" => Some("german"),
        "ko" | "kor" | "korean" => Some("korean"),
        "ja" | "jpn" | "japanese" | "japan" => Some("japan"),
        "chi_tra" | "zh_tw" | "zh_hant" | "chinese_cht" => Some("chinese_cht"),
        "ru" | "rus" | "russian" | "uk" | "ukr" | "ukrainian" | "be" | "bel" | "belarusian" | "cyrillic" => {
            Some("cyrillic")
        }
        "th" | "tha" | "thai" => Some("thai"),
        "el" | "ell" | "greek" => Some("greek"),
        // Arabic script languages
        "ar" | "ara" | "arabic" | "fa" | "fas" | "persian" | "ur" | "urd" | "urdu" => Some("arabic"),
        // Devanagari script languages
        "hi" | "hin" | "hindi" | "mr" | "mar" | "marathi" | "sa" | "san" | "sanskrit" | "ne" | "nep" | "nepali"
        | "devanagari" => Some("devanagari"),
        // Tamil
        "ta" | "tam" | "tamil" => Some("tamil"),
        // Telugu
        "te" | "tel" | "telugu" => Some("telugu"),
        // Latin script fallback for European languages
        "latin" | "es" | "spa" | "spanish" | "it" | "ita" | "italian" | "pt" | "por" | "portuguese" | "nl" | "nld"
        | "dutch" | "pl" | "pol" | "polish" | "sv" | "swe" | "swedish" | "da" | "dan" | "danish" | "no" | "nor"
        | "norwegian" | "fi" | "fin" | "finnish" | "cs" | "ces" | "czech" | "sk" | "slk" | "slovak" | "hr" | "hrv"
        | "croatian" | "hu" | "hun" | "hungarian" | "ro" | "ron" | "romanian" | "tr" | "tur" | "turkish" | "id"
        | "ind" | "indonesian" | "ms" | "msa" | "malay" | "vi" | "vie" | "vietnamese" => Some("latin"),
        _ => None,
    }
}
