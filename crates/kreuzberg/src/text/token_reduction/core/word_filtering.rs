use crate::text::token_reduction::cjk_utils::CjkTokenizer;
use ahash::AHashMap;

use super::analysis::TextAnalyzer;

/// Handles word filtering and token removal operations.
pub struct WordFilter {
    cjk_tokenizer: CjkTokenizer,
}

impl WordFilter {
    pub fn new() -> Self {
        Self {
            cjk_tokenizer: CjkTokenizer::new(),
        }
    }

    /// Removes additional common words based on frequency and characteristics.
    pub fn remove_additional_common_words(&self, text: &str) -> String {
        let words = self.universal_tokenize(text);

        if words.len() < 4 {
            return text.to_string();
        }

        let estimated_unique = (words.len() as f32 * 0.7).ceil() as usize;
        let mut word_freq = AHashMap::with_capacity(estimated_unique);

        let mut word_lengths = Vec::with_capacity(words.len());

        for word in &words {
            let clean_word = if word.chars().all(|c| c.is_alphabetic()) {
                word.to_lowercase()
            } else {
                word.chars()
                    .filter(|c| c.is_alphabetic())
                    .collect::<String>()
                    .to_lowercase()
            };

            if !clean_word.is_empty() {
                *word_freq.entry(clean_word.clone()).or_insert(0) += 1;
                word_lengths.push(clean_word.chars().count());
            }
        }

        let avg_length = if !word_lengths.is_empty() {
            word_lengths.iter().sum::<usize>() as f32 / word_lengths.len() as f32
        } else {
            5.0
        };

        let original_count = words.len();
        let has_cjk_content = text.chars().any(|c| c as u32 >= 0x4E00 && (c as u32) <= 0x9FFF);

        let mut filtered_words = Vec::with_capacity(words.len());
        for word in &words {
            let clean_word = if word.chars().all(|c| c.is_alphabetic()) {
                word.to_lowercase()
            } else {
                word.chars()
                    .filter(|c| c.is_alphabetic())
                    .collect::<String>()
                    .to_lowercase()
            };

            if clean_word.is_empty() {
                filtered_words.push(word.clone());
            } else {
                let freq = word_freq.get(&clean_word).unwrap_or(&0);
                let word_len = clean_word.chars().count() as f32;

                if TextAnalyzer::has_important_characteristics(word)
                    || (*freq <= 2 && word_len >= avg_length * 0.8)
                    || (word_len >= avg_length * 1.5)
                {
                    filtered_words.push(word.clone());
                }
            }
        }

        let fallback_threshold = if has_cjk_content {
            original_count / 5
        } else {
            original_count / 3
        };

        if filtered_words.len() < fallback_threshold {
            let mut fallback_words = Vec::with_capacity(words.len());
            for word in &words {
                let clean_word = if word.chars().all(|c| c.is_alphabetic()) {
                    word.to_lowercase()
                } else {
                    word.chars().filter(|c| c.is_alphabetic()).collect::<String>()
                };

                if clean_word.is_empty() || clean_word.chars().count() >= 3 || TextAnalyzer::has_important_characteristics(word)
                {
                    fallback_words.push(word.clone());
                }
            }
            self.smart_join(&fallback_words, has_cjk_content)
        } else {
            self.smart_join(&filtered_words, has_cjk_content)
        }
    }

    /// Smart joins tokens based on language type (CJK vs. other).
    pub fn smart_join(&self, tokens: &[String], has_cjk_content: bool) -> String {
        if has_cjk_content {
            tokens.join("")
        } else {
            tokens.join(" ")
        }
    }

    /// Universal tokenizer that handles both CJK and non-CJK text.
    pub fn universal_tokenize(&self, text: &str) -> Vec<String> {
        self.cjk_tokenizer.tokenize_mixed_text(text)
    }
}

impl Default for WordFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universal_tokenize_english() {
        let filter = WordFilter::new();
        let tokens = filter.universal_tokenize("hello world test");
        assert_eq!(tokens, vec!["hello", "world", "test"]);
    }

    #[test]
    fn test_universal_tokenize_cjk() {
        let filter = WordFilter::new();
        let tokens = filter.universal_tokenize("中文");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_fallback_threshold() {
        let filter = WordFilter::new();
        let input = "a the is of to in for on at by";
        let result = filter.remove_additional_common_words(input);
        assert!(!result.is_empty());
    }
}
