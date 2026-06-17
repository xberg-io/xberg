//! Vision-to-text connector for GLM-OCR.
//!
//! Matches the upstream `zai-org/GLM-OCR` `model.visual.{downsample, merger}`
//! layout (a Conv2d spatial downsample followed by a SwiGLU merger), NOT a
//! plain MLP. The connector accepts pre-merge vision features
//! `(B, N, vision_hidden_size)` and emits `(B, N / spatial_merge_size^2,
//! text_hidden_size)` projected into the decoder hidden space.
//!
//! Upstream tensor layout (`/tmp/glm-ocr-audit/tensors.txt`):
//! - `model.visual.downsample.weight` `[1536, 1024, 2, 2]` (Conv2d, biased)
//! - `model.visual.downsample.bias`   `[1536]`
//! - `model.visual.merger.gate_proj.weight` `[4608, 1536]` (no bias)
//! - `model.visual.merger.up_proj.weight`   `[4608, 1536]` (no bias)
//! - `model.visual.merger.down_proj.weight` `[1536, 4608]` (no bias)
//! - `model.visual.merger.proj.weight`      `[1536, 1536]` (no bias)
//! - `model.visual.merger.post_projection_norm.{weight,bias}` `[1536]`
//!   (LayerNorm, biased)
//!
//! Upstream reference: https://huggingface.co/zai-org/GLM-OCR
//! Config source: https://huggingface.co/zai-org/GLM-OCR/raw/main/config.json

use serde::{Deserialize, Serialize};

use super::vision::VisionConfig;

/// Default SwiGLU intermediate size for the merger, matching upstream
/// `zai-org/GLM-OCR` weights (`[4608, 1536]` gate/up projections). This is
/// NOT the same as `vision_config.intermediate_size` (which sizes the vision
/// transformer MLP, not the merger).
const DEFAULT_MERGER_INTERMEDIATE_SIZE: usize = 4608;

/// Connector parameters. Upstream `config.json` does not store these directly —
/// `vision_hidden_size`, `text_hidden_size`, and `spatial_merge_size` are
/// derived from `vision_config`. `merger_intermediate_size` is intrinsic to
/// the merger and defaults to the upstream weight shape (4608).
/// `from_vision_config()` is the canonical constructor; `Default::default()`
/// reproduces the upstream GLM-OCR-base values for unit tests.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectorConfig {
    /// Vision hidden size (input dimension of the per-patch features).
    pub vision_hidden_size: usize,
    /// Decoder hidden size (output dimension of the merger).
    pub text_hidden_size: usize,
    /// SwiGLU intermediate dimension inside the merger. Distinct from
    /// `vision_config.intermediate_size`; defaults to upstream `4608`.
    #[serde(default = "default_merger_intermediate_size")]
    pub merger_intermediate_size: usize,
    /// 2×2 spatial merge applied by the Conv2d downsample before the SwiGLU
    /// merger. Must match `vision_config.spatial_merge_size`.
    pub spatial_merge_size: usize,
}

fn default_merger_intermediate_size() -> usize {
    DEFAULT_MERGER_INTERMEDIATE_SIZE
}

impl ConnectorConfig {
    /// Derive a connector config from a vision config. The merger intermediate
    /// size is set to the upstream default (4608) because it is intrinsic to
    /// the merger weights and not exposed in `vision_config`.
    pub fn from_vision_config(vision: &VisionConfig) -> Self {
        Self {
            vision_hidden_size: vision.hidden_size,
            text_hidden_size: vision.out_hidden_size,
            merger_intermediate_size: DEFAULT_MERGER_INTERMEDIATE_SIZE,
            spatial_merge_size: vision.spatial_merge_size,
        }
    }
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self::from_vision_config(&VisionConfig::default())
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use candle_core::Tensor;
    use candle_nn::{Conv2d, Conv2dConfig, LayerNorm, Linear, Module, VarBuilder};

    use super::ConnectorConfig;
    use crate::CandleOcrError;
    use crate::error::Result;

