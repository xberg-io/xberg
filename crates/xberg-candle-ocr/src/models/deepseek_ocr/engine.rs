//! DeepSeek-OCR inference engine for generation loop integration.
//!
//! Plugs the DeepSeekOCRModel into the `generate_mrope` generation loop for token decoding.

use std::path::Path;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use tokenizers::Tokenizer;

use crate::CandleOcrError;
use crate::error::Result;
use crate::vendor::aha::InferenceModel;

use super::{config::DeepseekOCRConfig, model::DeepseekOCRModel, processor::DeepseekOCRProcessor};

// DeepSeek-OCR "deepencoder" preprocessing constants, taken from the reference
// (`base_size`, `patch_size`, `downsample_ratio` and the BasicImageTransform in
// deepseek-ai/DeepSeek-OCR's modeling code). The vision towers turn the padded
// global view into a fixed square grid of image tokens; `input_ids` has to carry
// exactly that many image tokens so the vision features scatter into the right slots.

/// Side length the global view is padded to before the SAM + CLIP towers.
const BASE_SIZE: u32 = 1024;
/// ViT patch size and the projector's token-merge downsample.
const PATCH_SIZE: u32 = 16;
const DOWNSAMPLE_RATIO: u32 = 4;
/// Image tokens per side for the global view:
/// `ceil((base_size / patch_size) / downsample_ratio)` = `(1024 / 16) / 4` = 16.
const NUM_QUERIES_BASE: usize = (BASE_SIZE / PATCH_SIZE / DOWNSAMPLE_RATIO) as usize;
/// Channel mean and std for normalization (reference BasicImageTransform uses 0.5).
const IMAGE_MEAN_STD: f32 = 0.5;
/// Default plain-OCR instruction. In the reference this is `"<image>\nFree OCR."`;
/// the `<image>` placeholder is materialized as the image-token run built below, so
/// only the trailing text lives here.
const DEFAULT_OCR_PROMPT: &str = "\nFree OCR.";

/// DeepSeek-OCR inference engine.
///
/// Manages model initialization, input processing, and generation.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug)]
pub struct DeepseekOCREngine {
    /// Inference model.
    model: DeepseekOCRModel,
    /// Input processor.
    processor: DeepseekOCRProcessor,
    /// Tokenizer for text encoding/decoding.
    tokenizer: Tokenizer,
    /// Model configuration.
    config: DeepseekOCRConfig,
    /// Computation device.
    device: Device,
    /// Model version.
    version: usize,
    /// Data type for weights.
    dtype: DType,
}

impl DeepseekOCREngine {
    /// Create a new DeepSeek-OCR engine.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] if model loading or initialization
    /// fails.
    pub fn new(
        vb: VarBuilder,
        config: DeepseekOCRConfig,
        device: &Device,
        version: usize,
        tokenizer: Tokenizer,
    ) -> Result<Self> {
        let dtype = vb.dtype();
        let processor = DeepseekOCRProcessor::new(device, dtype, version)?;
        let model = DeepseekOCRModel::new(vb, config.clone(), version)?;

        Ok(Self {
            model,
            processor,
            tokenizer,
            config,
            device: device.clone(),
            version,
            dtype,
        })
    }

