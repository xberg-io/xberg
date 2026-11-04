use super::*;

fn html_document(text: &str, page: Option<u32>) -> DerivedResult {
    let start = text.rfind("Doe").unwrap() as u32;
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(text, vec![underline(start, start + 3)], page, None);
    crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Html, true)
}

async fn apply_html(result: &mut DerivedResult, redaction: RedactionConfig) -> crate::Result<()> {
    let configuration = ExtractionConfig {
        output_format: OutputFormat::Html,
        redaction: Some(redaction),
        ..Default::default()
    };
    let (document, context) = result.redaction_parts()?;
    match context {
        Some(context) => {
            RedactionProcessor
                .process_with_surfaces(document, &configuration, context)
                .await
        }
        None => {
            RedactionProcessor
                .process_with_surfaces(document, &configuration, &())
                .await
        }
    }
}

fn finish_html(result: DerivedResult) -> ExtractedDocument {
    let (document, format) = result.into_output(OutputFormat::Html);
    crate::core::pipeline::apply_output_format(document, format)
}

fn html_text(html: &str) -> String {
    let tree = roxmltree::Document::parse(html).expect("actual valid escaped HTML");
    tree.descendants()
        .filter(|node| node.is_text())
        .filter_map(|node| node.text())
        .collect()
}

#[tokio::test]
async fn generated_html_page_covers_both_occurrences_and_reconciles_ranges_2361() {
    let text = "Jane Doe and Jane Doe end";
    let rule = config("Jane Doe", RedactionStrategy::Mask);
    let selected = findings(text, &rule).await;
    assert_eq!(selected.len(), 2);
    let mut result = html_document(text, Some(1));
    assert_eq!(result.pages.as_ref().unwrap().len(), 1);
    apply_html(&mut result, rule).await.unwrap();
    let result = finish_html(result);
    assert_eq!(result.metadata.output_format.as_deref(), Some("html"));
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(html_text(&result.content), "[REDACTED] and [REDACTED] end");
    assert_eq!(html_text(&pages[0].content), "[REDACTED] and [REDACTED] end");
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 4);
    assert_eq!(offsets(&report.findings[..2]), offsets(&selected));
    assert_eq!(offsets(&report.findings[2..]), offsets(&selected));
    if let Some(boundaries) = result
        .metadata
        .pages
        .as_ref()
        .and_then(|pages| pages.boundaries.as_ref())
    {
        assert_eq!(boundaries.len(), 1);
        let boundary = &boundaries[0];
        assert_eq!(
            result.content.get(boundary.byte_start..boundary.byte_end),
            Some(pages[0].content.as_str())
        );
    }
}

#[tokio::test]
async fn html_rebuild_does_not_spend_tokens_on_abandoned_link_destinations_2361() {
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
    let mut result = crate::extraction::derive::derive_with_surfaces(document, false, OutputFormat::Html, true);
    assert!(result.formatted_content.as_ref().unwrap().contains("CypressVale"));
    assert!(result.uris.is_none());
    let mut rule = config("JaneDoe", RedactionStrategy::TokenReplace);
    rule.custom_terms
        .extend(["AlderReed", "CypressVale", "CedarLane"].map(|term| RedactionTerm::labeled("person", term)));
    apply_html(&mut result, rule).await.unwrap();
    let result = finish_html(result);
    assert_eq!(
        html_text(&result.content),
        "[PERSON_1] and [PERSON_1] then [PERSON_2] open"
    );
    assert_eq!(result.metadata.title.as_deref(), Some("[PERSON_3]"));
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 4);
    assert_eq!(
        report
            .findings
            .iter()
            .map(|finding| (finding.start, finding.end, finding.replacement_token.as_str()))
            .collect::<Vec<_>>(),
        [
            (0, 7, "[PERSON_1]"),
            (12, 19, "[PERSON_1]"),
            (25, 34, "[PERSON_2]"),
            (0, 9, "[PERSON_3]")
        ]
    );
}

