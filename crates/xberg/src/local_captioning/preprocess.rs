use image::imageops::FilterType;

use super::policy::invalid;
use super::{LocalCaptioningConfig, LocalCaptioningFormat};

/// How the pinned processor describes input images and the text-side image token count.
pub(super) struct PreprocessConfig {
    pub(super) edge: u32,
    pub(super) image_seq_len: u32,
    pub(super) mean: [f32; 3],
    pub(super) std: [f32; 3],
    pub(super) rescale: f32,
    filter: FilterType,
}

/// A batch of normalized `[num_images, 3, edge, edge]` pixel tensors in CHW order.
pub(super) struct ImageBatch {
    pub(super) pixels: Vec<f32>,
    pub(super) num_images: usize,
    pub(super) edge: u32,
}

fn value_f32(value: &serde_json::Value, key: &str, fallback: Option<f32>) -> crate::Result<f32> {
    match value.get(key).and_then(serde_json::Value::as_f64) {
        Some(number) => Ok(number as f32),
        None => fallback.ok_or_else(|| invalid(format!("Local captioning processor config lacks a numeric {key}"))),
    }
}

fn value_u32(value: &serde_json::Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|number| u32::try_from(number).ok())
}

fn channel_triplet(value: &serde_json::Value, key: &str, fallback: [f32; 3]) -> crate::Result<[f32; 3]> {
    let Some(array) = value.get(key).and_then(serde_json::Value::as_array) else {
        return Ok(fallback);
    };
    let channels: Vec<f32> = array
        .iter()
        .filter_map(serde_json::Value::as_f64)
        .map(|number| number as f32)
        .collect();
    channels
        .try_into()
        .map_err(|_| invalid(format!("Local captioning processor config has a malformed {key}")))
}

fn filter(resample: Option<u32>) -> FilterType {
    match resample {
        Some(1) => FilterType::Lanczos3,
        Some(2) => FilterType::Triangle,
        Some(3) => FilterType::CatmullRom,
        _ => FilterType::Triangle,
    }
}

impl PreprocessConfig {
    pub(super) fn from_json(
        format: LocalCaptioningFormat,
        preprocessor: &serde_json::Value,
        processor: &serde_json::Value,
    ) -> crate::Result<Self> {
        let mean = channel_triplet(preprocessor, "image_mean", [0.5, 0.5, 0.5])?;
        let std = channel_triplet(preprocessor, "image_std", [0.5, 0.5, 0.5])?;
        let rescale = value_f32(preprocessor, "rescale_factor", Some(1.0 / 255.0))?;
        if !rescale.is_finite() || rescale <= 0.0 || mean.iter().chain(&std).any(|value| !value.is_finite()) {
            return Err(invalid("Local captioning preprocessing constants must be finite"));
        }
        let filter = filter(value_u32(preprocessor, "resample"));
        match format {
            LocalCaptioningFormat::SmolVlm => {
                let edge = preprocessor
                    .get("max_image_size")
                    .and_then(|size| value_u32(size, "longest_edge"))
                    .or_else(|| {
                        preprocessor
                            .get("size")
                            .and_then(|size| value_u32(size, "longest_edge"))
                    })
                    .ok_or_else(|| invalid("Local captioning preprocessor config lacks an image edge"))?;
                let image_seq_len = value_u32(processor, "image_seq_len")
                    .ok_or_else(|| invalid("SmolVLM processor config lacks image_seq_len"))?;
                Ok(Self {
                    edge,
                    image_seq_len,
                    mean,
                    std,
                    rescale,
                    filter,
                })
            }
            LocalCaptioningFormat::Florence2 => {
                let size = preprocessor
                    .get("size")
                    .ok_or_else(|| invalid("Florence-2 preprocessor config lacks a size"))?;
                let edge = value_u32(size, "height")
                    .ok_or_else(|| invalid("Florence-2 preprocessor config lacks a size height"))?;
                let image_seq_len = value_u32(preprocessor, "image_seq_length")
                    .ok_or_else(|| invalid("Florence-2 preprocessor config lacks image_seq_length"))?;
                Ok(Self {
                    edge,
                    image_seq_len,
                    mean,
                    std,
                    rescale,
                    filter,
                })
            }
        }
    }

    fn normalize(&self, image: &image::RgbImage) -> Vec<f32> {
        let (width, height) = image.dimensions();
        let mut pixels = vec![0.0f32; 3 * width as usize * height as usize];
        let plane = width as usize * height as usize;
        for (index, pixel) in image.pixels().enumerate() {
            for channel in 0..3 {
                let value = f32::from(pixel.0[channel]) * self.rescale;
                pixels[channel * plane + index] = (value - self.mean[channel]) / self.std[channel];
            }
        }
        pixels
    }
}

fn resize_exact(image: &image::RgbImage, edge: u32, filter: FilterType) -> image::RgbImage {
    image::imageops::resize(image, edge, edge, filter)
}

pub(super) fn decode_png_or_common(bytes: &[u8], max_pixels: u32) -> crate::Result<image::RgbImage> {
    let side = (f64::from(max_pixels).sqrt() as u32).max(1);
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| invalid(format!("Local captioning image format: {error}")))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(side);
    limits.max_image_height = Some(side);
    limits.max_alloc = Some(u64::from(max_pixels).saturating_mul(4));
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|error| invalid(format!("Local captioning image decode: {error}")))?;
    Ok(image.to_rgb8())
}

pub(super) fn preprocess(
    config: &LocalCaptioningConfig,
    format: LocalCaptioningFormat,
    preprocess: &PreprocessConfig,
    image: &image::RgbImage,
) -> crate::Result<ImageBatch> {
    if u64::from(image.width()) * u64::from(image.height()) > u64::from(config.limits.max_pixels) {
        return Err(invalid("Local captioning image exceeds the configured pixel limit"));
    }
    let mut frames = Vec::new();
    match format {
        // One global square frame, matching the pinned processor's no-splitting path. Tiled
        // splitting is not implemented yet; the qualification gap is recorded in the module docs.
        LocalCaptioningFormat::SmolVlm => {
            frames.push(resize_exact(image, preprocess.edge, preprocess.filter));
        }
        LocalCaptioningFormat::Florence2 => {
            frames.push(resize_exact(image, preprocess.edge, preprocess.filter));
        }
    }
    let num_images = frames.len();
    let mut pixels = Vec::with_capacity(num_images * 3 * (preprocess.edge as usize).pow(2));
    for frame in &frames {
        pixels.extend(preprocess.normalize(frame));
    }
    Ok(ImageBatch {
        pixels,
        num_images,
        edge: preprocess.edge,
    })
}
