// Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.

//! Inference engine for Hunyuan-OCR: orchestrates model loading and inference.

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;

use crate::CandleOcrOutput;
use crate::error::{CandleOcrError, Result};
use crate::models::hunyuan_ocr::config::{HunYuanVLConfig, HunyuanOCRGenerationConfig};
use crate::models::hunyuan_ocr::model::HunyuanVLModel;
use crate::models::hunyuan_ocr::processor::{HunyuanVLProcessor, IMAGE_TOKEN};
use crate::vendor::aha::InferenceModel;

/// Plain OCR instruction placed in the user turn of the chat template, the analog of
/// GLM-OCR's `GlmOcrTask::Ocr` prompt (`"Text Recognition:"`). Kept separate from the
/// chat-template scaffolding built in [`build_ocr_prompt`].
const OCR_INSTRUCTION: &str = "OCR this image.";

/// Build HunyuanOCR's chat-template prompt around `instruction`, mirroring the model's
/// own chat_template (like GLM-OCR's `tokenizer::build_input_ids`). The scaffolding is,
/// in order: begin-of-sentence, a format token, the image_start / image-placeholder /
/// image_end triple, the instruction, then the user turn. The image placeholder is the
/// processor's [`IMAGE_TOKEN`]; the processor expands it to one token per patch and
/// tokenizes the result, so `input_ids` carries the image tokens the mask and
/// vision-merge depend on. A bare "Image OCR:" prompt has no image tokens, so
/// image-position indexing narrows an empty tensor and inference fails.
fn build_ocr_prompt(instruction: &str) -> String {
    format!(
        "<｜hy_begin▁of▁sentence｜><｜hy_place▁holder▁no▁3｜><｜hy_place▁holder▁no▁100｜>\
         {IMAGE_TOKEN}<｜hy_place▁holder▁no▁101｜>{instruction}<｜hy_User｜>"
    )
}

/// Hunyuan-OCR inference engine: manages model, processor, and generation config.
#[cfg_attr(alef, alef(skip))]
pub struct HunyuanOCREngine {
    processor: HunyuanVLProcessor,
    model: HunyuanVLModel,
    device: Device,
    generation_config: HunyuanOCRGenerationConfig,
    model_name: String,
}

