//! Layout detection types and (when `layout-detection` feature is enabled) ONNX Runtime
//! inference via YOLO + RT-DETR.
//!
//! The `types` submodule is always available under the `layout-types` feature (pure-Rust,
//! no ORT dependency). The inference submodules (`engine`, `models`, `session`, etc.)
//! require the `layout-detection` feature and pull in ONNX Runtime.
//!
//! The ONNX session is cached globally so that repeated extractions (e.g. batch
//! processing) pay model-load cost only once.

/// Layout detection result types (pure-Rust, available under `layout-types`).
pub mod types;

#[cfg(feature = "layout-detection")]
/// High-level layout detection engine wrapping model loading and inference.
pub mod engine;
#[cfg(feature = "layout-detection")]
/// Error types for layout detection failures.
pub mod error;
#[cfg(feature = "layout-detection")]
pub(crate) mod inference_timings;
#[cfg(feature = "layout-detection")]
mod model_manager;
#[cfg(feature = "layout-detection")]
/// ONNX model implementations for layout detection (RT-DETR, YOLO, TATR, SLANeXT).
pub mod models;
#[cfg(feature = "layout-detection")]
/// Postprocessing heuristics and NMS for raw model detections.
pub mod postprocessing;
#[cfg(feature = "layout-detection")]
/// Image preprocessing (resize, letterbox, normalization) for layout model input.
pub mod preprocessing;
#[cfg(feature = "layout-detection")]
/// ONNX Runtime session creation and configuration helpers.
pub mod session;

pub use types::{BBox, DetectionResult, LayoutClass, LayoutDetection};

#[cfg(feature = "layout-detection")]
pub use engine::{CustomModelVariant, DetectTimings, LayoutEngine, LayoutEngineConfig, ModelBackend};
#[cfg(feature = "layout-detection")]
pub use error::LayoutError;
#[cfg(feature = "layout-detection")]
pub use model_manager::LayoutModelManager;
#[cfg(feature = "layout-detection")]
pub use models::LayoutModel;
#[cfg(feature = "layout-detection")]
pub use models::rtdetr::RtDetrModel;
#[cfg(feature = "layout-detection")]
pub use models::yolo::{YoloModel, YoloVariant};

#[cfg(all(feature = "layout-detection", feature = "pdf"))]
use std::sync::OnceLock;

#[cfg(all(feature = "layout-detection", any(feature = "pdf", feature = "ocr", feature = "ocr-wasm")))]
use crate::core::config::layout::LayoutDetectionConfig;
#[cfg(all(feature = "layout-detection", any(feature = "pdf", feature = "ocr", feature = "ocr-wasm")))]
use crate::model_cache::ModelCache;

// ---------------------------------------------------------------------------
// Engine + model caching — all gated on `layout-detection` (requires ORT)
// ---------------------------------------------------------------------------

/// Global cached layout engine.
///
/// Used by the image extractor (layout-detection + ocr/ocr-wasm) and the
/// PDF extractor (layout-detection + pdf).
#[cfg(all(feature = "layout-detection", any(feature = "pdf", feature = "ocr", feature = "ocr-wasm")))]
static CACHED_ENGINE: ModelCache<LayoutEngine> = ModelCache::new();

/// Global cached TATR table structure recognition model.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static CACHED_TATR: ModelCache<models::tatr::TatrModel> = ModelCache::new();

/// Tracks whether TATR loading has been attempted.
///
/// `true` means loading succeeded at least once; `false` means it failed and
/// we should not retry (avoids repeated model-download attempts and redundant
/// warning logs on every document).
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static TATR_TRIED: OnceLock<bool> = OnceLock::new();

/// Convert a [`LayoutDetectionConfig`] into a [`LayoutEngineConfig`].
#[cfg(all(feature = "layout-detection", any(feature = "pdf", feature = "ocr", feature = "ocr-wasm")))]
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
#[cfg(all(feature = "layout-detection", any(feature = "pdf", feature = "ocr", feature = "ocr-wasm")))]
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
#[cfg(all(feature = "layout-detection", any(feature = "pdf", feature = "ocr", feature = "ocr-wasm")))]
pub(crate) fn take_or_create_engine(layout_config: &LayoutDetectionConfig) -> Result<LayoutEngine, LayoutError> {
    CACHED_ENGINE.take_or_create(|| create_engine(layout_config))
}

