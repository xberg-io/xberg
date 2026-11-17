//! GLM-OCR model implementation: Z.ai's Glm4v vision encoder + GLM-4 decoder VLM.
//!
//! GLM-OCR is a 0.9 B-parameter compact vision-language model combining:
//! - **Glm4v vision encoder** (0.4 B) — 24-block transformer with Conv3d patch
//!   embedding (temporal_patch_size=2, patch_size=14), fused `qkv`/`q_norm`/
//!   `k_norm`, 2-D rotary position embeddings, SwiGLU MLP, and a final
//!   `post_layernorm`. See [`vision`].
//! - **Vision merger** — Conv2d 2×2 spatial downsample (stride 2) followed by
//!   a SwiGLU `merger` (`gate_proj` + `up_proj` + `down_proj` + `proj` +
//!   `post_projection_norm`). See [`connector`].
//! - **GLM-4 decoder** (0.5 B) — 16-block sandwich-norm transformer with M-RoPE,
//!   fused `mlp.gate_up_proj`, and a separate `lm_head` (top-level, NOT under
//!   `language_model.*`). See [`decoder`].
//!
//! Supports OCR, table-to-markdown, formula-to-LaTeX, chart-to-JSON, and image
//! captioning via task-specific prompt prefixes ([`GlmOcrTask`]).
//!
//! ## Why a thin in-tree fork?
//!
//! Upstream `candle_transformers` ships text-only `glm4` and has no `glm4v`
//! encoder, so the entire vision + connector + sandwich-norm decoder live in
//! tree. The decoder vendors candle's glm4 with a `forward_embeds()` addition
//! so the engine can splice in vision-projected embeddings instead of relying
//! on the private embedding layer.
//!
//! ## Remaining gaps (Phase 3+)
//!
//! - **MTP next-N predict layer.** Upstream ships `num_nextn_predict_layers: 1`;
//!   the decoder ignores it (vanilla autoregressive generation only).

#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

pub mod connector;
pub mod decoder;
pub mod mtp;
pub mod preprocess;
pub mod tokenizer;
pub mod vision;

const GLM_OCR_REVISION: &str = "ca5d8b3e287e52589e37c28385d9655ee4372f9d";
const GLM_OCR_CONFIG_SHA256: &str = "4e1daf0d8a3f63e58960ac14bcb58b7be96758cad231fb7a1e5fec60f42dcd8c";
const GLM_OCR_TOKENIZER_SHA256: &str = "aa0fd058c73a5718bb191f6672dc16d122ee0147b20c123d1726514298f9968a";
const GLM_OCR_MODEL_SHA256: &str = "a16eb0de98d199293371c560f95f83130d2a2c9612449df16839f08ff9498815";

use serde::{Deserialize, Serialize};

/// Diagnostic helper: at `TRACE` level, log per-stage tensor stats (shape,
/// NaN/Inf counts, min/max/mean) via `tracing`. No-op when trace is disabled.
///
/// Exists to bisect the CPU-vs-CUDA numerical divergence in the GLM-OCR
/// pipeline: the F32 CPU path recognises text correctly, but the F32 CUDA path
/// emits EOS first (empty output), so some op produces garbage/NaN only on CUDA.
pub(crate) fn glm_debug_tensor(label: &str, t: &candle_core::Tensor) {
    if !tracing::enabled!(tracing::Level::TRACE) {
        return;
    }
    let dims = t.dims().to_vec();
    let flat = match t
        .to_dtype(candle_core::DType::F32)
        .and_then(|x| x.flatten_all())
        .and_then(|x| x.to_vec1::<f32>())
    {
        Ok(v) => v,
        Err(e) => {
            tracing::trace!("[glm-debug] {label}: shape={dims:?} (stat error: {e})");
            return;
        }
    };
    let nan = flat.iter().filter(|x| x.is_nan()).count();
    let inf = flat.iter().filter(|x| x.is_infinite()).count();
    let finite: Vec<f32> = flat.iter().copied().filter(|x| x.is_finite()).collect();
    let (min, max, mean) = if finite.is_empty() {
        (f32::NAN, f32::NAN, f32::NAN)
    } else {
        let min = finite.iter().copied().fold(f32::INFINITY, f32::min);
        let max = finite.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mean = finite.iter().sum::<f32>() / finite.len() as f32;
        (min, max, mean)
    };
    tracing::trace!(
        "[glm-debug] {label}: shape={dims:?} n={} nan={nan} inf={inf} min={min:.4} max={max:.4} mean={mean:.4}",
        flat.len()
    );
}

