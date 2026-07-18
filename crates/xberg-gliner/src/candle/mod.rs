//! Candle-based GLiNER2 inference with runtime PEFT LoRA adapter merge-at-load.
//!
//! Ported from `anno::backends::gliner2_fastino_candle`. Reuses the crate's
//! V2 schema-prompt encoder (`encode_v2`/`V2Tokenizer`/`V2Splitter`) for
//! tokenization and prompt construction; only the Candle-specific encoder,
//! heads, LoRA merge, and decode logic are ported here.

mod decode;
mod encoder;
mod error;
mod heads;
/// PEFT LoRA adapter loading and merge-at-load. Only used by
/// [`Gliner2Candle::load_adapter`]/[`Gliner2Candle::unload_adapter`]; dead
/// weight on wasm32 (no filesystem, and LoRA merge is fs-driven).
#[cfg(not(target_arch = "wasm32"))]
mod lora;
mod model;
mod pipeline;
mod streaming_load;

pub use error::{GlinerCandleError, Result};
pub use model::Gliner2Candle;

#[cfg(test)]
mod tests;
