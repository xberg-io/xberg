// Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.

//! Subset of `aha::models::common::modules` covering the attention/MLP primitives
// Phase 4 model impls will consume all symbols here; suppress dead-code until then.
#![allow(dead_code)]
//! used by Hunyuan-OCR, DeepSeek-OCR (via Qwen2), and PaddleOCR-VL 1.5.
//!
//! Symbols provided:
//!
//! - [`GateUpDownMLP`] — gate/up/down SwiGLU-style MLP (Qwen2, HunYuan decoder, DeepSeek MoE experts)
//! - [`TwoLinearMLP`] — two-linear MLP with activation (vision transformer blocks)
//! - [`NaiveAttention`] — GQA attention with optional RoPE and KV cache
//! - [`QKVCatAttention`] — fused QKV projection attention with optional RoPE and KV cache
//! - [`NaiveAttnTwoLinearMLPBlock`] — pre-norm transformer block (Hunyuan vision, PaddleOCR-VL vision)
//! - [`NaiveAttnGateUpDownMLPBlock`] — pre-norm transformer block with GateUpDown MLP (PaddleOCR-VL text)
//! - [`eager_attention_forward`] — scaled dot-product attention kernel
//! - [`get_conv2d`] — `Conv2d` builder with full config
//! - [`get_conv1d`] — `Conv1d` builder with full config
//! - [`get_layer_norm`] — `LayerNorm` builder
//! - [`quick_gelu`] — `x * sigmoid(1.702 * x)` activation (CLIP/DeepSeek vision)

use candle_core::{D, IndexOp, Tensor};
use candle_nn::{
    Activation, Conv1d, Conv1dConfig, Conv2d, Conv2dConfig, LayerNorm, LayerNormConfig, Linear, Module, RmsNorm,
    VarBuilder, conv1d, conv1d_no_bias, conv2d, conv2d_no_bias, layer_norm, linear_b, ops::sigmoid, rms_norm,
};

use crate::{
    error::{CandleOcrError, Result},
    vendor::aha::rope::{apply_rotary_pos_emb, apply_rotary_pos_emb_roformer},
};

// ---------------------------------------------------------------------------
// repeat_kv — needed by eager_attention_forward for grouped-query attention.
// This is a pure tensor utility; it lives here because eager_attention_forward
// is the only caller in this vendored subset.
// ---------------------------------------------------------------------------

/// Repeat key/value heads `n_rep` times along the `num_kv_heads` axis (dim 1)
/// to broadcast grouped-query attention.
///
/// Input shape: `(batch, num_kv_heads, seq_len, head_dim)`
/// Output shape: `(batch, num_kv_heads * n_rep, seq_len, head_dim)`
///
/// # Errors
///
/// Propagates any [`candle_core::Error`] from reshape / expand / contiguous.
fn repeat_kv(xs: Tensor, n_rep: usize) -> Result<Tensor> {
    if n_rep == 1 {
        return Ok(xs);
    }
    let (b_sz, n_kv_head, seq_len, head_dim) = xs.dims4().map_err(CandleOcrError::Candle)?;
    xs.unsqueeze(2)
        .map_err(CandleOcrError::Candle)?
        .expand((b_sz, n_kv_head, n_rep, seq_len, head_dim))
        .map_err(CandleOcrError::Candle)?
        .reshape((b_sz, n_kv_head * n_rep, seq_len, head_dim))
        .map_err(CandleOcrError::Candle)
}

// ---------------------------------------------------------------------------
// GateUpDownMLP
// ---------------------------------------------------------------------------

/// SwiGLU / SiLU-gated MLP used in Qwen2, HunYuan, DeepSeek decoder layers.
///
/// Forward: `down_proj(act_fn(gate_proj(x)) * up_proj(x))`
#[derive(Debug, Clone)]
pub struct GateUpDownMLP {
    gate_proj: Linear,
    up_proj: Linear,
    down_proj: Linear,
    act_fn: Activation,
}

