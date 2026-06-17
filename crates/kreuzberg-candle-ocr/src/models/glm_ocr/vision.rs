//! Glm4v vision encoder (`Glm4vVisionModel`) for GLM-OCR.
//!
//! Consumes a `(B, C, H, W)` pixel tensor and emits `(B, num_image_tokens, hidden_size)`
//! patch embeddings. Architecture matches the upstream `Glm4vVisionModel`:
//!
//! - **Patch embedding** — `Conv3d` over `(temporal_patch_size, patch_size, patch_size)`.
//!   For still images the temporal axis is degenerate (input replicated to length 2 and
//!   collapsed to 1 by stride). We emulate Conv3d in candle 0.10 (no Conv3d op) by summing
//!   the temporal kernel slices at load time and applying a regular Conv2d. This is exact
//!   when the same image fills both temporal slots — which is the still-image case used
//!   by GLM-OCR.
//! - **Transformer blocks** — `norm1 (RMSNorm) -> attn -> residual -> norm2 (RMSNorm) -> mlp -> residual`.
//! - **Attention** — fused `qkv` projection, per-head `q_norm`/`k_norm` RMSNorm over
//!   `head_dim`, 2-D rotary embeddings on Q/K, scaled dot-product attention, output `proj`.
//! - **MLP** — SwiGLU (`down_proj(silu(gate_proj(x)) * up_proj(x))`).
//! - **Final norm** — `post_layernorm` (RMSNorm).
//!
//! No learned positional embedding is used inside this encoder. Positions are conveyed
//! entirely by 2-D RoPE inside attention.
//!
//! Upstream reference: <https://huggingface.co/zai-org/GLM-OCR>
//! Modeling code: HF `transformers.models.glm4v.modeling_glm4v.Glm4vVisionModel`.

use serde::{Deserialize, Serialize};

/// Glm4v vision encoder parameters, deserialised from the upstream
/// `config.json` `vision_config` block. JSON keys use `num_heads` and `depth`
/// (matching the upstream `Glm4vVisionConfig`), not the more typical
/// `num_attention_heads`/`num_hidden_layers`.
///
/// VarBuilder root: `model.visual.`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VisionConfig {
    pub hidden_size: usize,
    /// Number of attention heads. Upstream JSON key: `num_heads`.
    #[serde(rename = "num_heads")]
    pub num_attention_heads: usize,
    /// Number of transformer blocks. Upstream JSON key: `depth`.
    #[serde(rename = "depth")]
    pub num_hidden_layers: usize,
    pub patch_size: usize,
    pub intermediate_size: usize,
    pub image_size: usize,
    /// Output hidden size after the merger (text-decoder hidden size).
    #[serde(default = "default_out_hidden_size")]
    pub out_hidden_size: usize,
    /// Spatial 2×2 merge factor between vision tokens and decoder tokens.
    #[serde(default = "default_spatial_merge_size")]
    pub spatial_merge_size: usize,
    /// Temporal patch size (Glm4v uses Conv3d patch embedding with this depth).
    #[serde(default = "default_temporal_patch_size")]
    pub temporal_patch_size: usize,
    /// Number of image channels (defaulted; not in upstream JSON).
    #[serde(default = "default_num_channels")]
    pub num_channels: usize,
    /// RMS normalization epsilon.
    pub rms_norm_eps: f64,
    /// Hidden activation function.
    #[serde(default = "default_hidden_act")]
    pub hidden_act: String,
    /// Whether attention includes bias.
    #[serde(default = "default_attention_bias")]
    pub attention_bias: bool,
}

fn default_hidden_act() -> String {
    "silu".to_string()
}

fn default_attention_bias() -> bool {
    true
}

fn default_out_hidden_size() -> usize {
    1536
}

fn default_spatial_merge_size() -> usize {
    2
}

fn default_temporal_patch_size() -> usize {
    2
}

