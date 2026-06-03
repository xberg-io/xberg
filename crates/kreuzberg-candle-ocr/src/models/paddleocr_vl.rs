//! PaddleOCR-VL model implementation: Vision-Language model for document parsing.
//!
//! PaddleOCR-VL is a compact vision-language model (0.9B parameters) that combines
//! a NaViT-style visual encoder with ERNIE-4.5-0.3B for document understanding.
//!
//! Supports multiple OCR tasks:
//! - Text recognition (OCR)
//! - Table recognition
//! - Formula recognition
//! - Chart recognition

#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::{CandleOcrError, CandleOcrOutput};

#[cfg(not(target_arch = "wasm32"))]
use candle_core::{DType, Device, Tensor};
#[cfg(not(target_arch = "wasm32"))]
use candle_nn::VarBuilder;
#[cfg(not(target_arch = "wasm32"))]
use candle_transformers::models::paddleocr_vl::{Config, PaddleOCRVLModel};
#[cfg(not(target_arch = "wasm32"))]
use parking_lot::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tokenizers::Tokenizer;

/// PaddleOCR-VL task selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaddleOcrVlTask {
    /// Text recognition (OCR)
    #[default]
    Ocr,
    /// Table recognition
    Table,
    /// Formula recognition
    Formula,
    /// Chart recognition
    Chart,
}

impl PaddleOcrVlTask {
    /// Prompt text for this task as expected by the model.
    pub fn prompt(&self) -> &'static str {
        match self {
            PaddleOcrVlTask::Ocr => "OCR:",
            PaddleOcrVlTask::Table => "Table Recognition:",
            PaddleOcrVlTask::Formula => "Formula Recognition:",
            PaddleOcrVlTask::Chart => "Chart Recognition:",
        }
    }
}

impl std::fmt::Display for PaddleOcrVlTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            PaddleOcrVlTask::Ocr => "ocr",
            PaddleOcrVlTask::Table => "table",
            PaddleOcrVlTask::Formula => "formula",
            PaddleOcrVlTask::Chart => "chart",
        };
        write!(f, "{}", name)
    }
}

/// PaddleOCR-VL engine for inference.
///
/// Wraps the model and tokenizer for a specific task.
#[cfg(not(target_arch = "wasm32"))]
pub struct PaddleOcrVlEngine {
    model: Arc<Mutex<PaddleOCRVLModel>>,
    tokenizer: Tokenizer,
    config: Config,
    task: PaddleOcrVlTask,
    device: Device,
    dtype: DType,
    bos_token_id: u32,
    eos_token_id: u32,
}

