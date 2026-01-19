use crate::text::utf8_validation;
use ahash::AHashSet;
use once_cell::sync::Lazy;
use regex::Regex;

/// Regular expression for matching excessive newlines (3 or more consecutive newlines).
static EXCESSIVE_NEWLINES_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\n{3,}").expect("Excessive newlines regex pattern is valid and should compile"));

/// Regular expression for matching multiple consecutive spaces (2 or more).
static MULTIPLE_SPACES_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r" {2,}").expect("Multiple spaces regex pattern is valid and should compile"));

/// Normalizes whitespace in text by collapsing multiple spaces into a single space.
///
/// # Arguments
/// * `text` - The input text with potentially multiple consecutive spaces
///
/// # Returns
/// A new `String` with multiple spaces collapsed to single spaces
pub fn normalize_spaces(text: &str) -> String {
    if MULTIPLE_SPACES_REGEX.is_match(text) {
        MULTIPLE_SPACES_REGEX.replace_all(text, " ").into_owned()
    } else {
        text.to_string()
    }
}

/// Reduces excessive newlines in text by collapsing 3+ consecutive newlines into 2.
///
/// # Arguments
/// * `text` - The input text with potentially excessive newlines
///
/// # Returns
/// A new `String` with excessive newlines normalized to at most 2 consecutive newlines
pub fn normalize_newlines(text: &str) -> String {
    if EXCESSIVE_NEWLINES_REGEX.is_match(text) {
        EXCESSIVE_NEWLINES_REGEX.replace_all(text, "\n\n").into_owned()
    } else {
        text.to_string()
    }
}

/// Removes stopwords from text while preserving important patterns.
///
/// This function intelligently filters out common stopwords while preserving:
/// - All-uppercase words (acronyms)
/// - Words containing digits
/// - Words matching custom preserve patterns
/// - Single-letter words
/// - Words with non-alphabetic characters
///
/// # Arguments
/// * `text` - The input text to filter
/// * `stopwords` - Set of stopwords to remove (should be lowercase)
/// * `preserve_patterns` - Regex patterns for words that should never be removed
///
/// # Returns
/// A new `String` with stopwords removed
pub fn remove_stopwords(text: &str, stopwords: &AHashSet<String>, preserve_patterns: &[Regex]) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut filtered_words = Vec::with_capacity((words.len() as f32 * 0.7).ceil() as usize);

    for word in words {
        if word.is_empty() {
            continue;
        }

        // Check preserve patterns first
        if should_preserve_word(word, preserve_patterns) {
            filtered_words.push(word);
            continue;
        }

        // Preserve all-uppercase words (acronyms like API, SDK, HTTP)
        if word.len() > 1 && word.bytes().all(|b| b.is_ascii_uppercase() || !b.is_ascii_alphabetic()) {
            filtered_words.push(word);
            continue;
        }

        // Preserve words containing digits (version numbers, counts, etc.)
        if word.bytes().any(|b| b.is_ascii_digit()) {
            filtered_words.push(word);
            continue;
        }

        // Extract the alphabetic core of the word for stopword matching
        let clean_word = if word.is_ascii() {
            let clean_bytes: Vec<u8> = word
                .bytes()
                .filter(|&b| b.is_ascii_alphabetic())
                .map(|b| b.to_ascii_lowercase())
                .collect();
            utf8_validation::string_from_utf8(clean_bytes).unwrap_or_else(|_| {
                word.chars()
                    .filter(|c| c.is_alphabetic())
                    .collect::<String>()
                    .to_lowercase()
            })
        } else {
            word.chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase()
        };

        // If the clean word is empty (word was all punctuation), preserve it
        if clean_word.is_empty() {
            filtered_words.push(word);
            continue;
        }

        // Preserve single-letter words
        if clean_word.len() <= 1 {
            filtered_words.push(word);
            continue;
        }

        // Check if the clean word is a stopword
        if !stopwords.contains(&clean_word) {
            filtered_words.push(word);
        }
    }

    filtered_words.join(" ")
}

