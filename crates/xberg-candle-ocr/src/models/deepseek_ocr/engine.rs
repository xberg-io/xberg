//! DeepSeek-OCR inference engine for generation loop integration.
//!
//! Plugs the DeepSeekOCRModel into the `generate_mrope` generation loop for token decoding.

use std::path::Path;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use tokenizers::Tokenizer;

use crate::CandleOcrError;
use crate::error::Result;
use crate::models::image_processor::{DEFAULT_IMAGE_DECODE_BUDGET, decode_image_with_budget};
use crate::vendor::aha::InferenceModel;

use super::{config::DeepseekOCRConfig, model::DeepseekOCRModel, processor::DeepseekOCRProcessor};

/// Side length the global view is padded to before the SAM + CLIP towers.
const BASE_SIZE: u32 = 1024;
/// ViT patch size and the projector's token-merge downsample.
const PATCH_SIZE: u32 = 16;
const DOWNSAMPLE_RATIO: u32 = 4;
/// Image tokens per side for the global view:
/// `ceil((base_size / patch_size) / downsample_ratio)` = `(1024 / 16) / 4` = 16.
const NUM_QUERIES_BASE: usize = (BASE_SIZE / PATCH_SIZE / DOWNSAMPLE_RATIO) as usize;
/// Local-crop tile side length, matching the reference "Gundam" mode.
const CROP_TILE_SIZE: u32 = 640;
/// Minimum number of local-crop tiles considered by the aspect-ratio search.
const CROP_MIN_TILES: u32 = 2;
/// Maximum number of local-crop tiles considered by the aspect-ratio search.
const CROP_MAX_TILES: u32 = 9;
/// Image tokens per side for a single local-crop tile:
/// `ceil((crop_tile_size / patch_size) / downsample_ratio)` = `(640 / 16) / 4` = 10.
const NUM_QUERIES_LOCAL: usize = (CROP_TILE_SIZE / PATCH_SIZE / DOWNSAMPLE_RATIO) as usize;
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

