use crate::error::{KreuzbergError, Result};
use crate::stopwords::STOPWORDS;
use crate::text::token_reduction::config::TokenReductionConfig;
use ahash::{AHashMap, AHashSet};
use regex::Regex;
use std::sync::Arc;

// Import filter modules
mod general;
mod html;
mod markdown;

// Re-export all filter functions for backward compatibility
pub use general::{normalize_newlines, normalize_spaces, remove_stopwords, should_preserve_word, split_word_boundaries};
pub use html::remove_html_comments;
pub use markdown::{
    extract_and_preserve_code, is_markdown_header, is_markdown_list, is_markdown_table, preserve_markdown_structure,
    restore_preserved_blocks,
};

/// Main filter pipeline orchestrator that coordinates various text filtering operations.
///
/// The `FilterPipeline` provides a high-level interface for applying different levels
/// of text filtering, from light cleaning (HTML comments, whitespace) to moderate
/// filtering (stopword removal) while respecting preservation rules for code,
/// markdown, and custom patterns.
pub struct FilterPipeline {
    config: Arc<TokenReductionConfig>,
    stopwords: AHashSet<String>,
    preserve_patterns: Vec<Regex>,
    language: String,
}

impl FilterPipeline {
    /// Creates a new `FilterPipeline` with the specified configuration and language.
    ///
    /// # Arguments
    /// * `config` - Token reduction configuration
    /// * `language` - Language code for stopword selection (e.g., "en", "es", "de")
    ///
    /// # Returns
    /// A `Result` containing the new `FilterPipeline` or an error if regex patterns are invalid
    ///
    /// # Errors
    /// Returns a `KreuzbergError::Validation` if any preserve patterns are invalid regex
    pub fn new(config: &Arc<TokenReductionConfig>, language: &str) -> Result<Self> {
        let mut stopwords = STOPWORDS.get(language).cloned().unwrap_or_else(|| {
            STOPWORDS
                .get("en")
                .cloned()
                .expect("English stopwords must be available - indicates build failure if missing")
        });

        if let Some(ref custom) = config.custom_stopwords
            && let Some(custom_for_lang) = custom.get(language)
        {
            for word in custom_for_lang {
                stopwords.insert(word.to_lowercase());
            }
        }

        let preserve_patterns: std::result::Result<Vec<Regex>, _> = config
            .preserve_patterns
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect();

        let preserve_patterns =
            preserve_patterns.map_err(|e| KreuzbergError::validation(format!("Invalid regex pattern: {}", e)))?;

        Ok(Self {
            config: Arc::clone(config),
            stopwords,
            preserve_patterns,
            language: language.to_string(),
        })
    }

    /// Applies light filtering to text, removing HTML comments and normalizing whitespace.
    ///
    /// Light filters include:
    /// - HTML comment removal
    /// - Multiple space normalization
    /// - Excessive newline reduction
    /// - Markdown structure preservation (if enabled)
    /// - Code preservation (if enabled)
    ///
    /// # Arguments
    /// * `text` - The input text to filter
    ///
    /// # Returns
    /// A new `String` with light filters applied
    pub fn apply_light_filters(&self, text: &str) -> String {
        use std::borrow::Cow;

        let mut result = Cow::Borrowed(text);

        // Preserve markdown code blocks if configured
        let mut preserved_blocks: Option<AHashMap<String, String>> = None;
        if self.config.preserve_markdown {
            let mut blocks = AHashMap::new();
            result = Cow::Owned(extract_and_preserve_code(result.as_ref(), &mut blocks));
            preserved_blocks = Some(blocks);
        }

        // Remove HTML comments
        result = Cow::Owned(remove_html_comments(&result));

        // Normalize whitespace
        result = Cow::Owned(normalize_spaces(&result));
        result = Cow::Owned(normalize_newlines(&result));

        // Preserve markdown structure if configured
        if self.config.preserve_markdown {
            result = Cow::Owned(preserve_markdown_structure(&result));
        }

        // Restore preserved code blocks
        if let Some(blocks) = &preserved_blocks {
            result = Cow::Owned(restore_preserved_blocks(&result, blocks));
        }

        result.into_owned()
    }