/// Checks if a word should be preserved based on configured patterns.
///
/// # Arguments
/// * `word` - The word to check
/// * `preserve_patterns` - Regex patterns for words that should be preserved
///
/// # Returns
/// `true` if the word matches any preserve pattern, `false` otherwise
#[inline]
pub fn should_preserve_word(word: &str, preserve_patterns: &[Regex]) -> bool {
    preserve_patterns.iter().any(|pattern| pattern.is_match(word))
}

/// Splits a word into prefix (non-alphanumeric), core (alphanumeric), and suffix (non-alphanumeric).
///
/// This is useful for handling punctuation-wrapped words like "(hello)" or "world!".
/// Currently used in tests; reserved for future word boundary-aware filtering.
///
/// # Arguments
/// * `word` - The word to split
///
/// # Returns
/// A tuple of (prefix, core, suffix) strings
pub fn split_word_boundaries(word: &str) -> (String, String, String) {
    let chars: Vec<char> = word.chars().collect();
    let mut start = 0;
    let mut end = chars.len();

    // Find the start of alphanumeric content
    while start < chars.len() && !chars[start].is_alphanumeric() {
        start += 1;
    }

    // Find the end of alphanumeric content
    while end > start && !chars[end - 1].is_alphanumeric() {
        end -= 1;
    }

    let prefix: String = chars[..start].iter().collect();
    let core: String = chars[start..end].iter().collect();
    let suffix: String = chars[end..].iter().collect();

    (prefix, core, suffix)
}

#[cfg(all(test, feature = "stopwords"))]
mod tests {
    use super::*;

    fn create_test_stopwords() -> AHashSet<String> {
        let mut set = AHashSet::new();
        set.insert("the".to_string());
        set.insert("is".to_string());
        set.insert("a".to_string());
        set.insert("and".to_string());
        set.insert("with".to_string());
        set.insert("by".to_string());
        set
    }

    #[test]
    fn test_normalize_spaces() {
        let input = "Text  with    multiple     spaces";
        let result = normalize_spaces(input);
        assert!(!result.contains("  "));
        assert!(result.contains("Text with multiple spaces"));
    }