#[cfg(not(target_arch = "wasm32"))]
impl PaddleOcrVlEngine {
    /// Create a new PaddleOCR-VL engine for the given task and device.
    pub fn new(task: PaddleOcrVlTask, device: Device, dtype: DType) -> Result<Self> {
        // Download model weights from HuggingFace
        let api = hf_hub::api::sync::Api::new()
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("HF API init: {}", e)))?;

        let repo = api.repo(hf_hub::Repo::with_revision(
            "PaddlePaddle/PaddleOCR-VL".to_string(),
            hf_hub::RepoType::Model,
            "main".to_string(),
        ));

        // Load config
        let config_file = repo
            .get("config.json")
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to get config: {}", e)))?;
        let config_str = std::fs::read_to_string(&config_file)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to read config: {}", e)))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Config parse error: {}", e)))?;

        // Load tokenizer
        let tokenizer_file = repo
            .get("tokenizer.json")
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to get tokenizer: {}", e)))?;
        let tokenizer = Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Tokenizer load error: {}", e)))?;

        // Load model weights
        let model_file = match repo.get("model.safetensors") {
            Ok(f) => f,
            Err(_) => repo
                .get("pytorch_model.bin")
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to get model weights: {}", e)))?,
        };

        tracing::debug!("Loading weights from {:?}", model_file);
        // SAFETY: We're using mapped_safetensors with a valid file path. The file is read-only
        // and the lifetime is scoped to this function, ensuring memory safety.
        #[allow(unsafe_code)]
        let vb = if model_file.extension().is_some_and(|ext| ext == "bin") {
            VarBuilder::from_pth(&model_file, dtype, &device)
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load pth: {}", e)))?
        } else {
            unsafe {
                VarBuilder::from_mapped_safetensors(&[&model_file], dtype, &device)
                    .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load safetensors: {}", e)))?
            }
        };

        let model = PaddleOCRVLModel::new(&config, vb)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Model init error: {}", e)))?;

        // The upstream `paddleocr_vl::Config` struct does not expose bos/eos token ids
        // (only vision-token ids). Resolve from the tokenizer's vocab once at load time
        // and cache on the engine so the decode loop does not repeat name lookups.
        let bos_token_id = tokenizer.token_to_id("<|begin_of_sentence|>").unwrap_or(1);
        let eos_token_id = tokenizer
            .token_to_id("</s>")
            .or_else(|| tokenizer.token_to_id("<|end_of_sentence|>"))
            .or_else(|| tokenizer.token_to_id("<|endoftext|>"))
            .unwrap_or(2);
        tracing::debug!(
            bos_token_id,
            eos_token_id,
            "Resolved PaddleOCR-VL special tokens from tokenizer"
        );

        Ok(PaddleOcrVlEngine {
            model: Arc::new(Mutex::new(model)),
            tokenizer,
            config,
            task,
            device,
            dtype,
            bos_token_id,
            eos_token_id,
        })
    }

    /// Process an image and return the recognized text as markdown.
    pub fn process_image(&self, image_bytes: &[u8]) -> Result<CandleOcrOutput> {
        // Load and preprocess image
        let (pixel_values, grid_thw) = self.load_and_preprocess_image(image_bytes)?;

        // Build input tokens for the task
        let grid_vec = grid_thw
            .to_vec2::<u32>()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid shape error: {}", e)))?;
        let g = &grid_vec[0];
        let h_patches = g[1] as usize;
        let w_patches = g[2] as usize;
        let spatial_merge = self.config.vision_config.spatial_merge_size;
        let num_image_tokens = (h_patches / spatial_merge) * (w_patches / spatial_merge);

        let input_ids = self.build_input_tokens(num_image_tokens)?;

        let max_length = 4096; // Maximum new tokens to generate

        {
            let mut model = self.model.lock();
            model.clear_kv_cache();
        }

        let generated_tokens = {
            let mut model = self.model.lock();
            model
                .generate(&input_ids, &pixel_values, &grid_thw, max_length, self.eos_token_id)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Generation error: {}", e)))?
        };

        // Decode tokens to text
        let output_text = self
            .tokenizer
            .decode(&generated_tokens, true)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Decoding error: {}", e)))?;

        Ok(CandleOcrOutput {
            content: output_text.trim().to_string(),
            is_structured_markdown: true,
            confidence: None,
        })
    }

    /// Load and preprocess image for PaddleOCR-VL.
    fn load_and_preprocess_image(&self, image_bytes: &[u8]) -> Result<(Tensor, Tensor)> {
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image decode error: {}", e)))?;

        let img = img.to_rgb8();
        let (width, height) = (img.width() as usize, img.height() as usize);

        // PaddleOCR-VL uses dynamic resolution with patch size 14
        // Resize to be divisible by factor (patch_size * spatial_merge = 28)
        let patch_size = 14;
        let spatial_merge = 2;
        let factor = patch_size * spatial_merge; // 28
        let min_pixels = 147384; // from preprocessor_config.json
        let max_pixels = 2822400; // from preprocessor_config.json

        let (new_height, new_width) = self.smart_resize(height, width, factor, min_pixels, max_pixels)?;

        // Resize image: use CatmullRom as closest to PIL's BICUBIC
        let resized = image::imageops::resize(
            &img,
            new_width as u32,
            new_height as u32,
            image::imageops::FilterType::CatmullRom,
        );

        // Normalize to [-1, 1] range using tensor ops: 2 * (x / 255) - 1.
        // Equivalent to mean=[0.5, 0.5, 0.5], std=[0.5, 0.5, 0.5] normalization.
        let raw: Vec<u8> = resized.into_raw();
        let mean_vals = [0.5f32, 0.5, 0.5];
        let std_vals = [0.5f32, 0.5, 0.5];
        let mean_t = Tensor::from_vec(mean_vals.to_vec(), (3, 1, 1), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Mean tensor: {}", e)))?;
        let std_t = Tensor::from_vec(std_vals.to_vec(), (3, 1, 1), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Std tensor: {}", e)))?;
        let normalized = Tensor::from_vec(raw, &[new_height, new_width, 3], &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Raw tensor: {}", e)))?
            .permute((2, 0, 1))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Permute: {}", e)))?
            .to_dtype(DType::F32)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dtype F32: {}", e)))?
            .affine(1.0 / 255.0, 0.0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Scale [0,1]: {}", e)))?
            .broadcast_sub(&mean_t)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Subtract mean: {}", e)))?
            .broadcast_div(&std_t)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Divide std: {}", e)))?;

        // Create tensor: (1, 3, H, W)
        let pixel_values = normalized
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze batch: {}", e)))?
            .to_dtype(self.dtype)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dtype conversion: {}", e)))?;

        // Grid THW: (temporal, height_patches, width_patches)
        let h_patches = (new_height / patch_size) as u32;
        let w_patches = (new_width / patch_size) as u32;
        let grid_thw = Tensor::new(&[[1u32, h_patches, w_patches]], &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid tensor: {}", e)))?;

        Ok((pixel_values, grid_thw))
    }

    /// Smart resize algorithm matching PyTorch's PaddleOCRVLImageProcessor.
    fn smart_resize(
        &self,
        height: usize,
        width: usize,
        factor: usize,
        min_pixels: usize,
        max_pixels: usize,
    ) -> Result<(usize, usize)> {
        let mut h = height;
        let mut w = width;

        // Handle tiny images by scaling up to minimum factor
        if h < factor {
            w = (w * factor + h / 2) / h;
            h = factor;
        }
        if w < factor {
            h = (h * factor + w / 2) / w;
            w = factor;
        }

        // Check aspect ratio constraint
        let aspect = if h > w {
            h as f64 / w as f64
        } else {
            w as f64 / h as f64
        };
        if aspect > 200.0 {
            return Err(CandleOcrError::UnsupportedConfig(format!(
                "Aspect ratio {:.1} exceeds maximum of 200",
                aspect
            )));
        }

        // Round to nearest multiple of factor
        let mut h_bar = ((h + factor / 2) / factor) * factor;
        let mut w_bar = ((w + factor / 2) / factor) * factor;

        let total_pixels = h_bar * w_bar;

        if total_pixels > max_pixels {
            // Scale down to fit within max_pixels
            let beta = ((h * w) as f64 / max_pixels as f64).sqrt();
            h_bar = ((h as f64 / beta / factor as f64).floor() as usize) * factor;
            w_bar = ((w as f64 / beta / factor as f64).floor() as usize) * factor;
        } else if total_pixels < min_pixels {
            // Scale up to meet min_pixels
            let beta = (min_pixels as f64 / (h * w) as f64).sqrt();
            h_bar = ((h as f64 * beta / factor as f64).ceil() as usize) * factor;
            w_bar = ((w as f64 * beta / factor as f64).ceil() as usize) * factor;
        }

        Ok((h_bar, w_bar))
    }

    /// Build input token tensor for the given task and number of image tokens.
    fn build_input_tokens(&self, num_image_tokens: usize) -> Result<Tensor> {
        let bos_token_id = self.bos_token_id;

        let user_prefix = "User: ";
        let task_text = self.task.prompt();
        let assistant_prefix = "\nAssistant: ";

        // Tokenize parts
        let user_encoding = self
            .tokenizer
            .encode(user_prefix, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("User tokenization: {}", e)))?;
        let task_encoding = self
            .tokenizer
            .encode(task_text, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Task tokenization: {}", e)))?;
        let assistant_encoding = self
            .tokenizer
            .encode(assistant_prefix, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Assistant tokenization: {}", e)))?;

        // Build full input:
        // <BOS> + "User: " + <IMAGE_START> + <IMAGE_PLACEHOLDER>... + <IMAGE_END> + task + "\nAssistant: "
        let mut input_ids: Vec<u32> = vec![bos_token_id];
        input_ids.extend(user_encoding.get_ids());
        input_ids.push(self.config.vision_start_token_id);
        input_ids.extend(vec![self.config.image_token_id; num_image_tokens]);
        input_ids.push(self.config.vision_end_token_id);
        input_ids.extend(task_encoding.get_ids());
        input_ids.extend(assistant_encoding.get_ids());

        let tensor = Tensor::new(input_ids.as_slice(), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Input tensor: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?;
        Ok(tensor)
    }
}

#[cfg(target_arch = "wasm32")]
impl PaddleOcrVlEngine {
    pub fn new(_task: PaddleOcrVlTask, _device: Device, _dtype: DType) -> Result<Self> {
        Err(CandleOcrError::UnsupportedConfig(
            "PaddleOCR-VL is not supported on WASM target".to_string(),
        ))
    }

    pub fn process_image(&self, _image_bytes: &[u8]) -> Result<CandleOcrOutput> {
        Err(CandleOcrError::UnsupportedConfig(
            "PaddleOCR-VL is not supported on WASM target".to_string(),
        ))
    }
}

#[cfg(test)]
#[ignore]
mod tests {
    use super::*;

    #[test]
    fn test_paddleocr_vl_inference() -> Result<()> {
        // This test requires a test image and network access to download model
        // Run with: cargo test --release --features paddleocr-vl -- --ignored --nocapture test_paddleocr_vl_inference
        #[cfg(not(target_arch = "wasm32"))]
        {
            let device = Device::Cpu;
            let dtype = DType::F32;

            // Create engine
            let engine = PaddleOcrVlEngine::new(PaddleOcrVlTask::Ocr, device, dtype)?;

            // Load test image (you'll need to provide this)
            let test_image_path = "test_documents/images/ocr_image.jpg";
            if !std::path::Path::new(test_image_path).exists() {
                println!("Test image not found, skipping inference test");
                return Ok(());
            }

            let image_bytes = std::fs::read(test_image_path)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Read test image: {}", e)))?;

            // Run inference
            let output = engine.process_image(&image_bytes)?;

            // Assert non-empty markdown output
            assert!(!output.content.is_empty(), "Expected non-empty OCR output");
            assert!(output.is_structured_markdown);

            println!("OCR output:\n{}", output.content);
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        Ok(())
    }
}
