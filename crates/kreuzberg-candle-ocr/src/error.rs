use thiserror::Error;

#[derive(Debug, Error)]
pub enum CandleOcrError {
    #[cfg(not(target_arch = "wasm32"))]
    #[error("candle error: {0}")]
    Candle(#[from] candle_core::Error),

    #[error("tokenizer error: {0}")]
    Tokenizer(String),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("model load failed: {0}")]
    ModelLoadFailed(String),

    #[error("inference failed: {0}")]
    InferenceFailed(String),

    #[error("unsupported configuration: {0}")]
    UnsupportedConfig(String),

    /// A tensor had a shape that did not match what the operation expected.
    #[error("invalid tensor shape: expected {expected}, got {got}")]
    InvalidTensorShape {
        /// Description of the expected shape or rank.
        expected: String,
        /// Description of the actual shape seen.
        got: String,
    },

    /// A named weight was not found in the checkpoint or `VarBuilder`.
    #[error("missing weight: {0}")]
    MissingWeight(String),
}

pub type Result<T> = std::result::Result<T, CandleOcrError>;
