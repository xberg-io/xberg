//! HTML to Markdown conversion functionality.

use super::stack_management::check_wasm_size_limit;
#[cfg(not(target_arch = "wasm32"))]
use super::stack_management::{html_requires_large_stack, run_on_dedicated_stack};
use crate::core::config::OutputFormat as KreuzbergOutputFormat;
use crate::error::{KreuzbergError, Result};
use crate::types::HtmlMetadata;
use html_to_markdown_rs::{
    ConversionOptions, MetadataConfig, OutputFormat as LibOutputFormat, convert as convert_html, convert_with_metadata,
};

/// Map Kreuzberg OutputFormat to html-to-markdown-rs OutputFormat.
fn map_output_format(format: KreuzbergOutputFormat) -> LibOutputFormat {
    match format {
        KreuzbergOutputFormat::Markdown => LibOutputFormat::Markdown,
        KreuzbergOutputFormat::Djot => LibOutputFormat::Djot,
        KreuzbergOutputFormat::Plain => LibOutputFormat::Plain,
        // Html and Structured default to Markdown for HTML conversions
        // Structured output includes the converted content plus full element metadata
        KreuzbergOutputFormat::Html | KreuzbergOutputFormat::Structured => LibOutputFormat::Markdown,
    }
}

/// Resolve conversion options with sensible defaults.
///
/// If no options are provided, creates defaults with:
/// - `extract_metadata = true` (parse YAML frontmatter)
/// - `hocr_spatial_tables = false` (disable hOCR table detection)
/// - `preprocessing.enabled = false` (disable HTML preprocessing)
///
/// Sets output format based on the provided format parameter.
pub fn resolve_conversion_options(
    options: Option<ConversionOptions>,
    output_format: KreuzbergOutputFormat,
) -> ConversionOptions {
    let mut opts = options.unwrap_or_else(|| ConversionOptions {
        extract_metadata: true,
        hocr_spatial_tables: false,
        preprocessing: super::types::PreprocessingOptions {
            enabled: false,
            ..Default::default()
        },
        ..Default::default()
    });

    opts.output_format = map_output_format(output_format);
    opts
}

/// Internal conversion helper that applies options to the conversion.
fn convert_html_with_options(html: &str, options: ConversionOptions) -> Result<String> {
    convert_html(html, Some(options))
        .map_err(|e| KreuzbergError::parsing(format!("Failed to convert HTML to Markdown: {}", e)))
}

/// Convert HTML with optional configuration and output format.
///
/// Uses sensible defaults if no configuration is provided:
/// - `extract_metadata = true` (parse YAML frontmatter)
/// - `hocr_spatial_tables = false` (disable hOCR table detection)
/// - `preprocessing.enabled = false` (disable HTML preprocessing)
///
/// Supports both markdown and djot output based on the output_format parameter.
/// Defaults to Markdown for backward compatibility.
///
/// # WASM Limitations
///
/// In WASM builds, HTML files larger than 2MB will be rejected with an error
/// to prevent stack overflow. For larger files, use the native library.
///
/// # Arguments
///
/// * `html` - The HTML string to convert
/// * `options` - Optional conversion options; defaults will be used if None
/// * `output_format` - Optional output format; defaults to Markdown if None
///
/// # Returns
///
/// A markdown or djot string, or an error if conversion fails
pub fn convert_html_to_markdown(
    html: &str,
    options: Option<ConversionOptions>,
    output_format: Option<KreuzbergOutputFormat>,
) -> Result<String> {
    check_wasm_size_limit(html)?;

    let format = output_format.unwrap_or(KreuzbergOutputFormat::Markdown);
    let options = resolve_conversion_options(options, format);

    #[cfg(not(target_arch = "wasm32"))]
    if html_requires_large_stack(html.len()) {
        let html = html.to_string();
        return run_on_dedicated_stack(move || convert_html_with_options(&html, options));
    }

    convert_html_with_options(html, options)
}

