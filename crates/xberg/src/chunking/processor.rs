//! Text chunking post-processor.
//!
//! This module provides a PostProcessor plugin that chunks text content in
//! extraction results.

use crate::chunking::config::{ChunkerType, ChunkingConfig};
use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
use crate::types::Metadata;
use crate::{ExtractionConfig, ExtractionResult, Result, XbergError};
use async_trait::async_trait;

/// Post-processor that chunks text in document content.
///
/// This processor:
/// - Runs in the Middle processing stage
/// - Only processes when `config.chunking` is configured
/// - Stores chunks in `result.chunks`
/// - Uses configurable chunk size and overlap
///
/// # Example
///
/// ```rust,no_run
/// use xberg::plugins::{Plugin, PostProcessor};
/// use xberg::chunking::processor::ChunkingProcessor;
///
/// let processor = ChunkingProcessor;
/// assert_eq!(processor.name(), "text-chunking");
/// ```
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy)]
pub struct ChunkingProcessor;

impl Plugin for ChunkingProcessor {
    fn name(&self) -> &str {
        "text-chunking"
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
impl PostProcessor for ChunkingProcessor {
    async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()> {
        let chunking_config = match &config.chunking {
            Some(cfg) => cfg,
            None => return Ok(()),
        };

        let inferred = maybe_infer_yaml_chunker(chunking_config, &result.metadata);
        let effective_config = inferred.as_ref().unwrap_or(chunking_config);

        let chunking_result = crate::chunking::chunk_text(&result.content, effective_config, None)
            .map_err(|e| XbergError::Other(format!("Chunking failed: {}", e)))?;
        result.chunks = Some(chunking_result.chunks);

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }

    fn should_process(&self, _result: &ExtractionResult, config: &ExtractionConfig) -> bool {
        config.chunking.is_some()
    }

    fn estimated_duration_ms(&self, result: &ExtractionResult) -> u64 {
        let text_length = result.content.len();
        (text_length / 10240).max(1) as u64
    }
}

/// Returns an overridden config if auto-inference applies, or None to use the original.
///
/// If the user left the chunker type at default (`Text`) and the document metadata
/// indicates a YAML or JSON data format, returns a config with `Yaml` chunker type.
/// An explicit non-default choice by the user is never overridden.
fn maybe_infer_yaml_chunker(config: &ChunkingConfig, metadata: &Metadata) -> Option<ChunkingConfig> {
    if config.chunker_type != ChunkerType::Text {
        return None;
    }

    let is_structured = metadata
        .additional
        .get("data_format")
        .and_then(|v| v.as_str())
        .is_some_and(|fmt| fmt == "yaml" || fmt == "json");

    is_structured.then(|| ChunkingConfig {
        chunker_type: ChunkerType::Yaml,
        ..config.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ChunkingConfig;
    use crate::types::Metadata;
    use std::borrow::Cow;

    #[tokio::test]
    async fn test_chunking_processor() {
        let processor = ChunkingProcessor;
        let config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                max_characters: 100,
                overlap: 10,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };

        let mut result = ExtractionResult {
            content: "This is a longer text that should be split into multiple chunks to test the chunking processor functionality.".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        assert!(result.chunks.is_some());
        let chunks = result.chunks.unwrap();
        assert!(!chunks.is_empty());
    }

    #[tokio::test]
    async fn test_chunking_processor_no_config() {
        let processor = ChunkingProcessor;
        let config = ExtractionConfig::default();

        let mut result = ExtractionResult {
            content: "Some text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        assert!(result.chunks.is_none());
    }

    #[test]
    fn test_chunking_processor_plugin_interface() {
        let processor = ChunkingProcessor;
        assert_eq!(processor.name(), "text-chunking");
        assert!(!processor.version().is_empty());
        assert!(processor.initialize().is_ok());
        assert!(processor.shutdown().is_ok());
    }

    #[test]
    fn test_chunking_processor_stage() {
        let processor = ChunkingProcessor;
        assert_eq!(processor.processing_stage(), ProcessingStage::Middle);
    }

    #[test]
    fn test_chunking_processor_should_process() {
        let processor = ChunkingProcessor;

        let result = ExtractionResult {
            content: "Sample text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let config_with_chunking = ExtractionConfig {
            chunking: Some(crate::core::config::ChunkingConfig {
                max_characters: 100,
                overlap: 10,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(processor.should_process(&result, &config_with_chunking));

        let config_without_chunking = ExtractionConfig::default();
        assert!(!processor.should_process(&result, &config_without_chunking));
    }

    #[test]
    fn test_chunking_processor_estimated_duration() {
        let processor = ChunkingProcessor;

        let short_result = ExtractionResult {
            content: "Short".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let long_result = ExtractionResult {
            content: "a".repeat(100000),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let short_duration = processor.estimated_duration_ms(&short_result);
        let long_duration = processor.estimated_duration_ms(&long_result);

        assert!(long_duration > short_duration);
    }

    fn make_metadata_with_format(format: &str) -> Metadata {
        let mut metadata = Metadata::default();
        metadata
            .additional
            .insert(Cow::Borrowed("data_format"), serde_json::json!(format));
        metadata
    }

    #[tokio::test]
    async fn test_auto_infer_yaml_from_metadata() {
        let processor = ChunkingProcessor;
        let config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                max_characters: 10000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };

        let yaml_content = "server:\n  host: localhost\n  port: 8080";
        let mut result = ExtractionResult {
            content: yaml_content.to_string(),
            mime_type: Cow::Borrowed("text/yaml"),
            metadata: make_metadata_with_format("yaml"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();
        let chunks = result.chunks.unwrap();
        // Yaml chunker produces section-prefixed chunks
        assert!(chunks[0].content.contains("# server > host"));
    }

    #[tokio::test]
    async fn test_auto_infer_json_from_metadata() {
        let processor = ChunkingProcessor;
        let config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                max_characters: 10000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };

        let json_content = r#"{"name": "test", "version": "1.0"}"#;
        let mut result = ExtractionResult {
            content: json_content.to_string(),
            mime_type: Cow::Borrowed("application/json"),
            metadata: make_metadata_with_format("json"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();
        let chunks = result.chunks.unwrap();
        // JSON chunker produces section-prefixed chunks
        assert!(chunks[0].content.contains("# name"));
    }

    #[tokio::test]
    async fn test_explicit_type_not_overridden() {
        let processor = ChunkingProcessor;
        let config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                max_characters: 10000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Markdown,
                ..Default::default()
            }),
            ..Default::default()
        };

        let yaml_content = "server:\n  host: localhost\n  port: 8080";
        let mut result = ExtractionResult {
            content: yaml_content.to_string(),
            mime_type: Cow::Borrowed("text/yaml"),
            metadata: make_metadata_with_format("yaml"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();
        let chunks = result.chunks.unwrap();
        // Markdown chunker does NOT produce "# server > host" section headers
        assert!(!chunks[0].content.contains("# server > host"));
    }

    #[tokio::test]
    async fn test_missing_data_format_no_inference() {
        let processor = ChunkingProcessor;
        let config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                max_characters: 10000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };

        let yaml_content = "server:\n  host: localhost\n  port: 8080";
        let mut result = ExtractionResult {
            content: yaml_content.to_string(),
            mime_type: Cow::Borrowed("text/yaml"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();
        let chunks = result.chunks.unwrap();
        // Without data_format metadata, should NOT auto-infer yaml chunking
        assert!(!chunks[0].content.contains("# server > host"));
    }
}
