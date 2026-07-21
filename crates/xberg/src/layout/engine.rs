//! High-level layout detection engine.
//!
//! Provides [`LayoutEngine`] as the main entry point for layout detection,
//! with [`LayoutEngineConfig`] for full programmatic control.

use std::path::PathBuf;
use std::time::Instant;

use image::RgbImage;

use crate::layout::error::LayoutError;
#[cfg(not(target_arch = "wasm32"))]
use crate::layout::model_manager::LayoutModelManager;
use crate::layout::models::LayoutModel;
#[cfg(feature = "layout-detection")]
use crate::layout::models::pp_doclayout_v3::PpDocLayoutV3Model;
use crate::layout::models::rtdetr::RtDetrModel;
#[cfg(feature = "layout-detection")]
use crate::layout::models::yolo::{YoloModel, YoloVariant};
use crate::layout::postprocessing::heuristics;
use crate::layout::types::DetectionResult;
/// Which underlying model architecture to use for layout detection.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq)]
pub enum ModelBackend {
    /// YOLO trained on DocLayNet (11 classes, 640x640 input).
    YoloDocLayNet,
    /// RT-DETR v2 (17 classes, 640x640 input, NMS-free).
    RtDetr,
    /// PP-DocLayout-V3 (25 classes, 800×800 input, PaddleDetection DETR).
    PpDocLayoutV3,
    /// Custom model from a local file path.
    Custom {
        /// Filesystem path to the ONNX model file.
        path: PathBuf,
        /// Model architecture variant for the custom file.
        variant: CustomModelVariant,
    },
}
/// Variant selection for custom model paths, used with [`ModelBackend::Custom`].
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq)]
pub enum CustomModelVariant {
    /// RT-DETR v2 model format.
    RtDetr,
    /// PP-DocLayout-V3 model format.
    PpDocLayoutV3,
    /// YOLO trained on DocLayNet (11 classes).
    YoloDocLayNet,
    /// DocLayout-YOLO trained on DocStructBench (10 classes).
    YoloDocStructBench,
    /// YOLOX with explicit input dimensions.
    Yolox {
        /// Model input width in pixels.
        input_width: u32,
        /// Model input height in pixels.
        input_height: u32,
    },
}
#[cfg_attr(alef, alef(skip))]
/// Full configuration for the layout engine.
///
/// Provides fine-grained control over model selection, thresholds, and
/// postprocessing.
#[derive(Debug, Clone, PartialEq)]
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
#[cfg_attr(alef, alef(skip))]
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
#[cfg_attr(alef, alef(skip))]
pub struct LayoutEngine {
    model: Box<dyn LayoutModel>,
    config: LayoutEngineConfig,
    #[cfg(feature = "layout-detection")]
    thread_budget: usize,
}

impl LayoutEngine {
    #[cfg(feature = "layout-detection")]
    pub(crate) fn matches_config(&self, config: &LayoutEngineConfig, thread_budget: usize) -> bool {
        self.config == *config && self.thread_budget == thread_budget.max(1)
    }