impl GateUpDownMLP {
    /// Construct a [`GateUpDownMLP`].
    ///
    /// Projection names default to `gate_proj`, `up_proj`, `down_proj` when
    /// `None` is passed.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if any weight tensor cannot be loaded.
    pub fn new(
        vb: VarBuilder,
        hidden_size: usize,
        intermediate_size: usize,
        act_fn: Activation,
        bias: bool,
        gate_pp_name: Option<&str>,
        up_pp_name: Option<&str>,
        down_pp_name: Option<&str>,
    ) -> Result<Self> {
        let gate_pp = gate_pp_name.unwrap_or("gate_proj");
        let up_pp = up_pp_name.unwrap_or("up_proj");
        let down_pp = down_pp_name.unwrap_or("down_proj");
        let gate_proj =
            linear_b(hidden_size, intermediate_size, bias, vb.pp(gate_pp)).map_err(CandleOcrError::Candle)?;
        let up_proj = linear_b(hidden_size, intermediate_size, bias, vb.pp(up_pp)).map_err(CandleOcrError::Candle)?;
        let down_proj =
            linear_b(intermediate_size, hidden_size, bias, vb.pp(down_pp)).map_err(CandleOcrError::Candle)?;
        Ok(Self {
            gate_proj,
            up_proj,
            down_proj,
            act_fn,
        })
    }
}

impl Module for GateUpDownMLP {
    fn forward(&self, xs: &Tensor) -> candle_core::Result<Tensor> {
        let lhs = xs.apply(&self.gate_proj)?.apply(&self.act_fn)?;
        let rhs = xs.apply(&self.up_proj)?;
        (lhs * rhs)?.apply(&self.down_proj)
    }
}

// ---------------------------------------------------------------------------
// TwoLinearMLP
// ---------------------------------------------------------------------------

/// Two-layer MLP with a single activation in between.
///
/// Used inside `NaiveAttnTwoLinearMLPBlock` vision-encoder blocks (Hunyuan
/// vision encoder, PaddleOCR-VL SigLIP encoder, DeepSeek ViT).
#[derive(Debug)]
pub struct TwoLinearMLP {
    linear1: Linear,
    linear2: Linear,
    act: Activation,
}

impl TwoLinearMLP {
    /// Construct a [`TwoLinearMLP`].
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if weight loading fails.
    pub fn new(
        vb: VarBuilder,
        in_dim: usize,
        middle_dim: usize,
        out_dim: usize,
        act: Activation,
        bias: bool,
        linear1_pp_name: &str,
        linear2_pp_name: &str,
    ) -> Result<Self> {
        let linear1 = linear_b(in_dim, middle_dim, bias, vb.pp(linear1_pp_name)).map_err(CandleOcrError::Candle)?;
        let linear2 = linear_b(middle_dim, out_dim, bias, vb.pp(linear2_pp_name)).map_err(CandleOcrError::Candle)?;
        Ok(Self { linear1, linear2, act })
    }

    /// Run the two-linear forward pass.
    ///
    /// # Errors
    ///
    /// Propagates [`CandleOcrError`] from any candle operation.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        xs.apply(&self.linear1)
            .map_err(CandleOcrError::Candle)?
            .apply(&self.act)
            .map_err(CandleOcrError::Candle)?
            .apply(&self.linear2)
            .map_err(CandleOcrError::Candle)
    }
}

// ---------------------------------------------------------------------------
// NaiveAttention
// ---------------------------------------------------------------------------

/// Multi-head / grouped-query attention with optional RoPE and incremental KV cache.
///
/// Used by DeepSeek-OCR decoder and DeepSeek `Qwen2Decoder2Encoder`.
#[derive(Debug, Clone)]
pub struct NaiveAttention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    o_proj: Linear,
    num_heads: usize,
    num_kv_heads: usize,
    num_kv_groups: usize,
    head_dim: usize,
    middle_size: usize,
    kv_cache: Option<(Tensor, Tensor)>,
}

