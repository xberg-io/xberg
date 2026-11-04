//! Observed processors cannot erase an untrusted HTML rendering's privacy obligation.

use super::*;
use crate::core::config::OutputFormat;
use crate::core::config::redaction::{RedactionConfig, RedactionTerm};
use crate::extraction::derive::surfaces::{DerivedResult, derive_with_surfaces};
use crate::extractors::SyncExtractor;
use crate::extractors::html::HtmlExtractor;
use crate::plugins::{Plugin, PostProcessor};
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};

#[derive(Clone, Copy)]
enum Edit {
    AppendContent,
    ReplaceFormatted,
    RemoveFormatted,
}

#[derive(Clone)]
struct Receipt {
    before_plain: String,
    before_formatted: Option<String>,
    after_serialized: serde_json::Value,
    after_plain: String,
    after_formatted: Option<String>,
}

type Receipts = Arc<Mutex<Vec<Receipt>>>;

struct Mutate {
    edit: Edit,
    receipts: Receipts,
}

impl Plugin for Mutate {
    fn name(&self) -> &str {
        "untrusted-html-observed-mutation"
    }
}

#[async_trait]
impl PostProcessor for Mutate {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        let before_plain = result.content.clone();
        let before_formatted = result.formatted_content.clone();
        match self.edit {
            Edit::AppendContent => result.content.push_str("\nappended"),
            Edit::ReplaceFormatted => result.formatted_content = Some("Ja**neDoe** tail\n".into()),
            Edit::RemoveFormatted => result.formatted_content = None,
        }
        self.receipts.lock().push(Receipt {
            before_plain,
            before_formatted,
            after_serialized: serde_json::to_value(&*result).unwrap(),
            after_plain: result.content.clone(),
            after_formatted: result.formatted_content.clone(),
        });
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        redaction: Some(RedactionConfig {
            custom_terms: vec![RedactionTerm::labeled("private", "JaneDoe")],
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn actual_html(config: &ExtractionConfig) -> DerivedResult {
    let source = HtmlExtractor
        .extract_sync(b"<p>Jane<strong>Doe</strong> tail</p>", "text/html", config)
        .unwrap();
    assert!(source.elements.iter().any(|element| element.text == "JaneDoe tail"));
    assert!(
        source
            .pre_rendered_content
            .as_deref()
            .unwrap()
            .contains("Jane**Doe** tail")
    );
    let result = derive_with_surfaces(source, false, OutputFormat::Markdown, true);
    assert_eq!(result.content, "JaneDoe tail");
    assert!(result.pages.is_none());
    assert!(result.redaction_report.is_none());
    result
}

async fn mutation_is_rejected(edit: Edit) {
    let config = config();
    let mut result = actual_html(&config);
    let original_formatted = result.formatted_content.clone();
    let receipts = Arc::new(Mutex::new(Vec::new()));
    let mut registry = crate::plugins::registry::PostProcessorRegistry::new();
    assert!(registry.register_builtin_redaction().unwrap());
    registry
        .register(Arc::new(Mutate {
            edit,
            receipts: Arc::clone(&receipts),
        }))
        .unwrap();
    let cache = super::super::cache::ProcessorCache::from_registry(&Arc::new(RwLock::new(registry)), 0).unwrap();
    assert_eq!(cache.middle.len(), 1);
    assert_eq!(cache.late.len(), 1);
    assert!(Arc::ptr_eq(&cache.late[0], cache.builtin_redaction.as_ref().unwrap()));
    let outcome = execute_processor_stages(
        &mut result,
        &config,
        &None,
        &[
            (ProcessingStage::Middle, Arc::clone(&cache.middle)),
            (ProcessingStage::Late, Arc::clone(&cache.late)),
        ],
        cache.builtin_redaction.as_ref(),
    )
    .await;
    assert_eq!(
        receipts.lock().len(),
        1,
        "actual registered Middle callback must run exactly once"
    );
    let receipt = receipts.lock()[0].clone();
    assert_eq!(receipt.before_plain, "JaneDoe tail");
    assert_eq!(receipt.before_formatted, original_formatted);
    match edit {
        Edit::AppendContent => {
            assert_eq!(receipt.after_plain, "JaneDoe tail\nappended");
            assert_eq!(receipt.after_formatted, original_formatted);
        }
        Edit::ReplaceFormatted => {
            assert_eq!(receipt.after_plain, "JaneDoe tail");
            assert_eq!(receipt.after_formatted.as_deref(), Some("Ja**neDoe** tail\n"));
        }
        Edit::RemoveFormatted => {
            assert_eq!(receipt.after_plain, "JaneDoe tail");
            assert!(receipt.after_formatted.is_none());
        }
    }
    outcome.expect_err("observed edits must retain the original untrusted privacy obligation");
    assert_eq!(serde_json::to_value(&*result).unwrap(), receipt.after_serialized);
    assert_eq!(result.formatted_content, receipt.after_formatted);
    assert!(result.redaction_report.is_none());
}

#[tokio::test]
async fn observed_html_content_append_cannot_erase_the_untrusted_obligation() {
    mutation_is_rejected(Edit::AppendContent).await;
}

#[tokio::test]
async fn observed_html_formatted_replacement_cannot_become_raw_literal_authority() {
    mutation_is_rejected(Edit::ReplaceFormatted).await;
}

#[tokio::test]
async fn observed_html_formatted_removal_cannot_erase_the_untrusted_obligation() {
    mutation_is_rejected(Edit::RemoveFormatted).await;
}