    /// Applies moderate filtering to text, including stopword removal.
    ///
    /// Moderate filters include all light filters plus:
    /// - Stopword removal (with markdown awareness if enabled)
    /// - Code preservation during stopword removal
    ///
    /// # Arguments
    /// * `text` - The input text to filter
    ///
    /// # Returns
    /// A new `String` with moderate filters applied
    pub fn apply_moderate_filters(&self, text: &str) -> String {
        let mut result = self.apply_light_filters(text);

        // Preserve code blocks during stopword removal if configured
        let mut preserved_blocks: Option<AHashMap<String, String>> = None;
        if self.config.preserve_code {
            let mut blocks = AHashMap::new();
            result = extract_and_preserve_code(&result, &mut blocks);
            preserved_blocks = Some(blocks);
        }

        // Remove stopwords with markdown awareness if configured
        if self.config.preserve_markdown {
            result = self.remove_stopwords_preserving_markdown(&result);
        } else {
            result = remove_stopwords(&result, &self.stopwords, &self.preserve_patterns);
        }

        // Restore preserved code blocks
        if let Some(blocks) = &preserved_blocks {
            result = restore_preserved_blocks(&result, blocks);
        }

        result
    }

    /// Removes stopwords while preserving markdown structural elements.
    ///
    /// This function processes text line-by-line, preserving:
    /// - Markdown headers
    /// - List items
    /// - Table rows
    ///
    /// # Arguments
    /// * `text` - The input text to filter
    ///
    /// # Returns
    /// A new `String` with stopwords removed but markdown structure preserved
    fn remove_stopwords_preserving_markdown(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut processed_lines = Vec::with_capacity(lines.len());

        for line in lines {
            // Preserve markdown headers
            if is_markdown_header(line) {
                processed_lines.push(line.to_string());
                continue;
            }

            // Preserve markdown list items
            if is_markdown_list(line) {
                processed_lines.push(line.to_string());
                continue;
            }

            // Preserve markdown table rows
            if is_markdown_table(line) {
                processed_lines.push(line.to_string());
                continue;
            }

            // Apply stopword removal to regular text lines
            let processed_line = remove_stopwords(line, &self.stopwords, &self.preserve_patterns);
            processed_lines.push(processed_line);
        }

        processed_lines.join("\n")
    }

    /// Gets the language code for this filter pipeline.
    ///
    /// Primarily useful for testing and debugging to verify language configuration.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn language(&self) -> &str {
        &self.language
    }
}

#[cfg(all(test, feature = "stopwords"))]
mod tests {
    use super::*;

