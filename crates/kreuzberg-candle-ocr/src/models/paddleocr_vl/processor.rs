//! Image preprocessing for PaddleOCR-VL.
//!
//! Handles image loading, resizing, and normalization with JSON-driven configuration.
//! Adapted from aha's paddleocr_vl module.

use candle_core::{DType, Device, Shape, Tensor};
use image::DynamicImage;

use crate::CandleOcrError;
use crate::error::Result;

use super::config::PaddleOCRVLPreprocessorConfig;

/// Processor for PaddleOCR-VL image preprocessing.
pub struct PaddleOCRVLProcessor {
    process_cfg: PaddleOCRVLPreprocessorConfig,
    device: Device,
    dtype: DType,
}

impl PaddleOCRVLProcessor {
    /// Create a new processor with the given configuration.
    pub fn new(config: PaddleOCRVLPreprocessorConfig, device: &Device, dtype: DType) -> Result<Self> {
        Ok(Self {
            process_cfg: config,
            device: device.clone(),
            dtype,
        })
    }

    /// Process a single image with mean/std normalization.
    pub fn process_img(&self, img: &DynamicImage, img_mean: &Tensor, img_std: &Tensor) -> Result<Tensor> {
        let img_h = img.height();
        let img_w = img.width();

        // Resize to multiple of (patch_size * merge_size)
        let (resize_h, resize_w) = img_smart_resize(
            img_h,
            img_w,
            (self.process_cfg.patch_size * self.process_cfg.merge_size) as u32,
            self.process_cfg.min_pixels,
            self.process_cfg.max_pixels,
        )?;

        let img = img.resize_exact(resize_w, resize_h, image::imageops::FilterType::CatmullRom);
        let img_tensor = img_transform(&img, img_mean, img_std, &self.device, self.dtype)?;

        // (c, h, w) => (1, c, h, w)
        let img_tensor = img_tensor
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze failed: {}", e)))?;

        Ok(img_tensor)
    }

