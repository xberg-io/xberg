//! Lightweight markdown detection utilities.
//!
//! Always available (no feature gate). Used by both the `chunking` and
//! `quality` features to detect markdown structure without pulling in
//! heavy dependencies.

use once_cell::sync::Lazy;
use regex::Regex;

/// ATX heading pattern: 1-6 `#` characters followed by whitespace.
static MARKDOWN_HEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^#{1,6}\s+").expect("markdown header regex"));

/// Check whether a line is a markdown ATX header (`# ...` through `###### ...`).
#[inline]
pub fn is_markdown_header(line: &str) -> bool {
    MARKDOWN_HEADER_RE.is_match(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_h1_through_h6() {
        assert!(is_markdown_header("# H1"));
        assert!(is_markdown_header("## H2"));
        assert!(is_markdown_header("### H3"));
        assert!(is_markdown_header("#### H4"));
        assert!(is_markdown_header("##### H5"));
        assert!(is_markdown_header("###### H6"));
    }

    #[test]
    fn rejects_non_headers() {
        assert!(!is_markdown_header("Regular text"));
        assert!(!is_markdown_header("- List item"));
        assert!(!is_markdown_header("#no space"));
        assert!(!is_markdown_header("####### Seven hashes"));
    }
}
