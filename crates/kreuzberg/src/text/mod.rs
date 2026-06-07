pub mod utf8_validation;

#[cfg(feature = "quality")]
pub mod quality;

#[cfg(feature = "quality")]
pub mod string_utils;

#[cfg(feature = "quality")]
pub mod token_reduction;

#[cfg(feature = "quality")]
pub mod quality_processor;

#[cfg(feature = "quality")]
pub use quality_processor::QualityProcessor;

#[cfg(feature = "quality")]
pub use token_reduction::{ReductionLevel, TokenReductionConfig};

// OSS v5 follow-up text-analysis modules. Each subsystem is feature-gated so the
// non-OSS targets (no-ort-target, wasm-target, android-target) compile out cleanly.
#[cfg(feature = "classification")]
pub mod classification;

// Stub module when classification feature is disabled (wasm-target, android-target have no ORT).
#[cfg(not(feature = "classification"))]
/// Page-classification API stub (classification feature not enabled on this target).
pub mod classification {
    use crate::{ExtractionResult, PageClassificationConfig, Result};

    /// Classify pages in an extraction result.
    pub async fn classify_pages(_result: &mut ExtractionResult, _config: &PageClassificationConfig) -> Result<()> {
        Err(crate::KreuzbergError::Other(
            "classification feature not available on this target".into(),
        ))
    }
}

#[cfg(feature = "ner")]
pub mod ner;

// Stub module for Android x86_64 when ner feature is disabled (android-target has no ORT prebuilt).
// Allows alef-generated bindings to reference types and functions without compilation errors.
#[cfg(not(feature = "ner"))]
/// Named-entity recognition API stub (ner feature not enabled on this target).
pub mod ner {
    use crate::Result;
    use std::path::PathBuf;

    /// NER backend trait (stub for Android x86_64).
    pub trait NerBackend: Send + Sync {}

    /// Download a NER model into the kreuzberg cache.
    pub fn download_model(_name: &str, _cache_dir: Option<PathBuf>) -> Result<PathBuf> {
        Err(crate::KreuzbergError::Other(
            "ner feature not available on this target".into(),
        ))
    }

    /// Default NER model identifier.
    pub fn default_model_name() -> &'static str {
        "gliner-stub"
    }

    /// All NER models kreuzberg knows about.
    pub fn known_models() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(feature = "redaction")]
pub mod redaction;
#[cfg(feature = "summarization")]
pub mod summarization;

#[cfg(feature = "translation")]
pub mod translation;

// Stub module when translation feature is disabled (wasm-target, android-target have no ORT).
#[cfg(not(feature = "translation"))]
/// Translation API stub (translation feature not enabled on this target).
pub mod translation {
    use crate::{ExtractionResult, Result, TranslationConfig};

    /// Translate an extraction result.
    pub async fn translate_result(_result: &mut ExtractionResult, _config: &TranslationConfig) -> Result<()> {
        Err(crate::KreuzbergError::Other(
            "translation feature not available on this target".into(),
        ))
    }
}