    /// Create a layout engine from a full config.
    ///
    /// `ModelBackend::RtDetr` and `CustomModelVariant::RtDetr` work on either engine
    /// (ORT-backed `layout-detection` or pure-Rust `layout-tract`). `PpDocLayoutV3` and
    /// every YOLO-based `CustomModelVariant` require the ORT-backed `layout-detection`
    /// feature; under `layout-tract` alone they return a
    /// [`LayoutError::ModelDownload`] explaining why, rather than failing to compile
    /// or panicking.
    ///
    /// Not available on `wasm32`: model resolution goes through
    /// [`LayoutModelManager`], which downloads weights from Hugging Face Hub over
    /// `hf-hub`/`reqwest` — both unavailable on that target. WASM callers construct
    /// a [`LayoutEngine`] from injected model bytes via [`Self::from_rtdetr_bytes`]
    /// instead.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_config(config: LayoutEngineConfig) -> Result<Self, LayoutError> {
        Self::from_config_with_thread_budget(config, crate::core::config::concurrency::resolve_thread_budget(None))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_config_with_thread_budget(
        config: LayoutEngineConfig,
        thread_budget: usize,
    ) -> Result<Self, LayoutError> {
        #[cfg(feature = "layout-detection")]
        crate::ort_discovery::ensure_ort_available();
        let thread_budget = thread_budget.max(1);

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
                Box::new(RtDetrModel::from_file(
                    &path_str,
                    config.acceleration.as_ref(),
                    thread_budget,
                )?)
            }
            #[cfg(feature = "layout-detection")]
            ModelBackend::PpDocLayoutV3 => {
                let manager = LayoutModelManager::new(config.cache_dir.clone());
                let model_path = manager.ensure_pp_doclayout_v3_model()?;
                let path_str = model_path.to_string_lossy();
                Box::new(PpDocLayoutV3Model::from_file_with_thread_budget(
                    &path_str,
                    config.acceleration.as_ref(),
                    thread_budget,
                )?)
            }
            #[cfg(not(feature = "layout-detection"))]
            ModelBackend::PpDocLayoutV3 => {
                return Err(LayoutError::ModelDownload(
                    "PP-DocLayout-V3 requires the ORT-backed `layout-detection` feature \
                     (unsupported under the pure-Rust `layout-tract` engine — see \
                     docs-site/src/content/docs/concepts/tract-inference.md)"
                        .into(),
                ));
            }
            ModelBackend::Custom { path, variant } => {
                let path_str = path.to_string_lossy();
                let accel = config.acceleration.as_ref();
                match variant {
                    CustomModelVariant::RtDetr => Box::new(RtDetrModel::from_file(&path_str, accel, thread_budget)?),
                    #[cfg(feature = "layout-detection")]
                    CustomModelVariant::PpDocLayoutV3 => Box::new(PpDocLayoutV3Model::from_file_with_thread_budget(
                        &path_str,
                        accel,
                        thread_budget,
                    )?),
                    #[cfg(feature = "layout-detection")]
                    CustomModelVariant::YoloDocLayNet => Box::new(YoloModel::from_file(
                        &path_str,
                        YoloVariant::DocLayNet,
                        640,
                        640,
                        "Custom-YOLO-DocLayNet",
                        accel,
                        thread_budget,
                    )?),
                    #[cfg(feature = "layout-detection")]
                    CustomModelVariant::YoloDocStructBench => Box::new(YoloModel::from_file(
                        &path_str,
                        YoloVariant::DocStructBench,
                        1024,
                        1024,
                        "Custom-DocLayout-YOLO",
                        accel,
                        thread_budget,
                    )?),
                    #[cfg(feature = "layout-detection")]
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
                        thread_budget,
                    )?),
                    #[cfg(not(feature = "layout-detection"))]
                    CustomModelVariant::PpDocLayoutV3
                    | CustomModelVariant::YoloDocLayNet
                    | CustomModelVariant::YoloDocStructBench
                    | CustomModelVariant::Yolox { .. } => {
                        return Err(LayoutError::ModelDownload(
                            "this custom model variant requires the ORT-backed \
                             `layout-detection` feature (unsupported under the pure-Rust \
                             `layout-tract` engine)"
                                .into(),
                        ));
                    }
                }
            }
        };

        Ok(Self {
            model,
            config,
            #[cfg(feature = "layout-detection")]
            thread_budget,
        })
    }

    /// Create a layout engine directly from RT-DETR model bytes already resolved by the caller.
    ///
    /// Bypasses [`LayoutModelManager`] entirely — there is no filesystem path or HTTP
    /// download involved. This is the WASM entry point: the JS host fetches the ONNX
    /// weights (never embedded in the `.wasm` binary) and hands over the bytes, which
    /// flow straight through to the [`crate::inference`] seam's `load_from_memory`.
    /// Only the RT-DETR detection backend is supported this way; `PpDocLayoutV3` and
    /// the YOLO variants require the ORT-backed `layout-detection` feature, which is
    /// not available on `wasm32`.
    pub fn from_rtdetr_bytes(
        rtdetr_bytes: &[u8],
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<Self, LayoutError> {
        let model: Box<dyn LayoutModel> = Box::new(RtDetrModel::from_bytes(rtdetr_bytes, accel)?);
        Ok(Self {
            model,
            config: LayoutEngineConfig {
                backend: ModelBackend::RtDetr,
                acceleration: accel.cloned(),
                ..LayoutEngineConfig::default()
            },
            #[cfg(feature = "layout-detection")]
            thread_budget: crate::core::config::concurrency::resolve_thread_budget(None),
        })
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

    /// Decode `image_bytes` and run layout detection.
    ///
    /// A convenience wrapper over [`Self::detect`] for callers that hold encoded
    /// image bytes (PNG/JPEG/…) rather than a decoded [`RgbImage`] — notably the
    /// WASM bridge, which receives image bytes from JS.
    pub fn detect_image_bytes(&mut self, image_bytes: &[u8]) -> Result<DetectionResult, LayoutError> {
        let img = image::load_from_memory(image_bytes)?.to_rgb8();
        self.detect(&img)
    }

    /// Run layout detection on an image and return granular timing data.
    ///
    /// Identical to [`detect`] but also returns a [`DetectTimings`] breakdown.
    /// Use this when you need per-step profiling (preprocess / onnx / postprocess).
    pub(crate) fn detect_timed(&mut self, img: &RgbImage) -> Result<(DetectionResult, DetectTimings), LayoutError> {
        let model_start = Instant::now();
        let mut detections = if let Some(threshold) = self.config.confidence_threshold {
            self.model.detect_with_threshold(img, threshold)?
        } else {
            self.model.detect(img)?
        };
        let model_total_ms = model_start.elapsed().as_secs_f64() * 1000.0;

        let (preprocess_ms, onnx_ms) = crate::layout::inference_timings::take();

        let page_width = img.width();
        let page_height = img.height();

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
                    postprocess_ms: 0.0,
                },
            ));
        }

        let postprocess_ms = postprocess_start.elapsed().as_secs_f64() * 1000.0;
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

#[cfg(test)]
mod cache_key_tests {
    use super::*;
    use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};

    #[test]
    fn engine_config_equality_covers_every_session_and_output_setting() {
        let base = LayoutEngineConfig::default();

        let mut backend = base.clone();
        backend.backend = ModelBackend::YoloDocLayNet;
        assert_ne!(base, backend);

        let mut threshold = base.clone();
        threshold.confidence_threshold = Some(0.75);
        assert_ne!(base, threshold);

        let mut heuristics = base.clone();
        heuristics.apply_heuristics = false;
        assert_ne!(base, heuristics);

        let mut cache_dir = base.clone();
        cache_dir.cache_dir = Some(PathBuf::from("different-cache"));
        assert_ne!(base, cache_dir);

        let mut acceleration = base.clone();
        acceleration.acceleration = Some(AccelerationConfig {
            provider: ExecutionProviderType::Cpu,
            device_id: 1,
        });
        assert_ne!(base, acceleration);
    }
}