    /// Vision-to-text connector: Conv2d spatial downsample followed by a
    /// SwiGLU merger and a LayerNorm.
    ///
    /// Input: `(B, N, vision_hidden_size)` where `N = h_patches * w_patches`
    /// Output: `(B, N / spatial_merge_size^2, text_hidden_size)`
    pub struct VisionConnector {
        /// Conv2d with kernel and stride `spatial_merge_size`, biased.
        downsample: Conv2d,
        /// SwiGLU gate projection, no bias.
        gate_proj: Linear,
        /// SwiGLU up projection, no bias.
        up_proj: Linear,
        /// SwiGLU down projection, no bias.
        down_proj: Linear,
        /// Final output projection, no bias.
        proj: Linear,
        /// Post-projection LayerNorm (weight + bias).
        post_projection_norm: LayerNorm,
        config: ConnectorConfig,
    }

    impl VisionConnector {
        /// Build from a [`VarBuilder`] rooted at `model.visual.` (the same
        /// root the vision encoder uses). Loads `downsample.*` and
        /// `merger.*` as siblings.
        pub fn new(config: &ConnectorConfig, vb: VarBuilder) -> Result<Self> {
            let downsample = candle_nn::conv2d(
                config.vision_hidden_size,
                config.text_hidden_size,
                config.spatial_merge_size,
                Conv2dConfig {
                    stride: config.spatial_merge_size,
                    padding: 0,
                    ..Default::default()
                },
                vb.pp("downsample"),
            )
            .map_err(|e| {
                CandleOcrError::ModelLoadFailed(format!("Failed to load connector downsample Conv2d: {}", e))
            })?;

            let merger_vb = vb.pp("merger");

            let gate_proj = candle_nn::linear_no_bias(
                config.text_hidden_size,
                config.merger_intermediate_size,
                merger_vb.pp("gate_proj"),
            )
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load merger gate_proj: {}", e)))?;

            let up_proj = candle_nn::linear_no_bias(
                config.text_hidden_size,
                config.merger_intermediate_size,
                merger_vb.pp("up_proj"),
            )
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load merger up_proj: {}", e)))?;

            let down_proj = candle_nn::linear_no_bias(
                config.merger_intermediate_size,
                config.text_hidden_size,
                merger_vb.pp("down_proj"),
            )
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load merger down_proj: {}", e)))?;

            let proj =
                candle_nn::linear_no_bias(config.text_hidden_size, config.text_hidden_size, merger_vb.pp("proj"))
                    .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load merger proj: {}", e)))?;

            let post_projection_norm = candle_nn::layer_norm(
                config.text_hidden_size,
                candle_nn::LayerNormConfig::default(),
                merger_vb.pp("post_projection_norm"),
            )
            .map_err(|e| {
                CandleOcrError::ModelLoadFailed(format!("Failed to load merger post_projection_norm: {}", e))
            })?;

            Ok(Self {
                downsample,
                gate_proj,
                up_proj,
                down_proj,
                proj,
                post_projection_norm,
                config: config.clone(),
            })
        }