#[tokio::test]
async fn unchanged_untrusted_html_hashes_do_not_grant_source_authority_2361() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("Jane Doe and Jane Doe end", vec![], None, None);
    let mut document = builder.build();
    document.pre_rendered_content = Some("<p>Jane Doe and Jane <u>Doe</u> end</p>".into());
    document.metadata.output_format = Some("html".into());
    let mut result = crate::extraction::derive::derive_with_surfaces(document, false, OutputFormat::Html, true);
    let before = serde_json::to_value(&*result).unwrap();
    let outcome = apply_html(&mut result, config("Jane Doe", RedactionStrategy::Mask)).await;
    assert!(matches!(outcome, Err(crate::XbergError::Plugin { .. })));
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
}

#[tokio::test]
async fn html_hash_mutation_is_rejected_even_when_visible_text_is_unchanged_2361() {
    let mut result = html_document("Jane Doe and Jane Doe end", None);
    result.formatted_content = Some("<section>Jane Doe and Jane <u>Doe</u> end</section>".into());
    let before = serde_json::to_value(&*result).unwrap();
    let outcome = apply_html(&mut result, config("Jane Doe", RedactionStrategy::Mask)).await;
    assert!(matches!(outcome, Err(crate::XbergError::Plugin { .. })));
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
}

#[tokio::test]
async fn serde_cloning_html_cannot_reconstruct_private_renderer_authority_2361() {
    let result = html_document("Jane Doe and Jane Doe end", None);
    let mut clone: ExtractedDocument = serde_json::from_value(serde_json::to_value(&*result).unwrap()).unwrap();
    clone.metadata.output_format = Some("html".into());
    let before = serde_json::to_value(&clone).unwrap();
    let configuration = ExtractionConfig {
        output_format: OutputFormat::Html,
        redaction: Some(config("Jane Doe", RedactionStrategy::Mask)),
        ..Default::default()
    };
    let outcome = crate::plugins::PostProcessor::process(&RedactionProcessor, &mut clone, &configuration).await;
    assert!(matches!(outcome, Err(crate::XbergError::Plugin { .. })));
    assert_eq!(serde_json::to_value(&clone).unwrap(), before);
}

#[tokio::test]
async fn html_nul_normalization_is_rejected_before_mutation_or_audit_2361() {
    let mut result = html_document("Jane Doe and Jane Doe\0 end", None);
    let before = serde_json::to_value(&*result).unwrap();
    let outcome = apply_html(&mut result, config("Jane Doe", RedactionStrategy::Mask)).await;
    assert!(matches!(outcome, Err(crate::XbergError::Plugin { .. })));
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
}

#[tokio::test]
async fn built_in_html_with_unmodeled_raw_body_is_rejected_before_redaction_2361() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("Jane Doe end", vec![], None, None);
    builder.push_raw_block("html", "<p>Jane <u>Doe</u></p>", None);
    let mut result = crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Html, true);
    assert!(result.formatted_content.as_deref().unwrap().contains("Jane <u>Doe</u>"));
    let before = serde_json::to_value(&*result).unwrap();
    let error = apply_html(&mut result, config("Jane Doe", RedactionStrategy::Mask))
        .await
        .unwrap_err();
    assert!(matches!(error, crate::XbergError::Plugin { .. }));
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
}

#[tokio::test]
async fn html_token_label_cannot_insert_an_unrepresentable_nul_2361() {
    let mut result = html_document("Jane Doe and Jane Doe end", None);
    let before = serde_json::to_value(&*result).unwrap();
    let mut rule = config("Jane Doe", RedactionStrategy::TokenReplace);
    rule.custom_terms = vec![RedactionTerm::labeled("unsafe\0label", "Jane Doe")];
    let error = apply_html(&mut result, rule).await.unwrap_err();
    assert!(matches!(error, crate::XbergError::Plugin { .. }));
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
}

