//! ONNX Runtime library auto-discovery and execution provider configuration.
//!
//! Scans common installation paths and sets `ORT_DYLIB_PATH` so the `ort` crate
//! can find `libonnxruntime` via `dlopen`. Called once at init time.
//!
//! Also provides `apply_execution_providers` for configuring GPU acceleration
//! on ORT session builders across all subsystems (layout, embeddings, OCR, etc.).

#[cfg(not(feature = "ort-bundled"))]
use std::sync::Once;

#[cfg(not(feature = "ort-bundled"))]
static ORT_INIT: Once = Once::new();

/// Ensure ONNX Runtime is discoverable. Safe to call multiple times (no-op after first).
///
/// When the `ort-bundled` feature is enabled the ORT binaries are embedded via the
/// official Microsoft release and no system library search is needed.
pub(crate) fn ensure_ort_available() {
    #[cfg(feature = "ort-bundled")]
    {
        tracing::debug!("ONNX Runtime is bundled; skipping system library discovery");
    }

    #[cfg(not(feature = "ort-bundled"))]
    ORT_INIT.call_once(|| {
        if let Err(msg) = try_discover_ort() {
            tracing::warn!("ONNX Runtime not found: {msg}");
        }
    });
}

#[cfg(not(feature = "ort-bundled"))]
fn try_discover_ort() -> Result<(), &'static str> {
    // Already set and valid?
    if let Ok(path) = std::env::var("ORT_DYLIB_PATH")
        && std::path::Path::new(&path).exists()
    {
        return Ok(());
    }

    let candidates: &[&str] = platform_candidates();

    for path in candidates {
        if std::path::Path::new(path).exists() {
            // SAFETY: single-threaded inside Once::call_once
            #[allow(unsafe_code)]
            unsafe {
                std::env::set_var("ORT_DYLIB_PATH", path);
            }
            tracing::debug!("Auto-discovered ONNX Runtime at {path}");
            return Ok(());
        }
    }

    Err("ONNX Runtime library not found in common installation paths")
}

#[cfg(all(not(feature = "ort-bundled"), target_os = "macos"))]
fn platform_candidates() -> &'static [&'static str] {
    &[
        "/opt/homebrew/lib/libonnxruntime.dylib",
        "/usr/local/lib/libonnxruntime.dylib",
    ]
}

#[cfg(all(not(feature = "ort-bundled"), target_os = "linux"))]
fn platform_candidates() -> &'static [&'static str] {
    &[
        "/usr/lib/libonnxruntime.so",
        "/usr/local/lib/libonnxruntime.so",
        "/usr/lib/x86_64-linux-gnu/libonnxruntime.so",
        "/usr/lib/aarch64-linux-gnu/libonnxruntime.so",
    ]
}

#[cfg(all(not(feature = "ort-bundled"), target_os = "windows"))]
fn platform_candidates() -> &'static [&'static str] {
    &[
        "C:\\Program Files\\onnxruntime\\bin\\onnxruntime.dll",
        "C:\\Windows\\System32\\onnxruntime.dll",
    ]
}

#[cfg(all(
    not(feature = "ort-bundled"),
    not(any(target_os = "macos", target_os = "linux", target_os = "windows"))
))]
fn platform_candidates() -> &'static [&'static str] {
    &[]
}

