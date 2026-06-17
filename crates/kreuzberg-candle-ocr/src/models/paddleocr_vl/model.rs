//! PaddleOCR-VL model implementation: SigLIP vision encoder + ERNIE text decoder with M-RoPE.
//!
//! Adapted from aha's paddleocr_vl module.
//! Handles the complete forward pass for vision-language document understanding.

use candle_core::{D, IndexOp, Shape, Tensor};
use candle_nn::{
    Conv2d, Embedding, LayerNorm, Linear, Module, RmsNorm, VarBuilder, embedding, linear, linear_no_bias, rms_norm,
};

use crate::CandleOcrError;
use crate::error::Result;
use crate::vendor::aha::{
    InferenceModel, MultiModalData,
    modules::{NaiveAttnGateUpDownMLPBlock, NaiveAttnTwoLinearMLPBlock, get_conv2d, get_layer_norm},
    rope::{Qwen2_5VLTextRotaryEmbedding, Qwen2_5VisionRotaryEmbedding},
};

use super::config::{PaddleOCRVLConfig, PaddleOCRVLRopeScalingConfig, PaddleOCRVLVisionConfig};

// Helper function for vision position indexing
fn get_vision_next_indices(input_ids: &Tensor, vision_start_token_id: u32) -> Result<Tensor> {
    let input_vec = input_ids
        .to_vec1::<u32>()
        .map_err(|e| CandleOcrError::InferenceFailed(format!("to_vec1: {}", e)))?;
    let mut indices = Vec::new();
    for (i, &token_id) in input_vec.iter().enumerate() {
        if token_id == vision_start_token_id && i + 1 < input_vec.len() {
            indices.push((i + 1) as u32);
        }
    }
    if indices.is_empty() {
        return Err(CandleOcrError::InferenceFailed(
            "No vision start token found".to_string(),
        ));
    }
    Tensor::new(indices.as_slice(), input_ids.device())
        .map_err(|e| CandleOcrError::InferenceFailed(format!("create tensor: {}", e)))
}

/// Spatial merge projector for vision embeddings.
pub struct Projector {
    merge_size: usize,
    pre_norm: LayerNorm,
    linear_1: Linear,
    linear_2: Linear,
}

