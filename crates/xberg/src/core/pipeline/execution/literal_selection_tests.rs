use super::*;
use crate::core::config::OutputFormat;
use crate::core::config::redaction::{RedactionConfig, RedactionPattern, RedactionTerm};
use crate::extraction::derive::surfaces::{DerivedResult, ProcessingDocument, RedactionSurfaces, derive_with_surfaces};
use crate::plugins::{Plugin, PostProcessor};
use crate::text::redaction::strategy::{TokenCounter, apply_strategy};
use crate::types::builder::superscript;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::redaction::{PiiCategory, RedactionStrategy};
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};

type Cache = super::super::cache::ProcessorCache;
type Calls = Arc<Mutex<Vec<(serde_json::Value, serde_json::Value)>>>;
const BASELINE: &str = "Public baseline.";

struct Edit {
    formatted: &'static str,
    title: Option<&'static str>,
    calls: Calls,
}
impl Plugin for Edit {
    fn name(&self) -> &str {
        "literal-selection-probe"
    }
}
#[async_trait]
impl PostProcessor for Edit {
    async fn process(&self, document: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        let before = serde_json::to_value(&*document).unwrap();
        document.formatted_content = Some(self.formatted.into());
        document.metadata.title = self.title.map(str::to_owned);
        self.calls
            .lock()
            .push((before, serde_json::to_value(document).unwrap()));
        Ok(())
    }
    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
}

fn config(strategy: RedactionStrategy, terms: &[&str], patterns: &[&str]) -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        enable_quality_processing: false,
        redaction: Some(RedactionConfig {
            strategy,
            custom_terms: terms
                .iter()
                .map(|term| RedactionTerm::labeled("private", *term))
                .collect(),
            custom_patterns: patterns
                .iter()
                .map(|pattern| RedactionPattern::labeled("private", *pattern))
                .collect(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn cache(edit: Option<Arc<dyn PostProcessor>>) -> Cache {
    let mut registry = crate::plugins::registry::PostProcessorRegistry::new();
    assert!(registry.register_builtin_redaction().unwrap());
    if let Some(edit) = edit {
        registry.register(edit).unwrap();
    }
    let cache = Cache::from_registry(&Arc::new(RwLock::new(registry)), 0).unwrap();
    assert_eq!(cache.late.len(), 1);
    assert!(Arc::ptr_eq(
        cache.late.first().unwrap(),
        cache.builtin_redaction.as_ref().unwrap()
    ));
    cache
}

async fn edited(
    formatted: &'static str,
    title: Option<&'static str>,
    config: &ExtractionConfig,
) -> (DerivedResult, Cache) {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(BASELINE, vec![], Some(1), None);
    let mut result = derive_with_surfaces(builder.build(), false, OutputFormat::Markdown, true);
    assert_eq!(result.content, BASELINE);
    for term in &config.redaction.as_ref().unwrap().custom_terms {
        assert!(
            !BASELINE.contains(&term.value),
            "private value is introduced by the real callback"
        );
    }
    assert_eq!(result.formatted_content.as_deref(), Some("Public baseline.\n"));
    assert_eq!(result.pages.as_ref().unwrap().len(), 1);
    assert_eq!(result.pages.as_ref().unwrap()[0].content, "Public baseline.\n");
    let calls: Calls = Arc::default();
    let cache = cache(Some(Arc::new(Edit {
        formatted,
        title,
        calls: Arc::clone(&calls),
    })));
    assert_eq!(cache.middle.len(), 1);
    execute_processor_stages(
        &mut result,
        config,
        &None,
        &[(ProcessingStage::Middle, Arc::clone(&cache.middle))],
        cache.builtin_redaction.as_ref(),
    )
    .await
    .unwrap();
    let calls = calls.lock();
    assert_eq!(calls.len(), 1);
    let (before, after) = &calls[0];
    for field in ["content", "pages", "redaction_report"] {
        assert_eq!(before[field], after[field]);
    }
    assert_eq!(result.formatted_content.as_deref(), Some(formatted));
    assert!(result.redaction_report.is_none());
    assert_eq!(result.metadata.title.as_deref(), title);
    let (document, surfaces) = result.redaction_parts().unwrap();
    let surfaces = surfaces.expect("actual captured producer context");
    surfaces.validate(document).unwrap();
    assert_eq!(surfaces.formatted_literal_backup(), Some(formatted));
    assert!(surfaces.discard_formatted());
    (result, cache)
}

async fn late(result: &mut DerivedResult, cache: &Cache, config: &ExtractionConfig) -> Result<()> {
    execute_processor_stages(
        result,
        config,
        &None,
        &[(ProcessingStage::Late, Arc::clone(&cache.late))],
        cache.builtin_redaction.as_ref(),
    )
    .await
}

fn finish(result: DerivedResult) -> ExtractedDocument {
    assert!(result.formatted_content.is_none());
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "output_format")
    );
    assert!(result.redaction_report.is_some());
    let (document, format) = result.into_output(OutputFormat::Markdown);
    assert!(matches!(format, OutputFormat::Plain));
    let document = crate::core::pipeline::apply_output_format(document, format);
    assert_eq!(document.metadata.output_format.as_deref(), Some("plain"));
    document
}

fn visible(source: &str) -> String {
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, source, &crate::rendering::comrak_options());
    let mut text = String::new();
    for node in root.descendants() {
        match &node.data.borrow().value {
            comrak::nodes::NodeValue::Text(value) => text.push_str(value),
            comrak::nodes::NodeValue::Code(code) => text.push_str(&code.literal),
            _ => {}
        }
    }
    text
}

