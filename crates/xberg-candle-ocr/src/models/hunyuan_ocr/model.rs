// Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.

//! Hunyuan-VL model architecture: vision encoder + text decoder with XD-RoPE.

use candle_core::{D, IndexOp, Tensor};
use candle_nn::{Conv2d, Embedding, Init, Linear, Module, RmsNorm, VarBuilder, embedding, linear, linear_b, rms_norm};

use crate::error::{CandleOcrError, Result};
use crate::models::hunyuan_ocr::config::{HunYuanVLConfig, HunYuanVLVisionConfig};

// Re-export from vendored aha infrastructure.
use crate::vendor::aha::image::interpolate_bilinear;
use crate::vendor::aha::modules::{GateUpDownMLP, NaiveAttnTwoLinearMLPBlock, eager_attention_forward, get_conv2d};
use crate::vendor::aha::rope::{RoPE, apply_rotary_pos_emb, get_xd_cos_sin};
use crate::vendor::aha::{InferenceModel, MultiModalData};

/// Vision patch embedding layer: convolutional projection + positional embeddings.
pub struct HunYuanVisionPatchEmbed {
    patch_embedding: Conv2d,
    num_channels: usize,
    patch_size: usize,
    embed_dim: usize,
    patch_pos_embed: Tensor,
}

impl HunYuanVisionPatchEmbed {
    /// Create a new vision patch embedding layer.
    pub fn new(vb: VarBuilder, config: &HunYuanVLVisionConfig) -> Result<Self> {
        let patch_embedding = get_conv2d(
            vb.pp("patch_embedding"),
            config.num_channels,
            config.hidden_size,
            config.patch_size,
            0,
            config.patch_size,
            1,
            1,
            true,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("patch_embedding conv: {}", e)))?;

        let num_channels = config.num_channels;
        let patch_size = config.patch_size;
        let position_edge = config.max_image_size / patch_size;
        let num_positions = (position_edge).pow(2) + 1;
        let embed_dim = config.hidden_size;

        let position_embedding = embedding(num_positions, embed_dim, vb.pp("position_embedding"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("position_embedding: {}", e)))?;

        let patch_pos_embed = position_embedding
            .embeddings()
            .i(1..)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Index position embeddings: {}", e)))?
            .reshape((1, position_edge, position_edge, embed_dim))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape position embed: {}", e)))?
            .permute((0, 3, 1, 2))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Permute position embed: {}", e)))?;

