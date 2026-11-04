//! HTML rendering obligations cannot advance through processor mutations.

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
    RestoreContent,
    Noop,
    LateInjectHtml,
    LateNoop,
    LateInjectArchiveHtml,
    LateInjectOcrHtml,
    ReplaceFormatted,
    RemoveFormatted,
    ReplacePage,
    ReplaceNative,
}

impl Edit {
    fn is_late(self) -> bool {
        matches!(
            self,
            Self::LateInjectHtml | Self::LateNoop | Self::LateInjectArchiveHtml | Self::LateInjectOcrHtml
        )
    }
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
        match self.edit {
            Edit::RestoreContent => "html-surface-restore",
            _ => "html-surface-observed-mutation",
        }
    }
}

#[async_trait]
impl PostProcessor for Mutate {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        let before_plain = result.content.clone();
        let before_formatted = result.formatted_content.clone();
        match self.edit {
            Edit::AppendContent => result.content.push_str("\nappended"),
            Edit::RestoreContent => {
                assert_eq!(result.content, "JaneDoe tail\nappended");
                result.content.truncate("JaneDoe tail".len());
            }
            Edit::Noop | Edit::LateNoop => {}
            Edit::LateInjectHtml => {
                let html = "<p>Jane<strong>Doe</strong> tail</p>";
                result.formatted_content = Some(html.into());
                let page = &mut result.pages.as_mut().unwrap()[0];
                page.content = html.into();
                page.native_content = Some("JaneDoe raw".into());
            }
            Edit::LateInjectArchiveHtml | Edit::LateInjectOcrHtml => {
                let mut nested = ExtractedDocument {
                    content: "<p>Jane<strong>Doe</strong> nested</p>".into(),
                    ..Default::default()
                };
                nested.metadata.output_format = Some("html".into());
                if matches!(self.edit, Edit::LateInjectArchiveHtml) {
                    result.children = Some(vec![crate::types::ArchiveEntry {
                        path: "child".into(),
                        mime_type: "text/html".into(),
                        result: Box::new(nested),
                    }]);
                } else {
                    result.images = Some(vec![crate::types::ExtractedImage {
                        ocr_result: Some(Box::new(nested)),
                        ..Default::default()
                    }]);
                }
            }
            Edit::ReplaceFormatted => result.formatted_content = Some("<p>Ja<strong>neDoe</strong> tail</p>".into()),
            Edit::RemoveFormatted => result.formatted_content = None,
            Edit::ReplacePage => {
                result.pages.as_mut().unwrap()[0].content = "<p>Jane<strong>Doe</strong> replaced</p>".into()
            }
            Edit::ReplaceNative => {
                result.pages.as_mut().unwrap()[0].native_content = Some("Jane<strong>Doe</strong> raw".into())
            }
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
        if self.edit.is_late() {
            ProcessingStage::Late
        } else {
            ProcessingStage::Middle
        }
    }