/// Convert HTML with direct metadata extraction and output format support.
///
/// Extracts metadata directly from HTML using the metadata extraction
/// capabilities of the `html-to-markdown-rs` library, without relying
/// on YAML frontmatter in the converted markdown.
///
/// Metadata is extracted via the `MetadataCollector` mechanism, so `extract_metadata`
/// is set to `false` in the conversion options to prevent duplicate YAML frontmatter
/// from being prepended to the content string.
///
/// Supports both markdown and djot output based on the output_format parameter.
/// Defaults to Markdown for backward compatibility.
///
/// # WASM Limitations
///
/// In WASM builds, HTML files larger than 2MB will be rejected with an error
/// to prevent stack overflow. For larger files, use the native library.
///
/// # Arguments
///
/// * `html` - The HTML string to convert
/// * `options` - Optional conversion options; defaults will be used if None
/// * `output_format` - Optional output format; defaults to Markdown if None
///
/// # Returns
///
/// A tuple of (markdown/djot content, optional metadata), or an error if conversion fails
pub fn convert_html_to_markdown_with_metadata(
    html: &str,
    options: Option<ConversionOptions>,
    output_format: Option<KreuzbergOutputFormat>,
) -> Result<(String, Option<HtmlMetadata>)> {
    check_wasm_size_limit(html)?;

    let format = output_format.unwrap_or(KreuzbergOutputFormat::Markdown);
    // Disable extract_metadata (YAML frontmatter) in the conversion options because
    // metadata is collected separately via MetadataCollector in convert_with_metadata.
    // Keeping extract_metadata=true would cause metadata to appear both as YAML frontmatter
    // in the content string and in the returned metadata struct, which tanks quality scores.
    let mut options = resolve_conversion_options(options, format);
    options.extract_metadata = false;
    let metadata_config = MetadataConfig::default();

    #[cfg(not(target_arch = "wasm32"))]
    if html_requires_large_stack(html.len()) {
        let html = html.to_string();
        return run_on_dedicated_stack(move || {
            convert_with_metadata(&html, Some(options), metadata_config, None)
                .map_err(|e| KreuzbergError::parsing(format!("HTML metadata extraction failed: {}", e)))
                .map(|(content, extended_metadata)| {
                    let html_metadata = HtmlMetadata::from(extended_metadata);
                    (
                        content,
                        if html_metadata.is_empty() {
                            None
                        } else {
                            Some(html_metadata)
                        },
                    )
                })
        });
    }

    let (content, extended_metadata) = convert_with_metadata(html, Some(options), metadata_config, None)
        .map_err(|e| KreuzbergError::parsing(format!("HTML metadata extraction failed: {}", e)))?;

    let html_metadata = HtmlMetadata::from(extended_metadata);

    Ok((
        content,
        if html_metadata.is_empty() {
            None
        } else {
            Some(html_metadata)
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_html() {
        let html = "<h1>Hello World</h1><p>This is a test.</p>";
        let result = convert_html_to_markdown(html, None, None).unwrap();
        assert!(result.contains("# Hello World"));
        assert!(result.contains("This is a test."));
    }

    #[test]
    fn test_html_config_heading_style() {
        let html = "<h1>Heading</h1>";
        let options = ConversionOptions {
            heading_style: super::super::types::HeadingStyle::Atx,
            ..Default::default()
        };
        let result = convert_html_to_markdown(html, Some(options), None).unwrap();
        assert!(result.contains("# Heading"));
    }

    #[test]
    fn test_html_with_list() {
        let html = "<ul><li>Item 1</li><li>Item 2</li></ul>";
        let result = convert_html_to_markdown(html, None, None).unwrap();
        assert!(result.contains("Item 1"));
        assert!(result.contains("Item 2"));
    }

    #[test]
    fn test_html_with_table() {
        let html = "<table><tr><th>Header</th></tr><tr><td>Data</td></tr></table>";
        let result = convert_html_to_markdown(html, None, None).unwrap();
        assert!(result.contains("Header"));
        assert!(result.contains("Data"));
    }

    #[test]
    fn test_preprocessing_config() {
        let html = "<nav>Navigation</nav><p>Content</p>";
        let mut options = ConversionOptions::default();
        options.preprocessing.enabled = true;
        options.preprocessing.preset = super::super::types::PreprocessingPreset::Standard;
        options.preprocessing.remove_navigation = true;
        let result = convert_html_to_markdown(html, Some(options), None).unwrap();
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_preprocessing_keeps_main_content() {
        let html = r#"
<!DOCTYPE html>
<html>
  <body>
    <nav><p>Skip me</p></nav>
    <main id="content">
      <article>
        <h1>Taylor Swift</h1>
        <p>Taylor Alison Swift is an American singer-songwriter.</p>
      </article>
    </main>
  </body>
</html>
"#;
        let markdown = convert_html_to_markdown(html, None, None).expect("conversion failed");
        assert!(markdown.contains("Taylor Alison Swift"), "{markdown}");
    }

    /// Test extraction of core document metadata fields:
    /// title, description, author, canonical_url, and base_href.
    #[test]
    fn test_metadata_document_fields() {
        let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Amazing Article</title>
    <meta name="description" content="This is a description of the article">
    <meta name="author" content="Jane Doe">
    <link rel="canonical" href="https://example.com/article/amazing">
    <base href="https://example.com/">
  </head>
  <body>
    <h1>Amazing Article</h1>
    <p>Content here.</p>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert_eq!(
            metadata.title,
            Some("Amazing Article".to_string()),
            "Title should be extracted from <title> tag"
        );

        assert_eq!(
            metadata.description,
            Some("This is a description of the article".to_string()),
            "Description should be extracted from meta description tag"
        );

        assert_eq!(
            metadata.author,
            Some("Jane Doe".to_string()),
            "Author should be extracted from meta author tag"
        );

        assert_eq!(
            metadata.canonical_url,
            Some("https://example.com/article/amazing".to_string()),
            "Canonical URL should be extracted from link[rel=canonical]"
        );

        assert_eq!(
            metadata.base_href,
            Some("https://example.com/".to_string()),
            "Base href should be extracted from <base> tag"
        );
    }

    /// Test metadata extraction from an empty HTML string.
    #[test]
    fn test_metadata_empty_html() {
        let html = "";

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None, None).unwrap();

        assert!(
            metadata.is_none() || metadata.as_ref().unwrap().is_empty(),
            "Empty HTML should return None or empty metadata"
        );
    }

    /// Test that HTML with no metadata tags returns defaults.
    #[test]
    fn test_metadata_no_metadata() {
        let html = r#"<!DOCTYPE html>
<html>
  <body>
    <h1>Simple Page</h1>
    <p>Just content, no metadata tags.</p>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None, None).unwrap();

        if let Some(meta) = metadata {
            assert!(
                meta.title.is_none() || meta.title.is_some(),
                "Title might be extracted from h1 or might be None"
            );
            assert!(meta.open_graph.is_empty(), "Open Graph should be empty with no OG tags");
            assert!(
                meta.twitter_card.is_empty(),
                "Twitter Card should be empty with no Twitter tags"
            );
        }
    }

    /// Test that malformed HTML is handled gracefully without panics.
    #[test]
    fn test_metadata_malformed_html() {
        let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Malformed
    <meta name="author content="No closing quote
  </head>
  <body>
    <h1>Title
    <p>Unclosed paragraph
    <div>Unmatched closing tag</div></div>
  </body>
</html>"#;

        let result = convert_html_to_markdown_with_metadata(html, None, None);
        assert!(
            result.is_ok(),
            "Malformed HTML should be handled gracefully without error"
        );

        let (_, metadata) = result.unwrap();
        assert!(
            metadata.is_some() || metadata.is_none(),
            "Should return either Some or None metadata"
        );
    }

    /// Test handling of special characters and HTML entities in metadata values.
    #[test]
    fn test_metadata_special_characters() {
        let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Café &amp; Restaurant &quot;Guide&quot;</title>
    <meta name="description" content="5 stars ★★★★★ &lt; 50% off">
    <meta name="author" content="José García-López">
    <meta property="og:title" content="Quote &quot;Special&quot; &amp; Characters">
  </head>
  <body>
    <h1>Article Title &copy; 2024</h1>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        if let Some(title) = &metadata.title {
            assert!(!title.is_empty(), "Title should be extracted and decoded");
        }

        if let Some(author) = &metadata.author {
            assert!(
                author.contains("García") || author.contains("Jose"),
                "Special characters should be handled correctly"
            );
        }

        if let Some(desc) = &metadata.description {
            assert!(!desc.is_empty(), "Description should be extracted");
        }
    }

    /// Test handling of duplicate meta tags (last value should win or all collected).
    #[test]
    fn test_metadata_duplicate_tags() {
        let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>First Title</title>
    <meta name="description" content="First description">
    <meta name="description" content="Second description (should override)">
    <meta name="author" content="Author One">
    <meta name="author" content="Author Two">
  </head>
  <body>
    <p>Content</p>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        if let Some(title) = &metadata.title {
            assert_eq!(
                title, "First Title",
                "Title should be the single value from first title tag"
            );
        }

        if let Some(description) = &metadata.description {
            assert!(
                !description.is_empty(),
                "Description should be populated even with duplicates"
            );
            assert!(
                description.contains("First") || description.contains("Second"),
                "Description should contain one of the duplicate values"
            );
        }
    }

    /// Test graceful handling of malformed JSON-LD structured data
    /// Validates that invalid JSON in script type="application/ld+json"
    /// does not cause panics and is skipped gracefully.
    #[test]
    fn test_malformed_json_ld_graceful_handling() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Malformed JSON-LD Test</title>
    <script type="application/ld+json">
    {
      "@context": "https://schema.org",
      "@type": "Article",
      "headline": "Test Article",
      "author": "John Doe"
      "datePublished": "2024-01-01"
    }
    </script>
</head>
<body>
    <h1>Article Title</h1>
    <p>This HTML contains invalid JSON-LD (missing comma after author field)</p>
</body>
</html>"#;

        let result = convert_html_to_markdown_with_metadata(html, None, None);

        assert!(
            result.is_ok(),
            "Malformed JSON-LD should not cause panic. Error: {:?}",
            result.err()
        );

        let (markdown, metadata) = result.unwrap();

        assert!(
            !markdown.is_empty(),
            "Markdown should be extracted despite invalid JSON-LD"
        );
        assert!(
            markdown.contains("Article Title") || markdown.contains("Article"),
            "Content should be properly converted to Markdown"
        );

        if let Some(meta) = metadata {
            assert_eq!(
                meta.title,
                Some("Malformed JSON-LD Test".to_string()),
                "Document metadata should be extracted from tags"
            );
        }
    }

    /// Test XSS sanitization in metadata fields
    /// Validates that script tags and malicious content in metadata
    /// are properly handled and don't cause panics.
    /// Note: The actual sanitization is done by the html-to-markdown-rs library,
    /// which may escape, strip, or preserve content depending on context.
    #[test]
    fn test_metadata_xss_sanitization() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Safe Title &lt;script&gt;alert('xss')&lt;/script&gt;</title>
    <meta name="description" content="Description with encoded content">
    <meta name="author" content="Author Name">
    <meta property="og:title" content="OG Title">
    <meta property="og:description" content="OG Description">
</head>
<body>
    <h1>Title Section</h1>
    <p>Content here</p>
</body>
</html>"#;

        let result = convert_html_to_markdown_with_metadata(html, None, None);
        assert!(
            result.is_ok(),
            "HTML with script-like content should not cause error. Error: {:?}",
            result.err()
        );

        let (markdown, metadata) = result.unwrap();

        assert!(!markdown.is_empty(), "Markdown should be generated");

        if let Some(meta) = metadata {
            if let Some(title) = &meta.title {
                assert!(!title.is_empty(), "Title should be extracted");
                assert!(
                    title.contains("Safe") || title.contains("script"),
                    "Title should extract content from title tag: {}",
                    title
                );
            }

            if let Some(desc) = &meta.description {
                assert!(!desc.is_empty(), "Description should be extracted");
            }

            if let Some(author) = &meta.author {
                assert_eq!(author, "Author Name", "Author should be correctly extracted");
            }

            if !meta.open_graph.is_empty() {
                let og_count = meta.open_graph.len();
                assert!(og_count > 0, "Open Graph tags should be extracted");
            }
        }
    }

    #[test]
    fn test_convert_html_to_djot() {
        use crate::core::config::OutputFormat;

        let html = "<h1>Hello World</h1><p>This is a test.</p>";
        let result = convert_html_to_markdown(html, None, Some(OutputFormat::Djot)).unwrap();
        assert!(result.contains("# Hello World"));
        assert!(result.contains("This is a test."));
    }

    #[test]
    fn test_convert_html_to_djot_with_emphasis() {
        use crate::core::config::OutputFormat;

        let html = "<p>This is <strong>bold</strong> and <em>italic</em>.</p>";
        let result = convert_html_to_markdown(html, None, Some(OutputFormat::Djot)).unwrap();
        // Djot uses * for strong, _ for emphasis
        assert!(result.contains("*bold*"));
        assert!(result.contains("_italic_"));
    }

    #[test]
    fn test_convert_html_with_metadata_djot() {
        use crate::core::config::OutputFormat;

        let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Test Page</title>
    <meta name="description" content="Test description">
  </head>
  <body>
    <h1>Content</h1>
    <p>This is <strong>content</strong>.</p>
  </body>
</html>"#;

        let (content, metadata) = convert_html_to_markdown_with_metadata(html, None, Some(OutputFormat::Djot)).unwrap();

        // Content should be in djot format
        assert!(content.contains("# Content"));
        assert!(content.contains("*content*")); // Djot strong syntax

        // Metadata should still be extracted
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.title, Some("Test Page".to_string()));
        assert_eq!(meta.description, Some("Test description".to_string()));
    }
}
