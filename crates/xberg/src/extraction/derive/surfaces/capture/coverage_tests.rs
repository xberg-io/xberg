use super::*;
use crate::core::config::redaction::{RedactionConfig, RedactionPattern, RedactionTerm};
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::plugins::processor::builtin::redaction::RedactionProcessor;
use crate::types::builder::{code, link, underline};
use crate::types::document_structure::TextAnnotation;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::redaction::{PiiCategory, RedactionFinding, RedactionStrategy};
use std::io::{Cursor, Read};

const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn config(term: &str, strategy: RedactionStrategy) -> RedactionConfig {
    RedactionConfig {
        strategy,
        custom_terms: vec![RedactionTerm::labeled("person", term)],
        ..Default::default()
    }
}

fn prepare(doc: InternalDocument) -> DerivedResult {
    let result = crate::extraction::derive::derive_with_surfaces(doc, false, OutputFormat::Markdown, true);
    let context = result.context.as_ref().expect("actual renderer created provenance");
    assert!(context.formatted_markdown());
    context.validate(&result).unwrap();
    assert!(!result.content.is_empty());
    assert!(!result.formatted_content.as_ref().unwrap().is_empty());
    result
}

fn paragraph(text: &str, annotations: Vec<TextAnnotation>) -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(text, annotations, None, None);
    let result = prepare(builder.build());
    assert_eq!(result.content, text);
    assert!(result.pages.is_none());
    result
}

fn visible(markdown: &str) -> String {
    let bytes = crate::rendering::render_docx(markdown).expect("actual canonical binary consumer");
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert!(archive.len() >= 6);
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let document = roxmltree::Document::parse(&xml).unwrap();
    let nodes = document
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "t")))
        .collect::<Vec<_>>();
    assert!(
        !nodes.is_empty(),
        "real selectable text, not a source-string projection"
    );
    nodes.iter().filter_map(|node| node.text()).collect()
}

async fn findings(text: &str, config: &RedactionConfig) -> Vec<RedactionFinding> {
    let mut document = ExtractedDocument {
        content: text.to_owned(),
        ..Default::default()
    };
    crate::text::redaction::engine::redact(&mut document, config)
        .await
        .unwrap();
    document
        .redaction_report
        .expect("real matcher and applicability pass ran")
        .findings
}

async fn apply(result: &mut DerivedResult, redaction: RedactionConfig) -> crate::Result<()> {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        redaction: Some(redaction),
        ..Default::default()
    };
    let (document, context) = result.redaction_parts()?;
    RedactionProcessor
        .process_with_surfaces(document, &config, context.unwrap())
        .await
}

fn finish(result: DerivedResult) -> ExtractedDocument {
    let (document, format) = result.into_output(OutputFormat::Markdown);
    crate::core::pipeline::apply_output_format(document, format)
}

fn offsets(findings: &[RedactionFinding]) -> Vec<(u32, u32)> {
    findings.iter().map(|finding| (finding.start, finding.end)).collect()
}

fn assert_plain(result: &ExtractedDocument, expected: &str) {
    assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
    assert!(result.formatted_content.is_none());
    assert_eq!(result.content, expected);
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "output_format"
                && warning.message.contains("rendering was discarded")
                && warning.message.contains("plain text"))
    );
}

async fn assert_count_trap(result: &DerivedResult, term: &str, expected: usize) {
    let markdown = result.formatted_content.as_ref().unwrap();
    let uri = format!("https://example.test/{term}");
    assert_eq!(markdown.matches(&uri).count(), 1);
    let start = markdown.find(&uri).unwrap();
    let raw = findings(markdown, &config(term, RedactionStrategy::Mask)).await;
    let rendered = visible(markdown);
    let semantic = findings(&rendered, &config(term, RedactionStrategy::Mask)).await;
    assert_eq!(raw.len(), expected);
    assert_eq!(semantic.len(), expected, "equal counts are not occurrence coverage");
    assert_eq!(rendered, result.content);
    assert!(!rendered.contains(&uri), "the extra raw match is not visible body text");
    assert_eq!(
        raw.iter()
            .filter(|finding| finding.start as usize >= start && (finding.end as usize) <= start + uri.len())
            .count(),
        1
    );
    assert!(
        raw.iter()
            .all(|finding| finding.category == PiiCategory::Custom("person".into()))
    );
}

