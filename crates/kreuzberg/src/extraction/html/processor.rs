//! HTML processing pipeline with optional image extraction.
//!
//! This module provides the main processing functions that tie together
//! HTML conversion, image extraction, and metadata handling.
//!
//! Note: `process_html` and helpers are currently only exercised by tests in this module.

use super::converter::{convert_html_to_markdown, resolve_conversion_options};
use super::image_handling::inline_image_to_extracted;
use super::stack_management::check_wasm_size_limit;
#[cfg(not(target_arch = "wasm32"))]
use super::stack_management::{html_requires_large_stack, run_on_dedicated_stack};
use super::types::HtmlExtractionResult;
use crate::core::config::OutputFormat as KreuzbergOutputFormat;
use crate::error::{KreuzbergError, Result};
use html_to_markdown_rs::{ConversionOptions, InlineImageConfig as LibInlineImageConfig, convert_with_inline_images};

/// Convert HTML with inline image extraction.
///
/// Internal helper that performs the actual image extraction.
fn convert_inline_images_with_options(
    html: &str,
    options: ConversionOptions,
    image_config: LibInlineImageConfig,
) -> Result<html_to_markdown_rs::HtmlExtraction> {
    convert_with_inline_images(html, Some(options), image_config, None)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to convert HTML to Markdown with images: {}", e)))
}

/// Convert HTML with inline images using a dedicated stack.
///
/// On native platforms, uses a dedicated thread with larger stack size.
#[cfg(not(target_arch = "wasm32"))]
fn convert_inline_images_with_large_stack(
    html: String,
    options: ConversionOptions,
    image_config: LibInlineImageConfig,
) -> Result<html_to_markdown_rs::HtmlExtraction> {
    run_on_dedicated_stack(move || convert_inline_images_with_options(&html, options, image_config))
}

/// Process HTML with optional image extraction and output format support.
///
/// This is the main entry point for HTML processing. It handles:
/// - Size limit validation (WASM builds)
/// - Stack management for large documents (native builds)
/// - Image extraction if requested
/// - Warning collection
/// - Output format selection (markdown or djot)
///
/// # WASM Limitations
///
/// In WASM builds, HTML files larger than 2MB will be rejected to prevent stack overflow.
///
/// # Arguments
///
/// * `html` - The HTML string to process
/// * `options` - Optional conversion options
/// * `extract_images` - Whether to extract inline images
/// * `max_image_size` - Maximum size in bytes for extracted images
/// * `output_format` - Desired output format (markdown or djot)
///
/// # Returns
///
/// An HtmlExtractionResult containing the markdown/djot content, extracted images, and any warnings
pub fn process_html(
    html: &str,
    options: Option<ConversionOptions>,
    extract_images: bool,
    max_image_size: u64,
    output_format: KreuzbergOutputFormat,
) -> Result<HtmlExtractionResult> {
    check_wasm_size_limit(html)?;

    if extract_images {
        let options = resolve_conversion_options(options.clone(), output_format);
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
        let content = convert_html_to_markdown(html, options, Some(output_format))?;

        Ok(HtmlExtractionResult {
            markdown: content,
            images: Vec::new(),
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ImageType, LinkType, StructuredDataType, TextDirection};

    #[test]
    fn test_process_html_without_images() {
        let html = "<h1>Test</h1><p>Content</p>";
        let result = process_html(html, None, false, 1024 * 1024, KreuzbergOutputFormat::Markdown).unwrap();
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
        let result = process_html(html, Some(options), true, 1024 * 1024, KreuzbergOutputFormat::Markdown).unwrap();
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].format, "png");
    }

    #[test]
    fn test_process_html_empty_string() {
        let result = process_html("", None, false, 1024, KreuzbergOutputFormat::Markdown).unwrap();
        assert!(result.markdown.is_empty() || result.markdown.trim().is_empty());
        assert!(result.images.is_empty());
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

        let result = process_html(&html, None, false, 1024 * 1024, KreuzbergOutputFormat::Markdown);

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

        let result = process_html(&html_under, None, false, 1024, KreuzbergOutputFormat::Markdown);
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

        let result = process_html(&html_over, None, false, 1024, KreuzbergOutputFormat::Markdown);
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
                    let result =
                        super::super::converter::convert_html_to_markdown_with_metadata(html.as_ref(), None, None);

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

        let (markdown, metadata) =
            super::super::converter::convert_html_to_markdown_with_metadata(html, None, None).unwrap();
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

        let (markdown, metadata) =
            super::super::converter::convert_html_to_markdown_with_metadata(html, None, None).unwrap();
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
            img.alt
                .as_ref()
                .is_some_and(|a| a.contains("Hero") || a.contains("development") || a.contains("hero"))
        });
        if hero_image.is_none() {
            assert!(!metadata.images.is_empty(), "Should have extracted at least one image");
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
}