/// Per-region task selection. Each variant corresponds to an upstream prompt
/// prefix understood by the GLM-OCR decoder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlmOcrTask {
    /// Whole-page or text-region OCR.
    #[default]
    Ocr,
    /// Table region → markdown table.
    Table,
    /// Formula region → LaTeX.
    Formula,
    /// Chart region → structured JSON.
    Chart,
    /// Image region → caption.
    Caption,
}

impl GlmOcrTask {
    /// Prompt prefix expected by the GLM-OCR decoder for this task.
    pub fn prompt(&self) -> &'static str {
        match self {
            GlmOcrTask::Ocr => "Text Recognition:",
            GlmOcrTask::Table => "Table to Markdown:",
            GlmOcrTask::Formula => "Formula to LaTeX:",
            GlmOcrTask::Chart => "Chart to JSON:",
            GlmOcrTask::Caption => "Image Caption:",
        }
    }
}

impl std::fmt::Display for GlmOcrTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            GlmOcrTask::Ocr => "ocr",
            GlmOcrTask::Table => "table",
            GlmOcrTask::Formula => "formula",
            GlmOcrTask::Chart => "chart",
            GlmOcrTask::Caption => "caption",
        };
        write!(f, "{}", name)
    }
}

/// Configuration loaded from the HuggingFace `config.json` of the GLM-OCR repo.
///
/// Upstream `config.json` only ships `vision_config`, `text_config`, and a flat
/// set of image-token IDs. `connector_config`, `mtp_config`, and `max_new_tokens`
/// are xberg-side knobs with serde defaults so deserialising the real
/// config still succeeds.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlmOcrConfig {
    pub vision_config: vision::VisionConfig,
    pub text_config: decoder::DecoderConfig,
    /// Derived from `vision_config` when not explicitly set (matches upstream behaviour).
    #[serde(default)]
    pub connector_config: connector::ConnectorConfig,
    #[serde(default)]
    pub mtp_config: mtp::MtpConfig,
    #[serde(default = "default_max_new_tokens")]
    pub max_new_tokens: usize,
    /// Special-token id used as the placeholder for vision-projected tokens
    /// inside the input sequence. Upstream `image_token_id` = 59280.
    #[serde(default = "default_image_token_id")]
    pub image_token_id: u32,
    /// Special-token id wrapping the start of the image region. Upstream
    /// `image_start_token_id` = 59256.
    #[serde(default = "default_image_start_token_id")]
    pub image_start_token_id: u32,
    /// Special-token id wrapping the end of the image region. Upstream
    /// `image_end_token_id` = 59257.
    #[serde(default = "default_image_end_token_id")]
    pub image_end_token_id: u32,
}

fn default_max_new_tokens() -> usize {
    2048
}

fn default_image_token_id() -> u32 {
    59280
}

fn default_image_start_token_id() -> u32 {
    59256
}

fn default_image_end_token_id() -> u32 {
    59257
}

#[cfg(not(target_arch = "wasm32"))]
mod engine {
    use std::sync::Arc;

    use candle_core::{DType, Device, Tensor};
    use candle_nn::VarBuilder;
    use parking_lot::Mutex;
    use tokenizers::Tokenizer;

    use super::mtp;
    use super::{GlmOcrConfig, GlmOcrTask};
    use super::{connector::VisionConnector, decoder::Glm4Decoder, vision::CogVit};
    use super::{preprocess, tokenizer};
    use crate::error::Result;
    use crate::models::image_processor::{DEFAULT_IMAGE_DECODE_BUDGET, decode_image_with_budget};
    use crate::{CandleOcrError, CandleOcrOutput};

    /// GLM-OCR inference engine. Owns the loaded model components and runs
    /// single-image inference. Construct once per (task, device) pair and pool
    /// in the backend layer.
    pub struct GlmOcrEngine {
        pub(crate) vision: Arc<Mutex<CogVit>>,
        pub(crate) connector: Arc<Mutex<VisionConnector>>,
        pub(crate) decoder: Arc<Mutex<Glm4Decoder>>,
        pub(crate) tokenizer: Tokenizer,
        pub(crate) config: GlmOcrConfig,
        pub(crate) task: GlmOcrTask,
        pub(crate) device: Device,
        pub(crate) dtype: DType,
        pub(crate) special: tokenizer::SpecialTokens,
    }

