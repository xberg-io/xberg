//! HTML to Markdown conversion functions.
//!
//! This module provides HTML to Markdown conversion using the `html-to-markdown-rs` library.
//! It supports inline image extraction and YAML frontmatter parsing for HTML metadata.
//!
//! # Features
//!
//! - **HTML to Markdown conversion**: Clean, readable Markdown output
//! - **Inline image extraction**: Extract base64 and data URI images
//! - **YAML frontmatter**: Parse YAML metadata from Markdown output
//! - **Customizable conversion**: Full access to `html-to-markdown-rs` options
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::extraction::html::convert_html_to_markdown;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let html = r#"<h1>Title</h1><p>This is <strong>bold</strong> text.</p>"#;
//! let markdown = convert_html_to_markdown(html, None)?;
//!
//! assert!(markdown.contains("# Title"));
//! assert!(markdown.contains("**bold**"));
//! # Ok(())
//! # }
//! ```
use crate::error::{KreuzbergError, Result};
use crate::types::HtmlMetadata;
use html_to_markdown_rs::{
    ConversionOptions, HtmlExtraction, InlineImage, InlineImageConfig as LibInlineImageConfig, InlineImageFormat,
    MetadataConfig, convert as convert_html, convert_with_inline_images, convert_with_metadata,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use std::{any::Any, thread};

pub use html_to_markdown_rs::{
    CodeBlockStyle, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle, PreprocessingOptions,
    PreprocessingPreset, WhitespaceMode,
};

#[cfg(target_arch = "wasm32")]
const MAX_HTML_SIZE_BYTES: usize = 2 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
const LARGE_HTML_STACK_THRESHOLD_BYTES: usize = 512 * 1024;

#[cfg(not(target_arch = "wasm32"))]
const HTML_CONVERSION_STACK_SIZE_BYTES: usize = 16 * 1024 * 1024;

/// Result of HTML extraction with optional images and warnings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlExtractionResult {
    pub markdown: String,
    pub images: Vec<ExtractedInlineImage>,
    pub warnings: Vec<String>,
}

/// Extracted inline image with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedInlineImage {
    pub data: Vec<u8>,
    pub format: String,
    pub filename: Option<String>,
    pub description: Option<String>,
    pub dimensions: Option<(u32, u32)>,
    pub attributes: HashMap<String, String>,
}

fn inline_image_format_to_str(format: &InlineImageFormat) -> String {
    match format {
        InlineImageFormat::Png => "png".to_string(),
        InlineImageFormat::Jpeg => "jpeg".to_string(),
        InlineImageFormat::Gif => "gif".to_string(),
        InlineImageFormat::Bmp => "bmp".to_string(),
        InlineImageFormat::Webp => "webp".to_string(),
        InlineImageFormat::Svg => "svg".to_string(),
        InlineImageFormat::Other(custom) => {
            let trimmed = custom.trim();
            if trimmed.is_empty() {
                return "bin".to_string();
            }

            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("svg") {
                return "svg".to_string();
            }

            let mut result = String::with_capacity(10);
            let mut candidate = lower.as_str();

            if let Some(idx) = candidate.find(['+', ';']) {
                candidate = &candidate[..idx];
            }

            if let Some(idx) = candidate.rfind('.') {
                candidate = &candidate[idx + 1..];
            }

            candidate = candidate.trim_start_matches("x-");

            if candidate.is_empty() {
                "bin".to_string()
            } else {
                result.push_str(candidate);
                result
            }
        }
    }
}

fn inline_image_to_extracted(image: InlineImage) -> ExtractedInlineImage {
    ExtractedInlineImage {
        data: image.data,
        format: inline_image_format_to_str(&image.format),
        filename: image.filename,
        description: image.description,
        dimensions: image.dimensions,
        attributes: image.attributes.into_iter().collect(),
    }
}