#[tokio::test]
async fn formatted_literal_combines_raw_and_decoded_findings() {
    let source = r"Visit AT\&T and force now";
    assert_eq!(visible(source), "Visit AT&T and force now");
    assert_eq!(&source[6..11], r"AT\&T");
    assert!(!BASELINE.contains("AT&T") && !BASELINE.contains("force"));
    let config = config(RedactionStrategy::Mask, &["AT&T", "force"], &[]);
    let (mut result, cache) = edited(source, None, &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(
        (output.content.as_str(), output.redaction_report.unwrap().total_redacted),
        ("Visit [REDACTED] and [REDACTED] now", 2)
    );
}

async fn raw_tag_controls() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("power2", vec![superscript(5, 6)], Some(1), None);
    let mut generated = derive_with_surfaces(builder.build(), false, OutputFormat::Markdown, true);
    assert_eq!(generated.formatted_content.as_deref(), Some("power<sup>2</sup>\n"));
    let config = config(RedactionStrategy::Mask, &["sup"], &[]);
    late(&mut generated, &cache(None), &config).await.unwrap();
    assert_eq!(generated.formatted_content.as_deref(), Some("power<sup>2</sup>\n"));
    assert_eq!(generated.redaction_report.as_ref().unwrap().total_redacted, 0);
    let (mut literal, cache) = edited("Jane<sup>Doe</sup>", None, &config).await;
    late(&mut literal, &cache, &config).await.unwrap();
    let output = finish(literal);
    assert_eq!(
        (output.content.as_str(), output.redaction_report.unwrap().total_redacted),
        ("Jane<[REDACTED]>Doe</[REDACTED]>", 2)
    );
}