fn default_num_channels() -> usize {
    3
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            hidden_size: 1024,
            num_attention_heads: 16,
            num_hidden_layers: 24,
            patch_size: 14,
            intermediate_size: 4096,
            image_size: 336,
            out_hidden_size: default_out_hidden_size(),
            spatial_merge_size: default_spatial_merge_size(),
            temporal_patch_size: default_temporal_patch_size(),
            num_channels: default_num_channels(),
            rms_norm_eps: 1e-5,
            hidden_act: default_hidden_act(),
            attention_bias: default_attention_bias(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use candle_core::{D, DType, Device, IndexOp, Module, Result as CandleResult, Tensor};
    use candle_nn::VarBuilder;

    use super::VisionConfig;
    use crate::CandleOcrError;
    use crate::error::Result;

    const ROPE_THETA: f64 = 10000.0;

    /// RMSNorm: `x / sqrt(mean(x^2) + eps) * weight`.
    ///
    /// Matches upstream `Glm4vRMSNorm` (weight-only, no bias). The variance reduction is
    /// computed in f32 for numerical stability and cast back to the input dtype before
    /// scaling — same as the reference implementation.
    #[derive(Debug, Clone)]
    struct RmsNorm {
        weight: Tensor,
        eps: f64,
    }

    impl RmsNorm {
        fn new(size: usize, eps: f64, vb: VarBuilder) -> CandleResult<Self> {
            let weight = vb.get(size, "weight")?;
            Ok(Self { weight, eps })
        }

        fn forward(&self, xs: &Tensor) -> CandleResult<Tensor> {
            let input_dtype = xs.dtype();
            let xs_f32 = xs.to_dtype(DType::F32)?;
            let variance = xs_f32.sqr()?.mean_keepdim(D::Minus1)?;
            let normed = xs_f32.broadcast_div(&(variance + self.eps)?.sqrt()?)?;
            normed.to_dtype(input_dtype)?.broadcast_mul(&self.weight)
        }
    }

    /// 2-D rotary position embeddings for vision attention.
    ///
    /// Matches the upstream `Glm4vVisionRotaryEmbedding` + per-patch position assembly.
    /// The frequency table has length `head_dim/4`. For each token the row and column
    /// frequencies are concatenated (row first) to give a `(seq, head_dim/2)` table,
    /// which is then duplicated along the last axis to span `head_dim`. The resulting
    /// layout is `[row_freqs, col_freqs, row_freqs, col_freqs]` (four equal quarters),
    /// so the standard `rotate_half` splits cleanly into `[-x_back, x_front]` and
    /// pairs each element at index `i` with the element at index `i + head_dim/2` —
    /// both encoding the same spatial axis. This is REQUIRED for correctness: the
    /// natural-looking `[row_dup, col_dup]` layout (each axis filling one half of
    /// `head_dim`) pairs (row, col) features under `rotate_half` and scrambles the
    /// signal.
    ///
    /// Constructed once and re-used for every grid shape; the cos/sin tables are
    /// computed on demand per `(grid_h, grid_w)`.
    struct Vision2dRope {
        /// inv_freq for one axis (length `head_dim / 4`).
        inv_freq: Tensor,
        /// half of `head_dim` — the size of one axis's frequency vector after
        /// concatenating row+col freqs.
        half_dim: usize,
        device: Device,
    }

    impl Vision2dRope {
        fn new(head_dim: usize, device: Device) -> CandleResult<Self> {
            // Upstream uses head_dim // 2 as the per-axis dim (see
            // Glm4vVisionModel.__init__: self.rotary_pos_emb = Glm4vVisionRotaryEmbedding(head_dim // 2)).
            // The inv_freq table itself has length (head_dim // 2) / 2 = head_dim / 4.
            let half_dim = head_dim / 2;
            let freq_dim = half_dim / 2;
            let inv_freq: Vec<f32> = (0..freq_dim)
                .map(|i| (1.0 / ROPE_THETA.powf((i as f64 * 2.0) / half_dim as f64)) as f32)
                .collect();
            let inv_freq = Tensor::from_vec(inv_freq, freq_dim, &device)?;
            Ok(Self {
                inv_freq,
                half_dim,
                device,
            })
        }

        /// Build the `(seq_len, head_dim)` cos/sin tables for a grid of shape
        /// `(grid_h, grid_w)`. Token order is row-major (matching the patch embed flatten).
        fn cos_sin(&self, grid_h: usize, grid_w: usize, dtype: DType) -> CandleResult<(Tensor, Tensor)> {
            let positions_h: Vec<f32> = (0..grid_h)
                .flat_map(|h| std::iter::repeat_n(h as f32, grid_w))
                .collect();
            let positions_w: Vec<f32> = (0..grid_h).flat_map(|_| (0..grid_w).map(|w| w as f32)).collect();
            let seq_len = grid_h * grid_w;

            // (seq_len, 1) @ (1, freq_dim) -> (seq_len, freq_dim)
            let pos_h = Tensor::from_vec(positions_h, (seq_len, 1), &self.device)?.to_dtype(DType::F32)?;
            let pos_w = Tensor::from_vec(positions_w, (seq_len, 1), &self.device)?.to_dtype(DType::F32)?;
            let inv_freq = self.inv_freq.reshape((1, ()))?;
            let freqs_h = pos_h.broadcast_mul(&inv_freq)?;
            let freqs_w = pos_w.broadcast_mul(&inv_freq)?;

            // Concatenate row freqs and col freqs into a (seq_len, head_dim/2) tensor,
            // then duplicate along the last axis to (seq_len, head_dim). This produces
            // the [row, col, row, col] quarter layout that pairs index i with index
            // i + head_dim/2 under rotate_half within the SAME spatial axis.
            let freqs = Tensor::cat(&[&freqs_h, &freqs_w], D::Minus1)?;
            let emb = Tensor::cat(&[&freqs, &freqs], D::Minus1)?;

            let cos = emb.cos()?.to_dtype(dtype)?;
            let sin = emb.sin()?.to_dtype(dtype)?;
            Ok((cos, sin))
        }

        /// Apply 2-D RoPE to `q` and `k`, both shaped `(B, H, N, head_dim)`.
        fn apply(&self, q: &Tensor, k: &Tensor, grid_h: usize, grid_w: usize) -> CandleResult<(Tensor, Tensor)> {
            debug_assert_eq!(q.dim(D::Minus1)?, 2 * self.half_dim);
            let (cos, sin) = self.cos_sin(grid_h, grid_w, q.dtype())?;
            // cos/sin: (N, head_dim) -> broadcast to (B, H, N, head_dim) via two unsqueezes.
            let cos = cos.unsqueeze(0)?.unsqueeze(0)?;
            let sin = sin.unsqueeze(0)?.unsqueeze(0)?;
            let q_rot = q.broadcast_mul(&cos)? + rotate_half(q)?.broadcast_mul(&sin)?;
            let k_rot = k.broadcast_mul(&cos)? + rotate_half(k)?.broadcast_mul(&sin)?;
            Ok((q_rot?, k_rot?))
        }
    }

    fn rotate_half(x: &Tensor) -> CandleResult<Tensor> {
        let last = x.dim(D::Minus1)?;
        let h = last / 2;
        let x1 = x.narrow(D::Minus1, 0, h)?;
        let x2 = x.narrow(D::Minus1, h, last - h)?;
        Tensor::cat(&[&x2.neg()?, &x1], D::Minus1)
    }

    /// Glm4v vision attention: fused QKV + per-head Q/K RMSNorm + 2-D RoPE + output proj.
    struct Attention {
        qkv: candle_nn::Linear,
        q_norm: RmsNorm,
        k_norm: RmsNorm,
        proj: candle_nn::Linear,
        num_heads: usize,
        head_dim: usize,
        scale: f64,
    }

    impl Attention {
        fn new(config: &VisionConfig, vb: VarBuilder) -> CandleResult<Self> {
            let hidden = config.hidden_size;
            let head_dim = hidden / config.num_attention_heads;
            if !head_dim.is_multiple_of(2) {
                return Err(candle_core::Error::Msg(format!(
                    "head_dim must be even for rotate_half; got {head_dim}"
                )));
            }
            let scale = 1.0 / (head_dim as f64).sqrt();

            let qkv = if config.attention_bias {
                candle_nn::linear(hidden, 3 * hidden, vb.pp("qkv"))?
            } else {
                candle_nn::linear_no_bias(hidden, 3 * hidden, vb.pp("qkv"))?
            };

            // Per-head Q/K norm over head_dim. Upstream weight shape is [head_dim].
            let q_norm = RmsNorm::new(head_dim, config.rms_norm_eps, vb.pp("q_norm"))?;
            let k_norm = RmsNorm::new(head_dim, config.rms_norm_eps, vb.pp("k_norm"))?;

            // `proj` carries a bias in the published GLM-OCR safetensors (the upstream
            // transformers main branch has `bias=False` for vanilla Glm4v, but the
            // GLM-OCR variant ships `proj.bias`). The tensor file is authoritative.
            let proj = candle_nn::linear(hidden, hidden, vb.pp("proj"))?;

            Ok(Self {
                qkv,
                q_norm,
                k_norm,
                proj,
                num_heads: config.num_attention_heads,
                head_dim,
                scale,
            })
        }

        fn forward(&self, xs: &Tensor, rope: &Vision2dRope, grid_h: usize, grid_w: usize) -> CandleResult<Tensor> {
            let (batch, seq_len, _) = xs.dims3()?;

            // Fused QKV projection: (B, N, 3 * hidden) -> (3, B, H, N, D).
            let qkv = self.qkv.forward(xs)?;
            let qkv = qkv
                .reshape((batch, seq_len, 3, self.num_heads, self.head_dim))?
                .permute([2, 0, 3, 1, 4])?;
            let q = qkv.i(0)?.contiguous()?;
            let k = qkv.i(1)?.contiguous()?;
            let v = qkv.i(2)?.contiguous()?;

            // Per-head Q/K RMSNorm over head_dim.
            let q = self.q_norm.forward(&q)?;
            let k = self.k_norm.forward(&k)?;

            // 2-D RoPE on Q and K.
            let (q, k) = rope.apply(&q, &k, grid_h, grid_w)?;

            // Scaled dot-product attention.
            let scores = (q.matmul(&k.transpose(D::Minus2, D::Minus1)?)? * self.scale)?;
            let attn = candle_nn::ops::softmax_last_dim(&scores)?;
            let out = attn.matmul(&v)?;

            // (B, H, N, D) -> (B, N, H * D)
            let out = out
                .permute([0, 2, 1, 3])?
                .reshape((batch, seq_len, self.num_heads * self.head_dim))?;
            self.proj.forward(&out)
        }
    }

    /// SwiGLU MLP: `down_proj(silu(gate_proj(x)) * up_proj(x))`.
    ///
    /// All three projections carry biases (matches the GLM-OCR safetensors). Note this
    /// differs from current `transformers` main where `Glm4VisionMlp` defaults to
    /// `bias=False`; the published GLM-OCR weights include the biases.
    struct Mlp {
        gate_proj: candle_nn::Linear,
        up_proj: candle_nn::Linear,
        down_proj: candle_nn::Linear,
    }

    impl Mlp {
        fn new(config: &VisionConfig, vb: VarBuilder) -> CandleResult<Self> {
            let hidden = config.hidden_size;
            let inter = config.intermediate_size;
            let gate_proj = candle_nn::linear(hidden, inter, vb.pp("gate_proj"))?;
            let up_proj = candle_nn::linear(hidden, inter, vb.pp("up_proj"))?;
            let down_proj = candle_nn::linear(inter, hidden, vb.pp("down_proj"))?;
            Ok(Self {
                gate_proj,
                up_proj,
                down_proj,
            })
        }

        fn forward(&self, xs: &Tensor) -> CandleResult<Tensor> {
            let gate = self.gate_proj.forward(xs)?.silu()?;
            let up = self.up_proj.forward(xs)?;
            self.down_proj.forward(&(gate * up)?)
        }
    }

    /// Transformer encoder block: `norm1 -> attn -> residual -> norm2 -> mlp -> residual`.
    struct TransformerBlock {
        norm1: RmsNorm,
        attn: Attention,
        norm2: RmsNorm,
        mlp: Mlp,
    }

    impl TransformerBlock {
        fn new(config: &VisionConfig, vb: VarBuilder) -> CandleResult<Self> {
            let norm1 = RmsNorm::new(config.hidden_size, config.rms_norm_eps, vb.pp("norm1"))?;
            let attn = Attention::new(config, vb.pp("attn"))?;
            let norm2 = RmsNorm::new(config.hidden_size, config.rms_norm_eps, vb.pp("norm2"))?;
            let mlp = Mlp::new(config, vb.pp("mlp"))?;
            Ok(Self {
                norm1,
                attn,
                norm2,
                mlp,
            })
        }

        fn forward(&self, xs: &Tensor, rope: &Vision2dRope, grid_h: usize, grid_w: usize) -> CandleResult<Tensor> {
            let xs = (xs + self.attn.forward(&self.norm1.forward(xs)?, rope, grid_h, grid_w)?)?;
            &xs + self.mlp.forward(&self.norm2.forward(&xs)?)?
        }
    }

    /// Patch embedding via emulated `Conv3d` over `(temporal_patch_size, patch_size, patch_size)`.
    ///
    /// Candle 0.10 does not expose a `Conv3d`. For still images the input is replicated
    /// to length `temporal_patch_size` along the time axis, and stride `temporal_patch_size`
    /// collapses it back to length 1. With both temporal slices identical, the 3-D conv
    /// reduces exactly to a 2-D conv whose kernel is the sum of the temporal slices of
    /// the original 5-D weight: `W2d[d, c, h, w] = sum_t W5d[d, c, t, h, w]`.
    struct PatchEmbedding {
        conv: candle_nn::Conv2d,
    }

    impl PatchEmbedding {
        fn new(config: &VisionConfig, vb: VarBuilder) -> CandleResult<Self> {
            // Load the 5-D Conv3d weight: (out_channels, in_channels, T, kH, kW).
            let w5d = vb.get(
                (
                    config.hidden_size,
                    config.num_channels,
                    config.temporal_patch_size,
                    config.patch_size,
                    config.patch_size,
                ),
                "proj.weight",
            )?;
            // Sum over the temporal axis (dim 2): (out, in, kH, kW).
            let w2d = w5d.sum(2)?.contiguous()?;
            let bias = vb.get(config.hidden_size, "proj.bias")?;

            let cfg = candle_nn::Conv2dConfig {
                stride: config.patch_size,
                padding: 0,
                ..Default::default()
            };
            let conv = candle_nn::Conv2d::new(w2d, Some(bias), cfg);
            Ok(Self { conv })
        }

        /// Forward `(B, C, H, W) -> (B, num_patches, hidden)` where
        /// `num_patches = (H / patch_size) * (W / patch_size)`.
        fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
            let x = self.conv.forward(x)?;
            let (b, c, h, w) = x.dims4()?;
            // (B, hidden, h, w) -> (B, h * w, hidden)
            x.reshape((b, c, h * w))?.permute([0, 2, 1])
        }
    }

    /// Glm4v vision encoder. Owns the patch embed, transformer blocks, post layernorm,
    /// and the 2-D RoPE table.
    pub struct CogVit {
        patch_embed: PatchEmbedding,
        blocks: Vec<TransformerBlock>,
        post_layernorm: RmsNorm,
        rope: Vision2dRope,
        config: VisionConfig,
        #[allow(dead_code)]
        device: Device,
    }

    impl CogVit {
        /// Construct the encoder from a [`VarBuilder`] rooted at `model.visual.`.
        pub fn new(config: &VisionConfig, vb: VarBuilder, device: Device) -> Result<Self> {
            let patch_embed = PatchEmbedding::new(config, vb.pp("patch_embed"))
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Patch embedding init: {}", e)))?;

            let mut blocks = Vec::with_capacity(config.num_hidden_layers);
            for i in 0..config.num_hidden_layers {
                let block = TransformerBlock::new(config, vb.pp(format!("blocks.{}", i)))
                    .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Block {} init: {}", i, e)))?;
                blocks.push(block);
            }

            let post_layernorm = RmsNorm::new(config.hidden_size, config.rms_norm_eps, vb.pp("post_layernorm"))
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("post_layernorm init: {}", e)))?;

            let head_dim = config.hidden_size / config.num_attention_heads;
            let rope = Vision2dRope::new(head_dim, device.clone())
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Vision RoPE init: {}", e)))?;

            Ok(Self {
                patch_embed,
                blocks,
                post_layernorm,
                rope,
                config: config.clone(),
                device,
            })
        }

        /// Encode a `(B, C, H, W)` pixel tensor into `(B, num_image_tokens, hidden_size)`.
        ///
        /// `H` and `W` must be multiples of `patch_size`. The connector handles the
        /// subsequent `spatial_merge_size` merging step; this returns pre-merge tokens.
        pub fn forward(&self, pixel_values: &Tensor) -> Result<Tensor> {
            let dims = pixel_values
                .dims4()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("pixel_values must be 4-D: {}", e)))?;
            let (_, _, height, width) = dims;
            let patch = self.config.patch_size;
            if height % patch != 0 || width % patch != 0 {
                return Err(CandleOcrError::InferenceFailed(format!(
                    "pixel_values H/W ({}, {}) must be multiples of patch_size {}",
                    height, width, patch
                )));
            }
            let grid_h = height / patch;
            let grid_w = width / patch;

            // Patch embedding: emulated Conv3d collapses the temporal axis at load time
            // (see `PatchEmbedding`), so the 2-D forward here is exact for still images.
            let mut x = self
                .patch_embed
                .forward(pixel_values)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Patch embedding forward: {}", e)))?;

            for (i, block) in self.blocks.iter().enumerate() {
                x = block
                    .forward(&x, &self.rope, grid_h, grid_w)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Block {} forward: {}", i, e)))?;
            }

            self.post_layernorm
                .forward(&x)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("post_layernorm forward: {}", e)))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use imp::CogVit;