/// Apply execution providers to an ORT session builder based on [`AccelerationConfig`].
///
/// Shared by all ORT consumers (layout detection, embeddings, PaddleOCR, doc orientation).
///
/// When a GPU provider is **explicitly requested** (e.g. `cuda`, `tensorrt`) and the
/// corresponding execution provider is not available in the loaded ONNX Runtime, this
/// function returns an error with an actionable message. When `auto` is used, unavailable
/// GPU providers fall back to CPU with an info-level log.
///
/// [`AccelerationConfig`]: crate::core::config::acceleration::AccelerationConfig
#[cfg(any(
    feature = "layout-detection",
    feature = "embeddings",
    feature = "paddle-ocr",
    feature = "auto-rotate",
    feature = "reranker",
    feature = "transcription"
))]
pub(crate) fn apply_execution_providers(
    builder: ort::session::builder::SessionBuilder,
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    use crate::core::config::acceleration::ExecutionProviderType;
    use ort::ep::ExecutionProvider;

    // Resolve the execution provider. An `XBERG_ORT_EP` env override wins over the
    // configured provider so operators can force (and A/B) an EP without a recompile,
    // across every ORT subsystem that shares this function (layout, embeddings,
    // PaddleOCR, reranker, auto-rotate, transcription).
    let provider = std::env::var("XBERG_ORT_EP")
        .ok()
        .and_then(|e| match e.trim().to_ascii_lowercase().as_str() {
            "cpu" => Some(ExecutionProviderType::Cpu),
            "coreml" => Some(ExecutionProviderType::CoreMl),
            "cuda" => Some(ExecutionProviderType::Cuda),
            "tensorrt" => Some(ExecutionProviderType::TensorRt),
            "auto" => Some(ExecutionProviderType::Auto),
            _ => None,
        })
        .unwrap_or_else(|| accel.map(|a| a.provider.clone()).unwrap_or(ExecutionProviderType::Auto));
    let device_id = accel.map(|a| a.device_id).unwrap_or(0);

    // Build a CoreML EP, honoring optional env tuning for A/B experiments. Defaults to
    // ORT's own defaults (NeuralNetwork format / All compute units) when unset, so the
    // committed behavior is unchanged. `XBERG_COREML_FORMAT` = mlprogram|neuralnetwork;
    // `XBERG_COREML_UNITS` = all|cpu_and_ne|cpu_and_gpu|cpu_only.
    fn build_coreml_ep() -> ort::ep::CoreML {
        use ort::ep::coreml::{ComputeUnits, ModelFormat};
        let mut ep = ort::ep::CoreML::default();
        if let Ok(fmt) = std::env::var("XBERG_COREML_FORMAT") {
            match fmt.trim().to_ascii_lowercase().as_str() {
                "mlprogram" => ep = ep.with_model_format(ModelFormat::MLProgram),
                "neuralnetwork" | "nn" => ep = ep.with_model_format(ModelFormat::NeuralNetwork),
                other => tracing::warn!(value = other, "ignoring unknown XBERG_COREML_FORMAT"),
            }
        }
        if let Ok(units) = std::env::var("XBERG_COREML_UNITS") {
            match units.trim().to_ascii_lowercase().as_str() {
                "all" => ep = ep.with_compute_units(ComputeUnits::All),
                "cpu_and_ne" => ep = ep.with_compute_units(ComputeUnits::CPUAndNeuralEngine),
                "cpu_and_gpu" => ep = ep.with_compute_units(ComputeUnits::CPUAndGPU),
                "cpu_only" => ep = ep.with_compute_units(ComputeUnits::CPUOnly),
                other => tracing::warn!(value = other, "ignoring unknown XBERG_COREML_UNITS"),
            }
        }
        ep
    }

    let builder = match provider {
        ExecutionProviderType::Cpu => {
            tracing::debug!("ORT session: CPU execution provider (explicit)");
            builder
        }
        ExecutionProviderType::CoreMl => {
            let ep = build_coreml_ep();
            if ep.is_available().unwrap_or(false) {
                tracing::info!("ORT session: CoreML execution provider available, using GPU");
                builder
                    .with_execution_providers([ep.build()])
                    .map_err(|e| ort::Error::new(e.message()))?
            } else {
                return Err(ort::Error::new(
                    "CoreML execution provider requested but not available in the loaded \
                     ONNX Runtime. Set ORT_DYLIB_PATH to an ONNX Runtime build that \
                     includes CoreML support.",
                ));
            }
        }
        ExecutionProviderType::Cuda => {
            let ep = ort::ep::CUDA::default().with_device_id(device_id as i32);
            if ep.is_available().unwrap_or(false) {
                tracing::info!(device_id, "ORT session: CUDA execution provider available, using GPU");
                builder
                    .with_execution_providers([ep.build()])
                    .map_err(|e| ort::Error::new(e.message()))?
            } else {
                return Err(ort::Error::new(
                    "CUDA execution provider requested but not available in the loaded \
                     ONNX Runtime. Install a CUDA-enabled ONNX Runtime and set \
                     ORT_DYLIB_PATH to point at it \
                     (see https://github.com/microsoft/onnxruntime/releases).",
                ));
            }
        }
        ExecutionProviderType::TensorRt => {
            let ep = ort::ep::TensorRT::default().with_device_id(device_id as i32);
            if ep.is_available().unwrap_or(false) {
                tracing::info!(
                    device_id,
                    "ORT session: TensorRT execution provider available, using GPU"
                );
                builder
                    .with_execution_providers([ep.build()])
                    .map_err(|e| ort::Error::new(e.message()))?
            } else {
                return Err(ort::Error::new(
                    "TensorRT execution provider requested but not available in the loaded \
                     ONNX Runtime. Install a TensorRT-enabled ONNX Runtime and set \
                     ORT_DYLIB_PATH to point at it \
                     (see https://github.com/microsoft/onnxruntime/releases).",
                ));
            }
        }
        ExecutionProviderType::Auto => {
            #[cfg(target_os = "macos")]
            let builder = {
                let ep = build_coreml_ep();
                if ep.is_available().unwrap_or(false) {
                    tracing::info!("ORT session: auto — CoreML available, using GPU");
                    builder
                        .with_execution_providers([ep.build()])
                        .map_err(|e| ort::Error::new(e.message()))?
                } else {
                    tracing::info!("ORT session: auto — CoreML not available, using CPU");
                    builder
                }
            };
            #[cfg(target_os = "linux")]
            let builder = {
                let ep = ort::ep::CUDA::default();
                if ep.is_available().unwrap_or(false) {
                    tracing::info!("ORT session: auto — CUDA available, using GPU");
                    builder
                        .with_execution_providers([ep.build()])
                        .map_err(|e| ort::Error::new(e.message()))?
                } else {
                    tracing::info!(
                        "ORT session: auto — CUDA not available, using CPU. \
                         For GPU support, set ORT_DYLIB_PATH to a CUDA-enabled ONNX Runtime."
                    );
                    builder
                }
            };
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let builder = {
                tracing::debug!("ORT session: auto — no platform GPU EP, using CPU");
                builder
            };
            builder
        }
    };

    Ok(builder)
}
