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
#[cfg(not(target_arch = "wasm32"))]
use crate::models::image_processor::{DEFAULT_IMAGE_DECODE_BUDGET, ImageProcessor, decode_image_with_budget};
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

    /// Immutable Hugging Face revision for this variant.
    ///
    /// Some safetensors conversions originated on PR refs; pinning the resolved
    /// commits prevents those mutable refs from changing beneath the cache.
    pub fn revision(&self) -> &'static str {
        match self {
            TrocrVariant::BasePrinted => "24216f24cd78fe1a9c8b4e6e4565aec5c9220e63",
            TrocrVariant::LargePrinted => "9ff792d8e7c22061f2ee67e1ed2246b1f9ef1e98",
            TrocrVariant::BaseHandwritten => "47db63bbc18d32eca4cb813eb7728c891903e289",
            TrocrVariant::LargeHandwritten => "f07eb3a73a9b06a73141dba2ae1f1671c5c346af",
        }
    }

    /// Backward-compatible alias for [`Self::revision`].
    pub fn branch(&self) -> &'static str {
        self.revision()
    }

    fn model_sha256(&self) -> &'static str {
        match self {
            TrocrVariant::BasePrinted => "1cf4a6eedab26afaaf505f1c7f73d9634944924dbd1ed049d569db98039cd596",
            TrocrVariant::LargePrinted => "8d770e31b1d58a033bd023ddd5790764c78fc2ab8074c605c49bba1c4a938616",
            TrocrVariant::BaseHandwritten => "25a40cddc7e6120140a3d5b9e3dd3878a92ada7b4f312953ab22edc19c2a5acc",
            TrocrVariant::LargeHandwritten => "21b96861916e0c021488df17d90f33bef7d298f0bce464f8ff0ab1bd345b4e70",
        }
    }

    fn config_sha256(&self) -> &'static str {
        match self {
            TrocrVariant::BasePrinted => "5bda1deab455661feb3d91906656e5600e2ca520d5c00a2a03836614b850c93e",
            TrocrVariant::LargePrinted => "9fd06abe8e2b3b835968210cfaccbed6b8f5698ab3fe9743fa2ac021b69f2028",
            TrocrVariant::BaseHandwritten => "4c779f24e063c437c3dafd5b2e6c9f59f2fa2bd1dbb4ae6a30153bbbbf19e647",
            TrocrVariant::LargeHandwritten => "4e4b5be06883d2dcceb299c717dfc96c3853b85e4f62393eac649183b923c5ec",
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
        Self::new_with_hf(variant, device, None, None)
    }

    /// Load a pinned TrOCR variant with optional Hugging Face cache settings.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_with_hf(
        variant: TrocrVariant,
        device: Device,
        cache_dir: Option<&std::path::Path>,
        revision: Option<&str>,
    ) -> Result<Self> {
        tracing::info!("Loading TrOCR variant: {}", variant);

        let repo_id = variant.repo_id().to_string();
        let pinned_revision = variant.revision();
        let revision = revision.unwrap_or(pinned_revision);
        if revision != pinned_revision {
            return Err(CandleOcrError::UnsupportedConfig(format!(
                "{variant} is checksum-pinned to revision {pinned_revision}; requested {revision}"
            )));
        }
        let model_file = crate::download_guard::hf_download(
            &repo_id,
            "model.safetensors",
            revision,
            cache_dir,
            variant.model_sha256(),
        )
        .map_err(|e| {
            CandleOcrError::ModelLoadFailed(format!(
                "Failed to download model weights for {} (revision {}): {}",
                variant, revision, e
            ))
        })?;

        tracing::info!("Downloaded model weights to: {}", model_file.display());

        let config_file =
            crate::download_guard::hf_download(&repo_id, "config.json", revision, cache_dir, variant.config_sha256())
                .map_err(|e| {
                CandleOcrError::ModelLoadFailed(format!("Failed to download config.json for {}: {}", variant, e))
            })?;

        let config_str = std::fs::read_to_string(&config_file)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to read config.json: {}", e)))?;

        let full_config: TrocrFullConfig = serde_json::from_str(&config_str)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to parse config.json: {}", e)))?;

        #[allow(unsafe_code)]
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_file], DType::F32, &device)
                .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to load safetensors: {}", e)))?
        };

        tracing::info!("Building TrOCR encoder-decoder model");
        let model = trocr::TrOCRModel::new(&full_config.encoder, &full_config.decoder, vb)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Failed to build TrOCR model: {}", e)))?;

        let tokenizer_file = crate::download_guard::hf_download(
            "ToluClassics/candle-trocr-tokenizer",
            "tokenizer.json",
            "7253d6cb8df4b0beed072ff65092a90f22f98a89",
            cache_dir,
            "2f1a555a1ee93656b4e6f67aa75d492a843c225e5ef754bae24c36bd85851cd7",
        )
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

    /// Autoregressively decode token ids from `encoder_hidden_states`, one token per iteration,
    /// stopping at `eos_token_id` or 1000 iterations. Split out of [`Self::process_image`] to
    /// keep that function under the workspace line-count limit. ~keep
    fn decode_tokens(&self, model: &mut trocr::TrOCRModel, encoder_hidden_states: &Tensor) -> Result<Vec<u32>> {
        let decoder_start_token_id = self.decoder_start_token_id;
        let eos_token_id = self.eos_token_id;

        let mut token_ids = vec![decoder_start_token_id];

        let mut logits_processor = candle_transformers::generation::LogitsProcessor::new(1337, None, None);

        tracing::debug!(
            start_token = decoder_start_token_id,
            eos_token = eos_token_id,
            "TrOCR: beginning decoding loop"
        );

        for index in 0..1000 {
            let context_size = if index >= 1 { 1 } else { token_ids.len() };
            let start_pos = token_ids.len().saturating_sub(context_size);
            let input_ids = Tensor::new(&token_ids[start_pos..], &self.device)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token tensor creation failed: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token unsqueeze failed: {}", e)))?;

            let logits = model
                .decode(&input_ids, encoder_hidden_states, start_pos)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Decoder forward failed: {}", e)))?;

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

            let token = logits_processor
                .sample(&logits)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token sampling failed: {}", e)))?;

            token_ids.push(token);

            if index < 5 {
                tracing::trace!(
                    iteration = index,
                    token = token,
                    num_tokens = token_ids.len(),
                    "TrOCR: decode iteration"
                );
            }

            if token == eos_token_id {
                tracing::debug!(
                    iterations = index + 1,
                    num_tokens = token_ids.len(),
                    "TrOCR: reached EOS token"
                );
                break;
            }
        }

        Ok(token_ids)
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
    /// - Image decode fails or exceeds its 100 MiB budget ([`CandleOcrError::Image`])
    /// - Model inference fails
    ///
    /// Resize and inference allocate separately. Use [`Self::process_decoded_image`]
    /// for a raster decoded under your own budget.
    pub fn process_image(&self, image_bytes: &[u8]) -> Result<CandleOcrOutput> {
        if image_bytes.is_empty() {
            return Err(CandleOcrError::UnsupportedConfig("Empty image data".to_string()));
        }

        tracing::debug!(image_size = image_bytes.len(), "TrOCR: preprocessing image");

        let img = decode_image_with_budget(image_bytes, DEFAULT_IMAGE_DECODE_BUDGET, false)?;
        self.process_decoded_image(&img)
    }

    /// Recognize text from a raster already decoded and budgeted by the caller.
    pub fn process_decoded_image(&self, img: &image::DynamicImage) -> Result<CandleOcrOutput> {
        let processor = ImageProcessor::default();
        let image_tensor = processor
            .process_decoded_image(img, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image preprocessing failed: {}", e)))?;

        tracing::debug!(tensor_shape = ?image_tensor.shape().dims(), "TrOCR: image tensor shape after preprocessing");

        let mut model_guard = self.model.lock();
        model_guard.reset_kv_cache();

        tracing::debug!("TrOCR: running encoder forward pass");
        let encoder_hidden_states = model_guard
            .encoder()
            .forward(&image_tensor)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Encoder forward failed: {}", e)))?;

        tracing::debug!(encoder_shape = ?encoder_hidden_states.shape().dims(), "TrOCR: encoder hidden states shape");

        let token_ids = self.decode_tokens(&mut model_guard, &encoder_hidden_states)?;

        let decoded_text = self
            .tokenizer
            .decode(&token_ids, true)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Tokenizer decode failed: {}", e)))?;

        if decoded_text.trim().is_empty() {
            tracing::warn!(num_tokens = token_ids.len(), "TrOCR: decoded text is empty");
        } else {
            tracing::debug!(
                text_len = decoded_text.len(),
                num_tokens = token_ids.len(),
                "TrOCR: decoding complete"
            );
        }

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
    fn test_trocr_variant_revisions_are_immutable_commits() {
        for variant in [
            TrocrVariant::BasePrinted,
            TrocrVariant::LargePrinted,
            TrocrVariant::BaseHandwritten,
            TrocrVariant::LargeHandwritten,
        ] {
            let revision = variant.revision();
            assert_eq!(revision.len(), 40);
            assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
    }

    #[test]
    #[ignore]
    fn test_engine_creation() {
        let device = Device::Cpu;
        let engine = TrocrEngine::new(TrocrVariant::BasePrinted, device).expect("Engine creation failed");
        assert_eq!(engine.variant(), TrocrVariant::BasePrinted);
        assert_eq!(engine.model_kind(), ModelKind::Trocr);
    }

    #[test]
    #[ignore]
    fn test_inference_on_real_image() {
        use std::fs;
        use std::path::Path;

        let image_path = Path::new("../../test_documents/images/ocr_image.jpg");
        if !image_path.exists() {
            tracing::warn!(
                "Test image not found at {}; skipping real inference test",
                image_path.display()
            );
            return;
        }

        let image_bytes = fs::read(image_path).expect("Failed to read test image");

        let device = Device::Cpu;
        let engine = TrocrEngine::new(TrocrVariant::BasePrinted, device).expect("Failed to create TrOCR engine");

        let result = engine.process_image(&image_bytes).expect("OCR inference failed");

        assert!(!result.content.is_empty(), "OCR returned empty text");

        let has_letter = result.content.chars().any(|c| c.is_ascii_alphabetic());
        assert!(
            has_letter,
            "OCR output contains no ASCII letters. Got: {}",
            result.content
        );

        println!("OCR Result:\n{}", result.content);
    }
}
