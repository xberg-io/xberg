//! DeepSeek-OCR model configuration.
//!
//! Defines configuration structures for the DeepSeek-OCR vision-language model,
//! including vision encoder (SAM + ViT), CLIP projector, and Qwen2 language decoder.

/// DeepSeek V2 language decoder configuration.
///
/// Configuration for the Mixture-of-Experts language model variant used in DeepSeek-OCR v2.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct DeepseekV2Config {
    /// BOS (beginning of sequence) token ID.
    pub bos_token_id: u32,
    /// EOS (end of sequence) token ID.
    pub eos_token_id: u32,
    /// Number of initial dense layers before MoE routing.
    pub first_k_dense_replace: usize,
    /// Hidden/embedding dimension.
    pub hidden_size: usize,
    /// Feed-forward intermediate dimension.
    pub intermediate_size: usize,
    /// Key-value LoRA rank (optional compression for attention cache).
    pub kv_lora_rank: Option<usize>,
    /// Whether to use language model head projection.
    pub lm_head: bool,
    /// Maximum sequence length for position embeddings.
    pub max_position_embeddings: usize,
    /// Intermediate dimension for MoE experts.
    pub moe_intermediate_size: usize,
    /// Frequency of MoE layers: every N-th layer is MoE.
    #[serde(default = "default_moe_layer_freq")]
    pub moe_layer_freq: usize,
    /// Scaling factor for routed expert outputs.
    #[serde(default = "default_routed_scaling_factor")]
    pub routed_scaling_factor: f64,
    /// Scoring function for expert routing: "softmax" or "sigmoid".
    #[serde(default = "default_scoring_func")]
    pub scoring_func: String,
    /// Weight for auxiliary loss in load balancing.
    #[serde(default = "default_aux_loss_alpha")]
    pub aux_loss_alpha: f32,
    /// Whether to use sequence-level auxiliary loss.
    #[serde(default = "default_true")]
    pub seq_aux: bool,
    /// Normalize top-k probabilities during routing.
    #[serde(default = "default_false")]
    pub norm_topk_prob: bool,
    /// Number of expert groups.
    pub n_group: usize,
    /// Number of routed experts (not shared).
    pub n_routed_experts: usize,
    /// Number of shared experts (always active).
    pub n_shared_experts: usize,
    /// Number of attention heads.
    pub num_attention_heads: usize,
    /// Number of experts to route to per token.
    pub num_experts_per_tok: usize,
    /// Number of transformer decoder layers.
    pub num_hidden_layers: usize,
    /// Number of key-value cache heads (for grouped query attention).
    pub num_key_value_heads: usize,
    /// Query LoRA rank (optional compression).
    pub q_lora_rank: Option<usize>,
    /// Dimension of non-RoPE query-key embedding.
    pub qk_nope_head_dim: usize,
    /// Dimension of RoPE query-key embedding.
    pub qk_rope_head_dim: usize,
    /// Whether to include reward model head.
    pub rm_head: bool,
    /// Top-k group selection size.
    pub topk_group: usize,
    /// Top-k selection method: "greedy" or "prob".
    pub topk_method: String,
    /// Torch data type: "torch.float32" or similar.
    pub torch_dtype: String,
    /// Whether to use Multi-head Latent Attention.
    pub use_mla: bool,
    /// Dimension of value embeddings.
    pub v_head_dim: usize,
    /// Vocabulary size.
    pub vocab_size: usize,
    /// Epsilon for RMSNorm.
    #[serde(default = "default_rms_norm_eps")]
    pub rms_norm_eps: f64,
}

fn default_moe_layer_freq() -> usize {
    1
}
fn default_routed_scaling_factor() -> f64 {
    1.0
}
fn default_scoring_func() -> String {
    "softmax".to_string()
}
fn default_aux_loss_alpha() -> f32 {
    0.001
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_rms_norm_eps() -> f64 {
    1e-6
}

/// Vision-to-language projection configuration.
///
/// Configuration for the linear projection layer that adapts vision encoder
/// output to the language model's embedding dimension.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ProjectorConfig {
    /// Input dimension (from vision encoder output).
    pub input_dim: usize,
    /// Model type identifier (informational).
    pub model_type: String,
    /// Output embedding dimension.
    pub n_embed: usize,
    /// Projector type: "mlp", "linear", etc.
    pub projector_type: String,
}

/// CLIP L14/224 vision encoder configuration.
///
/// Configuration for the CLIP-L/14 vision transformer used as vision encoder.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ClipL14_224 {
    /// Number of attention heads.
    pub heads: usize,
    /// Input image size (square).
    pub image_size: usize,
    /// Number of transformer layers.
    pub layers: usize,
    /// Patch size (for patch embedding).
    pub patch_size: usize,
    /// Hidden embedding width.
    pub width: usize,
}

