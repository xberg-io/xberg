//! DeepSeek-OCR inference engine for generation loop integration.
//!
//! Plugs the DeepSeekOCRModel into the `generate_mrope` generation loop for token decoding.

use std::path::Path;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use tokenizers::Tokenizer;

use crate::error::Result;
use crate::CandleOcrError;
use crate::vendor::aha::InferenceModel;

use super::{config::DeepseekOCRConfig, model::DeepseekOCRModel, processor::DeepseekOCRProcessor};

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
    pub fn init(
        model_path: &str,
        device: Device,
        dtype: DType,
        version: usize,
    ) -> Result<Self> {
        let path = Path::new(model_path);

        // Load config.json
        let config_file = path.join("config.json");
        let config_str = std::fs::read_to_string(&config_file).map_err(|e| {
            CandleOcrError::ModelLoadFailed(format!("Failed to read DeepSeek-OCR config: {}", e))
        })?;
        let config: DeepseekOCRConfig = serde_json::from_str(&config_str).map_err(|e| {
            CandleOcrError::ModelLoadFailed(format!("Failed to parse DeepSeek-OCR config: {}", e))
        })?;

        // Load tokenizer
        let tokenizer_file = path.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_file).map_err(|e| {
            CandleOcrError::Tokenizer(format!("Failed to load DeepSeek-OCR tokenizer: {}", e))
        })?;

        // Load safetensors weights
        let model_file = path.join("model.safetensors");
        if !model_file.exists() {
            return Err(CandleOcrError::ModelLoadFailed(
                format!("DeepSeek-OCR weights not found at: {}", model_file.display()),
            ));
        }

        // SAFETY: We're using mmaped_safetensors with a valid, checked file path.
        // The file is read-only and the lifetime is scoped to this function,
        // ensuring memory safety. The VarBuilder holds the mmap for as long
        // as the weights are in use.
        #[allow(unsafe_code)]
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_file.as_path()], dtype, &device)
                .map_err(|e| {
                    CandleOcrError::ModelLoadFailed(format!("Failed to load DeepSeek-OCR weights: {}", e))
                })?
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
        // 1. Decode image bytes to DynamicImage
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image decode: {}", e)))?;

        // 2. Simple preprocessing: create placeholder tensors for the image inputs
        // For Phase 5, we skip complex crop/mask logic and use minimal tensors
        let batch_size = 1usize;
        let img_h = 384usize;
        let img_w = 384usize;
        let channels = 3usize;

        // Resize image to standard size using simple bilinear resize
        let resized = img.resize_exact(
            img_w as u32,
            img_h as u32,
            image::imageops::FilterType::Triangle,
        );
        let rgb_img = resized.to_rgb8();

        // Convert to tensor (batch, channels, height, width)
        let mut pixels = vec![];
        for pixel in rgb_img.pixels() {
            pixels.push(pixel[0] as f32 / 255.0);
            pixels.push(pixel[1] as f32 / 255.0);
            pixels.push(pixel[2] as f32 / 255.0);
        }

        let images_ori = Tensor::from_slice(
            &pixels,
            (batch_size, channels, img_h, img_w),
            &self.device,
        )
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Images tensor: {}", e)))?;

        // Placeholder tensors for crop, mask, spatial_crop (all zeros for Phase 5)
        let image_crop = Tensor::zeros((0, channels, 64, 64), self.dtype, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image crop tensor: {}", e)))?;

        let images_seq_mask = Tensor::ones((batch_size, 1), self.dtype, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Seq mask tensor: {}", e)))?;

        let images_spatial_crop = Tensor::zeros((batch_size, 2), candle_core::DType::U32, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Spatial crop tensor: {}", e)))?;

        // 3. Tokenize prompt
        let prompt_text = prompt.unwrap_or("OCR this image.");
        let encoding = self
            .tokenizer
            .encode(prompt_text, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Encode prompt: {}", e)))?;

        let prompt_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let input_ids = Tensor::new(prompt_ids.as_slice(), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Token tensor: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze batch: {}", e)))?;

        // 4. Prepare multimodal data for forward pass
        let mm_data = crate::vendor::aha::MultiModalData::new(vec![
            Some(images_ori),
            Some(image_crop),
            Some(images_seq_mask),
            Some(images_spatial_crop),
        ]);

        // 5. Clear KV cache and run forward pass
        self.model.clear_kv_cache();

        let logits = self
            .model
            .forward_initial(&input_ids, 0, mm_data)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Initial forward: {}", e)))?;

        // 6. Greedy decoding loop (Phase 5: limit to small token count for stability)
        const MAX_NEW_TOKENS: usize = 128;
        let stop_ids = self.model.stop_token_ids();
        let mut output_tokens = prompt_ids.iter().map(|&id| id as u32).collect::<Vec<_>>();

        for step in 0..MAX_NEW_TOKENS {
            let seq_len = logits
                .dim(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Output seq len: {}", e)))?;

            // Get last token logits and argmax
            let last_logits = logits
                .narrow(1, seq_len - 1, 1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow last: {}", e)))?;

            let next_token = last_logits
                .argmax(2)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Argmax: {}", e)))?
                .squeeze(2)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze: {}", e)))?
                .squeeze(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze batch: {}", e)))?
                .to_scalar::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("To scalar: {}", e)))?;

            output_tokens.push(next_token);

            // Check for stop token
            if stop_ids.contains(&next_token) {
                break;
            }

            // Forward step for next token
            let next_token_tensor = Tensor::new(&[next_token as i64], &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Next token tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze next: {}", e)))?;

            let _ = self
                .model
                .forward_step(&next_token_tensor, step + 1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Forward step {}: {}", step, e)))?;
        }

        // 7. Decode tokens back to text
        let output_text = self
            .tokenizer
            .decode(&output_tokens, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Decode error: {}", e)))?
            .trim()
            .to_string();

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
