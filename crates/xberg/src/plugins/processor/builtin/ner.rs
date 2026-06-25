//! Built-in Middle-stage post-processor that runs named-entity recognition.
//!
//! Activates when [`ExtractionConfig::ner`](crate::core::config::ExtractionConfig::ner)
//! is `Some(_)`. Resolves the backend declared in
//! [`NerConfig::backend`](crate::core::config::ner::NerConfig::backend) and
//! writes the detected entities into
//! [`ExtractionResult::entities`](crate::types::ExtractionResult::entities).

use std::sync::Arc;

use async_trait::async_trait;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::core::config::ner::{NerBackendKind, NerConfig};
use crate::plugins::{Plugin, PostProcessor, ProcessingStage, register_post_processor};
use crate::text::ner::NerBackend;
use crate::types::ExtractionResult;

/// NER post-processor.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, Default)]
pub struct NerProcessor;

impl Plugin for NerProcessor {
    fn name(&self) -> &str {
        "ner"
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
impl PostProcessor for NerProcessor {
    async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()> {
        let Some(ner_config) = config.ner.as_ref() else {
            return Ok(());
        };
        if result.content.is_empty() {
            return Ok(());
        }

        tracing::info!(
            target: "xberg::ner",
            backend = ?ner_config.backend,
            text_len = result.content.len(),
            "running NER backend"
        );

        let backend = make_backend(ner_config)?;
        let entities = backend
            .detect_with_custom(&result.content, &ner_config.categories, &ner_config.custom_labels)
            .await?;

        result.entities = Some(entities);
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }

    fn should_process(&self, _result: &ExtractionResult, config: &ExtractionConfig) -> bool {
        config.ner.is_some()
    }

    fn priority(&self) -> i32 {
        50
    }
}

fn make_backend(config: &NerConfig) -> Result<Arc<dyn NerBackend>> {
    match config.backend {
        NerBackendKind::Onnx => {
            #[cfg(feature = "ner-onnx")]
            {
                Ok(crate::text::ner::gline::get_or_init_backend(config.model.as_deref())?)
            }
            #[cfg(not(feature = "ner-onnx"))]
            {
                Err(crate::XbergError::MissingDependency(
                    "ner-onnx feature is not enabled — rebuild xberg with --features ner-onnx".to_string(),
                ))
            }
        }
        NerBackendKind::Llm => {
            #[cfg(all(feature = "ner-llm", not(all(target_os = "android", target_arch = "x86_64"))))]
            {
                let llm = config.llm.clone().ok_or_else(|| {
                    crate::XbergError::validation("Llm NER backend selected but NerConfig.llm is None".to_string())
                })?;
                Ok(Arc::new(crate::text::ner::llm::LlmBackend::new(llm)))
            }
            #[cfg(not(all(feature = "ner-llm", not(all(target_os = "android", target_arch = "x86_64")))))]
            {
                Err(crate::XbergError::MissingDependency(
                    "ner-llm feature is not enabled — rebuild xberg with --features ner-llm".to_string(),
                ))
            }
        }
    }
}

/// Register the default NER post-processor with the global registry.
#[cfg_attr(alef, alef(skip))]
pub fn register() -> Result<()> {
    register_post_processor(Arc::new(NerProcessor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn processor_metadata_is_correct() {
        let p = NerProcessor;
        assert_eq!(p.name(), "ner");
        assert_eq!(p.processing_stage(), ProcessingStage::Middle);
    }

    #[test]
    fn should_process_only_when_ner_configured() {
        let p = NerProcessor;
        let result = ExtractionResult {
            content: "Alice works at Acme".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };
        assert!(!p.should_process(&result, &ExtractionConfig::default()));

        let cfg = ExtractionConfig {
            ner: Some(NerConfig::default()),
            ..Default::default()
        };
        assert!(p.should_process(&result, &cfg));
    }
}
