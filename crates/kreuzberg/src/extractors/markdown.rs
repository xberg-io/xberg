//! Enhanced Markdown extractor with YAML frontmatter support.
//!
//! This extractor provides:
//! - Comprehensive markdown parsing using pulldown-cmark
//! - Complete YAML frontmatter metadata extraction:
//!   - Standard fields: title, author, date, description, keywords
//!   - Extended fields: abstract, subject, category, tags, language, version
//! - Automatic conversion of array fields (keywords, tags) to comma-separated strings
//! - Table extraction as structured data
//! - Heading structure preservation
//! - Code block and link extraction
//! - Data URI image extraction
//!
//! Requires the `office` feature (which includes `pulldown-cmark`).

#[cfg(feature = "office")]
use super::frontmatter_utils::{
    cells_to_markdown, extract_frontmatter, extract_metadata_from_yaml, extract_title_from_content,
};
#[cfg(feature = "office")]
use crate::Result;
#[cfg(feature = "office")]
use crate::core::config::ExtractionConfig;
#[cfg(feature = "office")]
use crate::plugins::{DocumentExtractor, Plugin};
#[cfg(feature = "office")]
use crate::types::{ExtractedImage, ExtractionResult, Metadata, Table};
#[cfg(feature = "office")]
use async_trait::async_trait;
#[cfg(feature = "office")]
use base64::Engine;
#[cfg(feature = "office")]
use bytes::Bytes;
#[cfg(feature = "office")]
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
#[cfg(feature = "office")]
use std::borrow::Cow;

/// Enhanced Markdown extractor with metadata and table support.
///
/// Parses markdown documents with YAML frontmatter, extracting:
/// - Metadata from YAML frontmatter
/// - Plain text content
/// - Tables as structured data
/// - Document structure (headings, links, code blocks)
/// - Images from data URIs
#[cfg(feature = "office")]
pub struct MarkdownExtractor;

#[cfg(feature = "office")]
impl MarkdownExtractor {
    /// Create a new Markdown extractor.
    pub fn new() -> Self {
        Self
    }

    // Frontmatter utilities moved to shared frontmatter_utils module

