use super::*;
use crate::core::config::OutputFormat;
use crate::core::config::redaction::{RedactionConfig, RedactionTerm};
use crate::extraction::derive::surfaces::DerivedResult;
use crate::plugins::{Plugin, PostProcessor};
use crate::types::builder::superscript;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::redaction::RedactionStrategy;
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};

type Calls = Arc<Mutex<Vec<(&'static str, String, String)>>>;

struct Mutation {
    name: &'static str,
    priority: i32,
    suffix: Option<&'static str>,
    remove_formatted: bool,
    calls: Calls,
}

impl Plugin for Mutation {
    fn name(&self) -> &str {
        self.name
    }
}

#[async_trait]
impl PostProcessor for Mutation {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        let before = result.content.clone();
        if let Some(suffix) = self.suffix {
            result.content.push_str(suffix);
            if let Some(formatted) = &mut result.formatted_content {
                formatted.push_str(suffix);
            }
        }
        if self.remove_formatted {
            result.formatted_content = None;
        }
        self.calls.lock().push((self.name, before, result.content.clone()));
        Ok(())
    }
    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
    fn priority(&self) -> i32 {
        self.priority
    }
}

fn document() -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("secret", vec![superscript(0, 6)], Some(1), None);
    let result = crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Markdown, true);
    assert_eq!(result.content, "secret");
    assert_eq!(result.formatted_content.as_deref(), Some("<sup>secret</sup>\n"));
    let pages = result.pages.as_ref().expect("actual derived page");
    assert_eq!(pages.len(), 1);
    assert_eq!(
        (pages[0].page_number, pages[0].content.as_str()),
        (1, "<sup>secret</sup>\n")
    );
    result
}

async fn run(
    result: &mut DerivedResult,
    mutations: &[(&'static str, Option<&'static str>, bool)],
    calls: &Calls,
) -> Result<()> {
    let mut registry = crate::plugins::registry::PostProcessorRegistry::new();
    assert!(registry.register_builtin_redaction().unwrap());
    for (index, &(name, suffix, remove_formatted)) in mutations.iter().enumerate() {
        registry
            .register(Arc::new(Mutation {
                name,
                priority: 100 - i32::try_from(index).unwrap(),
                suffix,
                remove_formatted,
                calls: Arc::clone(calls),
            }))
            .unwrap();
    }
    let cache = super::super::cache::ProcessorCache::from_registry(&Arc::new(RwLock::new(registry)), 0).unwrap();
    assert_eq!(cache.middle.len(), mutations.len());
    assert_eq!(cache.late.len(), 1);
    let builtin = cache.builtin_redaction.as_ref().expect("actual builtin handle");
    assert!(Arc::ptr_eq(&cache.late[0], builtin));
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        redaction: Some(RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("private", "secret")],
            ..Default::default()
        }),
        ..Default::default()
    };
    execute_processor_stages(
        result,
        &config,
        &None,
        &[
            (ProcessingStage::Middle, Arc::clone(&cache.middle)),
            (ProcessingStage::Late, Arc::clone(&cache.late)),
        ],
        cache.builtin_redaction.as_ref(),
    )
    .await
}

fn assert_plain_output(result: DerivedResult, expected: &str) {
    assert!(result.formatted_content.is_none());
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "output_format")
    );
    let page = &result.pages.as_ref().unwrap()[0];
    assert_eq!(
        (page.page_number, page.content.as_str()),
        (1, "<sup>[REDACTED]</sup>\n")
    );
    let (document, format) = result.into_output(OutputFormat::Markdown);
    assert_eq!(
        format,
        OutputFormat::Plain,
        "discard must retain the original Markdown routing marker"
    );
    let output = super::super::apply_output_format(document, format);
    assert_eq!(output.metadata.output_format.as_deref(), Some("plain"));
    assert_eq!(output.content, expected);
}

#[tokio::test]
async fn should_chain_legitimate_middle_edits_without_noop_recertifying_markdown() {
    let mut result = document();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let outcome = run(
        &mut result,
        &[
            ("first", Some(" first secret"), false),
            ("second", Some(" second secret"), false),
            ("noop", None, false),
        ],
        &calls,
    )
    .await;
    assert_eq!(
        calls.lock().as_slice(),
        [
            ("first", "secret".into(), "secret first secret".into()),
            (
                "second",
                "secret first secret".into(),
                "secret first secret second secret".into()
            ),
            (
                "noop",
                "secret first secret second secret".into(),
                "secret first secret second secret".into()
            )
        ]
    );
    outcome.expect("validated actual Middle edits must chain before redaction");
    assert_plain_output(result, "[REDACTED] first [REDACTED] second [REDACTED]");
}

#[tokio::test]
async fn should_keep_warned_plain_routing_when_a_middle_edit_removes_formatted_content() {
    let mut result = document();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let outcome = run(&mut result, &[("remove", Some(" added secret"), true)], &calls).await;
    assert_eq!(
        calls.lock().as_slice(),
        [("remove", "secret".into(), "secret added secret".into())]
    );
    assert!(
        result.formatted_content.is_none(),
        "actual removal persists before the final outcome assertion"
    );
    outcome.expect("legitimate removal must remain processable with its original output-kind marker");
    assert_plain_output(result, "[REDACTED] added [REDACTED]");
}