impl NaiveAttention {
    /// Build a [`NaiveAttention`] layer.
    ///
    /// Projection names default to `q_proj`, `k_proj`, `v_proj`, `o_proj`.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] when a weight tensor cannot be loaded.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vb: VarBuilder,
        hidden_size: usize,
        num_attention_heads: usize,
        num_key_value_heads: usize,
        head_dim: Option<usize>,
        bias: bool,
        q_proj_pp_name: Option<&str>,
        k_proj_pp_name: Option<&str>,
        v_proj_pp_name: Option<&str>,
        o_proj_pp_name: Option<&str>,
    ) -> Result<Self> {
        let num_kv_groups = num_attention_heads / num_key_value_heads;
        let head_dim = head_dim.unwrap_or(hidden_size / num_attention_heads);
        let q_name = q_proj_pp_name.unwrap_or("q_proj");
        let k_name = k_proj_pp_name.unwrap_or("k_proj");
        let v_name = v_proj_pp_name.unwrap_or("v_proj");
        let o_name = o_proj_pp_name.unwrap_or("o_proj");
        let q_proj = linear_b(hidden_size, num_attention_heads * head_dim, bias, vb.pp(q_name))
            .map_err(CandleOcrError::Candle)?;
        let k_proj = linear_b(hidden_size, num_key_value_heads * head_dim, bias, vb.pp(k_name))
            .map_err(CandleOcrError::Candle)?;
        let v_proj = linear_b(hidden_size, num_key_value_heads * head_dim, bias, vb.pp(v_name))
            .map_err(CandleOcrError::Candle)?;
        let o_proj = linear_b(num_attention_heads * head_dim, hidden_size, bias, vb.pp(o_name))
            .map_err(CandleOcrError::Candle)?;
        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            num_heads: num_attention_heads,
            num_kv_heads: num_key_value_heads,
            num_kv_groups,
            head_dim,
            middle_size: num_attention_heads * head_dim,
            kv_cache: None,
        })
    }

    /// Stateless forward pass (no KV cache).
    ///
    /// # Errors
    ///
    /// Propagates [`CandleOcrError`] from projection, reshape, or attention ops.
    pub fn forward(
        &self,
        xs: &Tensor,
        cos: Option<&Tensor>,
        sin: Option<&Tensor>,
        attention_mask: Option<&Tensor>,
        tof32: bool,
    ) -> Result<Tensor> {
        let (b_sz, q_len, _) = xs.dims3().map_err(CandleOcrError::Candle)?;
        let query_states = self.q_proj.forward(xs).map_err(CandleOcrError::Candle)?;
        let key_states = self.k_proj.forward(xs).map_err(CandleOcrError::Candle)?;
        let value_states = self.v_proj.forward(xs).map_err(CandleOcrError::Candle)?;
        let query_states = query_states
            .reshape((b_sz, q_len, self.num_heads, self.head_dim))
            .map_err(CandleOcrError::Candle)?
            .transpose(1, 2)
            .map_err(CandleOcrError::Candle)?;
        let key_states = key_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))
            .map_err(CandleOcrError::Candle)?
            .transpose(1, 2)
            .map_err(CandleOcrError::Candle)?;
        let value_states = value_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))
            .map_err(CandleOcrError::Candle)?
            .transpose(1, 2)
            .map_err(CandleOcrError::Candle)?;
        let (query_states, key_states) = if let (Some(cos), Some(sin)) = (cos, sin) {
            apply_rotary_pos_emb(&query_states, &key_states, cos, sin, tof32)?
        } else {
            (query_states, key_states)
        };
        let scale = 1f64 / f64::sqrt(self.head_dim as f64);
        let attn_output = eager_attention_forward(
            &query_states,
            &key_states,
            &value_states,
            Some(self.num_kv_groups),
            attention_mask,
            scale,
        )?;
        let attn_output = attn_output
            .reshape((b_sz, q_len, self.middle_size))
            .map_err(CandleOcrError::Candle)?;
        attn_output.apply(&self.o_proj).map_err(CandleOcrError::Candle)
    }

    /// Stateful forward pass — appends to the internal KV cache.
    ///
    /// # Errors
    ///
    /// Propagates [`CandleOcrError`] from projection or attention ops.
    pub fn forward_with_cache(
        &mut self,
        xs: &Tensor,
        cos: Option<&Tensor>,
        sin: Option<&Tensor>,
        attention_mask: Option<&Tensor>,
        tof32: bool,
    ) -> Result<Tensor> {
        let (b_sz, q_len, _) = xs.dims3().map_err(CandleOcrError::Candle)?;
        let query_states = self.q_proj.forward(xs).map_err(CandleOcrError::Candle)?;
        let key_states = self.k_proj.forward(xs).map_err(CandleOcrError::Candle)?;
        let value_states = self.v_proj.forward(xs).map_err(CandleOcrError::Candle)?;
        let query_states = query_states
            .reshape((b_sz, q_len, self.num_heads, self.head_dim))
            .map_err(CandleOcrError::Candle)?
            .transpose(1, 2)
            .map_err(CandleOcrError::Candle)?;
        let key_states = key_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))
            .map_err(CandleOcrError::Candle)?
            .transpose(1, 2)
            .map_err(CandleOcrError::Candle)?;
        let value_states = value_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))
            .map_err(CandleOcrError::Candle)?
            .transpose(1, 2)
            .map_err(CandleOcrError::Candle)?;
        let (query_states, key_states) = if let (Some(cos), Some(sin)) = (cos, sin) {
            apply_rotary_pos_emb(&query_states, &key_states, cos, sin, tof32)?
        } else {
            (query_states, key_states)
        };
        let (key_states, value_states) = match &self.kv_cache {
            None => (key_states, value_states),
            Some((prev_k, prev_v)) => {
                let key_states = Tensor::cat(&[prev_k, &key_states], 2).map_err(CandleOcrError::Candle)?;
                let value_states = Tensor::cat(&[prev_v, &value_states], 2).map_err(CandleOcrError::Candle)?;
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
        let attn_output = attn_output
            .reshape((b_sz, q_len, self.middle_size))
            .map_err(CandleOcrError::Candle)?;
        attn_output.apply(&self.o_proj).map_err(CandleOcrError::Candle)
    }

    /// Clear the accumulated KV cache.
    pub fn clear_kv_cache(&mut self) {
        self.kv_cache = None;
    }
}