        /// Project pre-merge vision features into the decoder hidden space.
        ///
        /// `vision_embeds` shape: `(B, N, vision_hidden_size)` where
        /// `N = h_patches * w_patches`.
        /// Output: `(B, N / spatial_merge_size^2, text_hidden_size)`.
        pub fn forward(&self, vision_embeds: &Tensor, h_patches: usize, w_patches: usize) -> Result<Tensor> {
            let (batch, num_tokens, vision_hidden) = vision_embeds
                .dims3()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Invalid input shape: {}", e)))?;

            if vision_hidden != self.config.vision_hidden_size {
                return Err(CandleOcrError::InferenceFailed(format!(
                    "Vision hidden mismatch: expected {}, got {}",
                    self.config.vision_hidden_size, vision_hidden
                )));
            }

            if h_patches * w_patches != num_tokens {
                return Err(CandleOcrError::InferenceFailed(format!(
                    "Patch grid mismatch: h_patches * w_patches = {} but N = {}",
                    h_patches * w_patches,
                    num_tokens
                )));
            }

            let merge = self.config.spatial_merge_size;
            if merge == 0 || !h_patches.is_multiple_of(merge) || !w_patches.is_multiple_of(merge) {
                return Err(CandleOcrError::InferenceFailed(format!(
                    "Patch grid ({}, {}) not divisible by spatial_merge_size {}",
                    h_patches, w_patches, merge
                )));
            }

            // (B, N, C_in) -> (B, h_patches, w_patches, C_in) -> (B, C_in, H, W)
            let x = vision_embeds
                .reshape((batch, h_patches, w_patches, vision_hidden))
                .and_then(|t| t.permute([0, 3, 1, 2]))
                .and_then(|t| t.contiguous())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Connector reshape to (B,C,H,W): {}", e)))?;

            // Conv2d downsample: (B, C_in, H, W) -> (B, C_out, H/merge, W/merge)
            let x = self
                .downsample
                .forward(&x)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Connector downsample Conv2d forward: {}", e)))?;

            let (_b, c_out, h_merged, w_merged) = x
                .dims4()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Connector downsample output shape: {}", e)))?;

            // (B, C_out, H', W') -> (B, H', W', C_out) -> (B, N', C_out)
            let new_num_tokens = h_merged * w_merged;
            let x = x
                .permute([0, 2, 3, 1])
                .and_then(|t| t.contiguous())
                .and_then(|t| t.reshape((batch, new_num_tokens, c_out)))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Connector reshape to (B,N',C): {}", e)))?;

            // Upstream Glm4vVisionPatchMerger.forward order (transformers
            // modeling_glm4v.py lines 126-129):
            //   hidden = proj(hidden)
            //   hidden = gelu(post_projection_norm(hidden))
            //   return down_proj(silu(gate_proj(hidden)) * up_proj(hidden))
            //
            // i.e. proj -> LayerNorm -> GELU -> SwiGLU. The earlier version of
            // this function inverted the entire flow — SwiGLU -> down_proj ->
            // proj -> LayerNorm — which scrambles vision features beyond any
            // useful signal. Match upstream exactly.
            let x = x
                .apply(&self.proj)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Merger proj: {}", e)))?;

            let x = x
                .apply(&self.post_projection_norm)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Merger post_projection_norm: {}", e)))?;

            // Upstream `Glm4vVisionPatchMerger.act1 = nn.GELU()` defaults to
            // `approximate='none'`, which is the exact erf-based GELU. Candle's
            // `.gelu()` is tanh-approximate; use `.gelu_erf()` for the exact form.
            let x = x
                .gelu_erf()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Merger post_projection_norm gelu_erf: {}", e)))?;

            let gate = x
                .apply(&self.gate_proj)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Merger gate_proj: {}", e)))?;
            let up = x
                .apply(&self.up_proj)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Merger up_proj: {}", e)))?;
            let hidden = gate
                .silu()
                .and_then(|g| g.mul(&up))
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Merger SwiGLU activation: {}", e)))?;

            let x = hidden
                .apply(&self.down_proj)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Merger down_proj: {}", e)))?;

            Ok(x)
        }

        /// Back-compat shim for the engine wiring pass. Assumes a square patch
        /// grid (`N = sqrt(N) * sqrt(N)`) and delegates to [`Self::forward`].
        /// Will be removed once `mod.rs` is updated to pass the real
        /// `(h_patches, w_patches)`.
        #[deprecated(
            note = "Use `forward(vision_embeds, h_patches, w_patches)`; the engine-wiring pass will remove this shim."
        )]
        pub fn forward_compat(&self, vision_embeds: &Tensor) -> Result<Tensor> {
            let (_batch, num_tokens, _hidden) = vision_embeds
                .dims3()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Invalid input shape: {}", e)))?;

            let side = (num_tokens as f64).sqrt() as usize;
            if side * side != num_tokens {
                return Err(CandleOcrError::InferenceFailed(format!(
                    "forward_compat requires a square patch grid; got N = {}",
                    num_tokens
                )));
            }

            self.forward(vision_embeds, side, side)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use imp::VisionConnector;
