use xberg::core::config::ExtractionConfig;
use xberg::extractors::djot_format::DjotExtractor;
use xberg::plugins::InternalDocumentExtractor;
use xberg::types::internal::ElementKind;

fn markdown_footnote_text(content: &str) -> String {
    let arena = comrak::Arena::new();
    let mut options = comrak::Options::default();
    options.extension.footnotes = true;
    let document = comrak::parse_document(&arena, content, &options);
    let definitions: Vec<_> = document
        .descendants()
        .filter(|node| {
            matches!(
                node.data.borrow().value,
                comrak::nodes::NodeValue::FootnoteDefinition(_)
            )
        })
        .collect();
    assert_eq!(definitions.len(), 1, "{}", content);
    definitions[0]
        .descendants()
        .filter_map(|node| match &node.data.borrow().value {
            comrak::nodes::NodeValue::Text(text) => Some(text.to_string()),
            _ => None,
        })
        .collect()
}

async fn footnote_text(input: &[u8]) -> String {
    let document = DjotExtractor
        .extract_content(input, "text/djot", &ExtractionConfig::default())
        .await
        .expect("the Djot footnote should extract");
    let definitions: Vec<_> = document
        .elements
        .iter()
        .filter(|element| element.kind == ElementKind::FootnoteDefinition)
        .collect();
    assert_eq!(definitions.len(), 1, "got {:?}", document.elements);
    definitions[0].text.clone()
}

#[tokio::test]
async fn should_separate_paragraphs_inside_a_djot_footnote() {
    let input = b"Text[^n].\n\n[^n]: para one\n\n    para two\n";
    assert_eq!(footnote_text(input).await, "para one\n\npara two");
}

#[tokio::test]
async fn should_separate_a_heading_inside_a_djot_footnote() {
    let input = b"Text[^n].\n\n[^n]: intro\n\n    # Heading\n";
    assert_eq!(footnote_text(input).await, "intro\n\nHeading");
}

#[tokio::test]
async fn should_separate_list_items_inside_a_djot_footnote() {
    let input = b"Text[^n].\n\n[^n]: intro\n\n    - first\n    - second\n";
    assert_eq!(footnote_text(input).await, "intro\n\nfirst\n\nsecond");
}

#[tokio::test]
async fn should_separate_definition_terms_inside_a_djot_footnote() {
    let input = b"Text[^n].\n\n[^n]: intro\n\n    : Term\n\n      detail\n";
    assert_eq!(footnote_text(input).await, "intro\n\nTerm\n\ndetail");
}

#[tokio::test]
async fn should_separate_a_table_caption_inside_a_djot_footnote() {
    let input = b"Text[^n].\n\n[^n]: intro\n\n    | a |\n\n    ^ cap one\n    cap two\n";
    let events: Vec<_> = jotdown::Parser::new(std::str::from_utf8(input).expect("valid UTF-8")).collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, jotdown::Event::Start(jotdown::Container::Caption, _)))
    );
    assert_eq!(footnote_text(input).await, "intro\n\ncap one cap two");
}

#[tokio::test]
async fn should_separate_nested_list_items_inside_a_djot_footnote() {
    let input = b"Text[^n].\n\n[^n]: intro\n\n    - first\n\n      - nested\n\n    - second\n";
    let events: Vec<_> = jotdown::Parser::new(std::str::from_utf8(input).expect("valid UTF-8")).collect();
    let items = events
        .iter()
        .filter(|event| matches!(event, jotdown::Event::Start(jotdown::Container::ListItem, _)))
        .count();
    assert_eq!(items, 3, "the fixture must contain three parsed list items");
    assert_eq!(footnote_text(input).await, "intro\n\nfirst\n\nnested\n\nsecond");
}

#[tokio::test]
async fn should_keep_inline_formatting_inside_one_djot_footnote_block() {
    let input = b"Text[^n].\n\n[^n]: first *bold* and _italic_ [link](https://example.com) last\n";
    assert_eq!(footnote_text(input).await, "first bold and italic link last");
}

#[tokio::test]
async fn should_keep_soft_and_hard_breaks_inside_one_djot_footnote_block() {
    let cases: [(&[u8], &str); 2] = [
        (b"Text[^n].\n\n[^n]: first\n    second\n", "first second"),
        (b"Text[^n].\n\n[^n]: first\\\n    second\n", "first\nsecond"),
    ];
    for (input, expected) in cases {
        assert_eq!(footnote_text(input).await, expected);
    }
}

#[tokio::test]
async fn should_leave_normal_djot_paragraphs_unchanged() {
    let document = DjotExtractor
        .extract_content(b"para one\n\npara two\n", "text/djot", &ExtractionConfig::default())
        .await
        .expect("ordinary paragraphs should extract");
    let paragraphs: Vec<_> = document
        .elements
        .iter()
        .map(|element| (element.kind, element.text.as_str()))
        .collect();
    assert_eq!(
        paragraphs,
        [
            (ElementKind::Paragraph, "para one"),
            (ElementKind::Paragraph, "para two")
        ]
    );
}

#[tokio::test]
async fn should_keep_adjacent_footnote_text_apart_in_public_markdown_output() {
    let config = ExtractionConfig {
        output_format: xberg::core::config::OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    };
    let input = xberg::ExtractInput::from_bytes(
        b"Text[^n].\n\n[^n]: para one\n\n    para two\n".to_vec(),
        "text/djot",
        None,
    );
    let mut output = xberg::extract(input, &config)
        .await
        .expect("public extraction should succeed");
    assert_eq!(output.errors.len(), 0);
    assert_eq!(output.results.len(), 1);
    let result = output.results.remove(0);
    let text = markdown_footnote_text(&result.content);
    assert_eq!(text, "para one para two", "{}", result.content);
}

#[tokio::test]
async fn should_render_a_nested_table_caption_once_inside_its_footnote() {
    let config = ExtractionConfig {
        output_format: xberg::core::config::OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    };
    let input = xberg::ExtractInput::from_bytes(
        b"Text[^n].\n\n[^n]: intro\n\n    | a |\n\n    ^ cap one\n    cap two\n".to_vec(),
        "text/djot",
        None,
    );
    let mut output = xberg::extract(input, &config)
        .await
        .expect("public extraction succeeds");
    assert_eq!(output.errors.len(), 0);
    assert_eq!(output.results.len(), 1);
    let result = output.results.remove(0);
    assert_eq!(markdown_footnote_text(&result.content), "intro cap one cap two");
    assert_eq!(
        result.content.matches("cap one cap two").count(),
        1,
        "{}",
        result.content
    );
}