/// Return a layout engine to the global cache for reuse by future extractions.
#[cfg(all(feature = "layout-detection", any(feature = "pdf", feature = "ocr", feature = "ocr-wasm")))]
pub(crate) fn return_engine(engine: LayoutEngine) {
    CACHED_ENGINE.put(engine);
}

/// Take the cached TATR model, or create a new one if the cache is empty.
///
/// Returns `None` if the model cannot be loaded. Once a load attempt fails,
/// subsequent calls return `None` immediately without retrying, avoiding
/// repeated download attempts and redundant warning logs.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
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
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
pub(crate) fn return_tatr(model: models::tatr::TatrModel) {
    CACHED_TATR.put(model);
}

// ---------------------------------------------------------------------------
// SLANeXT table model caching
// ---------------------------------------------------------------------------

/// Global cached SLANeXT wired model.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static CACHED_SLANET_WIRED: ModelCache<models::slanet::SlanetModel> = ModelCache::new();

/// Global cached SLANeXT wireless model.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static CACHED_SLANET_WIRELESS: ModelCache<models::slanet::SlanetModel> = ModelCache::new();

/// Global cached SLANet-plus model.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static CACHED_SLANET_PLUS: ModelCache<models::slanet::SlanetModel> = ModelCache::new();

/// Global cached table classifier model.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static CACHED_TABLE_CLASSIFIER: ModelCache<models::table_classifier::TableClassifier> = ModelCache::new();

/// Tracks whether SLANeXT loading has been attempted per variant.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static SLANET_WIRED_TRIED: OnceLock<bool> = OnceLock::new();
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static SLANET_WIRELESS_TRIED: OnceLock<bool> = OnceLock::new();
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static SLANET_PLUS_TRIED: OnceLock<bool> = OnceLock::new();
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
static TABLE_CLASSIFIER_TRIED: OnceLock<bool> = OnceLock::new();

/// Take a cached SLANeXT model for the given variant, or create a new one.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
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

/// Returns `true` if the TATR table structure model is loadable.
///
/// On first call, attempts to load TATR (using default acceleration) to populate
/// `TATR_TRIED`. Subsequent calls return the cached result. This makes the check
/// safe to use as a fail-fast guard before code paths that would otherwise be
/// the first to attempt the load — without this, the check would always return
/// `false` until some other call site tried to load.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
pub(crate) fn is_tatr_available() -> bool {
    if let Some(&result) = TATR_TRIED.get() {
        return result;
    }
    if let Some(model) = take_or_create_tatr(None) {
        return_tatr(model);
        true
    } else {
        false
    }
}

/// Returns `true` if any SLANeXT table model variant is loadable.
///
/// On first call (when no SLANeXT variant has been tried yet), attempts to
/// load `slanet_wired` (the default variant) to populate the tried flag. Same
/// rationale as [`is_tatr_available`]: without this, the check would always
/// return `false` until some other call site tried to load.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
pub(crate) fn is_slanet_available() -> bool {
    if SLANET_WIRED_TRIED.get().is_none() && SLANET_WIRELESS_TRIED.get().is_none() && SLANET_PLUS_TRIED.get().is_none()
    {
        // Side effect: take_or_create_slanet populates SLANET_WIRED_TRIED.
        // We drop the model — the caller will reload from cache when needed.
        drop(take_or_create_slanet("slanet_wired", None));
    }
    SLANET_WIRED_TRIED.get().copied().unwrap_or(false)
        || SLANET_WIRELESS_TRIED.get().copied().unwrap_or(false)
        || SLANET_PLUS_TRIED.get().copied().unwrap_or(false)
}

/// Take a cached table classifier, or create a new one.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
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
