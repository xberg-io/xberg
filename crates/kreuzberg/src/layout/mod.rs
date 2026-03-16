//! Layout detection via ONNX Runtime (YOLO + RT-DETR).
//!
//! This module provides ONNX-based document layout detection, integrated into
//! the kreuzberg extraction pipeline. Models are auto-downloaded from HuggingFace
//! on first use.
//!
//! The ONNX session is cached globally so that repeated extractions (e.g. batch
//! processing) pay model-load cost only once.

pub mod engine;
pub mod error;
pub(crate) mod inference_timings;
mod model_manager;
pub mod models;
pub mod postprocessing;
pub mod preprocessing;
pub mod session;
pub mod types;

pub use engine::{CustomModelVariant, DetectTimings, LayoutEngine, LayoutEngineConfig, LayoutPreset, ModelBackend};
pub use error::LayoutError;
pub use model_manager::LayoutModelManager;
pub use models::LayoutModel;
pub use models::rtdetr::RtDetrModel;
pub use models::yolo::{YoloModel, YoloVariant};
pub use types::{BBox, DetectionResult, LayoutClass, LayoutDetection};

use std::sync::OnceLock;

use crate::core::config::layout::LayoutDetectionConfig;
use crate::model_cache::ModelCache;

/// Global cached layout engine.
static CACHED_ENGINE: ModelCache<LayoutEngine> = ModelCache::new();

/// Global cached TATR table structure recognition model.
static CACHED_TATR: ModelCache<models::tatr::TatrModel> = ModelCache::new();

/// Tracks whether TATR loading has been attempted.
///
/// `true` means loading succeeded at least once; `false` means it failed and
/// we should not retry (avoids repeated model-download attempts and redundant
/// warning logs on every document).
static TATR_TRIED: OnceLock<bool> = OnceLock::new();

/// Convert an [`LayoutDetectionConfig`] into a [`LayoutEngineConfig`].
pub fn config_from_extraction(layout_config: &LayoutDetectionConfig) -> LayoutEngineConfig {
    let preset: LayoutPreset = layout_config.preset.parse().unwrap_or_else(|_| {
        tracing::warn!(
            preset = %layout_config.preset,
            "unrecognized layout preset, falling back to 'accurate'"
        );
        LayoutPreset::Accurate
    });

    let mut engine_config = LayoutEngineConfig::from_preset(preset);
    engine_config.confidence_threshold = layout_config.confidence_threshold;
    engine_config.apply_heuristics = layout_config.apply_heuristics;
    engine_config
}

/// Create a [`LayoutEngine`] from a [`LayoutDetectionConfig`].
///
/// Ensures ORT is available, then creates the engine with model download.
pub fn create_engine(layout_config: &LayoutDetectionConfig) -> Result<LayoutEngine, LayoutError> {
    crate::ort_discovery::ensure_ort_available();
    let config = config_from_extraction(layout_config);
    LayoutEngine::from_config(config)
}

/// Take the cached layout engine, or create a new one if the cache is empty.
///
/// The caller owns the engine for the duration of its work and should
/// return it via [`return_engine`] when done. This avoids holding the
/// global mutex during inference.
pub fn take_or_create_engine(layout_config: &LayoutDetectionConfig) -> Result<LayoutEngine, LayoutError> {
    CACHED_ENGINE.take_or_create(|| create_engine(layout_config))
}

/// Return a layout engine to the global cache for reuse by future extractions.
pub fn return_engine(engine: LayoutEngine) {
    CACHED_ENGINE.put(engine);
}

/// Take the cached TATR model, or create a new one if the cache is empty.
///
/// Returns `None` if the model cannot be loaded. Once a load attempt fails,
/// subsequent calls return `None` immediately without retrying, avoiding
/// repeated download attempts and redundant warning logs.
pub fn take_or_create_tatr() -> Option<models::tatr::TatrModel> {
    // Fast path: if we already know TATR is unavailable, skip immediately.
    if let Some(&false) = TATR_TRIED.get() {
        return None;
    }

    let result = CACHED_TATR.take_or_create(|| {
        crate::ort_discovery::ensure_ort_available();
        let manager = LayoutModelManager::new(None);
        let model_path = manager.ensure_tatr_model()?;
        models::tatr::TatrModel::from_file(&model_path.to_string_lossy())
    });

    match result {
        Ok(model) => {
            // Mark as available (no-op if already set to true).
            TATR_TRIED.get_or_init(|| true);
            Some(model)
        }
        Err(e) => {
            // Only log and set the flag on the first failure.
            TATR_TRIED.get_or_init(|| {
                tracing::warn!("TATR table structure model unavailable, table structure recognition disabled: {e}");
                false
            });
            None
        }
    }
}

/// Return a TATR model to the global cache for reuse.
pub fn return_tatr(model: models::tatr::TatrModel) {
    CACHED_TATR.put(model);
}
