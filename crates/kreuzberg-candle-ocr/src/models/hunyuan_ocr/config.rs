// Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.

use candle_nn::Activation;
use serde::{Deserialize, Serialize};

/// Hunyuan-VL main model configuration.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HunYuanVLConfig {
    /// Enable bias in attention projections.
    pub attention_bias: bool,
    /// Attention dropout probability.
    pub attention_dropout: f64,
    /// Dimension of each attention head.
    pub attention_head_dim: usize,
    /// Beginning-of-sequence token ID.
    pub bos_token_id: u32,
    /// End-of-document token ID.
    pub eod_token_id: u32,
    /// End-of-sequence token ID.
    pub eos_token_id: u32,
    /// Head dimension for attention.
    pub head_dim: usize,
    /// Hidden activation function.
    pub hidden_act: Activation,
    /// Hidden layer size.
    pub hidden_size: usize,
    /// Image start token ID.
    pub image_start_token_id: u32,
    /// Image end token ID.
    pub image_end_token_id: u32,
    /// Image token ID.
    pub image_token_id: u32,
    /// Image newline token ID.
    pub image_newline_token_id: u32,
    /// Weight initialization range.
    pub initializer_range: f64,
    /// Intermediate (feed-forward) layer size.
    pub intermediate_size: usize,
    /// Maximum position embeddings.
    pub max_position_embeddings: usize,
    /// Enable bias in MLP layers.
    pub mlp_bias: bool,
    /// Normalization type.
    pub norm_type: String,
    /// Number of attention heads.
    pub num_attention_heads: usize,
    /// Number of experts (for MoE).
    pub num_experts: usize,
    /// Number of hidden layers.
    pub num_hidden_layers: usize,
    /// Number of key-value attention heads.
    pub num_key_value_heads: usize,
    /// Original vocabulary size.
    pub org_vocab_size: usize,
    /// Pad token ID.
    pub pad_id: i32,
    /// Pad token ID (alternative).
    pub pad_token_id: i32,
    /// Pretraining tensor parallel factor.
    pub pretraining_tp: i32,
    /// RMS normalization epsilon.
    pub rms_norm_eps: f64,
    /// RoPE (Rotary Position Embedding) scaling configuration.
    pub rope_scaling: HunYuanVLRopeScaling,
    /// RoPE base frequency.
    pub rope_theta: f64,
    /// Routed scaling factor (for experts).
    pub routed_scaling_factor: f64,
    /// Separator token ID.
    pub sep_token_id: u32,
    /// Text end token ID.
    pub text_end_id: u32,
    /// Text start token ID.
    pub text_start_id: u32,
    /// Tie word embeddings between input and output.
    pub tie_word_embeddings: bool,
    /// Data type (typically "float32" or "bfloat16").
    pub dtype: String,
    /// Enable KV cache usage.
    pub use_cache: bool,
    /// Apply QK layer normalization.
    pub use_qk_norm: bool,
    /// Use CLA (Contextual Layer Attention).
    pub use_cla: bool,
    /// Vision transformer configuration.
    pub vision_config: HunYuanVLVisionConfig,
    /// Vocabulary size.
    pub vocab_size: usize,
}

/// RoPE scaling configuration with multi-dimensional position embeddings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HunYuanVLRopeScaling {
    /// Alpha parameter for RoPE scaling.
    pub alpha: f64,
    /// Beta fast parameter.
    pub beta_fast: i32,
    /// Beta slow parameter.
    pub beta_slow: i32,
    /// Scaling factor.
    pub factor: f64,
    /// M-scale parameter.
    pub mscale: f64,
    /// M-scale for all dimensions.
    pub mscale_all_dim: f64,
    /// RoPE scaling type (e.g., "linear" or "dynamic").
    #[serde(rename = "type")]
    pub type_field: String,
    /// Dimensions for cross-dimensional RoPE sections.
    pub xdrope_section: Vec<usize>,
}

/// Vision transformer (ViT) configuration for Hunyuan-VL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HunYuanVLVisionConfig {
    /// Add bias to patch embeddings.
    pub add_patchemb_bias: bool,
    /// Attention dropout probability.
    pub attention_dropout: f64,
    /// Extra tokens to concatenate.
    pub cat_extra_token: i32,
    /// Hidden activation function.
    pub hidden_act: Activation,
    /// Hidden layer dropout probability.
    pub hidden_dropout: f64,
    /// Hidden layer size.
    pub hidden_size: usize,
    /// Maximum image tokens.
    pub img_max_token_num: usize,
    /// Intermediate (feed-forward) layer size.
    pub intermediate_size: usize,
    /// Interpolation mode for resizing (e.g., "bilinear").
    pub interpolate_mode: String,
    /// Maximum image size in pixels.
    pub max_image_size: usize,
    /// Maximum ViT sequence length.
    pub max_vit_seq_len: usize,
    /// Number of attention heads.
    pub num_attention_heads: usize,
    /// Number of image channels (typically 3 for RGB).
    pub num_channels: usize,
    /// Number of hidden layers.
    pub num_hidden_layers: usize,
    /// Output hidden layer size (projection dimension).
    pub out_hidden_size: usize,
    /// Patch size (patches are square).
    pub patch_size: usize,
    /// RMS normalization epsilon.
    pub rms_norm_eps: f64,
    /// Spatial merge size for patch merging.
    pub spatial_merge_size: usize,
}

/// Generation configuration for Hunyuan-OCR.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HunyuanOCRGenerationConfig {
    /// Beginning-of-sequence token ID.
    pub bos_token_id: usize,
    /// Pad token ID.
    pub pad_token_id: usize,
    /// Enable sampling.
    pub do_sample: bool,
    /// End-of-sequence token IDs (may include multiple valid EOS markers).
    pub eos_token_id: Vec<u32>,
    /// Nucleus sampling probability threshold.
    pub top_p: f32,
    /// Top-K sampling limit.
    pub top_k: usize,
    /// Temperature for sampling.
    pub temperature: f32,
    /// Repetition penalty.
    pub repetition_penalty: f32,
}

/// Preprocessor configuration for Hunyuan-OCR image preprocessing.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HunyuanOCRPreprocessorConfig {
    /// Minimum number of pixels for an image.
    pub min_pixels: usize,
    /// Maximum number of pixels for an image.
    pub max_pixels: usize,
    /// Patch size.
    pub patch_size: usize,
    /// Resample filter.
    pub resample: usize,
    /// Temporal patch size (for video/multi-frame input).
    pub temporal_patch_size: usize,
    /// Merge size for spatial merging.
    pub merge_size: usize,
    /// Image normalization mean (per-channel).
    pub image_mean: Vec<f32>,
    /// Image normalization standard deviation (per-channel).
    pub image_std: Vec<f32>,
}