        Ok(Self {
            patch_embedding,
            num_channels,
            patch_size,
            embed_dim,
            patch_pos_embed,
        })
    }

    /// Forward pass: embed patches and add positional embeddings.
    pub fn forward(&self, pixel_values: &Tensor, grid_thw: &Tensor) -> Result<Tensor> {
        let (num_patches, _) = pixel_values
            .dims2()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dims2: {}", e)))?;

        let pixel_values = pixel_values
            .reshape((num_patches, self.num_channels, self.patch_size, self.patch_size))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape pixels: {}", e)))?;

        let patch_embeds = self
            .patch_embedding
            .forward(&pixel_values)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Conv forward: {}", e)))?;

        let patch_embeds = patch_embeds
            .squeeze(D::Minus1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze -1: {}", e)))?
            .squeeze(D::Minus1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze -1: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze 0: {}", e)))?;

        let mut patch_pos_embed_list = vec![];
        let img_num = grid_thw
            .dim(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid dim0: {}", e)))?;

        for i in 0..img_num {
            let grid_i = grid_thw
                .i(i)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid[{}]: {}", i, e)))?;

            let grid_h = grid_i
                .i(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid[1]: {}", e)))?
                .to_scalar::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid h scalar: {}", e)))?
                as usize;

            let grid_w = grid_i
                .i(2)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid[2]: {}", e)))?
                .to_scalar::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid w scalar: {}", e)))?
                as usize;

            let patch_pos_embed_ = interpolate_bilinear(&self.patch_pos_embed, (grid_h, grid_w), Some(false), None)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Interpolate: {}", e)))?;

            let patch_pos_embed_ = patch_pos_embed_
                .reshape((self.embed_dim, ()))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape embed: {}", e)))?
                .transpose(0, 1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Transpose: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?;

            patch_pos_embed_list.push(patch_pos_embed_);
        }

        let patch_pos_embed = Tensor::cat(&patch_pos_embed_list, 1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat pos embeds: {}", e)))?;

        let embedding = patch_embeds
            .add(&patch_pos_embed)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Add pos embed: {}", e)))?;

        Ok(embedding)
    }
}

/// Vision patch merger: spatial downsampling + projection.
pub struct HunYuanVisionPatchMerger {
    proj_0: Conv2d,
    proj_2: Conv2d,
    mlp: Linear,
    image_newline: Tensor,
    image_begin: Tensor,
    image_end: Tensor,
    before_rms: RmsNorm,
    after_rms: RmsNorm,
}

impl HunYuanVisionPatchMerger {
    /// Create a new patch merger.
    pub fn new(vb: VarBuilder, config: &HunYuanVLVisionConfig) -> Result<Self> {
        let proj_0 = get_conv2d(
            vb.pp("proj.0"),
            config.hidden_size,
            config.hidden_size * 2,
            config.spatial_merge_size,
            0,
            config.spatial_merge_size,
            1,
            1,
            true,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("proj_0: {}", e)))?;

        let proj_2 = get_conv2d(
            vb.pp("proj.2"),
            config.hidden_size * 2,
            config.hidden_size * 4,
            1,
            0,
            1,
            1,
            1,
            true,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("proj_2: {}", e)))?;

        let mlp = linear(config.hidden_size * 4, config.out_hidden_size, vb.pp("mlp"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("mlp: {}", e)))?;

        let image_newline = vb
            .get_with_hints(config.hidden_size * 4, "image_newline", Init::Const(0.))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("image_newline: {}", e)))?;

        let image_begin = vb
            .get_with_hints(config.out_hidden_size, "image_begin", Init::Const(0.))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("image_begin: {}", e)))?;

        let image_end = vb
            .get_with_hints(config.out_hidden_size, "image_end", Init::Const(0.))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("image_end: {}", e)))?;

        let before_rms = rms_norm(config.hidden_size, config.rms_norm_eps, vb.pp("before_rms"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("before_rms: {}", e)))?;

        let after_rms = rms_norm(config.out_hidden_size, config.rms_norm_eps, vb.pp("after_rms"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("after_rms: {}", e)))?;

        Ok(Self {
            proj_0,
            proj_2,
            mlp,
            image_newline,
            image_begin,
            image_end,
            before_rms,
            after_rms,
        })
    }

    /// Forward pass: merge patches spatially and project.
    pub fn forward(&self, xs: &Tensor, size: (usize, usize)) -> Result<Tensor> {
        let xs = self
            .before_rms
            .forward(xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Before RMS: {}", e)))?;

        let (h, w) = size;
        let xs = xs
            .permute((0, 2, 1))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Permute: {}", e)))?
            .reshape((
                xs.dim(0)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Dim 0: {}", e)))?,
                (),
                h,
                w,
            ))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape: {}", e)))?;

        let xs = self
            .proj_0
            .forward(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Proj 0: {}", e)))?
            .gelu()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("GELU: {}", e)))?;

        let xs = self
            .proj_2
            .forward(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Proj 2: {}", e)))?;

        let (b, c, h, _) = xs
            .dims4()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dims4: {}", e)))?;

        let image_newline = self
            .image_newline
            .reshape((1, c, 1, 1))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape newline: {}", e)))?
            .broadcast_as((b, c, h, 1))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Broadcast newline: {}", e)))?
            .to_dtype(xs.dtype())
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dtype newline: {}", e)))?;

        let xs = Tensor::cat(&[xs, image_newline], D::Minus1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat newline: {}", e)))?;

        let xs = xs
            .reshape((b, c, ()))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape: {}", e)))?
            .permute((0, 2, 1))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Permute: {}", e)))?;

        let xs = self
            .mlp
            .forward(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("MLP: {}", e)))?;

        let begin = self
            .image_begin
            .reshape((1, 1, ()))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape begin: {}", e)))?
            .broadcast_as((
                b,
                1,
                xs.dim(D::Minus1)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Dim -1: {}", e)))?,
            ))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Broadcast begin: {}", e)))?
            .to_dtype(xs.dtype())
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dtype begin: {}", e)))?;

        let end = self
            .image_end
            .reshape((1, 1, ()))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape end: {}", e)))?
            .broadcast_as((
                b,
                1,
                xs.dim(D::Minus1)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Dim -1: {}", e)))?,
            ))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Broadcast end: {}", e)))?
            .to_dtype(xs.dtype())
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dtype end: {}", e)))?;

        let xs = Tensor::cat(&[begin, xs, end], 1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat begin/end: {}", e)))?;

        let xs = self
            .after_rms
            .forward(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("After RMS: {}", e)))?;

        Ok(xs)
    }
}

/// Vision transformer: embeddings + transformer layers + patch merger.
pub struct HunYuanVisionTransformer {
    embeddings: HunYuanVisionPatchEmbed,
    layers: Vec<NaiveAttnTwoLinearMLPBlock>,
    perceive: HunYuanVisionPatchMerger,
}

impl HunYuanVisionTransformer {
    /// Create a new vision transformer.
    pub fn new(vb: VarBuilder, config: &HunYuanVLVisionConfig) -> Result<Self> {
        let embeddings = HunYuanVisionPatchEmbed::new(vb.pp("embeddings"), config)?;

        let mut layers = vec![];
        let vb_layers = vb.pp("layers");
        for i in 0..config.num_hidden_layers {
            let layer_i = NaiveAttnTwoLinearMLPBlock::new(
                vb_layers.pp(i),
                config.hidden_size,
                config.num_attention_heads,
                None,
                None,
                true,
                "self_attn",
                None,
                config.intermediate_size,
                config.hidden_act,
                true,
                "mlp",
                "dense_h_to_4h",
                "dense_4h_to_h",
                config.rms_norm_eps,
                "input_layernorm",
                "post_attention_layernorm",
            )
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Layer {}: {}", i, e)))?;

            layers.push(layer_i);
        }

        let perceive = HunYuanVisionPatchMerger::new(vb.pp("perceive"), config)?;

        Ok(Self {
            embeddings,
            layers,
            perceive,
        })
    }