/// Resolve the safetensors file(s) for a DeepSeek-OCR model directory: a single
/// `model.safetensors`, or every shard named in `model.safetensors.index.json`. Split out of
/// [`DeepseekOCREngine::init`] to keep that function under the workspace line-count limit. ~keep
fn resolve_model_files(path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let single_file = path.join("model.safetensors");
    if single_file.exists() {
        return Ok(vec![single_file]);
    }

    let index_file = path.join("model.safetensors.index.json");
    if !index_file.exists() {
        return Err(CandleOcrError::ModelLoadFailed(format!(
            "DeepSeek-OCR weights not found: no model.safetensors or model.safetensors.index.json at {}",
            path.display()
        )));
    }

    let index_str = std::fs::read_to_string(&index_file)
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to read safetensors index: {}", e)))?;

    let index: serde_json::Value = serde_json::from_str(&index_str)
        .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to parse safetensors index: {}", e)))?;

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
    Ok(result)
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

        let config_file = path.join("config.json");
        let config_str = std::fs::read_to_string(&config_file)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to read DeepSeek-OCR config: {}", e)))?;
        let config: DeepseekOCRConfig = serde_json::from_str(&config_str)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to parse DeepSeek-OCR config: {}", e)))?;

        let tokenizer_file = path.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Failed to load DeepSeek-OCR tokenizer: {}", e)))?;

        let model_files = resolve_model_files(path)?;

        #[allow(unsafe_code)]
        let vb = {
            let file_refs: Vec<&std::path::Path> = model_files.iter().map(|p| p.as_path()).collect();
            unsafe {
                VarBuilder::from_mmaped_safetensors(&file_refs, dtype, &device).map_err(|e| {
                    CandleOcrError::ModelLoadFailed(format!("Failed to load DeepSeek-OCR weights: {}", e))
                })?
            }
        };

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

    /// Build the local-crop tensors (`image_crop`, `images_spatial_crop`) and the resulting
    /// image-token count for `img`, choosing between the tiled "Gundam" grid and the
    /// global-view-only path per [`crop_grid_for`]. Split out of [`Self::process_image`] to
    /// keep that function under the workspace line-count limit. ~keep
    fn build_crop_tensors(
        &self,
        img: &image::DynamicImage,
        mean: &Tensor,
        std: &Tensor,
    ) -> Result<(Tensor, Tensor, usize)> {
        let channels = 3usize;
        match crop_grid_for(img.width(), img.height()) {
            Some(_) => {
                let (tiles, (w_crop, h_crop)) = crate::vendor::aha::image::dynamic_preprocess(
                    img,
                    CROP_MIN_TILES,
                    CROP_MAX_TILES,
                    CROP_TILE_SIZE,
                    false,
                )
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Local-crop tiling: {}", e)))?;

                tracing::debug!(w_crop, h_crop, num_tiles = tiles.len(), "DeepSeek-OCR: local-crop grid");

                let mut tile_tensors = Vec::with_capacity(tiles.len());
                for tile in &tiles {
                    let tile_tensor =
                        crate::vendor::aha::image::img_transform(tile, mean, std, &self.device, self.dtype)
                            .map_err(|e| CandleOcrError::InferenceFailed(format!("Crop transform: {}", e)))?;
                    tile_tensors.push(tile_tensor);
                }
                let image_crop = Tensor::stack(&tile_tensors, 0)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Crop stack: {}", e)))?;
                let images_spatial_crop = Tensor::new(&[[w_crop, h_crop]], &self.device)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Spatial crop tensor: {}", e)))?;

                Ok((image_crop, images_spatial_crop, image_token_count(w_crop, h_crop)))
            }
            None => {
                let image_crop = Tensor::zeros(
                    (0, channels, BASE_SIZE as usize, BASE_SIZE as usize),
                    self.dtype,
                    &self.device,
                )
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Image crop tensor: {}", e)))?;
                let images_spatial_crop = Tensor::new(&[[1u32, 1u32]], &self.device)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Spatial crop tensor: {}", e)))?;

                Ok((image_crop, images_spatial_crop, image_token_count(0, 0)))
            }
        }
    }

    /// Build the prompt/image token id sequence (`input_ids`, `prompt_ids`) and its
    /// image/text `images_seq_mask`. Split out of [`Self::process_image`] to keep that function
    /// under the workspace line-count limit. ~keep
    fn build_input_tensors(
        &self,
        prompt: Option<&str>,
        num_image_tokens: usize,
        image_token_id: u32,
    ) -> Result<(Tensor, Vec<i64>, Tensor)> {
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

        Ok((input_ids, prompt_ids, images_seq_mask))
    }

    /// Autoregressive decode loop: repeatedly sample the next token from `logits`, feed it back
    /// through `forward_step`, and stop at a model stop token, a degenerate repeat, or
    /// `max_new_tokens`. Returns the full token sequence (`prompt_ids` prefix included). Split
    /// out of [`Self::process_image`] to keep that function under the workspace line-count
    /// limit. ~keep
    fn decode_loop(&mut self, mut logits: Tensor, prompt_ids: &[i64]) -> Result<Vec<u32>> {
        let max_new_tokens = self.config.max_new_tokens;
        let stop_ids = self.model.stop_token_ids();
        let mut output_tokens = prompt_ids.iter().map(|&id| id as u32).collect::<Vec<_>>();

        tracing::debug!(
            max_tokens = max_new_tokens,
            num_stop_ids = stop_ids.len(),
            "DeepSeek-OCR: starting decoding loop"
        );

        for step in 0..max_new_tokens {
            let seq_len = logits
                .dim(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Output seq len: {}", e)))?;

            let last_logits = logits
                .narrow(1, seq_len - 1, 1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow last: {}", e)))?;

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

            if stop_ids.contains(&next_token) {
                tracing::debug!(
                    step = step,
                    num_tokens = output_tokens.len(),
                    "DeepSeek-OCR: reached stop token"
                );
                break;
            }

            if let Some(period) = crate::generation::stop_if_degenerate(&mut output_tokens) {
                tracing::warn!(period, step, "DeepSeek-OCR: degenerate repetition, stopping");
                break;
            }

            let next_token_tensor = Tensor::new(&[next_token as i64], &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Next token tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze next: {}", e)))?;

            logits = self
                .model
                .forward_step(&next_token_tensor, prompt_ids.len() + step)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Forward step {}: {}", step, e)))?;
        }

        Ok(output_tokens)
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
        let img = decode_image_with_budget(image_bytes, DEFAULT_IMAGE_DECODE_BUDGET, false)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image decode: {e}")))?;
        self.process_decoded_image(&img, prompt)
    }

    /// Run recognition on a raster already decoded and budgeted by the caller.
    pub fn process_decoded_image(&mut self, img: &image::DynamicImage, prompt: Option<&str>) -> Result<String> {
        let (img_width, img_height) = (img.width(), img.height());
        tracing::debug!(width = img_width, height = img_height, "DeepSeek-OCR: image dimensions");

        let image_token_id = self.processor.image_token_id();

        let mean = Tensor::from_slice(&[IMAGE_MEAN_STD; 3], (3, 1, 1), &self.device)
            .and_then(|t| t.to_dtype(self.dtype))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Mean tensor: {}", e)))?;
        let std = Tensor::from_slice(&[IMAGE_MEAN_STD; 3], (3, 1, 1), &self.device)
            .and_then(|t| t.to_dtype(self.dtype))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Std tensor: {}", e)))?;

        let pad = (IMAGE_MEAN_STD * 255.0) as u8;
        let global_view =
            crate::vendor::aha::image::resize_with_edge_padding(img, BASE_SIZE, BASE_SIZE, [pad, pad, pad]);
        let images_ori = crate::vendor::aha::image::img_transform(&global_view, &mean, &std, &self.device, self.dtype)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Global transform: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Global batch: {}", e)))?;

        let (image_crop, images_spatial_crop, num_image_tokens) = self.build_crop_tensors(img, &mean, &std)?;
        let (input_ids, prompt_ids, images_seq_mask) =
            self.build_input_tensors(prompt, num_image_tokens, image_token_id)?;

        let mm_data = crate::vendor::aha::MultiModalData::new(vec![
            Some(images_ori),
            Some(image_crop),
            Some(images_seq_mask),
            Some(images_spatial_crop),
        ]);

        tracing::debug!("DeepSeek-OCR: clearing cache and running forward_initial");
        self.model.clear_kv_cache();

        let logits = self
            .model
            .forward_initial(&input_ids, 0, mm_data)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Initial forward: {}", e)))?;

        let output_tokens = self.decode_loop(logits, &prompt_ids)?;

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

/// Number of image tokens the model consumes for a page whose local-crop grid is
/// `(w_crop, h_crop)`. `(0, 0)` is the global-view-only path (no local crops), matching the
/// `image_crop.sum_all() == 0` branch `DeepseekOCRModel::forward` takes in that case.
fn image_token_count(w_crop: u32, h_crop: u32) -> usize {
    let global = NUM_QUERIES_BASE * (NUM_QUERIES_BASE + 1) + 1;
    let local = (NUM_QUERIES_LOCAL * w_crop as usize + 1) * (NUM_QUERIES_LOCAL * h_crop as usize);
    global + local
}

/// Local-crop grid `(width_tiles, height_tiles)` the reference "Gundam" mode selects for a
/// `width x height` page, mirroring `dynamic_preprocess`'s aspect-ratio search. Returns `None`
/// when both sides already fit the `CROP_TILE_SIZE` global view and tiling would add nothing.
fn crop_grid_for(width: u32, height: u32) -> Option<(u32, u32)> {
    if width <= CROP_TILE_SIZE && height <= CROP_TILE_SIZE {
        return None;
    }
    let aspect_ratio = f64::from(width) / f64::from(height);
    let target_ratios = crate::vendor::aha::image::generate_target_ratios_sorted(CROP_MIN_TILES, CROP_MAX_TILES);
    Some(crate::vendor::aha::image::find_closest_aspect_ratio(
        aspect_ratio,
        &target_ratios,
        width,
        height,
        CROP_TILE_SIZE,
    ))
}

#[cfg(test)]
mod helper_tests {
    use super::{crop_grid_for, image_token_count};

    #[test]
    fn image_token_count_matches_global_only_path() {
        assert_eq!(image_token_count(0, 0), 273);
    }

    #[test]
    fn image_token_count_matches_letter_page_local_crops() {
        assert_eq!(image_token_count(2, 3), 903);
    }

    #[test]
    fn crop_grid_for_returns_none_for_a_small_image() {
        assert_eq!(crop_grid_for(600, 600), None);
    }

    #[test]
    fn crop_grid_for_selects_2x3_for_a_letter_page_at_150dpi() {
        assert_eq!(crop_grid_for(1275, 1650), Some((2, 3)));
    }
}
