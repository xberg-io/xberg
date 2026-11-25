use ort::session::{Session, builder::SessionBuilder};

use crate::core::config::acceleration::AccelerationConfig;
use crate::layout::error::LayoutError;

/// Build an optimized ORT session from an ONNX model file.
///
/// `thread_budget` controls the number of intra-op threads for this session.
/// Pass the result of [`crate::core::config::concurrency::resolve_thread_budget`]
/// to respect the user's `ConcurrencyConfig`.
///
/// When `accel` is `None` or `Auto`, uses platform defaults:
/// - macOS: CoreML (Neural Engine / GPU)
/// - Linux: CUDA (GPU)
/// - Others: CPU only
///
/// An auto-detected EP (the default) silently falls back to CPU if unavailable. An
/// explicitly-requested EP (e.g. `cuda`, `tensorrt`, `coreml`) that is unavailable or fails to
/// build returns an error instead — see [`crate::ort_discovery::apply_execution_providers`].
pub(crate) fn build_session(
    path: &str,
    accel: Option<&AccelerationConfig>,
    thread_budget: usize,
) -> Result<Session, LayoutError> {
    let configure = || -> Result<SessionBuilder, ort::Error> {
        let builder = Session::builder(crate::ort_discovery::ort_environment()?)?
            .with_intra_threads(thread_budget)
            .map_err(|e| ort::Error::new(e.message()))?
            .with_inter_threads(1)
            .map_err(|e| ort::Error::new(e.message()))?
            .with_device_allocated_initializers()
            .map_err(|e| ort::Error::new(e.message()))?;
        crate::ort_discovery::apply_execution_providers(builder, accel)
    };
    let session = crate::inference::ort_optimized::commit_from_file(
        std::path::Path::new(path),
        crate::ort_discovery::session_is_cpu_only(accel, true),
        &configure,
    )
    .map_err(LayoutError::Ort)?;

    Ok(session)
}
