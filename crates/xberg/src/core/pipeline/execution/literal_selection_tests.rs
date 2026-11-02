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

async fn unsupported_link_is_transactional(term: &str, matched: bool) {
    let source = "Visit [JaneDoe](https://example.test) now";
    let decoded = visible(source);
    assert_eq!(decoded, "Visit JaneDoe now");
    assert_eq!(source.contains(term), matched);
    assert_eq!(decoded.contains(term), matched);
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, source, &crate::rendering::comrak_options());
    assert_eq!(
        root.descendants()
            .filter(|node| matches!(node.data.borrow().value, comrak::nodes::NodeValue::Link(_)))
            .count(),
        1
    );
    let config = config(RedactionStrategy::Mask, &[term], &[]);
    let (mut result, cache) = edited(source, None, &config).await;
    let before = serde_json::to_value(&*result).unwrap();
    let formatted = result.formatted_content.clone();
    let error = late(&mut result, &cache, &config)
        .await
        .expect_err("unsupported witnessed formatted syntax must fail even without findings");
    assert!(matches!(&error, XbergError::Plugin { plugin_name, .. } if plugin_name == "redaction"));
    assert!(
        error
            .to_string()
            .contains("formatted literal Markdown redaction: unsupported syntax"),
        "{error}"
    );
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
    assert_eq!(result.formatted_content, formatted);
    assert!(result.redaction_report.is_none());
}

#[tokio::test]
async fn witnessed_unsupported_formatted_link_with_match_aborts_before_mutation() {
    unsupported_link_is_transactional("JaneDoe", true).await;
}

#[tokio::test]
async fn witnessed_unsupported_formatted_link_without_match_still_aborts_before_mutation() {
    unsupported_link_is_transactional("AbsentUnique", false).await;
}

#[tokio::test]
async fn captured_generated_heading_and_list_keep_existing_markdown_path() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_heading(1, "Public heading", Some(1), None);
    builder.push_list(false);
    builder.push_list_item("Public item", false, vec![], Some(1), None);
    builder.end_list();
    let mut result = derive_with_surfaces(builder.build(), false, OutputFormat::Markdown, true);
    let formatted = result.formatted_content.clone().unwrap();
    let pages = serde_json::to_value(result.pages.as_ref().unwrap()).unwrap();
    assert_eq!(result.pages.as_ref().unwrap().len(), 1);
    assert!(formatted.contains("Public heading") && formatted.contains("Public item"));
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, &formatted, &crate::rendering::comrak_options());
    assert_eq!(
        root.descendants()
            .filter(|node| matches!(node.data.borrow().value, comrak::nodes::NodeValue::Heading(_)))
            .count(),
        1
    );
    assert_eq!(
        root.descendants()
            .filter(|node| matches!(node.data.borrow().value, comrak::nodes::NodeValue::List(_)))
            .count(),
        1
    );
    {
        let (document, surfaces) = result.redaction_parts().unwrap();
        let surfaces = surfaces.expect("actual generated complex Markdown capture");
        surfaces.validate(document).unwrap();
        assert!(surfaces.formatted_markdown());
        assert!(!surfaces.discard_formatted());
        assert_eq!(surfaces.formatted_literal_backup(), None);
    }
    let config = config(RedactionStrategy::Mask, &["AbsentUnique"], &[]);
    late(&mut result, &cache(None), &config).await.unwrap();
    assert_eq!(result.formatted_content.as_deref(), Some(formatted.as_str()));
    assert_eq!(serde_json::to_value(result.pages.as_ref().unwrap()).unwrap(), pages);
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 0);
    assert!(result.processing_warnings.is_empty());
}

#[tokio::test]
async fn ordinary_unwitnessed_complex_markdown_keeps_existing_redaction_path() {
    let source = "Visit [JaneDoe](https://example.test) now";
    assert_eq!(visible(source), "Visit JaneDoe now");
    let mut result = ExtractedDocument {
        content: "JaneDoe".into(),
        formatted_content: Some(source.into()),
        ..Default::default()
    };
    let config = config(RedactionStrategy::Mask, &["JaneDoe"], &[]);
    crate::plugins::processor::builtin::redaction::RedactionProcessor
        .process(&mut result, &config)
        .await
        .unwrap();
    assert_eq!(result.content, "[REDACTED]");
    assert_eq!(
        result.formatted_content.as_deref(),
        Some("Visit [[REDACTED]](https://example.test) now")
    );
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 2);
    assert!(result.processing_warnings.is_empty());
}

const BUDGET_LITERAL: &str = "Visit force now";

