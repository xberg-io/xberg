//! Frontmatter remains metadata rather than becoming document body text.

#![cfg(all(feature = "tokio-runtime", feature = "chunking"))]

use serde_json::json;
use xberg::core::config::{ChunkingConfig, OutputFormat};
use xberg::types::ResultFormat;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, extract};

const BODY: &str = "# Body heading\n\nBody paragraph.\n";
const YAML: &str = r#"---
title: Frontmatter title
draft: false
weight: 10
tags: [rust, parsing]
settings:
  enabled: true
  counts: [1, 2]
items: [{label: one, active: false}]
---

"#;
const TOML: &str = r#"+++
title = 'Frontmatter title'
draft = false
weight = 10
tags = ['rust', 'parsing']
settings = { enabled = true, counts = [1, 2] }
items = [{ label = 'one', active = false }]
+++

"#;

async fn extract_document(
    source: &str,
    mime_type: &str,
    output_format: OutputFormat,
    downstream: bool,
) -> ExtractedDocument {
    let mut config = ExtractionConfig {
        output_format,
        ..Default::default()
    };
    if downstream {
        config.result_format = ResultFormat::ElementBased;
        config.chunking = Some(ChunkingConfig {
            max_characters: 2048,
            overlap: 0,
            ..Default::default()
        });
    }
    let mut output = extract(ExtractInput::from_bytes(source.as_bytes(), mime_type, None), &config)
        .await
        .expect("actual public extraction succeeds");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    assert_eq!(output.results.len(), 1);
    let document = output.results.remove(0);
    assert_eq!(document.mime_type, mime_type);
    assert!(!document.content.is_empty());
    document
}

fn assert_metadata(document: &ExtractedDocument) {
    assert_eq!(document.metadata.title.as_deref(), Some("Frontmatter title"));
    assert_eq!(
        document.metadata.tags.as_deref(),
        Some([String::from("rust"), String::from("parsing")].as_slice())
    );
    assert_eq!(document.metadata.additional["draft"], json!(false));
    assert_eq!(document.metadata.additional["weight"], json!(10));
    assert_eq!(
        document.metadata.additional["settings"],
        json!({"enabled": true, "counts": [1, 2]})
    );
    assert_eq!(
        document.metadata.additional["items"],
        json!([{"label": "one", "active": false}])
    );
}

async fn assert_body_content(frontmatter: &str, format: OutputFormat, mime_type: &str) {
    assert!(frontmatter.starts_with("---\n") || frontmatter.starts_with("+++\n"));
    let source = format!("{frontmatter}{BODY}");
    assert!(source.ends_with(BODY));
    let observed = extract_document(&source, mime_type, format, false).await;
    let body = extract_document(BODY, mime_type, format, false).await;
    assert_metadata(&observed);
    assert_eq!(body.metadata.title.as_deref(), Some("Body heading"));
    assert!(body.content.contains("Body heading"));
    assert!(body.content.contains("Body paragraph."));
    assert_eq!(
        observed.content, body.content,
        "frontmatter must not become rendered body"
    );
}

fn element_texts(document: &ExtractedDocument) -> Vec<&str> {
    let elements = document
        .elements
        .as_ref()
        .expect("actual element extraction is enabled");
    assert!(!elements.is_empty());
    elements.iter().map(|element| element.text.as_str()).collect()
}

fn chunk_texts(document: &ExtractedDocument) -> Vec<&str> {
    let chunks = document.chunks.as_ref().expect("actual chunking is enabled");
    assert!(!chunks.is_empty());
    chunks.iter().map(|chunk| chunk.content.as_str()).collect()
}

async fn assert_body_downstream(frontmatter: &str, format: OutputFormat) {
    let source = format!("{frontmatter}{BODY}");
    let observed = extract_document(&source, "text/markdown", format, true).await;
    let body = extract_document(BODY, "text/markdown", format, true).await;
    assert_metadata(&observed);
    let expected_elements = element_texts(&body);
    let expected_chunks = chunk_texts(&body);
    assert!(expected_elements.join("\n").contains("Body heading"));
    assert!(expected_elements.join("\n").contains("Body paragraph."));
    assert!(expected_chunks.join("\n").contains("Body heading"));
    assert!(expected_chunks.join("\n").contains("Body paragraph."));
    assert_eq!(
        element_texts(&observed),
        expected_elements,
        "frontmatter must not become a body element"
    );
    assert_eq!(
        chunk_texts(&observed),
        expected_chunks,
        "frontmatter must not enter body chunks"
    );
    assert_eq!(observed.content, body.content);
}

macro_rules! frontmatter_cases {
    ($content:ident, $downstream:ident, $frontmatter:expr, $format:expr) => {
        #[tokio::test]
        async fn $content() {
            assert_body_content($frontmatter, $format, "text/markdown").await;
        }

        #[tokio::test]
        async fn $downstream() {
            assert_body_downstream($frontmatter, $format).await;
        }
    };
}

frontmatter_cases!(
    yaml_plain_retains_metadata_without_body_frontmatter,
    yaml_plain_excludes_frontmatter_from_elements_and_chunks,
    YAML,
    OutputFormat::Plain
);
frontmatter_cases!(
    yaml_markdown_retains_metadata_without_body_frontmatter,
    yaml_markdown_excludes_frontmatter_from_elements_and_chunks,
    YAML,
    OutputFormat::Markdown
);
frontmatter_cases!(
    toml_plain_retains_metadata_without_body_frontmatter,
    toml_plain_excludes_frontmatter_from_elements_and_chunks,
    TOML,
    OutputFormat::Plain
);
frontmatter_cases!(
    toml_markdown_retains_metadata_without_body_frontmatter,
    toml_markdown_excludes_frontmatter_from_elements_and_chunks,
    TOML,
    OutputFormat::Markdown
);

#[tokio::test]
async fn djot_yaml_plain_already_keeps_frontmatter_out_of_body() {
    assert_body_content(YAML, OutputFormat::Plain, "text/x-djot").await;
}

#[tokio::test]
async fn djot_toml_markdown_already_keeps_frontmatter_out_of_body() {
    assert_body_content(TOML, OutputFormat::Markdown, "text/x-djot").await;
}

async fn assert_literal_debug_body(format: OutputFormat) {
    let literal = "Body marker Bool(false) Number(10).";
    let document = extract_document(literal, "text/markdown", format, false).await;
    assert!(document.metadata.title.is_none());
    assert_eq!(document.content.trim(), literal);
}

#[tokio::test]
async fn plain_preserves_literal_debug_like_body_text() {
    assert_literal_debug_body(OutputFormat::Plain).await;
}

#[tokio::test]
async fn markdown_preserves_literal_debug_like_body_text() {
    assert_literal_debug_body(OutputFormat::Markdown).await;
}
