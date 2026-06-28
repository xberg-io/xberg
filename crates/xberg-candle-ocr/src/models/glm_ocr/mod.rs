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

use serde::{Deserialize, Serialize};

/// Diagnostic helper: when `XBERG_GLM_DEBUG` is set, log per-stage tensor stats
/// (shape, NaN/Inf counts, min/max/mean) to stderr. No-op otherwise.
///
/// Exists to bisect the CPU-vs-CUDA numerical divergence in the GLM-OCR
/// pipeline: the F32 CPU path recognises text correctly, but the F32 CUDA path
/// emits EOS first (empty output), so some op produces garbage/NaN only on CUDA.
pub(crate) fn glm_debug_tensor(label: &str, t: &candle_core::Tensor) {
    if std::env::var_os("XBERG_GLM_DEBUG").is_none() {
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
            eprintln!("[glm-debug] {label}: shape={dims:?} (stat error: {e})");
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
    eprintln!(
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

    impl GlmOcrEngine {
        /// Load weights from HuggingFace Hub and assemble the engine.
        ///
        /// Downloads `config.json`, `preprocessor_config.json`, `tokenizer.json`,
        /// and safetensors from the GLM-OCR HuggingFace repo. Constructs the
        /// vision encoder, connector, and decoder modules.
        pub fn new(task: GlmOcrTask, device: Device, dtype: DType) -> Result<Self> {
            // BF16 on Metal is unsupported in candle 0.10 (kernel gap).
            if matches!(dtype, candle_core::DType::BF16) && device.is_metal() {
                return Err(CandleOcrError::InferenceFailed(
                    "BF16 on Metal is unsupported in candle 0.10 (kernel gap). Use DType::F32 instead.".into(),
                ));
            }

            // Initialize HuggingFace API for weight downloads
            let api = hf_hub::api::sync::Api::new()
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("HF API init: {}", e)))?;

            let repo = api.repo(hf_hub::Repo::with_revision(
                "zai-org/GLM-OCR".to_string(),
                hf_hub::RepoType::Model,
                "main".to_string(),
            ));

            // Load and parse config.json
            let config_file = repo
                .get("config.json")
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to get config: {}", e)))?;
            let config_str = std::fs::read_to_string(&config_file)
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to read config: {}", e)))?;
            let config: GlmOcrConfig = serde_json::from_str(&config_str)
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Config parse error: {}", e)))?;

            // The upstream preprocessor_config.json uses key names
            // (`temporal_patch_size`, `merge_size`, nested `size.shortest_edge`)
            // that differ from our flat `PreprocessConfig` schema. The
            // canonical GLM-OCR preprocess defaults already encode the right
            // values, so skip the parse and use `PreprocessConfig::default()`
            // directly. A future pass can wire in custom preprocessor configs
            // if/when fine-tunes ship divergent settings.

            // Load tokenizer
            let tokenizer_file = repo
                .get("tokenizer.json")
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to get tokenizer: {}", e)))?;
            let tokenizer = Tokenizer::from_file(&tokenizer_file)
                .map_err(|e| CandleOcrError::Tokenizer(format!("Tokenizer load error: {}", e)))?;

            // Load model weights: try model.safetensors first, fall back to index if sharded
            let model_files = match repo.get("model.safetensors") {
                Ok(f) => vec![f],
                Err(_) => {
                    // Try loading sharded weights via index file
                    let index_file = repo.get("model.safetensors.index.json").map_err(|e| {
                        CandleOcrError::ModelLoadFailed(format!("Failed to get model.safetensors or index: {}", e))
                    })?;

                    let index_str = std::fs::read_to_string(&index_file).map_err(|e| {
                        CandleOcrError::ModelLoadFailed(format!("Failed to read safetensors index: {}", e))
                    })?;

                    let index: serde_json::Value = serde_json::from_str(&index_str).map_err(|e| {
                        CandleOcrError::ModelLoadFailed(format!("Failed to parse safetensors index: {}", e))
                    })?;

                    // Extract unique weight file names from the index
                    let mut files = std::collections::HashSet::new();
                    if let Some(weights) = index.get("weight_map").and_then(|m| m.as_object()) {
                        for (_key, val) in weights {
                            if let Some(filename) = val.as_str() {
                                files.insert(filename.to_string());
                            }
                        }
                    }

                    // Download all shards
                    let mut result = Vec::new();
                    for filename in files {
                        let shard_file = repo.get(&filename).map_err(|e| {
                            CandleOcrError::ModelLoadFailed(format!("Failed to get shard {}: {}", filename, e))
                        })?;
                        result.push(shard_file);
                    }
                    result
                }
            };

            tracing::debug!("Loading GLM-OCR weights from {:?}", model_files);

            // SAFETY: We're using mmaped_safetensors with valid file paths. The files are read-only
            // and the lifetime is scoped to this function, ensuring memory safety.
            #[allow(unsafe_code)]
            let vb = if model_files.len() == 1 {
                unsafe {
                    VarBuilder::from_mmaped_safetensors(&[&model_files[0]], dtype, &device)
                        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load safetensors: {}", e)))?
                }
            } else {
                // Multiple shards: use from_mmaped_safetensors with all files
                unsafe {
                    let file_refs: Vec<&std::path::Path> = model_files.iter().map(|f| f.as_path()).collect();
                    VarBuilder::from_mmaped_safetensors(&file_refs, dtype, &device).map_err(|e| {
                        CandleOcrError::ModelLoadFailed(format!("Failed to load safetensors shards: {}", e))
                    })?
                }
            };

            // Resolve special tokens from tokenizer, with fallback to canonical IDs
            let special = tokenizer::resolve_special_tokens(&tokenizer)?;

            // Build model components.
            //
            // Upstream weight layout (verified against
            // `model.safetensors` header for zai-org/GLM-OCR):
            //   - Vision encoder + connector share `model.visual.*` root
            //     (vision owns `patch_embed`, `blocks.*`, `post_layernorm`;
            //     connector owns `downsample`, `merger`).
            //   - Decoder trunk lives at `model.language_model.*`.
            //   - LM head is top-level (`lm_head.weight`), NOT nested under
            //     `model.language_model.*` despite that being the apparent parent.
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
        /// instead of `self.task`. Useful for overriding the engine's default task per invocation.
        pub fn process_image_with_task(&self, image_bytes: &[u8], task: GlmOcrTask) -> Result<CandleOcrOutput> {
            self.process_image_inner(image_bytes, task)
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
            self.process_image_inner(image_bytes, self.task)
        }

        fn process_image_inner(&self, image_bytes: &[u8], task: GlmOcrTask) -> Result<CandleOcrOutput> {
            // Load and preprocess image
            let preprocess_config = preprocess::PreprocessConfig::default();
            let (pixel_values, grid_thw) =
                preprocess::preprocess(image_bytes, &preprocess_config, &self.device, self.dtype)?;
            super::glm_debug_tensor("pixel_values", &pixel_values);

            // Extract grid dimensions for token count calculation
            let grid_vec = grid_thw
                .to_vec2::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid shape error: {}", e)))?;
            let g = &grid_vec[0];
            let h_patches = g[1] as usize;
            let w_patches = g[2] as usize;

            // Vision encoder outputs (B, num_image_tokens, vision_hidden)
            let vision_embeds = {
                let vision = self.vision.lock();
                vision
                    .forward(&pixel_values)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Vision encoding: {}", e)))?
            };
            super::glm_debug_tensor("vision_embeds", &vision_embeds);

            // Project to text-hidden space: (B, num_image_tokens, text_hidden) or reduced via downsampling
            let projected = {
                let connector = self.connector.lock();
                connector
                    .forward(&vision_embeds, h_patches, w_patches)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Vision projection: {}", e)))?
            };
            super::glm_debug_tensor("projected", &projected);

            // The connector's downsample Conv2d uses kernel=stride=spatial_merge_size,
            // collapsing (h_patches, w_patches) → (h_patches/merge, w_patches/merge).
            // smart_resize rounds the input dims to multiples of patch_size *
            // temporal_patch_size = 28, so both grid axes are guaranteed even.
            let merge = self.config.connector_config.spatial_merge_size.max(1);
            let h_merged = h_patches / merge;
            let w_merged = w_patches / merge;
            let num_image_tokens_after_merge = h_merged * w_merged;

            // Build input token IDs following the upstream chat template:
            // [gMASK]<sop><|user|>\n<|begin_of_image|><|image|>×N<|end_of_image|>{prompt}<|assistant|>\n
            let (input_ids, image_tokens_start) = tokenizer::build_input_ids(
                &self.special,
                &self.tokenizer,
                task.prompt(),
                num_image_tokens_after_merge,
            )?;

            // Embed text tokens
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

            // Splice vision embeddings into text embeddings
            // Replace placeholder tokens (image_tokens_start..image_tokens_start + num_image_tokens_after_merge)
            // with the projected vision embeddings
            let input_embeds = Self::splice_embeddings(
                &text_embeds,
                &projected,
                image_tokens_start,
                num_image_tokens_after_merge,
            )?;
            super::glm_debug_tensor("text_embeds", &text_embeds);
            super::glm_debug_tensor("input_embeds", &input_embeds);

            // Build 3-axis M-RoPE position_ids: (3, 1, seq_len) where rows are
            // [t, h, w]. Layout per upstream `Glm4vRotaryEmbedding.get_rope_index`:
            //   * tokens before the vision region get t = h = w = index;
            //   * vision-placeholder tokens form a 2-D grid anchored at
            //     `image_tokens_start`: t = image_tokens_start (constant),
            //     h = image_tokens_start + row, w = image_tokens_start + col,
            //     flattened row-major across (h_merged, w_merged);
            //   * tokens after the vision region resume at
            //     `image_tokens_start + max(h_merged, w_merged)` and increment.
            let seq_len = input_embeds
                .dim(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Seq len: {}", e)))?;
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

            // Pack into a single (3, 1, seq_len) tensor.
            let mut packed: Vec<u32> = Vec::with_capacity(3 * seq_len);
            packed.extend_from_slice(&t_positions);
            packed.extend_from_slice(&h_positions);
            packed.extend_from_slice(&w_positions);
            let prefill_position_ids = Tensor::from_vec(packed, (3, 1, seq_len), &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Position tensor: {}", e)))?;

            // First decoded token continues from the end of the prefill range.
            // After the vision region the indices keep climbing; the next
            // decoded token's position is exactly `post_vision_base + (seq_len -
            // vision_end)`, which is one past the last text position in the
            // prefill window.
            let next_text_pos_start = (post_vision_base + (seq_len - vision_end)) as u32;

            // Run autoregressive generation with proper M-RoPE positions.
            let output_ids = {
                let mut decoder = self.decoder.lock();
                decoder.clear_kv_cache();

                mtp::generate_mrope(
                    &mut decoder,
                    &input_embeds,
                    &prefill_position_ids,
                    next_text_pos_start,
                    &self.config.mtp_config,
                    self.config.max_new_tokens,
                    &self.special.eos_token_ids,
                )
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Generation: {}", e)))?
            };

            // Decode output tokens to text
            let output_text = tokenizer::decode_output(&self.tokenizer, &output_ids)?;

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
        // A single line beginning with "- " is OCR typography noise, not a list.
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