async fn budget_case(limits: crate::extractors::security::SecurityLimits) -> (DerivedResult, Cache, ExtractionConfig) {
    assert_eq!(visible(BUDGET_LITERAL), BUDGET_LITERAL);
    assert_eq!(&BUDGET_LITERAL[6..11], "force");
    let mut config = config(RedactionStrategy::Mask, &["force"], &[]);
    let redaction = config.redaction.as_ref().unwrap();
    assert_eq!(redaction.custom_terms.len(), 1);
    assert!(redaction.custom_patterns.is_empty());
    assert!(redaction.findings.is_empty());
    assert!(redaction.findings_path.is_none());
    assert!(crate::text::redaction::external::current_external_redaction().is_none());
    assert!(limits.max_iterations > redaction.custom_terms.len());
    assert!(BASELINE.len() <= limits.max_content_size);
    assert!(BUDGET_LITERAL.len() <= limits.max_content_size);
    assert!("Visit [REDACTED] now".len() <= limits.max_content_size);
    assert!(BUDGET_LITERAL.len() <= limits.max_entity_length);
    assert!(limits.max_nesting_depth > 3);
    config.security_limits = Some(limits);
    let (result, cache) = edited(BUDGET_LITERAL, None, &config).await;
    assert!(
        result.pages.as_ref().unwrap()[0].content.len() <= config.security_limits.as_ref().unwrap().max_content_size
    );
    (result, cache, config)
}

async fn assert_literal_budget_error(limits: crate::extractors::security::SecurityLimits, reason: &str) {
    let (mut result, cache, config) = budget_case(limits).await;
    let before = serde_json::to_value(&*result).unwrap();
    let formatted = result.formatted_content.clone();
    let error = late(&mut result, &cache, &config)
        .await
        .expect_err("literal planning budget must abort before document mutation");
    assert!(matches!(&error, XbergError::Plugin { plugin_name, .. } if plugin_name == "redaction"));
    assert!(error.to_string().contains(reason), "{error}");
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
    assert_eq!(result.formatted_content, formatted);
    assert!(result.redaction_report.is_none());
}

#[tokio::test]
async fn witnessed_literal_with_ample_operation_and_owned_limits_redacts_normally() {
    let (mut result, cache, config) = budget_case(crate::extractors::security::SecurityLimits::default()).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.content, "Visit [REDACTED] now");
    assert_eq!(output.redaction_report.as_ref().unwrap().total_redacted, 1);
}

#[tokio::test]
async fn witnessed_literal_iteration_limit_aborts_before_document_mutation() {
    let limits = crate::extractors::security::SecurityLimits {
        max_iterations: BUDGET_LITERAL.len() - 1,
        ..Default::default()
    };
    assert_literal_budget_error(limits, "formatted literal Markdown redaction: iteration limit").await;
}

#[tokio::test]
async fn witnessed_literal_requested_owned_limit_aborts_when_all_text_inputs_fit() {
    let input_bytes = BASELINE.len() + BUDGET_LITERAL.len() + BASELINE.len() + 1;
    assert!(
        BUDGET_LITERAL.len() * std::mem::size_of::<(std::ops::Range<usize>, std::ops::Range<usize>)>() > input_bytes
    );
    let limits = crate::extractors::security::SecurityLimits {
        max_content_size: input_bytes,
        ..Default::default()
    };
    assert_literal_budget_error(
        limits,
        "formatted literal Markdown redaction: requested helper capacity",
    )
    .await;
}

#[tokio::test]
async fn witnessed_literal_long_winner_covers_disjoint_candidate_requirements() {
    let source = "safe alpha beta suffix";
    assert_eq!(visible(source), source);
    assert_eq!(&source[5..15], "alpha beta");
    assert_eq!((&source[5..10], &source[11..15]), ("alpha", "beta"));
    let config = config(RedactionStrategy::Mask, &[], &["alpha beta", "alpha", "beta"]);
    let (mut result, cache) = edited(source, None, &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.content, "safe [REDACTED] suffix");
    assert_eq!(output.redaction_report.as_ref().unwrap().total_redacted, 1);
}

