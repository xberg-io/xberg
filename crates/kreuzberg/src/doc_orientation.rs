//! Document orientation detection using PP-LCNet_x1_0_doc_ori.
//!
//! Detects page-level orientation (0°, 90°, 180°, 270°) for scanned documents
//! and images. Gated behind the `auto-rotate` feature.
//!
//! Used by ALL OCR backends when `auto_rotate` is enabled in `OcrConfig`.
//! More reliable than Tesseract's `DetectOrientationScript` which crashes
//! on raw images without DPI metadata.

use std::fs;
use std::path::PathBuf;

use image::RgbImage;
use ort::session::Session;
use ort::session::builder::{GraphOptimizationLevel, SessionBuilder};
use ort::value::Tensor;

use crate::Result;
use crate::error::KreuzbergError;

/// HuggingFace repository containing the model.
const HF_REPO_ID: &str = "Kreuzberg/paddleocr-onnx-models";
const REMOTE_FILENAME: &str = "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx";
const SHA256: &str = "6b742aebce6f0f7f71f747931ac7becfc7c96c51641e14943b291eeb334e7947";

// PP-LCNet preprocessing constants.
// Input: resize short side to 256, center crop 224×224, ImageNet normalize (BGR).
const INPUT_SIZE: u32 = 224;
const RESIZE_SHORT: u32 = 256;

/// Output labels: index -> degrees.
const ORIENTATION_LABELS: [u32; 4] = [0, 90, 180, 270];

/// PP-LCNet doc_ori outputs ~45% confidence for correct class in a 4-class problem.
/// Uniform baseline is 25%. A threshold of 0.35 provides good discrimination.
pub const MIN_CONFIDENCE: f32 = 0.35;

/// Document orientation detection result.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct OrientationResult {
    /// Detected orientation in degrees (0, 90, 180, or 270).
    pub degrees: u32,
    /// Confidence score (0.0-1.0).
    pub confidence: f32,
}

/// Detects document page orientation using the PP-LCNet model.
///
/// Thread-safe: uses unsafe pointer cast for ONNX session (same pattern as embedding engine).
/// The model is downloaded from HuggingFace on first use and cached locally.
pub struct DocOrientationDetector {
    session: once_cell::sync::OnceCell<Session>,
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

        // Preprocess: resize short side to 256, center crop 224×224
        let preprocessed = preprocess(image);

        // Build input tensor: [1, 3, 224, 224]
        let input_tensor = normalize(&preprocessed);
        let tensor = Tensor::from_array(input_tensor).map_err(|e| KreuzbergError::Ocr {
            message: format!("Failed to create doc_ori input tensor: {e}"),
            source: None,
        })?;

        // SAFETY: ONNX Runtime C API is thread-safe for concurrent inference.
        // The ort crate's &mut self on Session::run is overly conservative.
        #[allow(unsafe_code)]
        let outputs = unsafe {
            let session_ptr = session as *const Session as *mut Session;
            (*session_ptr).run(ort::inputs!["x" => tensor])
        }
        .map_err(|e| KreuzbergError::Ocr {
            message: format!("Doc orientation inference failed: {e}"),
            source: None,
        })?;

        // Parse output: argmax over 4 orientation classes
        let (_, output_value) = outputs.iter().next().ok_or_else(|| KreuzbergError::Ocr {
            message: "No output from doc orientation model".to_string(),
            source: None,
        })?;

        let scores: Vec<f32> = output_value
            .try_extract_tensor::<f32>()
            .map_err(|e| KreuzbergError::Ocr {
                message: format!("Failed to extract doc_ori output: {e}"),
                source: None,
            })?
            .1
            .to_vec();

        // Softmax + argmax
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

