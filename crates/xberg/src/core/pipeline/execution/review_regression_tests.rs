use super::*;
use crate::core::config::OutputFormat;
use crate::core::config::redaction::{RedactionConfig, RedactionTerm};
use crate::extraction::derive::surfaces::{DerivedResult, derive_with_surfaces};
use crate::plugins::{Plugin, PostProcessor};
use crate::types::annotations::{PdfAnnotation, PdfAnnotationType};
use crate::types::builder::superscript;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::redaction::RedactionStrategy;
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};

const SOURCE_TAGS: &str = "sup <sup>note</sup>";
const REDACTED_TAGS: &str = "[REDACTED] <[REDACTED]>note</[REDACTED]>";
const GENERATED_POWER: &str = "power<sup>2</sup>";

type Cache = super::super::cache::ProcessorCache;

fn config(term: &str) -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        redaction: Some(RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("private", term)],
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn document() -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("power2", vec![superscript(5, 6)], Some(1), None);
    let result = derive_with_surfaces(builder.build(), false, OutputFormat::Markdown, true);
    assert_eq!(result.content, "power2");
    assert_eq!(result.formatted_content.as_deref(), Some("power<sup>2</sup>\n"));
    assert_eq!(result.pages.as_ref().unwrap().len(), 1);
    result
}

fn cache(extra: Option<Arc<dyn PostProcessor>>) -> Cache {
    let mut registry = crate::plugins::registry::PostProcessorRegistry::new();
    assert!(registry.register_builtin_redaction().unwrap());
    if let Some(processor) = extra {
        registry.register(processor).unwrap();
    }
    let cache = Cache::from_registry(&Arc::new(RwLock::new(registry)), 0).unwrap();
    let builtin = cache.builtin_redaction.as_ref().expect("actual builtin identity");
    assert!(Arc::ptr_eq(cache.late.first().unwrap(), builtin));
    cache
}

async fn stages(
    result: &mut DerivedResult,
    cache: &Cache,
    config: &ExtractionConfig,
    selected: &[ProcessorStage],
) -> Result<()> {
    execute_processor_stages(result, config, &None, selected, cache.builtin_redaction.as_ref()).await
}

async fn builtin(result: &mut DerivedResult, config: &ExtractionConfig) -> Result<()> {
    let cache = cache(None);
    assert_eq!(cache.late.len(), 1);
    stages(
        result,
        &cache,
        config,
        &[(ProcessingStage::Late, Arc::clone(&cache.late))],
    )
    .await
}

#[derive(Clone, Copy)]
enum SourceSite {
    AnnotationText,
    AnnotationAuthor,
    RawBody,
}

fn annotation(content: &str, author: Option<&str>) -> PdfAnnotation {
    PdfAnnotation {
        annotation_type: PdfAnnotationType::Text,
        content: Some(content.to_string()),
        page_number: 1,
        bounding_box: None,
        author: author.map(str::to_string),
        modified: None,
        color: None,
        subject: None,
        quad_points: None,
        marked_text: None,
    }
}

fn source_document(site: SourceSite) -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("power2", vec![superscript(5, 6)], Some(1), None);
    match site {
        SourceSite::AnnotationText => builder.set_pdf_annotations(vec![annotation(SOURCE_TAGS, None)]),
        SourceSite::AnnotationAuthor => {
            builder.set_pdf_annotations(vec![annotation("plain annotation", Some(SOURCE_TAGS))]);
        }
        SourceSite::RawBody => {
            builder.push_raw_block("html", SOURCE_TAGS, Some(1));
        }
    }
    let result = derive_with_surfaces(builder.build(), false, OutputFormat::Markdown, true);
    if matches!(site, SourceSite::AnnotationAuthor) {
        let annotations = result.annotations.as_ref().expect("actual structured annotation");
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].author.as_deref(), Some(SOURCE_TAGS));
    }
    assert!(result.content.contains(SOURCE_TAGS));
    assert_eq!(result.content.matches("sup").count(), 3);
    let formatted = result.formatted_content.as_deref().unwrap();
    assert!(formatted.contains(GENERATED_POWER));
    assert!(formatted.contains(SOURCE_TAGS));
    assert_eq!(formatted.matches("sup").count(), 5);
    result
}

async fn generated_style_control() {
    let mut result = document();
    let before = result.formatted_content.clone();
    builtin(&mut result, &config("sup")).await.unwrap();
    assert_eq!(result.content, "power2");
    assert_eq!(result.formatted_content, before);
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 0);
}

