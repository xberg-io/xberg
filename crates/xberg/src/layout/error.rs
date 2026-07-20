use thiserror::Error;

/// Errors that can occur during layout detection inference or model management.
#[cfg_attr(alef, alef(skip))]
#[derive(Error, Debug)]
pub enum LayoutError {
    /// ONNX Runtime returned an error during session creation or inference.
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