#[tokio::test]
async fn formatted_literal_split_match_closes_only_its_wrappers() {
    raw_tag_controls().await;
    let source = "Visit Jane<u>Doe</u> now safe-branch";
    assert_eq!(visible(source), "Visit JaneDoe now safe-branch");
    assert_eq!(&source[6..20], "Jane<u>Doe</u>");
    assert!(!BASELINE.contains("JaneDoe"));
    let config = config(RedactionStrategy::Mask, &["JaneDoe"], &[]);
    let (mut result, cache) = edited(source, None, &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(
        (output.content.as_str(), output.redaction_report.unwrap().total_redacted),
        ("Visit [REDACTED] now safe-branch", 1)
    );
}

fn encoded_identity_preconditions(source: &str) {
    let raw = regex::Regex::new(r"AT\\&T").unwrap();
    let primary = raw.find(source).unwrap();
    assert_eq!((primary.start(), primary.end(), primary.as_str()), (6, 11, r"AT\&T"));
    assert_eq!(visible(source), "Visit AT&T now");
    assert!(regex::Regex::new("AT&T").unwrap().find(source).is_none());
    assert_eq!(
        regex::Regex::new("AT&T").unwrap().find_iter(&visible(source)).count(),
        1
    );
}

#[tokio::test]
async fn formatted_literal_tokens_use_decoded_identity() {
    let source = r"Visit AT\&T now";
    encoded_identity_preconditions(source);
    let config = config(RedactionStrategy::TokenReplace, &["AT&T"], &[r"AT\\&T"]);
    let (mut result, cache) = edited(source, Some("AT&T"), &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.redaction_report.as_ref().unwrap().total_redacted, 2);
    assert_eq!(
        output.content, "Visit [PRIVATE_1] now",
        "the old raw path already replaces this primary"
    );
    assert_eq!(
        output.metadata.title.as_deref(),
        Some("[PRIVATE_1]"),
        "raw and decoded spellings are one identity"
    );
}

#[tokio::test]
async fn formatted_literal_hashes_use_decoded_identity() {
    let source = r"Visit AT\&T now";
    encoded_identity_preconditions(source);
    let category = PiiCategory::Custom("private".into());
    let expected = apply_strategy(RedactionStrategy::Hash, "AT&T", &category, &mut TokenCounter::new());
    let wrong = apply_strategy(RedactionStrategy::Hash, r"AT\&T", &category, &mut TokenCounter::new());
    assert_ne!(expected, wrong);
    let config = config(RedactionStrategy::Hash, &["AT&T"], &[r"AT\\&T"]);
    let (mut result, cache) = edited(source, Some("AT&T"), &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.redaction_report.as_ref().unwrap().total_redacted, 2);
    assert_eq!(output.metadata.title.as_deref(), Some(expected.as_str()));
    assert_eq!(
        output.content,
        format!("Visit {expected} now"),
        "hash semantic bytes, not the encoded envelope"
    );
}

#[tokio::test]
async fn formatted_literal_overlap_tail_aborts_before_mutation() {
    let source = r"ab\&cd";
    assert_eq!(visible(source), "ab&cd");
    let raw = regex::Regex::new(r"ab\\&").unwrap().find(source).unwrap();
    assert_eq!((raw.start(), raw.end(), raw.as_str()), (0, 4, r"ab\&"));
    let decoded = visible(source);
    let semantic_regex = regex::Regex::new("&cd").unwrap();
    let semantic = semantic_regex.find(&decoded).unwrap();
    assert_eq!((semantic.start(), semantic.end()), (2, 5));
    let raw_tail = semantic_regex.find(source).unwrap();
    assert_eq!((raw_tail.start(), raw_tail.end()), (3, 6));
    assert_eq!(&source[2..6], r"\&cd");
    let config = config(RedactionStrategy::TokenReplace, &[], &[r"ab\\&", "&cd"]);
    let (mut result, cache) = edited(source, None, &config).await;
    let before = serde_json::to_value(&*result).unwrap();
    let before_formatted = result.formatted_content.clone();
    let outcome = late(&mut result, &cache, &config).await;
    let error = outcome.expect_err("selected source ranges must cover the decoded finding's tail");
    assert!(error.to_string().contains("coverage"), "{error}");
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
    assert_eq!(result.formatted_content, before_formatted);
    assert!(result.redaction_report.is_none());
}
