//! Candle-based GLiNER2 inference with runtime PEFT LoRA adapter merge-at-load.
//!
//! Ported from `anno::backends::gliner2_fastino_candle`. Reuses
//! `xberg-gliner`'s already-shipped V2 schema-prompt encoder
//! (`encode_v2`/`V2Tokenizer`/`V2Splitter`) for tokenization and prompt
//! construction — only the Candle-specific encoder, heads, LoRA merge, and
//! decode logic are ported here.

mod decode;
mod encoder;
mod error;
mod heads;
mod lora;
mod model;
mod pipeline;

pub use error::{GlinerCandleError, Result};
pub use model::Gliner2Candle;
/// Re-export [`xberg_gliner::Span`] so downstream crates that depend only on
/// `xberg-gliner-candle` (without a direct `xberg-gliner` dep) can work with
/// the type returned by [`Gliner2Candle::extract_ner`].
pub use xberg_gliner::Span;

#[cfg(test)]
mod tests;
