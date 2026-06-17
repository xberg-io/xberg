//! Image preprocessing shared by candle OCR models.
//!
//! Ported from upstream `candle-examples/examples/trocr/image_processor.rs`
//! with bytes-in API (instead of paths) so backends can pass image data
//! directly from `OcrBackend::process_image`.

#![cfg(not(target_arch = "wasm32"))]

use candle_core::{DType, Device, Tensor};

use crate::error::{CandleOcrError, Result};

/// ViT-style image processor: resize to NxN, normalize to mean/std.
///
/// Defaults match TrOCR / PaddleOCR-VL ViT pretraining: 384x384, mean/std `[0.5; 3]`.
#[derive(Debug, Clone)]
pub struct ImageProcessor {
    pub height: u32,
    pub width: u32,
    pub image_mean: [f32; 3],
    pub image_std: [f32; 3],
}

impl Default for ImageProcessor {
    fn default() -> Self {
        Self {
            height: 384,
            width: 384,
            image_mean: [0.5, 0.5, 0.5],
            image_std: [0.5, 0.5, 0.5],
        }
    }
}

impl ImageProcessor {
    /// Decode `image_bytes`, resize, normalize, return `[1, 3, H, W]` float32 on `device`.
    pub fn process(&self, image_bytes: &[u8], device: &Device) -> Result<Tensor> {
        if image_bytes.is_empty() {
            return Err(CandleOcrError::UnsupportedConfig("empty image data".to_string()));
        }

        let img = image::load_from_memory(image_bytes)?;
        let resized = img.resize_exact(self.width, self.height, image::imageops::FilterType::Triangle);
        let rgb = resized.to_rgb8();
        let raw = rgb.into_raw();

        let height = self.height as usize;
        let width = self.width as usize;

        let mean = Tensor::from_vec(self.image_mean.to_vec(), (3, 1, 1), device)?;
        let std = Tensor::from_vec(self.image_std.to_vec(), (3, 1, 1), device)?;

        let data = Tensor::from_vec(raw, &[height, width, 3], device)?.permute((2, 0, 1))?;
        let normalized = (data.to_dtype(DType::F32)? / 255.)?
            .broadcast_sub(&mean)?
            .broadcast_div(&std)?;

        Ok(normalized.unsqueeze(0)?)
    }
}
