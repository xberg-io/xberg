use thiserror::Error;

/// Errors that can occur during layout detection inference or model management.
#[cfg_attr(alef, alef(skip))]
#[derive(Error, Debug)]
pub enum LayoutError {
    /// ONNX Runtime returned an error during session creation or inference.
    ///
    /// Only constructible under the ORT-backed `layout-detection` feature (session.rs,
    /// `tatr.rs`, `slanet.rs` are the sole producers); absent under the pure-Rust
    /// `layout-tract` variant, where seam-based models surface `Inference` instead.
    #[cfg(feature = "layout-detection")]
    #[error("ORT error: {0}")]
    Ort(#[from] ort::Error),
    /// A model on the engine-neutral [`crate::inference`] seam failed to load or
    /// run. Engine-agnostic (ORT or tract) — carries the seam error's message so
    /// the layout models need not name a concrete engine's error type.
    #[error("inference error: {0}")]
    Inference(String),
    /// Image decoding or preprocessing failed.
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    /// The ONNX session was not initialized before calling `detect`.
    #[error("Session not initialized")]
    SessionNotInitialized,
    /// The model returned output tensors with an unexpected shape or type.
    #[error("Invalid model output: {0}")]
    InvalidOutput(String),
    /// Downloading or locating the model weights file failed.
    #[error("Model download failed: {0}")]
    ModelDownload(String),
}

#[cfg(feature = "layout-detection")]
impl LayoutError {
    /// A runtime that could not be loaded becomes `Inference`: it carries no `ort::Error`, and
    /// building one would make ort retry its own library search. ~keep
    pub(crate) fn from_ort_session(error: crate::ort_discovery::OrtSessionError) -> Self {
        match error {
            crate::ort_discovery::OrtSessionError::Ort(error) => Self::Ort(error),
            #[cfg(ort_dynamic_discovery)]
            unavailable @ crate::ort_discovery::OrtSessionError::Runtime(_) => Self::Inference(unavailable.to_string()),
        }
    }
}
