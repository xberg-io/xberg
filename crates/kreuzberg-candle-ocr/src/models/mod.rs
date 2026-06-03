//! OCR model implementations.

pub mod image_processor;

#[cfg(feature = "trocr")]
pub mod trocr;

#[cfg(feature = "trocr")]
pub use trocr::{TrocrEngine, TrocrVariant};

#[cfg(feature = "paddleocr-vl")]
pub mod paddleocr_vl;

#[cfg(feature = "paddleocr-vl")]
pub use paddleocr_vl::{PaddleOcrVlEngine, PaddleOcrVlTask};
