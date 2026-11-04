use super::*;
use crate::core::config::redaction::{RedactionConfig, RedactionPattern, RedactionTerm};
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::plugins::processor::builtin::redaction::RedactionProcessor;
use crate::types::builder::{subscript, superscript};
use crate::types::document_structure::{AnnotationKind, TextAnnotation};
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::redaction::RedactionStrategy;

fn source(text: &str, annotations: Vec<TextAnnotation>) -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(text, annotations, Some(1), None);
    let result = crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Markdown, true);
    assert_eq!(result.pages.as_ref().unwrap().len(), 1);
    assert!(result.context.as_ref().unwrap().page_markdown(0));
    result
}

fn term(strategy: RedactionStrategy) -> RedactionConfig {
    RedactionConfig {
        strategy,
        custom_terms: vec![RedactionTerm::labeled("private", "u")],
        ..Default::default()
    }
}

async fn apply(result: &mut DerivedResult, redaction: RedactionConfig) -> crate::Result<()> {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        redaction: Some(redaction),
        ..Default::default()
    };
    let (document, surfaces) = result.redaction_parts()?;
    match surfaces {
        Some(surfaces) => {
            RedactionProcessor
                .process_with_surfaces(document, &config, surfaces)
                .await
        }
        None => crate::plugins::PostProcessor::process(&RedactionProcessor, document, &config).await,
    }
}

fn underlined() -> DerivedResult {
    source(
        "éunder secret",
        vec![TextAnnotation {
            start: 2,
            end: 7,
            kind: AnnotationKind::Underline,
        }],
    )
}

#[tokio::test]
async fn mask_preserves_generated_syntax_and_reports_only_content() {
    let mut result = underlined();
    assert_eq!(result.formatted_content.as_deref(), Some("é<u>under</u> secret\n"));
    apply(&mut result, term(RedactionStrategy::Mask)).await.unwrap();
    assert_eq!(result.content, "é[REDACTED]nder secret");
    assert_eq!(
        result.formatted_content.as_deref(),
        Some("é<u>[REDACTED]nder</u> secret\n")
    );
    assert_eq!(
        result.pages.as_ref().unwrap()[0].content,
        "é<u>[REDACTED]nder</u> secret\n"
    );
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 3);
    assert_eq!(
        report.findings.iter().map(|f| (f.start, f.end)).collect::<Vec<_>>(),
        [(2, 3), (5, 6), (5, 6)]
    );
}

#[tokio::test]
async fn tokens_reuse_body_identity_without_allocating_for_tag_names() {
    let mut result = underlined();
    apply(&mut result, term(RedactionStrategy::TokenReplace)).await.unwrap();
    assert_eq!(result.content, "é[PRIVATE_1]nder secret");
    assert_eq!(
        result.formatted_content.as_deref(),
        Some("é<u>[PRIVATE_1]nder</u> secret\n")
    );
    assert_eq!(
        result.pages.as_ref().unwrap()[0].content,
        "é<u>[PRIVATE_1]nder</u> secret\n"
    );
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 3);
    assert!(report.findings.iter().all(|f| f.replacement_token == "[PRIVATE_1]"));
}

fn overlap(strategy: RedactionStrategy) -> RedactionConfig {
    RedactionConfig {
        strategy,
        custom_terms: vec![RedactionTerm::labeled("private", "safe")],
        custom_patterns: vec![RedactionPattern::labeled("private", "secret(?:</sup>)?")],
        ..Default::default()
    }
}

async fn assert_overlap(strategy: RedactionStrategy, expected: &str) {
    let text = "secret <u>literal</u> & [safe]";
    let mut result = source(text, vec![superscript(0, 6)]);
    apply(&mut result, overlap(strategy)).await.unwrap();
    assert!(
        result.formatted_content.is_none(),
        "discard the entire serialized document plan"
    );
    assert_eq!(result.content, expected);
    let page = &result.pages.as_ref().unwrap()[0];
    let html = comrak::markdown_to_html(&page.content, &comrak::Options::default());
    assert!(
        !html.contains("<u>") && html.contains("&lt;u&gt;literal&lt;/u&gt;"),
        "{html}"
    );
    assert!(!html.contains("secret") && !html.contains("safe"));
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(
        report.total_redacted, 4,
        "document plain + chosen page plain, no discarded Markdown findings"
    );
    assert_eq!(
        report.findings.iter().map(|f| (f.start, f.end)).collect::<Vec<_>>(),
        [(0, 6), (25, 29), (0, 6), (25, 29)]
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|w| w.source == "output_format" && w.message.contains("rendering was discarded"))
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|w| w.source == "output_format" && w.message.contains("before escaping"))
    );
    let (document, format) = result.into_output(OutputFormat::Markdown);
    let finished = crate::core::pipeline::apply_output_format(document, format);
    assert_eq!(finished.metadata.output_format.as_deref(), Some("plain"));
    assert_eq!(finished.content, expected);
    assert!(finished.formatted_content.is_none());
}

