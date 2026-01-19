//! Text formatting utilities for RTF content.

/// Normalize whitespace in a string using a single-pass algorithm.
///
/// Collapses multiple consecutive whitespace characters into single spaces
/// and trims leading/trailing whitespace.
pub fn normalize_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_space = false;

    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    result.trim().to_string()
}
