//! Top-level engine for PaddleOCR-VL model inference.
//!
//! Wraps the model and tokenizer for end-to-end OCR processing.

use candle_core::{DType, Device, Tensor};
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;

use crate::error::Result;
use crate::{CandleOcrError, CandleOcrOutput};

use super::config::{PaddleOCRVLConfig, PaddleOCRVLPreprocessorConfig};
use super::model::PaddleOCRVLModel;
use super::processor::PaddleOCRVLProcessor;

/// PaddleOCR-VL task selection.
#[cfg_attr(alef, alef(skip))]
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

/// PaddleOCR-VL inference engine.
#[cfg_attr(alef, alef(skip))]
#[allow(dead_code)]
pub struct PaddleOcrVlEngine {
    model: PaddleOCRVLModel,
    tokenizer: Tokenizer,
    config: PaddleOCRVLConfig,
    processor_config: PaddleOCRVLPreprocessorConfig,
    processor: PaddleOCRVLProcessor,
    task: PaddleOcrVlTask,
    device: Device,
    dtype: DType,
    bos_token_id: u32,
    eos_token_id: u32,
}

impl PaddleOcrVlEngine {
    /// Create a new PaddleOCR-VL engine for the given task and device.
    ///
    /// # Arguments
    /// * `model_path` - Path to model directory containing config.json, tokenizer.json, and weights
    /// * `task` - OCR task (Ocr, Table, Formula, Chart)
    /// * `device` - Candle device (CPU, CUDA, Metal)
    /// * `dtype` - Data type (F32, F16, BF16)
    pub fn new(model_path: &str, task: PaddleOcrVlTask, device: Device, dtype: DType) -> Result<Self> {
        // Load main config
        let config_file = std::path::Path::new(model_path).join("config.json");
        let config_str = std::fs::read_to_string(&config_file)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Read config: {}", e)))?;
        let config: PaddleOCRVLConfig = serde_json::from_str(&config_str)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Parse config: {}", e)))?;

        // Load preprocessor config
        let processor_config_file = std::path::Path::new(model_path).join("preprocessor_config.json");
        let processor_config_str = std::fs::read_to_string(&processor_config_file)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Read preprocessor_config: {}", e)))?;
        let processor_config: PaddleOCRVLPreprocessorConfig = serde_json::from_str(&processor_config_str)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Parse preprocessor_config: {}", e)))?;

        // Load tokenizer
        let tokenizer_file = std::path::Path::new(model_path).join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Load tokenizer: {}", e)))?;

        // Load model weights
        let model_file = {
            let safetensors_path = std::path::Path::new(model_path).join("model.safetensors");
            let bin_path = std::path::Path::new(model_path).join("pytorch_model.bin");

            if safetensors_path.exists() {
                safetensors_path
            } else if bin_path.exists() {
                bin_path
            } else {
                return Err(CandleOcrError::ModelLoadFailed(
                    "No model weights found (expected model.safetensors or pytorch_model.bin)".to_string(),
                ));
            }
        };

        tracing::debug!("Loading weights from {:?}", model_file);

        let vb = if model_file.extension().is_some_and(|ext| ext == "bin") {
            candle_nn::VarBuilder::from_pth(&model_file, dtype, &device)
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Load pth: {}", e)))?
        } else {
            // SAFETY: We're using mmaped_safetensors with a valid file path. The file is read-only
            // and the lifetime is scoped to this function, ensuring memory safety.
            #[allow(unsafe_code)]
            unsafe {
                candle_nn::VarBuilder::from_mmaped_safetensors(&[&model_file], dtype, &device)
                    .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Load safetensors: {}", e)))?
            }
        };

        let model = PaddleOCRVLModel::new(config.clone(), vb, vec![2])
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Model init: {}", e)))?;

        // Resolve special token IDs from tokenizer
        let bos_token_id = tokenizer.token_to_id("<|begin_of_sentence|>").unwrap_or(1);
        let eos_token_id = tokenizer
            .token_to_id("</s>")
            .or_else(|| tokenizer.token_to_id("<|end_of_sentence|>"))
            .or_else(|| tokenizer.token_to_id("<|endoftext|>"))
            .unwrap_or(2);

        tracing::debug!(bos_token_id, eos_token_id, "Resolved PaddleOCR-VL special tokens");

        let processor = PaddleOCRVLProcessor::new(processor_config.clone(), &device, dtype)?;

        Ok(PaddleOcrVlEngine {
            model,
            tokenizer,
            config,
            processor_config,
            processor,
            task,
            device,
            dtype,
            bos_token_id,
            eos_token_id,
        })
    }