// ---------------------------------------------------------------------------
// QKVCatAttention
// ---------------------------------------------------------------------------

/// Fused QKV-projection attention used in DeepSeek-OCR's CLIP-style vision tower.
///
/// A single `qkv_proj` linear maps hidden → 3 * num_heads * head_dim,
/// then splits into Q, K, V.
#[derive(Debug, Clone)]
pub struct QKVCatAttention {
    qkv_proj: Linear,
    o_proj: Linear,
    num_heads: usize,
    scaling: f64,
    kv_cache: Option<(Tensor, Tensor)>,
}

impl QKVCatAttention {
    /// Build a [`QKVCatAttention`] layer.
    ///
    /// Projection names default to `qkv_proj` and `out_proj`.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] when a weight tensor cannot be loaded.
    pub fn new(
        vb: VarBuilder,
        hidden_size: usize,
        num_attention_heads: usize,
        head_dim: Option<usize>,
        bias: bool,
        qkv_proj_pp_name: Option<&str>,
        o_proj_pp_name: Option<&str>,
    ) -> Result<Self> {
        let head_dim = head_dim.unwrap_or(hidden_size / num_attention_heads);
        let qkv_name = qkv_proj_pp_name.unwrap_or("qkv_proj");
        let o_name = o_proj_pp_name.unwrap_or("out_proj");
        let qkv_proj = linear_b(hidden_size, 3 * num_attention_heads * head_dim, bias, vb.pp(qkv_name))
            .map_err(CandleOcrError::Candle)?;
        let o_proj = linear_b(num_attention_heads * head_dim, hidden_size, bias, vb.pp(o_name))
            .map_err(CandleOcrError::Candle)?;
        let scaling = 1f64 / f64::sqrt(head_dim as f64);
        Ok(Self {
            qkv_proj,
            o_proj,
            num_heads: num_attention_heads,
            scaling,
            kv_cache: None,
        })
    }