#[tokio::test]
async fn equal_raw_and_visible_match_counts_cannot_cover_a_styled_occurrence_with_a_url() {
    let mut result = paragraph(
        "JaneDoe and JaneDoe open",
        vec![underline(16, 19), link(20, 24, "https://example.test/JaneDoe", None)],
    );
    assert_count_trap(&result, "JaneDoe", 2).await;
    assert_eq!(result.content.matches("JaneDoe").count(), 2);
    apply(&mut result, config("JaneDoe", RedactionStrategy::Mask))
        .await
        .unwrap();
    let result = finish(result);
    assert_plain(&result, "[REDACTED] and [REDACTED] open");
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 2);
}

#[tokio::test]
async fn repeated_identical_terms_keep_their_own_positions_in_coverage_proof() {
    let mut result = paragraph(
        "JaneDoe and JaneDoe then JaneDoe open",
        vec![underline(16, 19), link(33, 37, "https://example.test/JaneDoe", None)],
    );
    assert_count_trap(&result, "JaneDoe", 3).await;
    let before = findings(&result.content, &config("JaneDoe", RedactionStrategy::Mask)).await;
    assert_eq!(offsets(&before), [(0, 7), (12, 19), (25, 32)]);
    apply(&mut result, config("JaneDoe", RedactionStrategy::Mask))
        .await
        .unwrap();
    let result = finish(result);
    assert_plain(&result, "[REDACTED] and [REDACTED] then [REDACTED] open");
    assert_eq!(
        offsets(&result.redaction_report.as_ref().unwrap().findings),
        [(0, 7), (12, 19), (25, 32)]
    );
}

fn pages() -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("LEFT Jane Doe", vec![], Some(1), None);
    builder.push_paragraph("RIGHT Jane Doe and Jane Doe", vec![underline(24, 27)], Some(2), None);
    let result = prepare(builder.build());
    assert_eq!(result.content, "LEFT Jane Doe\n\nRIGHT Jane Doe and Jane Doe");
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages.iter().map(|page| page.page_number).collect::<Vec<_>>(), [1, 2]);
    let context = result.context.as_ref().unwrap();
    assert_eq!(context.page_backup(0), Some("LEFT Jane Doe"));
    assert_eq!(context.page_backup(1), Some("RIGHT Jane Doe and Jane Doe"));
    result
}

#[tokio::test]
async fn each_page_uses_only_its_own_plain_backup_and_original_selected_offsets() {
    let mut result = pages();
    let original_pages = result.pages.as_ref().unwrap();
    assert_eq!(visible(&original_pages[0].content), "LEFT Jane Doe");
    assert_eq!(visible(&original_pages[1].content), "RIGHT Jane Doe and Jane Doe");
    assert_eq!(
        findings(&original_pages[1].content, &config("Jane Doe", RedactionStrategy::Mask))
            .await
            .len(),
        1
    );
    apply(&mut result, config("Jane Doe", RedactionStrategy::Mask))
        .await
        .unwrap();
    let result = finish(result);
    assert_plain(&result, "LEFT [REDACTED]\n\nRIGHT [REDACTED] and [REDACTED]");
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages[0].content, "LEFT [REDACTED]\n");
    assert_eq!(pages[1].content, r"RIGHT \[REDACTED\] and \[REDACTED\]");
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 6);
    assert_eq!(
        offsets(&report.findings),
        [(5, 13), (21, 29), (34, 42), (5, 13), (6, 14), (19, 27)]
    );
}