    /// Initialize a DeepSeek-OCR engine from a local model directory.
    ///
    /// Loads the model weights (safetensors), configuration (config.json),
    /// and tokenizer (tokenizer.json) from the specified path.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to the directory containing model files
    /// * `device` - Computation device (CPU, CUDA, Metal, etc.)
    /// * `dtype` - Data type for weights (F32, F16, BF16)
    /// * `version` - DeepSeek-OCR model version (typically 2)
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] if:
    /// - The model directory is inaccessible
    /// - Config or tokenizer files are missing or malformed
    /// - Weight loading fails
    /// - Model initialization fails
    pub fn init(model_path: &str, device: Device, dtype: DType, version: usize) -> Result<Self> {
        let path = Path::new(model_path);

        // Load config.json
        let config_file = path.join("config.json");
        let config_str = std::fs::read_to_string(&config_file)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to read DeepSeek-OCR config: {}", e)))?;
        let config: DeepseekOCRConfig = serde_json::from_str(&config_str)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to parse DeepSeek-OCR config: {}", e)))?;

        // Load tokenizer
        let tokenizer_file = path.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Failed to load DeepSeek-OCR tokenizer: {}", e)))?;

        // Load safetensors weights: try single file first, then sharded via index
        let model_files = {
            let single_file = path.join("model.safetensors");
            if single_file.exists() {
                vec![single_file]
            } else {
                // Try loading sharded weights via index file
                let index_file = path.join("model.safetensors.index.json");
                if !index_file.exists() {
                    return Err(CandleOcrError::ModelLoadFailed(format!(
                        "DeepSeek-OCR weights not found: no model.safetensors or model.safetensors.index.json at {}",
                        path.display()
                    )));
                }

                let index_str = std::fs::read_to_string(&index_file)
                    .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to read safetensors index: {}", e)))?;

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

                if files.is_empty() {
                    return Err(CandleOcrError::ModelLoadFailed(
                        "DeepSeek-OCR safetensors index exists but contains no weight files".to_string(),
                    ));
                }

                // Resolve all shard files relative to model path
                let mut result = Vec::new();
                for filename in files {
                    let shard_path = path.join(&filename);
                    if !shard_path.exists() {
                        return Err(CandleOcrError::ModelLoadFailed(format!(
                            "DeepSeek-OCR shard not found: {}",
                            shard_path.display()
                        )));
                    }
                    result.push(shard_path);
                }
                result
            }
        };

        // SAFETY: We're using mmaped_safetensors with valid, checked file paths.
        // The files are read-only and the lifetime is scoped to this function,
        // ensuring memory safety. The VarBuilder holds the mmap for as long
        // as the weights are in use.
        #[allow(unsafe_code)]
        let vb = {
            let file_refs: Vec<&std::path::Path> = model_files.iter().map(|p| p.as_path()).collect();
            unsafe {
                VarBuilder::from_mmaped_safetensors(&file_refs, dtype, &device).map_err(|e| {
                    CandleOcrError::ModelLoadFailed(format!("Failed to load DeepSeek-OCR weights: {}", e))
                })?
            }
        };

        // Create processor and model
        let processor = DeepseekOCRProcessor::new(&device, dtype, version)?;
        let model = DeepseekOCRModel::new(vb, config.clone(), version)?;

        Ok(Self {
            model,
            processor,
            tokenizer,
            config,
            device,
            version,
            dtype,
        })
    }

    /// Return the tokenizer for text encoding/decoding.
    #[must_use]
    pub fn tokenizer(&self) -> &Tokenizer {
        &self.tokenizer
    }

    /// Return the model configuration.
    #[must_use]
    pub fn config(&self) -> &DeepseekOCRConfig {
        &self.config
    }

    /// Return the data type for weights.
    #[must_use]
    pub fn dtype(&self) -> DType {
        self.dtype
    }

    /// Process an image and return the recognized text.
    ///
    /// Runs the full inference pipeline:
    /// 1. Decode image bytes to DynamicImage
    /// 2. Preprocess to tensor format (images_ori, image_crop, images_seq_mask, images_spatial_crop)
    /// 3. Tokenize prompt
    /// 4. Run autoregressive token generation via forward_initial and forward_step
    /// 5. Decode the output token sequence to text
    ///
    /// # Arguments
    ///
    /// * `image_bytes` - Raw image data (PNG, JPG, etc.)
    /// * `prompt` - Optional prompt override (otherwise uses default OCR prompt)
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] if:
    /// - Image decoding fails
    /// - Preprocessing fails
    /// - Tokenization fails
    /// - Inference fails
    /// - Token decoding fails
    pub fn process_image(&mut self, image_bytes: &[u8], prompt: Option<&str>) -> Result<String> {
        tracing::debug!(
            image_size = image_bytes.len(),
            version = self.version,
            "DeepSeek-OCR: starting inference"
        );
        // 1. Decode image bytes to DynamicImage
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image decode: {}", e)))?;

        let (img_width, img_height) = (img.width(), img.height());
        tracing::debug!(width = img_width, height = img_height, "DeepSeek-OCR: image dimensions");

        // 2. Preprocess the image into DeepSeek-OCR's "deepencoder" inputs. The global
        // view is aspect-preserving-padded to BASE_SIZE and normalized (see the module
        // constants). The test path uses the global view only (no dynamic 640 crops),
        // which the forward's no-crop branch handles.
        let channels = 3usize;
        let image_token_id = self.processor.image_token_id();

        let mean = Tensor::from_slice(&[IMAGE_MEAN_STD; 3], (3, 1, 1), &self.device)
            .and_then(|t| t.to_dtype(self.dtype))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Mean tensor: {}", e)))?;
        let std = Tensor::from_slice(&[IMAGE_MEAN_STD; 3], (3, 1, 1), &self.device)
            .and_then(|t| t.to_dtype(self.dtype))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Std tensor: {}", e)))?;

        // Pad with the mean grey (mean * 255), matching the reference ImageOps.pad(color=mean).
        let pad = (IMAGE_MEAN_STD * 255.0) as u8;
        let global_view =
            crate::vendor::aha::image::resize_with_edge_padding(&img, BASE_SIZE, BASE_SIZE, [pad, pad, pad]);
        let images_ori = crate::vendor::aha::image::img_transform(&global_view, &mean, &std, &self.device, self.dtype)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Global transform: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Global batch: {}", e)))?;

        // No dynamic crops on this path: an empty crop tensor selects the forward's
        // global-only branch (image_crop.sum() == 0).
        let image_crop = Tensor::zeros((0, channels, BASE_SIZE as usize, BASE_SIZE as usize), self.dtype, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image crop tensor: {}", e)))?;
        let images_spatial_crop = Tensor::new(&[[1u32, 1u32]], &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Spatial crop tensor: {}", e)))?;

        // 3. Build input_ids and the image-token mask. The global view is an
        // NUM_QUERIES_BASE x NUM_QUERIES_BASE grid; the reference appends one
        // image_newline per row plus a trailing view_separator, giving
        // `n * (n + 1) + 1` = 16 * 17 + 1 = 273 image tokens (the model inserts the
        // learned image_newline / view_separator at these positions during the
        // forward). Layout: BOS, the image tokens, then the OCR prompt.
        let num_image_tokens = NUM_QUERIES_BASE * (NUM_QUERIES_BASE + 1) + 1;
        let prompt_text = prompt.unwrap_or(DEFAULT_OCR_PROMPT);
        let text_ids: Vec<u32> = self
            .tokenizer
            .encode(prompt_text, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Encode prompt: {}", e)))?
            .get_ids()
            .to_vec();

        let mut ids: Vec<i64> = Vec::with_capacity(1 + num_image_tokens + text_ids.len());
        let mut mask: Vec<u32> = Vec::with_capacity(ids.capacity());
        ids.push(self.config.bos_token_id as i64);
        mask.push(0);
        ids.extend(std::iter::repeat_n(image_token_id as i64, num_image_tokens));
        mask.extend(std::iter::repeat_n(1u32, num_image_tokens));
        ids.extend(text_ids.iter().map(|&t| t as i64));
        mask.extend(std::iter::repeat_n(0u32, text_ids.len()));

        tracing::debug!(
            seq_len = ids.len(),
            num_image_tokens,
            "DeepSeek-OCR: input construction"
        );

        let input_ids = Tensor::new(ids.as_slice(), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Token tensor: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze batch: {}", e)))?;
        let prompt_ids: Vec<i64> = ids;
        let images_seq_mask = Tensor::new(mask.as_slice(), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Seq mask tensor: {}", e)))?;

        // 4. Prepare multimodal data for forward pass
        let mm_data = crate::vendor::aha::MultiModalData::new(vec![
            Some(images_ori),
            Some(image_crop),
            Some(images_seq_mask),
            Some(images_spatial_crop),
        ]);

        // 5. Clear KV cache and run forward pass
        tracing::debug!("DeepSeek-OCR: clearing cache and running forward_initial");
        self.model.clear_kv_cache();

        let mut logits = self
            .model
            .forward_initial(&input_ids, 0, mm_data)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Initial forward: {}", e)))?;

        // 6. Greedy decoding loop, capped so a degenerate image (no stop token) can't
        // run unbounded.
        const MAX_NEW_TOKENS: usize = 128;
        let stop_ids = self.model.stop_token_ids();
        let mut output_tokens = prompt_ids.iter().map(|&id| id as u32).collect::<Vec<_>>();

        tracing::debug!(
            max_tokens = MAX_NEW_TOKENS,
            num_stop_ids = stop_ids.len(),
            "DeepSeek-OCR: starting decoding loop"
        );

        for step in 0..MAX_NEW_TOKENS {
            let seq_len = logits
                .dim(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Output seq len: {}", e)))?;

            // Get last token logits and argmax
            let last_logits = logits
                .narrow(1, seq_len - 1, 1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow last: {}", e)))?;

            // last_logits is [1, 1, vocab]; argmax over the vocab dim yields [1, 1],
            // so squeeze the two length-1 dims to a scalar token id.
            let next_token = last_logits
                .argmax(2)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Argmax: {}", e)))?
                .squeeze(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze seq: {}", e)))?
                .squeeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze batch: {}", e)))?
                .to_scalar::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("To scalar: {}", e)))?;

            output_tokens.push(next_token);

            // Check for stop token
            if stop_ids.contains(&next_token) {
                tracing::debug!(
                    step = step,
                    num_tokens = output_tokens.len(),
                    "DeepSeek-OCR: reached stop token"
                );
                break;
            }

            // Feed the token back and take ITS logits for the next iteration; the
            // seqlen_offset is the token's absolute position (the prompt occupies
            // 0..prompt_len), which drives RoPE. Discarding the step logits (or
            // restarting the offset near zero) re-reads the initial forward's logits
            // every iteration and emits the same token MAX_NEW_TOKENS times.
            let next_token_tensor = Tensor::new(&[next_token as i64], &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Next token tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze next: {}", e)))?;

            logits = self
                .model
                .forward_step(&next_token_tensor, prompt_ids.len() + step)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Forward step {}: {}", step, e)))?;
        }

        // 7. Decode only the newly generated tokens (skip the prompt + image tokens).
        let generated = output_tokens.get(prompt_ids.len()..).unwrap_or(&[]);
        let output_text = self
            .tokenizer
            .decode(generated, true)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Decode error: {}", e)))?
            .trim()
            .to_string();

        if output_text.is_empty() {
            tracing::warn!(num_tokens = output_tokens.len(), "DeepSeek-OCR: output is empty");
        } else {
            tracing::debug!(
                text_len = output_text.len(),
                num_tokens = output_tokens.len(),
                "DeepSeek-OCR: decoding complete"
            );
        }

        Ok(output_text)
    }

    /// Return a mutable reference to the inference model (for integration with generation loop).
    #[must_use]
    pub fn model_mut(&mut self) -> &mut DeepseekOCRModel {
        &mut self.model
    }

    /// Return the input processor.
    #[must_use]
    pub fn processor(&self) -> &DeepseekOCRProcessor {
        &self.processor
    }

    /// Return the computation device.
    #[must_use]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Return the model version.
    #[must_use]
    pub fn version(&self) -> usize {
        self.version
    }
}