#[tokio::test]
async fn public_redact_rejects_a_serialized_html_result_without_renderer_authority() {
    let result = html_document("Jane Doe and Jane Doe end", None);
    let mut clone: ExtractedDocument = serde_json::from_value(serde_json::to_value(&*result).unwrap()).unwrap();
    clone.metadata.output_format = Some("html".into());
    let before = serde_json::to_value(&clone).unwrap();
    let error = crate::text::redaction::redact(&mut clone, &config("Jane Doe", RedactionStrategy::Mask))
        .await
        .unwrap_err();
    assert!(matches!(error, crate::XbergError::Validation { .. }));
    assert_eq!(serde_json::to_value(&clone).unwrap(), before);
    assert!(clone.redaction_report.is_none());
}

fn serialized_parent_with_nested_text(html: bool, image: bool) -> ExtractedDocument {
    let mut child = ExtractedDocument {
        content: if html {
            "<p>Jane <u>Doe</u> end</p>"
        } else {
            "Jane Doe end"
        }
        .into(),
        ..Default::default()
    };
    child.metadata.output_format = Some(if html { "html" } else { "plain" }.into());
    let mut parent = ExtractedDocument {
        content: "Jane Doe parent".into(),
        ..Default::default()
    };
    parent.metadata.output_format = Some("plain".into());
    if image {
        parent.images = Some(vec![crate::types::ExtractedImage {
            ocr_result: Some(Box::new(child)),
            ..Default::default()
        }]);
    } else {
        parent.children = Some(vec![crate::types::ArchiveEntry {
            path: "doc".into(),
            mime_type: "text/plain".into(),
            result: Box::new(child),
        }]);
    }
    serde_json::from_value(serde_json::to_value(parent).unwrap()).unwrap()
}

async fn reject_unproved_nested_html(image: bool) {
    let mut parent = serialized_parent_with_nested_text(true, image);
    let before = serde_json::to_value(&parent).unwrap();
    let error = crate::text::redaction::redact(&mut parent, &config("Jane Doe", RedactionStrategy::Mask))
        .await
        .unwrap_err();
    assert!(matches!(error, crate::XbergError::Validation { .. }));
    assert_eq!(serde_json::to_value(&parent).unwrap(), before);
    assert!(parent.redaction_report.is_none());
}

#[tokio::test]
async fn unproved_nested_archive_html_is_rejected_transactionally() {
    reject_unproved_nested_html(false).await;
}

#[tokio::test]
async fn unproved_nested_image_ocr_html_is_rejected_transactionally() {
    reject_unproved_nested_html(true).await;
}

#[tokio::test]
async fn nested_plain_children_keep_existing_literal_redaction() {
    for image in [false, true] {
        let mut parent = serialized_parent_with_nested_text(false, image);
        crate::text::redaction::redact(&mut parent, &config("Jane Doe", RedactionStrategy::Mask))
            .await
            .unwrap();
        assert_eq!(parent.content, "[REDACTED] parent");
        let child = if image {
            parent.images.as_ref().unwrap()[0].ocr_result.as_deref().unwrap()
        } else {
            &*parent.children.as_ref().unwrap()[0].result
        };
        assert_eq!(child.content, "[REDACTED] end");
        assert_eq!(parent.redaction_report.unwrap().total_redacted, 2);
    }
}

async fn limited_html(result: &mut DerivedResult, max_iterations: usize) -> crate::Result<()> {
    let mut redaction = config("x", RedactionStrategy::Mask);
    redaction
        .categories
        .insert(crate::types::redaction::PiiCategory::Person);
    apply_limited_html(result, redaction, max_iterations).await
}

async fn apply_limited_html(
    result: &mut DerivedResult,
    redaction: RedactionConfig,
    max_iterations: usize,
) -> crate::Result<()> {
    let configuration = ExtractionConfig {
        output_format: OutputFormat::Html,
        redaction: Some(redaction),
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_iterations,
            ..Default::default()
        }),
        ..Default::default()
    };
    let (document, surfaces) = result.redaction_parts().unwrap();
    RedactionProcessor
        .process_with_surfaces(document, &configuration, surfaces.unwrap())
        .await
}