    /// Stateless forward pass.
    ///
    /// `use_roformer` selects between standard RoPE and RoFormer-style RoPE.
    ///
    /// # Errors
    ///
    /// Propagates [`CandleOcrError`] from projection or attention ops.
    pub fn forward(
        &self,
        xs: &Tensor,
        cos: Option<&Tensor>,
        sin: Option<&Tensor>,
        attention_mask: Option<&Tensor>,
        tof32: bool,
        use_roformer: bool,
    ) -> Result<Tensor> {
        let (b, q_len, _) = xs.dims3().map_err(CandleOcrError::Candle)?;
        // Fuse QKV: (3, B, n_head, seq_len, head_dim)
        let qkv = self
            .qkv_proj
            .forward(xs)
            .map_err(CandleOcrError::Candle)?
            .reshape((b, q_len, 3, self.num_heads, ()))
            .map_err(CandleOcrError::Candle)?
            .permute((2, 0, 3, 1, 4))
            .map_err(CandleOcrError::Candle)?
            .contiguous()
            .map_err(CandleOcrError::Candle)?;
        let query_states = qkv
            .i(0)
            .map_err(CandleOcrError::Candle)?
            .contiguous()
            .map_err(CandleOcrError::Candle)?;
        let key_states = qkv
            .i(1)
            .map_err(CandleOcrError::Candle)?
            .contiguous()
            .map_err(CandleOcrError::Candle)?;
        let value_states = qkv
            .i(2)
            .map_err(CandleOcrError::Candle)?
            .contiguous()
            .map_err(CandleOcrError::Candle)?;
        let (query_states, key_states) = if let (Some(cos), Some(sin)) = (cos, sin) {
            if use_roformer {
                apply_rotary_pos_emb_roformer(&query_states, &key_states, cos, sin)?
            } else {
                apply_rotary_pos_emb(&query_states, &key_states, cos, sin, tof32)?
            }
        } else {
            (query_states, key_states)
        };
        let attn_output = eager_attention_forward(
            &query_states,
            &key_states,
            &value_states,
            None,
            attention_mask,
            self.scaling,
        )?;
        let attn_output = attn_output.reshape((b, q_len, ())).map_err(CandleOcrError::Candle)?;
        attn_output.apply(&self.o_proj).map_err(CandleOcrError::Candle)
    }

    /// Stateful forward pass — appends to the internal KV cache.
    ///
    /// # Errors
    ///
    /// Propagates [`CandleOcrError`] from projection or attention ops.
    pub fn forward_with_cache(
        &mut self,
        xs: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
        tof32: bool,
        use_roformer: bool,
    ) -> Result<Tensor> {
        let (b, q_len, _) = xs.dims3().map_err(CandleOcrError::Candle)?;
        let qkv = self
            .qkv_proj
            .forward(xs)
            .map_err(CandleOcrError::Candle)?
            .reshape((b, q_len, 3, self.num_heads, ()))
            .map_err(CandleOcrError::Candle)?
            .permute((2, 0, 3, 1, 4))
            .map_err(CandleOcrError::Candle)?
            .contiguous()
            .map_err(CandleOcrError::Candle)?;
        let query_states = qkv
            .i(0)
            .map_err(CandleOcrError::Candle)?
            .contiguous()
            .map_err(CandleOcrError::Candle)?;
        let key_states = qkv
            .i(1)
            .map_err(CandleOcrError::Candle)?
            .contiguous()
            .map_err(CandleOcrError::Candle)?;
        let value_states = qkv
            .i(2)
            .map_err(CandleOcrError::Candle)?
            .contiguous()
            .map_err(CandleOcrError::Candle)?;
        let (query_states, key_states) = if use_roformer {
            apply_rotary_pos_emb_roformer(&query_states, &key_states, cos, sin)?
        } else {
            apply_rotary_pos_emb(&query_states, &key_states, cos, sin, tof32)?
        };
        let (key_states, value_states) = match &self.kv_cache {
            None => (key_states, value_states),
            Some((prev_k, prev_v)) => {
                let key_states = Tensor::cat(&[prev_k, &key_states], 2).map_err(CandleOcrError::Candle)?;
                let value_states = Tensor::cat(&[prev_v, &value_states], 2).map_err(CandleOcrError::Candle)?;
                (key_states, value_states)
            }
        };
        self.kv_cache = Some((key_states.clone(), value_states.clone()));
        let attn_output = eager_attention_forward(
            &query_states,
            &key_states,
            &value_states,
            None,
            attention_mask,
            self.scaling,
        )?;
        let attn_output = attn_output.reshape((b, q_len, ())).map_err(CandleOcrError::Candle)?;
        attn_output.apply(&self.o_proj).map_err(CandleOcrError::Candle)
    }

    /// Clear the accumulated KV cache.
    pub fn clear_kv_cache(&mut self) {
        self.kv_cache = None;
    }
}

// ---------------------------------------------------------------------------
// NaiveAttnTwoLinearMLPBlock
// ---------------------------------------------------------------------------