    /// Forward pass: embed patches → transformer layers → merge patches.
    pub fn forward(&self, xs: &Tensor, grid_thw: &Tensor) -> Result<Tensor> {
        let mut hidden_states = self.embeddings.forward(xs, grid_thw)?;

        for layer in &self.layers {
            hidden_states = layer
                .forward(&hidden_states, None, None, None, false)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Layer forward: {}", e)))?;
        }

        let img_num = grid_thw
            .dim(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid num: {}", e)))?;

        let mut cu_seqlens = vec![];
        for i in 0..img_num {
            let grid_i_vec = grid_thw
                .i(i)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid: {}", e)))?
                .to_vec1::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid vec: {}", e)))?;

            if grid_i_vec.len() != 3 {
                return Err(CandleOcrError::InvalidTensorShape {
                    expected: "3 elements (T, H, W)".to_string(),
                    got: format!("{} elements", grid_i_vec.len()),
                });
            }

            let h = grid_i_vec[1];
            let w = grid_i_vec[2];
            cu_seqlens.push((h * w) as usize);
        }

        let split_items = split_tensor(&hidden_states, &cu_seqlens, 1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Split tensor: {}", e)))?;

        let mut processed_item = vec![];
        for (i, item) in split_items.into_iter().enumerate().take(img_num) {
            let grid_i_vec = grid_thw
                .i(i)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid: {}", e)))?
                .to_vec1::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid vec: {}", e)))?;

            let h = grid_i_vec[1] as usize;
            let w = grid_i_vec[2] as usize;

            let processed = self
                .perceive
                .forward(&item, (h, w))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Perceive forward: {}", e)))?;

            processed_item.push(processed);
        }

        let xs = Tensor::cat(&processed_item, 1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat processed: {}", e)))?;

        Ok(xs)
    }
}

/// Multi-head self-attention with QK normalization and key-value caching.
pub struct HunYuanVLAttention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    o_proj: Linear,
    query_layernorm: RmsNorm,
    key_layernorm: RmsNorm,
    num_attention_heads: usize,
    num_key_value_heads: usize,
    num_kv_groups: usize,
    head_dim: usize,
    scaling: f64,
    kv_cache: Option<(Tensor, Tensor)>,
}

impl HunYuanVLAttention {
    /// Create a new attention layer.
    pub fn new(
        vb: VarBuilder,
        hidden_size: usize,
        head_dim: usize,
        num_attention_heads: usize,
        num_key_value_heads: usize,
        attention_bias: bool,
        rms_norm_eps: f64,
    ) -> Result<Self> {
        let num_kv_groups = num_attention_heads / num_key_value_heads;
        let scaling = 1f64 / f64::sqrt(head_dim as f64);

        let q_proj = linear_b(
            hidden_size,
            num_attention_heads * head_dim,
            attention_bias,
            vb.pp("q_proj"),
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("q_proj: {}", e)))?;

        let k_proj = linear_b(
            hidden_size,
            num_key_value_heads * head_dim,
            attention_bias,
            vb.pp("k_proj"),
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("k_proj: {}", e)))?;

        let v_proj = linear_b(
            hidden_size,
            num_key_value_heads * head_dim,
            attention_bias,
            vb.pp("v_proj"),
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("v_proj: {}", e)))?;

        let o_proj = linear_b(
            num_attention_heads * head_dim,
            hidden_size,
            attention_bias,
            vb.pp("o_proj"),
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("o_proj: {}", e)))?;

        let query_layernorm = rms_norm(head_dim, rms_norm_eps, vb.pp("query_layernorm"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("query_layernorm: {}", e)))?;

        let key_layernorm = rms_norm(head_dim, rms_norm_eps, vb.pp("key_layernorm"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("key_layernorm: {}", e)))?;

        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            query_layernorm,
            key_layernorm,
            num_attention_heads,
            num_key_value_heads,
            num_kv_groups,
            head_dim,
            scaling,
            kv_cache: None,
        })
    }

    /// Forward pass with RoPE and key-value caching.
    pub fn forward(
        &mut self,
        xs: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        let (b_sz, q_len, _) = xs
            .dims3()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dims3: {}", e)))?;

        let query_states = self
            .q_proj
            .forward(xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Q proj: {}", e)))?
            .reshape((b_sz, q_len, self.num_attention_heads, self.head_dim))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape Q: {}", e)))?
            .transpose(1, 2)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Transpose Q: {}", e)))?;

        let key_states = self
            .k_proj
            .forward(xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("K proj: {}", e)))?
            .reshape((b_sz, q_len, self.num_key_value_heads, self.head_dim))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape K: {}", e)))?
            .transpose(1, 2)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Transpose K: {}", e)))?;

        let value_states = self
            .v_proj
            .forward(xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("V proj: {}", e)))?
            .reshape((b_sz, q_len, self.num_key_value_heads, self.head_dim))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape V: {}", e)))?
            .transpose(1, 2)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Transpose V: {}", e)))?;

        let (query_states, key_states) = apply_rotary_pos_emb(&query_states, &key_states, cos, sin, false)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Apply RoPE: {}", e)))?;

        let query_states = self
            .query_layernorm
            .forward(&query_states)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Query LN: {}", e)))?;

        let key_states = self
            .key_layernorm
            .forward(&key_states)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Key LN: {}", e)))?;

        let (key_states, value_states) = match &self.kv_cache {
            None => (key_states, value_states),
            Some((prev_k, prev_v)) => {
                let key_states = Tensor::cat(&[prev_k, &key_states], 2)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat K: {}", e)))?;
                let value_states = Tensor::cat(&[prev_v, &value_states], 2)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat V: {}", e)))?;
                (key_states, value_states)
            }
        };

        self.kv_cache = Some((key_states.clone(), value_states.clone()));

        let attn_output = eager_attention_forward(
            &query_states,
            &key_states,
            &value_states,
            Some(self.num_kv_groups),
            attention_mask,
            self.scaling,
        )
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Attention forward: {}", e)))?;

        let attn_output = attn_output
            .reshape((b_sz, q_len, self.num_attention_heads * self.head_dim))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape output: {}", e)))?;

        let attn_output = attn_output
            .apply(&self.o_proj)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("O proj: {}", e)))?;

        Ok(attn_output)
    }

    /// Clear the key-value cache for a new sequence.
    pub fn clear_kv_cache(&mut self) {
        self.kv_cache = None
    }
}

