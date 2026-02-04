use thiserror::Error;

#[derive(Error, Debug)]
pub enum OcrError {
    #[error("Ort error: {0}")]
    Ort(#[from] ort::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Session not initialized")]
    SessionNotInitialized,
}