    fn priority(&self) -> i32 {
        if matches!(self.edit, Edit::AppendContent) { 2 } else { 1 }
    }
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Html,
        redaction: Some(RedactionConfig {
            custom_terms: vec![RedactionTerm::labeled("private", "JaneDoe")],
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn actual_html(config: &ExtractionConfig, edit: Edit) -> DerivedResult {
    let mut source = HtmlExtractor
        .extract_sync(b"<p>Jane<strong>Doe</strong> tail</p>", "text/html", config)
        .unwrap();
    assert!(source.elements.iter().any(|element| element.text == "JaneDoe tail"));
    assert!(source.pre_rendered_content.is_none());
    if matches!(edit, Edit::ReplacePage | Edit::ReplaceNative | Edit::LateInjectHtml) {
        for element in &mut source.elements {
            element.page = Some(1);
        }
    }
    let result = derive_with_surfaces(source, false, OutputFormat::Html, true);
    assert_eq!(result.content, "JaneDoe tail");
    if matches!(edit, Edit::ReplacePage | Edit::ReplaceNative | Edit::LateInjectHtml) {
        assert_eq!(result.pages.as_ref().unwrap().len(), 1);
    } else {
        assert!(result.pages.is_none());
    }
    assert!(result.redaction_report.is_none());
    result
}

async fn execute_edits(
    result: &mut DerivedResult,
    config: &ExtractionConfig,
    edits: &[Edit],
    receipts: &Receipts,
) -> Result<()> {
    let mut registry = crate::plugins::registry::PostProcessorRegistry::new();
    assert!(registry.register_builtin_redaction().unwrap());
    for &edit in edits {
        registry
            .register(Arc::new(Mutate {
                edit,
                receipts: Arc::clone(receipts),
            }))
            .unwrap();
    }
    let cache = super::super::cache::ProcessorCache::from_registry(&Arc::new(RwLock::new(registry)), 0).unwrap();
    let late = edits.iter().filter(|edit| edit.is_late()).count();
    assert_eq!(cache.middle.len(), edits.len() - late);
    assert_eq!(cache.late.len(), 1 + late);
    assert!(Arc::ptr_eq(&cache.late[0], cache.builtin_redaction.as_ref().unwrap()));
    execute_processor_stages(
        result,
        config,
        &None,
        &[
            (ProcessingStage::Middle, Arc::clone(&cache.middle)),
            (ProcessingStage::Late, Arc::clone(&cache.late)),
        ],
        cache.builtin_redaction.as_ref(),
    )
    .await
}

async fn mutation_is_rejected(edit: Edit) {
    let config = config();
    let mut result = actual_html(&config, edit);
    let original_formatted = result.formatted_content.clone();
    let receipts = Arc::new(Mutex::new(Vec::new()));
    let outcome = execute_edits(&mut result, &config, &[edit], &receipts).await;
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
            assert_eq!(
                receipt.after_formatted.as_deref(),
                Some("<p>Ja<strong>neDoe</strong> tail</p>")
            );
        }
        Edit::RemoveFormatted => {
            assert_eq!(receipt.after_plain, "JaneDoe tail");
            assert!(receipt.after_formatted.is_none());
        }
        Edit::ReplacePage | Edit::ReplaceNative => {
            assert_eq!(receipt.after_plain, "JaneDoe tail");
            assert_eq!(receipt.after_formatted, original_formatted);
        }
        Edit::RestoreContent
        | Edit::Noop
        | Edit::LateInjectHtml
        | Edit::LateNoop
        | Edit::LateInjectArchiveHtml
        | Edit::LateInjectOcrHtml => {
            unreachable!("only isolated changes use this test helper")
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

#[tokio::test]
async fn observed_html_page_replacement_retains_the_original_hash_obligation() {
    mutation_is_rejected(Edit::ReplacePage).await;
}

#[tokio::test]
async fn observed_html_native_text_replacement_retains_the_original_hash_obligation() {
    mutation_is_rejected(Edit::ReplaceNative).await;
}

#[tokio::test]
async fn html_edit_then_restore_cannot_clear_the_observed_obligation() {
    let config = config();
    let mut result = actual_html(&config, Edit::AppendContent);
    let before = serde_json::to_value(&*result).unwrap();
    let receipts = Arc::new(Mutex::new(Vec::new()));
    let outcome = execute_edits(
        &mut result,
        &config,
        &[Edit::AppendContent, Edit::RestoreContent],
        &receipts,
    )
    .await;
    let receipts = receipts.lock();
    assert_eq!(receipts.len(), 2);
    assert_eq!(receipts[0].after_plain, "JaneDoe tail\nappended");
    assert_eq!(receipts[1].before_plain, "JaneDoe tail\nappended");
    assert_eq!(receipts[1].after_serialized, before);
    assert!(matches!(outcome, Err(XbergError::Plugin { .. })));
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
    assert!(result.redaction_report.is_none());
}

#[tokio::test]
async fn exact_html_noop_callback_retains_original_authority() {
    let config = config();
    let mut result = actual_html(&config, Edit::Noop);
    let before = serde_json::to_value(&*result).unwrap();
    let receipts = Arc::new(Mutex::new(Vec::new()));
    execute_edits(&mut result, &config, &[Edit::Noop], &receipts)
        .await
        .unwrap();
    let receipts = receipts.lock();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].after_serialized, before);
    assert_eq!(
        result.formatted_content.as_deref(),
        Some("<pre><span>[REDACTED] tail</span></pre>")
    );
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 1);
}

#[tokio::test]
async fn html_lower_priority_late_processor_cannot_reinsert_unredacted_surfaces() {
    let config = config();
    let mut result = actual_html(&config, Edit::LateInjectHtml);
    let receipts = Arc::new(Mutex::new(Vec::new()));
    let outcome = execute_edits(&mut result, &config, &[Edit::LateInjectHtml], &receipts).await;
    let receipts = receipts.lock();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].before_plain, "[REDACTED] tail");
    assert_eq!(
        receipts[0].before_formatted.as_deref(),
        Some("<pre><span>[REDACTED] tail</span></pre>")
    );
    assert_eq!(
        receipts[0].after_serialized["pages"][0]["native_content"],
        serde_json::json!("JaneDoe raw")
    );
    assert!(matches!(outcome, Err(XbergError::Plugin { .. })));
}

#[tokio::test]
async fn exact_html_lower_priority_late_noop_keeps_verified_output() {
    let config = config();
    let mut result = actual_html(&config, Edit::LateNoop);
    let receipts = Arc::new(Mutex::new(Vec::new()));
    execute_edits(&mut result, &config, &[Edit::LateNoop], &receipts)
        .await
        .unwrap();
    let receipts = receipts.lock();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].before_plain, "[REDACTED] tail");
    assert_eq!(
        receipts[0].before_formatted.as_deref(),
        Some("<pre><span>[REDACTED] tail</span></pre>")
    );
    assert_eq!(receipts[0].after_formatted, receipts[0].before_formatted);
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 1);
}

async fn sealed_html_rejects_nested_injection(edit: Edit, path: &str) {
    let config = config();
    let mut result = actual_html(&config, edit);
    let receipts = Arc::new(Mutex::new(Vec::new()));
    let outcome = execute_edits(&mut result, &config, &[edit], &receipts).await;
    let receipts = receipts.lock();
    assert_eq!(receipts.len(), 1);
    let receipt = &receipts[0];
    assert_eq!(receipt.before_plain, "[REDACTED] tail");
    assert_eq!(receipt.after_plain, receipt.before_plain);
    assert_eq!(receipt.after_formatted, receipt.before_formatted);
    assert!(result.pages.is_none());
    let child = receipt.after_serialized.pointer(path).unwrap();
    assert_eq!(child["metadata"]["output_format"], "html");
    assert_eq!(child["content"], "<p>Jane<strong>Doe</strong> nested</p>");
    assert!(matches!(outcome, Err(XbergError::Plugin { .. })));
}

#[tokio::test]
async fn sealed_html_rejects_an_archive_child_injected_by_late_processor() {
    sealed_html_rejects_nested_injection(Edit::LateInjectArchiveHtml, "/children/0/result").await;
}

#[tokio::test]
async fn sealed_html_rejects_an_ocr_child_injected_by_late_processor() {
    sealed_html_rejects_nested_injection(Edit::LateInjectOcrHtml, "/images/0/ocr_result").await;
}