    #[test]
    fn test_normalize_spaces_no_change() {
        let input = "Text with single spaces";
        let result = normalize_spaces(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_normalize_newlines() {
        let input = "Paragraph 1\n\n\n\n\nParagraph 2";
        let result = normalize_newlines(input);
        assert!(!result.contains("\n\n\n"));
        assert!(result.contains("Paragraph 1"));
        assert!(result.contains("Paragraph 2"));
    }

    #[test]
    fn test_normalize_newlines_no_change() {
        let input = "Paragraph 1\n\nParagraph 2";
        let result = normalize_newlines(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_remove_stopwords() {
        let stopwords = create_test_stopwords();
        let preserve_patterns = vec![];

        let input = "The quick brown fox is jumping over the lazy dog";
        let result = remove_stopwords(input, &stopwords, &preserve_patterns);

        assert!(!result.contains(" the "));
        assert!(!result.contains(" is "));
        assert!(result.contains("quick"));
        assert!(result.contains("brown"));
        assert!(result.contains("fox"));
    }

    #[test]
    fn test_remove_stopwords_preserves_uppercase() {
        let stopwords = create_test_stopwords();
        let preserve_patterns = vec![];

        let input = "The API is working WITH the SDK";
        let result = remove_stopwords(input, &stopwords, &preserve_patterns);

        assert!(result.contains("API"));
        assert!(result.contains("SDK"));
        assert!(result.contains("WITH"));
        assert!(!result.contains("The "));
        assert!(!result.contains(" is "));
    }

    #[test]
    fn test_remove_stopwords_preserves_numbers() {
        let stopwords = create_test_stopwords();
        let preserve_patterns = vec![];

        let input = "The version is 3.14 and the count is 42";
        let result = remove_stopwords(input, &stopwords, &preserve_patterns);

        assert!(result.contains("3.14"));
        assert!(result.contains("42"));
        assert!(result.contains("version"));
        assert!(result.contains("count"));
    }

    #[cfg_attr(coverage, ignore = "coverage instrumentation disables SIMD stopword paths")]
    #[test]
    fn test_remove_stopwords_handles_punctuation() {
        let stopwords = create_test_stopwords();
        let preserve_patterns = vec![];

        let input = "Hello, the world! This is great.";
        let result = remove_stopwords(input, &stopwords, &preserve_patterns);

        assert!(result.contains("Hello,"));
        assert!(result.contains("world!"));
        assert!(result.contains("great."));
    }

    #[test]
    fn test_remove_stopwords_single_letter() {
        let stopwords = create_test_stopwords();
        let preserve_patterns = vec![];

        let input = "I a x test";
        let result = remove_stopwords(input, &stopwords, &preserve_patterns);

        assert!(result.contains("I"));
        assert!(result.contains("x"));
    }

    #[test]
    fn test_preserve_patterns() {
        let stopwords = create_test_stopwords();
        let preserve_patterns = vec![
            Regex::new(r"\b[A-Z]{2,}\b").unwrap(),
            Regex::new(r"\b\d+\.\d+\.\d+\b").unwrap(),
            Regex::new(r"@\w+").unwrap(),
        ];

        let input = "The NASA and HTTP protocols version 1.2.3 by @john";
        let result = remove_stopwords(input, &stopwords, &preserve_patterns);

        assert!(result.contains("NASA"));
        assert!(result.contains("HTTP"));
        assert!(result.contains("1.2.3"));
        assert!(result.contains("@john"));

        assert!(!result.contains(" the "));
        assert!(!result.contains(" and "));
        assert!(!result.contains(" by "));
    }

    #[test]
    fn test_should_preserve_word() {
        let patterns = vec![
            Regex::new(r"\b[A-Z]{2,}\b").unwrap(),
        ];

        assert!(should_preserve_word("NASA", &patterns));
        assert!(should_preserve_word("HTTP", &patterns));
        assert!(!should_preserve_word("hello", &patterns));
    }

    #[test]
    fn test_split_word_boundaries() {
        let (prefix, core, suffix) = split_word_boundaries("(hello)");
        assert_eq!(prefix, "(");
        assert_eq!(core, "hello");
        assert_eq!(suffix, ")");

        let (prefix2, core2, suffix2) = split_word_boundaries("world!");
        assert_eq!(prefix2, "");
        assert_eq!(core2, "world");
        assert_eq!(suffix2, "!");

        let (prefix3, core3, suffix3) = split_word_boundaries("'test");
        assert_eq!(prefix3, "'");
        assert_eq!(core3, "test");
        assert_eq!(suffix3, "");

        let (prefix4, core4, suffix4) = split_word_boundaries("simple");
        assert_eq!(prefix4, "");
        assert_eq!(core4, "simple");
        assert_eq!(suffix4, "");

        let (prefix5, core5, suffix5) = split_word_boundaries("\"example!!!\"");
        assert_eq!(prefix5, "\"");
        assert_eq!(core5, "example");
        assert_eq!(suffix5, "!!!\"");
    }

    #[test]
    fn test_split_word_boundaries_edge_cases() {
        let (prefix, core, suffix) = split_word_boundaries("!!!");
        assert_eq!(prefix, "!!!");
        assert_eq!(core, "");
        assert_eq!(suffix, "");

        let (prefix2, core2, suffix2) = split_word_boundaries("");
        assert_eq!(prefix2, "");
        assert_eq!(core2, "");
        assert_eq!(suffix2, "");

        let (prefix3, core3, suffix3) = split_word_boundaries("a");
        assert_eq!(prefix3, "");
        assert_eq!(core3, "a");
        assert_eq!(suffix3, "");

        let (prefix4, core4, suffix4) = split_word_boundaries("(café)");
        assert_eq!(prefix4, "(");
        assert_eq!(core4, "café");
        assert_eq!(suffix4, ")");
    }

    #[test]
    fn test_lazy_regex_initialization() {
        let _ = &*EXCESSIVE_NEWLINES_REGEX;
        let _ = &*MULTIPLE_SPACES_REGEX;
    }
}