/// Pre-norm transformer block combining [`NaiveAttention`] with [`TwoLinearMLP`].
///
/// Used by Hunyuan-OCR vision encoder and PaddleOCR-VL SigLIP encoder.
pub struct NaiveAttnTwoLinearMLPBlock {
    self_attn: NaiveAttention,
    mlp: TwoLinearMLP,
    input_layernorm: LayerNorm,
    post_attention_layernorm: LayerNorm,
}

impl NaiveAttnTwoLinearMLPBlock {
    /// Build a [`NaiveAttnTwoLinearMLPBlock`].
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] when any sub-module weight fails to load.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vb: VarBuilder,
        hidden_size: usize,
        num_attention_heads: usize,
        num_key_value_heads: Option<usize>,
        head_dim: Option<usize>,
        attn_bias: bool,
        attn_pp_name: &str,
        o_proj_pp_name: Option<&str>,
        intermediate_size: usize,
        hidden_act: Activation,
        mlp_bias: bool,
        mlp_pp_name: &str,
        linear1_pp_name: &str,
        linear2_pp_name: &str,
        norm_eps: f64,
        input_norm_pp_name: &str,
        post_norm_pp_name: &str,
    ) -> Result<Self> {
        let num_kv_heads = num_key_value_heads.unwrap_or(num_attention_heads);
        let self_attn = NaiveAttention::new(
            vb.pp(attn_pp_name),
            hidden_size,
            num_attention_heads,
            num_kv_heads,
            head_dim,
            attn_bias,
            None,
            None,
            None,
            o_proj_pp_name,
        )?;
        let mlp = TwoLinearMLP::new(
            vb.pp(mlp_pp_name),
            hidden_size,
            intermediate_size,
            hidden_size,
            hidden_act,
            mlp_bias,
            linear1_pp_name,
            linear2_pp_name,
        )?;
        let input_layernorm = get_layer_norm(vb.pp(input_norm_pp_name), norm_eps, hidden_size, true)?;
        let post_attention_layernorm = get_layer_norm(vb.pp(post_norm_pp_name), norm_eps, hidden_size, true)?;
        Ok(Self {
            self_attn,
            mlp,
            input_layernorm,
            post_attention_layernorm,
        })
    }

    /// Run pre-norm attention + pre-norm MLP with residuals.
    ///
    /// # Errors
    ///
    /// Propagates [`CandleOcrError`] from any sub-module.
    pub fn forward(
        &self,
        xs: &Tensor,
        cos: Option<&Tensor>,
        sin: Option<&Tensor>,
        attention_mask: Option<&Tensor>,
        tof32: bool,
    ) -> Result<Tensor> {
        let residual = xs.clone();
        let xs = self.input_layernorm.forward(xs).map_err(CandleOcrError::Candle)?;
        let xs = self.self_attn.forward(&xs, cos, sin, attention_mask, tof32)?;
        let residual = residual.add(&xs).map_err(CandleOcrError::Candle)?;
        let xs = self
            .post_attention_layernorm
            .forward(&residual)
            .map_err(CandleOcrError::Candle)?;
        let xs = self.mlp.forward(&xs)?;
        residual.add(&xs).map_err(CandleOcrError::Candle)
    }
}

// ---------------------------------------------------------------------------
// NaiveAttnGateUpDownMLPBlock
// ---------------------------------------------------------------------------

/// Pre-norm transformer block combining [`NaiveAttention`] with [`GateUpDownMLP`]
/// and RmsNorm.
///
/// Used by PaddleOCR-VL's text decoder (`Ernie4_5Model` layers).
pub struct NaiveAttnGateUpDownMLPBlock {
    self_attn: NaiveAttention,
    mlp: GateUpDownMLP,
    input_layernorm: RmsNorm,
    post_attention_layernorm: RmsNorm,
}