    /// Process vision tensor into patch-based representation.
    pub fn process_vision_tensor(&self, img_tensor: &Tensor) -> Result<(Tensor, Tensor)> {
        let channel = img_tensor
            .dim(1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get channel dim: {}", e)))?;

        // img_tensor.dim[0] = 1, temporal_patch_size = 1, grid_t = 1
        let grid_t = img_tensor
            .dim(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get grid_t: {}", e)))?
            / self.process_cfg.temporal_patch_size;
        let grid_h = img_tensor
            .dim(2)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get grid_h: {}", e)))?
            / self.process_cfg.patch_size;
        let grid_w = img_tensor
            .dim(3)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get grid_w: {}", e)))?
            / self.process_cfg.patch_size;

        let shape = Shape::from(vec![
            grid_t,
            self.process_cfg.temporal_patch_size,
            channel,
            grid_h,
            self.process_cfg.patch_size,
            grid_w,
            self.process_cfg.patch_size,
        ]);

        let img_tensor = img_tensor
            .reshape(shape)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Initial reshape: {}", e)))?;

        // Permute: (grid_t, grid_h, grid_w, channel, temporal_patch_size, patch_size, patch_size)
        let img_tensor = img_tensor
            .permute(vec![0, 3, 5, 2, 1, 4, 6])
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Permute: {}", e)))?;

        let img_tensor = img_tensor
            .reshape((
                grid_t * grid_h * grid_w,
                channel,
                self.process_cfg.patch_size,
                self.process_cfg.patch_size,
            ))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Final reshape: {}", e)))?
            .contiguous()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Contiguous: {}", e)))?;

        let grid_thw = Tensor::new(&[[grid_t as u32, grid_h as u32, grid_w as u32]], &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid tensor: {}", e)))?;

        Ok((img_tensor, grid_thw))
    }

    /// Process multiple images for batch processing.
    pub fn process_images(
        &self,
        imgs: &[DynamicImage],
        img_mean: &Tensor,
        img_std: &Tensor,
    ) -> Result<(Tensor, Tensor)> {
        let mut pixel_values_vec = Vec::new();
        let mut vision_grid_thws_vec = Vec::new();

        for img in imgs {
            let img_tensor = self.process_img(img, img_mean, img_std)?;
            let (img_tensor, grid_thw) = self.process_vision_tensor(&img_tensor)?;
            pixel_values_vec.push(img_tensor);
            vision_grid_thws_vec.push(grid_thw);
        }

        let pixel_values = Tensor::cat(&pixel_values_vec, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat pixel values: {}", e)))?;
        let vision_grid_thws = Tensor::cat(&vision_grid_thws_vec, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat grid thws: {}", e)))?;

        Ok((pixel_values, vision_grid_thws))
    }
}

/// Smart resize algorithm matching the original preprocessor.
fn img_smart_resize(img_h: u32, img_w: u32, factor: u32, min_pixels: u32, max_pixels: u32) -> Result<(u32, u32)> {
    let mut h = img_h as f64;
    let mut w = img_w as f64;
    let factor = factor as f64;

    // Handle tiny images
    if h < factor {
        w = (w * factor + h / 2.0) / h;
        h = factor;
    }
    if w < factor {
        h = (h * factor + w / 2.0) / w;
        w = factor;
    }

    // Check aspect ratio constraint
    let aspect = if h > w { h / w } else { w / h };
    if aspect > 200.0 {
        return Err(CandleOcrError::UnsupportedConfig(format!(
            "Aspect ratio {:.1} exceeds maximum of 200",
            aspect
        )));
    }

    // Round to nearest multiple of factor
    let mut h_bar = ((h + factor / 2.0) / factor).floor() * factor;
    let mut w_bar = ((w + factor / 2.0) / factor).floor() * factor;

    let total_pixels = h_bar * w_bar;

    if total_pixels > max_pixels as f64 {
        // Scale down to fit within max_pixels
        let beta = ((h * w) / max_pixels as f64).sqrt();
        h_bar = ((h / beta / factor).floor()) * factor;
        w_bar = ((w / beta / factor).floor()) * factor;
    } else if total_pixels < min_pixels as f64 {
        // Scale up to meet min_pixels
        let beta = (min_pixels as f64 / (h * w)).sqrt();
        h_bar = ((h * beta / factor).ceil()) * factor;
        w_bar = ((w * beta / factor).ceil()) * factor;
    }

    Ok((h_bar as u32, w_bar as u32))
}

/// Transform image to normalized tensor.
fn img_transform(
    img: &DynamicImage,
    img_mean: &Tensor,
    img_std: &Tensor,
    device: &Device,
    dtype: DType,
) -> Result<Tensor> {
    let img_rgb8 = img.to_rgb8();
    let raw: Vec<u8> = img_rgb8.into_raw();
    let height = img.height() as usize;
    let width = img.width() as usize;

    let tensor = Tensor::new(raw.as_slice(), device)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Create tensor: {}", e)))?
        .reshape((height, width, 3))
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape to hwc: {}", e)))?
        .permute(vec![2, 0, 1])
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Permute to chw: {}", e)))?
        .to_dtype(DType::F32)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Convert to f32: {}", e)))?
        .affine(1.0 / 255.0, 0.0)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Normalize to [0,1]: {}", e)))?
        .broadcast_sub(img_mean)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Subtract mean: {}", e)))?
        .broadcast_div(img_std)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Divide by std: {}", e)))?
        .to_dtype(dtype)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Convert dtype: {}", e)))?;

    Ok(tensor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_resize_upscale() {
        // Test upscaling to minimum pixels
        let (h, w) = img_smart_resize(100, 100, 28, 147_384, 2_822_400).unwrap();
        assert_eq!(h % 28, 0);
        assert_eq!(w % 28, 0);
        assert!(h * w >= 147_384);
    }

    #[test]
    fn test_smart_resize_downscale() {
        // Test downscaling to maximum pixels
        let (h, w) = img_smart_resize(2000, 2000, 28, 147_384, 2_822_400).unwrap();
        assert_eq!(h % 28, 0);
        assert_eq!(w % 28, 0);
        assert!(h * w <= 2_822_400);
    }

    #[test]
    fn test_aspect_ratio_check() {
        // Test aspect ratio constraint
        let result = img_smart_resize(100, 30000, 28, 147_384, 2_822_400);
        assert!(result.is_err());
    }
}