#[tokio::test]
async fn visible_plain_whitespace_mismatch_aborts_before_any_result_or_report_mutation() {
    let mut result = paragraph("Jane Doe and Jane\nDoe end", vec![underline(18, 21)]);
    let config = config("Jane Doe", RedactionStrategy::TokenReplace);
    let markdown = result.formatted_content.as_ref().unwrap();
    assert_eq!(markdown.matches("<u>Doe</u>").count(), 1);
    assert_eq!(visible(markdown), "Jane Doe and Jane Doe end");
    assert_eq!(findings(&result.content, &config).await.len(), 1);
    assert_eq!(findings(markdown, &config).await.len(), 1);
    assert_eq!(findings(&visible(markdown), &config).await.len(), 2);
    assert!(result.redaction_report.is_none());
    let before = serde_json::to_value(&*result).unwrap();
    let outcome = apply(&mut result, config).await;
    assert_eq!(
        (
            matches!(outcome, Err(crate::XbergError::Plugin { .. })),
            serde_json::to_value(&*result).unwrap()
        ),
        (true, before),
        "uncovered visible text cannot spend or publish a partial result"
    );
}

fn token_source() -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(
        "JaneDoe and JaneDoe then AlderReed open",
        vec![
            underline(16, 19),
            link(35, 39, "https://example.test/CypressVale", None),
        ],
        None,
        None,
    );
    let mut document = builder.build();
    document.metadata.title = Some("CedarLane".into());
    let result = prepare(document);
    assert!(
        result.uris.is_none(),
        "the abandoned link destination belongs only to formatted output"
    );
    result
}

#[tokio::test]
async fn token_fallback_does_not_spend_findings_or_identity_on_abandoned_formatted_text() {
    let mut result = token_source();
    let mut config = config("JaneDoe", RedactionStrategy::TokenReplace);
    config
        .custom_terms
        .extend(["AlderReed", "CypressVale", "CedarLane"].map(|term| RedactionTerm::labeled("person", term)));
    let markdown = result.formatted_content.as_ref().unwrap();
    assert!(!markdown.contains("CedarLane"));
    assert_eq!(findings(&result.content, &config).await.len(), 3);
    assert_eq!(findings(markdown, &config).await.len(), 3);
    assert_eq!(visible(markdown), result.content);
    apply(&mut result, config).await.unwrap();
    let result = finish(result);
    assert_token_outcome(&result);
}

fn assert_token_outcome(result: &ExtractedDocument) {
    let report = result.redaction_report.as_ref().unwrap();
    let actual = (
        result.metadata.output_format.as_deref(),
        result.content.as_str(),
        result.metadata.title.as_deref(),
        report.total_redacted,
        report
            .findings
            .iter()
            .map(|finding| (finding.start, finding.end, finding.replacement_token.as_str()))
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        actual,
        (
            Some("plain"),
            "[PERSON_1] and [PERSON_1] then [PERSON_2] open",
            Some("[PERSON_3]"),
            4,
            vec![
                (0, 7, "[PERSON_1]"),
                (12, 19, "[PERSON_1]"),
                (25, 34, "[PERSON_2]"),
                (0, 9, "[PERSON_3]")
            ]
        )
    );
}

#[tokio::test]
async fn utf8_code_and_unknown_literal_markup_remain_redactable_data_without_style_loss() {
    let original = "é <sup>literal</sup> <mystery>Jane</mystery> Doe";
    let mut result = paragraph(original, vec![code(3, 21)]);
    let config = RedactionConfig {
        custom_terms: vec![
            RedactionTerm::labeled("syntax", "sup"),
            RedactionTerm::labeled("person", "Jane Doe"),
        ],
        ..Default::default()
    };
    let markdown = result.formatted_content.as_ref().unwrap();
    assert_eq!(visible(markdown), original);
    let raw = findings(markdown, &config).await;
    let semantic = findings(&visible(markdown), &config).await;
    assert_eq!(raw.len(), 2);
    assert_eq!(semantic.len(), 2);
    assert!(
        semantic
            .iter()
            .all(|finding| finding.category == PiiCategory::Custom("syntax".into()))
    );
    apply(&mut result, config).await.unwrap();
    assert!(result.formatted_content.is_some());
    assert_eq!(
        visible(result.formatted_content.as_ref().unwrap()),
        "é <[REDACTED]>literal</[REDACTED]> <mystery>Jane</mystery> Doe"
    );
}

