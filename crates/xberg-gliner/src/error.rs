use thiserror::Error;

/// Result type used by `xberg-gliner`.
pub type Result<T> = std::result::Result<T, GlinerError>;

/// Errors returned by GLiNER preprocessing, inference, and decoding.
#[derive(Debug, Error)]
pub enum GlinerError {
    /// Input text or label data is invalid.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// Tokenizer loading or encoding failed.
    #[error("tokenizer error: {0}")]
    Tokenizer(String),
    /// Regex splitter construction failed.
    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),
    /// ONNX Runtime failed.
    #[error("onnx runtime error: {0}")]
    Ort(#[from] ort::Error),
    /// An expected tensor was missing from model output.
    #[error("missing model output tensor '{0}'")]
    MissingOutput(&'static str),
    /// The loaded model does not expose the expected input or output names.
    #[error("unexpected model {kind} schema: expected {expected:?}, got {actual:?}")]
    UnexpectedModelSchema {
        /// Schema side being validated.
        kind: &'static str,
        /// Required names.
        expected: Vec<&'static str>,
        /// Actual names exposed by the model.
        actual: Vec<String>,
    },
    /// The logits tensor shape did not match the span-mode decoder contract.
    #[error("unexpected logits shape: expected {expected:?}, got {actual:?}")]
    UnexpectedLogitsShape {
        /// Expected dimensions.
        expected: Vec<usize>,
        /// Actual dimensions.
        actual: Vec<usize>,
    },
    /// Internal metadata referred to a missing item.
    #[error("index error: {target}[{index}] is missing")]
    Index {
        /// Indexed collection name.
        target: &'static str,
        /// Missing index.
        index: usize,
    },
    /// Source text offsets were not valid UTF-8 boundaries.
    #[error("invalid source text offsets {start}..{end}")]
    InvalidOffsets {
        /// Start byte offset.
        start: usize,
        /// End byte offset.
        end: usize,
    },
}