impl NaiveAttnGateUpDownMLPBlock {
    /// Build a [`NaiveAttnGateUpDownMLPBlock`].
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] when any sub-module weight fails to load.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vb: VarBuilder,
        hidden_size: usize,
        num_attention_heads: usize,
        num_key_value_heads: Option<usize>,
        head_dim: Option<usize>,
        attn_bias: bool,
        attn_pp_name: &str,
        o_proj_pp_name: Option<&str>,
        intermediate_size: usize,
        hidden_act: Activation,
        mlp_bias: bool,
        mlp_pp_name: &str,
        norm_eps: f64,
        input_norm_pp_name: &str,
        post_norm_pp_name: &str,
    ) -> Result<Self> {
        let num_kv_heads = num_key_value_heads.unwrap_or(num_attention_heads);
        let self_attn = NaiveAttention::new(
            vb.pp(attn_pp_name),
            hidden_size,
            num_attention_heads,
            num_kv_heads,
            head_dim,
            attn_bias,
            None,
            None,
            None,
            o_proj_pp_name,
        )?;
        let mlp = GateUpDownMLP::new(
            vb.pp(mlp_pp_name),
            hidden_size,
            intermediate_size,
            hidden_act,
            mlp_bias,
            None,
            None,
            None,
        )?;
        let input_layernorm =
            rms_norm(hidden_size, norm_eps, vb.pp(input_norm_pp_name)).map_err(CandleOcrError::Candle)?;
        let post_attention_layernorm =
            rms_norm(hidden_size, norm_eps, vb.pp(post_norm_pp_name)).map_err(CandleOcrError::Candle)?;
        Ok(Self {
            self_attn,
            mlp,
            input_layernorm,
            post_attention_layernorm,
        })
    }

    /// Run pre-norm attention + pre-norm MLP with residuals.
    ///
    /// Requires pre-computed `cos`/`sin` RoPE tensors.
    ///
    /// # Errors
    ///
    /// Propagates [`CandleOcrError`] from any sub-module.
    pub fn forward(
        &mut self,
        xs: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        let residual = xs.clone();
        let xs = self.input_layernorm.forward(xs).map_err(CandleOcrError::Candle)?;
        let xs = self
            .self_attn
            .forward_with_cache(&xs, Some(cos), Some(sin), attention_mask, false)?;
        let residual = residual.add(&xs).map_err(CandleOcrError::Candle)?;
        let xs = self
            .post_attention_layernorm
            .forward(&residual)
            .map_err(CandleOcrError::Candle)?;
        let xs = self.mlp.forward(&xs).map_err(CandleOcrError::Candle)?;
        residual.add(&xs).map_err(CandleOcrError::Candle)
    }

    /// Clear the accumulated KV cache in the self-attention layer.
    pub fn clear_kv_cache(&mut self) {
        self.self_attn.clear_kv_cache();
    }
}

// ---------------------------------------------------------------------------
// eager_attention_forward
// ---------------------------------------------------------------------------

/// Scaled dot-product attention kernel.
///
/// Input shapes:
/// - `query_states`: `(batch, num_heads, seq_len, head_dim)`
/// - `key_states` / `value_states`: `(batch, num_kv_heads, seq_len, head_dim)`
///
/// Output shape: `(batch, seq_len, num_heads, head_dim)` — note the transpose
/// back so callers can `reshape` to `(batch, seq_len, hidden_size)`.
///
/// When `num_key_value_groups` is `Some(g)` the KV tensors are repeated `g`
/// times along the head axis to align with the query head count.
///
/// # Errors
///
/// Propagates any [`CandleOcrError`] from candle tensor operations.
pub fn eager_attention_forward(
    query_states: &Tensor,
    key_states: &Tensor,
    value_states: &Tensor,
    num_key_value_groups: Option<usize>,
    attention_mask: Option<&Tensor>,
    scaling: f64,
) -> Result<Tensor> {
    let key_states = match num_key_value_groups {
        Some(g) => repeat_kv(key_states.clone(), g)?,
        None => key_states.clone(),
    };
    let value_states = match num_key_value_groups {
        Some(g) => repeat_kv(value_states.clone(), g)?,
        None => value_states.clone(),
    };
    let query_states = query_states.contiguous().map_err(CandleOcrError::Candle)?;
    let key_states = key_states.contiguous().map_err(CandleOcrError::Candle)?;
    let value_states = value_states.contiguous().map_err(CandleOcrError::Candle)?;

    let attn_weights = query_states
        .matmul(
            &key_states
                .transpose(D::Minus2, D::Minus1)
                .map_err(CandleOcrError::Candle)?
                .contiguous()
                .map_err(CandleOcrError::Candle)?,
        )
        .map_err(CandleOcrError::Candle)?;
    let attn_weights = (attn_weights * scaling).map_err(CandleOcrError::Candle)?;
    let attn_weights = match attention_mask {
        None => attn_weights,
        Some(mask) => attn_weights
            .broadcast_add(&mask.to_dtype(attn_weights.dtype()).map_err(CandleOcrError::Candle)?)
            .map_err(CandleOcrError::Candle)?,
    };
    let attn_weights = candle_nn::ops::softmax_last_dim(&attn_weights)
        .map_err(CandleOcrError::Candle)?
        .contiguous()
        .map_err(CandleOcrError::Candle)?;
    let attn_output = attn_weights.matmul(&value_states).map_err(CandleOcrError::Candle)?;
    // (b, n_head, seq_len, dim) -> (b, seq_len, n_head, dim)
    attn_output
        .transpose(1, 2)
        .map_err(CandleOcrError::Candle)?
        .contiguous()
        .map_err(CandleOcrError::Candle)
}

