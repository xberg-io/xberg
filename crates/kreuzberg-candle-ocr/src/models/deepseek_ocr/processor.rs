//! Image preprocessing and tokenization for DeepSeek-OCR.
//!
//! Handles dynamic image preprocessing (resizing with edge padding and cropping),
//! tokenization of text prompts, and assembly of multimodal input for the model.

use candle_core::{DType, Device};

use crate::error::Result;

/// DeepSeek-OCR input processor.
///
/// Manages image preprocessing pipeline and text tokenization for the model.
/// Supports both cropped and non-cropped image processing modes.
#[cfg_attr(alef, alef(skip))]
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DeepseekOCRProcessor {
    /// Computation device.
    device: Device,
    /// Data type for image tensors.
    dtype: DType,
    /// Text token representing an image placeholder.
    image_token: String,
    /// Token ID for the image placeholder.
    image_token_id: u32,
    /// Patch size of the vision encoder.
    patch_size: u32,
    /// Downsampling ratio in the vision pipeline.
    downsample_ratio: u32,
    /// Model version (1 or 2).
    version: usize,
}

impl DeepseekOCRProcessor {
    /// Create a new DeepSeek-OCR processor.
    ///
    /// # Arguments
    ///
    /// - `device`: Computation device.
    /// - `dtype`: Data type for tensors.
    /// - `version`: Model version (1 or 2).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CandleOcrError`] on device or tensor creation failure.
    pub fn new(device: &Device, dtype: DType, version: usize) -> Result<Self> {
        Ok(Self {
            device: device.clone(),
            dtype,
            image_token: "<image>".to_string(),
            image_token_id: 128_815,
            patch_size: 16,
            downsample_ratio: 4,
            version,
        })
    }

    /// Get the image token ID.
    #[must_use]
    pub fn image_token_id(&self) -> u32 {
        self.image_token_id
    }

    /// Get the model version.
    #[must_use]
    pub fn version(&self) -> usize {
        self.version
    }

    /// Get the computation device.
    #[must_use]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get the data type for tensors.
    #[must_use]
    pub fn dtype(&self) -> DType {
        self.dtype
    }
}
