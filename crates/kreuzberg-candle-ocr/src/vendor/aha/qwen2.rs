//! Qwen2 decoder vendored for DeepSeek-OCR's vision-language fusion.
//!
//! Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.
//!
//! DeepSeek-OCR uses a `forward_no_cache` path where the full sequence is processed
//! at once without maintaining a mutable KV cache across decode steps. This variant
//! is compatible with stateless forward passes needed during image processing prefill.

use candle_core::Tensor;
use candle_nn::{Activation, Linear, Module, RmsNorm, VarBuilder, linear, linear_no_bias, rms_norm};

use crate::error::Result;

use super::{
    eager_attention_forward,
    rope::{RoPE, apply_rotary_pos_emb},
};

/// Qwen2 model configuration.
///
/// Defines the architecture of the Qwen2 decoder (dimensions, layer counts,
/// position embeddings, normalization, activation).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Qwen2Config {
    /// Vocabulary size.
    pub vocab_size: usize,
    /// Embedding and hidden dimension.
    pub hidden_size: usize,
    /// Feed-forward intermediate dimension.
    pub intermediate_size: usize,
    /// Number of transformer layers.
    pub num_hidden_layers: usize,
    /// Number of query attention heads.
    pub num_attention_heads: usize,
    /// Number of key-value cache heads (for grouped query attention).
    pub num_key_value_heads: usize,
    /// Maximum sequence length for position embeddings.
    pub max_position_embeddings: usize,
    /// Sliding window size for local attention (if used).
    pub sliding_window: usize,
    /// Maximum layers with sliding window attention.
    pub max_window_layers: usize,
    /// Whether to tie word embeddings to output projection.
    pub tie_word_embeddings: bool,
    /// RoPE theta (base for rotary position embeddings).
    pub rope_theta: f32,
    /// RMSNorm epsilon for numerical stability.
    pub rms_norm_eps: f64,
    /// Whether to use sliding window attention.
    pub use_sliding_window: bool,
    /// Activation function for the feed-forward layer.
    pub hidden_act: Activation,
}

/// Qwen2 self-attention layer.
///
/// Implements grouped query attention (GQA) with RoPE position embeddings.
/// Supports both cached and cache-less forward passes.
#[derive(Debug, Clone)]
pub struct Qwen2Attention {
    /// Linear projection to query.
    q_proj: Linear,
    /// Linear projection to key.
    k_proj: Linear,
    /// Linear projection to value.
    v_proj: Linear,
    /// Linear projection for output.
    o_proj: Linear,
    /// Total number of query heads.
    num_heads: usize,
    /// Number of key-value heads.
    num_kv_heads: usize,
    /// Grouping factor: `num_heads / num_kv_heads`.
    num_kv_groups: usize,
    /// Dimension per head.
    head_dim: usize,
    /// Total hidden dimension.
    hidden_size: usize,
    /// Optional KV cache: `(key_states, value_states)`.
    kv_cache: Option<(Tensor, Tensor)>,
}