impl Projector {
    /// Create a new projector.
    pub fn new(vb: VarBuilder, config: &PaddleOCRVLConfig) -> Result<Self> {
        let merge_size = config.vision_config.spatial_merge_size;
        let hidden_size = config.vision_config.hidden_size * merge_size * merge_size;
        let pre_norm = get_layer_norm(
            vb.pp("pre_norm"),
            config.rms_norm_eps,
            config.vision_config.hidden_size,
            true,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Pre-norm creation: {}", e)))?;
        let linear_1 = linear(hidden_size, hidden_size, vb.pp("linear_1"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Linear 1 creation: {}", e)))?;
        let linear_2 = linear(hidden_size, config.hidden_size, vb.pp("linear_2"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Linear 2 creation: {}", e)))?;

        Ok(Self {
            merge_size,
            pre_norm,
            linear_1,
            linear_2,
        })
    }

    /// Forward pass with spatial merging.
    pub fn forward(&self, xs: &Tensor, image_grid_thw: &Tensor) -> Result<Tensor> {
        let img_num = image_grid_thw
            .dim(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get img_num: {}", e)))?;
        let mut processed_features = vec![];
        let mut start = 0usize;

        for i in 0..img_num {
            let grid_row = image_grid_thw
                .i(i)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid: {}", e)))?
                .to_vec1::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid to_vec1: {}", e)))?;

            if grid_row.len() != 3 {
                return Err(CandleOcrError::InferenceFailed(
                    "grid_thw expected 3 elements".to_string(),
                ));
            }

            let [t, h, w] = [grid_row[0], grid_row[1], grid_row[2]];
            let end = start + (t * h * w) as usize;
            let xs_i = xs
                .i((start..end, ..))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index xs: {}", e)))?;
            let xs_i = self
                .pre_norm
                .forward(&xs_i)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Pre-norm forward: {}", e)))?;

            let dim = xs_i
                .dim(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Get dim: {}", e)))?;

            let shape = Shape::from(vec![
                t as usize,
                h as usize / self.merge_size,
                self.merge_size,
                w as usize / self.merge_size,
                self.merge_size,
                dim,
            ]);

            let xs_i = xs_i
                .reshape((t as usize, h as usize, w as usize, dim))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape 1: {}", e)))?
                .reshape(shape)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape 2: {}", e)))?
                .permute(vec![0, 1, 3, 2, 4, 5])
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Permute: {}", e)))?
                .reshape((
                    (t * h * w) as usize / self.merge_size / self.merge_size,
                    self.merge_size * self.merge_size * dim,
                ))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape 3: {}", e)))?;

            let xs_i = self
                .linear_1
                .forward(&xs_i)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Linear 1 forward: {}", e)))?
                .gelu()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("GELU: {}", e)))?;
            let xs_i = self
                .linear_2
                .forward(&xs_i)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Linear 2 forward: {}", e)))?;

            processed_features.push(xs_i);
            start = end;
        }

        Tensor::cat(&processed_features, 0).map_err(|e| CandleOcrError::InferenceFailed(format!("Cat features: {}", e)))
    }
}

/// SigLIP vision embeddings with patch embedding and position encoding.
#[allow(dead_code)]
pub struct SiglipVisionEmbeddings {
    embed_dim: usize,
    patch_size: usize,
    patch_embedding: Conv2d,
    num_positions: usize,
    position_embedding: Embedding,
    packing_position_embedding: Embedding,
}

impl SiglipVisionEmbeddings {
    /// Create a new vision embeddings layer.
    pub fn new(vb: VarBuilder, config: &PaddleOCRVLVisionConfig) -> Result<Self> {
        let embed_dim = config.hidden_size;
        let image_size = config.image_size;
        let patch_size = config.patch_size;

        let patch_embedding = get_conv2d(
            vb.pp("patch_embedding"),
            config.num_channels,
            embed_dim,
            patch_size,
            0,
            patch_size,
            1,
            1,
            true,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Patch embedding: {}", e)))?;

        let num_positions = (image_size / patch_size).pow(2);
        let position_embedding = embedding(num_positions, embed_dim, vb.pp("position_embedding"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Position embedding: {}", e)))?;
        let packing_position_embedding = embedding(32768, embed_dim, vb.pp("packing_position_embedding"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Packing embedding: {}", e)))?;

        Ok(Self {
            embed_dim,
            patch_size,
            patch_embedding,
            num_positions,
            position_embedding,
            packing_position_embedding,
        })
    }

    /// Forward pass for vision embeddings.
    pub fn forward(
        &self,
        pixel_values: &Tensor,
        position_ids: &Tensor,
        image_grid_thw: &Tensor,
        interpolate_pos_encoding: bool,
    ) -> Result<Tensor> {
        let (bs, seq_len, c, h, w) = pixel_values
            .dims5()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get dims: {}", e)))?;
        let pixel_values = pixel_values
            .reshape((bs * seq_len, c, h, w))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape pixels: {}", e)))?;
        let patch_embeds = self
            .patch_embedding
            .forward(&pixel_values)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Patch embedding: {}", e)))?;

        let mut embeddings = patch_embeds
            .squeeze(D::Minus1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze 1: {}", e)))?
            .squeeze(D::Minus1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze 2: {}", e)))?;

        if interpolate_pos_encoding {
            let mut tmp_embeddings = vec![];
            let img_num = image_grid_thw
                .dim(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Get img_num: {}", e)))?;
            let mut start = 0usize;

            for i in 0..img_num {
                let grid_row = image_grid_thw
                    .i(i)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid: {}", e)))?
                    .to_vec1::<u32>()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid vec1: {}", e)))?;

                if grid_row.len() != 3 {
                    return Err(CandleOcrError::InferenceFailed("grid_thw len != 3".to_string()));
                }

                let [t, h, w] = [grid_row[0], grid_row[1], grid_row[2]];
                let end = start + (t * h * w) as usize;
                let image_embeddings = embeddings
                    .i(start..end)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Index embeds: {}", e)))?;

                // NOTE: interpolate_pos_encoding not implemented; using static position embeddings
                let position_embedding = self
                    .position_embedding
                    .forward(position_ids)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Position embedding: {}", e)))?;
                let image_embeddings = image_embeddings
                    .add(&position_embedding)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Add embeddings: {}", e)))?;
                tmp_embeddings.push(image_embeddings);
                start = end;
            }

            embeddings = Tensor::cat(&tmp_embeddings, 0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat embeddings: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?;
        } else {
            let packing_pos_embed = self
                .packing_position_embedding
                .forward(position_ids)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Packing pos embed: {}", e)))?;
            embeddings = embeddings
                .add(&packing_pos_embed)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Add packing: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze 2: {}", e)))?;
        }

        Ok(embeddings)
    }
}

/// SigLIP encoder with attention and feed-forward layers.
pub struct SiglipEncoder {
    layers: Vec<NaiveAttnTwoLinearMLPBlock>,
    rotary_pos_emb: Qwen2_5VisionRotaryEmbedding,
}

impl SiglipEncoder {
    /// Create a new encoder.
    pub fn new(vb: VarBuilder, config: &PaddleOCRVLVisionConfig) -> Result<Self> {
        let vb_layers = vb.pp("layers");
        let mut layers = vec![];

        for i in 0..config.num_hidden_layers {
            let layer_i = NaiveAttnTwoLinearMLPBlock::new(
                vb_layers.pp(i),
                config.hidden_size,
                config.num_attention_heads,
                None,
                None,
                true,
                "self_attn",
                Some("out_proj"),
                config.intermediate_size,
                config.hidden_act,
                true,
                "mlp",
                "fc1",
                "fc2",
                config.layer_norm_eps,
                "layer_norm1",
                "layer_norm2",
            )
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Layer {}: {}", i, e)))?;
            layers.push(layer_i);
        }

        let head_dim = config.hidden_size / config.num_attention_heads;
        let rotary_pos_emb = Qwen2_5VisionRotaryEmbedding::new(head_dim / 2, Some(10000.0));

        Ok(Self { layers, rotary_pos_emb })
    }

    /// Forward pass with RoPE position embeddings.
    pub fn forward(&self, xs: &Tensor, image_grid_thw: &Tensor) -> Result<Tensor> {
        let mut split_hids = vec![];
        let mut split_wids = vec![];

        for i in 0..image_grid_thw
            .dim(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get img_num: {}", e)))?
        {
            let grid_row = image_grid_thw
                .i(i)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid: {}", e)))?
                .to_vec1::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid vec1: {}", e)))?;

            if grid_row.len() != 3 {
                return Err(CandleOcrError::InferenceFailed("grid_thw len != 3".to_string()));
            }

            let [_t, h, w] = [grid_row[0], grid_row[1], grid_row[2]];
            let pos_w: Vec<u32> = (0..h).flat_map(|_| 0u32..w).collect();
            let pos_w = Tensor::new(pos_w.as_slice(), xs.device())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Pos w tensor: {}", e)))?;
            let pos_h: Vec<u32> = (0..h).flat_map(|h| vec![h; w as usize]).collect();
            let pos_h = Tensor::new(pos_h.as_slice(), xs.device())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Pos h tensor: {}", e)))?;

            split_hids.push(pos_h);
            split_wids.push(pos_w);
        }

        let width_position_ids = Tensor::cat(&split_wids, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat width ids: {}", e)))?;
        let height_position_ids = Tensor::cat(&split_hids, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat height ids: {}", e)))?;

        let max_grid_size = image_grid_thw
            .i((.., 1..))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid: {}", e)))?
            .max_all()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Max grid: {}", e)))?
            .to_scalar::<u32>()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("To scalar: {}", e)))?;

        let rope_emb_max_grid = self
            .rotary_pos_emb
            .forward(max_grid_size as usize, xs.device())
            .map_err(|e| CandleOcrError::InferenceFailed(format!("RoPE forward: {}", e)))?;

        let rotary_pos_emb_h = rope_emb_max_grid
            .index_select(&height_position_ids, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Index height: {}", e)))?;
        let rotary_pos_emb_w = rope_emb_max_grid
            .index_select(&width_position_ids, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Index width: {}", e)))?;

        let rope_emb = Tensor::cat(&[rotary_pos_emb_h, rotary_pos_emb_w], 1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat rope: {}", e)))?
            .contiguous()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Contiguous: {}", e)))?
            .repeat((1, 2))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Repeat: {}", e)))?;

        let cos = rope_emb
            .cos()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cos: {}", e)))?;
        let sin = rope_emb
            .sin()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Sin: {}", e)))?;

        let mut xs = xs.clone();
        for layer in &self.layers {
            xs = layer
                .forward(&xs, Some(&cos), Some(&sin), None, false)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Layer forward: {}", e)))?;
        }

        Ok(xs)
    }
}

/// Complete SigLIP vision model (embeddings + encoder + norm).
pub struct SiglipVisionModel {
    embeddings: SiglipVisionEmbeddings,
    encoder: SiglipEncoder,
    post_layernorm: LayerNorm,
}

impl SiglipVisionModel {
    /// Create a new vision model.
    pub fn new(vb: VarBuilder, config: &PaddleOCRVLVisionConfig) -> Result<Self> {
        let vb = vb.pp("vision_model");
        let embeddings = SiglipVisionEmbeddings::new(vb.pp("embeddings"), config)?;
        let encoder = SiglipEncoder::new(vb.pp("encoder"), config)?;
        let post_layernorm = get_layer_norm(vb.pp("post_layernorm"), config.layer_norm_eps, config.hidden_size, true)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Post-norm: {}", e)))?;

        Ok(Self {
            embeddings,
            encoder,
            post_layernorm,
        })
    }

    /// Forward pass.
    pub fn forward(
        &self,
        pixel_values: &Tensor,
        image_grid_thw: &Tensor,
        position_ids: &Tensor,
        interpolate_pos_encoding: bool,
    ) -> Result<Tensor> {
        let xs = self
            .embeddings
            .forward(pixel_values, position_ids, image_grid_thw, interpolate_pos_encoding)?;
        let xs = self.encoder.forward(&xs, image_grid_thw)?;
        let xs = self
            .post_layernorm
            .forward(&xs)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Post-norm forward: {}", e)))?;
        Ok(xs)
    }
}

/// ERNIE 4.5 text decoder with KV cache and M-RoPE support.
pub struct Ernie4_5Model {
    embed_tokens: Embedding,
    layers: Vec<NaiveAttnGateUpDownMLPBlock>,
    norm: RmsNorm,
    rotary_emb: Qwen2_5VLTextRotaryEmbedding,
    rope_scaling: PaddleOCRVLRopeScalingConfig,
}

impl Ernie4_5Model {
    /// Create a new text decoder.
    pub fn new(vb: VarBuilder, config: &PaddleOCRVLConfig) -> Result<Self> {
        let embed_tokens = embedding(config.vocab_size, config.hidden_size, vb.pp("embed_tokens"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Embed tokens: {}", e)))?;

        let vb_layers = vb.pp("layers");
        let mut layers = vec![];

        for i in 0..config.num_hidden_layers {
            let layer_i = NaiveAttnGateUpDownMLPBlock::new(
                vb_layers.pp(i),
                config.hidden_size,
                config.num_attention_heads,
                Some(config.num_key_value_heads),
                Some(config.head_dim),
                config.use_bias,
                "self_attn",
                None,
                config.intermediate_size,
                config.hidden_act,
                config.use_bias,
                "mlp",
                config.rms_norm_eps,
                "input_layernorm",
                "post_attention_layernorm",
            )
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Layer {}: {}", i, e)))?;
            layers.push(layer_i);
        }

        let norm = rms_norm(config.hidden_size, config.rms_norm_eps, vb.pp("norm"))
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("RMS norm: {}", e)))?;
        let rotary_emb = Qwen2_5VLTextRotaryEmbedding::new(config.head_dim, config.rope_theta as f32);

        Ok(Self {
            embed_tokens,
            layers,
            norm,
            rotary_emb,
            rope_scaling: config.rope_scaling.clone(),
        })
    }

    /// Forward pass with position IDs.
    pub fn forward(
        &mut self,
        inputs_embeds: &Tensor,
        seqlen_offset: usize,
        position_ids: Option<&Tensor>,
    ) -> Result<Tensor> {
        let (b_size, seq_len, _) = inputs_embeds
            .dims3()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get dims: {}", e)))?;

        let position_ids = match position_ids {
            Some(ids) => ids.clone(),
            None => Tensor::arrange(
                seqlen_offset as u32,
                (seq_len + seqlen_offset) as u32,
                inputs_embeds.device(),
            )
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Arrange: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze 1: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze 2: {}", e)))?
            .broadcast_as((3, b_size, seq_len))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Broadcast: {}", e)))?,
        };

        let (cos, sin) = self
            .rotary_emb
            .forward(
                &position_ids,
                inputs_embeds.dtype(),
                self.rope_scaling.mrope_section.clone(),
            )
            .map_err(|e| CandleOcrError::InferenceFailed(format!("RoPE forward: {}", e)))?;

        let mut xs = inputs_embeds.clone();

        for layer in self.layers.iter_mut() {
            xs = layer
                .forward(&xs, &cos, &sin, None)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Layer forward: {}", e)))?;
        }

        let xs = xs
            .apply(&self.norm)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Norm apply: {}", e)))?;

        Ok(xs)
    }

    /// Clear KV cache for all layers.
    pub fn clear_kv_cache(&mut self) {
        for layer in self.layers.iter_mut() {
            layer.clear_kv_cache();
        }
    }
}

/// Complete PaddleOCR-VL model combining vision and text components.
pub struct PaddleOCRVLModel {
    mlp_ar: Projector,
    visual: SiglipVisionModel,
    model: Ernie4_5Model,
    pub cfg: PaddleOCRVLConfig,
    lm_head: Linear,
    rope_deltas: Option<Tensor>,
    stop_token_ids: Vec<u32>,
}

impl PaddleOCRVLModel {
    /// Create a new PaddleOCR-VL model.
    pub fn new(cfg: PaddleOCRVLConfig, vb: VarBuilder, eos_ids: Vec<u32>) -> Result<Self> {
        let mlp_ar = Projector::new(vb.pp("mlp_AR"), &cfg)?;
        let visual = SiglipVisionModel::new(vb.pp("visual"), &cfg.vision_config)?;
        let model = Ernie4_5Model::new(vb.pp("model"), &cfg)?;
        let vocab_size = cfg.vocab_size;

        let lm_head = if cfg.tie_word_embeddings {
            Linear::new(model.embed_tokens.embeddings().clone(), None)
        } else {
            linear_no_bias(cfg.hidden_size, vocab_size, vb.pp("lm_head"))
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("LM head: {}", e)))?
        };

        Ok(Self {
            mlp_ar,
            visual,
            model,
            cfg,
            lm_head,
            rope_deltas: None,
            stop_token_ids: eos_ids,
        })
    }

    /// Compute RoPE indices for vision and text tokens with 3D rope support.
    #[allow(clippy::too_many_lines)]
    pub fn get_rope_index(
        &self,
        input_ids: &Tensor,
        image_grid_thw: Option<&Tensor>,
        _video_grid_thw: Option<&Tensor>,
        mask: Option<&Tensor>,
        _second_per_grid_ts: Option<Vec<f32>>,
    ) -> Result<(Tensor, Tensor)> {
        let spatial_merge_size = self.cfg.vision_config.spatial_merge_size;

        if let Some(image_grid_thw) = image_grid_thw {
            let total_input_ids = input_ids.clone();
            let mask_ = mask.cloned().unwrap_or(
                Tensor::ones_like(&total_input_ids)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Ones: {}", e)))?,
            );

            let mut position_ids = Tensor::ones(
                (3, input_ids.dim(0)?, input_ids.dim(1)?),
                input_ids.dtype(),
                input_ids.device(),
            )
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Position ids: {}", e)))?;

            let mut image_index = 0;
            let mut mrope_position_deltas: Vec<i64> = Vec::new();

            for i in 0..total_input_ids.dim(0)? {
                let input_ids_i = total_input_ids
                    .i(i)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Index input: {}", e)))?;
                let _mask_i = mask_
                    .i(i)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Index mask: {}", e)))?;

                let mut llm_pos_ids_list: Vec<Tensor> = Vec::new();
                let mut text_start = 0u32;
                #[allow(unused_assignments)]
                let mut text_end = 0u32;

                // Try to find vision tokens
                if let Ok(vision_indices) = get_vision_next_indices(&input_ids_i, self.cfg.vision_start_token_id) {
                    let vision_tokens = vision_indices
                        .to_vec1::<u32>()
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Vision vec1: {}", e)))?;

                    for &j in vision_tokens.iter() {
                        if j > 0 {
                            let token_val = input_ids_i
                                .i(j as usize)
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index token: {}", e)))?
                                .to_scalar::<u32>()
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token scalar: {}", e)))?;

                            if token_val == self.cfg.image_token_id {
                                let grid_row = image_grid_thw
                                    .i(image_index)
                                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid: {}", e)))?
                                    .to_vec1::<u32>()
                                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid vec1: {}", e)))?;

                                if grid_row.len() != 3 {
                                    return Err(CandleOcrError::InferenceFailed("grid_thw len != 3".to_string()));
                                }

                                let [_t, h, w] = [grid_row[0], grid_row[1], grid_row[2]];
                                text_end = j;
                                let llm_grid_h = h / spatial_merge_size as u32;
                                let llm_grid_w = w / spatial_merge_size as u32;
                                let text_len = text_end - text_start;

                                let start_idx = if !llm_pos_ids_list.is_empty() {
                                    llm_pos_ids_list[llm_pos_ids_list.len() - 1]
                                        .max_all()
                                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Max: {}", e)))?
                                        .to_scalar::<u32>()
                                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Scalar: {}", e)))?
                                        + 1
                                } else {
                                    0
                                };

                                let pos_ids = Tensor::arrange(start_idx, start_idx + text_len, input_ids_i.device())
                                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Arrange: {}", e)))?
                                    .unsqueeze(0)
                                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?
                                    .broadcast_as((3usize, text_len as usize))
                                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Broadcast: {}", e)))?;

                                llm_pos_ids_list.push(pos_ids);

                                // Vision patch position IDs
                                let h_index = Tensor::arrange(
                                    start_idx + text_len,
                                    start_idx + text_len + llm_grid_h,
                                    input_ids_i.device(),
                                )
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("H arrange: {}", e)))?
                                .unsqueeze(0)
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("H unsqueeze: {}", e)))?
                                .broadcast_as((llm_grid_h as usize, llm_grid_w as usize))
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("H broadcast: {}", e)))?
                                .flatten_all()
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("H flatten: {}", e)))?;

                                let w_index = Tensor::arrange(
                                    start_idx + text_len,
                                    start_idx + text_len + llm_grid_w,
                                    input_ids_i.device(),
                                )
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("W arrange: {}", e)))?
                                .unsqueeze(0)
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("W unsqueeze: {}", e)))?
                                .broadcast_as((llm_grid_h as usize, llm_grid_w as usize))
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("W broadcast: {}", e)))?
                                .flatten_all()
                                .map_err(|e| CandleOcrError::InferenceFailed(format!("W flatten: {}", e)))?;

                                let thw_index = Tensor::stack(&[h_index, w_index], 0)
                                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Stack: {}", e)))?;

                                llm_pos_ids_list.push(thw_index);
                                text_start = text_end + llm_grid_h * llm_grid_w;
                                image_index += 1;
                            }
                        }
                    }
                }

                // Handle trailing text
                let input_len = input_ids_i
                    .dim(0)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Input len: {}", e)))?;
                if (text_start as usize) < input_len {
                    let start_idx = if !llm_pos_ids_list.is_empty() {
                        llm_pos_ids_list[llm_pos_ids_list.len() - 1]
                            .max_all()
                            .map_err(|e| CandleOcrError::InferenceFailed(format!("Max: {}", e)))?
                            .to_scalar::<u32>()
                            .map_err(|e| CandleOcrError::InferenceFailed(format!("Scalar: {}", e)))?
                            + 1
                    } else {
                        0
                    };

                    let text_len = (input_len as u32) - text_start;
                    let pos_ids = Tensor::arrange(start_idx, start_idx + text_len, input_ids_i.device())
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Arrange: {}", e)))?
                        .unsqueeze(0)
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?
                        .broadcast_as((3usize, text_len as usize))
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Broadcast: {}", e)))?;

                    llm_pos_ids_list.push(pos_ids);
                }

                let llm_position = Tensor::cat(&llm_pos_ids_list, 1)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat: {}", e)))?
                    .reshape((3, 1, ()))
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape: {}", e)))?;

                position_ids = position_ids
                    .slice_assign(&[(0..3), (i..i + 1), (0..input_ids.dim(1)?)], &llm_position)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Slice assign: {}", e)))?;

                let position_deltas = llm_position
                    .max_all()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Max: {}", e)))?
                    .to_scalar::<u32>()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Scalar: {}", e)))?
                    as i64
                    + 1
                    - (input_ids_i.dim(0)? as i64);

                mrope_position_deltas.push(position_deltas);
            }

            let mut mrope_position_deltas = Tensor::new(mrope_position_deltas.as_slice(), input_ids.device())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Deltas tensor: {}", e)))?;
            if mrope_position_deltas.rank() == 1 {
                mrope_position_deltas = mrope_position_deltas
                    .unsqueeze(0)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?;
            }

            Ok((position_ids.contiguous()?, mrope_position_deltas))
        } else {
            // No vision: simple text-only position IDs
            let position_ids = Tensor::arrange(0_u32, input_ids.dim(D::Minus1)? as u32, input_ids.device())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Arrange: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze 1: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze 2: {}", e)))?
                .broadcast_as((3, input_ids.dim(0)?, input_ids.dim(D::Minus1)?))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Broadcast: {}", e)))?
                .contiguous()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Contiguous: {}", e)))?;

            let mrope_position_deltas = Tensor::zeros((input_ids.dim(0)?, 1), input_ids.dtype(), input_ids.device())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Zeros: {}", e)))?;

            Ok((position_ids, mrope_position_deltas))
        }
    }

    /// Forward pass for the complete model.
    pub fn forward(
        &mut self,
        input_ids: &Tensor,
        pixel_values: Option<&Tensor>,
        image_grid_thw: Option<&Tensor>,
        image_mask: Option<&Tensor>,
        cache_position: Option<&Tensor>,
        seqlen_offset: usize,
    ) -> Result<Tensor> {
        let mut inputs_embeds = self
            .model
            .embed_tokens
            .forward(input_ids)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Embed forward: {}", e)))?;

        if let (Some(pixel_values), Some(image_grid_thw), Some(image_mask)) = (pixel_values, image_grid_thw, image_mask)
        {
            let pixel_values = pixel_values
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?;

            let mut siglip_position_ids = vec![];
            let img_num = image_grid_thw
                .dim(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Get img_num: {}", e)))?;

            for i in 0..img_num {
                let grid_row = image_grid_thw
                    .i(i)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Index grid: {}", e)))?
                    .to_vec1::<u32>()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid vec1: {}", e)))?;

                if grid_row.len() != 3 {
                    return Err(CandleOcrError::InferenceFailed("grid_thw len != 3".to_string()));
                }

                let [t, h, w] = [grid_row[0], grid_row[1], grid_row[2]];
                let numel = h * w;

                let image_position_ids = Tensor::arrange(0, numel, pixel_values.device())
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Arrange: {}", e)))?
                    .repeat(t as usize)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Repeat: {}", e)))?;

                siglip_position_ids.push(image_position_ids);
            }

            let siglip_position_ids = Tensor::cat(&siglip_position_ids, 0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat ids: {}", e)))?;

            let image_embed = self
                .visual
                .forward(&pixel_values, image_grid_thw, &siglip_position_ids, true)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Vision forward: {}", e)))?
                .squeeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze: {}", e)))?;

            let image_embed = self
                .mlp_ar
                .forward(&image_embed, image_grid_thw)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Projector forward: {}", e)))?;

            // Apply image mask to embed
            let image_mask_f = image_mask
                .to_dtype(inputs_embeds.dtype())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Mask dtype: {}", e)))?
                .unsqueeze(D::Minus1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Mask unsqueeze: {}", e)))?;

            let image_contrib = image_embed
                .broadcast_mul(&image_mask_f)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Mask mul: {}", e)))?;

            inputs_embeds = inputs_embeds
                .add(&image_contrib)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Add image: {}", e)))?;
        }

        let position_ids;
        let rope_deltas;

        if (cache_position.is_some()
            && cache_position
                .unwrap()
                .i(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Index: {}", e)))?
                .to_scalar::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Scalar: {}", e)))?
                == 0)
            || self.rope_deltas.is_none()
        {
            (position_ids, rope_deltas) = self.get_rope_index(input_ids, image_grid_thw, None, None, None)?;
            self.rope_deltas = Some(rope_deltas);
        } else {
            let (bs, seq_len, _) = inputs_embeds
                .dims3()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Get dims: {}", e)))?;

            let delta = if let (Some(cache_position), Some(rope_deltas)) = (cache_position, self.rope_deltas.as_ref()) {
                cache_position
                    .i(0)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Index: {}", e)))?
                    .to_dtype(rope_deltas.dtype())
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Dtype: {}", e)))?
                    .broadcast_add(rope_deltas)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Add: {}", e)))?
                    .contiguous()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Contiguous: {}", e)))?
                    .to_dtype(candle_core::DType::U32)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("U32: {}", e)))?
            } else {
                Tensor::zeros(1, inputs_embeds.dtype(), inputs_embeds.device())
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Zeros: {}", e)))?
            };

            position_ids = Tensor::arrange(0u32, seq_len as u32, input_ids.device())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Arrange: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze 1: {}", e)))?
                .broadcast_as((bs, seq_len))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Broadcast: {}", e)))?
                .broadcast_add(&delta)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Add delta: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze 2: {}", e)))?
                .broadcast_as((3, bs, seq_len))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Broadcast 2: {}", e)))?
                .contiguous()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Contiguous: {}", e)))?;
        }

        let outputs = self
            .model
            .forward(&inputs_embeds, seqlen_offset, Some(&position_ids))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Text forward: {}", e)))?;

        let seq_len = outputs
            .dim(1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get seq_len: {}", e)))?;
        let hidden_state = outputs
            .narrow(1, seq_len - 1, 1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow: {}", e)))?;

        let logits = self
            .lm_head
            .forward(&hidden_state)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("LM head: {}", e)))?;

        Ok(logits)
    }

    /// Clear KV cache.
    pub fn clear_kv_cache(&mut self) {
        self.model.clear_kv_cache();
    }
}

impl InferenceModel for PaddleOCRVLModel {
    fn forward_initial(&mut self, input_ids: &Tensor, seqlen_offset: usize, data: MultiModalData) -> Result<Tensor> {
        if data.data_vec.len() != 4 {
            return Err(CandleOcrError::InferenceFailed(
                "Expected 4 data elements: pixel_values, image_grid_thw, image_mask, cache_position".to_string(),
            ));
        }

        let pixel_values = &data.data_vec[0];
        let image_grid_thw = &data.data_vec[1];
        let image_mask = &data.data_vec[2];
        let cache_position = &data.data_vec[3];

        self.forward(
            input_ids,
            pixel_values.as_ref(),
            image_grid_thw.as_ref(),
            image_mask.as_ref(),
            cache_position.as_ref(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device, Tensor};
    use candle_nn::VarBuilder;

    fn tiny_vision_config() -> PaddleOCRVLVisionConfig {
        PaddleOCRVLVisionConfig {
            attention_dropout: 0.0,
            hidden_act: candle_nn::Activation::Gelu,
            hidden_size: 16,
            image_size: 28,
            intermediate_size: 32,
            layer_norm_eps: 1e-6,
            num_attention_heads: 2,
            num_channels: 3,
            num_hidden_layers: 1,
            pad_token_id: 0,
            patch_size: 14,
            spatial_merge_size: 1,
            temporal_patch_size: 1,
            tokens_per_second: 2,
            torch_dtype: "float32".to_string(),
        }
    }

    fn tiny_model_config() -> PaddleOCRVLConfig {
        PaddleOCRVLConfig {
            compression_ratio: 0.5,
            head_dim: 8,
            hidden_act: candle_nn::Activation::Silu,
            hidden_dropout_prob: 0.0,
            hidden_size: 16,
            ignored_index: -100,
            image_token_id: 100,
            intermediate_size: 32,
            max_position_embeddings: 128,
            max_sequence_length: None,
            num_attention_heads: 2,
            num_hidden_layers: 1,
            num_key_value_heads: 2,
            pad_token_id: 0,
            rms_norm_eps: 1e-6,
            rope_scaling: PaddleOCRVLRopeScalingConfig {
                // mrope_section entries must sum to head_dim / 2 (= 8 / 2 = 4).
                mrope_section: vec![1, 1, 2],
                rope_type: "mrope".to_string(),
                scaling_type: "mrope".to_string(),
            },
            rope_theta: 10000.0,
            sliding_window: None,
            tie_word_embeddings: true,
            torch_dtype: "float32".to_string(),
            use_bias: false,
            use_cache: false,
            use_flash_attention: false,
            video_token_id: 101,
            vision_config: tiny_vision_config(),
            vision_start_token_id: 102,
            vision_end_token_id: Some(103),
            vocab_size: 256,
            weight_share_add_bias: false,
            use_3d_rope: true,
            rope_is_neox_style: true,
        }
    }

    /// SigLIP vision encoder constructs and forward-passes on a synthetic input without error.
    ///
    /// Image size 28 × 28, patch_size 14 → 2 × 2 = 4 patches; hidden_size 16.
    /// The encoder expects 3D input `(1, num_patches, hidden_size)` (batch dimension
    /// inserted by `SiglipVisionEmbeddings::forward`).
    /// Expected output shape: (1, 4, 16).
    #[test]
    fn siglip_encoder_forward_shape_matches_patch_grid() -> crate::error::Result<()> {
        let dev = Device::Cpu;
        let cfg = tiny_vision_config();
        let vb = VarBuilder::zeros(DType::F32, &dev);

        let encoder = SiglipEncoder::new(vb, &cfg)?;

        // SiglipVisionEmbeddings::forward unsqueezes to (1, num_patches, hidden_size).
        let num_patches = 4usize;
        let xs = Tensor::zeros((1, num_patches, cfg.hidden_size), DType::F32, &dev)
            .map_err(|e| crate::CandleOcrError::InferenceFailed(e.to_string()))?;

        // grid_thw: [t=1, h=2, w=2] → 1 image of 2×2 spatial patches
        let grid_thw = Tensor::new(&[[1u32, 2u32, 2u32]], &dev)
            .map_err(|e| crate::CandleOcrError::InferenceFailed(e.to_string()))?;

        let out = encoder.forward(&xs, &grid_thw)?;

        assert_eq!(
            out.dims(),
            &[1, num_patches, cfg.hidden_size],
            "SigLIP encoder output should preserve (1, num_patches, hidden_size) shape"
        );
        Ok(())
    }

    /// ERNIE-4.5 text decoder constructs and forward-passes on synthetic token embeddings.
    ///
    /// Batch=1, seq_len=4, hidden_size=16. Expected output shape: (1, 4, 16).
    #[test]
    fn ernie4_5_decoder_forward_shape_matches_input_sequence() -> crate::error::Result<()> {
        let dev = Device::Cpu;
        let cfg = tiny_model_config();
        let vb = VarBuilder::zeros(DType::F32, &dev);

        let mut decoder = Ernie4_5Model::new(vb, &cfg)?;

        let batch = 1usize;
        let seq_len = 4usize;
        let inputs_embeds = Tensor::zeros((batch, seq_len, cfg.hidden_size), DType::F32, &dev)
            .map_err(|e| crate::CandleOcrError::InferenceFailed(e.to_string()))?;

        let out = decoder.forward(&inputs_embeds, 0, None)?;

        assert_eq!(
            out.dims(),
            &[batch, seq_len, cfg.hidden_size],
            "ERNIE-4.5 decoder output should be (batch, seq_len, hidden_size)"
        );
        Ok(())
    }

    /// `get_rope_index` (text-only path) produces position_ids of shape (3, batch, seq_len)
    /// and mrope_deltas of shape (1, batch) for a pure-text input without vision tokens.
    #[test]
    fn get_rope_index_text_only_path_returns_correct_shapes() -> crate::error::Result<()> {
        let dev = Device::Cpu;
        let cfg = tiny_model_config();
        let vb = VarBuilder::zeros(DType::F32, &dev);

        let model = PaddleOCRVLModel::new(cfg.clone(), vb, vec![2])?;

        let batch = 1usize;
        let seq_len = 6usize;
        // All tokens are plain text (no vision_start_token_id among them).
        let input_ids = Tensor::arrange(0u32, (batch * seq_len) as u32, &dev)
            .map_err(|e| crate::CandleOcrError::InferenceFailed(e.to_string()))?
            .reshape((batch, seq_len))
            .map_err(|e| crate::CandleOcrError::InferenceFailed(e.to_string()))?;

        let (position_ids, mrope_deltas) = model.get_rope_index(&input_ids, None, None, None, None)?;

        assert_eq!(
            position_ids.dims(),
            &[3, batch, seq_len],
            "text-only rope index should be (3, batch, seq_len)"
        );
        assert_eq!(
            mrope_deltas.dims(),
            &[batch, 1],
            "text-only mrope_deltas should be (batch, 1)"
        );

        // The position IDs for text-only should form a simple [0..seq_len) range
        // broadcast across all 3 rope sections.
        let first_row = position_ids
            .i((0usize, 0usize, ..))
            .map_err(|e| crate::CandleOcrError::InferenceFailed(e.to_string()))?
            .to_vec1::<u32>()
            .map_err(|e| crate::CandleOcrError::InferenceFailed(e.to_string()))?;
        let expected: Vec<u32> = (0..seq_len as u32).collect();
        assert_eq!(
            first_row, expected,
            "text-only rope position ids should be [0..seq_len)"
        );
        Ok(())
    }

    /// `PaddleOCRVLConfig` round-trips cleanly through serde JSON serialization/deserialization.
    #[test]
    fn paddleocr_vl_config_round_trips_through_serde_json() {
        let cfg = tiny_model_config();
        let json = serde_json::to_string(&cfg).expect("PaddleOCRVLConfig should serialize to JSON");
        let decoded: PaddleOCRVLConfig =
            serde_json::from_str(&json).expect("PaddleOCRVLConfig should deserialize from JSON");

        assert_eq!(
            decoded.hidden_size, cfg.hidden_size,
            "hidden_size should survive serde round-trip"
        );
        assert_eq!(
            decoded.vocab_size, cfg.vocab_size,
            "vocab_size should survive serde round-trip"
        );
        assert_eq!(
            decoded.vision_config.num_hidden_layers, cfg.vision_config.num_hidden_layers,
            "vision_config.num_hidden_layers should survive serde round-trip"
        );
        assert_eq!(
            decoded.rope_scaling.mrope_section, cfg.rope_scaling.mrope_section,
            "mrope_section should survive serde round-trip"
        );
        assert_eq!(
            decoded.use_3d_rope, cfg.use_3d_rope,
            "use_3d_rope should survive serde round-trip"
        );
    }

    /// `PaddleOCRVLPreprocessorConfig` round-trips cleanly through serde JSON.
    #[test]
    fn paddleocr_vl_preprocessor_config_round_trips_through_serde_json() {
        use super::super::config::PaddleOCRVLPreprocessorConfig;

        let cfg = PaddleOCRVLPreprocessorConfig {
            do_convert_rgb: true,
            do_normalize: true,
            do_rescale: true,
            do_resize: true,
            image_mean: vec![0.485, 0.456, 0.406],
            image_std: vec![0.229, 0.224, 0.225],
            max_pixels: 2_822_400,
            merge_size: 2,
            min_pixels: 147_384,
            patch_size: 14,
            resample: 3,
            rescale_factor: 1.0 / 255.0,
            size: None,
            temporal_patch_size: 1,
        };

        let json = serde_json::to_string(&cfg).expect("PaddleOCRVLPreprocessorConfig should serialize");
        let decoded: PaddleOCRVLPreprocessorConfig =
            serde_json::from_str(&json).expect("PaddleOCRVLPreprocessorConfig should deserialize");

        assert_eq!(decoded.do_normalize, cfg.do_normalize);
        assert_eq!(
            decoded.patch_size, cfg.patch_size,
            "patch_size should survive round-trip"
        );
        assert_eq!(
            decoded.merge_size, cfg.merge_size,
            "merge_size should survive round-trip"
        );
        assert_eq!(
            decoded.max_pixels, cfg.max_pixels,
            "max_pixels should survive round-trip"
        );
        assert_eq!(
            decoded.min_pixels, cfg.min_pixels,
            "min_pixels should survive round-trip"
        );
        assert_eq!(
            decoded.image_mean, cfg.image_mean,
            "image_mean should survive round-trip"
        );
    }
}
