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
        tracing::debug!(image_size = image_bytes.len(), task = %self.task, "PaddleOCR-VL: starting inference");
        // Decode image
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Decode image: {}", e)))?;
        let img = img.to_rgb8();

        let (img_width, img_height) = (img.width(), img.height());
        tracing::debug!(width = img_width, height = img_height, "PaddleOCR-VL: image dimensions");

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

        tracing::debug!(
            h_patches = h_patches,
            w_patches = w_patches,
            spatial_merge = spatial_merge,
            num_image_tokens = num_image_tokens,
            "PaddleOCR-VL: grid dimensions"
        );

        let input_ids = self.build_input_tokens(num_image_tokens)?;

        let max_length = 4096;

        // Clear KV cache
        tracing::debug!("PaddleOCR-VL: clearing cache and starting generation");
        self.model.clear_kv_cache();

        // Run generation
        let generated_tokens = self.generate(&input_ids, &pixel_values, &grid_thw, max_length)?;

        // Decode tokens to text
        let output_text = self
            .tokenizer
            .decode(&generated_tokens, true)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Decode: {}", e)))?;

        let output_trimmed = output_text.trim().to_string();
        if output_trimmed.is_empty() {
            tracing::warn!(num_tokens = generated_tokens.len(), "PaddleOCR-VL: output is empty");
        } else {
            tracing::debug!(
                text_len = output_trimmed.len(),
                num_tokens = generated_tokens.len(),
                "PaddleOCR-VL: decoding complete"
            );
        }

        Ok(CandleOcrOutput {
            content: output_trimmed,
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

    /// Generate tokens using the model (greedy decoding with KV cache).
    ///
    /// Prefills once with the full prompt (vision features injected), then feeds
    /// each new token back through the cached decode path at its absolute
    /// position, mirroring the Hunyuan-OCR generation loop. Returns only the
    /// newly generated tokens, so decoding the result never echoes the prompt.
    fn generate(
        &mut self,
        input_ids: &Tensor,
        pixel_values: &Tensor,
        grid_thw: &Tensor,
        max_length: usize,
    ) -> Result<Vec<u32>> {
        let prompt_tokens = input_ids
            .to_vec2::<u32>()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Input to_vec2: {}", e)))?
            .into_iter()
            .next()
            .ok_or_else(|| CandleOcrError::InferenceFailed("Empty input".to_string()))?;
        let prompt_len = prompt_tokens.len();

        tracing::debug!(
            initial_tokens = prompt_len,
            max_length = max_length,
            eos_token = self.eos_token_id,
            "PaddleOCR-VL: starting greedy decoding"
        );

        // Image mask over the prompt (1 for image tokens, 0 for text).
        let image_mask: Vec<u32> = prompt_tokens
            .iter()
            .map(|&token| u32::from(token == self.config.image_token_id))
            .collect();
        let image_mask = Tensor::new(image_mask.as_slice(), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Mask tensor: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze mask: {}", e)))?;

        // cache_position starting at 0 makes the forward pass compute the full
        // multimodal rope index for the prompt (and reset any cached rope delta
        // from a previous image).
        let cache_position = Tensor::arange(0u32, prompt_len as u32, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cache position: {}", e)))?;

        let mut logits = self
            .model
            .forward(
                input_ids,
                Some(pixel_values),
                Some(grid_thw),
                Some(&image_mask),
                Some(&cache_position),
                0,
            )
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Forward: {}", e)))?;

        let mut generated: Vec<u32> = Vec::new();
        let max_new_tokens = max_length.saturating_sub(prompt_len);

        for step in 0..max_new_tokens {
            // The forward pass narrows to the last position, so logits is
            // [1, 1, vocab]; argmax over the vocab dim yields [1, 1] — squeeze
            // the two length-1 dims to a scalar token id.
            let next_token = logits
                .argmax(candle_core::D::Minus1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Argmax: {}", e)))?
                .squeeze(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze seq: {}", e)))?
                .squeeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze batch: {}", e)))?
                .to_scalar::<u32>()
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token scalar: {}", e)))?;

            generated.push(next_token);

            if step < 5 {
                tracing::trace!(
                    step = step,
                    token = next_token,
                    num_tokens = generated.len(),
                    "PaddleOCR-VL: decode iteration"
                );
            }

            if next_token == self.eos_token_id {
                tracing::debug!(
                    step = step,
                    num_tokens = generated.len(),
                    "PaddleOCR-VL: reached EOS token"
                );
                break;
            }

            if step + 1 == max_new_tokens {
                break;
            }

            // Feed the new token back at its absolute position; the KV cache
            // holds the prompt and all previously generated tokens.
            let next_input = Tensor::new(&[next_token], &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Next token tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze next: {}", e)))?;

            logits = self
                .model
                .forward(&next_input, None, None, None, None, prompt_len + step)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Forward step {}: {}", step, e)))?;
        }

        Ok(generated)
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

    /// Each `PaddleOcrVlTask` variant serializes to the expected lowercase string
    /// and deserializes back to the same variant (round-trip fidelity).
    #[test]
    fn paddle_ocr_vl_task_all_variants_serde_round_trip() {
        let cases: &[(PaddleOcrVlTask, &str)] = &[
            (PaddleOcrVlTask::Ocr, "\"ocr\""),
            (PaddleOcrVlTask::Table, "\"table\""),
            (PaddleOcrVlTask::Formula, "\"formula\""),
            (PaddleOcrVlTask::Chart, "\"chart\""),
        ];

        for (variant, expected_json) in cases {
            let serialized = serde_json::to_string(variant).unwrap_or_else(|e| panic!("serialize {:?}: {e}", variant));
            assert_eq!(
                serialized, *expected_json,
                "PaddleOcrVlTask::{:?} should serialize to {expected_json}",
                variant
            );

            let decoded: PaddleOcrVlTask =
                serde_json::from_str(&serialized).unwrap_or_else(|e| panic!("deserialize {:?}: {e}", variant));
            assert_eq!(
                decoded, *variant,
                "PaddleOcrVlTask::{:?} should round-trip through serde",
                variant
            );
        }
    }

    /// `PaddleOcrVlTask` derives `Default` and `Copy`; verify the default is `Ocr`.
    #[test]
    fn paddle_ocr_vl_task_default_is_ocr() {
        assert_eq!(
            PaddleOcrVlTask::default(),
            PaddleOcrVlTask::Ocr,
            "Default PaddleOcrVlTask should be Ocr"
        );
    }
}
