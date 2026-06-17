use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse file {path}: {reason}")]
    Parse { path: PathBuf, reason: String },

    #[error("validation failed for {path}: {reason}")]
    Validation { path: PathBuf, reason: String },

    #[error("validator not available for language: {0}")]
    ValidatorUnavailable(String),

    #[error("command timed out after {timeout_secs}s: {command}")]
    Timeout { command: String, timeout_secs: u64 },

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
