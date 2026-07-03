//! Built-in Late-stage post-processor that redacts PII from the extraction
//! result.
//!
//! Activates when [`ExtractionConfig::redaction`](crate::core::config::ExtractionConfig::redaction)
//! is `Some(_)`. Runs the pure-Rust pattern engine, the optional NER backend
//! (for PERSON / ORG / LOCATION), and rewrites every textual field on the
//! result in place. Produces an audit trail in
//! [`ExtractedDocument::redaction_report`](crate::types::ExtractedDocument::redaction_report).

use std::sync::Arc;

use async_trait::async_trait;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{Plugin, PostProcessor, ProcessingStage, register_post_processor};
use crate::text::redaction::redact;
use crate::types::ExtractedDocument;

/// Redaction post-processor.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, Default)]
pub struct RedactionProcessor;

impl Plugin for RedactionProcessor {
    fn name(&self) -> &str {
        "redaction"
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
impl PostProcessor for RedactionProcessor {
    async fn process(&self, result: &mut ExtractedDocument, config: &ExtractionConfig) -> Result<()> {
        let Some(redaction_config) = config.redaction.as_ref() else {
            return Ok(());
        };

        tracing::info!(
            target: "xberg::redaction",
            strategy = ?redaction_config.strategy,
            ner_enabled = redaction_config.ner.is_some(),
            "running redaction pipeline"
        );

        redact(result, redaction_config).await
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Late
    }

    fn should_process(&self, _result: &ExtractedDocument, config: &ExtractionConfig) -> bool {
        config.redaction.is_some()
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Register the redaction post-processor with the global registry.
#[cfg_attr(alef, alef(skip))]
pub fn register() -> Result<()> {
    register_post_processor(Arc::new(RedactionProcessor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::redaction::RedactionConfig;
    use std::borrow::Cow;

    #[test]
    fn processor_metadata_is_correct() {
        let p = RedactionProcessor;
        assert_eq!(p.name(), "redaction");
        assert_eq!(p.processing_stage(), ProcessingStage::Late);
    }

    #[test]
    fn should_process_only_when_redaction_configured() {
        let p = RedactionProcessor;
        let result = ExtractedDocument {
            content: "hello".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };
        assert!(!p.should_process(&result, &ExtractionConfig::default()));

        let cfg = ExtractionConfig {
            redaction: Some(RedactionConfig::default()),
            ..Default::default()
        };
        assert!(p.should_process(&result, &cfg));
    }

    #[tokio::test]
    async fn redacts_email_in_content() {
        let p = RedactionProcessor;
        let mut result = ExtractedDocument {
            content: "Contact me at alice@example.com.".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };
        let cfg = ExtractionConfig {
            redaction: Some(RedactionConfig::default()),
            ..Default::default()
        };

        p.process(&mut result, &cfg).await.unwrap();
        assert!(result.content.contains("[REDACTED]"));
        assert!(!result.content.contains("alice@example.com"));
        assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 1);
    }
}
