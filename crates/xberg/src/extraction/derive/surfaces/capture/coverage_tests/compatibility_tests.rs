use super::*;

fn raw_html(text: &str) -> DerivedResult {
    let source = format!("<div>{text}</div>");
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_raw_block("html", &source, None);
    let document = builder.build();
    assert_eq!(document.elements.len(), 1);
    assert_eq!(document.elements[0].kind, crate::types::internal::ElementKind::RawBlock);
    assert_eq!(document.elements[0].text, source);
    let result = prepare(document);
    assert_eq!(result.content, source);
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(
        &arena,
        result.formatted_content.as_ref().unwrap(),
        &comrak::Options::default(),
    );
    let blocks = root
        .descendants()
        .filter_map(|node| {
            if let comrak::nodes::NodeValue::HtmlBlock(block) = &node.data.borrow().value {
                Some(block.literal.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(blocks, [format!("{source}\n")]);
    assert_eq!(visible(result.formatted_content.as_ref().unwrap()), source);
    result
}

async fn redact_literal(
    mut result: DerivedResult,
    config: RedactionConfig,
    raw: usize,
    semantic: usize,
    expected: &str,
) {
    let markdown = result.formatted_content.as_ref().unwrap();
    assert_eq!(findings(markdown, &config).await.len(), raw);
    assert_eq!(findings(&visible(markdown), &config).await.len(), semantic);
    assert_eq!(findings(&result.content, &config).await.len(), semantic);
    apply(&mut result, config)
        .await
        .expect("canonical literal data must remain processable");
    let result = finish(result);
    assert_eq!(finished_code_text(&result), expected);
    let expected_findings = if result.metadata.output_format.as_deref() == Some("plain") {
        semantic
    } else {
        raw + semantic
    };
    assert_eq!(
        result.redaction_report.as_ref().unwrap().total_redacted as usize,
        expected_findings
    );
}

#[tokio::test]
async fn canonical_safe_raw_html_block_remains_literal_processable_data() {
    let result = raw_html("safe literal");
    let config = config("Jane Doe", RedactionStrategy::Mask);
    redact_literal(result, config, 0, 0, "<div>safe literal</div>").await;
}

#[tokio::test]
async fn canonical_sensitive_raw_html_block_redacts_content_without_interpreting_tags() {
    let result = raw_html("Jane Doe");
    let config = config("Jane Doe", RedactionStrategy::Mask);
    redact_literal(result, config, 1, 1, "<div>[REDACTED]</div>").await;
}

fn entity_literal(text: &str) -> DerivedResult {
    let result = paragraph(text, vec![]);
    let markdown = result.formatted_content.as_ref().unwrap();
    assert_eq!(
        markdown.matches(r"\&amp;").count(),
        1,
        "canonical Normal text escapes ampersand before alphabetic data: {markdown:?}"
    );
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());
    let leaves = root
        .descendants()
        .filter_map(|node| {
            if let comrak::nodes::NodeValue::Text(text) = &node.data.borrow().value {
                Some(text.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(leaves.concat(), text);
    assert_eq!(visible(markdown), text);
    result
}

#[tokio::test]
async fn canonical_safe_entity_shaped_text_keeps_its_literal_data() {
    let result = entity_literal("before safe &amp; literal after");
    redact_literal(
        result,
        config("Jane &amp; Doe", RedactionStrategy::Mask),
        0,
        0,
        "before safe &amp; literal after",
    )
    .await;
}

#[tokio::test]
async fn canonical_sensitive_entity_shaped_text_redacts_the_exact_visible_literal() {
    let result = entity_literal("before Jane &amp; Doe after");
    redact_literal(
        result,
        config("Jane &amp; Doe", RedactionStrategy::Mask),
        0,
        1,
        "before [REDACTED] after",
    )
    .await;
}

fn padded_code(literal: &str) -> DerivedResult {
    assert!(literal.starts_with('`') && literal.ends_with('`'));
    let source = format!("before {literal} after");
    let end = u32::try_from("before ".len() + literal.len()).unwrap();
    let result = paragraph(&source, vec![code(7, end)]);
    let markdown = result.formatted_content.as_ref().unwrap();
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());
    let spans = root
        .descendants()
        .filter_map(|node| {
            let data = node.data.borrow();
            if let comrak::nodes::NodeValue::Code(code) = &data.value {
                assert_eq!(data.sourcepos.start.line, 1);
                assert_eq!(data.sourcepos.end.line, 1);
                let start = data.sourcepos.start.column - 1 + code.num_backticks;
                let end = data.sourcepos.end.column - code.num_backticks;
                Some((
                    code.num_backticks,
                    code.literal.clone(),
                    markdown[start..end].to_string(),
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(spans, [(2, literal.to_owned(), format!(" {literal} "))]);
    assert_eq!(visible(markdown), source);
    result
}

#[tokio::test]
async fn canonical_safe_inline_code_with_delimiter_padding_remains_processable() {
    let result = padded_code("`safe literal`");
    redact_literal(
        result,
        config("Jane Doe", RedactionStrategy::Mask),
        0,
        0,
        "before `safe literal` after",
    )
    .await;
}

#[tokio::test]
async fn canonical_sensitive_inline_code_with_delimiter_padding_is_fully_redacted() {
    let result = padded_code("`Jane Doe`");
    redact_literal(
        result,
        config("Jane Doe", RedactionStrategy::Mask),
        1,
        1,
        "before `[REDACTED]` after",
    )
    .await;
}
