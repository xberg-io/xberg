//! Span-mode GLiNER ONNX inference.
//!
//! This crate vendors the span-mode preprocessing and decoding path from the
//! `gline-rs` project and replaces its pipeline wrapper with direct `ort`
//! session management.

mod config;
pub mod decode;
mod engine;
mod error;
mod input;
mod preprocess;
mod session;
mod splitter;
mod tensor;
mod tokenizer;
mod v2_decode;
mod v2_engine;
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
pub use v2_engine::Gliner2;
pub use v2_preprocess::{V2Encoded, encode_v2};
pub use v2_session::{INPUT_NAMES_V2, OUTPUT_NAMES_V2};
pub use v2_splitter::V2Splitter;
pub use v2_tokenizer::{PretokenizedEncoding, PretokenizingTokenizer, V2Tokenizer};

pub(crate) use decode::EntityContext;
pub(crate) use preprocess::EncodedInput;

#[cfg(test)]
mod tests;