    /// Download and load a pinned GLM-OCR revision's config, tokenizer, and weights. Split out
    /// of [`GlmOcrEngine::new_with_hf`] to keep that function under the workspace line-count
    /// limit. ~keep
    fn load_model_resources<'device>(
        cache_dir: Option<&std::path::Path>,
        revision: &str,
        device: &'device Device,
        dtype: DType,
    ) -> Result<(GlmOcrConfig, Tokenizer, VarBuilder<'device>)> {
        let config_file = crate::download_guard::hf_download(
            "zai-org/GLM-OCR",
            "config.json",
            revision,
            cache_dir,
            super::GLM_OCR_CONFIG_SHA256,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to get config: {}", e)))?;
        let config_str = std::fs::read_to_string(&config_file)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to read config: {}", e)))?;
        let config: GlmOcrConfig = serde_json::from_str(&config_str)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Config parse error: {}", e)))?;

        let tokenizer_file = crate::download_guard::hf_download(
            "zai-org/GLM-OCR",
            "tokenizer.json",
            revision,
            cache_dir,
            super::GLM_OCR_TOKENIZER_SHA256,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to get tokenizer: {}", e)))?;
        let tokenizer = Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Tokenizer load error: {}", e)))?;

        let model_file = crate::download_guard::hf_download(
            "zai-org/GLM-OCR",
            "model.safetensors",
            revision,
            cache_dir,
            super::GLM_OCR_MODEL_SHA256,
        )
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to get model weights: {e}")))?;
        let model_files = [model_file];

        tracing::debug!("Loading GLM-OCR weights from {:?}", model_files);

        #[allow(unsafe_code)]
        let vb = if model_files.len() == 1 {
            unsafe {
                VarBuilder::from_mmaped_safetensors(&[&model_files[0]], dtype, device)
                    .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load safetensors: {}", e)))?
            }
        } else {
            unsafe {
                let file_refs: Vec<&std::path::Path> = model_files.iter().map(|f| f.as_path()).collect();
                VarBuilder::from_mmaped_safetensors(&file_refs, dtype, device)
                    .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load safetensors shards: {}", e)))?
            }
        };

        Ok((config, tokenizer, vb))
    }

    pub(super) fn process_encoded_image<T>(
        image_bytes: &[u8],
        decode_budget: u64,
        process_rgb: impl FnOnce(&image::RgbImage) -> Result<T>,
    ) -> Result<T> {
        let img = decode_image_with_budget(image_bytes, decode_budget, true)?.into_rgb8();
        process_rgb(&img)
    }

    impl GlmOcrEngine {
        /// Immutable Hugging Face revision covered by the built-in checksums.
        pub fn revision() -> &'static str {
            super::GLM_OCR_REVISION
        }

        /// Load weights from HuggingFace Hub and assemble the engine.
        ///
        /// Downloads `config.json`, `tokenizer.json`, and safetensors from the GLM-OCR
        /// HuggingFace repo (there is no separate `preprocessor_config.json` fetch — the
        /// image preprocessor's `patch_size`/`t_patch_size` are derived from the same
        /// `config.json` `vision_config` block the vision encoder uses; the remaining
        /// preprocessing knobs, min/max pixel budget and CLIP mean/std, have no
        /// `config.json` counterpart and stay at their [`preprocess::PreprocessConfig`]
        /// defaults). Constructs the vision encoder, connector, and decoder modules.
        pub fn new(task: GlmOcrTask, device: Device, dtype: DType) -> Result<Self> {
            Self::new_with_hf(task, device, dtype, None, None)
        }

        /// Load the pinned GLM-OCR model with optional Hugging Face cache settings.
        pub fn new_with_hf(
            task: GlmOcrTask,
            device: Device,
            dtype: DType,
            cache_dir: Option<&std::path::Path>,
            revision: Option<&str>,
        ) -> Result<Self> {
            if matches!(dtype, candle_core::DType::BF16) && device.is_metal() {
                return Err(CandleOcrError::InferenceFailed(
                    "BF16 on Metal is unsupported in candle 0.10 (kernel gap). Use DType::F32 instead.".into(),
                ));
            }
            let revision = revision.unwrap_or(super::GLM_OCR_REVISION);
            if revision != super::GLM_OCR_REVISION {
                return Err(CandleOcrError::UnsupportedConfig(format!(
                    "GLM-OCR is checksum-pinned to revision {}; requested {revision}",
                    super::GLM_OCR_REVISION
                )));
            }

            let (config, tokenizer, vb) = load_model_resources(cache_dir, revision, &device, dtype)?;

            let special = tokenizer::resolve_special_tokens(&tokenizer)?;

            let visual_vb = vb.pp("model").pp("visual");

            let vision = CogVit::new(&config.vision_config, visual_vb.clone(), device.clone())
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load vision encoder: {}", e)))?;

            let connector = VisionConnector::new(&config.connector_config, visual_vb)
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load connector: {}", e)))?;

            let mut decoder = Glm4Decoder::new(
                &config.text_config,
                vb.pp("model").pp("language_model"),
                vb.pp("lm_head"),
            )
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load decoder: {}", e)))?;

            // The mmap-backed `VarBuilder` borrows `device`, and every weight it holds has now
            // been loaded into the submodules above, so release it before `device` moves into
            // the engine below. ~keep
            drop(vb);

            decoder.clear_kv_cache();

            tracing::debug!(
                eos_token_id = special.eos,
                image_start = special.image_start,
                image_end = special.image_end,
                image_token = special.image_token,
                "Resolved GLM-OCR special tokens"
            );

            Ok(Self {
                vision: Arc::new(Mutex::new(vision)),
                connector: Arc::new(Mutex::new(connector)),
                decoder: Arc::new(Mutex::new(decoder)),
                tokenizer,
                config,
                task,
                device,
                dtype,
                special,
            })
        }

        /// Run inference over a single image with a specified task and return the recognised content.
        ///
        /// This mirrors `process_image` but uses the supplied task for prompt construction
        /// instead of `self.task`. Encoded input, decoding and RGB conversion share a 100 MiB
        /// budget. Image decode failures return [`CandleOcrError::Image`].
        pub fn process_image_with_task(&self, image_bytes: &[u8], task: GlmOcrTask) -> Result<CandleOcrOutput> {
            process_encoded_image(image_bytes, DEFAULT_IMAGE_DECODE_BUDGET, |img| {
                self.process_rgb_image_with_task(img, task)
            })
        }

        /// Run inference over a single image and return the recognised content.
        ///
        /// Pipeline:
        /// 1. Preprocess image into pixel_values and grid descriptor
        /// 2. Encode with vision encoder to get vision embeddings
        /// 3. Project vision embeddings to text-hidden space
        /// 4. Build text token sequence with image placeholder tokens
        /// 5. Embed text tokens
        /// 6. Splice vision embeddings into the text embedding sequence
        /// 7. Build multimodal position_ids for M-RoPE
        /// 8. Run autoregressive generation with MTP decoding
        /// 9. Decode token sequence to markdown text
        pub fn process_image(&self, image_bytes: &[u8]) -> Result<CandleOcrOutput> {
            self.process_image_with_task(image_bytes, self.task)
        }

        /// Run the vision encoder and connector over `pixel_values`, returning the projected
        /// vision embeddings plus the merged patch grid `(h_merged, w_merged)` and the
        /// resulting image-token count after spatial merge. Split out of
        /// [`Self::process_rgb_image_with_task`] to keep that function under the workspace line-count
        /// limit. ~keep
        fn encode_and_project(
            &self,
            pixel_values: &Tensor,
            h_patches: usize,
            w_patches: usize,
        ) -> Result<(Tensor, usize, usize, usize)> {
            let vision_embeds = {
                let vision = self.vision.lock();
                vision
                    .forward(pixel_values)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Vision encoding: {}", e)))?
            };
            super::glm_debug_tensor("vision_embeds", &vision_embeds);

            let projected = {
                let connector = self.connector.lock();
                connector
                    .forward(&vision_embeds, h_patches, w_patches)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Vision projection: {}", e)))?
            };
            super::glm_debug_tensor("projected", &projected);

            let merge = self.config.connector_config.spatial_merge_size.max(1);
            let h_merged = h_patches / merge;
            let w_merged = w_patches / merge;
            let num_image_tokens_after_merge = h_merged * w_merged;

            Ok((projected, h_merged, w_merged, num_image_tokens_after_merge))
        }

        /// Build the token sequence with image placeholders, embed it, and splice the
        /// projected vision embeddings into the placeholder positions. Returns
        /// `(input_embeds, image_tokens_start)`. Split out of [`Self::process_rgb_image_with_task`] to
        /// keep that function under the workspace line-count limit. ~keep
        fn build_input_embeds(
            &self,
            task: GlmOcrTask,
            num_image_tokens_after_merge: usize,
            projected: &Tensor,
        ) -> Result<(Tensor, usize)> {
            let (input_ids, image_tokens_start) = tokenizer::build_input_ids(
                &self.special,
                &self.tokenizer,
                task.prompt(),
                num_image_tokens_after_merge,
            )?;

            let ids_vec: Vec<i64> = input_ids.iter().map(|&id| id as i64).collect();
            let input_ids_tensor = Tensor::new(ids_vec.as_slice(), &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token tensor creation: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze batch: {}", e)))?;

            let text_embeds = {
                let decoder = self.decoder.lock();
                decoder
                    .embed_tokens(&input_ids_tensor)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Text embedding: {}", e)))?
            };

            let input_embeds = Self::splice_embeddings(
                &text_embeds,
                projected,
                image_tokens_start,
                num_image_tokens_after_merge,
            )?;
            super::glm_debug_tensor("text_embeds", &text_embeds);
            super::glm_debug_tensor("input_embeds", &input_embeds);

            Ok((input_embeds, image_tokens_start))
        }

        /// Build the flattened `(3, 1, seq_len)` M-RoPE position tensor for a vision-prefixed
        /// sequence, and the position assigned to the first decoded text token. Positions
        /// before `image_tokens_start` and after the vision region are plain sequential
        /// offsets; positions inside the vision region encode `(t, h, w)` via
        /// `image_tokens_start` plus the row/column within the merged patch grid. Split out of
        /// [`Self::process_rgb_image_with_task`] to keep that function under the workspace line-count
        /// limit. ~keep
        fn build_mrope_position_ids(
            &self,
            seq_len: usize,
            image_tokens_start: usize,
            num_image_tokens_after_merge: usize,
            h_merged: usize,
            w_merged: usize,
        ) -> Result<(Tensor, u32)> {
            let vision_end = image_tokens_start + num_image_tokens_after_merge;
            let vision_max_offset = h_merged.max(w_merged);
            let post_vision_base = image_tokens_start + vision_max_offset;

            let mut t_positions = Vec::with_capacity(seq_len);
            let mut h_positions = Vec::with_capacity(seq_len);
            let mut w_positions = Vec::with_capacity(seq_len);

            for idx in 0..seq_len {
                if idx < image_tokens_start {
                    let p = idx as u32;
                    t_positions.push(p);
                    h_positions.push(p);
                    w_positions.push(p);
                } else if idx < vision_end {
                    let local = idx - image_tokens_start;
                    let row = local / w_merged;
                    let col = local % w_merged;
                    t_positions.push(image_tokens_start as u32);
                    h_positions.push((image_tokens_start + row) as u32);
                    w_positions.push((image_tokens_start + col) as u32);
                } else {
                    let post_offset = idx - vision_end;
                    let p = (post_vision_base + post_offset) as u32;
                    t_positions.push(p);
                    h_positions.push(p);
                    w_positions.push(p);
                }
            }

            let mut packed: Vec<u32> = Vec::with_capacity(3 * seq_len);
            packed.extend_from_slice(&t_positions);
            packed.extend_from_slice(&h_positions);
            packed.extend_from_slice(&w_positions);
            let prefill_position_ids = Tensor::from_vec(packed, (3, 1, seq_len), &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Position tensor: {}", e)))?;

            let next_text_pos_start = (post_vision_base + (seq_len - vision_end)) as u32;

            Ok((prefill_position_ids, next_text_pos_start))
        }

        /// Run the chosen task on an RGB image decoded and budgeted by the caller.
        pub fn process_rgb_image_with_task(&self, img: &image::RgbImage, task: GlmOcrTask) -> Result<CandleOcrOutput> {
            tracing::debug!(width = img.width(), height = img.height(), task = %task, "GLM-OCR: starting inference");
            // `patch_size`/`t_patch_size` must match the vision encoder's `config.json`-derived
            // values (the encoder rejects pixel_values whose H/W are not multiples of
            // `patch_size`), so they are taken from the same deserialized `VisionConfig` rather
            // than from `PreprocessConfig::default()`. The remaining fields (min/max pixel
            // budget, CLIP mean/std) have no `config.json` counterpart and keep their defaults.
            let preprocess_config = preprocess::PreprocessConfig {
                patch_size: self.config.vision_config.patch_size,
                t_patch_size: self.config.vision_config.temporal_patch_size,
                ..preprocess::PreprocessConfig::default()
            };
            let (pixel_values, grid_thw) =
                preprocess::preprocess_rgb(img, &preprocess_config, &self.device, self.dtype)?;
            super::glm_debug_tensor("pixel_values", &pixel_values);

            let grid_vec = grid_thw
                .to_vec2::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid shape error: {}", e)))?;
            let g = &grid_vec[0];
            let h_patches = g[1] as usize;
            let w_patches = g[2] as usize;

            let (projected, h_merged, w_merged, num_image_tokens_after_merge) =
                self.encode_and_project(&pixel_values, h_patches, w_patches)?;

            let (input_embeds, image_tokens_start) =
                self.build_input_embeds(task, num_image_tokens_after_merge, &projected)?;

            let seq_len = input_embeds
                .dim(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Seq len: {}", e)))?;

            let (prefill_position_ids, next_text_pos_start) = self.build_mrope_position_ids(
                seq_len,
                image_tokens_start,
                num_image_tokens_after_merge,
                h_merged,
                w_merged,
            )?;

            let output_ids = {
                let mut decoder = self.decoder.lock();
                decoder.clear_kv_cache();

                mtp::generate_mrope(
                    &mut decoder,
                    &input_embeds,
                    &prefill_position_ids,
                    &self.config.mtp_config,
                    mtp::GenerationLimits {
                        next_text_pos_start,
                        max_new_tokens: self.config.max_new_tokens,
                        eos_token_ids: &self.special.eos_token_ids,
                    },
                )
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Generation: {}", e)))?
            };

            let output_text = tokenizer::decode_output(&self.tokenizer, &output_ids)?;

            if output_text.trim().is_empty() {
                tracing::warn!(num_output_tokens = output_ids.len(), "GLM-OCR: output is empty");
            } else {
                tracing::debug!(
                    text_len = output_text.len(),
                    num_output_tokens = output_ids.len(),
                    is_markdown = Self::detect_structured_markdown(&output_text),
                    "GLM-OCR: decoding complete"
                );
            }

            Ok(CandleOcrOutput {
                content: output_text.clone(),
                is_structured_markdown: Self::detect_structured_markdown(&output_text),
                confidence: None,
            })
        }

        /// Splice vision embeddings into the text embedding sequence.
        ///
        /// `text_embeds` is `(B, seq, hidden)`; `vision_embeds` is
        /// `(B, num_image_tokens, hidden)`. Replaces the placeholder tokens at
        /// `[image_start, image_start + num_image_tokens)` with the projected
        /// vision embeddings, concatenating `[before, vision, after]` along the
        /// sequence axis (dim 1).
        fn splice_embeddings(
            text_embeds: &Tensor,
            vision_embeds: &Tensor,
            image_start: usize,
            num_image_tokens: usize,
        ) -> Result<Tensor> {
            let (text_b, text_seq, text_hidden) = text_embeds
                .dims3()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Text embeds shape: {}", e)))?;
            let (vision_b, vision_seq, vision_hidden) = vision_embeds
                .dims3()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Vision embeds shape: {}", e)))?;

            if text_b != vision_b {
                return Err(CandleOcrError::InferenceFailed(format!(
                    "Batch size mismatch: text {} vs vision {}",
                    text_b, vision_b
                )));
            }
            if text_hidden != vision_hidden {
                return Err(CandleOcrError::InferenceFailed(format!(
                    "Hidden size mismatch: text {} vs vision {}",
                    text_hidden, vision_hidden
                )));
            }
            if vision_seq != num_image_tokens {
                return Err(CandleOcrError::InferenceFailed(format!(
                    "Vision token count {} does not match expected placeholders {}",
                    vision_seq, num_image_tokens
                )));
            }
            if image_start + num_image_tokens > text_seq {
                return Err(CandleOcrError::InferenceFailed(format!(
                    "Image token range [{}, {}) exceeds sequence length {}",
                    image_start,
                    image_start + num_image_tokens,
                    text_seq
                )));
            }

            let after_start = image_start + num_image_tokens;
            let mut parts: Vec<Tensor> = Vec::with_capacity(3);

            if image_start > 0 {
                parts.push(
                    text_embeds
                        .narrow(1, 0, image_start)
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow before: {}", e)))?,
                );
            }
            parts.push(vision_embeds.clone());
            if after_start < text_seq {
                parts.push(
                    text_embeds
                        .narrow(1, after_start, text_seq - after_start)
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow after: {}", e)))?,
                );
            }

            Tensor::cat(&parts, 1).map_err(|e| CandleOcrError::InferenceFailed(format!("Cat embeddings: {}", e)))
        }

        /// Detect structured markdown (heading, table, fenced code, LaTeX, bullet list) in output text.
        ///
        /// A bullet list requires at least two lines starting with `- ` to avoid
        /// false positives on OCR-produced typography where a stray hyphen prefix
        /// appears on a single line.
        pub(crate) fn detect_structured_markdown(text: &str) -> bool {
            let mut bullet_count: usize = 0;
            for line in text.lines() {
                let t = line.trim_start();
                if t.starts_with("## ") || t.starts_with("# ") {
                    return true;
                }
                if t.starts_with('|') && t.ends_with('|') && t.matches('|').count() >= 2 {
                    return true;
                }
                if t.starts_with("```") || t.starts_with("$$") {
                    return true;
                }
                if t.starts_with("- ") {
                    bullet_count += 1;
                    if bullet_count >= 2 {
                        return true;
                    }
                }
            }
            false
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use engine::GlmOcrEngine;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::engine;
    use crate::CandleOcrError;
    use crate::models::image_processor::DEFAULT_IMAGE_DECODE_BUDGET;

    const PNG_SIDE: u32 = 16;
    const PIXEL: image::Rgb<u8> = image::Rgb([60, 110, 190]);

    pub(super) fn small_rgb_png() -> (Vec<u8>, u64) {
        let image = image::RgbImage::from_pixel(PNG_SIDE, PNG_SIDE, PIXEL);
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let png = encoded.into_inner();
        let encoded_bytes = u64::try_from(png.len()).unwrap();
        let raster_bytes = u64::from(PNG_SIDE).pow(2) * u64::from(image::ColorType::Rgb8.bytes_per_pixel());
        (png, encoded_bytes + raster_bytes - 1)
    }

    #[test]
    fn standalone_glm_encoded_engine_checks_budget_before_inference() {
        let (png, budget) = small_rgb_png();
        let result = engine::process_encoded_image(&png, budget, |_| -> crate::error::Result<()> {
            panic!("the inference continuation ran after a failed budget check")
        });
        let Err(CandleOcrError::Image(image::ImageError::Limits(error))) = result else {
            panic!("valid PNG must exceed the test budget with a typed image limit");
        };
        assert_eq!(error.kind(), image::error::LimitErrorKind::InsufficientMemory);

        let size = engine::process_encoded_image(&png, DEFAULT_IMAGE_DECODE_BUDGET, |img| {
            assert_eq!(*img.get_pixel(0, 0), PIXEL);
            Ok(img.dimensions())
        })
        .unwrap();
        assert_eq!(size, (PNG_SIDE, PNG_SIDE));
    }

    pub(super) fn assert_uses_default_image_budget<T>(run: impl FnOnce() -> crate::error::Result<T>) {
        use crate::models::image_processor::decode_probe;

        let (result, dispatch) = decode_probe::intercept(run);
        assert_eq!(dispatch, Some((104_857_600, true)));
        let Err(CandleOcrError::Image(image::ImageError::Limits(error))) = result else {
            panic!("the public entry point must return the decoder's typed refusal");
        };
        assert_eq!(error.kind(), image::error::LimitErrorKind::InsufficientMemory);
    }

    #[test]
    fn standalone_glm_public_encoded_task_uses_the_default_decode_budget() {
        let (png, _) = small_rgb_png();
        let engine = cpu_engine_with_zero_weights();
        assert_uses_default_image_budget(|| engine.process_image_with_task(&png, super::GlmOcrTask::Table));
    }

    fn cpu_engine_with_zero_weights() -> super::GlmOcrEngine {
        use std::sync::Arc;

        use candle_core::{DType, Device};
        use candle_nn::VarBuilder;
        use parking_lot::Mutex;
        use tokenizers::{Tokenizer, models::bpe::BPE};

        use super::{GlmOcrConfig, GlmOcrEngine, GlmOcrTask, connector, decoder, tokenizer, vision};

        const WIDTH: usize = 8;
        let device = Device::Cpu;
        let weights = VarBuilder::zeros(DType::F32, &device);
        let config = GlmOcrConfig {
            vision_config: vision::VisionConfig {
                hidden_size: WIDTH,
                out_hidden_size: WIDTH,
                intermediate_size: WIDTH,
                num_attention_heads: 1,
                num_hidden_layers: 0,
                ..Default::default()
            },
            text_config: decoder::DecoderConfig {
                hidden_size: WIDTH,
                intermediate_size: WIDTH,
                num_attention_heads: 1,
                num_key_value_heads: 1,
                num_hidden_layers: 0,
                vocab_size: WIDTH,
                max_position_embeddings: WIDTH,
                head_dim: WIDTH,
                rope_parameters: decoder::RopeParameters {
                    mrope_section: Vec::new(),
                    ..Default::default()
                },
                ..Default::default()
            },
            connector_config: connector::ConnectorConfig {
                vision_hidden_size: WIDTH,
                text_hidden_size: WIDTH,
                merger_intermediate_size: WIDTH,
                spatial_merge_size: 2,
            },
            mtp_config: Default::default(),
            max_new_tokens: 0,
            image_token_id: 3,
            image_start_token_id: 1,
            image_end_token_id: 2,
        };
        let vision = vision::CogVit::new(&config.vision_config, weights.clone(), device.clone()).unwrap();
        let connector = connector::VisionConnector::new(&config.connector_config, weights.clone()).unwrap();
        let decoder = decoder::Glm4Decoder::new(&config.text_config, weights.clone(), weights).unwrap();
        GlmOcrEngine {
            vision: Arc::new(Mutex::new(vision)),
            connector: Arc::new(Mutex::new(connector)),
            decoder: Arc::new(Mutex::new(decoder)),
            tokenizer: Tokenizer::new(BPE::default()),
            config,
            task: GlmOcrTask::Ocr,
            device,
            dtype: DType::F32,
            special: tokenizer::SpecialTokens {
                eos: 0,
                eos_token_ids: vec![0],
                image_start: 1,
                image_end: 2,
                image_token: 3,
            },
        }
    }

    #[test]
    fn standalone_glm_default_decode_budget_is_100_mib() {
        const MIB: u64 = 1024 * 1024;
        assert_eq!(DEFAULT_IMAGE_DECODE_BUDGET, 100 * MIB);
    }

    #[test]
    fn detect_structured_markdown_recognises_table() {
        let text = "| a | b |\n|---|---|\n| 1 | 2 |";
        assert!(engine::GlmOcrEngine::detect_structured_markdown(text));
    }

    #[test]
    fn detect_structured_markdown_recognises_heading() {
        assert!(engine::GlmOcrEngine::detect_structured_markdown("## Hello"));
    }

    #[test]
    fn detect_structured_markdown_rejects_plain_text() {
        assert!(!engine::GlmOcrEngine::detect_structured_markdown(
            "just a plain sentence"
        ));
    }

    #[test]
    fn detect_structured_markdown_rejects_single_dash() {
        assert!(!engine::GlmOcrEngine::detect_structured_markdown(
            "- hyphen but not a list"
        ));
    }

    #[test]
    fn detect_structured_markdown_recognises_two_dash_lines() {
        let text = "- first item\n- second item";
        assert!(engine::GlmOcrEngine::detect_structured_markdown(text));
    }
}
