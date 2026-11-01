use super::*;
use crate::core::config::OutputFormat;
use crate::core::config::redaction::{RedactionConfig, RedactionTerm};
use crate::extraction::derive::surfaces::DerivedResult;
use crate::plugins::{Plugin, PostProcessor};
use crate::types::builder::superscript;
use crate::types::internal_builder::InternalDocumentBuilder;
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};

#[derive(Clone, Copy)]
enum Action {
    Record,
    Foreign,
    Soft,
    Fatal,
    Rewrite,
}

struct Probe {
    name: &'static str,
    priority: i32,
    enabled: bool,
    action: Action,
    log: Arc<Mutex<Vec<(String, String)>>>,
}

impl Plugin for Probe {
    fn name(&self) -> &str {
        self.name
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PostProcessor for Probe {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        self.log.lock().push((self.name.into(), result.content.clone()));
        match self.action {
            Action::Record => Ok(()),
            Action::Foreign => {
                result.content = "foreign handled".into();
                Ok(())
            }
            Action::Soft => Err(XbergError::validation("soft failure")),
            Action::Fatal => Err(XbergError::Plugin {
                plugin_name: self.name.into(),
                message: "fatal failure".into(),
            }),
            Action::Rewrite => {
                result.formatted_content = Some("unproved <u>literal</u>".into());
                Ok(())
            }
        }
    }
    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Late
    }
    fn priority(&self) -> i32 {
        self.priority
    }
    fn should_process(&self, _result: &ExtractedDocument, _config: &ExtractionConfig) -> bool {
        self.enabled
    }
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        redaction: Some(RedactionConfig {
            custom_terms: vec![RedactionTerm::labeled("private", "secret")],
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn document() -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("secret", vec![superscript(0, 6)], Some(1), None);
    crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Markdown, true)
}

fn registry() -> Arc<RwLock<crate::plugins::registry::PostProcessorRegistry>> {
    let mut registry = crate::plugins::registry::PostProcessorRegistry::new();
    assert!(registry.register_builtin_redaction().unwrap());
    Arc::new(RwLock::new(registry))
}

fn probe(
    name: &'static str,
    priority: i32,
    enabled: bool,
    action: Action,
    log: &Arc<Mutex<Vec<(String, String)>>>,
) -> Arc<dyn PostProcessor> {
    Arc::new(Probe {
        name,
        priority,
        enabled,
        action,
        log: Arc::clone(log),
    })
}

async fn execute(
    registry: &Arc<RwLock<crate::plugins::registry::PostProcessorRegistry>>,
    result: &mut DerivedResult,
) -> Result<()> {
    let cache = super::super::cache::ProcessorCache::from_registry(registry, 0).unwrap();
    assert!(!cache.late.is_empty());
    execute_processor_stages(
        result,
        &config(),
        &None,
        &[(ProcessingStage::Late, Arc::clone(&cache.late))],
        cache.builtin_redaction.as_ref(),
    )
    .await
}

#[tokio::test]
async fn foreign_same_name_replacement_does_not_inherit_builtin_dispatch() {
    let registry = registry();
    let old = super::super::cache::ProcessorCache::from_registry(&registry, 0).unwrap();
    let builtin = old.builtin_redaction.as_ref().unwrap();
    assert!(old.late.iter().any(|processor| Arc::ptr_eq(processor, builtin)));
    let log = Arc::new(Mutex::new(Vec::new()));
    registry
        .write()
        .register(probe("redaction", builtin.priority(), true, Action::Foreign, &log))
        .unwrap();
    assert!(registry.read().builtin_redaction().is_none());
    assert!(
        !registry.write().register_builtin_redaction().unwrap(),
        "a present foreign plugin must not be marked builtin"
    );
    let new = super::super::cache::ProcessorCache::from_registry(&registry, 0).unwrap();
    assert!(new.builtin_redaction.is_none());
    assert_ne!(old.generation, new.generation);
    let mut result = document();
    execute(&registry, &mut result).await.unwrap();
    assert_eq!(result.content, "foreign handled");
    assert_eq!(log.lock().as_slice(), [("redaction".into(), "secret".into())]);
}

#[test]
fn remove_clear_and_shutdown_retire_actual_builtin_identity() {
    for operation in ["remove", "clear", "shutdown"] {
        let registry = registry();
        let generation = registry.read().generation();
        match operation {
            "remove" => registry.write().remove("redaction").unwrap(),
            "clear" => registry.write().clear().unwrap(),
            _ => registry.write().shutdown_all().unwrap(),
        }
        assert!(registry.read().builtin_redaction().is_none(), "{operation}");
        assert!(registry.read().list().is_empty());
        assert!(registry.read().generation() > generation);
    }
}

#[tokio::test]
async fn contextual_builtin_keeps_priority_order_and_should_process() {
    let registry = registry();
    let priority = registry.read().builtin_redaction().unwrap().priority();
    let log = Arc::new(Mutex::new(Vec::new()));
    for (name, priority, enabled) in [
        ("before", priority + 1, true),
        ("after", priority - 1, true),
        ("disabled", priority + 2, false),
    ] {
        registry
            .write()
            .register(probe(name, priority, enabled, Action::Record, &log))
            .unwrap();
    }
    let mut result = document();
    execute(&registry, &mut result).await.unwrap();
    assert_eq!(
        log.lock().as_slice(),
        [
            ("before".into(), "secret".into()),
            ("after".into(), "[REDACTED]".into())
        ]
    );
    assert_eq!(result.formatted_content.as_deref(), Some("<sup>[REDACTED]</sup>\n"));
}

#[tokio::test]
async fn contextual_executor_preserves_soft_and_fatal_error_policy() {
    for action in [Action::Soft, Action::Fatal] {
        let registry = registry();
        let priority = registry.read().builtin_redaction().unwrap().priority();
        let log = Arc::new(Mutex::new(Vec::new()));
        registry
            .write()
            .register(probe("failure", priority + 1, true, action, &log))
            .unwrap();
        let mut result = document();
        let outcome = execute(&registry, &mut result).await;
        assert_eq!(log.lock().len(), 1);
        if matches!(action, Action::Fatal) {
            assert!(matches!(outcome, Err(XbergError::Plugin { .. })));
            assert_eq!(result.content, "secret");
        } else {
            outcome.unwrap();
            assert_eq!(result.content, "[REDACTED]");
            assert!(
                result
                    .processing_warnings
                    .iter()
                    .any(|warning| warning.source == "failure")
            );
        }
    }
}

#[tokio::test]
async fn earlier_processor_cannot_replace_a_verified_formatted_preimage() {
    let registry = registry();
    let priority = registry.read().builtin_redaction().unwrap().priority();
    let log = Arc::new(Mutex::new(Vec::new()));
    registry
        .write()
        .register(probe("rewrite", priority + 1, true, Action::Rewrite, &log))
        .unwrap();
    let mut result = document();
    let error = execute(&registry, &mut result).await.unwrap_err();
    assert!(matches!(error, XbergError::Plugin { .. }));
    assert_eq!(result.content, "secret", "no partially redacted result may be returned");
    assert!(result.redaction_report.is_none());
}

#[tokio::test]
async fn direct_public_processor_calls_keep_ordinary_data_semantics() {
    let mut result = ExtractedDocument::default();
    result.content = "<sup>literal</sup>".into();
    result.formatted_content = Some(result.content.clone());
    let config = ExtractionConfig {
        redaction: Some(RedactionConfig {
            custom_terms: vec![RedactionTerm::labeled("private", "sup")],
            ..Default::default()
        }),
        ..Default::default()
    };
    crate::plugins::processor::builtin::redaction::RedactionProcessor
        .process(&mut result, &config)
        .await
        .unwrap();
    assert_eq!(result.content, "<[REDACTED]>literal</[REDACTED]>");
    assert_eq!(
        result.formatted_content.as_deref(),
        Some("<[REDACTED]>literal</[REDACTED]>")
    );
}

#[tokio::test]
async fn explicit_builtin_registration_replaces_foreign_and_binds_actual_handle() {
    let registry = registry();
    let log = Arc::new(Mutex::new(Vec::new()));
    registry
        .write()
        .register(probe("redaction", 50, true, Action::Foreign, &log))
        .unwrap();
    assert!(registry.read().builtin_redaction().is_none());
    registry.write().register_redaction_processor().unwrap();
    let cache = super::super::cache::ProcessorCache::from_registry(&registry, 0).unwrap();
    assert!(
        cache
            .late
            .iter()
            .any(|processor| Arc::ptr_eq(processor, cache.builtin_redaction.as_ref().unwrap()))
    );
    let mut result = document();
    execute(&registry, &mut result).await.unwrap();
    assert!(log.lock().is_empty());
    assert_eq!(result.content, "[REDACTED]");
    assert_eq!(result.formatted_content.as_deref(), Some("<sup>[REDACTED]</sup>\n"));
}