impl Qwen2Attention {
    /// Create a new Qwen2 attention layer.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] if weight loading or shape
    /// inference fails.
    pub fn new(cfg: &Qwen2Config, vb: VarBuilder) -> Result<Self> {
        let hidden_size = cfg.hidden_size;
        let num_heads = cfg.num_attention_heads;
        let num_kv_heads = cfg.num_key_value_heads;
        let num_kv_groups = num_heads / num_kv_heads;
        let head_dim = hidden_size / num_heads;
        let q_proj = linear(hidden_size, num_heads * head_dim, vb.pp("q_proj"))?;
        let k_proj = linear(hidden_size, num_kv_heads * head_dim, vb.pp("k_proj"))?;
        let v_proj = linear(hidden_size, num_kv_heads * head_dim, vb.pp("v_proj"))?;
        let o_proj = linear_no_bias(hidden_size, hidden_size, vb.pp("o_proj"))?;
        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            num_heads,
            num_kv_heads,
            num_kv_groups,
            head_dim,
            hidden_size,
            kv_cache: None,
        })
    }

    /// Forward pass with KV cache accumulation.
    ///
    /// Maintains a KV cache across calls. Subsequent calls will concatenate
    /// new key-value states with the cached ones.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] on tensor operation failure.
    pub fn forward(
        &mut self,
        xs: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        let (b_sz, q_len, _) = xs.dims3()?;
        let query_states = self.q_proj.forward(xs)?;
        let key_states = self.k_proj.forward(xs)?;
        let value_states = self.v_proj.forward(xs)?;
        let query_states = query_states
            .reshape((b_sz, q_len, self.num_heads, self.head_dim))?
            .transpose(1, 2)?;
        let key_states = key_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let value_states = value_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let (query_states, key_states) = apply_rotary_pos_emb(&query_states, &key_states, cos, sin, false)?;
        let (key_states, value_states) = match &self.kv_cache {
            None => (key_states, value_states),
            Some((prev_k, prev_v)) => {
                let key_states = Tensor::cat(&[prev_k, &key_states], 2)?;
                let value_states = Tensor::cat(&[prev_v, &value_states], 2)?;
                (key_states, value_states)
            }
        };

        self.kv_cache = Some((key_states.clone(), value_states.clone()));
        let scale = 1f64 / f64::sqrt(self.head_dim as f64);
        let attn_output = eager_attention_forward(
            &query_states,
            &key_states,
            &value_states,
            Some(self.num_kv_groups),
            attention_mask,
            scale,
        )?;
        let attn_output = attn_output.reshape((b_sz, q_len, self.hidden_size))?;
        let attn_output = attn_output.apply(&self.o_proj)?;
        Ok(attn_output)
    }

    /// Forward pass without KV cache.
    ///
    /// All key-value states are computed fresh from the input sequence.
    /// No cache is maintained or consumed.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] on tensor operation failure.
    pub fn forward_no_cache(
        &self,
        xs: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        let (b_sz, q_len, _) = xs.dims3()?;
        let query_states = self.q_proj.forward(xs)?;
        let key_states = self.k_proj.forward(xs)?;
        let value_states = self.v_proj.forward(xs)?;
        let query_states = query_states
            .reshape((b_sz, q_len, self.num_heads, self.head_dim))?
            .transpose(1, 2)?;
        let key_states = key_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let value_states = value_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let (query_states, key_states) = apply_rotary_pos_emb(&query_states, &key_states, cos, sin, false)?;

        let scale = 1f64 / f64::sqrt(self.head_dim as f64);
        let attn_output = eager_attention_forward(
            &query_states,
            &key_states,
            &value_states,
            Some(self.num_kv_groups),
            attention_mask,
            scale,
        )?;
        let attn_output = attn_output.reshape((b_sz, q_len, self.hidden_size))?;
        let attn_output = attn_output.apply(&self.o_proj)?;
        Ok(attn_output)
    }

    /// Clear the KV cache so this layer can process a fresh sequence.
    pub fn clear_kv_cache(&mut self) {
        self.kv_cache = None
    }
}

/// Qwen2 decoder layer (attention + MLP + residuals + layer norms).
///
/// Pre-normalization architecture: layer norm before each sub-layer,
/// followed by residual connections.
#[derive(Debug, Clone)]
pub struct Qwen2DecoderLayer {
    /// Self-attention sub-layer.
    self_attn: Qwen2Attention,
    /// Feed-forward MLP: gate-up-down variant.
    mlp: super::GateUpDownMLP,
    /// RMS normalization before attention.
    input_layernorm: RmsNorm,
    /// RMS normalization before MLP.
    post_attention_layernorm: RmsNorm,
}

impl Qwen2DecoderLayer {
    /// Create a new Qwen2 decoder layer.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] if weight loading or
    /// configuration fails.
    pub fn new(cfg: &Qwen2Config, vb: VarBuilder) -> Result<Self> {
        let self_attn = Qwen2Attention::new(cfg, vb.pp("self_attn"))?;
        let mlp = super::GateUpDownMLP::new(
            vb.pp("mlp"),
            cfg.hidden_size,
            cfg.intermediate_size,
            cfg.hidden_act,
            false,
            None,
            None,
            None,
        )?;
        let input_layernorm = rms_norm(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("input_layernorm"))?;
        let post_attention_layernorm = rms_norm(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("post_attention_layernorm"))?;
        Ok(Self {
            self_attn,
            mlp,
            input_layernorm,
            post_attention_layernorm,
        })
    }

    /// Forward pass with KV cache.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] on tensor operation failure.
    pub fn forward(
        &mut self,
        xs: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        let residual = xs;
        let xs = self.input_layernorm.forward(xs)?;
        let xs = self.self_attn.forward(&xs, cos, sin, attention_mask)?;
        let xs = (xs + residual)?;
        let residual = &xs;
        let xs = xs.apply(&self.post_attention_layernorm)?.apply(&self.mlp)?;
        let xs = (residual + xs)?;
        Ok(xs)
    }

    /// Forward pass without KV cache.
    ///
    /// All positions are processed in a single forward pass.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] on tensor operation failure.
    pub fn forward_no_cache(
        &self,
        xs: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        let residual = xs;
        let xs = self.input_layernorm.forward(xs)?;
        let xs = self.self_attn.forward_no_cache(&xs, cos, sin, attention_mask)?;
        let xs = (xs + residual)?;
        let residual = &xs;
        let xs = xs.apply(&self.post_attention_layernorm)?.apply(&self.mlp)?;
        let xs = (residual + xs)?;
        Ok(xs)
    }

