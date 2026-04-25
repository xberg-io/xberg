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
pub mod markdown_utils;
pub mod pool;
pub mod pool_sizing;
pub mod string_pool;
pub mod timing;
pub mod xml_utils;

#[cfg(feature = "quality")]
pub use string_utils::{calculate_text_confidence, fix_mojibake, safe_decode};

pub use json_utils::{camel_to_snake, snake_to_camel};
pub(crate) use xml_utils::xml_tag_name;

use std::borrow::Cow;

/// Escape `&`, `<`, and `>` in text destined for markdown/HTML output.
///
/// Underscores are intentionally **not** escaped. In extracted PDF text they are
/// literal content (e.g. identifiers like `CTC_ARP_01`), not markdown italic
/// delimiters.
///
/// Uses a single-pass scan: if no special characters are found, returns a
/// borrowed `Cow` with no allocation.
#[inline]
pub fn escape_html_entities(text: &str) -> Cow<'_, str> {
    let needs_amp = text.contains('&');
    let needs_lt = text.contains('<');
    let needs_gt = text.contains('>');

    if !needs_amp && !needs_lt && !needs_gt {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            _ => result.push(ch),
        }
    }
    Cow::Owned(result)
}

/// Normalizes whitespace by collapsing multiple whitespace characters into single spaces.
/// Returns Cow::Borrowed if no normalization needed.
#[inline]
pub fn normalize_whitespace(s: &str) -> Cow<'_, str> {
    // Check if normalization is needed
    let needs_normalization = s
        .as_bytes()
        .windows(2)
        .any(|w| w[0].is_ascii_whitespace() && w[1].is_ascii_whitespace())
        || s.bytes().any(|b| b != b' ' && b.is_ascii_whitespace());

    if needs_normalization {
        Cow::Owned(s.split_whitespace().collect::<Vec<_>>().join(" "))
    } else {
        Cow::Borrowed(s)
    }
}