#[tokio::test]
async fn mask_overlap_selects_whole_plain_fallback_and_escapes_literal_markup() {
    assert_overlap(RedactionStrategy::Mask, "[REDACTED] <u>literal</u> & [[REDACTED]]").await;
}

#[tokio::test]
async fn token_overlap_allocates_only_chosen_plain_replacements() {
    assert_overlap(
        RedactionStrategy::TokenReplace,
        "[PRIVATE_1] <u>literal</u> & [[PRIVATE_2]]",
    )
    .await;
}

#[tokio::test]
async fn missing_plain_backup_aborts_before_any_field_is_redacted() {
    let mut result = source("secret safe", vec![superscript(0, 6)]);
    result.context.as_mut().unwrap().capture.pages[0].plain = None;
    let before = serde_json::to_value(&*result).unwrap();
    let error = apply(&mut result, overlap(RedactionStrategy::TokenReplace))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("no verified plain backup"));
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
    assert!(result.redaction_report.is_none());
}

#[tokio::test]
async fn stale_rendered_preimages_and_page_identity_fail_before_mutation() {
    for route in ["formatted", "page_text", "page_number"] {
        let mut result = underlined();
        match route {
            "formatted" => result.formatted_content.as_mut().unwrap().push('!'),
            "page_text" => result.pages.as_mut().unwrap()[0].content.push('!'),
            _ => result.pages.as_mut().unwrap()[0].page_number = 2,
        }
        let before = serde_json::to_value(&*result).unwrap();
        let error = apply(&mut result, term(RedactionStrategy::Mask)).await.unwrap_err();
        assert!(error.to_string().contains("provenance changed"), "{route}: {error}");
        assert_eq!(serde_json::to_value(&*result).unwrap(), before);
    }
}

#[test]
fn derived_results_cannot_exchange_local_surface_contexts() {
    let mut first = underlined();
    let mut second = source("different", vec![subscript(0, 1)]);
    std::mem::swap(&mut first.context, &mut second.context);
    assert!(matches!(first.redaction_parts(), Err(crate::XbergError::Plugin { .. })));
    assert!(matches!(
        second.redaction_parts(),
        Err(crate::XbergError::Plugin { .. })
    ));
}

fn literal_page(number: u32) -> crate::types::PageContent {
    serde_json::from_value(
        serde_json::json!({"page_number": number, "content": "<sup>literal</sup>",
        "tables": [], "image_indices": []}),
    )
    .unwrap()
}

#[tokio::test]
async fn unmatched_plain_pages_remain_redaction_data() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("ordinary", vec![], Some(1), None);
    let mut doc = builder.build();
    doc.prebuilt_pages = Some(vec![literal_page(2)]);
    let mut result = crate::extraction::derive::derive_with_surfaces(doc, false, OutputFormat::Markdown, true);
    assert!(!result.context.as_ref().unwrap().page_markdown(0));
    let config = RedactionConfig {
        custom_terms: vec![RedactionTerm::labeled("private", "sup")],
        ..Default::default()
    };
    apply(&mut result, config).await.unwrap();
    assert_eq!(
        result.pages.as_ref().unwrap()[0].content,
        "<[REDACTED]>literal</[REDACTED]>"
    );
}

#[test]
fn empty_page_render_does_not_forge_markdown_provenance() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("", vec![], Some(1), None);
    let mut doc = builder.build();
    doc.prebuilt_pages = Some(vec![literal_page(1)]);
    assert_eq!(crate::rendering::render_markdown(&doc), "");
    let result = crate::extraction::derive::derive_with_surfaces(doc, false, OutputFormat::Markdown, true);
    assert_eq!(result.pages.as_ref().unwrap()[0].content, "<sup>literal</sup>");
    assert!(!result.context.as_ref().unwrap().page_markdown(0));
}

