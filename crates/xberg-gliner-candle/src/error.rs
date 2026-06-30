use thiserror::Error;

/// Result type used by `xberg-gliner-candle`.
pub type Result<T> = std::result::Result<T, GlinerCandleError>;

/// Errors returned by Candle GLiNER2 inference, LoRA loading, and merge.
#[derive(Debug, Error)]
pub enum GlinerCandleError {
    /// Underlying Candle tensor/model error.
    #[error("candle error: {0}")]
    Candle(#[from] candle_core::Error),
    /// `xberg-gliner` prompt-encoding or decode error.
    #[error("gliner error: {0}")]
    Gliner(#[from] xberg_gliner::GlinerError),
    /// Filesystem or config-parsing error during model/adapter loading.
    #[error("backend error: {0}")]
    Backend(String),
}
