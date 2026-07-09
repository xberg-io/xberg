//! Document orientation detection using PP-LCNet_x1_0_doc_ori.
//!
//! The `types` submodule is always available under the `auto-rotate-types` feature
//! (pure-Rust, no ORT dependency). The full detection implementation requires the
//! `auto-rotate` feature and ONNX Runtime.

pub mod types;
pub use types::OrientationResult;

#[cfg(feature = "auto-rotate")]
pub(crate) mod detector;
#[cfg(all(feature = "auto-rotate", feature = "paddle-ocr"))]
pub(crate) use detector::detect_and_rotate;
#[cfg(feature = "auto-rotate")]
pub(crate) use detector::resolve_cache_dir;
#[cfg(feature = "auto-rotate")]
pub use detector::{DocOrientationDetector, MIN_CONFIDENCE};
