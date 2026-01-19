use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

static REPEATED_EXCLAMATION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[!]{2,}").expect("Repeated exclamation regex pattern is valid and should compile"));
static REPEATED_QUESTION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[?]{2,}").expect("Repeated question regex pattern is valid and should compile"));
static REPEATED_COMMA: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[,]{2,}").expect("Repeated comma regex pattern is valid and should compile"));

/// Handles punctuation cleaning and normalization.
pub struct PunctuationCleaner;

impl PunctuationCleaner {
    /// Cleans excessive punctuation from text using optimized Cow pattern.
    pub fn clean_punctuation_optimized(text: &str) -> String {
        let mut result = Cow::Borrowed(text);

        if REPEATED_EXCLAMATION.is_match(&result) {
            result = Cow::Owned(REPEATED_EXCLAMATION.replace_all(&result, "!").into_owned());
        }
        if REPEATED_QUESTION.is_match(&result) {
            result = Cow::Owned(REPEATED_QUESTION.replace_all(&result, "?").into_owned());
        }
        if REPEATED_COMMA.is_match(&result) {
            result = Cow::Owned(REPEATED_COMMA.replace_all(&result, ",").into_owned());
        }

        result.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_punctuation_normalization() {
        let input = "Text!!!!!! with????? excessive,,,,,, punctuation";
        let result = PunctuationCleaner::clean_punctuation_optimized(input);

        assert!(!result.contains("!!!!!!"));
        assert!(!result.contains("?????"));
        assert!(!result.contains(",,,,,,"));
    }

    #[test]
    fn test_punctuation_no_change() {
        let input = "Text with normal punctuation!";
        let result = PunctuationCleaner::clean_punctuation_optimized(input);
        assert_eq!(result, input);
    }
}
