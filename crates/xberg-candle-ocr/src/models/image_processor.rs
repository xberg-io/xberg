//! Image preprocessing shared by candle OCR models.
//!
//! Ported from upstream `candle-examples/examples/trocr/image_processor.rs`
//! with bytes-in API (instead of paths) so backends can pass image data
//! directly from `OcrBackend::process_image`.

#![cfg(not(target_arch = "wasm32"))]

use candle_core::{DType, Device, Tensor};

use crate::error::{CandleOcrError, Result};

pub(super) const DEFAULT_IMAGE_DECODE_BUDGET: u64 = 100 * 1024 * 1024;

pub(super) fn decode_image_with_budget(
    bytes: &[u8],
    budget: u64,
    convert_to_rgb8: bool,
) -> image::ImageResult<image::DynamicImage> {
    use image::ImageDecoder as _;

    let insufficient_memory = || {
        image::ImageError::Limits(image::error::LimitError::from_kind(
            image::error::LimitErrorKind::InsufficientMemory,
        ))
    };
    let encoded_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    // The JPEG decoder holds its own copy of the encoded input during decoding. ~keep
    let encoded_live_bytes = if reader.format() == Some(image::ImageFormat::Jpeg) {
        encoded_bytes.checked_mul(2).ok_or_else(insufficient_memory)?
    } else {
        encoded_bytes
    };
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(budget.checked_sub(encoded_live_bytes).ok_or_else(insufficient_memory)?);
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    limits.check_dimensions(width, height)?;
    limits.reserve(decoder.total_bytes())?;
    if convert_to_rgb8 && decoder.color_type() != image::ColorType::Rgb8 {
        let rgb_bytes = (u64::from(width) * u64::from(height))
            .checked_mul(u64::from(image::ColorType::Rgb8.bytes_per_pixel()))
            .ok_or_else(insufficient_memory)?;
        // Conversion follows decoding, so this checks a separate peak from decoder scratch space. ~keep
        limits.clone().reserve(rgb_bytes)?;
    }
    decoder.set_limits(limits)?;
    image::DynamicImage::from_decoder(decoder)
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

/// Read the image header and return its `(width, height)` in pixels.
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
    Ok(image::ImageReader::new(std::io::Cursor::new(image_bytes))
        .with_guessed_format()
        .map_err(image::ImageError::from)?
        .into_dimensions()?)
}

impl ImageProcessor {
    /// Decode `image_bytes` with a 100 MiB budget, resize and normalize into a
    /// `[1, 3, H, W]` float32 tensor. Resize and tensor allocations are separate.
    pub fn process(&self, image_bytes: &[u8], device: &Device) -> Result<Tensor> {
        if image_bytes.is_empty() {
            return Err(CandleOcrError::UnsupportedConfig("empty image data".to_string()));
        }

        let img = decode_image_with_budget(image_bytes, DEFAULT_IMAGE_DECODE_BUDGET, false)?;
        self.process_decoded_image(&img, device)
    }

    pub(super) fn process_decoded_image(&self, img: &image::DynamicImage, device: &Device) -> Result<Tensor> {
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

#[cfg(test)]
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
            decode_image_with_budget(png.get_ref(), budget, false),
            Err(image::ImageError::Limits(_))
        ));
        let image = decode_image_with_budget(png.get_ref(), 1 << 20, false).unwrap();
        assert_eq!((image.width(), image.height()), (64, 64));
    }

    #[test]
    fn standalone_raster_budget_rejects_before_reading_truncated_pixels() {
        use image::ImageDecoder as _;

        let mut bmp = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(32, 32)
            .write_to(&mut bmp, image::ImageFormat::Bmp)
            .unwrap();
        let mut truncated = bmp.into_inner();
        let pixel_offset = u32::from_le_bytes(truncated[10..14].try_into().unwrap());
        truncated.truncate(usize::try_from(pixel_offset).unwrap());
        let decoder = image::codecs::bmp::BmpDecoder::new(std::io::Cursor::new(&truncated)).unwrap();
        assert_eq!(decoder.dimensions(), (32, 32));
        assert_eq!(super::dimensions(&truncated).unwrap(), (32, 32));
        let budget = 1024;
        let encoded_bytes = u64::try_from(truncated.len()).unwrap();
        assert!(encoded_bytes < budget && budget < encoded_bytes + decoder.total_bytes());

        assert!(matches!(
            decode_image_with_budget(&truncated, 1 << 20, false),
            Err(image::ImageError::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof
        ));
        let Err(image::ImageError::Limits(error)) = decode_image_with_budget(&truncated, budget, false) else {
            panic!("preflight must reject the raster before reading pixel data");
        };
        assert_eq!(error.kind(), image::error::LimitErrorKind::InsufficientMemory);
    }

    #[test]
    fn grayscale_and_rgb_rasters_must_fit_when_standalone_decode_converts_color() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_luma8(64, 64)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let budget = 15 * 1024;
        assert!(decode_image_with_budget(png.get_ref(), budget, false).is_ok());
        assert!(matches!(
            decode_image_with_budget(png.get_ref(), budget, true),
            Err(image::ImageError::Limits(_))
        ));
        assert!(decode_image_with_budget(png.get_ref(), 1 << 20, true).is_ok());
    }

    #[test]
    fn standalone_jpeg_decoder_copy_counts_toward_the_budget() {
        let mut jpeg = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(1, 1)
            .write_to(&mut jpeg, image::ImageFormat::Jpeg)
            .unwrap();
        let budget = u64::try_from(jpeg.get_ref().len()).unwrap() + 3;
        assert!(matches!(
            decode_image_with_budget(jpeg.get_ref(), budget, false),
            Err(image::ImageError::Limits(_))
        ));
        assert!(decode_image_with_budget(jpeg.get_ref(), 1 << 20, false).is_ok());
    }
}
