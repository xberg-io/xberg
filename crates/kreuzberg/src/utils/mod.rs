//! Text utility functions for quality processing and string manipulation.
//!
//! This module provides:
//! - Quality processing: clean OCR artifacts, calculate quality scores
//! - String utilities: safe decoding, mojibake fixing, encoding detection
//! - Object pooling: reusable pools for batch processing to reduce allocations

#[cfg(feature = "quality")]
/// OCR quality analysis helpers (artifact detection, scoring, confidence aggregation).
pub mod quality;

#[cfg(feature = "quality")]
/// String utilities: mojibake repair, encoding detection, safe truncation.
pub mod string_utils;

/// JSON helper utilities for safe value traversal and extraction.
pub mod json_utils;
/// Markdown post-processing helpers used by extractors that emit Markdown output.
pub mod markdown_utils;
/// Generic object pool for reusing allocations across batch operations.
pub mod pool;
/// Heuristics for sizing thread and object pools based on CPU and workload.
pub mod pool_sizing;
/// Interned string pool for reducing allocation pressure on repeated strings.
pub mod string_pool;
/// XML helper utilities for tag-name extraction and attribute traversal.
pub mod xml_utils;

#[cfg(feature = "quality")]
pub(crate) use string_utils::safe_decode;
#[cfg(any(feature = "xml", feature = "office"))]
pub(crate) use xml_utils::xml_tag_name;

#[cfg(any(all(feature = "layout-detection", feature = "pdf"), feature = "office"))]
use std::borrow::Cow;

/// Escape `&`, `<`, and `>` in text destined for markdown/HTML output.
///
/// Underscores are intentionally **not** escaped. In extracted PDF text they are
/// literal content (e.g. identifiers like `CTC_ARP_01`), not markdown italic
/// delimiters.
///
/// Uses a single-pass scan: if no special characters are found, returns a
/// borrowed `Cow` with no allocation.
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
#[inline]
pub(crate) fn escape_html_entities(text: &str) -> Cow<'_, str> {
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
#[cfg(feature = "office")]
#[inline]
#[cfg_attr(alef, alef(skip))]
pub(crate) fn normalize_whitespace(s: &str) -> Cow<'_, str> {
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
