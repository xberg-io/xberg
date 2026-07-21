//! Backend and session traits for the engine-neutral inference seam.
//!
//! Two traits split model loading from model execution:
//!
//! - [`InferenceBackend`] is a factory — it turns an ONNX artifact into a
//!   runnable session. The concrete backend is chosen at compile time by
//!   [`super::default_backend`]: ONNX Runtime on native builds, tract on no-ORT
//!   targets (later phase). A byte-buffer `load_from_memory` variant is added in
//!   the WASM/Android phase, where weights are embedded rather than read from a
//!   file.
//! - [`InferenceSession`] is the runner — it takes named [`InferenceTensor`]
//!   inputs and returns named outputs. `run` takes `&self` so a single session can
//!   be shared across threads (page-parallel layout), matching how xberg's ORT
//!   sessions are used today.
//!
//! Since v5.0.0 (issue #1275).

use std::path::Path;

use crate::core::config::acceleration::AccelerationConfig;

use super::tensor::InferenceTensor;

/// An error from loading or running a model through the inference seam.
///
/// Callers map this into their module-specific error (e.g. `LayoutError`,
/// `XbergError`) at the migration site, so no engine detail leaks past the seam.
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    /// The model could not be loaded (bad path/bytes, or the runtime is missing).
    #[error("failed to load inference model: {0}")]
    Load(String),
    /// Inference execution failed.
    #[error("inference run failed: {0}")]
    Run(String),
    /// A tensor could not be converted across the engine boundary.
    #[error("tensor conversion failed: {0}")]
    Tensor(String),
}

/// A loaded, runnable model.
///
/// `run` is `&self` (not `&mut self`) so one session can serve concurrent
/// callers; backends provide the necessary interior synchronization.
pub trait InferenceSession: Send + Sync {
    /// Run inference on the named inputs, returning the named outputs in the
    /// model's output order.
    fn run(&self, inputs: Vec<(String, InferenceTensor)>) -> Result<Vec<(String, InferenceTensor)>, InferenceError>;

    /// The model's input names, in graph order.
    fn input_names(&self) -> &[String];
}

/// A factory that loads ONNX models into [`InferenceSession`]s.
pub trait InferenceBackend: Send + Sync {
    /// Load a model from a filesystem path.
    fn load(
        &self,
        model_path: &Path,
        accel: Option<&AccelerationConfig>,
    ) -> Result<Box<dyn InferenceSession>, InferenceError>;

    /// Load a model with an explicit intra-op thread budget.
    ///
    /// Backends without configurable session threads may use the default
    /// implementation. Native ORT overrides this for batch layout planning.
    fn load_with_thread_budget(
        &self,
        model_path: &Path,
        accel: Option<&AccelerationConfig>,
        thread_budget: usize,
    ) -> Result<Box<dyn InferenceSession>, InferenceError> {
        let _ = thread_budget;
        self.load(model_path, accel)
    }

    /// Load a model from an in-memory ONNX byte buffer.
    ///
    /// Used where there is no model file to read — WASM (weights embedded via
    /// `include_bytes!` or streamed from JS) and any caller that already holds the
    /// bytes. Native callers normally use [`load`](Self::load) with a cached path.
    ///
    /// Landed as seam infrastructure ahead of its consumer: the only callers today
    /// are the cross-engine parity tests. The WASM embedded-weight path wires the
    /// first production caller in a later phase, so it is `dead_code`-allowed until
    /// then rather than gated behind a narrower cfg.
    #[allow(dead_code)]
    fn load_from_memory(
        &self,
        model_bytes: &[u8],
        accel: Option<&AccelerationConfig>,
    ) -> Result<Box<dyn InferenceSession>, InferenceError>;
}
