//! Blank page detection utilities.
//!
//! Provides functions to determine if a page is blank based on its text content.
//! A page is considered blank if it contains no meaningful text after normalization.

/// Minimum number of non-whitespace characters to consider a page non-blank.
///
/// Pages with fewer than this many non-whitespace characters are considered blank.
/// This threshold accounts for stray characters, page numbers, or artifacts that
/// may appear on otherwise empty pages.
#[cfg(any(feature = "pdf", feature = "office", feature = "ocr", feature = "ocr-wasm"))]
const MIN_NON_WHITESPACE_CHARS: usize = 3;

/// Determine if a page's text content indicates a blank page.
///
/// A page is blank if it has fewer than [`MIN_NON_WHITESPACE_CHARS`] non-whitespace characters.
///
/// # Arguments
///
/// * `text` - The extracted text content of the page
///
/// # Returns
///
/// `true` if the page is considered blank, `false` otherwise
#[cfg(any(feature = "pdf", feature = "office", feature = "ocr", feature = "ocr-wasm"))]
pub(crate) fn is_page_text_blank(text: &str) -> bool {
    let non_whitespace_count = text.chars().filter(|c| !c.is_whitespace()).count();
    non_whitespace_count < MIN_NON_WHITESPACE_CHARS
}

#[cfg(all(
    test,
    any(feature = "pdf", feature = "office", feature = "ocr", feature = "ocr-wasm")
))]
mod tests {
    use super::*;

    #[test]
    fn empty_string_is_blank() {
        assert!(is_page_text_blank(""));
    }

    #[test]
    fn whitespace_only_is_blank() {
        assert!(is_page_text_blank("   \n\t\n   "));
    }

    #[test]
    fn single_character_is_blank() {
        assert!(is_page_text_blank("1"));
    }

    #[test]
    fn two_characters_is_blank() {
        assert!(is_page_text_blank("- "));
    }

    #[test]
    fn three_non_whitespace_chars_is_not_blank() {
        assert!(!is_page_text_blank("abc"));
    }

    #[test]
    fn meaningful_text_is_not_blank() {
        assert!(!is_page_text_blank("This page has content."));
    }
}
