//! Image preprocessing shared by candle OCR models.
//!
//! Ported from upstream `candle-examples/examples/trocr/image_processor.rs`
//! with bytes-in API (instead of paths) so backends can pass image data
//! directly from `OcrBackend::process_image`.

#![cfg(not(target_arch = "wasm32"))]

use candle_core::{DType, Device, Tensor};

use crate::error::{CandleOcrError, Result};

#[cfg(any(feature = "paddleocr-vl", feature = "deepseek-ocr"))]
pub(super) const DEFAULT_IMAGE_DECODE_BUDGET: u64 = 100 * 1024 * 1024;

#[cfg(any(feature = "paddleocr-vl", feature = "deepseek-ocr"))]
pub(super) fn decode_image_with_budget(bytes: &[u8], budget: u64) -> image::ImageResult<image::DynamicImage> {
    let encoded_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(budget.checked_sub(encoded_bytes).ok_or_else(|| {
        image::ImageError::Limits(image::error::LimitError::from_kind(
            image::error::LimitErrorKind::InsufficientMemory,
        ))
    })?);
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    reader.limits(limits);
    reader.decode()
}

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

/// Decode `image_bytes` and return its `(width, height)` in pixels.
///
/// Callers that expect a specific input granularity (e.g. TrOCR expects a single
/// cropped text line, not a full page) can use this to inspect the raw raster shape
/// *before* it gets force-resized to a fixed square by [`ImageProcessor::process`],
/// which silently discards the original aspect ratio and would otherwise hide a
/// granularity mismatch behind a successful-looking (but fabricated) decode.
pub fn dimensions(image_bytes: &[u8]) -> Result<(u32, u32)> {
    if image_bytes.is_empty() {
        return Err(CandleOcrError::UnsupportedConfig("empty image data".to_string()));
    }
    let img = image::load_from_memory(image_bytes)?;
    Ok((img.width(), img.height()))
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

#[cfg(all(test, any(feature = "paddleocr-vl", feature = "deepseek-ocr")))]
mod decode_tests {
    use super::decode_image_with_budget;

    #[test]
    fn encoded_and_decoded_bytes_must_fit_standalone_decode_budget() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(64, 64)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let budget = 4 * 1024;
        assert!(png.get_ref().len() < usize::try_from(budget).unwrap());
        assert!(matches!(
            decode_image_with_budget(png.get_ref(), budget),
            Err(image::ImageError::Limits(_))
        ));
        let image = decode_image_with_budget(png.get_ref(), 1 << 20).unwrap();
        assert_eq!((image.width(), image.height()), (64, 64));
    }
}
