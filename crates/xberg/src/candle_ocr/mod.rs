//! Candle-based VLM OCR backends.
//!
//! Pure-Rust transformer OCR via the `xberg-candle-ocr` crate. This module
//! holds the `OcrBackend + Plugin` impls and the per-model configuration
//! plumbing; model code itself lives in `xberg-candle-ocr::models`.

#[cfg(any(
    feature = "candle-trocr",
    feature = "candle-paddleocr-vl",
    all(feature = "candle-deepseek-ocr", not(target_arch = "wasm32"))
))]
async fn read_image_file(
    path: &std::path::Path,
    limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt as _;

    let file = tokio::fs::File::open(path).await?;
    let limit = u64::try_from(limits.max_content_size).unwrap_or(u64::MAX);
    let size_error = || {
        crate::XbergError::validation(format!(
            "The encoded image file exceeds security_limits.max_content_size ({limit} bytes)"
        ))
    };
    if file.metadata().await?.len() > limit {
        return Err(size_error());
    }
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes).await?;
    if bytes.len() > limits.max_content_size {
        return Err(size_error());
    }
    Ok(bytes)
}

mod config;

// Shared `ExtractedDocument` construction (metadata, tables, detected_languages)
// used by every candle-* backend below, so the same logic isn't pasted per
// backend (issue #179).
//
// Gated on the union of the concrete backends that consume it, NOT on the umbrella
// `candle-ocr` feature. The umbrella can be enabled on its own -- `xberg-cli`'s
// `candle-ocr` feature profile does exactly that -- which compiles this module with
// no backend to call into it, and every item here becomes dead code under
// `-D warnings`. Same union the device-preference helpers below use. ~keep
#[cfg(any(
    feature = "candle-trocr",
    feature = "candle-paddleocr-vl",
    all(
        not(target_arch = "wasm32"),
        any(feature = "candle-glm-ocr", feature = "candle-deepseek-ocr")
    )
))]
mod ocr_result;

#[cfg(feature = "candle-trocr")]
pub mod trocr_backend;

#[cfg(feature = "candle-paddleocr-vl")]
pub mod paddleocr_vl_backend;

#[cfg(all(feature = "candle-glm-ocr", not(target_arch = "wasm32")))]
pub mod glm_ocr_backend;

#[cfg(any(
    feature = "candle-paddleocr-vl",
    all(feature = "candle-deepseek-ocr", not(target_arch = "wasm32"))
))]
pub(crate) mod model_stager;

#[cfg(all(feature = "candle-deepseek-ocr", not(target_arch = "wasm32")))]
pub mod deepseek_ocr_backend;

pub use config::{
    CandleDeepseekOcrDtype, CandleDevicePreference, CandleModelId, CandleOcrConfig, CandleTrocrVariant,
    DeepseekOcrBackendOptions, GlmOcrBackendOptions, GlmOcrLayoutMode, GlmOcrTaskKind, PaddleOcrVlBackendOptions,
    PaddleOcrVlTaskKind, TrocrBackendOptions,
};

#[cfg(feature = "candle-trocr")]
pub use trocr_backend::TrocrBackend;

#[cfg(feature = "candle-paddleocr-vl")]
pub use paddleocr_vl_backend::PaddleOcrVlBackend;

#[cfg(all(feature = "candle-glm-ocr", not(target_arch = "wasm32")))]
pub use glm_ocr_backend::GlmOcrBackend;

#[cfg(all(feature = "candle-deepseek-ocr", not(target_arch = "wasm32")))]
pub use deepseek_ocr_backend::DeepseekOcrBackend;

#[cfg(any(
    feature = "candle-trocr",
    feature = "candle-paddleocr-vl",
    all(
        not(target_arch = "wasm32"),
        any(feature = "candle-glm-ocr", feature = "candle-deepseek-ocr")
    )
))]
use crate::core::config::{AccelerationConfig, ExecutionProviderType, OcrConfig};
#[cfg(any(
    feature = "candle-trocr",
    feature = "candle-paddleocr-vl",
    all(
        not(target_arch = "wasm32"),
        any(feature = "candle-glm-ocr", feature = "candle-deepseek-ocr")
    )
))]
use xberg_candle_ocr::DevicePreference;

