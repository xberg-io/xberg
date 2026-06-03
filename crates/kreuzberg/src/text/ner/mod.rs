//! Named-entity recognition (NER).
//!
//! Shared by:
//! - the NER post-processor at `crate::plugins::processor::builtin::ner` (populates
//!   [`ExtractionResult::entities`](crate::types::ExtractionResult::entities))
//! - the redaction engine at `crate::text::redaction::engine` (consumes the same
//!   `Entity` stream to redact PERSON / ORGANIZATION / LOCATION mentions that the
//!   pure-Rust pattern engine cannot detect).
//!
//! Backends implement the [`NerBackend`] trait. Two are bundled:
//!
//! - [`gline::GlineBackend`] under `#[cfg(feature = "ner-onnx")]` — local ONNX
//!   inference via the upstream `gline-rs` crate. Models download lazily from
//!   HuggingFace via [`crate::model_download`].
//! - [`llm::LlmBackend`] under `#[cfg(feature = "ner-llm")]` — liter-llm with a
//!   structured-output schema. Used when categories outstrip the ONNX taxonomy.

#![cfg(feature = "ner")]

pub mod backend;
#[cfg(feature = "ner-onnx")]
pub mod gline;
#[cfg(all(
    feature = "ner-llm",
    not(target_os = "windows"),
    not(all(target_os = "android", target_arch = "x86_64"))
))]
pub mod llm;

pub use backend::NerBackend;

#[cfg(feature = "ner-onnx")]
use std::path::PathBuf;

/// Eagerly download a NER model into the kreuzberg cache.
///
/// `name` is a HuggingFace repo id (e.g. `urchade/gliner_multi-v2.1`). The
/// CLI flag `kreuzberg warm --ner` delegates here.
#[cfg(feature = "ner-onnx")]
pub fn download_model(name: &str, cache_dir: Option<PathBuf>) -> crate::Result<PathBuf> {
    gline::download_model(name, cache_dir)
}

/// Pinned default NER model identifier.
#[cfg(feature = "ner-onnx")]
pub fn default_model_name() -> &'static str {
    gline::DEFAULT_MODEL_REPO
}

/// All NER models kreuzberg knows about (used by `--all-ner-models`).
#[cfg(feature = "ner-onnx")]
pub fn known_models() -> &'static [&'static str] {
    gline::KNOWN_MODELS
}