/// Transformer decoder layer: self-attention + MLP.
pub struct HunYuanVLDecoderLayer {
    self_attn: HunYuanVLAttention,
    mlp: GateUpDownMLP,
    input_layernorm: RmsNorm,
    post_attention_layernorm: RmsNorm,
}

impl HunYuanVLDecoderLayer {
    /// Create a new decoder layer.
    pub fn new(config: &HunYuanVLConfig, vb: VarBuilder) -> Result<Self> {
        let self_attn = HunYuanVLAttention::new(
            vb.pp("self_attn"),
            config.hidden_size,
            config.head_dim,
            config.num_attention_heads,
            config.num_key_value_heads,
            config.attention_bias,
            config.rms_norm_eps,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Self-attn: {}", e)))?;

        let mlp = GateUpDownMLP::new(
            vb.pp("mlp"),
            config.hidden_size,
            config.intermediate_size,
            config.hidden_act,
            false,
            None,
            None,
            None,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("MLP: {}", e)))?;

        let input_layernorm = rms_norm(config.hidden_size, config.rms_norm_eps, vb.pp("input_layernorm"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Input LN: {}", e)))?;

        let post_attention_layernorm = rms_norm(
            config.hidden_size,
            config.rms_norm_eps,
            vb.pp("post_attention_layernorm"),
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Post-attn LN: {}", e)))?;

        Ok(Self {
            self_attn,
            mlp,
            input_layernorm,
            post_attention_layernorm,
        })
    }

    /// Forward pass with residual connections.
    pub fn forward(
        &mut self,
        xs: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        let residual = xs.clone();

        let xs = self
            .input_layernorm
            .forward(xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Input LN: {}", e)))?;

        let xs = self
            .self_attn
            .forward(&xs, cos, sin, attention_mask)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Self-attn: {}", e)))?;

        let xs = residual
            .add(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Add residual 1: {}", e)))?;

        let residual = xs.clone();

        let xs = self
            .post_attention_layernorm
            .forward(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Post-attn LN: {}", e)))?;

        let xs = self
            .mlp
            .forward(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("MLP: {}", e)))?;

        let xs = residual
            .add(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Add residual 2: {}", e)))?;

        Ok(xs)
    }

    /// Clear the key-value cache.
    pub fn clear_kv_cache(&mut self) {
        self.self_attn.clear_kv_cache();
    }
}

/// Text decoder with embeddings, transformer layers, and RoPE.
pub struct HunYuanVLTextModel {
    embed_tokens: Embedding,
    layers: Vec<HunYuanVLDecoderLayer>,
    norm: RmsNorm,
    rope: RoPE,
    xdrope_section: Vec<usize>,
}

impl HunYuanVLTextModel {
    /// Create a new text model.
    pub fn new(vb: VarBuilder, config: &HunYuanVLConfig) -> Result<Self> {
        let embed_tokens = embedding(config.vocab_size, config.hidden_size, vb.pp("embed_tokens"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Embed tokens: {}", e)))?;

        let mut layers = vec![];
        let vb_layers = vb.pp("layers");
        for i in 0..config.num_hidden_layers {
            let layer = HunYuanVLDecoderLayer::new(config, vb_layers.pp(i))
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Layer {}: {}", i, e)))?;
            layers.push(layer);
        }

        let norm = rms_norm(config.hidden_size, config.rms_norm_eps, vb.pp("norm"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Norm: {}", e)))?;

        let base = config.rope_theta
            * config
                .rope_scaling
                .alpha
                .powf(config.head_dim as f64 / (config.head_dim - 2) as f64);

        let rope = RoPE::new(config.head_dim, base as f32, vb.device())
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("RoPE: {}", e)))?;

        let xdrope_section = config.rope_scaling.xdrope_section.clone();

        Ok(Self {
            embed_tokens,
            layers,
            norm,
            rope,
            xdrope_section,
        })
    }

    /// Forward pass with optional position IDs for XD-RoPE.
    pub fn forward(
        &mut self,
        inputs_embeds: &Tensor,
        position_ids: Option<&Tensor>,
        seqlen_offset: usize,
    ) -> Result<Tensor> {
        let (b_size, seq_len, _) = inputs_embeds
            .dims3()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dims3: {}", e)))?;

        let attention_mask: Option<Tensor> = if seq_len <= 1 {
            None
        } else {
            Some(
                prepare_causal_attention_mask(b_size, seq_len, inputs_embeds.device())
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Causal mask: {}", e)))?,
            )
        };

        let (cos, sin) = self
            .rope
            .forward(seqlen_offset, seq_len, inputs_embeds.device())
            .map_err(|e| CandleOcrError::InferenceFailed(format!("RoPE forward: {}", e)))?;

        let mut xs = inputs_embeds.clone();

        for (i, layer) in self.layers.iter_mut().enumerate() {
            if i == 0 {
                if let Some(pos_ids) = position_ids {
                    let (cos, sin) = get_xd_cos_sin(&cos, &sin, pos_ids, self.xdrope_section.clone())
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("XD-RoPE: {}", e)))?;
                    xs = layer
                        .forward(&xs, &cos, &sin, attention_mask.as_ref())
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Layer {}: {}", i, e)))?;
                } else {
                    xs = layer
                        .forward(&xs, &cos, &sin, attention_mask.as_ref())
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Layer {}: {}", i, e)))?;
                }
            } else {
                xs = layer
                    .forward(&xs, &cos, &sin, attention_mask.as_ref())
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Layer {}: {}", i, e)))?;
            }
        }

        let xs = self
            .norm
            .forward(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Norm: {}", e)))?;

        Ok(xs)
    }

    /// Clear the key-value cache for all layers.
    pub fn clear_kv_cache(&mut self) {
        for layer in self.layers.iter_mut() {
            layer.clear_kv_cache()
        }
    }
}

/// Complete Hunyuan-VL model: vision encoder + text decoder + LM head.
pub struct HunyuanVLModel {
    vit: HunYuanVisionTransformer,
    model: HunYuanVLTextModel,
    lm_head: Linear,
    stop_token_ids: Vec<u32>,
}

impl HunyuanVLModel {
    /// Create a new Hunyuan-VL model.
    pub fn new(vb: VarBuilder, config: HunYuanVLConfig, eos_ids: Vec<u32>) -> Result<Self> {
        let vit = HunYuanVisionTransformer::new(vb.pp("vit"), &config.vision_config)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("ViT: {}", e)))?;

        let model = HunYuanVLTextModel::new(vb.pp("model"), &config)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Text model: {}", e)))?;

        let lm_head = Linear::new(model.embed_tokens.embeddings().clone(), None);

        Ok(Self {
            vit,
            model,
            lm_head,
            stop_token_ids: eos_ids,
        })
    }

    /// Forward pass with optional multimodal data.
    pub fn forward(
        &mut self,
        input_ids: &Tensor,
        pixel_values: Option<&Tensor>,
        image_grid_thw: Option<&Tensor>,
        image_mask: Option<&Tensor>,
        position_ids: Option<&Tensor>,
        seqlen_offset: usize,
    ) -> Result<Tensor> {
        let mut inputs_embeds = self
            .model
            .embed_tokens
            .forward(input_ids)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Embed tokens: {}", e)))?;

        if let (Some(pixel_values), Some(grid_thw), Some(image_mask)) = (pixel_values, image_grid_thw, image_mask) {
            let image_embeds = self
                .vit
                .forward(pixel_values, grid_thw)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("ViT: {}", e)))?
                .squeeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze: {}", e)))?;

            inputs_embeds = masked_scatter_dim0(&inputs_embeds, &image_embeds, image_mask)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Scatter image: {}", e)))?;
        }

        let outputs = self
            .model
            .forward(&inputs_embeds, position_ids, seqlen_offset)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Text forward: {}", e)))?;

        let seq_len = outputs
            .dim(1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dim 1: {}", e)))?;

        let hidden_state = outputs
            .narrow(1, seq_len - 1, 1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow: {}", e)))?;

        let logits = self
            .lm_head
            .forward(&hidden_state)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("LM head: {}", e)))?;

        Ok(logits)
    }

    /// Clear the key-value cache.
    pub fn clear_kv_cache(&mut self) {
        self.model.clear_kv_cache();
    }
}

impl InferenceModel for HunyuanVLModel {
    fn forward_initial(&mut self, input_ids: &Tensor, seqlen_offset: usize, data: MultiModalData) -> Result<Tensor> {
        if data.data_vec.len() != 4 {
            return Err(CandleOcrError::InferenceFailed(
                "Hunyuan-VL requires 4 multimodal data items: pixel_values, image_grid_thw, image_mask, position_ids"
                    .to_string(),
            ));
        }

        let pixel_values = &data.data_vec[0];
        let image_grid_thw = &data.data_vec[1];
        let image_mask = &data.data_vec[2];
        let position_ids = &data.data_vec[3];

        self.forward(
            input_ids,
            pixel_values.as_ref(),
            image_grid_thw.as_ref(),
            image_mask.as_ref(),
            position_ids.as_ref(),
            seqlen_offset,
        )
    }

    fn forward_step(&mut self, input_ids: &Tensor, seqlen_offset: usize) -> Result<Tensor> {
        self.forward(input_ids, None, None, None, None, seqlen_offset)
    }

    fn clear_cache(&mut self) {
        self.clear_kv_cache();
    }

    fn stop_token_ids(&self) -> Vec<u32> {
        self.stop_token_ids.clone()
    }
}

// ---------------------------------------------------------------------------
// Internal tensor helpers
// ---------------------------------------------------------------------------

/// Split tensor `t` along `dim` into consecutive slices of sizes `splits`.
fn split_tensor(t: &Tensor, splits: &[usize], dim: usize) -> Result<Vec<Tensor>> {
    let mut results: Vec<Tensor> = Vec::with_capacity(splits.len());
    let mut offset = 0;
    for &size in splits {
        results.push(
            t.narrow(dim, offset, size)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow: {}", e)))?,
        );
        offset += size;
    }
    Ok(results)
}

/// Create a causal attention mask for autoregressive decoding.
fn prepare_causal_attention_mask(batch_size: usize, seq_len: usize, device: &candle_core::Device) -> Result<Tensor> {
    let mut mask_data = vec![0f32; batch_size * seq_len * seq_len];
    for b in 0..batch_size {
        for i in 0..seq_len {
            for j in 0..seq_len {
                if j > i {
                    // Mask out future positions.
                    mask_data[b * seq_len * seq_len + i * seq_len + j] = f32::NEG_INFINITY;
                }
            }
        }
    }
    Tensor::from_vec(mask_data, (batch_size, seq_len, seq_len), device)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Create mask: {}", e)))
}

/// Scatter image embeddings into position placeholders using a mask.
fn masked_scatter_dim0(base: &Tensor, image_embeds: &Tensor, mask: &Tensor) -> Result<Tensor> {
    // The forward passes a batched [1, seq, hidden]; the scatter replaces rows along
    // the sequence axis, so drop the leading batch dim, scatter, then restore it.
    // Without this base_len is the batch size (1) and every text row past position 0
    // is dropped while image rows keep a stale rank, so Tensor::stack sees mixed ranks.
    let batched = base.rank() == 3;
    let base = if batched {
        base.squeeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Base squeeze: {}", e)))?
    } else {
        base.clone()
    };
    let mask_vec = mask
        .flatten_all()
        .and_then(|t| t.to_vec1::<u32>())
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Mask vec: {}", e)))?;
    let base_len = base
        .dim(0)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Base dim: {}", e)))?;

    // Collect rows to build the output tensor.
    let mut result_rows = vec![];
    let mut img_idx = 0;

    for (pos, &is_image) in mask_vec.iter().enumerate() {
        if is_image == 1
            && img_idx
                < image_embeds
                    .dim(0)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Img dim: {}", e)))?
        {
            let img_embed = image_embeds
                .i(img_idx)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index image: {}", e)))?;
            result_rows.push(img_embed);
            img_idx += 1;
        } else if pos < base_len {
            let base_row = base
                .i(pos)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index base: {}", e)))?;
            result_rows.push(base_row);
        }
    }

    let out = Tensor::stack(&result_rows, 0)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Stack: {}", e)))?;
    if batched {
        out.unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Scatter unsqueeze: {}", e)))
    } else {
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device, Tensor};
    use candle_nn::Activation;

    use crate::models::hunyuan_ocr::config::{HunYuanVLConfig, HunYuanVLRopeScaling, HunYuanVLVisionConfig};

    // -----------------------------------------------------------------------
    // Minimal config helpers
    // -----------------------------------------------------------------------

    /// Build a minimal `HunYuanVLVisionConfig` suitable for CPU/synthetic tests.
    fn tiny_vision_config() -> HunYuanVLVisionConfig {
        HunYuanVLVisionConfig {
            add_patchemb_bias: true,
            attention_dropout: 0.0,
            cat_extra_token: 0,
            hidden_act: Activation::Gelu,
            hidden_dropout: 0.0,
            hidden_size: 16,
            img_max_token_num: 4096,
            intermediate_size: 32,
            interpolate_mode: "bilinear".to_string(),
            max_image_size: 32,
            max_vit_seq_len: 4096,
            num_attention_heads: 2,
            num_channels: 3,
            num_hidden_layers: 1,
            out_hidden_size: 16,
            patch_size: 16,
            rms_norm_eps: 1e-6,
            spatial_merge_size: 1,
        }
    }

    /// Build a minimal `HunYuanVLConfig` that does not require real weights.
    fn tiny_full_config() -> HunYuanVLConfig {
        HunYuanVLConfig {
            attention_bias: false,
            attention_dropout: 0.0,
            attention_head_dim: 16,
            bos_token_id: 1,
            eod_token_id: 2,
            eos_token_id: 2,
            head_dim: 16,
            hidden_act: Activation::Silu,
            hidden_size: 32,
            image_start_token_id: 100,
            image_end_token_id: 101,
            image_token_id: 102,
            image_newline_token_id: 103,
            initializer_range: 0.02,
            intermediate_size: 64,
            max_position_embeddings: 512,
            mlp_bias: false,
            norm_type: "rms_norm".to_string(),
            num_attention_heads: 2,
            num_experts: 1,
            num_hidden_layers: 1,
            num_key_value_heads: 2,
            org_vocab_size: 128,
            pad_id: 0,
            pad_token_id: 0,
            pretraining_tp: 1,
            rms_norm_eps: 1e-6,
            rope_scaling: HunYuanVLRopeScaling {
                alpha: 1.0,
                beta_fast: 32,
                beta_slow: 1,
                factor: 1.0,
                mscale: 1.0,
                mscale_all_dim: 1.0,
                type_field: "linear".to_string(),
                // 4 sections, each occupies head_dim/4 = 4 dims (rope works on
                // head_dim/2 = 8 halves; sections must sum to that).
                xdrope_section: vec![2, 2, 2, 2],
            },
            rope_theta: 10000.0,
            routed_scaling_factor: 1.0,
            sep_token_id: 10,
            text_end_id: 11,
            text_start_id: 12,
            tie_word_embeddings: false,
            dtype: "float32".to_string(),
            use_cache: true,
            use_qk_norm: true,
            use_cla: false,
            vision_config: tiny_vision_config(),
            vocab_size: 128,
        }
    }

    // -----------------------------------------------------------------------
    // E1-1: existing — attention layer construction (renamed to follow convention)
    // -----------------------------------------------------------------------

    #[test]
    fn should_construct_attention_layer_given_valid_dimensions() -> Result<()> {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);

        let _attn = HunYuanVLAttention::new(
            vb, 256,  // hidden_size
            64,   // head_dim
            4,    // num_attention_heads
            2,    // num_key_value_heads
            true, // attention_bias
            1e-6, // rms_norm_eps
        )?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // E1-2: Config deserialization
    // -----------------------------------------------------------------------

    #[test]
    fn should_deserialize_vision_config_from_json_when_all_fields_present() {
        let json = r#"{
            "add_patchemb_bias": true,
            "attention_dropout": 0.0,
            "cat_extra_token": 0,
            "hidden_act": "gelu",
            "hidden_dropout": 0.0,
            "hidden_size": 1280,
            "img_max_token_num": 16384,
            "intermediate_size": 5120,
            "interpolate_mode": "bilinear",
            "max_image_size": 448,
            "max_vit_seq_len": 16384,
            "num_attention_heads": 16,
            "num_channels": 3,
            "num_hidden_layers": 32,
            "out_hidden_size": 3584,
            "patch_size": 14,
            "rms_norm_eps": 1e-6,
            "spatial_merge_size": 2
        }"#;

        let cfg: HunYuanVLVisionConfig =
            serde_json::from_str(json).expect("HunYuanVLVisionConfig should deserialize from JSON");

        assert_eq!(cfg.hidden_size, 1280, "hidden_size mismatch");
        assert_eq!(cfg.num_hidden_layers, 32, "num_hidden_layers mismatch");
        assert_eq!(cfg.patch_size, 14, "patch_size mismatch");
        assert_eq!(cfg.out_hidden_size, 3584, "out_hidden_size mismatch");
        assert_eq!(cfg.num_channels, 3, "num_channels must be 3 for RGB");
        assert_eq!(cfg.spatial_merge_size, 2, "spatial_merge_size mismatch");
        assert_eq!(cfg.interpolate_mode, "bilinear", "interpolate_mode mismatch");
    }

    // -----------------------------------------------------------------------
    // E1-3: Vision transformer construction with zero-weights
    // -----------------------------------------------------------------------

    #[test]
    fn should_construct_vision_transformer_given_tiny_config_and_zero_weights() -> Result<()> {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);
        let cfg = tiny_vision_config();

        // Construction must succeed without panicking.
        let _vit = HunYuanVisionTransformer::new(vb, &cfg)?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // E1-4: Vision patch embedding forward produces expected shape
    //
    // patch_size=16, max_image_size=32 → position_edge = 32/16 = 2
    // input pixels: (1, 3, 16, 16) → after conv → 1 patch of shape (1, hidden_size)
    // With a 2×2 position grid, a single 1×1 grid must interpolate correctly.
    // -----------------------------------------------------------------------

    #[test]
    fn should_produce_correct_output_shape_from_vision_patch_embed_given_single_patch() -> Result<()> {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);
        let cfg = tiny_vision_config();
        // patch_size=16, hidden_size=16, num_channels=3
        // pixel_values shape: (num_patches=1, num_channels * patch_size^2)
        let embed = HunYuanVisionPatchEmbed::new(vb, &cfg)?;

        let patch_pixels = cfg.num_channels * cfg.patch_size * cfg.patch_size; // 3*16*16 = 768
        let pixel_values = Tensor::zeros((1, patch_pixels), DType::F32, &dev)?;
        // grid_thw: (1, 3) — one image, T=1, H=1, W=1
        let grid_thw = Tensor::from_vec(vec![1u32, 1u32, 1u32], (1, 3), &dev)?;

        let output = embed.forward(&pixel_values, &grid_thw)?;

        // Expected: (batch=1, num_patches=1, hidden_size=16)
        let dims = output.dims();
        assert_eq!(
            dims,
            &[1, 1, cfg.hidden_size],
            "Vision patch embed output shape should be (1, 1, hidden_size=16); got {:?}",
            dims
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // E1-5: Text model construction with zero-weights
    // -----------------------------------------------------------------------

    #[test]
    fn should_construct_text_model_given_tiny_config_and_zero_weights() -> Result<()> {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);
        let cfg = tiny_full_config();

        let _text_model = HunYuanVLTextModel::new(vb, &cfg)?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // E1-6: Text model forward without position_ids (standard RoPE path)
    // -----------------------------------------------------------------------

    #[test]
    fn should_produce_output_with_hidden_size_dim_when_text_model_forward_called_without_position_ids() -> Result<()> {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);
        let cfg = tiny_full_config();
        let hidden_size = cfg.hidden_size;

        let mut text_model = HunYuanVLTextModel::new(vb, &cfg)?;

        // inputs_embeds: (batch=1, seq_len=3, hidden_size=32)
        let inputs_embeds = Tensor::zeros((1, 3, hidden_size), DType::F32, &dev)?;
        let output = text_model.forward(&inputs_embeds, None, 0)?;

        let dims = output.dims();
        assert_eq!(
            dims,
            &[1, 3, hidden_size],
            "Text model output shape should be (batch=1, seq_len=3, hidden_size=32); got {:?}",
            dims
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // E1-7: XD-RoPE path — forward with position_ids accepted without panic
    // -----------------------------------------------------------------------

    #[test]
    fn should_accept_position_ids_and_produce_output_when_text_model_forward_uses_xd_rope_path() -> Result<()> {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);
        let cfg = tiny_full_config();
        let hidden_size = cfg.hidden_size;
        let seq_len: usize = 4;

        let mut text_model = HunYuanVLTextModel::new(vb, &cfg)?;

        let inputs_embeds = Tensor::zeros((1, seq_len, hidden_size), DType::F32, &dev)?;

        // position_ids shape: (1, 4, seq_len) — 4 XD-RoPE axes (t/h/w/text)
        // values are simple sequential positions 0..seq_len for each axis.
        let pos_row: Vec<u32> = (0..seq_len as u32).collect();
        let pos_flat: Vec<u32> = pos_row.iter().cycle().take(4 * seq_len).cloned().collect();
        let position_ids = Tensor::from_vec(pos_flat, (1, 4, seq_len), &dev)?;

        let output = text_model.forward(&inputs_embeds, Some(&position_ids), 0)?;

        // Output must be a tensor with the correct shape — the XD-RoPE path
        // only applies to layer 0; subsequent layers fall through normally.
        let dims = output.dims();
        assert_eq!(
            dims,
            &[1, seq_len, hidden_size],
            "Text model output shape should be (1, seq_len={}, hidden_size={}); got {:?}",
            seq_len,
            hidden_size,
            dims
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // E1-8: KV cache cleared between sequences
    // -----------------------------------------------------------------------

    #[test]
    fn should_clear_kv_cache_when_clear_kv_cache_called_on_text_model() -> Result<()> {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);
        let cfg = tiny_full_config();
        let hidden_size = cfg.hidden_size;

        let mut text_model = HunYuanVLTextModel::new(vb, &cfg)?;

        // Prime the KV cache with a sequence.
        let inputs_embeds = Tensor::zeros((1, 2, hidden_size), DType::F32, &dev)?;
        let _ = text_model.forward(&inputs_embeds, None, 0)?;

        // Clearing must not panic.
        text_model.clear_kv_cache();

        // A second forward (offset=0 again) should succeed after cache is cleared.
        let output = text_model.forward(&inputs_embeds, None, 0)?;
        let dims = output.dims();
        assert_eq!(
            dims,
            &[1, 2, hidden_size],
            "Forward after KV-cache clear should produce (1, 2, hidden_size={}); got {:?}",
            hidden_size,
            dims
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // E1-9: Causal mask lower-triangular shape assertion
    // -----------------------------------------------------------------------

    #[test]
    fn should_produce_causal_mask_where_future_positions_are_neg_infinity() -> Result<()> {
        let dev = Device::Cpu;
        let batch = 1usize;
        let seq_len = 3usize;

        let mask = prepare_causal_attention_mask(batch, seq_len, &dev)?;

        let dims = mask.dims();
        assert_eq!(
            dims,
            &[batch, seq_len, seq_len],
            "Causal mask shape should be (batch={}, seq_len={}, seq_len={}); got {:?}",
            batch,
            seq_len,
            seq_len,
            dims
        );

        let data = mask.to_vec3::<f32>()?;
        // Position (0, 0, 1): query 0, key 1 — key is in future → NEG_INFINITY
        assert!(
            data[0][0][1].is_infinite() && data[0][0][1] < 0.0,
            "Mask[0][0][1] should be -inf (future key), got {}",
            data[0][0][1]
        );
        // Position (0, 1, 1): query 1, key 1 — same position → not masked
        assert_eq!(
            data[0][1][1], 0.0,
            "Mask[0][1][1] should be 0.0 (not masked), got {}",
            data[0][1][1]
        );
        // Position (0, 2, 0): query 2, key 0 — past → not masked
        assert_eq!(
            data[0][2][0], 0.0,
            "Mask[0][2][0] should be 0.0 (past key), got {}",
            data[0][2][0]
        );

        Ok(())
    }
}