/// SAM ViT-B vision encoder configuration.
///
/// Configuration for the Segment Anything Model ViT-B visual encoder.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SamVitB {
    /// Output channel dimensions for downsampling blocks.
    pub downsample_channels: Vec<usize>,
    /// Layer indices with global attention (vs. local window attention).
    pub global_attn_indexes: Vec<usize>,
    /// Number of attention heads.
    pub heads: usize,
    /// Number of transformer layers.
    pub layers: usize,
    /// Hidden embedding width.
    pub width: usize,
}

/// Qwen2 0.5B configuration placeholder.
///
/// Marker struct for Qwen2 0.5B variant selection in vision config.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Qwen2_0_5B {
    /// Hidden dimension.
    dim: usize,
}

/// Vision encoder variant selection.
///
/// Specifies which vision encoder to use: CLIP-L/14, SAM ViT-B,
/// or Qwen2 0.5B (for DeepSeek-OCR v2).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Width {
    /// CLIP L14/224 configuration (if used).
    #[serde(rename = "clip-l-14-224")]
    pub clip_l_14_224: Option<ClipL14_224>,
    /// Qwen2 0.5B configuration (if used, for v2).
    #[serde(rename = "qwen2-0-5b")]
    pub qwen2_0_5b: Option<Qwen2_0_5B>,
    /// SAM ViT-B configuration (always present).
    pub sam_vit_b: SamVitB,
}

/// Vision encoder configuration for DeepSeek-OCR.
///
/// Defines the vision pipeline: image size, MLP ratio for attention blocks,
/// and choice of vision encoder architecture (SAM + ViT or SAM + Qwen2).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct DeepseekOCRVisionConfig {
    /// Input image size (square).
    pub image_size: usize,
    /// MLP hidden/intermediate ratio in attention blocks.
    pub mlp_ratio: f32,
    /// Vision encoder variant configuration.
    pub width: Width,
}

/// Complete DeepSeek-OCR model configuration.
///
/// Top-level configuration combining language (DeepSeek V2 or Qwen2),
/// vision (SAM + ViT/Qwen2), and projection configurations.
///
/// DeepSeek-OCR v1 uses SAM + ViT for vision and DeepSeek V2 for language.
/// DeepSeek-OCR v2 uses SAM + Qwen2 for vision and DeepSeek V2/Qwen2 for language.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct DeepseekOCRConfig {
    /// Language decoder configuration.
    pub language_config: DeepseekV2Config,
    /// Vision-to-language projection configuration.
    pub projector_config: ProjectorConfig,
    /// Torch data type for model weights.
    pub torch_dtype: String,
    /// Vision encoder configuration.
    pub vision_config: DeepseekOCRVisionConfig,
    /// BOS token ID (copied from language_config).
    pub bos_token_id: u32,
    /// EOS token ID (copied from language_config).
    pub eos_token_id: u32,
    /// Number of initial dense layers before MoE.
    pub first_k_dense_replace: u32,
    /// Hidden dimension.
    pub hidden_size: usize,
    /// MLP intermediate dimension.
    pub intermediate_size: usize,
    /// Key-value LoRA rank.
    pub kv_lora_rank: Option<usize>,
    /// Whether to use LM head.
    pub lm_head: bool,
    /// Maximum position embeddings.
    pub max_position_embeddings: usize,
    /// MoE intermediate size.
    pub moe_intermediate_size: usize,
    /// Number of expert groups.
    pub n_group: usize,
    /// Number of routed experts.
    pub n_routed_experts: usize,
    /// Number of shared experts.
    pub n_shared_experts: usize,
    /// Number of attention heads.
    pub num_attention_heads: usize,
    /// Number of experts per token.
    pub num_experts_per_tok: usize,
    /// Number of hidden layers.
    pub num_hidden_layers: usize,
    /// Number of key-value heads.
    pub num_key_value_heads: usize,
    /// Query LoRA rank.
    pub q_lora_rank: Option<usize>,
    /// QK non-RoPE dimension.
    pub qk_nope_head_dim: usize,
    /// QK RoPE dimension.
    pub qk_rope_head_dim: usize,
    /// Whether to use RM head.
    pub rm_head: bool,
    /// Top-k group size.
    pub topk_group: usize,
    /// Top-k method.
    pub topk_method: String,
    /// Whether to use MLA.
    pub use_mla: bool,
    /// Value head dimension.
    pub v_head_dim: usize,
    /// Vocabulary size.
    pub vocab_size: usize,
}
