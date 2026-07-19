//! Document orientation detection implementation using PP-LCNet_x1_0_doc_ori.
//!
//! Detects page-level orientation (0°, 90°, 180°, 270°) for scanned documents
//! and images. Runs through the [`crate::inference`] seam, so it works on either
//! engine: the ORT-backed `auto-rotate` feature or the pure-Rust `auto-rotate-tract`
//! variant (no-ORT targets). The model is engine-neutral either way.
//!
//! Used by ALL OCR backends when `auto_rotate` is enabled in `OcrConfig`.
//! More reliable than Tesseract's `DetectOrientationScript` which crashes
//! on raw images without DPI metadata.

use std::path::PathBuf;

use image::RgbImage;

use crate::Result;
use crate::error::XbergError;
use crate::inference::{InferenceSession, InferenceTensor, default_backend};

use super::types::OrientationResult;

/// HuggingFace repository containing the model.
const HF_REPO_ID: &str = "xberg-io/paddleocr-onnx-models";
const HF_REPO_REVISION: &str = "bfaf0b492cfc1dee0c73245fc5860bfdcf2c3443";
const REMOTE_FILENAME: &str = "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx";
const SHA256: &str = "6b742aebce6f0f7f71f747931ac7becfc7c96c51641e14943b291eeb334e7947";

const INPUT_SIZE: u32 = 224;
const RESIZE_SHORT: u32 = 256;

/// Output labels: index -> degrees.
const ORIENTATION_LABELS: [u32; 4] = [0, 90, 180, 270];

/// PP-LCNet doc_ori outputs ~45% confidence for correct class in a 4-class problem.
/// Uniform baseline is 25%. A threshold of 0.35 provides good discrimination.
pub const MIN_CONFIDENCE: f32 = 0.35;

/// Detects document page orientation using the PP-LCNet model.
///
/// Thread-safe: the model runs behind `&self` through the [`crate::inference`]
/// seam, which owns the session synchronization. The model is downloaded from
/// HuggingFace on first use and cached locally.
#[cfg_attr(alef, alef(skip))]
pub struct DocOrientationDetector {
    session: once_cell::sync::OnceCell<Box<dyn InferenceSession>>,
    cache_dir: PathBuf,
    acceleration: Option<crate::core::config::acceleration::AccelerationConfig>,
}

impl DocOrientationDetector {
    /// Creates a new detector with the given cache directory and acceleration config.
    pub(crate) fn with_acceleration(
        cache_dir: PathBuf,
        accel: Option<crate::core::config::acceleration::AccelerationConfig>,
    ) -> Self {
        Self {
            session: once_cell::sync::OnceCell::new(),
            cache_dir,
            acceleration: accel,
        }
    }

    /// Detect document page orientation.
    ///
    /// Returns the detected orientation (0°, 90°, 180°, 270°) and confidence.
    /// Thread-safe: can be called concurrently from multiple pages.
    pub(crate) fn detect(&self, image: &RgbImage) -> Result<OrientationResult> {
        let session = self.get_or_init_session()?;

        let preprocessed = preprocess(image);
        let input_tensor = normalize(&preprocessed);

        // PP-LCNet's single input is named "x"; read it from the graph rather than
        // hard-coding, so the same call works whichever engine loaded the model.
        let input_name = session
            .input_names()
            .first()
            .cloned()
            .unwrap_or_else(|| "x".to_string());
        let outputs = session
            .run(vec![(input_name, InferenceTensor::F32(input_tensor.into_dyn()))])
            .map_err(|e| XbergError::Ocr {
                message: format!("Doc orientation inference failed: {e}"),
                source: None,
            })?;

        let (_, output_value) = outputs.first().ok_or_else(|| XbergError::Ocr {
            message: "No output from doc orientation model".to_string(),
            source: None,
        })?;

        let scores: Vec<f32> = output_value
            .as_f32()
            .ok_or_else(|| XbergError::Ocr {
                message: "doc orientation output is not an f32 tensor".to_string(),
                source: None,
            })?
            .iter()
            .copied()
            .collect();

        let max_score = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_scores: Vec<f32> = scores.iter().map(|&s| (s - max_score).exp()).collect();
        let sum_exp: f32 = exp_scores.iter().sum();
        let probabilities: Vec<f32> = exp_scores.iter().map(|&e| e / sum_exp).collect();

        let (best_idx, &best_prob) = probabilities
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((0, &0.0));

        let degrees = ORIENTATION_LABELS.get(best_idx).copied().unwrap_or(0);

        Ok(OrientationResult {
            degrees,
            confidence: best_prob,
        })
    }

    /// Resolve the verified ONNX model directly from the Hugging Face cache.
    fn ensure_model(&self) -> Result<PathBuf> {
        crate::model_download::hf_resolve_file(
            HF_REPO_ID,
            REMOTE_FILENAME,
            Some(HF_REPO_REVISION),
            Some(&self.cache_dir),
            Some(SHA256),
        )
        .map_err(|e| XbergError::Plugin {
            message: e,
            plugin_name: "auto-rotate".to_string(),
        })
    }

