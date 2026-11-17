//! Image preprocessing for GLM-OCR.
//!
//! Decodes image bytes, smart-resizes them to a multiple of `patch_size * t_patch_size`
//! within the configured min/max pixel budget, and normalises with the CogViT mean/std.
//! Returns a `(1, C, H, W)` pixel tensor in the requested dtype plus a `(1, 3)`
//! `[T, H_patches, W_patches]` grid descriptor that downstream code uses to compute the
//! number of vision tokens.

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
    use crate::models::image_processor::{DEFAULT_IMAGE_DECODE_BUDGET, decode_image_with_budget};

    /// Preprocess image bytes into a `(pixel_values, grid_thw)` pair compatible
    /// with the CogViT encoder. Encoded input, decode and RGB conversion share a
    /// 100 MiB budget. The resize and tensor allocations are separate.
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
        preprocess_with_budget(image_bytes, config, device, dtype, DEFAULT_IMAGE_DECODE_BUDGET)
    }

    pub(super) fn preprocess_with_budget(
        image_bytes: &[u8],
        config: &PreprocessConfig,
        device: &Device,
        dtype: DType,
        decode_budget: u64,
    ) -> Result<(Tensor, Tensor)> {
        let img = decode_image_with_budget(image_bytes, decode_budget, true)?.into_rgb8();
        preprocess_rgb(&img, config, device, dtype)
    }

    pub fn preprocess_rgb(
        img: &image::RgbImage,
        config: &PreprocessConfig,
        device: &Device,
        dtype: DType,
    ) -> Result<(Tensor, Tensor)> {
        let (width, height) = (img.width() as usize, img.height() as usize);

        let factor = config.patch_size * config.t_patch_size;
        let (new_height, new_width) = smart_resize(height, width, factor, config.min_pixels, config.max_pixels)?;

        let resized = image::imageops::resize(
            img,
            new_width as u32,
            new_height as u32,
            image::imageops::FilterType::CatmullRom,
        );

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

        let pixel_values = normalized
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?
            .to_dtype(dtype)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Target dtype: {}", e)))?;

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

        if h < factor {
            w = (w * factor + h / 2) / h;
            h = factor;
        }
        if w < factor {
            h = (h * factor + w / 2) / w;
            w = factor;
        }

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

        let mut h_bar = ((h + factor / 2) / factor) * factor;
        let mut w_bar = ((w + factor / 2) / factor) * factor;

        let total_pixels = h_bar * w_bar;

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

#[cfg(not(target_arch = "wasm32"))]
pub(super) use imp::preprocess_rgb;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use candle_core::{DType, Device};

    #[test]
    fn standalone_glm_preprocessing_enforces_the_decode_budget() {
        use crate::CandleOcrError;
        use crate::models::glm_ocr::tests::small_rgb_png;

        let (png, budget) = small_rgb_png();
        let config = PreprocessConfig::default();

        let Err(CandleOcrError::Image(image::ImageError::Limits(error))) =
            imp::preprocess_with_budget(&png, &config, &Device::Cpu, DType::F32, budget)
        else {
            panic!("valid PNG must exceed the test budget with a typed image limit");
        };
        assert_eq!(error.kind(), image::error::LimitErrorKind::InsufficientMemory);

        let (pixels, grid) = preprocess(&png, &config, &Device::Cpu, DType::F32).unwrap();
        assert_eq!(pixels.dims(), &[1, 3, 112, 112]);
        assert_eq!(grid.to_vec2::<u32>().unwrap(), vec![vec![1, 8, 8]]);
    }

    #[test]
    fn standalone_glm_public_preprocess_uses_the_default_decode_budget() {
        use crate::models::glm_ocr::tests::{assert_uses_default_image_budget, small_rgb_png};

        let (png, _) = small_rgb_png();
        let config = PreprocessConfig::default();
        assert_uses_default_image_budget(|| preprocess(&png, &config, &Device::Cpu, DType::F32));
    }

    #[test]
    fn encoded_and_decoded_inputs_produce_equal_visual_tensors() {
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(28, 28, |x, y| {
            image::Rgba([x as u8, y as u8, 100, 128])
        }));
        let mut png = std::io::Cursor::new(Vec::new());
        img.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let config = PreprocessConfig::default();
        let (encoded, encoded_grid) = preprocess(png.get_ref(), &config, &Device::Cpu, DType::F32).unwrap();
        let (rgb, rgb_grid) = preprocess_rgb(&img.to_rgb8(), &config, &Device::Cpu, DType::F32).unwrap();
        assert_eq!(
            encoded.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            rgb.flatten_all().unwrap().to_vec1::<f32>().unwrap()
        );
        assert_eq!(
            encoded_grid.to_vec2::<u32>().unwrap(),
            rgb_grid.to_vec2::<u32>().unwrap()
        );
    }
}
