//! GLiNER2 inference heads (Candle).
//!
//! `token_gather`, `schema_gather`, `scorer` are parameter-free utilities.
//! `span_rep`, `count_pred`, `count_lstm` are parametric (Task 5b). The
//! `classifier` head from anno is intentionally NOT ported — this crate
//! ships `extract_ner` parity only (see plan Global Constraints).

pub mod count_lstm;
pub mod count_pred;
pub mod schema_gather;
pub mod scorer;
pub mod span_rep;
pub mod token_gather;

/// Maximum span width baked into the v2 Candle heads' trained weights
/// (`span_rep`'s reshape, `scorer`'s axis sizing). Model-architecture-fixed —
/// see Global Constraints.
pub(crate) const MAX_WIDTH: usize = 8;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use candle_core::Device;
use candle_nn::VarBuilder;

/// Container for the three parametric inference heads.
pub struct AllHeads {
    pub span_rep: span_rep::SpanRep,
    pub count_lstm: count_lstm::CountLstmFixed,
    pub count_pred: count_pred::CountPred,
}

impl AllHeads {
    /// Load all heads' weights from a single safetensors file.
    ///
    /// Only used by [`crate::Gliner2Candle::from_local_with_device`] and
    /// [`crate::Gliner2Candle::unload_adapter`] — dead weight on wasm32
    /// (no filesystem).
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(unsafe_code)]
    pub fn from_safetensors(weights_path: &Path, device: &Device) -> crate::Result<Self> {
        // SAFETY: mmap-reads the weights file; safe as long as it isn't
        // mutated under us — matches `encoder::Encoder`'s pattern.
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, device) }
            .map_err(|e| crate::GlinerCandleError::Backend(format!("heads safetensors: {e}")))?;
        Self::load(vb, device)
    }

    /// Load all heads from an already-built [`VarBuilder`] (post-LoRA-merge path).
    ///
    /// Only used by [`crate::Gliner2Candle::load_adapter`] — dead weight on
    /// wasm32 (no filesystem, and LoRA merge is fs-driven).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_var_builder(vb: VarBuilder<'_>, device: &Device) -> crate::Result<Self> {
        Self::load(vb, device)
    }

    /// Load all heads' weights from in-memory safetensors bytes (wasm/no-fs
    /// path). Mirrors [`Self::from_safetensors`] but reads from a buffer.
    /// `dtype` matches the encoder's dtype (kept in sync by callers) so heads
    /// and encoder share the same in-memory representation.
    pub fn from_buffered_safetensors(bytes: &[u8], device: &Device, dtype: candle_core::DType) -> crate::Result<Self> {
        let tensors = crate::streaming_load::load_buffer_streaming(bytes, device, dtype)?;
        let vb = VarBuilder::from_tensors(tensors, dtype, device);
        Self::load(vb, device)
    }

    fn load(vb: VarBuilder<'_>, device: &Device) -> crate::Result<Self> {
        let span_rep = span_rep::SpanRep::from_var_builder(&vb.pp("span_rep").pp("span_rep_layer"))
            .map_err(|e| crate::GlinerCandleError::Backend(format!("span_rep load: {e}")))?;
        let count_lstm = count_lstm::CountLstmFixed::from_var_builder(&vb.pp("count_embed"), device)
            .map_err(|e| crate::GlinerCandleError::Backend(format!("count_embed load: {e}")))?;
        let count_pred = count_pred::CountPred::from_var_builder(&vb.pp("count_pred"))
            .map_err(|e| crate::GlinerCandleError::Backend(format!("count_pred load: {e}")))?;

        Ok(Self {
            span_rep,
            count_lstm,
            count_pred,
        })
    }
}
