//! Configuration structures for PaddleOCR-VL model and preprocessing.
//!
//! Adapted from aha's paddleocr_vl module.

use candle_nn::Activation;
use serde::{Deserialize, Serialize};

/// Main model configuration for PaddleOCR-VL.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaddleOCRVLConfig {
    pub compression_ratio: f64,
    pub head_dim: usize,
    pub hidden_act: Activation,
    pub hidden_dropout_prob: f64,
    pub hidden_size: usize,
    pub ignored_index: i32,
    pub image_token_id: u32,
    pub intermediate_size: usize,
    pub max_position_embeddings: usize,
    pub max_sequence_length: Option<usize>,
    pub num_attention_heads: usize,
    pub num_hidden_layers: usize,
    pub num_key_value_heads: usize,
    pub pad_token_id: u32,
    pub rms_norm_eps: f64,
    pub rope_scaling: PaddleOCRVLRopeScalingConfig,
    pub rope_theta: f64,
    pub sliding_window: Option<u32>,
    pub tie_word_embeddings: bool,
    pub torch_dtype: String,
    pub use_bias: bool,
    pub use_cache: bool,
    pub use_flash_attention: bool,
    pub video_token_id: u32,
    pub vision_config: PaddleOCRVLVisionConfig,
    pub vision_start_token_id: u32,
    pub vision_end_token_id: Option<u32>,
    pub vocab_size: usize,
    pub weight_share_add_bias: bool,
    pub use_3d_rope: bool,
    pub rope_is_neox_style: bool,
}

/// RoPE scaling configuration for multi-rope support (M-RoPE).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaddleOCRVLRopeScalingConfig {
    pub mrope_section: Vec<usize>,
    pub rope_type: String,
    #[serde(rename = "type")]
    pub scaling_type: String,
}

/// Vision encoder configuration (SigLIP).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaddleOCRVLVisionConfig {
    pub attention_dropout: f64,
    pub hidden_act: Activation,
    pub hidden_size: usize,
    pub image_size: usize,
    pub intermediate_size: usize,
    pub layer_norm_eps: f64,
    pub num_attention_heads: usize,
    pub num_channels: usize,
    pub num_hidden_layers: usize,
    pub pad_token_id: u32,
    pub patch_size: usize,
    pub spatial_merge_size: usize,
    pub temporal_patch_size: usize,
    pub tokens_per_second: usize,
    pub torch_dtype: String,
}

/// Image preprocessor configuration.
/// Contains 56 fields for JSON-driven image processing (replaces hardcoded constants).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaddleOCRVLPreprocessorConfig {
    pub do_convert_rgb: bool,
    pub do_normalize: bool,
    pub do_rescale: bool,
    pub do_resize: bool,
    pub image_mean: Vec<f64>,
    pub image_std: Vec<f64>,
    pub max_pixels: u32,
    pub merge_size: usize,
    pub min_pixels: u32,
    pub patch_size: usize,
    pub resample: u32,
    pub rescale_factor: f64,
    pub size: Option<SizeConfig>,
    pub temporal_patch_size: usize,
}

/// Size configuration for image resizing.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SizeConfig {
    pub max_pixels: usize,
    pub min_pixels: usize,
}