    /// Process an image and return the recognized text as markdown.
    pub fn process_image(&mut self, image_bytes: &[u8]) -> Result<CandleOcrOutput> {
        // Decode image
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Decode image: {}", e)))?;
        let img = img.to_rgb8();

        // Prepare mean/std tensors from processor config
        let img_mean = Tensor::new(
            &[[
                self.processor_config.image_mean[0] as f32,
                self.processor_config.image_mean[1] as f32,
                self.processor_config.image_mean[2] as f32,
            ]],
            &self.device,
        )
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Mean tensor: {}", e)))?
        .reshape((3, 1, 1))
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape mean: {}", e)))?;

        let img_std = Tensor::new(
            &[[
                self.processor_config.image_std[0] as f32,
                self.processor_config.image_std[1] as f32,
                self.processor_config.image_std[2] as f32,
            ]],
            &self.device,
        )
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Std tensor: {}", e)))?
        .reshape((3, 1, 1))
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape std: {}", e)))?;

        // Process image
        let dyn_img = image::DynamicImage::ImageRgb8(img);
        let pixel_values = self.processor.process_img(&dyn_img, &img_mean, &img_std)?;
        let (pixel_values, grid_thw) = self.processor.process_vision_tensor(&pixel_values)?;

        // Build input tokens
        let grid_vec = grid_thw
            .to_vec2::<u32>()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid shape: {}", e)))?;
        let g = &grid_vec[0];
        let h_patches = g[1] as usize;
        let w_patches = g[2] as usize;
        let spatial_merge = self.config.vision_config.spatial_merge_size;
        let num_image_tokens = (h_patches / spatial_merge) * (w_patches / spatial_merge);

        let input_ids = self.build_input_tokens(num_image_tokens)?;

        let max_length = 4096;

        // Clear KV cache
        self.model.clear_kv_cache();

        // Run generation
        let generated_tokens = self.generate(&input_ids, &pixel_values, &grid_thw, max_length)?;

        // Decode tokens to text
        let output_text = self
            .tokenizer
            .decode(&generated_tokens, true)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Decode: {}", e)))?;

        Ok(CandleOcrOutput {
            content: output_text.trim().to_string(),
            is_structured_markdown: true,
            confidence: None,
        })
    }

    /// Build input token tensor for the given task and number of image tokens.
    fn build_input_tokens(&self, num_image_tokens: usize) -> Result<Tensor> {
        let user_prefix = "User: ";
        let task_text = self.task.prompt();
        let assistant_prefix = "\nAssistant: ";

        let user_encoding = self
            .tokenizer
            .encode(user_prefix, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Encode user: {}", e)))?;
        let task_encoding = self
            .tokenizer
            .encode(task_text, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Encode task: {}", e)))?;
        let assistant_encoding = self
            .tokenizer
            .encode(assistant_prefix, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Encode assistant: {}", e)))?;

        let mut input_ids: Vec<u32> = vec![self.bos_token_id];
        input_ids.extend(user_encoding.get_ids());
        input_ids.push(self.config.vision_start_token_id);
        input_ids.extend(vec![self.config.image_token_id; num_image_tokens]);
        if let Some(vision_end_token_id) = self.config.vision_end_token_id {
            input_ids.push(vision_end_token_id);
        }
        input_ids.extend(task_encoding.get_ids());
        input_ids.extend(assistant_encoding.get_ids());

        let tensor = Tensor::new(input_ids.as_slice(), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Input tensor: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?;

        Ok(tensor)
    }

    /// Generate tokens using the model (simple greedy decoding).
    fn generate(
        &mut self,
        input_ids: &Tensor,
        pixel_values: &Tensor,
        grid_thw: &Tensor,
        max_length: usize,
    ) -> Result<Vec<u32>> {
        let mut generated_tokens = input_ids
            .to_vec2::<u32>()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Input to_vec2: {}", e)))?
            .into_iter()
            .next()
            .ok_or_else(|| CandleOcrError::InferenceFailed("Empty input".to_string()))?;

        for _ in 0..max_length {
            if generated_tokens.len() >= max_length {
                break;
            }

            let input_tensor = Tensor::new(generated_tokens.as_slice(), &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Create tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?;

            // Create image mask (1 for image tokens, 0 for text)
            let image_mask: Vec<u32> = generated_tokens
                .iter()
                .map(|&token| if token == self.config.image_token_id { 1 } else { 0 })
                .collect();
            let image_mask = Tensor::new(image_mask.as_slice(), &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Mask tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze mask: {}", e)))?;

            let logits = self
                .model
                .forward(
                    &input_tensor,
                    Some(pixel_values),
                    Some(grid_thw),
                    Some(&image_mask),
                    None,
                    0,
                )
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Forward: {}", e)))?;

            // Greedy decoding: take argmax of last token
            let logits_last = logits
                .narrow(1, logits.dim(1)? - 1, 1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow: {}", e)))?;
            let next_token = logits_last
                .argmax(candle_core::D::Minus1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Argmax: {}", e)))?
                .to_vec1::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("To vec1: {}", e)))?[0];

            generated_tokens.push(next_token);

            if next_token == self.eos_token_id {
                break;
            }
        }

        Ok(generated_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_prompt() {
        assert_eq!(PaddleOcrVlTask::Ocr.prompt(), "OCR:");
        assert_eq!(PaddleOcrVlTask::Table.prompt(), "Table Recognition:");
        assert_eq!(PaddleOcrVlTask::Formula.prompt(), "Formula Recognition:");
        assert_eq!(PaddleOcrVlTask::Chart.prompt(), "Chart Recognition:");
    }

    #[test]
    fn test_task_display() {
        assert_eq!(PaddleOcrVlTask::Ocr.to_string(), "ocr");
        assert_eq!(PaddleOcrVlTask::Table.to_string(), "table");
        assert_eq!(PaddleOcrVlTask::Formula.to_string(), "formula");
        assert_eq!(PaddleOcrVlTask::Chart.to_string(), "chart");
    }
}
