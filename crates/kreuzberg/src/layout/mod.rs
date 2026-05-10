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

pub use engine::{CustomModelVariant, DetectTimings, LayoutEngine, LayoutEngineConfig, ModelBackend};
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

/// Convert a [`LayoutDetectionConfig`] into a [`LayoutEngineConfig`].
pub(crate) fn config_from_extraction(layout_config: &LayoutDetectionConfig) -> LayoutEngineConfig {
    LayoutEngineConfig {
        backend: ModelBackend::RtDetr,
        confidence_threshold: layout_config.confidence_threshold,
        apply_heuristics: layout_config.apply_heuristics,
        cache_dir: None,
        acceleration: layout_config.acceleration.clone(),
    }
}

/// Create a [`LayoutEngine`] from a [`LayoutDetectionConfig`].
///
/// Ensures ORT is available, then creates the engine with model download.
pub(crate) fn create_engine(layout_config: &LayoutDetectionConfig) -> Result<LayoutEngine, LayoutError> {
    crate::ort_discovery::ensure_ort_available();
    let config = config_from_extraction(layout_config);
    LayoutEngine::from_config(config)
}

/// Take the cached layout engine, or create a new one if the cache is empty.
///
/// The caller owns the engine for the duration of its work and should
/// return it via [`return_engine`] when done. This avoids holding the
/// global mutex during inference.
pub(crate) fn take_or_create_engine(layout_config: &LayoutDetectionConfig) -> Result<LayoutEngine, LayoutError> {
    CACHED_ENGINE.take_or_create(|| create_engine(layout_config))
}

/// Return a layout engine to the global cache for reuse by future extractions.
pub(crate) fn return_engine(engine: LayoutEngine) {
    CACHED_ENGINE.put(engine);
}

/// Take the cached TATR model, or create a new one if the cache is empty.
///
/// Returns `None` if the model cannot be loaded. Once a load attempt fails,
/// subsequent calls return `None` immediately without retrying, avoiding
/// repeated download attempts and redundant warning logs.
pub(crate) fn take_or_create_tatr(
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> Option<models::tatr::TatrModel> {
    // Fast path: if we already know TATR is unavailable, skip immediately.
    if let Some(&false) = TATR_TRIED.get() {
        return None;
    }

    let accel_cloned = accel.cloned();
    let result = CACHED_TATR.take_or_create(|| {
        crate::ort_discovery::ensure_ort_available();
        let manager = LayoutModelManager::new(None);
        let model_path = manager.ensure_tatr_model()?;
        models::tatr::TatrModel::from_file(&model_path.to_string_lossy(), accel_cloned.as_ref())
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
pub(crate) fn return_tatr(model: models::tatr::TatrModel) {
    CACHED_TATR.put(model);
}

// ---------------------------------------------------------------------------
// SLANeXT table model caching
// ---------------------------------------------------------------------------

/// Global cached SLANeXT wired model.
static CACHED_SLANET_WIRED: ModelCache<models::slanet::SlanetModel> = ModelCache::new();

/// Global cached SLANeXT wireless model.
static CACHED_SLANET_WIRELESS: ModelCache<models::slanet::SlanetModel> = ModelCache::new();

/// Global cached SLANet-plus model.
static CACHED_SLANET_PLUS: ModelCache<models::slanet::SlanetModel> = ModelCache::new();

/// Global cached table classifier model.
static CACHED_TABLE_CLASSIFIER: ModelCache<models::table_classifier::TableClassifier> = ModelCache::new();

/// Tracks whether SLANeXT loading has been attempted per variant.
static SLANET_WIRED_TRIED: OnceLock<bool> = OnceLock::new();
static SLANET_WIRELESS_TRIED: OnceLock<bool> = OnceLock::new();
static SLANET_PLUS_TRIED: OnceLock<bool> = OnceLock::new();
static TABLE_CLASSIFIER_TRIED: OnceLock<bool> = OnceLock::new();

/// Take a cached SLANeXT model for the given variant, or create a new one.
pub(crate) fn take_or_create_slanet(
    variant: &str,
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> Option<models::slanet::SlanetModel> {
    let (cache, tried) = match variant {
        "slanet_wired" => (&CACHED_SLANET_WIRED, &SLANET_WIRED_TRIED),
        "slanet_wireless" => (&CACHED_SLANET_WIRELESS, &SLANET_WIRELESS_TRIED),
        "slanet_plus" => (&CACHED_SLANET_PLUS, &SLANET_PLUS_TRIED),
        _ => return None,
    };

    if let Some(&false) = tried.get() {
        return None;
    }

    let accel_cloned = accel.cloned();
    let result = cache.take_or_create(|| {
        crate::ort_discovery::ensure_ort_available();
        let manager = LayoutModelManager::new(None);
        let model_path = manager.ensure_slanet_model(variant)?;
        models::slanet::SlanetModel::from_file(&model_path.to_string_lossy(), accel_cloned.as_ref())
    });

    match result {
        Ok(model) => {
            tried.get_or_init(|| true);
            Some(model)
        }
        Err(e) => {
            tried.get_or_init(|| {
                tracing::warn!(variant, "SLANeXT model unavailable: {e}");
                false
            });
            None
        }
    }
}

/// Returns `true` if the TATR table structure model has been successfully loaded.
pub(crate) fn is_tatr_available() -> bool {
    TATR_TRIED.get().copied().unwrap_or(false)
}

/// Returns `true` if any SLANeXT table model variant has been successfully loaded.
pub(crate) fn is_slanet_available() -> bool {
    SLANET_WIRED_TRIED.get().copied().unwrap_or(false)
        || SLANET_WIRELESS_TRIED.get().copied().unwrap_or(false)
        || SLANET_PLUS_TRIED.get().copied().unwrap_or(false)
}

/// Take a cached table classifier, or create a new one.
pub(crate) fn take_or_create_table_classifier(
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> Option<models::table_classifier::TableClassifier> {
    if let Some(&false) = TABLE_CLASSIFIER_TRIED.get() {
        return None;
    }

    let accel_cloned = accel.cloned();
    let result = CACHED_TABLE_CLASSIFIER.take_or_create(|| {
        crate::ort_discovery::ensure_ort_available();
        let manager = LayoutModelManager::new(None);
        let model_path = manager.ensure_table_classifier()?;
        models::table_classifier::TableClassifier::from_file(&model_path.to_string_lossy(), accel_cloned.as_ref())
    });

    match result {
        Ok(model) => {
            TABLE_CLASSIFIER_TRIED.get_or_init(|| true);
            Some(model)
        }
        Err(e) => {
            TABLE_CLASSIFIER_TRIED.get_or_init(|| {
                tracing::warn!("Table classifier unavailable: {e}");
                false
            });
            None
        }
    }
}