// ---------------------------------------------------------------------------
// Utility builders
// ---------------------------------------------------------------------------

/// Build a [`Conv2d`] layer from explicit configuration parameters.
///
/// Passes `bias = true` through `conv2d`; `bias = false` through `conv2d_no_bias`.
///
/// # Errors
///
/// Returns [`CandleOcrError`] if weight loading fails.
#[allow(clippy::too_many_arguments)]
pub fn get_conv2d(
    vb: VarBuilder,
    in_c: usize,
    out_c: usize,
    kernel_size: usize,
    padding: usize,
    stride: usize,
    dilation: usize,
    groups: usize,
    bias: bool,
) -> Result<Conv2d> {
    let cfg = Conv2dConfig {
        padding,
        stride,
        dilation,
        groups,
        cudnn_fwd_algo: None,
    };
    let layer = if bias {
        conv2d(in_c, out_c, kernel_size, cfg, vb).map_err(CandleOcrError::Candle)?
    } else {
        conv2d_no_bias(in_c, out_c, kernel_size, cfg, vb).map_err(CandleOcrError::Candle)?
    };
    Ok(layer)
}

/// Build a [`Conv1d`] layer from explicit configuration parameters.
///
/// # Errors
///
/// Returns [`CandleOcrError`] if weight loading fails.
#[allow(clippy::too_many_arguments)]
pub fn get_conv1d(
    vb: VarBuilder,
    in_c: usize,
    out_c: usize,
    kernel_size: usize,
    padding: usize,
    stride: usize,
    dilation: usize,
    groups: usize,
    bias: bool,
) -> Result<Conv1d> {
    let cfg = Conv1dConfig {
        padding,
        stride,
        dilation,
        groups,
        cudnn_fwd_algo: None,
    };
    let layer = if bias {
        conv1d(in_c, out_c, kernel_size, cfg, vb).map_err(CandleOcrError::Candle)?
    } else {
        conv1d_no_bias(in_c, out_c, kernel_size, cfg, vb).map_err(CandleOcrError::Candle)?
    };
    Ok(layer)
}

/// Build a [`LayerNorm`] layer.
///
/// `affine = true` loads learnable weight and bias; `affine = false` uses
/// constant weight=1, bias=0.
///
/// # Errors
///
/// Returns [`CandleOcrError`] if the weight/bias tensors cannot be loaded.
pub fn get_layer_norm(vb: VarBuilder, eps: f64, dim: usize, affine: bool) -> Result<LayerNorm> {
    let cfg = LayerNormConfig {
        eps,
        remove_mean: true,
        affine,
    };
    layer_norm(dim, cfg, vb).map_err(CandleOcrError::Candle)
}

// ---------------------------------------------------------------------------
// Activation helpers
// ---------------------------------------------------------------------------

/// Quick GELU: `x * sigmoid(1.702 * x)`.
///
/// Used in CLIP-style vision towers inside DeepSeek-OCR.
///
/// # Errors
///
/// Propagates any [`CandleOcrError`] from candle tensor operations.
pub fn quick_gelu(xs: &Tensor) -> Result<Tensor> {
    let x = xs.affine(1.702, 0.0).map_err(CandleOcrError::Candle)?;
    let x = sigmoid(&x).map_err(CandleOcrError::Candle)?;
    xs.mul(&x).map_err(CandleOcrError::Candle)
}