    /// Extract plain text from markdown AST, collecting data URI images.
    fn extract_text_from_events(events: &[Event], images: &mut Vec<ExtractedImage>) -> String {
        let mut text = String::new();
        let mut link_url: Option<String> = None;
        let mut in_heading = false;

        for event in events {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    text.push('\n');
                    let prefix = match *level {
                        pulldown_cmark::HeadingLevel::H1 => "# ",
                        pulldown_cmark::HeadingLevel::H2 => "## ",
                        pulldown_cmark::HeadingLevel::H3 => "### ",
                        pulldown_cmark::HeadingLevel::H4 => "#### ",
                        pulldown_cmark::HeadingLevel::H5 => "##### ",
                        pulldown_cmark::HeadingLevel::H6 => "###### ",
                    };
                    text.push_str(prefix);
                    in_heading = true;
                }
                Event::End(TagEnd::Heading(_)) => {
                    text.push('\n');
                    in_heading = false;
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    link_url = Some(dest_url.to_string());
                }
                Event::End(TagEnd::Link) => {
                    if let Some(url) = link_url.take()
                        && !url.is_empty()
                        && !url.starts_with('#')
                    {
                        text.push_str(" (");
                        text.push_str(&url);
                        text.push(')');
                    }
                }
                Event::Start(Tag::Image { dest_url, .. }) => {
                    text.push_str("[Image");
                    if !dest_url.is_empty() {
                        text.push_str(": ");
                        text.push_str(dest_url);
                    }
                    text.push(']');
                    // Extract image from data URIs
                    if dest_url.starts_with("data:image/")
                        && let Some(image) = Self::decode_data_uri_image(dest_url, images.len())
                    {
                        images.push(image);
                    }
                }
                Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(lang))) => {
                    text.push('\n');
                    text.push_str("```");
                    if !lang.is_empty() {
                        text.push_str(lang);
                    }
                    text.push('\n');
                }
                Event::End(TagEnd::CodeBlock) => {
                    text.push_str("```\n");
                }
                Event::Start(Tag::BlockQuote(_)) => {
                    text.push_str("\n> ");
                }
                Event::Start(Tag::Paragraph) => {
                    if !in_heading {
                        text.push('\n');
                    }
                }
                Event::End(TagEnd::Paragraph) => {
                    text.push('\n');
                }
                Event::Text(s) | Event::Code(s) | Event::Html(s) => {
                    text.push_str(s);
                }
                Event::SoftBreak | Event::HardBreak => {
                    text.push('\n');
                }
                Event::TaskListMarker(checked) => {
                    text.push_str(if *checked { "[x] " } else { "[ ] " });
                }
                Event::FootnoteReference(s) => {
                    text.push('[');
                    text.push_str(s);
                    text.push(']');
                }
                Event::Rule => {
                    text.push_str("\n---\n");
                }
                _ => {}
            }
        }
        text
    }

    /// Decode a data URI into an `ExtractedImage`.
    ///
    /// Supports base64-encoded PNG, JPEG, GIF, and WebP data URIs.
    /// Returns `None` for non-base64 encodings or unsupported formats.
    fn decode_data_uri_image(uri: &str, index: usize) -> Option<ExtractedImage> {
        let after_data = uri.strip_prefix("data:")?;
        let (mime_and_encoding, data) = after_data.split_once(',')?;

        if !mime_and_encoding.contains("base64") {
            return None;
        }

        let format: &str = if mime_and_encoding.contains("image/png") {
            "png"
        } else if mime_and_encoding.contains("image/jpeg") {
            "jpeg"
        } else if mime_and_encoding.contains("image/gif") {
            "gif"
        } else if mime_and_encoding.contains("image/webp") {
            "webp"
        } else {
            return None;
        };

        let cleaned = data.replace(['\n', '\r'], "");
        let decoded = base64::engine::general_purpose::STANDARD.decode(&cleaned).ok()?;

        Some(ExtractedImage {
            data: Bytes::from(decoded),
            format: Cow::Borrowed(format),
            image_index: index,
            page_number: None,
            width: None,
            height: None,
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
        })
    }

    /// Extract tables from markdown AST.
    fn extract_tables_from_events(events: &[Event]) -> Vec<Table> {
        let mut tables = Vec::new();
        let mut current_table: Option<(Vec<Vec<String>>, usize)> = None;
        let mut current_row: Vec<String> = Vec::new();
        let mut current_cell = String::new();
        let mut in_table_cell = false;
        let mut table_index = 0;

        for event in events {
            match event {
                Event::Start(Tag::Table(_)) => {
                    current_table = Some((Vec::new(), table_index));
                }
                Event::Start(Tag::TableHead) => {}
                Event::Start(Tag::TableRow) => {
                    current_row = Vec::new();
                }
                Event::Start(Tag::TableCell) => {
                    current_cell = String::new();
                    in_table_cell = true;
                }
                Event::Text(s) if in_table_cell => {
                    current_cell.push_str(s);
                }
                Event::Code(s) if in_table_cell => {
                    current_cell.push_str(s);
                }
                Event::End(TagEnd::TableCell) => {
                    if in_table_cell {
                        current_row.push(current_cell.trim().to_string());
                        current_cell = String::new();
                        in_table_cell = false;
                    }
                }
                Event::End(TagEnd::TableHead) => {
                    if !current_row.is_empty()
                        && let Some((ref mut rows, _)) = current_table
                    {
                        rows.push(std::mem::take(&mut current_row));
                    }
                    current_row = Vec::new();
                }
                Event::End(TagEnd::TableRow) => {
                    if !current_row.is_empty()
                        && let Some((ref mut rows, _)) = current_table
                    {
                        rows.push(std::mem::take(&mut current_row));
                    }
                    current_row = Vec::new();
                }
                Event::End(TagEnd::Table) => {
                    if let Some((cells, idx)) = current_table.take()
                        && !cells.is_empty()
                    {
                        let markdown = cells_to_markdown(&cells);
                        tables.push(Table {
                            cells,
                            markdown,
                            page_number: idx + 1,
                        });
                        table_index += 1;
                    }
                }
                _ => {}
            }
        }

        tables
    }

    // cells_to_markdown and extract_title_from_content moved to shared frontmatter_utils module
}

