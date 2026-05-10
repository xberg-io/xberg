//! High-level layout detection engine.
//!
//! Provides [`LayoutEngine`] as the main entry point for layout detection,
//! with [`LayoutEngineConfig`] for full programmatic control.

use std::path::PathBuf;
use std::time::Instant;

use image::RgbImage;

use crate::layout::error::LayoutError;
use crate::layout::model_manager::LayoutModelManager;
use crate::layout::models::LayoutModel;
use crate::layout::models::rtdetr::RtDetrModel;
use crate::layout::models::yolo::{YoloModel, YoloVariant};
use crate::layout::postprocessing::heuristics;
use crate::layout::types::DetectionResult;

/// Which underlying model architecture to use.
#[derive(Debug, Clone)]
pub enum ModelBackend {
    /// YOLO trained on DocLayNet (11 classes, 640x640 input).
    YoloDocLayNet,
    /// RT-DETR v2 (17 classes, 640x640 input, NMS-free).
    RtDetr,
    /// Custom model from a local file path.
    Custom { path: PathBuf, variant: CustomModelVariant },
}

/// Variant selection for custom model paths.
#[derive(Debug, Clone)]
pub enum CustomModelVariant {
    RtDetr,
    YoloDocLayNet,
    YoloDocStructBench,
    Yolox { input_width: u32, input_height: u32 },
}

/// Full configuration for the layout engine.
///
/// Provides fine-grained control over model selection, thresholds, and
/// postprocessing.
#[derive(Debug, Clone)]
pub struct LayoutEngineConfig {
    /// Which model backend to use.
    pub backend: ModelBackend,
    /// Confidence threshold override (None = use model default).
    pub confidence_threshold: Option<f32>,
    /// Whether to apply postprocessing heuristics.
    pub apply_heuristics: bool,
    /// Custom cache directory for model files (None = default).
    pub cache_dir: Option<PathBuf>,
    /// Hardware acceleration for ONNX inference.
    pub acceleration: Option<crate::core::config::acceleration::AccelerationConfig>,
}

impl Default for LayoutEngineConfig {
    fn default() -> Self {
        Self {
            backend: ModelBackend::RtDetr,
            confidence_threshold: None,
            apply_heuristics: true,
            cache_dir: None,
            acceleration: None,
        }
    }
}

/// Granular timing breakdown for a single `detect()` call.
#[derive(Debug, Clone, Default)]
pub struct DetectTimings {
    /// Time spent in image preprocessing (resize, letterbox, normalize, tensor allocation).
    pub preprocess_ms: f64,
    /// Time for the ONNX `session.run()` call (actual neural network computation).
    pub onnx_ms: f64,
    /// Total time from start of model call to end of raw output decoding.
    pub model_total_ms: f64,
    /// Time spent in postprocessing heuristics (confidence filtering, overlap resolution).
    pub postprocess_ms: f64,
}

/// High-level layout detection engine.
///
/// Wraps model loading, inference, and postprocessing into a single
/// reusable object. Models are downloaded and cached on first use.
pub struct LayoutEngine {
    model: Box<dyn LayoutModel>,
    config: LayoutEngineConfig,
}

impl LayoutEngine {
    /// Create a layout engine from a full config.
    pub fn from_config(config: LayoutEngineConfig) -> Result<Self, LayoutError> {
        crate::ort_discovery::ensure_ort_available();

        let model: Box<dyn LayoutModel> = match &config.backend {
            ModelBackend::YoloDocLayNet => {
                return Err(LayoutError::ModelDownload(
                    "YOLO DocLayNet model is not available for automatic download. \
                     Use ModelBackend::Custom with a local YOLO ONNX file instead."
                        .into(),
                ));
            }
            ModelBackend::RtDetr => {
                let manager = LayoutModelManager::new(config.cache_dir.clone());
                let model_path = manager.ensure_rtdetr_model()?;
                let path_str = model_path.to_string_lossy();
                Box::new(RtDetrModel::from_file(&path_str, config.acceleration.as_ref())?)
            }
            ModelBackend::Custom { path, variant } => {
                let path_str = path.to_string_lossy();
                let accel = config.acceleration.as_ref();
                match variant {
                    CustomModelVariant::RtDetr => Box::new(RtDetrModel::from_file(&path_str, accel)?),
                    CustomModelVariant::YoloDocLayNet => Box::new(YoloModel::from_file(
                        &path_str,
                        YoloVariant::DocLayNet,
                        640,
                        640,
                        "Custom-YOLO-DocLayNet",
                        accel,
                    )?),
                    CustomModelVariant::YoloDocStructBench => Box::new(YoloModel::from_file(
                        &path_str,
                        YoloVariant::DocStructBench,
                        1024,
                        1024,
                        "Custom-DocLayout-YOLO",
                        accel,
                    )?),
                    CustomModelVariant::Yolox {
                        input_width,
                        input_height,
                    } => Box::new(YoloModel::from_file(
                        &path_str,
                        YoloVariant::Yolox,
                        *input_width,
                        *input_height,
                        "Custom-YOLOX",
                        accel,
                    )?),
                }
            }
        };

        Ok(Self { model, config })
    }