    #[test]
    fn test_stopword_removal() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The quick brown fox is jumping over the lazy dog";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(!result.contains(" the "));
        assert!(!result.contains(" is "));
        assert!(result.contains("quick"));
        assert!(result.contains("brown"));
        assert!(result.contains("fox"));
    }

    #[test]
    fn test_preserve_patterns() {
        let config = TokenReductionConfig {
            preserve_patterns: vec!["\\b[A-Z]{2,}\\b".to_string()],
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The NASA mission is a success";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(result.contains("NASA"));
        assert!(result.contains("mission"));
        assert!(result.contains("success"));
    }

    #[test]
    fn test_markdown_preservation() {
        let config = TokenReductionConfig {
            preserve_markdown: true,
            preserve_code: true,
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "# Header\nThis is `code` and ```\ncode block\n``` text";
        let result = pipeline.apply_moderate_filters(input);

        assert!(result.contains("# Header"));
        assert!(result.contains("`code`"));
        assert!(result.contains("```\ncode block\n```"));
    }

    #[test]
    fn test_apply_light_filters_removes_html_comments() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "Text before <!-- comment --> text after";
        let result = pipeline.apply_light_filters(input);

        assert!(!result.contains("<!-- comment -->"));
        assert!(result.contains("Text before"));
        assert!(result.contains("text after"));
    }

    #[test]
    fn test_apply_light_filters_normalizes_whitespace() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "Text  with    multiple     spaces";
        let result = pipeline.apply_light_filters(input);

        assert!(!result.contains("  "));
        assert!(result.contains("Text with multiple spaces"));
    }

    #[test]
    fn test_apply_light_filters_reduces_newlines() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "Paragraph 1\n\n\n\n\nParagraph 2";
        let result = pipeline.apply_light_filters(input);

        assert!(!result.contains("\n\n\n"));
        assert!(result.contains("Paragraph 1"));
        assert!(result.contains("Paragraph 2"));
    }

    #[test]
    fn test_stopword_removal_preserves_uppercase() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The API is working WITH the SDK";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(result.contains("API"));
        assert!(result.contains("SDK"));
        assert!(result.contains("WITH"));
        assert!(!result.contains("The "));
        assert!(!result.contains(" is "));
    }

    #[test]
    fn test_stopword_removal_preserves_numbers() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The version is 3.14 and the count is 42";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(result.contains("3.14"));
        assert!(result.contains("42"));
        assert!(result.contains("version"));
        assert!(result.contains("count"));
    }

    #[cfg_attr(coverage, ignore = "coverage instrumentation disables SIMD stopword paths")]
    #[test]
    fn test_stopword_removal_handles_punctuation() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "Hello, the world! This is great.";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(result.contains("Hello,"));
        assert!(result.contains("world!"));
        assert!(result.contains("great."));
    }

    #[cfg_attr(coverage, ignore = "coverage instrumentation disables SIMD stopword paths")]
    #[test]
    fn test_custom_stopwords() {
        use std::collections::HashMap;

        let mut custom_stopwords = HashMap::new();
        custom_stopwords.insert("en".to_string(), vec!["custom".to_string(), "word".to_string()]);

        let config = TokenReductionConfig {
            custom_stopwords: Some(custom_stopwords),
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "This is a custom word test";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(!result.contains("custom"));
        assert!(!result.contains("word"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_spanish_stopwords() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "es").unwrap();

        let input = "El perro grande bonito tiene";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(result.contains("perro"));
        assert!(result.contains("grande"));
        assert!(result.contains("bonito"));
        let words: Vec<&str> = result.split_whitespace().collect();
        assert!(!words.contains(&"el"));
        assert!(!words.contains(&"El"));
    }

    #[cfg_attr(coverage, ignore = "coverage instrumentation disables SIMD stopword paths")]
    #[test]
    fn test_unknown_language_fallback() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "unknown").unwrap();

        let input = "The quick test with unknown language";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(!result.contains("The "));
        assert!(result.contains("quick"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_markdown_header_preservation() {
        let config = TokenReductionConfig {
            preserve_markdown: true,
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "# Header 1\n## Header 2\n### Header 3\nRegular text";
        let result = pipeline.remove_stopwords_preserving_markdown(input);

        assert!(result.contains("# Header 1"));
        assert!(result.contains("## Header 2"));
        assert!(result.contains("### Header 3"));
    }

    #[test]
    fn test_markdown_list_preservation() {
        let config = TokenReductionConfig {
            preserve_markdown: true,
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "- Item 1\n* Item 2\n+ Item 3";
        let result = pipeline.remove_stopwords_preserving_markdown(input);

        assert!(result.contains("- Item 1"));
        assert!(result.contains("* Item 2"));
        assert!(result.contains("+ Item 3"));
    }

    #[test]
    fn test_markdown_table_preservation() {
        let config = TokenReductionConfig {
            preserve_markdown: true,
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        let result = pipeline.remove_stopwords_preserving_markdown(input);

        assert!(result.contains("| Header 1 | Header 2 |"));
        assert!(result.contains("|----------|----------|"));
    }

    #[test]
    fn test_code_block_preservation() {
        let config = Arc::new(TokenReductionConfig {
            preserve_code: true,
            ..Default::default()
        });
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let mut preserved = AHashMap::new();
        let input = "Text before\n```rust\nfn main() {}\n```\nText after";
        let result = extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 1);
        assert!(preserved.values().any(|v| v.contains("fn main()")));
        assert!(result.contains("__CODEBLOCK_0__"));
    }

    #[test]
    fn test_inline_code_preservation() {
        let config = Arc::new(TokenReductionConfig {
            preserve_code: true,
            ..Default::default()
        });
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let mut preserved = AHashMap::new();
        let input = "Use the `println!` macro";
        let result = extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 1);
        assert!(preserved.values().any(|v| v == "`println!`"));
        assert!(result.contains("__INLINECODE_0__"));
    }

    #[test]
    fn test_restore_preserved_blocks() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let mut preserved = AHashMap::new();
        preserved.insert("__CODEBLOCK_0__".to_string(), "```code```".to_string());
        preserved.insert("__INLINECODE_0__".to_string(), "`inline`".to_string());
        let input = "Text __CODEBLOCK_0__ and __INLINECODE_0__ here";
        let result = restore_preserved_blocks(input, &preserved);

        assert!(result.contains("```code```"));
        assert!(result.contains("`inline`"));
        assert!(!result.contains("__CODEBLOCK_0__"));
        assert!(!result.contains("__INLINECODE_0__"));
    }

    #[test]
    fn test_apply_moderate_filters_with_stopwords() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The quick brown fox is jumping";
        let result = pipeline.apply_moderate_filters(input);

        assert!(!result.contains("The "));
        assert!(!result.contains(" is "));
        assert!(result.contains("quick"));
        assert!(result.contains("brown"));
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let config = TokenReductionConfig {
            preserve_patterns: vec!["[invalid".to_string()],
            ..Default::default()
        };

        let config = Arc::new(config);
        let result = FilterPipeline::new(&config, "en");

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, KreuzbergError::Validation { .. }));
        }
    }

    #[test]
    fn test_empty_input() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let result = pipeline.apply_light_filters("");
        assert_eq!(result, "");

        let result = pipeline.apply_moderate_filters("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_stopword_removal_single_letter_words() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "I a x test";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(result.contains("I"));
        assert!(result.contains("x"));
    }

    #[cfg_attr(coverage, ignore = "coverage instrumentation disables SIMD stopword paths")]
    #[test]
    fn test_stopword_removal_mixed_case() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The Test Is Working";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(!result.contains("The"));
        assert!(!result.contains("Is"));
        assert!(result.contains("Test"));
        assert!(result.contains("Working"));
    }

    #[test]
    fn test_multiple_code_blocks_hashmap_approach() {
        let config = Arc::new(TokenReductionConfig {
            preserve_code: true,
            ..Default::default()
        });
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input =
            "Start ```rust\nlet x = 1;\n``` middle `inline1` text ```python\nprint('hi')\n``` and `inline2` end";
        let mut preserved = AHashMap::new();
        let result = extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 4);
        assert!(preserved.contains_key("__CODEBLOCK_0__"));
        assert!(preserved.contains_key("__CODEBLOCK_1__"));
        assert!(preserved.contains_key("__INLINECODE_0__"));
        assert!(preserved.contains_key("__INLINECODE_1__"));

        assert_eq!(preserved.get("__CODEBLOCK_0__").unwrap(), "```rust\nlet x = 1;\n```");
        assert_eq!(preserved.get("__CODEBLOCK_1__").unwrap(), "```python\nprint('hi')\n```");
        assert_eq!(preserved.get("__INLINECODE_0__").unwrap(), "`inline1`");
        assert_eq!(preserved.get("__INLINECODE_1__").unwrap(), "`inline2`");

        let restored = restore_preserved_blocks(&result, &preserved);
        assert!(restored.contains("```rust\nlet x = 1;\n```"));
        assert!(restored.contains("```python\nprint('hi')\n```"));
        assert!(restored.contains("`inline1`"));
        assert!(restored.contains("`inline2`"));
        assert!(!restored.contains("__CODEBLOCK_"));
        assert!(!restored.contains("__INLINECODE_"));
    }

    #[test]
    fn test_hashmap_order_independence() {
        let config = Arc::new(TokenReductionConfig {
            preserve_code: true,
            ..Default::default()
        });
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "Text `a` and `b` and `c` here";
        let mut preserved = AHashMap::new();
        let result = extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 3);
        let restored = restore_preserved_blocks(&result, &preserved);

        assert!(restored.contains("`a`"));
        assert!(restored.contains("`b`"));
        assert!(restored.contains("`c`"));
        assert_eq!(restored, "Text `a` and `b` and `c` here");
    }

    #[test]
    fn test_preserve_patterns_regex() {
        let config = TokenReductionConfig {
            preserve_patterns: vec![
                r"\b[A-Z]{2,}\b".to_string(),
                r"\b\d+\.\d+\.\d+\b".to_string(),
                r"@\w+".to_string(),
            ],
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The NASA and HTTP protocols version 1.2.3 by @john";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(result.contains("NASA"));
        assert!(result.contains("HTTP"));
        assert!(result.contains("1.2.3"));
        assert!(result.contains("@john"));

        assert!(!result.contains(" the "));
        assert!(!result.contains(" and "));
        assert!(!result.contains(" by "));
    }

    #[test]
    fn test_language_specific_stopwords() {
        let config_en = Arc::new(TokenReductionConfig::default());
        let pipeline_en = FilterPipeline::new(&config_en, "en").unwrap();
        assert_eq!(pipeline_en.language(), "en");

        let input_en = "the quick brown fox";
        let result_en = remove_stopwords(input_en, &pipeline_en.stopwords, &pipeline_en.preserve_patterns);
        assert!(!result_en.contains(" the "));

        let config_de = Arc::new(TokenReductionConfig::default());
        let pipeline_de = FilterPipeline::new(&config_de, "de").unwrap();
        assert_eq!(pipeline_de.language(), "de");

        let input_de = "der schnelle braune fuchs";
        let result_de = remove_stopwords(input_de, &pipeline_de.stopwords, &pipeline_de.preserve_patterns);
        assert!(!result_de.contains(" der "));
        assert!(result_de.contains("schnelle"));
    }

    #[test]
    fn test_language_fallback_to_english() {
        let config = Arc::new(TokenReductionConfig::default());

        let pipeline = FilterPipeline::new(&config, "unsupported_lang").unwrap();
        assert_eq!(pipeline.language(), "unsupported_lang");

        let input = "the quick brown fox";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(!result.contains(" the "));
        assert!(result.contains("quick"));
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
    fn test_custom_stopwords_with_preserve_patterns() {
        use std::collections::HashMap;

        let mut custom_stopwords = HashMap::new();
        custom_stopwords.insert("en".to_string(), vec!["custom".to_string(), "stopword".to_string()]);

        let config = TokenReductionConfig {
            custom_stopwords: Some(custom_stopwords),
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "this is a custom stopword test";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(!result.contains(" custom "));
        assert!(!result.contains(" stopword "));
        assert!(!result.contains(" is "));
        assert!(!result.contains(" a "));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_preserve_patterns_empty() {
        let config = TokenReductionConfig {
            preserve_patterns: vec![],
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The quick brown fox";
        let result = remove_stopwords(input, &pipeline.stopwords, &pipeline.preserve_patterns);

        assert!(!result.contains(" The "));
        assert!(result.contains("quick"));
    }

    #[test]
    fn test_invalid_preserve_pattern() {
        let config = TokenReductionConfig {
            preserve_patterns: vec!["[invalid".to_string()],
            ..Default::default()
        };

        let config = Arc::new(config);
        let result = FilterPipeline::new(&config, "en");

        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                KreuzbergError::Validation { message, .. } => {
                    assert!(message.contains("Invalid regex pattern"));
                }
                _ => panic!("Expected ValidationError"),
            }
        }
    }
}