/// Check `backend_options` for the candle backend named `backend`.
///
/// Uses the same check the backend runs on each page. A backend that is not a
/// compiled candle backend has nothing to check here. ~keep
#[cfg(any(
    feature = "candle-trocr",
    feature = "candle-paddleocr-vl",
    all(
        not(target_arch = "wasm32"),
        any(feature = "candle-glm-ocr", feature = "candle-deepseek-ocr")
    )
))]
pub(crate) fn validate_backend_options(backend: &str, value: Option<&serde_json::Value>) -> crate::Result<()> {
    match crate::plugins::registry::canonical_ocr_backend_name(backend).as_str() {
        #[cfg(feature = "candle-trocr")]
        "candle-trocr" => trocr_backend::check_backend_options(value).map(drop),
        #[cfg(feature = "candle-paddleocr-vl")]
        "candle-paddleocr-vl" => paddleocr_vl_backend::check_backend_options(value).map(drop),
        #[cfg(all(feature = "candle-glm-ocr", not(target_arch = "wasm32")))]
        "candle-glm-ocr" => glm_ocr_backend::check_backend_options(value).map(drop),
        #[cfg(all(feature = "candle-deepseek-ocr", not(target_arch = "wasm32")))]
        "candle-deepseek-ocr" => deepseek_ocr_backend::check_backend_options(value).map(drop),
        _ => Ok(()),
    }
}

/// Resolve a candle [`DevicePreference`] from the centralised acceleration
/// config plus a validated backend-specific device override.
///
/// Precedence (highest first):
/// 1. The typed backend option's device (when present) — an explicit per-call override.
/// 2. `OcrConfig.acceleration.provider` — the central config that already
///    drives layout-detection and embeddings.
/// 3. `DevicePreference::Auto`.
///
/// The mapping from [`ExecutionProviderType`] (ORT-flavoured) to
/// [`DevicePreference`] (candle-flavoured) is:
/// - `Auto`     -> `DevicePreference::Auto`
/// - `Cpu`      -> `DevicePreference::Cpu`
/// - `Cuda`     -> `DevicePreference::Cuda`
/// - `CoreMl`   -> `DevicePreference::Metal` (Apple Neural Engine + GPU runs on Metal in candle)
/// - `TensorRt` -> `DevicePreference::Cuda` (TensorRT runs on CUDA hardware; candle has no separate TRT path)
#[cfg(any(
    feature = "candle-trocr",
    feature = "candle-paddleocr-vl",
    all(
        not(target_arch = "wasm32"),
        any(feature = "candle-glm-ocr", feature = "candle-deepseek-ocr")
    )
))]
pub(crate) fn resolve_device_preference(
    config: &OcrConfig,
    explicit: Option<CandleDevicePreference>,
) -> DevicePreference {
    if let Some(explicit) = explicit {
        return explicit.into();
    }

    if let Some(accel) = &config.acceleration {
        return device_preference_from_acceleration(accel);
    }

    DevicePreference::Auto
}

/// Map an [`AccelerationConfig`] to the candle [`DevicePreference`] taxonomy.
///
/// Lifted out of `resolve_device_preference` so the mapping is independently
/// testable and reusable from future candle backends.
#[cfg(any(
    feature = "candle-trocr",
    feature = "candle-paddleocr-vl",
    all(
        not(target_arch = "wasm32"),
        any(feature = "candle-glm-ocr", feature = "candle-deepseek-ocr")
    )
))]
fn device_preference_from_acceleration(accel: &AccelerationConfig) -> DevicePreference {
    match accel.provider {
        ExecutionProviderType::Auto => DevicePreference::Auto,
        ExecutionProviderType::Cpu => DevicePreference::Cpu,
        ExecutionProviderType::Cuda | ExecutionProviderType::TensorRt => DevicePreference::Cuda,
        ExecutionProviderType::CoreMl => DevicePreference::Metal,
    }
}
