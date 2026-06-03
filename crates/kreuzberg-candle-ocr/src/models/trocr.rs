//! TrOCR model implementation: Microsoft's transformer-based OCR engine.
//!
//! TrOCR is an encoder-decoder model that achieves strong text recognition
//! on both printed and handwritten documents. The encoder is a BEiT vision
//! transformer, and the decoder is a RoBERTa-based sequence-to-sequence model.
//!
//! Supported variants:
//! - `base-printed` (default): ~330M params, optimized for printed text
//! - `large-printed`: higher accuracy, slower inference
//! - `base-handwritten`: tuned for handwritten text
//! - `large-handwritten`: high-quality handwritten text recognition

#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::{CandleOcrError, CandleOcrOutput, ModelKind};

#[cfg(not(target_arch = "wasm32"))]
use candle_core::{DType, Device, Tensor};
#[cfg(not(target_arch = "wasm32"))]
use candle_nn::VarBuilder;
#[cfg(not(target_arch = "wasm32"))]
use candle_transformers::models::{trocr, vit};
#[cfg(not(target_arch = "wasm32"))]
use parking_lot::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tokenizers::Tokenizer;

/// TrOCR model variant selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum TrocrVariant {
    /// Base printed text model (330M params) — recommended default
    #[default]
    BasePrinted,
    /// Large printed text model (555M params) — higher accuracy, slower
    LargePrinted,
    /// Base handwritten text model (330M params)
    BaseHandwritten,
    /// Large handwritten text model (555M params)
    LargeHandwritten,
}

impl TrocrVariant {
    /// HuggingFace repository ID for this variant.
    pub fn repo_id(&self) -> &'static str {
        match self {
            TrocrVariant::BasePrinted => "microsoft/trocr-base-printed",
            TrocrVariant::LargePrinted => "microsoft/trocr-large-printed",
            TrocrVariant::BaseHandwritten => "microsoft/trocr-base-handwritten",
            TrocrVariant::LargeHandwritten => "microsoft/trocr-large-handwritten",
        }
    }

    /// HuggingFace git branch for this variant.
    /// Some variants use PR branches; others use main.
    pub fn branch(&self) -> &'static str {
        match self {
            TrocrVariant::BasePrinted => "refs/pr/7",
            TrocrVariant::LargePrinted => "main",
            TrocrVariant::BaseHandwritten => "refs/pr/3",
            TrocrVariant::LargeHandwritten => "refs/pr/6",
        }
    }

    /// Brief description of this variant.
    pub fn description(&self) -> &'static str {
        match self {
            TrocrVariant::BasePrinted => "Printed text (330M params)",
            TrocrVariant::LargePrinted => "Printed text (555M params)",
            TrocrVariant::BaseHandwritten => "Handwritten text (330M params)",
            TrocrVariant::LargeHandwritten => "Handwritten text (555M params)",
        }
    }
}

impl std::fmt::Display for TrocrVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            TrocrVariant::BasePrinted => "base-printed",
            TrocrVariant::LargePrinted => "large-printed",
            TrocrVariant::BaseHandwritten => "base-handwritten",
            TrocrVariant::LargeHandwritten => "large-handwritten",
        };
        write!(f, "{}", name)
    }
}

/// Full TrOCR config combining encoder and decoder configurations.
#[derive(Debug, Clone, Deserialize)]
#[cfg(not(target_arch = "wasm32"))]
struct TrocrFullConfig {
    encoder: vit::Config,
    decoder: trocr::TrOCRConfig,
}

/// TrOCR engine combining encoder and decoder.
#[cfg(not(target_arch = "wasm32"))]
pub struct TrocrEngine {
    variant: TrocrVariant,
    device: Device,
    model: Arc<Mutex<trocr::TrOCRModel>>,
    tokenizer: Tokenizer,
    decoder_start_token_id: u32,
    eos_token_id: u32,
}

#[cfg(target_arch = "wasm32")]
pub struct TrocrEngine {
    variant: TrocrVariant,
}