#[tokio::test]
async fn dense_html_match_work_fails_before_mutation_or_findings() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(&"x ".repeat(80), vec![], None, None);
    let mut result = crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Html, true);
    let before = serde_json::to_value(&*result).unwrap();
    let error = limited_html(&mut result, 450).await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("HTML redaction exceeds the verified work or output size limit")
    );
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);

    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(&format!("x {}", "y ".repeat(79)), vec![], None, None);
    let mut sparse = crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Html, true);
    assert_eq!(sparse.content.len(), result.content.len());
    limited_html(&mut sparse, 450).await.unwrap();
    assert_eq!(sparse.redaction_report.as_ref().unwrap().total_redacted, 1);
}

#[tokio::test]
async fn html_root_and_pages_share_a_redaction_work_budget() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("x x x x x x x x end", vec![], Some(1), None);
    builder.push_paragraph("x x x x x x x x end", vec![], Some(2), None);
    let doc = builder.build();
    let mut pageless = doc.clone();
    for element in &mut pageless.elements {
        element.page = None;
    }
    let mut root = crate::extraction::derive::derive_with_surfaces(pageless, false, OutputFormat::Html, true);
    limited_html(&mut root, 190).await.unwrap();
    assert_eq!(root.redaction_report.as_ref().unwrap().total_redacted, 16);
    let mut result = crate::extraction::derive::derive_with_surfaces(doc, false, OutputFormat::Html, true);
    assert_eq!(result.pages.as_ref().unwrap().len(), 2);
    let before = serde_json::to_value(&*result).unwrap();
    let error = limited_html(&mut result, 190).await.unwrap_err();
    assert!(matches!(error, crate::XbergError::Plugin { .. }));
    assert_eq!(serde_json::to_value(&*result).unwrap(), before);
}

#[tokio::test]
async fn arbitrary_html_regex_rescan_spends_suffix_work_before_redaction() {
    let source = "A".repeat(64);
    let run = || {
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph(&source, vec![], None, None);
        crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Html, true)
    };
    let rule = |pattern| {
        let mut config = config("unmatched", RedactionStrategy::Mask);
        config.custom_terms.clear();
        config.categories.insert(crate::types::redaction::PiiCategory::Person);
        config.custom_patterns = vec![crate::core::config::redaction::RedactionPattern::labeled(
            "private", pattern,
        )];
        config
    };

    let mut simple = run();
    apply_limited_html(&mut simple, rule("A+"), 700_000).await.unwrap();
    assert_eq!(
        simple.formatted_content.as_deref(),
        Some("<pre><span>[REDACTED]</span></pre>")
    );
    let report = simple.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 1);
    assert_eq!(offsets(&report.findings), [(0, 64)]);

    let mut adversarial = run();
    let before = serde_json::to_value(&*adversarial).unwrap();
    let error = apply_limited_html(&mut adversarial, rule(".*[^A-Z]|[A-Z]"), 700_000)
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("HTML redaction exceeds the verified work or output size limit")
    );
    assert_eq!(serde_json::to_value(&*adversarial).unwrap(), before);
}

#[tokio::test]
async fn ordinary_custom_html_patterns_work_on_a_multi_kilobyte_body_at_default_limits() {
    let prefix = "public context ".repeat(256);
    let source = format!("{prefix}employee-42 tail");
    assert_eq!(source.len(), 3856);
    for (pattern, expected, expected_ranges) in [
        ("absent-[0-9]+", source.clone(), vec![]),
        (
            "employee-[0-9]+",
            format!("{prefix}[REDACTED] tail"),
            vec![(3840, 3851)],
        ),
    ] {
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph(&source, vec![], None, None);
        let mut result =
            crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Html, true);
        let mut rule = config("unmatched", RedactionStrategy::Mask);
        rule.custom_terms.clear();
        rule.custom_patterns = vec![crate::core::config::redaction::RedactionPattern::labeled(
            "private", pattern,
        )];
        apply_html(&mut result, rule).await.unwrap();
        let output = finish_html(result);
        assert_eq!(output.metadata.output_format.as_deref(), Some("html"));
        assert_eq!(html_text(&output.content), expected);
        let report = output.redaction_report.as_ref().unwrap();
        assert_eq!(report.total_redacted as usize, expected_ranges.len());
        assert_eq!(offsets(&report.findings), expected_ranges);
    }
}
