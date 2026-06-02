//! LLM integration via liter-llm.
//!
//! This module provides VLM OCR, VLM embeddings, structured extraction,
//! and per-region VLM extraction for diagrams and complex layouts.

#[cfg(all(feature = "liter-llm", not(target_os = "windows")))]
pub mod client;
#[cfg(all(feature = "liter-llm", not(target_os = "windows")))]
pub mod prompts;
#[cfg(all(feature = "liter-llm", not(target_os = "windows")))]
pub mod region_extractor;
#[cfg(all(feature = "liter-llm", not(target_os = "windows")))]
pub mod structured;
#[cfg(all(feature = "liter-llm", not(target_os = "windows")))]
pub mod usage;
#[cfg(all(feature = "liter-llm", not(target_os = "windows")))]
pub mod vlm_embeddings;
#[cfg(all(feature = "liter-llm", not(target_os = "windows")))]
pub mod vlm_ocr;
