//! Subset of jhqxxx/aha (Apache-2.0) tailored to kreuzberg's three
//! VLM-OCR backends. Populated as Phase 3 lands shared infra.
//!
//! See repo-root `ATTRIBUTIONS.md` § jhqxxx/aha for full attribution,
//! modifications, and license compatibility notes.
//!
//! Phase 3 vendors the shared infra; Phase 4 wires it up to model impls.
//! The dead-code / unused-import suppression below will be removed when Phase 4
//! arrives and every symbol gets a concrete caller.
#![allow(dead_code, unused_imports)]

pub mod image;
pub mod modules;
pub mod qwen2;
pub mod rope;

pub use modules::{
    GateUpDownMLP, NaiveAttention, NaiveAttnGateUpDownMLPBlock, NaiveAttnTwoLinearMLPBlock, QKVCatAttention,
    TwoLinearMLP, eager_attention_forward, get_conv1d, get_conv2d, get_layer_norm, quick_gelu,
};

use candle_core::Tensor;

use crate::error::Result;

// ---------------------------------------------------------------------------
// MultiModalData
// ---------------------------------------------------------------------------

/// Multimodal side-data passed to [`InferenceModel::forward_initial`].
///
/// Each model uses a different set of tensors; they are stored positionally and
/// the concrete `forward_initial` impl indexes into them in the expected order.
#[derive(Clone, Debug)]
pub struct MultiModalData {
    /// Ordered collection of optional tensors (pixel values, grids, masks, …).
    pub data_vec: Vec<Option<Tensor>>,
}

impl MultiModalData {
    /// Create a new [`MultiModalData`] from an ordered list of optional tensors.
    #[must_use]
    pub fn new(data_vec: Vec<Option<Tensor>>) -> Self {
        Self { data_vec }
    }
}

// ---------------------------------------------------------------------------
// InferenceModel trait
// ---------------------------------------------------------------------------

/// Autoregressive inference interface shared by Hunyuan-OCR, DeepSeek-OCR,
/// and PaddleOCR-VL.
///
/// Implementors hold their own KV cache and expose a two-phase forward pass:
/// an initial pass that handles multimodal inputs, then a decode loop that
/// repeatedly calls [`forward_step`].
pub trait InferenceModel {
    /// Initial forward pass — incorporates multimodal side-data (images, grids,
    /// masks) in addition to token embeddings.
    ///
    /// The default implementation ignores `data` and delegates to
    /// [`forward_step`], so existing implementors remain source-compatible.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] on any inference failure.
    fn forward_initial(&mut self, input_ids: &Tensor, seqlen_offset: usize, data: MultiModalData) -> Result<Tensor> {
        let _ = data;
        self.forward_step(input_ids, seqlen_offset)
    }

    /// Decode step — pure-token autoregressive forward pass (no image inputs).
    ///
    /// Called repeatedly after [`forward_initial`] during beam search or
    /// greedy decoding.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] on any inference failure.
    fn forward_step(&mut self, input_ids: &Tensor, seqlen_offset: usize) -> Result<Tensor>;

    /// Decode step with explicit position ids.
    ///
    /// Unlocks the XD-RoPE plug-in path required by Hunyuan-OCR where the
    /// first decoder layer receives custom 2-D position ids while subsequent
    /// layers use the standard offset-based RoPE.
    ///
    /// The default implementation ignores `position_ids` and delegates to
    /// [`forward_step`], so existing implementors remain source-compatible.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] on any inference failure.
    fn forward_step_with_position_ids(
        &mut self,
        input_ids: &Tensor,
        position_ids: Option<&Tensor>,
        seqlen_offset: usize,
    ) -> Result<Tensor> {
        let _ = position_ids;
        self.forward_step(input_ids, seqlen_offset)
    }

    /// Clear all accumulated KV caches so the model can process a new sequence.
    fn clear_cache(&mut self);

    /// Token ids that signal end-of-generation for this model.
    fn stop_token_ids(&self) -> Vec<u32>;
}