#[tokio::test]
async fn generated_page_context_never_classifies_a_nested_plain_page() {
    let mut result = underlined();
    let child = ExtractedDocument {
        content: "<sup>literal</sup>".into(),
        pages: Some(vec![literal_page(1)]),
        ..Default::default()
    };
    result.children = Some(vec![crate::types::ArchiveEntry {
        path: "child".into(),
        mime_type: "text/plain".into(),
        result: Box::new(child),
    }]);
    let config = RedactionConfig {
        custom_terms: vec![RedactionTerm::labeled("private", "sup")],
        ..Default::default()
    };
    apply(&mut result, config).await.unwrap();
    let child = &result.children.as_ref().unwrap()[0].result;
    assert_eq!(child.content, "<[REDACTED]>literal</[REDACTED]>");
    assert_eq!(
        child.pages.as_ref().unwrap()[0].content,
        "<[REDACTED]>literal</[REDACTED]>"
    );
    assert_eq!(result.pages.as_ref().unwrap()[0].content, "é<u>under</u> secret\n");
}

#[test]
fn plain_output_does_not_capture_markdown_surface_context() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("secret", vec![superscript(0, 6)], Some(1), None);
    let mut result = crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Plain, true);
    assert!(result.context.is_none());
    assert!(result.identity.is_none());
    result.content = "processed plain".into();
    assert!(result.redaction_parts().unwrap().1.is_none());
}

#[tokio::test]
async fn pre_rendered_text_cannot_claim_generated_markdown_from_metadata() {
    let mut doc = crate::types::internal::InternalDocument::new("test");
    doc.pre_rendered_content = Some("<sup>literal</sup>".into());
    doc.metadata.output_format = Some("markdown".into());
    let mut result = crate::extraction::derive::derive_with_surfaces(doc, false, OutputFormat::Markdown, true);
    assert!(result.context.is_none());
    assert_eq!(result.formatted_content.as_deref(), Some("<sup>literal</sup>"));
    apply(
        &mut result,
        RedactionConfig {
            custom_terms: vec![RedactionTerm::labeled("private", "sup")],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(result.content, "<[REDACTED]>literal</[REDACTED]>");
    assert_eq!(
        result.formatted_content.as_deref(),
        Some("<[REDACTED]>literal</[REDACTED]>")
    );
}

#[tokio::test]
async fn page_fallback_preserves_newlines_and_reports_plain_utf8_byte_offsets() {
    let original = "secret\né [safe]\n<u>literal</u>";
    let mut result = source(original, vec![superscript(0, 6)]);
    assert_eq!(result.content, original);
    assert_eq!(result.content.matches('\n').count(), 2);
    apply(&mut result, overlap(RedactionStrategy::Mask)).await.unwrap();
    assert_eq!(result.content, "[REDACTED]\né [[REDACTED]]\n<u>literal</u>");
    assert_eq!(
        result.pages.as_ref().unwrap()[0].content,
        r"\[REDACTED\]
é \[\[REDACTED\]\]
\<u\>literal\<\/u\>"
    );
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 4);
    assert_eq!(
        report
            .findings
            .iter()
            .map(|finding| (finding.start, finding.end))
            .collect::<Vec<_>>(),
        [(0, 6), (11, 15), (0, 6), (11, 15)]
    );
}

#[tokio::test]
async fn metadata_and_container_only_placeholders_preserve_the_no_body_literal_protocol() {
    let empty = crate::types::internal::InternalDocument::new("test");
    let mut metadata = crate::types::internal::InternalDocument::new("test");
    metadata.metadata.title = Some("non-body placeholder".into());
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_list(false);
    builder.end_list();
    let containers = builder.build();
    assert_eq!(containers.elements.len(), 2);
    let mut results = Vec::new();
    for mut doc in [empty, metadata, containers] {
        assert_eq!(crate::rendering::render_plain(&doc), "");
        doc.pre_rendered_content = Some("<sup>literal</sup>".into());
        doc.metadata.output_format = Some("markdown".into());
        let result = crate::extraction::derive::derive_with_surfaces(doc, false, OutputFormat::Markdown, true);
        assert_eq!(result.content, "<sup>literal</sup>");
        results.push(result);
    }
    let contexts = results
        .iter()
        .map(|result| (result.context.is_some(), result.identity.is_some()))
        .collect::<Vec<_>>();
    assert_eq!(contexts, [(false, false), (false, false), (false, false)]);
    for mut result in results {
        apply(
            &mut result,
            RedactionConfig {
                custom_terms: vec![RedactionTerm::labeled("private", "sup")],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(result.content, "<[REDACTED]>literal</[REDACTED]>");
        assert_eq!(
            result.formatted_content.as_deref(),
            Some("<[REDACTED]>literal</[REDACTED]>")
        );
        assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 4);
    }
}