impl TrocrEngine {
    /// Create a new TrOCR engine for the given variant and device.
    ///
    /// # Arguments
    ///
    /// * `variant` - Which TrOCR variant to load
    /// * `device` - Candle compute device (CPU, CUDA, Metal)
    ///
    /// # Returns
    ///
    /// A ready-to-use TrOCR engine with tokenizer.
    ///
    /// # Errors
    ///
    /// - Model weight download or loading fails
    /// - Config parsing fails
    /// - Tokenizer loading fails
    /// - Device initialization fails
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(variant: TrocrVariant, device: Device) -> Result<Self> {
        use hf_hub::RepoType;
        use hf_hub::api::sync::Api;

        let api = Api::new()
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to initialize HF Hub API: {}", e)))?;

        tracing::info!("Loading TrOCR variant: {}", variant);

        // Use per-variant branch for model repo
        let repo_id = variant.repo_id().to_string();
        let branch = variant.branch().to_string();
        let model_repo = hf_hub::Repo::with_revision(repo_id.clone(), RepoType::Model, branch.clone());

        // Download model weights (~1.4GB for base variants)
        let model_file = api.repo(model_repo.clone()).get("model.safetensors").map_err(|e| {
            CandleOcrError::ModelLoadFailed(format!(
                "Failed to download model weights for {} (branch {}): {}",
                variant, branch, e
            ))
        })?;

        tracing::info!("Downloaded model weights to: {}", model_file.display());

        // Download and parse config.json to get both encoder and decoder configs
        let config_file = api.repo(model_repo).get("config.json").map_err(|e| {
            CandleOcrError::ModelLoadFailed(format!("Failed to download config.json for {}: {}", variant, e))
        })?;

        let config_str = std::fs::read_to_string(&config_file)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to read config.json: {}", e)))?;

        let full_config: TrocrFullConfig = serde_json::from_str(&config_str)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to parse config.json: {}", e)))?;

        // Load weights using memory mapping
        // SAFETY: VarBuilder::from_mapped_safetensors requires that:
        // 1. The file path is valid and readable (guaranteed by hf_hub cache)
        // 2. The safetensors format is valid (guaranteed by HF validation)
        // 3. The device is compatible (guaranteed by candle)
        // 4. No concurrent writes (guaranteed by hf_hub's file locking)
        //
        // The mmap'd buffer is kept in memory for the lifetime of the VarBuilder,
        // and the underlying file handle is managed by hf_hub's cache system.
        #[allow(unsafe_code)]
        let vb = unsafe {
            VarBuilder::from_mapped_safetensors(&[model_file], DType::F32, &device)
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load safetensors: {}", e)))?
        };

        // Build TrOCRModel with both encoder and decoder configs
        tracing::info!("Building TrOCR encoder-decoder model");
        let model = trocr::TrOCRModel::new(&full_config.encoder, &full_config.decoder, vb)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to build TrOCR model: {}", e)))?;

        // Download and load tokenizer from ToluClassics/candle-trocr-tokenizer
        let tokenizer_repo = api.model("ToluClassics/candle-trocr-tokenizer".to_string());
        let tokenizer_file = tokenizer_repo
            .get("tokenizer.json")
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to download tokenizer: {}", e)))?;

        let tokenizer = Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load tokenizer: {}", e)))?;

        tracing::info!("TrOCR {} initialized successfully", variant);

        Ok(Self {
            variant,
            device,
            model: Arc::new(Mutex::new(model)),
            tokenizer,
            decoder_start_token_id: full_config.decoder.decoder_start_token_id,
            eos_token_id: full_config.decoder.eos_token_id,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new(variant: TrocrVariant, _device: candle_core::Device) -> Result<Self> {
        Err(CandleOcrError::UnsupportedConfig(
            "TrOCR not available on WASM: requires HF Hub API and native compute".to_string(),
        ))
    }

    /// Process a single image and extract text via OCR.
    ///
    /// # Arguments
    ///
    /// * `image_bytes` - Raw JPEG/PNG/TIFF image data
    ///
    /// # Returns
    ///
    /// Extracted text with optional confidence score.
    ///
    /// # Errors
    ///
    /// - Image decode fails
    /// - Model inference fails
    pub fn process_image(&self, image_bytes: &[u8]) -> Result<CandleOcrOutput> {
        // Validate image
        if image_bytes.is_empty() {
            return Err(CandleOcrError::UnsupportedConfig("Empty image data".to_string()));
        }

        // Preprocess image: resize to 384x384, normalize with mean=[0.5,0.5,0.5], std=[0.5,0.5,0.5]
        let processor = crate::models::image_processor::ImageProcessor::default();
        let image_tensor = processor
            .process(image_bytes, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image preprocessing failed: {}", e)))?;

        // Run encoder forward pass to get encoder hidden states
        let mut model_guard = self.model.lock();
        model_guard.reset_kv_cache();

        let encoder_hidden_states = model_guard
            .encoder()
            .forward(&image_tensor)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Encoder forward failed: {}", e)))?;

        // Decoder configuration for generation (from the loaded checkpoint config)
        let decoder_start_token_id = self.decoder_start_token_id;
        let eos_token_id = self.eos_token_id;

        // Initialize decoder input with start token
        let mut token_ids = vec![decoder_start_token_id];

        // Logits processor for sampling
        let mut logits_processor = candle_transformers::generation::LogitsProcessor::new(1337, None, None);

        // Decoding loop (max 1000 iterations)
        for index in 0..1000 {
            let context_size = if index >= 1 { 1 } else { token_ids.len() };
            let start_pos = token_ids.len().saturating_sub(context_size);
            let input_ids = Tensor::new(&token_ids[start_pos..], &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token tensor creation failed: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token unsqueeze failed: {}", e)))?;

            // Decoder forward pass
            let logits = model_guard
                .decode(&input_ids, &encoder_hidden_states, start_pos)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Decoder forward failed: {}", e)))?;

            // Get logits for next token
            let logits = logits
                .squeeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Logits squeeze(0) failed: {}", e)))?;
            let logits = logits
                .get(
                    logits
                        .dim(0)
                        .map_err(|e| CandleOcrError::InferenceFailed(format!("Logits dim(0) failed: {}", e)))?
                        - 1,
                )
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Logits indexing failed: {}", e)))?;

            // Sample next token
            let token = logits_processor
                .sample(&logits)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token sampling failed: {}", e)))?;

            token_ids.push(token);

            // Stop on EOS token
            if token == eos_token_id {
                break;
            }
        }

        // Decode all collected token ids to text
        let decoded_text = self
            .tokenizer
            .decode(&token_ids, true)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Tokenizer decode failed: {}", e)))?;

        Ok(CandleOcrOutput {
            content: decoded_text,
            is_structured_markdown: false,
            confidence: None,
        })
    }

    /// Get the variant this engine was initialized with.
    pub fn variant(&self) -> TrocrVariant {
        self.variant
    }

    /// Get model kind identifier for telemetry.
    pub fn model_kind(&self) -> ModelKind {
        ModelKind::Trocr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trocr_variant_repo_ids() {
        assert_eq!(TrocrVariant::BasePrinted.repo_id(), "microsoft/trocr-base-printed");
        assert_eq!(TrocrVariant::LargePrinted.repo_id(), "microsoft/trocr-large-printed");
        assert_eq!(
            TrocrVariant::BaseHandwritten.repo_id(),
            "microsoft/trocr-base-handwritten"
        );
        assert_eq!(
            TrocrVariant::LargeHandwritten.repo_id(),
            "microsoft/trocr-large-handwritten"
        );
    }

    #[test]
    fn test_trocr_variant_default() {
        assert_eq!(TrocrVariant::default(), TrocrVariant::BasePrinted);
    }

    #[test]
    fn test_trocr_variant_display() {
        assert_eq!(TrocrVariant::BasePrinted.to_string(), "base-printed");
        assert_eq!(TrocrVariant::LargePrinted.to_string(), "large-printed");
        assert_eq!(TrocrVariant::BaseHandwritten.to_string(), "base-handwritten");
        assert_eq!(TrocrVariant::LargeHandwritten.to_string(), "large-handwritten");
    }

    #[test]
    fn test_trocr_variant_branches() {
        assert_eq!(TrocrVariant::BasePrinted.branch(), "refs/pr/7");
        assert_eq!(TrocrVariant::LargePrinted.branch(), "main");
        assert_eq!(TrocrVariant::BaseHandwritten.branch(), "refs/pr/3");
        assert_eq!(TrocrVariant::LargeHandwritten.branch(), "refs/pr/6");
    }

    #[test]
    #[ignore] // Expensive: downloads ~1.4GB model on first run
    fn test_engine_creation() {
        let device = Device::Cpu;
        let engine = TrocrEngine::new(TrocrVariant::BasePrinted, device).expect("Engine creation failed");
        assert_eq!(engine.variant(), TrocrVariant::BasePrinted);
        assert_eq!(engine.model_kind(), ModelKind::Trocr);
    }

    #[test]
    #[ignore] // Expensive: downloads ~1.4GB model on first run
    fn test_inference_on_real_image() {
        use std::fs;
        use std::path::Path;

        // Load test image
        let image_path = Path::new("../../test_documents/images/ocr_image.jpg");
        if !image_path.exists() {
            tracing::warn!(
                "Test image not found at {}; skipping real inference test",
                image_path.display()
            );
            return;
        }

        let image_bytes = fs::read(image_path).expect("Failed to read test image");

        // Create engine (will download model on first run)
        let device = Device::Cpu;
        let engine = TrocrEngine::new(TrocrVariant::BasePrinted, device).expect("Failed to create TrOCR engine");

        // Run OCR
        let result = engine.process_image(&image_bytes).expect("OCR inference failed");

        // Verify we got text output
        assert!(!result.content.is_empty(), "OCR returned empty text");

        // Verify at least one ASCII letter is present
        let has_letter = result.content.chars().any(|c| c.is_ascii_alphabetic());
        assert!(
            has_letter,
            "OCR output contains no ASCII letters. Got: {}",
            result.content
        );

        println!("OCR Result:\n{}", result.content);
    }
}
