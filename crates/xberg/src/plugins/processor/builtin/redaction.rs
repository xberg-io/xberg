//! Built-in Late-stage post-processor that redacts PII from the extraction
//! result.
//!
//! Activates when [`ExtractionConfig::redaction`](crate::core::config::ExtractionConfig::redaction)
//! is `Some(_)`. Runs the pure-Rust pattern engine, the optional NER backend
//! (for PERSON / ORG / LOCATION), and rewrites every textual field on the
//! result in place. Produces an audit trail in
//! [`ExtractedDocument::redaction_report`](crate::types::ExtractedDocument::redaction_report).

use std::borrow::Cow;

use async_trait::async_trait;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
use crate::text::redaction::engine::{redact_with_external_findings, redact_with_security_limits};
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

fn selected_redaction_config(
    config: &ExtractionConfig,
    external: bool,
) -> Option<Cow<'_, crate::core::config::redaction::RedactionConfig>> {
    config
        .redaction
        .as_ref()
        .map(Cow::Borrowed)
        .or_else(|| external.then(|| Cow::Owned(crate::core::config::redaction::RedactionConfig::default())))
}

impl RedactionProcessor {
    pub(crate) async fn process_with_surfaces(
        &self,
        result: &mut ExtractedDocument,
        config: &ExtractionConfig,
        surfaces: &impl crate::extraction::derive::surfaces::RedactionSurfaces,
    ) -> Result<()> {
        let external = crate::text::redaction::external::current_external_redaction();
        let Some(redaction_config) = selected_redaction_config(config, external.is_some()) else {
            return Ok(());
        };

        tracing::info!(
            target: "xberg::redaction",
            strategy = ?redaction_config.strategy,
            ner_enabled = redaction_config.ner.is_some(),
            "running redaction pipeline"
        );

        let limits = config
            .security_limits
            .as_ref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(crate::extractors::security::SecurityLimits::default()));

        let outcome = match external.as_ref() {
            Some(request) => redact_with_external_findings(result, &redaction_config, request, &limits, surfaces).await,
            None => redact_with_security_limits(result, &redaction_config, &limits, surfaces).await,
        };
        // Redaction is security processing: every failure must abort instead of becoming a warning. ~keep
        let outcome = outcome.map_err(|err| match err {
            fatal @ (crate::XbergError::Io(_)
            | crate::XbergError::LockPoisoned(_)
            | crate::XbergError::Plugin { .. }) => fatal,
            other => crate::XbergError::Plugin {
                message: other.to_string(),
                plugin_name: self.name().to_string(),
            },
        });
        if outcome.is_ok()
            && let Some(request) = external
        {
            request.mark_consumed();
        }
        outcome
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PostProcessor for RedactionProcessor {
    async fn process(&self, result: &mut ExtractedDocument, config: &ExtractionConfig) -> Result<()> {
        self.process_with_surfaces(result, config, &()).await
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Late
    }

    fn should_process(&self, _result: &ExtractedDocument, config: &ExtractionConfig) -> bool {
        config.redaction.is_some() || crate::text::redaction::external::external_redaction_is_scoped()
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Register the redaction post-processor with the global registry.
#[cfg_attr(alef, alef(skip))]
pub fn register() -> Result<()> {
    crate::plugins::processor::register_redaction_processor()
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
