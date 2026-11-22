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
//!   a key/value cache. Preprocessing follows the pinned processor's `do_image_splitting` path:
//!   the image is resized to the processor's `size`, split into `max_image_size` tiles with
//!   `<row_N_col_M>` tags in row-major order, and a squashed global frame is appended last. The
//!   tile count and total image-token expansion are bounded before inference.
//! * Florence-2 descriptors and graph pins are validated, but the adapter refuses the family
//!   explicitly instead of substituting SmolVLM. The earlier refusal cited a merged-decoder
//!   failure in the bundled ONNX Runtime; that does not reproduce on the pinned ONNX Runtime
//!   1.31.0 (`ort` 2.0.0-rc.15 `download-binaries`). Both pinned quantizations load, and the
//!   merged decoder executes its uncached and cached branches with a 581-token encoder context
//!   (`florence_merged_decoder_diagnostic`, `florence_pipeline_diagnostic`). The actual blocker is
//!   pipeline scope, not the graphs: the artifact reader never resolves the declared
//!   `text_encoder_file`, generation has no encoder-decoder or task-token path, and Florence-2
//!   needs per-task post-processing and beam search (`generation_config`: three beams, a
//!   no-repeat trigram) that this adapter does not implement.
//!
//! # What is not qualified
//!
//! These adapters exist to make the candidates measurable, not to claim a result. Qualification
//! (xberg-io/xberg#2334) is outstanding, and these limitations are part of why:
//!
//! * Decoding is greedy with no repetition control or beam search, and this adapter reports no
//!   quality, latency, memory or calibration numbers.
//! * Florence-2 is not enabled (see above); its exclusion is an unimplemented pipeline, not a
//!   demonstrated runtime defect.
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
mod inspection;
#[cfg(feature = "captioning-onnx")]
mod policy;
#[cfg(feature = "captioning-onnx")]
mod preprocess;
#[cfg(feature = "captioning-onnx")]
mod runtime;
mod types;

#[cfg(feature = "captioning-onnx")]
pub use inspection::{LocalCaptioningArtifact, LocalCaptioningArtifactReport, LocalCaptioningArtifactState};
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
mod diagnostics_tests;
#[cfg(all(test, feature = "captioning-onnx"))]
mod tests;