impl HunyuanOCREngine {
    /// Initialize the Hunyuan-OCR engine from a model directory.
    ///
    /// # Arguments
    /// * `path` - Path to model directory containing config.json, generation_config.json,
    ///   tokenizer.json, and safetensors weight files.
    /// * `device` - Optional device; defaults to CPU if None
    /// * `dtype` - Optional data type; defaults to based on config.dtype
    ///
    /// # Errors
    ///
    /// Returns an error if any model files are missing, cannot be read, or if
    /// weight loading/model construction fails.
    pub fn init(path: &str, device: Option<&Device>, dtype: Option<DType>) -> Result<Self> {
        let config_path = format!("{}/config.json", path);
        let config_bytes = std::fs::read(&config_path)?;
        let cfg: HunYuanVLConfig = serde_json::from_slice(&config_bytes)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Parse config: {}", e)))?;

        let device = device.cloned().unwrap_or(Device::Cpu);

        let cfg_dtype = cfg.dtype.as_str();
        let dtype = dtype.unwrap_or(match cfg_dtype {
            "bfloat16" => DType::BF16,
            "float16" => DType::F16,
            _ => DType::F32,
        });

        // The processor owns the tokenizer; the engine reads it back for decoding.
        let processor = HunyuanVLProcessor::new(path, &device, dtype)?;

        // Find and mmap safetensors files.
        let model_files: Vec<String> = std::fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "safetensors"))
            .filter_map(|entry| entry.path().to_str().map(|s| s.to_string()))
            .collect();

        if model_files.is_empty() {
            return Err(CandleOcrError::ModelLoadFailed(
                "No safetensors files found in model path".to_string(),
            ));
        }

        // SAFETY: mmaped_safetensors is called with valid, existence-checked file paths.
        // The files are opened read-only and the mmap is held by the returned VarBuilder
        // for as long as the weights are in use, so the mapping outlives every tensor read.
        #[allow(unsafe_code)]
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&model_files, dtype, &device)
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Load weights: {}", e)))?
        };

        let generation_config_path = format!("{}/generation_config.json", path);
        let gen_config_bytes = std::fs::read(&generation_config_path)?;
        let generation_config: HunyuanOCRGenerationConfig = serde_json::from_slice(&gen_config_bytes)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Parse generation config: {}", e)))?;

        let model = HunyuanVLModel::new(vb, cfg, generation_config.eos_token_id.clone())?;

        let model_name = std::path::Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("hunyuan_ocr")
            .to_string();

        Ok(Self {
            processor,
            model,
            device,
            generation_config,
            model_name,
        })
    }

    /// Get the model name.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Get a reference to the generation config.
    pub fn generation_config(&self) -> &HunyuanOCRGenerationConfig {
        &self.generation_config
    }

    /// Get a mutable reference to the model.
    pub fn model_mut(&mut self) -> &mut HunyuanVLModel {
        &mut self.model
    }

    /// Get a reference to the processor.
    pub fn processor(&self) -> &HunyuanVLProcessor {
        &self.processor
    }

    /// Get the device used for inference.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Run inference over a single image and return the recognised content.
    ///
    /// Pipeline:
    /// 1. Load and preprocess image into pixel values and grid dimensions
    /// 2. Build OCR prompt with image placeholders
    /// 3. Prepare multimodal data (image tensors, mask, position embeddings)
    /// 4. Run autoregressive generation with image embeddings injected
    /// 5. Decode generated token sequence to markdown text
    ///
    /// # Errors
    ///
    /// Returns `CandleOcrError::InferenceFailed` if preprocessing, encoding,
    /// or generation fails. Returns `CandleOcrError::Tokenizer` if
    /// tokenization or decoding fails.
    pub fn process_image(&mut self, image_bytes: &[u8]) -> Result<CandleOcrOutput> {
        tracing::debug!(image_size = image_bytes.len(), model = %self.model_name, "Hunyuan-OCR: starting inference");
        // Decode image bytes to DynamicImage
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image decode: {}", e)))?;

        let (img_width, img_height) = (img.width(), img.height());
        tracing::debug!(width = img_width, height = img_height, "Hunyuan-OCR: image dimensions");

        // Preprocess the image and build multimodal data; the processor expands the
        // image placeholder and tokenizes to produce input_ids, image mask and position IDs.
        tracing::debug!("Hunyuan-OCR: processing image and text to multimodal data");
        let prompt = build_ocr_prompt(OCR_INSTRUCTION);
        let hunyuan_data = self.processor.process_images_and_text(&[img], &prompt)?;

        // Prepare multimodal data for the model's forward_initial call
        // The model expects: [pixel_values, image_grid_thw, image_mask, position_ids]
        let mm_data = crate::vendor::aha::MultiModalData::new(vec![
            hunyuan_data.pixel_values,
            hunyuan_data.image_grid_thw,
            Some(hunyuan_data.image_mask),
            Some(hunyuan_data.position_ids),
        ]);

        // Run autoregressive generation
        tracing::debug!("Hunyuan-OCR: clearing cache and running forward pass");
        self.model.clear_kv_cache();

        let mut logits = self
            .model
            .forward_initial(&hunyuan_data.input_ids, 0, mm_data)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Initial forward: {}", e)))?;

        // Greedy autoregressive decode, mirroring the reference generation loop:
        // take a token from the initial forward's logits, then feed each token back
        // through forward_step at its absolute position (the prompt occupies
        // 0..prompt_len). Decode steps use the standard offset-based RoPE
        // (no position_ids); the XD-RoPE positions only shape the prompt pass, and
        // generated text tokens continue sequentially after it. Capped so a
        // degenerate image (no stop token) can't run unbounded.
        const MAX_NEW_TOKENS: usize = 128;
        let prompt_len = hunyuan_data
            .input_ids
            .dim(1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Prompt len: {}", e)))?;
        let stop_ids = self.model.stop_token_ids();
        let mut generated: Vec<u32> = Vec::new();

        for step in 0..MAX_NEW_TOKENS {
            let seq_len = logits
                .dim(1)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Output seq len: {}", e)))?;

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

            generated.push(next_token);

            if stop_ids.contains(&next_token) {
                tracing::debug!(step = step, num_tokens = generated.len(), "Hunyuan-OCR: reached stop token");
                break;
            }

            let next_token_tensor = Tensor::new(&[next_token as i64], &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Next token tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze next: {}", e)))?;

            logits = self
                .model
                .forward_step(&next_token_tensor, prompt_len + step)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Forward step {}: {}", step, e)))?;
        }

        // Decode the generated tokens, skipping the stop/special tokens.
        let output_text = self
            .processor
            .tokenizer()
            .decode(&generated, true)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Decode error: {}", e)))?
            .trim()
            .to_string();

        if output_text.is_empty() {
            tracing::warn!("Hunyuan-OCR: output is empty");
        } else {
            tracing::debug!(
                text_len = output_text.len(),
                num_tokens = generated.len(),
                "Hunyuan-OCR: decoding complete"
            );
        }

        Ok(CandleOcrOutput {
            content: output_text,
            is_structured_markdown: false,
            confidence: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_ocr_prompt_carries_exactly_one_image_token_and_the_instruction() {
        let prompt = build_ocr_prompt(OCR_INSTRUCTION);
        // The processor's expansion loop finds image positions by this exact token;
        // zero occurrences reintroduces the empty-tensor crash, two would double the
        // image-token run.
        assert_eq!(prompt.matches(IMAGE_TOKEN).count(), 1);
        assert!(prompt.contains(OCR_INSTRUCTION));
        assert!(prompt.ends_with("<｜hy_User｜>"));
        // The line-continuation in the template must not leak whitespace into the
        // special-token scaffolding.
        let scaffolding = prompt.replace(OCR_INSTRUCTION, "");
        assert!(!scaffolding.contains(' ') && !scaffolding.contains('\n'));
    }
}