async fn source_tags_case(site: SourceSite) {
    generated_style_control().await;
    let mut result = source_document(site);
    builtin(&mut result, &config("sup")).await.unwrap();
    assert!(result.content.contains(REDACTED_TAGS));
    assert!(!result.content.contains("sup"));
    if let Some(formatted) = &result.formatted_content {
        assert!(
            formatted.contains(GENERATED_POWER),
            "real generated script styling must survive"
        );
        assert!(formatted.contains("note"), "annotation/raw source data must survive");
        assert!(
            !formatted.replacen(GENERATED_POWER, "power2", 1).contains("sup"),
            "source-authored tag names cannot inherit generated-tag exemptions"
        );
    } else {
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|warning| warning.source == "output_format")
        );
    }
}

#[tokio::test]
async fn annotation_text_tags_are_redacted_as_source_data() {
    source_tags_case(SourceSite::AnnotationText).await;
}

#[tokio::test]
async fn annotation_author_tags_in_derived_rendering_are_redacted_as_source_data() {
    source_tags_case(SourceSite::AnnotationAuthor).await;
}

#[tokio::test]
async fn raw_body_tags_are_not_certified_as_renderer_owned() {
    source_tags_case(SourceSite::RawBody).await;
}

fn assert_provenance_error(outcome: Result<()>) {
    let error = outcome.expect_err("an unobserved mutation must not gain provenance");
    assert!(error.to_string().contains("Generated Markdown provenance changed"));
}

#[tokio::test]
async fn unwitnessed_content_change_is_rejected_before_redaction() {
    let mut baseline = document();
    builtin(&mut baseline, &config("secret")).await.unwrap();
    let mut result = document();
    let formatted = result.formatted_content.clone();
    result.content.push_str(" direct secret");
    assert_eq!(result.content, "power2 direct secret");
    assert_provenance_error(builtin(&mut result, &config("secret")).await);
    assert_eq!(result.content, "power2 direct secret");
    assert_eq!(result.formatted_content, formatted);
    assert!(result.redaction_report.is_none());
}

#[derive(Clone, Copy)]
enum Edit {
    RemoveFormatted,
    AppendAfterBuiltin,
}

struct Mutation {
    edit: Edit,
    calls: Arc<Mutex<Vec<Option<u32>>>>,
}

impl Plugin for Mutation {
    fn name(&self) -> &str {
        "review-provenance-mutation-2360"
    }
}

#[async_trait]
impl PostProcessor for Mutation {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        self.calls
            .lock()
            .push(result.redaction_report.as_ref().map(|report| report.total_redacted));
        result.content.push_str(" observed edit");
        match self.edit {
            Edit::RemoveFormatted => result.formatted_content = None,
            Edit::AppendAfterBuiltin => result.formatted_content.as_mut().unwrap().push_str(" observed edit"),
        }
        Ok(())
    }
    fn processing_stage(&self) -> ProcessingStage {
        match self.edit {
            Edit::RemoveFormatted => ProcessingStage::Middle,
            Edit::AppendAfterBuiltin => ProcessingStage::Late,
        }
    }
    fn priority(&self) -> i32 {
        0
    }
}

#[tokio::test]
async fn unobserved_formatted_reinsertion_after_observed_removal_is_rejected() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let cache = cache(Some(Arc::new(Mutation {
        edit: Edit::RemoveFormatted,
        calls: Arc::clone(&calls),
    })));
    assert_eq!(cache.middle.len(), 1);
    let mut result = document();
    stages(
        &mut result,
        &cache,
        &config("secret"),
        &[(ProcessingStage::Middle, Arc::clone(&cache.middle))],
    )
    .await
    .unwrap();
    assert_eq!(calls.lock().as_slice(), [None]);
    assert_eq!(result.content, "power2 observed edit");
    assert!(result.formatted_content.is_none());
    builtin(&mut result, &config("secret")).await.unwrap();
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 0);
    result.formatted_content = Some("unobserved secret".to_string());
    assert_provenance_error(builtin(&mut result, &config("secret")).await);
    assert_eq!(result.formatted_content.as_deref(), Some("unobserved secret"));
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 0);
}

#[tokio::test]
async fn completed_builtin_cannot_recertify_late_mutations() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let cache = cache(Some(Arc::new(Mutation {
        edit: Edit::AppendAfterBuiltin,
        calls: Arc::clone(&calls),
    })));
    assert_eq!(cache.late.len(), 2);
    let mut result = document();
    stages(
        &mut result,
        &cache,
        &config("absent"),
        &[(ProcessingStage::Late, Arc::clone(&cache.late))],
    )
    .await
    .unwrap();
    assert_eq!(
        calls.lock().as_slice(),
        [Some(0)],
        "actual builtin completes before the ordinary Late edit"
    );
    assert_eq!(result.content, "power2 observed edit");
    assert!(result.formatted_content.as_deref().unwrap().contains(GENERATED_POWER));
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 0);
    assert_provenance_error(builtin(&mut result, &config("absent")).await);
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 0);
}