#[cfg(feature = "office")]
impl Default for MarkdownExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
impl Plugin for MarkdownExtractor {
    fn name(&self) -> &str {
        "markdown-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content from Markdown files with YAML frontmatter and table support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[async_trait]
impl DocumentExtractor for MarkdownExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining_content) = extract_frontmatter(&text);

        let mut metadata = if let Some(ref yaml_value) = yaml {
            extract_metadata_from_yaml(yaml_value)
        } else {
            Metadata::default()
        };

        if !metadata.additional.contains_key("title")
            && let Some(title) = extract_title_from_content(&remaining_content)
        {
            metadata.additional.insert(Cow::Borrowed("title"), title.into());
        }

        let parser = Parser::new_ext(&remaining_content, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();

        let mut extracted_images = Vec::new();
        let extracted_text = Self::extract_text_from_events(&events, &mut extracted_images);

        let tables = Self::extract_tables_from_events(&events);

        let images = if !extracted_images.is_empty() {
            #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
            {
                let processed =
                    crate::extraction::image_ocr::process_images_with_ocr(extracted_images, _config).await?;
                Some(processed)
            }
            #[cfg(not(all(feature = "ocr", feature = "tokio-runtime")))]
            {
                Some(extracted_images)
            }
        } else {
            None
        };

        Ok(ExtractionResult {
            content: extracted_text,
            mime_type: mime_type.to_string().into(),
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images,
            djot_content: None,
            pages: None,
            elements: None,
            ocr_elements: None,
            document: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "text/markdown",
            "text/x-markdown",
            "text/x-gfm",
            "text/x-commonmark",
            "text/x-markdown-extra",
            "text/x-multimarkdown",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::super::frontmatter_utils::{cells_to_markdown, extract_frontmatter, extract_metadata_from_yaml};
    use super::*;
    use serde_yaml_ng::Value as YamlValue;

    #[test]
    fn test_can_extract_markdown_mime_types() {
        let extractor = MarkdownExtractor::new();
        let mime_types = extractor.supported_mime_types();

        assert!(mime_types.contains(&"text/markdown"));
        assert!(mime_types.contains(&"text/x-markdown"));
        assert!(mime_types.contains(&"text/x-gfm"));
        assert!(mime_types.contains(&"text/x-commonmark"));
        assert!(mime_types.contains(&"text/x-markdown-extra"));
        assert!(mime_types.contains(&"text/x-multimarkdown"));
    }

    #[test]
    fn test_extract_simple_markdown() {
        let content =
            b"# Header\n\nThis is a paragraph with **bold** and *italic* text.\n\n## Subheading\n\nMore content here.";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());
        assert!(!remaining.is_empty());

        let parser = Parser::new_ext(&remaining, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events, &mut Vec::new());

        assert!(extracted.contains("Header"));
        assert!(extracted.contains("This is a paragraph"));
        assert!(extracted.contains("bold"));
        assert!(extracted.contains("italic"));
    }

    #[test]
    fn test_extract_frontmatter_metadata() {
        let content = b"---\ntitle: My Document\nauthor: John Doe\ndate: 2024-01-15\nkeywords: rust, markdown, extraction\ndescription: A test document\n---\n\n# Content\n\nBody text.";

        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml_opt, remaining) = extract_frontmatter(&text);
        assert!(yaml_opt.is_some());
        assert!(remaining.contains("# Content"));

        let yaml = yaml_opt.expect("Should extract YAML frontmatter");
        let metadata = extract_metadata_from_yaml(&yaml);

        assert_eq!(
            metadata
                .additional
                .get("title")
                .and_then(|v: &serde_json::Value| v.as_str()),
            Some("My Document")
        );
        assert_eq!(
            metadata
                .additional
                .get("author")
                .and_then(|v: &serde_json::Value| v.as_str()),
            Some("John Doe")
        );
        assert_eq!(metadata.created_at, Some("2024-01-15".to_string()));
        assert!(metadata.subject.is_some());
        assert!(
            metadata
                .subject
                .as_ref()
                .expect("Should have subject description")
                .contains("test document")
        );
    }

    #[test]
    fn test_extract_frontmatter_metadata_array_keywords() {
        let content = b"---\ntitle: Document\nkeywords:\n  - rust\n  - markdown\n  - parsing\n---\n\nContent";

        let text = String::from_utf8_lossy(content).into_owned();
        let (yaml_opt, _remaining) = extract_frontmatter(&text);

        assert!(yaml_opt.is_some());
        let yaml = yaml_opt.expect("Should extract YAML frontmatter");
        let metadata = extract_metadata_from_yaml(&yaml);

        let keywords = metadata
            .additional
            .get("keywords")
            .and_then(|v: &serde_json::Value| v.as_str());
        assert!(keywords.is_some());
        let keywords_str = keywords.expect("Should extract keywords from metadata");
        assert!(keywords_str.contains("rust"));
        assert!(keywords_str.contains("markdown"));
    }

    #[tokio::test]
    async fn test_extract_tables() {
        let content = b"# Tables Example\n\n| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |\n| Cell 3   | Cell 4   |";

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_bytes(content, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown with tables");

        assert!(!result.tables.is_empty());
        let table = &result.tables[0];
        assert!(!table.cells.is_empty());
        assert_eq!(table.cells[0].len(), 2);
        assert!(!table.markdown.is_empty());
    }

    #[test]
    fn test_extract_without_frontmatter() {
        let content = b"# Main Title\n\nSome content\n\nMore text";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());
        assert_eq!(remaining, text);

        let title = extract_title_from_content(&remaining);
        assert_eq!(title, Some("Main Title".to_string()));
    }

    #[test]
    fn test_empty_document() {
        let content = b"";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());
        assert!(remaining.is_empty());

        let parser = Parser::new_ext(&remaining, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events, &mut Vec::new());
        assert!(extracted.is_empty());
    }

