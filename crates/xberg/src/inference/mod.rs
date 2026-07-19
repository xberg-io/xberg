//! Engine-neutral inference seam.
//!
//! xberg runs its ONNX models — layout detection, table classification, document
//! orientation, and more — through a small backend abstraction rather than
//! calling an engine directly. Two traits split the concerns:
//!
//! - [`InferenceBackend`] loads an `.onnx` artifact into a session.
//! - [`InferenceSession`] runs it, exchanging [`InferenceTensor`] values.
//!
//! [`default_backend`] selects the engine at compile time via the `inference_ort`
//! cfg (set by `build.rs` whenever ONNX Runtime is linked, i.e. `ort-bundled` or
//! `ort-dynamic` is active): ONNX Runtime ([`ort_backend::OrtBackend`]) where it
//! links, the pure-Rust tract engine ([`tract_backend::TractBackend`]) on no-ORT
//! targets (WASM, Android x86_64). ORT stays the native default even when the
//! `tract` feature is also on — tract compiles alongside it there only so the two
//! engines can be compared in cross-engine parity tests.
//!
//! Not part of the language-binding surface — the whole module is `pub(crate)`
//! and its files are absent from `alef.toml` sources, so the generator emits
//! nothing for it.
//!
//! Since v5.0.0 (issue #1275).

mod backend;
mod tensor;

#[cfg(inference_ort)]
mod ort_backend;
// On no-ORT builds tract is the real backend. On native (`inference_ort`) the
// `tract` feature compiles it only so the parity tests can compare it against ORT
// — so there it is `cfg(test)`-only, keeping non-test builds dead-code-clean.
#[cfg(all(feature = "tract", any(not(inference_ort), test)))]
mod tract_backend;

pub(crate) use backend::{InferenceBackend, InferenceSession};
pub(crate) use tensor::InferenceTensor;

/// Construct the default inference backend for this build.
///
/// Returns ONNX Runtime where it is linked (`inference_ort`), otherwise the
/// pure-Rust tract backend. The two are mutually exclusive *as the default*: on
/// native, ORT wins even when `tract` is compiled for parity tests; on no-ORT
/// targets only tract is available.
#[cfg(inference_ort)]
pub(crate) fn default_backend() -> Box<dyn InferenceBackend> {
    Box::new(ort_backend::OrtBackend::new())
}

/// tract-only builds (no ONNX Runtime linked): the pure-Rust engine is the default.
#[cfg(all(feature = "tract", not(inference_ort)))]
pub(crate) fn default_backend() -> Box<dyn InferenceBackend> {
    Box::new(tract_backend::TractBackend::new())
}
