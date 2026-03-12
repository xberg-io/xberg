//! Text utility functions for quality processing and string manipulation.
//!
//! This module provides:
//! - Quality processing: clean OCR artifacts, calculate quality scores
//! - String utilities: safe decoding, mojibake fixing, encoding detection
//! - Object pooling: reusable pools for batch processing to reduce allocations

#[cfg(feature = "quality")]
pub mod quality;

#[cfg(feature = "quality")]
pub mod string_utils;

pub mod json_utils;
pub mod pool;
pub mod pool_sizing;
pub mod string_pool;

#[cfg(feature = "quality")]
pub use quality::{calculate_quality_score, clean_extracted_text, normalize_spaces};

#[cfg(feature = "quality")]
pub use string_utils::{calculate_text_confidence, fix_mojibake, safe_decode};

pub use pool::{
    ByteBufferPool, Pool, PoolError, PoolGuard, Recyclable, StringBufferPool, create_byte_buffer_pool,
    create_string_buffer_pool,
};

pub use pool_sizing::{PoolSizeHint, estimate_pool_size};

pub use json_utils::{camel_to_snake, snake_to_camel};