    /// Run layout detection on an image.
    ///
    /// Returns a [`DetectionResult`] with bounding boxes, classes, and confidence scores.
    /// If `apply_heuristics` is enabled in config, postprocessing is applied automatically.
    pub fn detect(&mut self, img: &RgbImage) -> Result<DetectionResult, LayoutError> {
        let (result, _timings) = self.detect_timed(img)?;
        for detection in &result.detections {
            tracing::trace!(class = ?detection.class_name, confidence = detection.confidence, "Layout detection result");
        }
        Ok(result)
    }

    /// Run layout detection on an image and return granular timing data.
    ///
    /// Identical to [`detect`] but also returns a [`DetectTimings`] breakdown.
    /// Use this when you need per-step profiling (preprocess / onnx / postprocess).
    pub(crate) fn detect_timed(&mut self, img: &RgbImage) -> Result<(DetectionResult, DetectTimings), LayoutError> {
        // Model inference (includes preprocessing + ONNX run internally).
        let model_start = Instant::now();
        let mut detections = if let Some(threshold) = self.config.confidence_threshold {
            self.model.detect_with_threshold(img, threshold)?
        } else {
            self.model.detect(img)?
        };
        let model_total_ms = model_start.elapsed().as_secs_f64() * 1000.0;

        // Retrieve granular preprocess/onnx split recorded by the model implementation
        // via the thread-local side-channel.
        let (preprocess_ms, onnx_ms) = crate::layout::inference_timings::take();

        let page_width = img.width();
        let page_height = img.height();

        // Postprocessing heuristics (confidence filtering, overlap resolution).
        let postprocess_start = Instant::now();
        if self.config.apply_heuristics {
            detections = heuristics::apply_heuristics(detections, page_width as f32, page_height as f32);
        }
        let postprocess_ms = postprocess_start.elapsed().as_secs_f64() * 1000.0;

        tracing::info!(
            preprocess_ms,
            onnx_ms,
            model_total_ms,
            postprocess_ms,
            final_detections = detections.len(),
            "Layout engine detect_timed() breakdown"
        );

        let timings = DetectTimings {
            preprocess_ms,
            onnx_ms,
            model_total_ms,
            postprocess_ms,
        };

        Ok((DetectionResult::new(page_width, page_height, detections), timings))
    }

    /// Run layout detection on a batch of images in a single model call.
    ///
    /// Returns one `(DetectionResult, DetectTimings)` tuple per input image.
    /// Postprocessing heuristics are applied per image when enabled in config.
    ///
    /// Timing note: `preprocess_ms` and `onnx_ms` in each `DetectTimings` are the
    /// amortized per-image share of the batch operation (total / N), not independent
    /// per-image measurements.
    // consumed via vendored kreuzberg in packages/ruby/, packages/r/
    #[allow(dead_code)]
    pub(crate) fn detect_batch(
        &mut self,
        images: &[&RgbImage],
    ) -> Result<Vec<(DetectionResult, DetectTimings)>, LayoutError> {
        if images.is_empty() {
            return Ok(Vec::new());
        }

        let model_start = Instant::now();
        let per_image_detections = self.model.detect_batch(images, self.config.confidence_threshold)?;
        let model_total_ms = model_start.elapsed().as_secs_f64() * 1000.0;

        // Retrieve amortized timings written by the batch implementation.
        let (preprocess_ms, onnx_ms) = crate::layout::inference_timings::take();

        let postprocess_start = Instant::now();
        let mut results = Vec::with_capacity(images.len());

        for (img, mut detections) in images.iter().zip(per_image_detections) {
            let page_width = img.width();
            let page_height = img.height();

            if self.config.apply_heuristics {
                detections = heuristics::apply_heuristics(detections, page_width as f32, page_height as f32);
            }

            results.push((
                DetectionResult::new(page_width, page_height, detections),
                DetectTimings {
                    preprocess_ms,
                    onnx_ms,
                    model_total_ms,
                    postprocess_ms: 0.0, // filled in after the loop
                },
            ));
        }

        let postprocess_ms = postprocess_start.elapsed().as_secs_f64() * 1000.0;
        // Distribute postprocess time across all results (amortized per image).
        let postprocess_ms_per = postprocess_ms / images.len() as f64;
        for (_, timings) in &mut results {
            timings.postprocess_ms = postprocess_ms_per;
        }

        tracing::info!(
            preprocess_ms,
            onnx_ms,
            model_total_ms,
            postprocess_ms,
            batch_size = images.len(),
            total_detections = results.iter().map(|(r, _)| r.detections.len()).sum::<usize>(),
            "Layout engine detect_batch() breakdown"
        );

        Ok(results)
    }
}