    /// Get or initialize the inference session (lazy, thread-safe via OnceCell).
    ///
    /// The session (optimization level, thread budget, execution-provider
    /// selection, and CPU fallback) is built by the [`crate::inference`] seam.
    fn get_or_init_session(&self) -> Result<&dyn InferenceSession> {
        let session = self
            .session
            .get_or_try_init(|| -> crate::Result<Box<dyn InferenceSession>> {
                let model_path = self.ensure_model()?;

                let session = default_backend()
                    .load(&model_path, self.acceleration.as_ref())
                    .map_err(|e| XbergError::Ocr {
                        message: format!("Failed to load doc_ori model: {e}"),
                        source: None,
                    })?;

                tracing::info!("Doc orientation model loaded");
                Ok(session)
            })?;
        Ok(session.as_ref())
    }
}

/// Resolve the standard Hugging Face cache directory for the auto-rotate model.
pub(crate) fn resolve_cache_dir() -> PathBuf {
    hf_hub::resolve_cache_dir()
}

/// Detect orientation and return a corrected image if rotation is needed.
///
/// Returns `Ok(Some(rotated_bytes))` if rotation was applied,
/// `Ok(None)` if no rotation needed (0° or low confidence).
#[cfg_attr(alef, alef(skip))]
#[cfg(feature = "paddle-ocr")]
pub(crate) fn detect_and_rotate(detector: &DocOrientationDetector, image_bytes: &[u8]) -> Result<Option<Vec<u8>>> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| XbergError::Ocr {
            message: format!("Failed to load image for orientation detection: {e}"),
            source: None,
        })?
        .to_rgb8();

    let result = detector.detect(&img)?;

    tracing::debug!(
        degrees = result.degrees,
        confidence = result.confidence,
        "Document orientation detected"
    );

    if result.degrees == 0 || result.confidence < MIN_CONFIDENCE {
        return Ok(None);
    }

    let rotated = match result.degrees {
        90 => image::imageops::rotate270(&img),
        180 => image::imageops::rotate180(&img),
        270 => image::imageops::rotate90(&img),
        _ => return Ok(None),
    };

    let mut buf = std::io::Cursor::new(Vec::new());
    rotated
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| XbergError::Ocr {
            message: format!("Failed to encode rotated image: {e}"),
            source: None,
        })?;

    tracing::info!(
        degrees = result.degrees,
        confidence = result.confidence,
        "Auto-rotated document page"
    );

    Ok(Some(buf.into_inner()))
}

/// Resize short side to 256, then center crop to 224×224.
fn preprocess(image: &RgbImage) -> RgbImage {
    let (w, h) = (image.width(), image.height());

    let (new_w, new_h) = if w < h {
        let scale = RESIZE_SHORT as f32 / w as f32;
        (RESIZE_SHORT, (h as f32 * scale).round() as u32)
    } else {
        let scale = RESIZE_SHORT as f32 / h as f32;
        ((w as f32 * scale).round() as u32, RESIZE_SHORT)
    };

    let resized = image::imageops::resize(image, new_w, new_h, image::imageops::FilterType::Triangle);

    let x_offset = (new_w.saturating_sub(INPUT_SIZE)) / 2;
    let y_offset = (new_h.saturating_sub(INPUT_SIZE)) / 2;
    let crop_w = INPUT_SIZE.min(new_w);
    let crop_h = INPUT_SIZE.min(new_h);

    image::imageops::crop_imm(&resized, x_offset, y_offset, crop_w, crop_h).to_image()
}

/// Normalize image to [1, 3, H, W] tensor with ImageNet mean/std in BGR order.
/// PP-LCNet expects BGR input: channel 0=Blue, 1=Green, 2=Red.
fn normalize(image: &RgbImage) -> ndarray::Array4<f32> {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut tensor = ndarray::Array4::<f32>::zeros((1, 3, h, w));

    const BGR_MEAN: [f32; 3] = [0.406 * 255.0, 0.456 * 255.0, 0.485 * 255.0];
    const BGR_NORM: [f32; 3] = [1.0 / (0.225 * 255.0), 1.0 / (0.224 * 255.0), 1.0 / (0.229 * 255.0)];

    for y in 0..h {
        for x in 0..w {
            let pixel = image.get_pixel(x as u32, y as u32);
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;
            tensor[[0, 0, y, x]] = (b - BGR_MEAN[0]) * BGR_NORM[0];
            tensor[[0, 1, y, x]] = (g - BGR_MEAN[1]) * BGR_NORM[1];
            tensor[[0, 2, y, x]] = (r - BGR_MEAN[2]) * BGR_NORM[2];
        }
    }

    tensor
}