fn multi_scalar_entity_preconditions() -> (&'static str, &'static str, &'static str) {
    let (source, short, full) = ("&NotEqualTilde;", "≂", "≂\u{0338}");
    assert_eq!(source.len(), 15);
    assert_eq!(visible(source), full);
    assert_eq!((short.len(), full.len(), full.chars().count()), (3, 5, 2));
    assert!(!source.contains(short) && !source.contains(full));
    let short_match = regex::Regex::new(&regex::escape(short)).unwrap().find(full).unwrap();
    let full_match = regex::Regex::new(&regex::escape(full)).unwrap().find(full).unwrap();
    assert_eq!((short_match.start(), short_match.end()), (0, 3));
    assert_eq!((full_match.start(), full_match.end()), (0, 5));
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, source, &crate::rendering::comrak_options());
    let mut texts = 0;
    for node in root.descendants() {
        let data = node.data.borrow();
        if let comrak::nodes::NodeValue::Text(text) = &data.value {
            texts += 1;
            let actual: &str = text.as_ref();
            assert_eq!(actual, full);
            assert_eq!(
                (
                    data.sourcepos.start.line,
                    data.sourcepos.start.column,
                    data.sourcepos.end.line,
                    data.sourcepos.end.column
                ),
                (1, 1, 1, 15)
            );
        }
    }
    assert_eq!(texts, 1);
    (source, short, full)
}

#[tokio::test]
async fn formatted_literal_multi_scalar_entity_tokens_follow_longer_semantic_identity() {
    let (source, short, full) = multi_scalar_entity_preconditions();
    let config = config(RedactionStrategy::TokenReplace, &[short, full], &[]);
    let mut ordinary = ExtractedDocument {
        content: full.into(),
        ..Default::default()
    };
    crate::plugins::processor::builtin::redaction::RedactionProcessor
        .process(&mut ordinary, &config)
        .await
        .unwrap();
    assert_eq!(ordinary.content, "[PRIVATE_1]");
    assert_eq!(ordinary.redaction_report.as_ref().unwrap().total_redacted, 1);
    let (mut result, cache) = edited(source, Some(full), &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.redaction_report.as_ref().unwrap().total_redacted, 2);
    assert_eq!(output.content, "[PRIVATE_1]");
    assert_eq!(output.metadata.title.as_deref(), Some("[PRIVATE_1]"));
}

#[tokio::test]
async fn formatted_literal_multi_scalar_entity_hashes_follow_longer_semantic_identity() {
    let (source, short, full) = multi_scalar_entity_preconditions();
    let category = PiiCategory::Custom("private".into());
    let expected = apply_strategy(RedactionStrategy::Hash, full, &category, &mut TokenCounter::new());
    let wrong = apply_strategy(RedactionStrategy::Hash, short, &category, &mut TokenCounter::new());
    assert_ne!(expected, wrong);
    let config = config(RedactionStrategy::Hash, &[short, full], &[]);
    let mut ordinary = ExtractedDocument {
        content: full.into(),
        ..Default::default()
    };
    crate::plugins::processor::builtin::redaction::RedactionProcessor
        .process(&mut ordinary, &config)
        .await
        .unwrap();
    assert_eq!(ordinary.content, expected);
    assert_eq!(ordinary.redaction_report.as_ref().unwrap().total_redacted, 1);
    let (mut result, cache) = edited(source, Some(full), &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.redaction_report.as_ref().unwrap().total_redacted, 2);
    assert_eq!(output.metadata.title.as_deref(), Some(expected.as_str()));
    assert_eq!(output.content, expected);
}

#[tokio::test]
async fn formatted_literal_disjoint_entity_identities_abort_before_mutation() {
    let (source, short, full) = multi_scalar_entity_preconditions();
    let combining = "\u{0338}";
    let combining_match = regex::Regex::new(&regex::escape(combining))
        .unwrap()
        .find(full)
        .unwrap();
    assert_eq!((combining_match.start(), combining_match.end()), (3, 5));
    let config = config(RedactionStrategy::TokenReplace, &[short, combining], &[]);
    let terms = &config.redaction.as_ref().unwrap().custom_terms;
    assert_eq!(terms.len(), 2);
    assert_eq!((terms[0].value.as_str(), terms[1].value.as_str()), (short, combining));
    assert!(terms.iter().all(|term| term.value != full));
    let mut ordinary = ExtractedDocument {
        content: full.into(),
        ..Default::default()
    };
    crate::plugins::processor::builtin::redaction::RedactionProcessor
        .process(&mut ordinary, &config)
        .await
        .unwrap();
    assert_eq!(ordinary.content, "[PRIVATE_1][PRIVATE_2]");
    assert_eq!(ordinary.redaction_report.as_ref().unwrap().total_redacted, 2);
    let (mut result, cache) = edited(source, Some(full), &config).await;
    let before = serde_json::to_value(&*result).unwrap();
    let formatted = result.formatted_content.clone();
    let error = late(&mut result, &cache, &config)
        .await
        .expect_err("one entity envelope cannot collapse disjoint semantic identities");
    assert!(matches!(&error, XbergError::Plugin { plugin_name, .. } if plugin_name == "redaction"));
    assert!(error.to_string().contains("coverage"), "{error}");
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
    assert_eq!(result.formatted_content, formatted);
    assert!(result.redaction_report.is_none());
}