    /// Clear KV caches in this layer.
    pub fn clear_kv_cache(&mut self) {
        self.self_attn.clear_kv_cache()
    }
}

/// Qwen2 language model decoder (stack of layers + output norm + rotary embeddings).
///
/// Implements the full stack of decoder layers with RoPE position embeddings.
#[derive(Debug, Clone)]
pub struct Qwen2Decoder {
    /// Stack of transformer decoder layers.
    layers: Vec<Qwen2DecoderLayer>,
    /// Output RMS normalization.
    norm: RmsNorm,
    /// Rotary position embeddings.
    rotary_emb: RoPE,
}

impl Qwen2Decoder {
    /// Create a new Qwen2 decoder.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] if weight loading or
    /// configuration fails.
    pub fn new(vb: VarBuilder, cfg: &Qwen2Config) -> Result<Self> {
        let mut layers = Vec::with_capacity(cfg.num_hidden_layers);
        let vb_l = vb.pp("layers");
        for layer_idx in 0..cfg.num_hidden_layers {
            let layer = Qwen2DecoderLayer::new(cfg, vb_l.pp(layer_idx))?;
            layers.push(layer)
        }
        let norm = rms_norm(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("norm"))?;
        let head_dim = cfg.hidden_size / cfg.num_attention_heads;
        let rotary_emb = RoPE::new(head_dim, cfg.rope_theta, vb.device())?;
        Ok(Self {
            layers,
            norm,
            rotary_emb,
        })
    }

    /// Forward pass without KV cache.
    ///
    /// Processes the entire sequence at once (prefill). All positions are computed
    /// fresh with no accumulated cache.
    ///
    /// # Arguments
    ///
    /// - `xs`: Input token embeddings, shape `(batch, seq_len, hidden_size)`.
    /// - `attention_mask`: Optional attention mask.
    /// - `seqlen_offset`: Sequence position offset for RoPE embeddings (usually 0
    ///   for prefill).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] on tensor operation failure.
    pub fn forward_no_cache(
        &self,
        xs: &Tensor,
        attention_mask: Option<&Tensor>,
        seqlen_offset: usize,
    ) -> Result<Tensor> {
        let seq_len = xs.dim(1)?;
        let (cos, sin) = self.rotary_emb.forward(seqlen_offset, seq_len, xs.device())?;
        let mut xs = xs.clone();
        for layer in self.layers.iter() {
            xs = layer.forward_no_cache(&xs, &cos, &sin, attention_mask)?;
        }
        let xs = xs.apply(&self.norm)?;
        Ok(xs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qwen2_config_deserialization() {
        let json = r#"{
            "vocab_size": 32000,
            "hidden_size": 256,
            "intermediate_size": 512,
            "num_hidden_layers": 2,
            "num_attention_heads": 4,
            "num_key_value_heads": 2,
            "max_position_embeddings": 2048,
            "sliding_window": 128,
            "max_window_layers": 2,
            "tie_word_embeddings": false,
            "rope_theta": 10000.0,
            "rms_norm_eps": 1e-6,
            "use_sliding_window": false,
            "hidden_act": "silu"
        }"#;
        let cfg: Qwen2Config = serde_json::from_str(json).expect("config deserialize");
        assert_eq!(cfg.vocab_size, 32000);
        assert_eq!(cfg.hidden_size, 256);
        assert_eq!(cfg.num_hidden_layers, 2);
        assert_eq!(cfg.num_attention_heads, 4);
        assert_eq!(cfg.rope_theta, 10000.0);
    }

    #[test]
    fn test_qwen2_config_dimensions() {
        let cfg = Qwen2Config {
            vocab_size: 32000,
            hidden_size: 256,
            intermediate_size: 512,
            num_hidden_layers: 2,
            num_attention_heads: 4,
            num_key_value_heads: 2,
            max_position_embeddings: 2048,
            sliding_window: 128,
            max_window_layers: 2,
            tie_word_embeddings: false,
            rope_theta: 10000.0,
            rms_norm_eps: 1e-6,
            use_sliding_window: false,
            hidden_act: candle_nn::Activation::Silu,
        };

        // Check dimension calculations
        assert_eq!(cfg.hidden_size / cfg.num_attention_heads, 64);
        assert_eq!(cfg.num_attention_heads / cfg.num_key_value_heads, 2);
        assert_eq!(cfg.num_attention_heads * 64, cfg.hidden_size);
    }
}
