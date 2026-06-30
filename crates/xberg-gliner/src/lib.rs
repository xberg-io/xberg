//! Span-mode GLiNER ONNX inference.
//!
//! This crate vendors the span-mode preprocessing and decoding path from the
//! `gline-rs` project and replaces its pipeline wrapper with direct `ort`
//! session management.

mod config;
mod decode;
mod engine;
mod error;
mod input;
mod preprocess;
mod session;
mod splitter;
mod tensor;
mod tokenizer;
mod v2_decode;
mod v2_preprocess;
mod v2_session;
mod v2_splitter;
mod v2_tensor;
mod v2_tokenizer;

pub use config::{Parameters, RuntimeConfig};
pub use decode::{Span, SpanOutput};
pub use engine::Gliner;
pub use error::{GlinerError, Result};
pub use input::{TextInput, Token};
pub use session::{INPUT_NAMES, OUTPUT_NAMES};

pub(crate) use decode::EntityContext;
pub(crate) use preprocess::EncodedInput;

#[cfg(test)]
mod tests;
