//! LLM integration via liter-llm.
//!
//! This module provides VLM OCR, VLM embeddings, structured extraction,
//! and per-region VLM extraction for diagrams and complex layouts.

#[cfg(feature = "liter-llm")]
pub mod client;
#[cfg(feature = "liter-llm")]
pub mod prompts;
#[cfg(feature = "liter-llm")]
pub mod region_extractor;
#[cfg(all(feature = "liter-llm", not(target_arch = "wasm32")))]
pub(crate) mod rerank;
#[cfg(feature = "liter-llm")]
pub mod structured;
#[cfg(feature = "liter-llm")]
pub mod text_completion;
#[cfg(feature = "liter-llm")]
pub mod usage;
#[cfg(feature = "liter-llm")]
pub mod vlm_embeddings;
#[cfg(feature = "liter-llm")]
mod vlm_markdown;
#[cfg(feature = "liter-llm")]
pub mod vlm_ocr;
