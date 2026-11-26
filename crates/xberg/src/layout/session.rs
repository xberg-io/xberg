use ort::session::{Session, builder::SessionBuilder};

use crate::core::config::acceleration::AccelerationConfig;
use crate::layout::error::LayoutError;

/// Names the session options [`session_builder`] sets, for the saved-graph cache key. Change it
/// whenever those options change in a way that can change the optimised graph.
#[cfg(unix)]
pub(crate) const SESSION_RECIPE: &str = "layout::session threads=budget/1 initializers=device v1";

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
    let configure = || session_builder(accel, thread_budget);
    #[cfg(unix)]
    let session = crate::inference::ort_optimized::commit_from_file(
        std::path::Path::new(path),
        crate::ort_discovery::session_is_cpu_only(accel, true),
        SESSION_RECIPE,
        &configure,
    );
    #[cfg(not(unix))]
    let session = configure().and_then(|builder| {
        let mut builder = builder
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::All)
            .map_err(|e| ort::Error::new(e.message()))?;
        Ok(builder.commit_from_file(path)?)
    });

    session.map_err(LayoutError::from_ort_session)
}

/// Everything but the optimisation level of a layout session: thread budget, a single inter-op
/// thread, initializers outside the arena, and the execution providers `accel` selects.
pub(crate) fn session_builder(
    accel: Option<&AccelerationConfig>,
    thread_budget: usize,
) -> Result<SessionBuilder, crate::ort_discovery::OrtSessionError> {
    let builder = Session::builder(crate::ort_discovery::ort_environment()?)?
        .with_intra_threads(thread_budget)
        .map_err(|e| ort::Error::new(e.message()))?
        .with_inter_threads(1)
        .map_err(|e| ort::Error::new(e.message()))?
        .with_device_allocated_initializers()
        .map_err(|e| ort::Error::new(e.message()))?;
    Ok(crate::ort_discovery::apply_execution_providers(builder, accel)?)
}
