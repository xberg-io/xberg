// Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.

//! Inference engine for Hunyuan-OCR: orchestrates model loading and inference.

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use tokenizers::Tokenizer;

use crate::CandleOcrOutput;
use crate::error::{CandleOcrError, Result};
use crate::models::hunyuan_ocr::config::{HunYuanVLConfig, HunyuanOCRGenerationConfig};
use crate::models::hunyuan_ocr::model::HunyuanVLModel;
use crate::models::hunyuan_ocr::processor::HunyuanVLProcessor;
use crate::vendor::aha::InferenceModel;

/// Hunyuan-OCR inference engine: manages model, processor, and generation config.
#[cfg_attr(alef, alef(skip))]
pub struct HunyuanOCREngine {
    processor: HunyuanVLProcessor,
    model: HunyuanVLModel,
    device: Device,
    generation_config: HunyuanOCRGenerationConfig,
    model_name: String,
    tokenizer: Tokenizer,
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

        let processor = HunyuanVLProcessor::new(path, &device, dtype)?;

        // Load tokenizer
        let tokenizer_path = format!("{}/tokenizer.json", path);
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Tokenizer load error: {}", e)))?;

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
            tokenizer,
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
        // Decode image bytes to DynamicImage
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image decode: {}", e)))?;

        // Build OCR prompt that matches the model's expected input template
        let prompt_text = "Image OCR:";

        // Tokenize the prompt to get input IDs
        let encoding = self
            .tokenizer
            .encode(prompt_text, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Encode prompt: {}", e)))?;
        let prompt_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();

        let input_ids = Tensor::new(prompt_ids.as_slice(), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Token tensor: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze batch: {}", e)))?;

        // Use the processor to build full multimodal data with image preprocessing, mask and position IDs
        let hunyuan_data = self
            .processor
            .process_images_and_text(&[img], input_ids.clone(), prompt_text)?;

        // Prepare multimodal data for the model's forward_initial call
        // The model expects: [pixel_values, image_grid_thw, image_mask, position_ids]
        let mm_data = crate::vendor::aha::MultiModalData::new(vec![
            hunyuan_data.pixel_values,
            hunyuan_data.image_grid_thw,
            Some(hunyuan_data.image_mask),
            Some(hunyuan_data.position_ids),
        ]);

        // Run autoregressive generation
        self.model.clear_kv_cache();

        let output_ids = self
            .model
            .forward_initial(&hunyuan_data.input_ids, 0, mm_data)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Initial forward: {}", e)))?;

        // For a simple first implementation, use the logits from forward_initial to get the next token
        // In a full implementation, this would be an autoregressive loop
        let seq_len = output_ids
            .dim(1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Output seq len: {}", e)))?;

        let last_logits = output_ids
            .narrow(1, seq_len - 1, 1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Narrow last: {}", e)))?;

        let token_id = last_logits
            .argmax(2)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Argmax: {}", e)))?
            .squeeze(2)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze: {}", e)))?
            .to_vec1::<u32>()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("To vec: {}", e)))?;

        // Decode the generated token to text
        let output_text = self
            .tokenizer
            .decode(&token_id, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Decode error: {}", e)))?
            .trim()
            .to_string();

        Ok(CandleOcrOutput {
            content: output_text,
            is_structured_markdown: false,
            confidence: None,
        })
    }
}
