//! Keyword extraction post-processor.
//!
//! This module provides a PostProcessor plugin that extracts keywords from
//! extraction results and stores them in metadata.

use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
use crate::{ExtractionConfig, ExtractionResult, Result, XbergError};
use async_trait::async_trait;

/// Post-processor that extracts keywords from document content.
///
/// This processor:
/// - Runs in the Middle processing stage
/// - Only processes when `config.keywords` is configured
/// - Stores extracted keywords in `metadata.additional["keywords"]`
/// - Uses the configured algorithm (YAKE or RAKE)
///
/// # Example
///
/// ```rust,no_run
/// use xberg::plugins::{Plugin, PostProcessor};
/// use xberg::keywords::processor::KeywordExtractor;
///
/// let processor = KeywordExtractor;
/// assert_eq!(processor.name(), "keyword-extraction");
/// ```
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy)]
pub struct KeywordExtractor;

impl Plugin for KeywordExtractor {
    fn name(&self) -> &str {
        "keyword-extraction"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PostProcessor for KeywordExtractor {
    async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()> {
        let keyword_config = match &config.keywords {
            Some(cfg) => cfg,
            None => return Ok(()),
        };

        let word_count = result.content.split_whitespace().count();
        if word_count < 10 {
            return Ok(());
        }

        let keywords = super::extract_keywords(&result.content, keyword_config)
            .map_err(|e| XbergError::Other(format!("Keyword extraction failed: {}", e)))?;

        result.extracted_keywords = Some(keywords);

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }

    fn should_process(&self, _result: &ExtractionResult, config: &ExtractionConfig) -> bool {
        config.keywords.is_some()
    }

    fn estimated_duration_ms(&self, result: &ExtractionResult) -> u64 {
        let word_count = result.content.split_whitespace().count();
        (word_count as u64) / 100 + 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keywords::KeywordConfig;
    use std::borrow::Cow;

    const TEST_TEXT: &str = r#"
Machine learning is a branch of artificial intelligence that focuses on
building systems that can learn from data. Deep learning is a subset of
machine learning that uses neural networks with multiple layers.
    "#;

    #[tokio::test]
    #[cfg(feature = "keywords-yake")]
    async fn test_keyword_processor_with_yake() {
        let processor = KeywordExtractor;
        let config = ExtractionConfig {
            keywords: Some(KeywordConfig::yake()),
            ..Default::default()
        };

        let mut result = ExtractionResult {
            content: TEST_TEXT.to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        let keywords = result.extracted_keywords.as_ref().expect("keywords should be set");
        assert!(!keywords.is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "keywords-rake")]
    async fn test_keyword_processor_with_rake() {
        let processor = KeywordExtractor;
        let config = ExtractionConfig {
            keywords: Some(KeywordConfig::rake()),
            ..Default::default()
        };

        let mut result = ExtractionResult {
            content: TEST_TEXT.to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        let keywords = result.extracted_keywords.as_ref().expect("keywords should be set");
        assert!(!keywords.is_empty());
    }

    #[tokio::test]
    async fn test_keyword_processor_no_config() {
        let processor = KeywordExtractor;
        let config = ExtractionConfig::default();

        let mut result = ExtractionResult {
            content: TEST_TEXT.to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        assert!(result.extracted_keywords.is_none());
    }

    #[tokio::test]
    #[cfg(feature = "keywords-yake")]
    async fn test_keyword_processor_short_content() {
        let processor = KeywordExtractor;
        let config = ExtractionConfig {
            keywords: Some(KeywordConfig::yake()),
            ..Default::default()
        };

        let mut result = ExtractionResult {
            content: "Short text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        assert!(result.extracted_keywords.is_none());
    }

    #[test]
    fn test_keyword_processor_plugin_interface() {
        let processor = KeywordExtractor;
        assert_eq!(processor.name(), "keyword-extraction");
        assert!(!processor.version().is_empty());
        assert!(processor.initialize().is_ok());
        assert!(processor.shutdown().is_ok());
    }

    #[test]
    fn test_keyword_processor_stage() {
        let processor = KeywordExtractor;
        assert_eq!(processor.processing_stage(), ProcessingStage::Middle);
    }

    #[test]
    #[cfg(feature = "keywords-yake")]
    fn test_keyword_processor_should_process() {
        let processor = KeywordExtractor;

        let result = ExtractionResult {
            content: TEST_TEXT.to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let config_with_keywords = ExtractionConfig {
            keywords: Some(KeywordConfig::yake()),
            ..Default::default()
        };
        assert!(processor.should_process(&result, &config_with_keywords));

        let config_without_keywords = ExtractionConfig::default();
        assert!(!processor.should_process(&result, &config_without_keywords));
    }

    #[test]
    fn test_keyword_processor_estimated_duration() {
        let processor = KeywordExtractor;

        let short_result = ExtractionResult {
            content: "Short text with just a few words".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let long_result = ExtractionResult {
            content: "word ".repeat(1000),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let short_duration = processor.estimated_duration_ms(&short_result);
        let long_duration = processor.estimated_duration_ms(&long_result);

        assert!(long_duration > short_duration);
    }
}