    /// Ensure the model is downloaded and return the ONNX file path.
    fn ensure_model(&self) -> Result<PathBuf> {
        let model_dir = self.cache_dir.join("doc-orientation");
        let model_file = model_dir.join("model.onnx");

        if model_file.exists() {
            return Ok(model_file);
        }

        tracing::info!("Downloading document orientation model...");
        fs::create_dir_all(&model_dir)?;

        let cached_path =
            crate::model_download::hf_download(HF_REPO_ID, REMOTE_FILENAME).map_err(|e| KreuzbergError::Plugin {
                message: e,
                plugin_name: "auto-rotate".to_string(),
            })?;

        crate::model_download::verify_sha256(&cached_path, SHA256, "doc_ori").map_err(|e| {
            KreuzbergError::Validation {
                message: e,
                source: None,
            }
        })?;

        fs::copy(&cached_path, &model_file).map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to copy doc_ori model: {e}"),
            plugin_name: "auto-rotate".to_string(),
        })?;

        tracing::info!("Document orientation model saved");
        Ok(model_file)
    }

    /// Get or initialize the ONNX session (lazy, thread-safe via OnceCell).
    fn get_or_init_session(&self) -> Result<&Session> {
        self.session.get_or_try_init(|| {
            let model_path = self.ensure_model()?;

            crate::ort_discovery::ensure_ort_available();

            let num_threads = crate::core::config::concurrency::resolve_thread_budget(None);
            let builder = SessionBuilder::new()
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("Failed to create doc_ori session builder: {e}"),
                    source: None,
                })?
                .with_optimization_level(GraphOptimizationLevel::All)
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("Failed to set doc_ori optimization level: {e}"),
                    source: None,
                })?
                .with_intra_threads(num_threads)
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("Failed to set doc_ori thread count: {e}"),
                    source: None,
                })?
                .with_inter_threads(1)
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("Failed to set doc_ori inter threads: {e}"),
                    source: None,
                })?;
            let mut builder = crate::ort_discovery::apply_execution_providers(builder, self.acceleration.as_ref())
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("Failed to set doc_ori execution providers: {e}"),
                    source: None,
                })?;
            let session = builder.commit_from_file(&model_path).map_err(|e| KreuzbergError::Ocr {
                message: format!("Failed to load doc_ori model: {e}"),
                source: None,
            })?;

            tracing::info!("Doc orientation model loaded");
            Ok(session)
        })
    }
}

/// Resolve the cache directory for the auto-rotate model.
pub(crate) fn resolve_cache_dir() -> PathBuf {
    crate::cache_dir::resolve_cache_dir("auto-rotate")
}

/// Detect orientation and return a corrected image if rotation is needed.
///
/// Returns `Ok(Some(rotated_bytes))` if rotation was applied,
/// `Ok(None)` if no rotation needed (0° or low confidence).
pub(crate) fn detect_and_rotate(detector: &DocOrientationDetector, image_bytes: &[u8]) -> Result<Option<Vec<u8>>> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| KreuzbergError::Ocr {
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

    // Rotate the image back to upright (opposite direction of detected orientation).
    let rotated = match result.degrees {
        90 => image::imageops::rotate270(&img),
        180 => image::imageops::rotate180(&img),
        270 => image::imageops::rotate90(&img),
        _ => return Ok(None),
    };

    // Encode back to PNG bytes
    let mut buf = std::io::Cursor::new(Vec::new());
    rotated
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| KreuzbergError::Ocr {
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

    // Resize: scale so short side = RESIZE_SHORT
    let (new_w, new_h) = if w < h {
        let scale = RESIZE_SHORT as f32 / w as f32;
        (RESIZE_SHORT, (h as f32 * scale).round() as u32)
    } else {
        let scale = RESIZE_SHORT as f32 / h as f32;
        ((w as f32 * scale).round() as u32, RESIZE_SHORT)
    };

    let resized = image::imageops::resize(image, new_w, new_h, image::imageops::FilterType::Triangle);

    // Center crop to INPUT_SIZE × INPUT_SIZE
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

    // ImageNet mean/std for BGR order (swap R and B)
    const BGR_MEAN: [f32; 3] = [0.406 * 255.0, 0.456 * 255.0, 0.485 * 255.0];
    const BGR_NORM: [f32; 3] = [1.0 / (0.225 * 255.0), 1.0 / (0.224 * 255.0), 1.0 / (0.229 * 255.0)];

    for y in 0..h {
        for x in 0..w {
            let pixel = image.get_pixel(x as u32, y as u32);
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;
            // BGR order: channel 0 = Blue, channel 1 = Green, channel 2 = Red
            tensor[[0, 0, y, x]] = (b - BGR_MEAN[0]) * BGR_NORM[0];
            tensor[[0, 1, y, x]] = (g - BGR_MEAN[1]) * BGR_NORM[1];
            tensor[[0, 2, y, x]] = (r - BGR_MEAN[2]) * BGR_NORM[2];
        }
    }

    tensor
}
