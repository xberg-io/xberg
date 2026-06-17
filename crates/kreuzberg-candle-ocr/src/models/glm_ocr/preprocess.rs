//! Image preprocessing for GLM-OCR.
//!
//! Phase 1 stub. GLM-OCR uses CogViT-style patch tiling with a configurable
//! `t_patch_size` and a max-pixel budget. The output is a normalised
//! `(1, C, H, W)` BF16 pixel tensor and a grid descriptor that downstream
//! code uses to compute the number of vision tokens.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PreprocessConfig {
    pub patch_size: usize,
    pub t_patch_size: usize,
    pub min_pixels: usize,
    pub max_pixels: usize,
    pub image_mean: [f32; 3],
    pub image_std: [f32; 3],
}

impl Default for PreprocessConfig {
    // Defaults pulled from https://huggingface.co/zai-org/GLM-OCR/raw/main/preprocessor_config.json
    // `size.shortest_edge` = 12_544 and `size.longest_edge` = 9_633_792 are pixel-count
    // budgets, not edge lengths. `merge_size: 2` is consumed by the connector.
    fn default() -> Self {
        Self {
            patch_size: 14,
            t_patch_size: 2,
            min_pixels: 12_544,
            max_pixels: 9_633_792,
            image_mean: [0.481_454_66, 0.457_827_5, 0.408_210_73],
            image_std: [0.268_629_54, 0.261_302_6, 0.275_777_1],
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use candle_core::{DType, Device, Tensor};

    use super::PreprocessConfig;
    use crate::CandleOcrError;
    use crate::error::Result;

    /// Preprocess image bytes into a `(pixel_values, grid_thw)` pair compatible
    /// with the CogViT encoder.
    ///
    /// Returns:
    /// - `pixel_values`: `(1, 3, H, W)` tensor in the specified dtype, normalized
    /// - `grid_thw`: `(1, 3)` u32 tensor with `[T, H_patches, W_patches]`
    pub fn preprocess(
        image_bytes: &[u8],
        config: &PreprocessConfig,
        device: &Device,
        dtype: DType,
    ) -> Result<(Tensor, Tensor)> {
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image decode: {}", e)))?;

        let img = img.to_rgb8();
        let (width, height) = (img.width() as usize, img.height() as usize);

        // Smart resize to fit within min_pixels/max_pixels bounds, rounded to
        // multiples of patch_size. T is always 1 for static images.
        let factor = config.patch_size * config.t_patch_size;
        let (new_height, new_width) = smart_resize(height, width, factor, config.min_pixels, config.max_pixels)?;

        // Resize: use CatmullRom as PIL's BICUBIC equivalent.
        let resized = image::imageops::resize(
            &img,
            new_width as u32,
            new_height as u32,
            image::imageops::FilterType::CatmullRom,
        );

        // Convert to tensor and normalize: (x / 255 - mean) / std
        let raw: Vec<u8> = resized.into_raw();
        let mean_t = Tensor::from_vec(config.image_mean.to_vec(), (3, 1, 1), device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Mean tensor: {}", e)))?;
        let std_t = Tensor::from_vec(config.image_std.to_vec(), (3, 1, 1), device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Std tensor: {}", e)))?;

        let normalized = Tensor::from_vec(raw, &[new_height, new_width, 3], device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Raw tensor: {}", e)))?
            .permute((2, 0, 1))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Permute: {}", e)))?
            .to_dtype(candle_core::DType::F32)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("F32 cast: {}", e)))?
            .affine(1.0 / 255.0, 0.0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Scale [0,1]: {}", e)))?
            .broadcast_sub(&mean_t)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Sub mean: {}", e)))?
            .broadcast_div(&std_t)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Div std: {}", e)))?;

        // Add batch dimension and convert to target dtype.
        let pixel_values = normalized
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?
            .to_dtype(dtype)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Target dtype: {}", e)))?;

        // Grid descriptor: [T=1, height_patches, width_patches]
        let h_patches = (new_height / config.patch_size) as u32;
        let w_patches = (new_width / config.patch_size) as u32;
        let grid_thw = Tensor::new(&[[1u32, h_patches, w_patches]], device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid tensor: {}", e)))?;

        Ok((pixel_values, grid_thw))
    }

    /// Smart resize matching the upstream Glm46VImageProcessor.
    ///
    /// Ensures the output is:
    /// - At least `factor` pixels in each dimension
    /// - Between `min_pixels` and `max_pixels` total
    /// - Rounded to multiples of `factor`
    /// - Aspect ratio constrained to 200:1 max
    fn smart_resize(
        height: usize,
        width: usize,
        factor: usize,
        min_pixels: usize,
        max_pixels: usize,
    ) -> Result<(usize, usize)> {
        let mut h = height;
        let mut w = width;

        // Ensure minimum dimension size.
        if h < factor {
            w = (w * factor + h / 2) / h;
            h = factor;
        }
        if w < factor {
            h = (h * factor + w / 2) / w;
            w = factor;
        }

        // Check aspect ratio constraint.
        let aspect = if h > w {
            h as f64 / w as f64
        } else {
            w as f64 / h as f64
        };
        if aspect > 200.0 {
            return Err(CandleOcrError::UnsupportedConfig(format!(
                "Aspect ratio {:.1} exceeds 200",
                aspect
            )));
        }

        // Round to nearest multiple of factor.
        let mut h_bar = ((h + factor / 2) / factor) * factor;
        let mut w_bar = ((w + factor / 2) / factor) * factor;

        let total_pixels = h_bar * w_bar;

        // Scale to fit pixel budget.
        if total_pixels > max_pixels {
            let beta = ((h * w) as f64 / max_pixels as f64).sqrt();
            h_bar = ((h as f64 / beta / factor as f64).floor() as usize) * factor;
            w_bar = ((w as f64 / beta / factor as f64).floor() as usize) * factor;
        } else if total_pixels < min_pixels {
            let beta = (min_pixels as f64 / (h * w) as f64).sqrt();
            h_bar = ((h as f64 * beta / factor as f64).ceil() as usize) * factor;
            w_bar = ((w as f64 * beta / factor as f64).ceil() as usize) * factor;
        }

        Ok((h_bar, w_bar))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use imp::preprocess;