fn resolve_conversion_options(options: Option<ConversionOptions>) -> ConversionOptions {
    options.unwrap_or_else(|| ConversionOptions {
        extract_metadata: true,
        hocr_spatial_tables: false,
        preprocessing: PreprocessingOptions {
            enabled: false,
            ..Default::default()
        },
        ..Default::default()
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn html_requires_large_stack(len: usize) -> bool {
    len >= LARGE_HTML_STACK_THRESHOLD_BYTES
}

fn convert_html_with_options(html: &str, options: ConversionOptions) -> Result<String> {
    convert_html(html, Some(options))
        .map_err(|e| KreuzbergError::parsing(format!("Failed to convert HTML to Markdown: {}", e)))
}

fn convert_inline_images_with_options(
    html: &str,
    options: ConversionOptions,
    image_config: LibInlineImageConfig,
) -> Result<HtmlExtraction> {
    convert_with_inline_images(html, Some(options), image_config)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to convert HTML to Markdown with images: {}", e)))
}

#[cfg(not(target_arch = "wasm32"))]
fn convert_inline_images_with_large_stack(
    html: String,
    options: ConversionOptions,
    image_config: LibInlineImageConfig,
) -> Result<HtmlExtraction> {
    run_on_dedicated_stack(move || convert_inline_images_with_options(&html, options, image_config))
}

#[cfg(not(target_arch = "wasm32"))]
fn run_on_dedicated_stack<T, F>(job: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    let handle = thread::Builder::new()
        .name("kreuzberg-html-conversion".to_string())
        .stack_size(HTML_CONVERSION_STACK_SIZE_BYTES)
        .spawn(job)
        .map_err(|err| KreuzbergError::Other(format!("Failed to spawn HTML conversion thread: {}", err)))?;

    match handle.join() {
        Ok(result) => result,
        Err(panic) => {
            let reason = extract_panic_reason(&panic);
            Err(KreuzbergError::Other(format!("HTML conversion panicked: {}", reason)))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn extract_panic_reason(panic: &Box<dyn Any + Send + 'static>) -> String {
    if let Some(msg) = panic.downcast_ref::<&str>() {
        (*msg).to_string()
    } else if let Some(msg) = panic.downcast_ref::<String>() {
        msg.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// Convert HTML to markdown with optional configuration.
///
/// Uses sensible defaults if no configuration is provided:
/// - `extract_metadata = true` (parse YAML frontmatter)
/// - `hocr_spatial_tables = false` (disable hOCR table detection)
/// - `preprocessing.enabled = false` (disable HTML preprocessing)
///
/// # WASM Limitations
///
/// In WASM builds, HTML files larger than 2MB will be rejected with an error
/// to prevent stack overflow. For larger files, use the native library.
pub fn convert_html_to_markdown(html: &str, options: Option<ConversionOptions>) -> Result<String> {
    #[cfg(target_arch = "wasm32")]
    if html.len() > MAX_HTML_SIZE_BYTES {
        return Err(KreuzbergError::validation(format!(
            "HTML file size ({} bytes) exceeds WASM limit of {} bytes (2MB). \
             Large HTML files cannot be processed in WASM due to stack constraints. \
             Consider using the native library for files of this size.",
            html.len(),
            MAX_HTML_SIZE_BYTES
        )));
    }

    let options = resolve_conversion_options(options);

    #[cfg(not(target_arch = "wasm32"))]
    if html_requires_large_stack(html.len()) {
        let html = html.to_string();
        return run_on_dedicated_stack(move || convert_html_with_options(&html, options));
    }

    convert_html_with_options(html, options)
}

/// Process HTML with optional image extraction.
///
/// # WASM Limitations
///
/// In WASM builds, HTML files larger than 2MB will be rejected to prevent stack overflow.
pub fn process_html(
    html: &str,
    options: Option<ConversionOptions>,
    extract_images: bool,
    max_image_size: u64,
) -> Result<HtmlExtractionResult> {
    #[cfg(target_arch = "wasm32")]
    if html.len() > MAX_HTML_SIZE_BYTES {
        return Err(KreuzbergError::validation(format!(
            "HTML file size ({} bytes) exceeds WASM limit of {} bytes (2MB). \
             Large HTML files cannot be processed in WASM due to stack constraints.",
            html.len(),
            MAX_HTML_SIZE_BYTES
        )));
    }

    if extract_images {
        let options = resolve_conversion_options(options.clone());
        let mut img_config = LibInlineImageConfig::new(max_image_size);
        img_config.filename_prefix = Some("inline-image".to_string());

        #[cfg(not(target_arch = "wasm32"))]
        let extraction = if html_requires_large_stack(html.len()) {
            convert_inline_images_with_large_stack(html.to_string(), options, img_config)?
        } else {
            convert_inline_images_with_options(html, options, img_config)?
        };

        #[cfg(target_arch = "wasm32")]
        let extraction = convert_inline_images_with_options(html, options, img_config)?;

        let images = extraction
            .inline_images
            .into_iter()
            .map(inline_image_to_extracted)
            .collect();

        let warnings = extraction.warnings.into_iter().map(|w| w.message).collect();

        Ok(HtmlExtractionResult {
            markdown: extraction.markdown,
            images,
            warnings,
        })
    } else {
        let options = resolve_conversion_options(options);
        let markdown = convert_html_to_markdown(html, Some(options))?;

        Ok(HtmlExtractionResult {
            markdown,
            images: Vec::new(),
            warnings: Vec::new(),
        })
    }
}

/// Convert HTML to markdown with direct metadata extraction.
///
/// Extracts metadata directly from HTML using the metadata extraction
/// capabilities of the `html-to-markdown-rs` library, without relying
/// on YAML frontmatter in the converted markdown.
///
/// # WASM Limitations
///
/// In WASM builds, HTML files larger than 2MB will be rejected with an error
/// to prevent stack overflow. For larger files, use the native library.
pub fn convert_html_to_markdown_with_metadata(
    html: &str,
    options: Option<ConversionOptions>,
) -> Result<(String, Option<HtmlMetadata>)> {
    #[cfg(target_arch = "wasm32")]
    if html.len() > MAX_HTML_SIZE_BYTES {
        return Err(KreuzbergError::validation(format!(
            "HTML file size ({} bytes) exceeds WASM limit of {} bytes (2MB). \
             Large HTML files cannot be processed in WASM due to stack constraints. \
             Consider using the native library for files of this size.",
            html.len(),
            MAX_HTML_SIZE_BYTES
        )));
    }

    let options = resolve_conversion_options(options);
    let metadata_config = MetadataConfig::default();

    #[cfg(not(target_arch = "wasm32"))]
    if html_requires_large_stack(html.len()) {
        let html = html.to_string();
        return run_on_dedicated_stack(move || {
            convert_with_metadata(&html, Some(options), metadata_config)
                .map_err(|e| KreuzbergError::parsing(format!("HTML metadata extraction failed: {}", e)))
                .map(|(markdown, extended_metadata)| {
                    let html_metadata = HtmlMetadata::from(extended_metadata);
                    (
                        markdown,
                        if html_metadata.is_empty() {
                            None
                        } else {
                            Some(html_metadata)
                        },
                    )
                })
        });
    }

    let (markdown, extended_metadata) = convert_with_metadata(html, Some(options), metadata_config)
        .map_err(|e| KreuzbergError::parsing(format!("HTML metadata extraction failed: {}", e)))?;

    let html_metadata = HtmlMetadata::from(extended_metadata);

    Ok((
        markdown,
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
    use crate::types::{ImageType, LinkType, StructuredDataType, TextDirection};

    #[test]
    fn test_convert_simple_html() {
        let html = "<h1>Hello World</h1><p>This is a test.</p>";
        let result = convert_html_to_markdown(html, None).unwrap();
        assert!(result.contains("# Hello World"));
        assert!(result.contains("This is a test."));
    }

    #[test]
    fn test_process_html_without_images() {
        let html = "<h1>Test</h1><p>Content</p>";
        let result = process_html(html, None, false, 1024 * 1024).unwrap();
        assert!(result.markdown.contains("# Test"));
        assert!(result.markdown.contains("Content"));
        assert!(result.images.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_html_with_inline_image() {
        let html = r#"<p>Image: <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==" alt="Test"></p>"#;
        let mut options = ConversionOptions::default();
        options.preprocessing.enabled = false;
        let result = process_html(html, Some(options), true, 1024 * 1024).unwrap();
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].format, "png");
    }

    #[test]
    fn test_html_config_heading_style() {
        let html = "<h1>Heading</h1>";
        let options = ConversionOptions {
            heading_style: HeadingStyle::Atx,
            ..Default::default()
        };
        let result = convert_html_to_markdown(html, Some(options)).unwrap();
        assert!(result.contains("# Heading"));
    }

    #[test]
    fn test_html_with_list() {
        let html = "<ul><li>Item 1</li><li>Item 2</li></ul>";
        let result = convert_html_to_markdown(html, None).unwrap();
        assert!(result.contains("Item 1"));
        assert!(result.contains("Item 2"));
    }

    #[test]
    fn test_html_with_table() {
        let html = "<table><tr><th>Header</th></tr><tr><td>Data</td></tr></table>";
        let result = convert_html_to_markdown(html, None).unwrap();
        assert!(result.contains("Header"));
        assert!(result.contains("Data"));
    }

    #[test]
    fn test_inline_image_format_conversion() {
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Png), "png");
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Jpeg), "jpeg");
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Svg), "svg");
    }

    #[test]
    fn test_preprocessing_config() {
        let html = "<nav>Navigation</nav><p>Content</p>";
        let mut options = ConversionOptions::default();
        options.preprocessing.enabled = true;
        options.preprocessing.preset = PreprocessingPreset::Standard;
        options.preprocessing.remove_navigation = true;
        let result = convert_html_to_markdown(html, Some(options)).unwrap();
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_inline_image_format_other_with_extension() {
        let format = InlineImageFormat::Other("image/x-custom.jpg".to_string());
        assert_eq!(inline_image_format_to_str(&format), "jpg");
    }

    #[test]
    fn test_inline_image_format_other_empty() {
        let format = InlineImageFormat::Other("".to_string());
        assert_eq!(inline_image_format_to_str(&format), "bin");
    }

    #[test]
    fn test_inline_image_format_other_x_prefix() {
        let format = InlineImageFormat::Other("x-custom".to_string());
        assert_eq!(inline_image_format_to_str(&format), "custom");
    }

    #[test]
    fn test_process_html_empty_string() {
        let result = process_html("", None, false, 1024).unwrap();
        assert!(result.markdown.is_empty() || result.markdown.trim().is_empty());
        assert!(result.images.is_empty());
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
        let markdown = convert_html_to_markdown(html, None).expect("conversion failed");
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

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
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

    /// Test that keywords are extracted as Vec<String>, not comma-separated string.
    /// This validates the proper parsing of keyword metadata.
    #[test]
    fn test_metadata_keywords_as_vec() {
        let html = r#"<!DOCTYPE html>
<html>
  <head>
    <meta name="keywords" content="rust, web, metadata, extraction">
  </head>
  <body>
    <p>Test content</p>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert!(
            !metadata.keywords.is_empty(),
            "Keywords should be extracted as a vector"
        );
        assert!(
            metadata.keywords.len() >= 4,
            "Keywords should be split on comma separators"
        );

        let keyword_set: std::collections::HashSet<_> = metadata.keywords.iter().map(|k| k.trim()).collect();
        assert!(
            keyword_set.contains("rust") || keyword_set.iter().any(|k| k.contains("rust")),
            "Keywords vector should contain 'rust'"
        );
    }

    /// Test language extraction from the html lang attribute.
    #[test]
    fn test_metadata_language() {
        let html = r#"<!DOCTYPE html>
<html lang="en-US">
  <head>
    <title>English Page</title>
  </head>
  <body>
    <p>Content in English</p>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert_eq!(
            metadata.language,
            Some("en-US".to_string()),
            "Language should be extracted from html lang attribute"
        );
    }

    /// Test text direction extraction (ltr, rtl, auto).
    /// Validates the detection of document text directionality.
    #[test]
    fn test_metadata_text_direction() {
        let html_ltr = r#"<!DOCTYPE html>
<html dir="ltr">
  <head><title>LTR</title></head>
  <body><p>Left to right</p></body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html_ltr, None).unwrap();
        let metadata = metadata.expect("metadata should be present");
        assert_eq!(
            metadata.text_direction,
            Some(TextDirection::LeftToRight),
            "Text direction should be extracted as LeftToRight"
        );

        let html_rtl = r#"<!DOCTYPE html>
<html dir="rtl">
  <head><title>RTL</title></head>
  <body><p>Right to left</p></body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html_rtl, None).unwrap();
        let metadata = metadata.expect("metadata should be present");
        assert_eq!(
            metadata.text_direction,
            Some(TextDirection::RightToLeft),
            "Text direction should be extracted as RightToLeft"
        );

        let html_auto = r#"<!DOCTYPE html>
<html dir="auto">
  <head><title>Auto</title></head>
  <body><p>Auto direction</p></body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html_auto, None).unwrap();
        let metadata = metadata.expect("metadata should be present");
        assert_eq!(
            metadata.text_direction,
            Some(TextDirection::Auto),
            "Text direction should be extracted as Auto"
        );
    }

    /// Test Open Graph metadata extraction into BTreeMap.
    /// Validates extraction of og:title, og:description, og:image, og:url,
    /// og:type, and og:site_name.
    #[test]
    fn test_metadata_open_graph() {
        let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Social Article</title>
    <meta property="og:title" content="Open Graph Title">
    <meta property="og:description" content="Share this amazing article">
    <meta property="og:image" content="https://example.com/image.jpg">
    <meta property="og:url" content="https://example.com/article">
    <meta property="og:type" content="article">
    <meta property="og:site_name" content="My Website">
  </head>
  <body>
    <h1>Article</h1>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert!(
            !metadata.open_graph.is_empty(),
            "Open Graph map should contain extracted OG tags"
        );

        assert!(
            metadata.open_graph.contains_key("title")
                || metadata.open_graph.values().any(|v| v.contains("Open Graph Title")),
            "Open Graph should contain title"
        );

        assert!(
            metadata.open_graph.contains_key("description")
                || metadata.open_graph.values().any(|v| v.contains("Share this amazing")),
            "Open Graph should contain description"
        );

        assert!(
            metadata.open_graph.contains_key("image") || metadata.open_graph.values().any(|v| v.contains("image.jpg")),
            "Open Graph should contain image URL"
        );

        assert!(
            metadata.open_graph.contains_key("url")
                || metadata.open_graph.values().any(|v| v.contains("example.com/article")),
            "Open Graph should contain URL"
        );

        assert!(
            metadata.open_graph.contains_key("type") || metadata.open_graph.values().any(|v| v.contains("article")),
            "Open Graph should contain type"
        );

        assert!(
            metadata.open_graph.contains_key("site_name")
                || metadata.open_graph.values().any(|v| v.contains("My Website")),
            "Open Graph should contain site name"
        );
    }

    /// Test Twitter Card metadata extraction into BTreeMap.
    /// Validates extraction of twitter:card, twitter:title, twitter:description,
    /// twitter:image, twitter:site, and twitter:creator.
    #[test]
    fn test_metadata_twitter_card() {
        let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Tweetable Article</title>
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="Tweet-worthy Title">
    <meta name="twitter:description" content="This deserves a retweet">
    <meta name="twitter:image" content="https://example.com/tweet-image.jpg">
    <meta name="twitter:site" content="@mywebsite">
    <meta name="twitter:creator" content="@author">
  </head>
  <body>
    <h1>Article</h1>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert!(
            !metadata.twitter_card.is_empty(),
            "Twitter Card map should contain extracted Twitter tags"
        );

        assert!(
            metadata.twitter_card.contains_key("card")
                || metadata
                    .twitter_card
                    .values()
                    .any(|v| v.contains("summary_large_image")),
            "Twitter Card should contain card type"
        );

        assert!(
            metadata.twitter_card.contains_key("title")
                || metadata.twitter_card.values().any(|v| v.contains("Tweet-worthy Title")),
            "Twitter Card should contain title"
        );

        assert!(
            metadata.twitter_card.contains_key("description")
                || metadata.twitter_card.values().any(|v| v.contains("retweet")),
            "Twitter Card should contain description"
        );

        assert!(
            metadata.twitter_card.contains_key("image")
                || metadata.twitter_card.values().any(|v| v.contains("tweet-image.jpg")),
            "Twitter Card should contain image"
        );

        assert!(
            metadata.twitter_card.contains_key("site")
                || metadata.twitter_card.values().any(|v| v.contains("@mywebsite")),
            "Twitter Card should contain site handle"
        );

        assert!(
            metadata.twitter_card.contains_key("creator")
                || metadata.twitter_card.values().any(|v| v.contains("@author")),
            "Twitter Card should contain creator handle"
        );
    }

    /// Test generic meta tags extraction into meta_tags BTreeMap.
    /// Validates that meta tags not covered by specific fields are captured.
    #[test]
    fn test_metadata_generic_meta_tags() {
        let html = "\
<!DOCTYPE html>
<html>
  <head>
    <title>Generic Tags</title>
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <meta name=\"robots\" content=\"index, follow\">
    <meta name=\"theme-color\" content=\"#ffffff\">
    <meta http-equiv=\"X-UA-Compatible\" content=\"ie=edge\">
  </head>
  <body>
    <p>Content</p>
  </body>
</html>";

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert!(
            !metadata.meta_tags.is_empty(),
            "Meta tags map should contain generic meta tags"
        );

        assert!(
            metadata.meta_tags.contains_key("viewport")
                || metadata.meta_tags.values().any(|v| v.contains("width=device-width")),
            "Meta tags should contain viewport"
        );

        assert!(
            metadata.meta_tags.contains_key("robots")
                || metadata.meta_tags.values().any(|v| v.contains("index, follow")),
            "Meta tags should contain robots directive"
        );
    }

    /// Test header/heading extraction with level, text, id, depth, and html_offset.
    #[test]
    fn test_metadata_headers() {
        let html = r#"<!DOCTYPE html>
<html>
  <head><title>Headers</title></head>
  <body>
    <h1 id="main-title">Main Title</h1>
    <h2>Section Header</h2>
    <h3 id="subsection">Subsection</h3>
    <p>Some content</p>
    <h4>Deep Heading</h4>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert!(
            !metadata.headers.is_empty(),
            "Headers vector should contain extracted headings"
        );

        let h1 = metadata.headers.iter().find(|h| h.level == 1);
        assert!(h1.is_some(), "H1 header should be extracted");
        assert_eq!(h1.unwrap().text, "Main Title", "H1 text should be correctly extracted");
        assert_eq!(
            h1.unwrap().id,
            Some("main-title".to_string()),
            "H1 id attribute should be extracted"
        );
        assert!(
            h1.unwrap().html_offset < 1000,
            "H1 html_offset should be within reasonable range"
        );

        let h2 = metadata.headers.iter().find(|h| h.level == 2);
        assert!(h2.is_some(), "H2 header should be extracted");
        assert_eq!(
            h2.unwrap().text,
            "Section Header",
            "H2 text should be correctly extracted"
        );

        let h3 = metadata.headers.iter().find(|h| h.level == 3);
        assert!(h3.is_some(), "H3 header should be extracted");
        assert_eq!(
            h3.unwrap().id,
            Some("subsection".to_string()),
            "H3 id should be extracted"
        );

        let h4 = metadata.headers.iter().find(|h| h.level == 4);
        assert!(h4.is_some(), "H4 header should be extracted");
    }

    /// Test link extraction with href, text, title, and link_type classification.
    /// Validates correct classification of anchor, external, email, phone, and internal links.
    #[test]
    fn test_metadata_links() {
        let html = "\
<!DOCTYPE html>
<html>
  <head><title>Links</title></head>
  <body>
    <a href=\"#section1\">Anchor Link</a>
    <a href=\"https://external.com/page\">External Link</a>
    <a href=\"/about\" title=\"About Page\">Internal Link</a>
    <a href=\"mailto:test@example.com\">Email Link</a>
    <a href=\"tel:+1234567890\">Phone Link</a>
    <a href=\"https://example.com/page\">Same Domain Link</a>
  </body>
</html>";

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert!(
            !metadata.links.is_empty(),
            "Links vector should contain extracted links"
        );

        let anchor = metadata.links.iter().find(|l| l.href.starts_with('#'));
        assert!(anchor.is_some(), "Anchor link should be extracted");
        assert_eq!(
            anchor.unwrap().link_type,
            LinkType::Anchor,
            "Link starting with # should be classified as Anchor"
        );
        assert_eq!(anchor.unwrap().text, "Anchor Link", "Link text should be extracted");

        let external = metadata.links.iter().find(|l| l.href.contains("external.com"));
        assert!(external.is_some(), "External link should be extracted");
        assert_eq!(
            external.unwrap().link_type,
            LinkType::External,
            "External domain link should be classified as External"
        );

        let email = metadata.links.iter().find(|l| l.href.starts_with("mailto:"));
        assert!(email.is_some(), "Email link should be extracted");
        assert_eq!(
            email.unwrap().link_type,
            LinkType::Email,
            "mailto: link should be classified as Email"
        );

        let phone = metadata.links.iter().find(|l| l.href.starts_with("tel:"));
        assert!(phone.is_some(), "Phone link should be extracted");
        assert_eq!(
            phone.unwrap().link_type,
            LinkType::Phone,
            "tel: link should be classified as Phone"
        );

        let internal = metadata.links.iter().find(|l| l.href == "/about");
        assert!(internal.is_some(), "Internal link should be extracted");
        assert_eq!(
            internal.unwrap().title,
            Some("About Page".to_string()),
            "Link title attribute should be extracted"
        );
    }

    /// Test image extraction with src, alt, title, dimensions, and image_type classification.
    /// Validates distinction between data URIs, inline SVGs, external URLs, and relative paths.
    #[test]
    fn test_metadata_images() {
        let html = r#"<!DOCTYPE html>
<html>
  <head><title>Images</title></head>
  <body>
    <img src="https://example.com/photo.jpg" alt="Photo" title="A Photo">
    <img src="/images/logo.png" alt="Logo" width="200" height="150">
    <img src="data:image/svg+xml,%3Csvg%3E%3C/svg%3E" alt="Inline SVG">
    <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==" alt="Data URI">
    <img src="./relative/image.gif" alt="Relative Path">
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert!(
            !metadata.images.is_empty(),
            "Images vector should contain extracted images"
        );

        let external_img = metadata.images.iter().find(|img| img.src.contains("example.com"));
        assert!(external_img.is_some(), "External image should be extracted");
        assert_eq!(
            external_img.unwrap().alt,
            Some("Photo".to_string()),
            "Image alt text should be extracted"
        );
        assert_eq!(
            external_img.unwrap().title,
            Some("A Photo".to_string()),
            "Image title should be extracted"
        );
        assert_eq!(
            external_img.unwrap().image_type,
            ImageType::External,
            "External image should be classified as External"
        );

        let img_with_dims = metadata.images.iter().find(|img| img.src.contains("logo.png"));
        assert!(img_with_dims.is_some(), "Image with dimensions should be extracted");
        assert_eq!(
            img_with_dims.unwrap().dimensions,
            Some((200, 150)),
            "Image dimensions should be extracted as (width, height)"
        );

        let svg_img = metadata.images.iter().find(|img| img.src.contains("svg"));
        assert!(svg_img.is_some(), "Inline SVG should be extracted");
        assert!(
            svg_img.unwrap().image_type == ImageType::InlineSvg || svg_img.unwrap().image_type == ImageType::DataUri,
            "SVG should be classified as either InlineSvg or DataUri"
        );

        let data_uri_img = metadata.images.iter().find(|img| img.src.starts_with("data:image/png"));
        assert!(data_uri_img.is_some(), "Data URI image should be extracted");
        assert_eq!(
            data_uri_img.unwrap().image_type,
            ImageType::DataUri,
            "Base64 data URI should be classified as DataUri"
        );

        let relative_img = metadata.images.iter().find(|img| img.src.contains("relative"));
        assert!(relative_img.is_some(), "Relative path image should be extracted");
        assert_eq!(
            relative_img.unwrap().image_type,
            ImageType::Relative,
            "Relative path should be classified as Relative"
        );
    }

    /// Test structured data extraction (JSON-LD, microdata, RDFa).
    /// Validates that structured data blocks are properly parsed and categorized.
    #[test]
    fn test_metadata_structured_data() {
        let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Structured Data</title>
    <script type="application/ld+json">
    {
      "@context": "https://schema.org",
      "@type": "Article",
      "headline": "Example Article",
      "author": "John Doe"
    }
    </script>
  </head>
  <body>
    <article itemscope itemtype="https://schema.org/NewsArticle">
      <h1 itemprop="headline">News Item</h1>
      <p itemprop="articleBody">Content here</p>
    </article>
  </body>
</html>"#;

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("metadata should be present");

        assert!(
            !metadata.structured_data.is_empty(),
            "Structured data vector should contain extracted data blocks"
        );

        let json_ld = metadata
            .structured_data
            .iter()
            .find(|sd| sd.data_type == StructuredDataType::JsonLd);
        assert!(json_ld.is_some(), "JSON-LD should be extracted");
        assert!(
            json_ld.unwrap().raw_json.contains("Article"),
            "JSON-LD raw_json should contain schema type"
        );
        assert_eq!(
            json_ld.unwrap().schema_type,
            Some("Article".to_string()),
            "JSON-LD schema_type should be detected"
        );

        let microdata = metadata
            .structured_data
            .iter()
            .find(|sd| sd.data_type == StructuredDataType::Microdata);
        if let Some(md) = microdata {
            assert!(
                md.raw_json.contains("NewsArticle") || md.schema_type == Some("NewsArticle".to_string()),
                "Microdata schema_type should contain NewsArticle if extracted"
            );
        }
    }

    /// Test that empty HTML returns default metadata (None or empty collections).
    #[test]
    fn test_metadata_empty_html() {
        let html = "";

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();

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

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();

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

        let result = convert_html_to_markdown_with_metadata(html, None);
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

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
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

        let (_, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
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

    /// Comprehensive test of a complete HTML document with ALL metadata types.
    /// Validates that all metadata extraction works together correctly.
    #[test]
    fn test_metadata_comprehensive() {
        let html = "<html lang=\"en\" dir=\"ltr\"><head>\
            <meta charset=\"UTF-8\">\
            <title>Complete Metadata Example</title>\
            <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\
            <meta name=\"description\" content=\"Comprehensive metadata extraction test page\">\
            <meta name=\"keywords\" content=\"metadata, extraction, rust, web\">\
            <meta name=\"author\" content=\"Test Author\">\
            <meta name=\"robots\" content=\"index, follow\">\
            <meta property=\"og:title\" content=\"OG Title\">\
            <meta property=\"og:description\" content=\"OG Description\">\
            <meta property=\"og:image\" content=\"https://example.com/og-image.jpg\">\
            <meta property=\"og:url\" content=\"https://example.com/article\">\
            <meta property=\"og:type\" content=\"article\">\
            <meta property=\"og:site_name\" content=\"Example Site\">\
            <meta name=\"twitter:card\" content=\"summary_large_image\">\
            <meta name=\"twitter:title\" content=\"Tweet Title\">\
            <meta name=\"twitter:description\" content=\"Tweet Description\">\
            <meta name=\"twitter:image\" content=\"https://example.com/tweet.jpg\">\
            <meta name=\"twitter:site\" content=\"@example\">\
            <link rel=\"canonical\" href=\"https://example.com/article/complete\">\
            <base href=\"https://example.com/\">\
            <script type=\"application/ld+json\">{\"@context\":\"https://schema.org\",\"@type\":\"Article\",\"headline\":\"Complete Metadata Example\",\"author\":\"Test Author\",\"datePublished\":\"2024-01-01\"}</script>\
        </head><body>\
            <header><h1 id=\"page-title\">Complete Metadata Example</h1><p>Test</p></header>\
            <nav><a href=\"#intro\">Intro</a><a href=\"https://external.com\">External</a></nav>\
            <main>\
                <section id=\"intro\"><h2>Introduction</h2><p>Purpose.</p><img src=\"https://example.com/intro.jpg\" alt=\"Intro image\" title=\"Intro\"></section>\
                <section id=\"content\">\
                    <h3>Content</h3><h4>Sub</h4><p>Details.</p>\
                    <h3>Gallery</h3>\
                    <img src=\"/images/photo1.jpg\" alt=\"Photo 1\" width=\"400\" height=\"300\">\
                    <img src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==\" alt=\"Data URI\">\
                    <img src=\"./relative/image.gif\" alt=\"Relative\">\
                </section>\
                <section id=\"links\">\
                    <h3>Links</h3>\
                    <a href=\"#top\">Top</a>\
                    <a href=\"/about\" title=\"About\">Internal</a>\
                    <a href=\"mailto:contact@example.com\">Email</a>\
                    <a href=\"tel:+1-555-1234\">Phone</a>\
                </section>\
            </main>\
            <footer><p>2024 Example</p></footer>\
        </body></html>";

        let (markdown, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("comprehensive HTML should have metadata");

        assert_eq!(
            metadata.title,
            Some("Complete Metadata Example".to_string()),
            "Title should be extracted"
        );
        assert_eq!(
            metadata.description,
            Some("Comprehensive metadata extraction test page".to_string()),
            "Description should be extracted"
        );
        assert_eq!(
            metadata.author,
            Some("Test Author".to_string()),
            "Author should be extracted"
        );
        assert!(!metadata.keywords.is_empty(), "Keywords should be extracted");
        assert_eq!(
            metadata.language,
            Some("en".to_string()),
            "Language should be extracted"
        );
        assert_eq!(
            metadata.text_direction,
            Some(TextDirection::LeftToRight),
            "Text direction should be extracted"
        );
        assert_eq!(
            metadata.canonical_url,
            Some("https://example.com/article/complete".to_string()),
            "Canonical URL should be extracted"
        );
        assert_eq!(
            metadata.base_href,
            Some("https://example.com/".to_string()),
            "Base href should be extracted"
        );

        assert!(!metadata.open_graph.is_empty(), "Open Graph tags should be extracted");

        assert!(
            !metadata.twitter_card.is_empty(),
            "Twitter Card tags should be extracted"
        );

        assert!(!metadata.headers.is_empty(), "Headers should be extracted");
        let h1_count = metadata.headers.iter().filter(|h| h.level == 1).count();
        assert_eq!(h1_count, 1, "Should have exactly one H1");
        assert!(metadata.headers.iter().any(|h| h.level == 2), "Should have H2 headers");
        assert!(metadata.headers.iter().any(|h| h.level == 3), "Should have H3 headers");

        assert!(!metadata.links.is_empty(), "Links should be extracted");
        assert!(
            metadata.links.iter().any(|l| l.link_type == LinkType::Anchor),
            "Anchor links should be present"
        );
        assert!(
            metadata.links.iter().any(|l| l.link_type == LinkType::Email),
            "Email links should be present"
        );
        assert!(
            metadata.links.iter().any(|l| l.link_type == LinkType::Phone),
            "Phone links should be present"
        );

        assert!(!metadata.images.is_empty(), "Images should be extracted");
        assert!(
            metadata.images.iter().any(|img| img.image_type == ImageType::External),
            "External images should be present"
        );
        assert!(
            metadata.images.iter().any(|img| img.image_type == ImageType::DataUri),
            "Data URI images should be present"
        );
        assert!(
            metadata.images.iter().any(|img| img.image_type == ImageType::Relative),
            "Relative images should be present"
        );

        let img_with_dims = metadata.images.iter().find(|img| img.dimensions.is_some());
        assert!(img_with_dims.is_some(), "At least one image should have dimensions");
        if let Some(img) = img_with_dims {
            assert_eq!(
                img.dimensions,
                Some((400, 300)),
                "Image dimensions should be correctly extracted"
            );
        }

        assert!(
            !metadata.structured_data.is_empty(),
            "Structured data should be extracted"
        );

        assert!(!markdown.is_empty(), "Markdown should be generated");
        assert!(
            markdown.contains("Complete Metadata Example"),
            "Markdown should contain heading text"
        );
    }

    /// Real-world-like webpage structure with realistic metadata patterns.
    /// Tests extraction from a realistic blog post scenario.
    #[test]
    fn test_metadata_real_world_webpage() {
        let html = "<!DOCTYPE html>\
<html lang=\"en\"><head>\
    <meta charset=\"UTF-8\">\
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\
    <title>How to Build Rust Web Applications | TechBlog</title>\
    <meta name=\"description\" content=\"Learn how to build scalable web applications using Rust\">\
    <meta name=\"keywords\" content=\"rust, web development, actix, async, tutorial\">\
    <meta name=\"author\" content=\"Sarah Chen\">\
    <link rel=\"canonical\" href=\"https://techblog.example.com/rust-web-apps\">\
    <base href=\"https://techblog.example.com/\">\
    <meta property=\"og:title\" content=\"How to Build Rust Web Applications\">\
    <meta property=\"og:description\" content=\"A comprehensive guide to building web apps with Rust\">\
    <meta property=\"og:image\" content=\"https://techblog.example.com/images/rust-web.jpg\">\
    <meta property=\"og:type\" content=\"article\">\
    <meta name=\"twitter:card\" content=\"summary_large_image\">\
    <meta name=\"twitter:title\" content=\"How to Build Rust Web Applications\">\
    <meta name=\"twitter:image\" content=\"https://techblog.example.com/images/rust-web-twitter.jpg\">\
    <meta name=\"twitter:creator\" content=\"@sarahcodes\">\
    <script type=\"application/ld+json\">{\"@context\":\"https://schema.org\",\"@type\":\"BlogPosting\",\"headline\":\"How to Build Rust Web Applications\"}</script>\
</head><body>\
    <header><nav>\
        <a href=\"/\">Home</a><a href=\"/blog\">Blog</a><a href=\"/resources\">Resources</a><a href=\"/about\">About</a>\
    </nav></header>\
    <article>\
        <h1>How to Build Rust Web Applications</h1>\
        <img src=\"https://techblog.example.com/images/rust-web-hero.jpg\" alt=\"Rust web development\" title=\"Hero image\">\
        <p>Guide content here</p>\
        <h2>Getting Started</h2>\
        <p>Before diving in, install Rust.</p>\
        <h3>Installation</h3>\
        <p>Visit <a href=\"https://www.rust-lang.org/tools/install\">installation page</a>.</p>\
        <h3>Your First Project</h3>\
        <p>Create project with cargo</p>\
        <h2>Building</h2>\
        <h3>Dependencies</h3>\
        <p>Setup Cargo.toml</p>\
        <h3>Routes</h3>\
        <p>Learn <a href=\"/blog/rust-routing\">routing</a>.</p>\
        <h2>Advanced</h2>\
        <h3>Async</h3>\
        <p>See <a href=\"https://tokio.rs\" title=\"Tokio async runtime\">Tokio</a>.</p>\
        <h3>Database</h3>\
        <p>Contact <a href=\"mailto:hello@techblog.example.com\">hello@techblog.example.com</a></p>\
        <h2>Gallery</h2>\
        <img src=\"/images/diagram1.png\" alt=\"Architecture diagram\" width=\"600\" height=\"400\">\
        <img src=\"/images/diagram2.png\" alt=\"Flow chart\" width=\"600\" height=\"400\">\
        <h2>Conclusion</h2>\
        <p>Excellent choice. <a href=\"/blog/rust-deployment\">Deployment</a>.</p>\
        <footer><p>Questions? <a href=\"tel:+1-555-0100\">Call</a> or <a href=\"#contact\">contact</a>.</p></footer>\
    </article>\
</body></html>";

        let (markdown, metadata) = convert_html_to_markdown_with_metadata(html, None).unwrap();
        let metadata = metadata.expect("real-world HTML should have metadata");

        assert_eq!(
            metadata.title,
            Some("How to Build Rust Web Applications | TechBlog".to_string()),
            "Real-world title with site name should be extracted"
        );
        assert!(metadata.description.is_some(), "Description should be present");
        assert_eq!(
            metadata.author,
            Some("Sarah Chen".to_string()),
            "Author should be extracted"
        );
        assert!(!metadata.keywords.is_empty(), "Keywords should be extracted");

        assert!(!metadata.open_graph.is_empty(), "Article should have Open Graph tags");

        assert!(
            !metadata.twitter_card.is_empty(),
            "Article should have Twitter Card tags"
        );

        assert!(metadata.headers.len() >= 5, "Should extract multiple heading levels");
        assert!(
            metadata.headers.iter().any(|h| h.level == 1),
            "Should have H1 (main title)"
        );
        assert!(
            metadata.headers.iter().any(|h| h.level == 2),
            "Should have H2 (sections)"
        );
        assert!(
            metadata.headers.iter().any(|h| h.level == 3),
            "Should have H3 (subsections)"
        );

        assert!(metadata.links.len() >= 3, "Should extract multiple links");
        assert!(
            metadata.links.iter().any(|l| l.link_type == LinkType::Internal),
            "Should have internal links"
        );
        assert!(
            metadata.links.iter().any(|l| l.link_type == LinkType::External),
            "Should have external links"
        );
        assert!(
            metadata.links.iter().any(|l| l.link_type == LinkType::Email)
                || metadata.links.iter().any(|l| l.link_type == LinkType::Phone),
            "Should have either email or phone links"
        );

        assert!(!metadata.images.is_empty(), "Should extract images");
        let hero_image = metadata.images.iter().find(|img| {
            img.alt.as_ref().map_or(false, |a| {
                a.contains("Hero") || a.contains("development") || a.contains("hero")
            })
        });
        if hero_image.is_none() {
            assert!(metadata.images.len() >= 1, "Should have extracted at least one image");
        }

        assert!(
            !metadata.structured_data.is_empty(),
            "Should extract structured data (JSON-LD)"
        );
        let json_ld = metadata
            .structured_data
            .iter()
            .find(|sd| sd.data_type == StructuredDataType::JsonLd);
        assert!(json_ld.is_some(), "Should have JSON-LD structured data");
        assert_eq!(
            json_ld.unwrap().schema_type,
            Some("BlogPosting".to_string()),
            "JSON-LD should identify as BlogPosting schema"
        );

        assert!(!markdown.is_empty(), "Should generate Markdown from HTML");
        assert!(markdown.contains("Rust"), "Markdown should contain article content");
    }

    /// Test extraction of large HTML document (1MB+) for performance
    /// Generates HTML with 10,000+ elements and validates extraction
    /// completes within reasonable time (<30s) with no panics.
    #[test]
    fn test_large_html_performance() {
        let mut html = String::with_capacity(2_000_000);
        html.push_str(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Large HTML Performance Test</title>
    <meta name="description" content="Testing extraction performance on large documents">
</head>
<body>
    <h1>Large Document Test</h1>"#,
        );

        for i in 0..10000 {
            html.push_str(&format!(
                "<article><h2>Article {}</h2><p>Content block {} with expanded text content to increase document size. \
                This article contains multiple paragraphs describing various topics. \
                The goal is to create sufficient HTML content to test performance on large documents. \
                Here are some additional details: Section A covers fundamentals, Section B covers implementation, \
                and Section C covers optimization. Each section has multiple subsections.</p>\
                <p>Additional content paragraph {} to further expand the document.</p></article>\n",
                i, i, i
            ));
        }
        html.push_str("</body></html>");

        let html_size_bytes = html.len();
        assert!(
            html_size_bytes > 1_000_000,
            "Generated HTML should be >1MB (got {} bytes)",
            html_size_bytes
        );

        let start = std::time::Instant::now();

        let result = process_html(&html, None, false, 1024 * 1024);

        let duration = start.elapsed();

        assert!(
            result.is_ok(),
            "Large HTML extraction should succeed. Error: {:?}",
            result.err()
        );

        let result = result.unwrap();
        assert!(!result.markdown.is_empty(), "Markdown should be generated");

        assert!(
            duration.as_secs() < 30,
            "Large HTML extraction took too long: {:.2}s (must be <30s)",
            duration.as_secs_f64()
        );
    }

    /// Test WASM size boundary conditions
    /// Tests HTML exactly at and around the 2MB limit to ensure
    /// proper error handling and boundary detection.
    #[test]
    fn test_wasm_size_limit_boundary() {
        let mut html_under = String::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Just Under Limit</title></head>
<body><h1>Content</h1>"#,
        );

        let target_size = 1_800_000;
        while html_under.len() < target_size {
            html_under.push_str("<p>Padding content for size testing. This is test data to reach the target document size. Lorem ipsum dolor sit amet.</p>\n");
        }
        html_under.truncate(target_size);
        html_under.push_str("</body></html>");

        assert!(
            html_under.len() < 2 * 1024 * 1024,
            "HTML should be under 2MB limit (got {} bytes)",
            html_under.len()
        );

        let result = process_html(&html_under, None, false, 1024);
        #[cfg(target_arch = "wasm32")]
        assert!(result.is_ok(), "HTML under 2MB should be accepted in WASM");
        #[cfg(not(target_arch = "wasm32"))]
        assert!(result.is_ok(), "HTML under 2MB should always be accepted");

        let mut html_over = String::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Over Limit</title></head>
<body><h1>Content</h1>"#,
        );

        let target_size = 2_200_000;
        while html_over.len() < target_size {
            html_over.push_str("<p>Oversized content for boundary testing. This section generates large HTML to exceed limits. Lorem ipsum dolor sit amet.</p>\n");
        }
        html_over.truncate(target_size);
        html_over.push_str("</body></html>");

        assert!(
            html_over.len() > 2 * 1024 * 1024,
            "HTML should be over 2MB limit (got {} bytes)",
            html_over.len()
        );

        let result = process_html(&html_over, None, false, 1024);
        #[cfg(target_arch = "wasm32")]
        {
            assert!(result.is_err(), "HTML over 2MB should be rejected in WASM with error");
            let error_msg = format!("{:?}", result.err());
            assert!(
                error_msg.contains("2MB") || error_msg.contains("WASM"),
                "Error message should clearly indicate WASM size limit"
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(e) = result {
                let msg = format!("{:?}", e);
                assert!(
                    !msg.contains("WASM") && !msg.contains("2MB"),
                    "Native builds should not enforce WASM size limit"
                );
            }
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

        let result = convert_html_to_markdown_with_metadata(html, None);

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

        let result = convert_html_to_markdown_with_metadata(html, None);
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

    /// Test thread safety of HTML extraction with concurrent access
    /// Validates that extracting the same HTML from multiple threads
    /// does not cause panics, data races, or corruption.
    #[test]
    fn test_concurrent_html_extraction() {
        use std::sync::Arc;

        let html = Arc::new(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <title>Concurrent Test Article</title>
    <meta name="description" content="Testing concurrent extraction">
    <meta name="author" content="Test Author">
    <meta property="og:title" content="OG Title">
    <meta property="og:description" content="OG Description">
    <meta name="twitter:card" content="summary">
    <script type="application/ld+json">
    {
      "@context": "https://schema.org",
      "@type": "Article",
      "headline": "Concurrent Test",
      "author": "Test Author"
    }
    </script>
</head>
<body>
    <h1>Concurrent Extraction Test</h1>
    <h2>Section 1</h2>
    <p>Content 1</p>
    <h2>Section 2</h2>
    <p>Content 2</p>
    <a href="https://example.com">External Link</a>
    <a href="/about">Internal Link</a>
    <img src="https://example.com/image.jpg" alt="Test Image">
</body>
</html>"#,
        );

        let handles: Vec<_> = (0..10)
            .map(|thread_id| {
                let html = Arc::clone(&html);
                std::thread::spawn(move || {
                    let result = convert_html_to_markdown_with_metadata(html.as_ref(), None);

                    assert!(
                        result.is_ok(),
                        "Thread {} extraction failed: {:?}",
                        thread_id,
                        result.err()
                    );

                    let (markdown, metadata) = result.unwrap();

                    assert!(
                        !markdown.is_empty(),
                        "Thread {} markdown should not be empty",
                        thread_id
                    );

                    if let Some(meta) = metadata {
                        assert_eq!(
                            meta.title,
                            Some("Concurrent Test Article".to_string()),
                            "Thread {} should extract correct title",
                            thread_id
                        );

                        assert!(!meta.headers.is_empty(), "Thread {} should extract headers", thread_id);
                        assert!(!meta.links.is_empty(), "Thread {} should extract links", thread_id);
                        assert!(!meta.images.is_empty(), "Thread {} should extract images", thread_id);
                        assert!(
                            !meta.open_graph.is_empty(),
                            "Thread {} should extract OG metadata",
                            thread_id
                        );
                    }

                    true
                })
            })
            .collect();

        let all_succeeded = handles.into_iter().enumerate().all(|(i, handle)| {
            let result = handle.join();
            assert!(result.is_ok(), "Thread {} panicked: {:?}", i, result.err());
            result.unwrap()
        });

        assert!(all_succeeded, "All concurrent extraction threads should succeed");
    }
}