    #[test]
    fn test_whitespace_only_document() {
        let content = b"   \n\n  \n";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());

        let parser = Parser::new_ext(&remaining, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events, &mut Vec::new());
        assert!(extracted.trim().is_empty());
    }

    #[test]
    fn test_unicode_content() {
        let content = "# 日本語のタイトル\n\nこれは日本語の内容です。\n\n## Español\n\nEste es un documento en español.\n\n## Русский\n\nЭто русский текст.".as_bytes();

        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());

        let parser = Parser::new_ext(&remaining, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events, &mut Vec::new());

        assert!(extracted.contains("日本語"));
        assert!(extracted.contains("Español"));
        assert!(extracted.contains("Русский"));
    }

    #[tokio::test]
    async fn test_full_extraction_with_frontmatter_and_tables() {
        let content = b"---\ntitle: Complete Document\nauthor: Test Author\ndate: 2024-01-20\n---\n\n# Document\n\nIntroduction text.\n\n| Name | Value |\n|------|-------|\n| A    | 1     |\n| B    | 2     |";

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_bytes(content, "text/x-markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown with frontmatter and tables");

        assert_eq!(result.mime_type, "text/x-markdown");
        assert!(result.content.contains("Introduction text"));
        assert_eq!(
            result.metadata.additional.get("title").and_then(|v| v.as_str()),
            Some("Complete Document")
        );
        assert_eq!(
            result.metadata.additional.get("author").and_then(|v| v.as_str()),
            Some("Test Author")
        );
        assert!(!result.tables.is_empty());
    }

    #[test]
    fn test_plugin_interface() {
        let extractor = MarkdownExtractor::new();
        assert_eq!(extractor.name(), "markdown-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(extractor.supported_mime_types().contains(&"text/markdown"));
    }

    #[test]
    fn test_cells_to_markdown() {
        let cells = vec![
            vec!["Header 1".to_string(), "Header 2".to_string()],
            vec!["Data 1".to_string(), "Data 2".to_string()],
            vec!["Data 3".to_string(), "Data 4".to_string()],
        ];

        let markdown = cells_to_markdown(&cells);
        assert!(markdown.contains("Header 1"));
        assert!(markdown.contains("Data 1"));
        assert!(markdown.contains("---"));
        let lines: Vec<&str> = markdown.lines().collect();
        assert!(lines.len() >= 4);
    }

    #[test]
    fn test_extract_markdown_with_links() {
        let content = b"# Page\n\nCheck [Google](https://google.com) and [Rust](https://rust-lang.org).";
        let text = String::from_utf8_lossy(content).into_owned();

        let parser = Parser::new_ext(&text, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events, &mut Vec::new());

        assert!(extracted.contains("Google"));
        assert!(extracted.contains("Rust"));
    }

    #[test]
    fn test_extract_markdown_with_code_blocks() {
        let content = b"# Code Example\n\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let text = String::from_utf8_lossy(content).into_owned();

        let parser = Parser::new_ext(&text, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events, &mut Vec::new());

        assert!(extracted.contains("main"));
        assert!(extracted.contains("println"));
    }

    #[test]
    fn test_malformed_frontmatter_fallback() {
        let content = b"---\nthis: is: invalid: yaml:\n---\n\nContent here";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, _remaining) = extract_frontmatter(&text);
        let _ = yaml;
    }

    #[test]
    fn test_metadata_extraction_completeness() {
        let yaml_str = r#"
title: "Test Document"
author: "Test Author"
date: "2024-01-15"
keywords:
  - rust
  - markdown
  - testing
description: "A test description"
abstract: "Test abstract"
subject: "Test subject"
category: "Documentation"
version: "1.2.3"
language: "en"
tags:
  - tag1
  - tag2
custom_field: "custom_value"
nested:
  organization: "Test Corp"
  contact:
    email: "test@example.com"
"#;

        let yaml: YamlValue = serde_yaml_ng::from_str(yaml_str).expect("Valid YAML");
        let metadata = extract_metadata_from_yaml(&yaml);

        assert_eq!(metadata.created_at, Some("2024-01-15".to_string()));
        assert_eq!(
            metadata.additional.get("title").and_then(|v| v.as_str()),
            Some("Test Document")
        );
        assert_eq!(
            metadata.additional.get("author").and_then(|v| v.as_str()),
            Some("Test Author")
        );

        assert!(metadata.additional.contains_key("keywords"));
        let keywords = metadata
            .additional
            .get("keywords")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(keywords.contains("rust"));
        assert!(keywords.contains("markdown"));

        assert_eq!(metadata.subject, Some("Test subject".to_string()));

        assert_eq!(
            metadata.additional.get("abstract").and_then(|v| v.as_str()),
            Some("Test abstract")
        );

        assert_eq!(
            metadata.additional.get("category").and_then(|v| v.as_str()),
            Some("Documentation")
        );

        assert!(metadata.additional.contains_key("tags"));
        let tags = metadata.additional.get("tags").and_then(|v| v.as_str()).unwrap_or("");
        assert!(tags.contains("tag1"));
        assert!(tags.contains("tag2"));

        assert_eq!(metadata.additional.get("language").and_then(|v| v.as_str()), Some("en"));

        assert_eq!(
            metadata.additional.get("version").and_then(|v| v.as_str()),
            Some("1.2.3")
        );

        assert_eq!(metadata.additional.len(), 8, "Should extract all standard fields");
        println!("\nSuccessfully extracted all 8 additional metadata fields");
    }

    #[test]
    fn test_decode_data_uri_png() {
        // 1x1 red PNG pixel (minimal valid PNG)
        let png_b64 =
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let uri = format!("data:image/png;base64,{png_b64}");

        let image = MarkdownExtractor::decode_data_uri_image(&uri, 0);
        assert!(image.is_some());
        let img = image.unwrap();
        assert_eq!(img.format.as_ref(), "png");
        assert_eq!(img.image_index, 0);
        assert!(!img.data.is_empty());
    }

    #[test]
    fn test_decode_data_uri_jpeg() {
        // Minimal JPEG-like base64 (tests the decode path)
        let uri = "data:image/jpeg;base64,/9j/4AAQSkZJRg==";

        let image = MarkdownExtractor::decode_data_uri_image(uri, 3);
        assert!(image.is_some());
        let img = image.unwrap();
        assert_eq!(img.format.as_ref(), "jpeg");
        assert_eq!(img.image_index, 3);
    }

    #[test]
    fn test_decode_data_uri_unsupported_format() {
        let uri = "data:image/tiff;base64,AAAA";
        let image = MarkdownExtractor::decode_data_uri_image(uri, 0);
        assert!(image.is_none());
    }

    #[test]
    fn test_decode_data_uri_non_base64() {
        let uri = "data:image/png,raw-data-here";
        let image = MarkdownExtractor::decode_data_uri_image(uri, 0);
        assert!(image.is_none());
    }

    #[test]
    fn test_decode_data_uri_invalid_base64() {
        let uri = "data:image/png;base64,!!!not-valid-base64!!!";
        let image = MarkdownExtractor::decode_data_uri_image(uri, 0);
        assert!(image.is_none());
    }

    #[test]
    fn test_decode_data_uri_not_data_uri() {
        let uri = "https://example.com/image.png";
        let image = MarkdownExtractor::decode_data_uri_image(uri, 0);
        assert!(image.is_none());
    }

    #[test]
    fn test_extract_text_collects_data_uri_images() {
        let png_b64 =
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let md = format!("# Title\n\n![alt](data:image/png;base64,{png_b64})\n\nSome text.");

        let parser = Parser::new_ext(&md, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let mut images = Vec::new();
        let text = MarkdownExtractor::extract_text_from_events(&events, &mut images);

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].format.as_ref(), "png");
        assert!(text.contains("[Image: data:image/png;base64,"));
    }

    #[test]
    fn test_extract_text_skips_http_images() {
        let md = "# Title\n\n![alt](https://example.com/photo.jpg)\n\nSome text.";

        let parser = Parser::new_ext(md, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let mut images = Vec::new();
        let text = MarkdownExtractor::extract_text_from_events(&events, &mut images);

        assert!(images.is_empty());
        assert!(text.contains("[Image: https://example.com/photo.jpg]"));
    }

    #[tokio::test]
    async fn test_extract_bytes_with_data_uri_image() {
        let png_b64 =
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let md = format!("# Doc\n\n![photo](data:image/png;base64,{png_b64})\n\nText.");

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_bytes(md.as_bytes(), "text/markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown with data URI image");

        assert!(result.images.is_some());
        let imgs = result.images.unwrap();
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].format.as_ref(), "png");
    }

    #[tokio::test]
    async fn test_extract_bytes_no_images() {
        let md = b"# Simple\n\nJust text, no images.";

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_bytes(md, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown without images");

        assert!(result.images.is_none());
    }
}