#[tokio::test]
async fn overlapping_raw_fragment_matches_cannot_stand_in_for_complete_visible_matches() {
    let mut result = paragraph("Jane Doe and Jane Doe end", vec![underline(18, 21)]);
    let mut config = config("Jane Doe", RedactionStrategy::TokenReplace);
    config
        .custom_patterns
        .push(RedactionPattern::labeled("fragment", r"\bJane\b"));
    let markdown = result.formatted_content.as_ref().unwrap();
    assert_eq!(visible(markdown), result.content);
    let raw = findings(markdown, &config).await;
    let semantic = findings(&visible(markdown), &config).await;
    assert_eq!(raw.len(), 2);
    assert_eq!(semantic.len(), 2);
    assert_eq!(
        raw.iter().map(|finding| &finding.category).collect::<Vec<_>>(),
        [
            &PiiCategory::Custom("person".into()),
            &PiiCategory::Custom("fragment".into())
        ]
    );
    assert!(
        semantic
            .iter()
            .all(|finding| finding.category == PiiCategory::Custom("person".into()))
    );
    apply(&mut result, config).await.unwrap();
    let result = finish(result);
    assert_plain(&result, "[PERSON_1] and [PERSON_1] end");
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 2);
    assert_eq!(offsets(&report.findings), [(0, 8), (13, 21)]);
    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.replacement_token == "[PERSON_1]")
    );
}

fn source_code_block(text: &str) -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_code(text, Some("rust"), None, None);
    let document = builder.build();
    assert_eq!(document.elements.len(), 1);
    assert_eq!(document.elements[0].kind, crate::types::internal::ElementKind::Code);
    assert_eq!(document.elements[0].text, text);
    let result = prepare(document);
    assert_eq!(result.content, text);
    assert!(result.pages.is_none());
    let markdown = result.formatted_content.as_ref().unwrap();
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());
    let blocks = root
        .descendants()
        .filter_map(|node| {
            let data = node.data.borrow();
            if let comrak::nodes::NodeValue::CodeBlock(block) = &data.value {
                Some((block.fenced, block.info.clone(), block.literal.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(blocks, [(true, "rust".to_owned(), format!("{text}\n"))]);
    assert_eq!(visible(markdown), text);
    result
}

fn finished_code_text(result: &ExtractedDocument) -> String {
    match result.metadata.output_format.as_deref() {
        Some("markdown") => visible(&result.content),
        Some("plain") => {
            assert_plain(result, &result.content);
            result.content.clone()
        }
        other => panic!("unexpected code-block output route: {other:?}"),
    }
}

#[tokio::test]
async fn generated_safe_code_block_with_no_matches_remains_processable_without_content_loss() {
    let source = "let value = \"safe\";";
    let mut result = source_code_block(source);
    let config = config("Jane Doe", RedactionStrategy::Mask);
    let markdown = result.formatted_content.as_ref().unwrap();
    assert_eq!(findings(markdown, &config).await.len(), 0);
    assert_eq!(findings(&visible(markdown), &config).await.len(), 0);
    assert_eq!(findings(&result.content, &config).await.len(), 0);
    apply(&mut result, config)
        .await
        .expect("safe generated code is a supported redaction surface");
    let result = finish(result);
    assert_eq!(finished_code_text(&result), source);
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 0);
}

#[tokio::test]
async fn generated_sensitive_code_block_is_completely_redacted_without_exposing_its_literal() {
    let source = "let owner = \"Jane Doe\";";
    let mut result = source_code_block(source);
    let config = config("Jane Doe", RedactionStrategy::Mask);
    let markdown = result.formatted_content.as_ref().unwrap();
    assert_eq!(findings(markdown, &config).await.len(), 1);
    assert_eq!(findings(&visible(markdown), &config).await.len(), 1);
    assert_eq!(offsets(&findings(&result.content, &config).await), [(13, 21)]);
    apply(&mut result, config)
        .await
        .expect("verified code literals must be fully redacted");
    let result = finish(result);
    assert_eq!(finished_code_text(&result), "let owner = \"[REDACTED]\";");
    let expected_findings = if result.metadata.output_format.as_deref() == Some("plain") {
        1
    } else {
        2
    };
    assert_eq!(
        result.redaction_report.as_ref().unwrap().total_redacted,
        expected_findings
    );
    assert!(!result.content.contains("Jane Doe"));
}

mod compatibility_tests;
