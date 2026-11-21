//! Experimental Rust-only local image captioning from explicitly staged immutable artifacts.
//!
//! The adapter is CPU-only, is selected only by explicit configuration, and stays experimental
//! until the qualification in xberg-io/xberg#2334 posts a go for this workload. It is not a
//! default, a supported preset or a replacement for the liter-llm [`crate::captioning`] APIs.
//!
//! Two pinned export families are described by the canonical
//! [`crate::candidate_models`] inventory:
//!
//! * SmolVLM (`Idefics3`): a SigLIP vision encoder, an embedding graph and a merged decoder with
//!   a key/value cache. The image becomes one 512x512 global frame whose features replace the
//!   repeated image token in the pinned chat prompt.
//! * Florence-2 descriptors and graph pins are validated, but the adapter refuses the family:
//!   its merged decoder fails on cached decoder steps in the ONNX Runtime this crate bundles, so
//!   [`LocalImageCaptioner::new`] returns an explicit error instead of substituting SmolVLM.
//!
//! # What is not qualified
//!
//! These adapters exist to make the candidates measurable, not to claim a result. Qualification
//! (xberg-io/xberg#2334) is outstanding, and these limitations are part of why:
//!
//! * SmolVLM uses a single global frame. The upstream processor's tile splitting
//!   (`do_image_splitting`, `<row_N_col_M>` tags) is not implemented, so large images lose the
//!   per-tile detail the export was trained with and the aspect ratio is not preserved.
//! * Florence-2 is not enabled (see above).
//! * Decoding is greedy with no repetition control or beam search, and this adapter reports no
//!   quality, latency, memory or calibration numbers.
//!
//! Preparation reads only the local Hugging Face cache; it never reaches the network. Staging a
//! missing artifact is a separate, explicit step ([`LocalImageCaptioner::stage`]). A missing,
//! truncated, corrupted or wrong-revision artifact fails with the pinned label instead of
//! falling back to another model or to a provider.
//!
//! The experimental API may change between releases.

#[cfg(feature = "captioning-onnx")]
mod artifacts;
#[cfg(feature = "captioning-onnx")]
mod control;
#[cfg(feature = "captioning-onnx")]
mod generation;
#[cfg(feature = "captioning-onnx")]
mod policy;
#[cfg(feature = "captioning-onnx")]
mod preprocess;
#[cfg(feature = "captioning-onnx")]
mod runtime;
mod types;

#[cfg(feature = "captioning-onnx")]
pub use runtime::LocalImageCaptioner;

pub use types::{
    LocalCaptioningBackend, LocalCaptioningConfig, LocalCaptioningFile, LocalCaptioningFormat, LocalCaptioningGraph,
    LocalCaptioningGraphRole, LocalCaptioningLimits, LocalCaptioningModel, LocalCaptioningModelInfo,
    LocalCaptioningOptions, LocalCaptioningPrecision, LocalCaptioningResult,
};

#[cfg(feature = "captioning-onnx")]
pub(crate) use policy::descriptor_from_selection;

#[cfg(all(test, feature = "captioning-onnx"))]
mod tests;
