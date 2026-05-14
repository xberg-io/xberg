//! Shared utilities for Markdown and MDX extractors.
//!
//! Provides common pulldown-cmark event processing logic used by both
//! the Markdown and MDX extractors.

use crate::types::ExtractedImage;
use base64::Engine;
use bytes::Bytes;
use pulldown_cmark::{Event, Tag, TagEnd};
use std::borrow::Cow;

/// Extract plain text from a pulldown-cmark event slice, collecting data URI images.
///
/// Processes markdown AST events and produces a plain-text representation that:
/// - Preserves heading structure with `#` prefixes
/// - Appends link URLs in parentheses
/// - Annotates images with `[Image: url]`
/// - Preserves fenced code blocks with language tags
/// - Decodes base64 data URI images into `ExtractedImage` values
///
/// # Arguments
///
/// * `events` - Slice of pulldown-cmark events to process
/// * `images` - Mutable vector into which decoded data URI images are pushed
///
/// # Returns
///
/// A `String` containing the extracted plain text.
pub(crate) fn extract_text_from_events(events: &[Event], images: &mut Vec<ExtractedImage>) -> String {
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
                    && let Some(image) = decode_data_uri_image(dest_url, images.len() as u32)
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
            Event::Start(Tag::Paragraph) if !in_heading => {
                text.push('\n');
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
///
/// # Arguments
///
/// * `uri` - The full `data:image/...;base64,...` URI string
/// * `index` - The zero-based image index within the document
pub(crate) fn decode_data_uri_image(uri: &str, index: u32) -> Option<ExtractedImage> {
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

    // Classify image based on metadata and visual properties
    let (image_kind, kind_confidence) =
        crate::extraction::image_kind::classify(&decoded, format, None, None, None, None, false);

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
        bounding_box: None,
        source_path: None,
        image_kind: Some(image_kind),
        kind_confidence: Some(kind_confidence),
        cluster_id: None,
    })
}