#[tokio::test]
async fn formatted_literal_disjoint_entity_mask_keeps_safe_physical_redaction() {
    let (source, short, full) = multi_scalar_entity_preconditions();
    let combining = "\u{0338}";
    let combining_match = regex::Regex::new(&regex::escape(combining))
        .unwrap()
        .find(full)
        .unwrap();
    assert_eq!((combining_match.start(), combining_match.end()), (3, 5));
    let config = config(RedactionStrategy::Mask, &[short, combining], &[]);
    let terms = &config.redaction.as_ref().unwrap().custom_terms;
    assert_eq!(terms.len(), 2);
    assert!(terms.iter().all(|term| term.value != full));
    let mut ordinary = ExtractedDocument {
        content: full.into(),
        ..Default::default()
    };
    crate::plugins::processor::builtin::redaction::RedactionProcessor
        .process(&mut ordinary, &config)
        .await
        .unwrap();
    assert_eq!(ordinary.content, "[REDACTED][REDACTED]");
    assert_eq!(ordinary.redaction_report.as_ref().unwrap().total_redacted, 2);
    let (mut result, cache) = edited(source, None, &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.content, "[REDACTED]");
    assert_eq!(output.redaction_report.as_ref().unwrap().total_redacted, 1);
}

async fn prefixed_entity_requires_all_semantic_identities(strategy: RedactionStrategy) {
    let source = "X&NotEqualTilde;";
    let first = "X≂";
    let second = "\u{0338}";
    let full = "X≂\u{0338}";
    assert_eq!((&source[..1], &source[1..], source.len()), ("X", "&NotEqualTilde;", 16));
    assert_eq!(visible(source), full);
    assert_eq!(
        (first.len(), second.len(), full.len(), full.chars().count()),
        (4, 2, 6, 3)
    );
    assert!(!source.contains(first) && !source.contains(second));
    let first_match = regex::Regex::new(&regex::escape(first)).unwrap().find(full).unwrap();
    let second_match = regex::Regex::new(&regex::escape(second)).unwrap().find(full).unwrap();
    assert_eq!((first_match.start(), first_match.end()), (0, 4));
    assert_eq!((second_match.start(), second_match.end()), (4, 6));
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, source, &crate::rendering::comrak_options());
    let mut texts = 0;
    for node in root.descendants() {
        let data = node.data.borrow();
        if let comrak::nodes::NodeValue::Text(value) = &data.value {
            texts += 1;
            let value: &str = value.as_ref();
            assert_eq!(value, full);
            assert_eq!(
                (
                    data.sourcepos.start.line,
                    data.sourcepos.start.column,
                    data.sourcepos.end.line,
                    data.sourcepos.end.column
                ),
                (1, 1, 1, 16)
            );
        }
    }
    assert_eq!(texts, 1);
    let config = config(strategy, &[first, second], &[]);
    assert_eq!(config.redaction.as_ref().unwrap().custom_terms.len(), 2);
    let category = PiiCategory::Custom("private".into());
    let mut counter = TokenCounter::new();
    let expected = format!(
        "{}{}",
        apply_strategy(strategy, first, &category, &mut counter),
        apply_strategy(strategy, second, &category, &mut counter)
    );
    let mut ordinary = ExtractedDocument {
        content: full.into(),
        ..Default::default()
    };
    crate::plugins::processor::builtin::redaction::RedactionProcessor
        .process(&mut ordinary, &config)
        .await
        .unwrap();
    assert_eq!(ordinary.content, expected);
    assert_eq!(ordinary.redaction_report.as_ref().unwrap().total_redacted, 2);
    let (mut result, cache) = edited(source, Some(full), &config).await;
    let before = serde_json::to_value(&*result).unwrap();
    let formatted = result.formatted_content.clone();
    let error = late(&mut result, &cache, &config)
        .await
        .expect_err("raw coverage must not discard a separate semantic identity in a skipped envelope");
    assert!(matches!(&error, XbergError::Plugin { plugin_name, .. } if plugin_name == "redaction"));
    assert!(error.to_string().contains("coverage"), "{error}");
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
    assert_eq!(result.formatted_content, formatted);
    assert!(result.redaction_report.is_none());
}

#[tokio::test]
async fn formatted_literal_prefixed_entity_tokens_reject_lost_semantic_identity() {
    prefixed_entity_requires_all_semantic_identities(RedactionStrategy::TokenReplace).await;
}

#[tokio::test]
async fn formatted_literal_prefixed_entity_hashes_reject_lost_semantic_identity() {
    prefixed_entity_requires_all_semantic_identities(RedactionStrategy::Hash).await;
}
