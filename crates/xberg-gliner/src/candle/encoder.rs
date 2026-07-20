//! Thin wrapper over `candle_transformers::models::debertav2::DebertaV2Model`.
//!
//! Ported from `anno::backends::gliner2_fastino_candle`. Provides bare-encoder
//! hidden states. Deliberately uses the upstream Candle implementation rather
//! than rolling a custom DeBERTa-v2 disentangled-attention module.

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::debertav2::{Config as DebertaV2Config, DebertaV2Model};

/// Wrapped DeBERTa-v2/v3 encoder. Loaded from safetensors + config.json
/// at the model snapshot root.
pub struct Encoder {
    pub(crate) model: DebertaV2Model,
    pub(crate) config: DebertaV2Config,
}

impl Encoder {
    /// Load the encoder from a `model.safetensors` + `config.json` pair.
    ///
    /// Only used by [`crate::candle::Gliner2Candle::from_local_with_device`] and
    /// [`crate::candle::Gliner2Candle::unload_adapter`]; dead weight on wasm32
    /// (no filesystem).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_safetensors(weights_path: &Path, config_path: &Path, device: &Device) -> crate::candle::Result<Self> {
        let cfg_str = std::fs::read_to_string(config_path).map_err(|e| {
            crate::candle::GlinerCandleError::Backend(format!("encoder config read {}: {e}", config_path.display()))
        })?;
        let config: DebertaV2Config = serde_json::from_str(&cfg_str).map_err(|e| {
            crate::candle::GlinerCandleError::Backend(format!("encoder config parse {}: {e}", config_path.display()))
        })?;

        // SAFETY: VarBuilder::from_mmaped_safetensors mmap-reads the weights
        // file. Safe as long as the file isn't mutated under us; Candle's
        // standard pattern. ~keep
        #[allow(unsafe_code)]
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, device) }
            .map_err(|e| crate::candle::GlinerCandleError::Backend(format!("encoder safetensors: {e}")))?;

        // GLiNER2 stores all encoder tensors under the `encoder.` prefix
        // (e.g. `encoder.embeddings.word_embeddings.weight`). DebertaV2Model
        // expects them at root, so scope into the prefix. ~keep
        let model = DebertaV2Model::load(vb.pp("encoder"), &config)
            .map_err(|e| crate::candle::GlinerCandleError::Backend(format!("encoder DebertaV2Model::load: {e}")))?;

        Ok(Self { model, config })
    }

    /// Load the encoder from in-memory safetensors bytes + parsed config
    /// (wasm/no-fs path). Mirrors [`Self::from_safetensors`] but reads the
    /// weights from a buffer instead of mmap'ing a path. `dtype` lets wasm32
    /// callers request `DType::F16` to halve resident memory after loading;
    /// the source safetensors bytes are always F32, so this only affects
    /// in-memory footprint, not download size.
    pub fn from_buffered_safetensors(
        bytes: &[u8],
        config: &DebertaV2Config,
        device: &Device,
        dtype: candle_core::DType,
    ) -> crate::candle::Result<Self> {
        let tensors = crate::candle::streaming_load::load_buffer_streaming(bytes, device, dtype)?;
        let vb = VarBuilder::from_tensors(tensors, dtype, device);
        Self::from_var_builder(vb.pp("encoder"), config)
    }

    /// Load the encoder from an already-built [`VarBuilder`] + parsed config.
    ///
    /// Used by [`crate::candle::Gliner2Candle::load_adapter`] after the LoRA merge
    /// has produced a `HashMap<String, Tensor>` that's wrapped into a
    /// `VarBuilder::from_tensors`. The caller is responsible for scoping
    /// into the `encoder.` prefix; this constructor calls `DebertaV2Model::load`
    /// directly on the provided VarBuilder.
    pub fn from_var_builder(vb: VarBuilder<'_>, config: &DebertaV2Config) -> crate::candle::Result<Self> {
        let model = DebertaV2Model::load(vb, config).map_err(|e| {
            crate::candle::GlinerCandleError::Backend(format!("encoder DebertaV2Model::load (vb): {e}"))
        })?;
        Ok(Self {
            model,
            config: config.clone(),
        })
    }

    /// Run the encoder forward pass. Returns hidden states of shape
    /// `[batch, seq_len, hidden_size]`.
    ///
    /// `token_type_ids` is optional; pass `None` for single-sequence
    /// inputs (which is GLiNER2's case; the schema prompt + text are
    /// concatenated without segment-A/B distinction).
    pub fn forward(
        &self,
        input_ids: &Tensor,
        attention_mask: &Tensor,
        token_type_ids: Option<&Tensor>,
    ) -> candle_core::Result<Tensor> {
        // DebertaV2Model::forward takes Option<Tensor> (owned). Clone the
        // borrowed inputs; Candle Tensors are Arc-backed so this is cheap. ~keep
        self.model
            .forward(input_ids, token_type_ids.cloned(), Some(attention_mask.clone()))
    }

    /// Hidden size (read from config). Matches the encoder's output
    /// last-dim and is passed to the heads at construction time.
    #[allow(dead_code)]
    pub fn hidden_size(&self) -> usize {
        self.config.hidden_size
    }
}
