//! Built-in Middle-stage post-processor that drives per-chunk multi-label LLM
//! classification.
//!
//! Activates when
//! [`ExtractionConfig::chunk_classification`](crate::core::config::ExtractionConfig::chunk_classification)
//! is `Some`. Delegates the heavy lifting to
//! [`crate::text::classification::classify_chunks`]. Runs at the Middle stage,
//! after chunking has populated `ExtractedDocument::chunks`
//! (see [`crate::core::pipeline::run_pipeline`]).

use std::sync::Arc;

use async_trait::async_trait;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{Plugin, PostProcessor, ProcessingStage, register_post_processor};
use crate::types::ExtractedDocument;

/// Post-processor that asks an LLM to assign multi-label classifications to
/// each chunk of the extracted content.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, Default)]
pub struct ChunkClassificationProcessor;

impl Plugin for ChunkClassificationProcessor {
    fn name(&self) -> &str {
        "chunk-classification"
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

#[async_trait]
impl PostProcessor for ChunkClassificationProcessor {
    async fn process(&self, result: &mut ExtractedDocument, config: &ExtractionConfig) -> Result<()> {
        if let Some(cc_config) = config.chunk_classification.as_ref() {
            tracing::info!(
                target: "xberg::classification",
                definitions = cc_config.definitions.len(),
                batch_size = cc_config.batch_size,
                max_concurrency = cc_config.max_concurrency,
                model = %cc_config.llm.model,
                "running per-chunk classification"
            );
            crate::text::classification::classify_chunks(result, cc_config).await?;
        }
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }

    fn should_process(&self, _result: &ExtractedDocument, config: &ExtractionConfig) -> bool {
        config.chunk_classification.is_some()
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Register the default chunk-classification post-processor with the global registry.
#[cfg_attr(alef, alef(skip))]
pub fn register() -> Result<()> {
    register_post_processor(Arc::new(ChunkClassificationProcessor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{ChunkClassificationConfig, ChunkClassificationDefinition, LlmConfig};
    use std::borrow::Cow;

    fn config_with_classification() -> ExtractionConfig {
        ExtractionConfig {
            chunk_classification: Some(ChunkClassificationConfig {
                prompt_template: None,
                definitions: vec![ChunkClassificationDefinition {
                    label: "director_appointment".to_string(),
                    description: "A director is appointed.".to_string(),
                }],
                llm: LlmConfig {
                    model: "openai/gpt-4o-mini".to_string(),
                    ..Default::default()
                },
                batch_size: 10,
                max_concurrency: 4,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn processor_metadata_is_correct() {
        let p = ChunkClassificationProcessor;
        assert_eq!(p.name(), "chunk-classification");
        assert_eq!(p.processing_stage(), ProcessingStage::Middle);
    }

    #[test]
    fn should_process_only_when_config_present() {
        let p = ChunkClassificationProcessor;
        let result = ExtractedDocument {
            content: "x".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };
        assert!(!p.should_process(&result, &ExtractionConfig::default()));
        assert!(p.should_process(&result, &config_with_classification()));
    }
}
