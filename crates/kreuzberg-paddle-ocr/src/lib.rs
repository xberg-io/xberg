//! # kreuzberg-paddle-ocr
//!
//! PaddleOCR via ONNX Runtime for Kreuzberg - high-performance text detection and recognition.
//!
//! This crate is vendored from [paddle-ocr-rs](https://github.com/mg-chao/paddle-ocr-rs)
//! by mg-chao, with modifications for Kreuzberg integration.
//!
//! ## Original License
//!
//! The original paddle-ocr-rs is licensed under Apache-2.0.
//! This vendored version is relicensed to MIT with the original author's copyright retained.

#![allow(clippy::too_many_arguments)]

pub mod angle_net;
pub mod base_net;
pub mod crnn_net;
pub mod db_net;
pub mod ocr_error;
pub mod ocr_lite;
pub mod ocr_result;
pub mod ocr_utils;
pub mod scale_param;

pub use ocr_error::OcrError;
pub use ocr_lite::OcrLite;
pub use ocr_result::{Angle, OcrResult, Point, TextBlock, TextBox, TextLine};
