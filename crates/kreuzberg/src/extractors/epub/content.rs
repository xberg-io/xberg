//! EPUB content extraction and text processing.
//!
//! Handles extraction of text content from XHTML files in spine order.
//!
//! For `OutputFormat::Plain`, this uses direct XML tree traversal to avoid lossy conversions.
//! For `OutputFormat::Markdown` / `OutputFormat::Djot`, this converts XHTML through the HTML
//! conversion pipeline so structural elements (like headings) are preserved.

use crate::Result;
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::types::ProcessingWarning;
use std::io::Cursor;
use zip::ZipArchive;

use super::metadata::parse_opf;
use super::parsing::{read_file_from_zip, resolve_path};

fn trim_trailing_newlines(s: &str) -> &str {
    s.trim_end_matches(['\n', '\r'])
}

/// Extract text content from an EPUB document by reading in spine order
pub(super) fn extract_content(
    archive: &mut ZipArchive<Cursor<Vec<u8>>>,
    opf_path: &str,
    manifest_dir: &str,
    config: &ExtractionConfig,
) -> Result<(String, bool, Vec<ProcessingWarning>)> {
    let opf_xml = read_file_from_zip(archive, opf_path)?;
    let (_, spine_hrefs) = parse_opf(&opf_xml)?;

    let mut content = String::new();
    let wants_markup = matches!(config.output_format, OutputFormat::Markdown | OutputFormat::Djot);
    let mut warnings: Vec<ProcessingWarning> = Vec::new();
    let mut had_markup_fallback = false;

    for (index, href) in spine_hrefs.iter().enumerate() {
        let file_path = resolve_path(manifest_dir, href);

        match read_file_from_zip(archive, &file_path) {
            Ok(xhtml_content) => {
                // Preserve structural elements like headings by converting XHTML → Markdown/Djot
                // using the same conversion pipeline as the HTML extractor. Use the API variant
                // that disables YAML/frontmatter injection.
                let extracted = if wants_markup {
                    match crate::extraction::html::convert_html_to_markdown_with_metadata(
                        &xhtml_content,
                        config.html_options.clone(),
                        Some(config.output_format),
                    ) {
                        Ok((converted, _)) => converted,
                        Err(err) => {
                            had_markup_fallback = true;
                            warnings.push(ProcessingWarning {
                                source: "epub".to_string(),
                                message: format!(
                                    "XHTML conversion failed for spine item '{}'; falling back to plain text: {}",
                                    file_path, err
                                ),
                            });
                            extract_text_from_xhtml(&xhtml_content)
                        }
                    }
                } else {
                    // Default: plain text extraction (no markup syntax).
                    extract_text_from_xhtml(&xhtml_content)
                };

                let extracted = if wants_markup {
                    trim_trailing_newlines(&extracted)
                } else {
                    extracted.trim_end()
                };

                if !extracted.is_empty() {
                    if index > 0 && !content.ends_with('\n') {
                        content.push('\n');
                    }
                    content.push_str(extracted);
                    content.push('\n');
                }
            }
            Err(err) => {
                warnings.push(ProcessingWarning {
                    source: "epub".to_string(),
                    message: format!("Failed to read spine item '{}' from EPUB archive: {}", file_path, err),
                });
                continue;
            }
        }
    }

    let content = trim_trailing_newlines(&content).to_string();
    let fully_converted = wants_markup && !had_markup_fallback;
    Ok((content, fully_converted, warnings))
}

/// Block-level HTML/XHTML elements that should produce newlines before/after their content.
const BLOCK_ELEMENTS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "caption",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "legend",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "ul",
];

/// Elements whose entire subtree should be skipped (no text extracted).
const SKIP_ELEMENTS: &[&str] = &[
    "head", "script", "style", "svg", "math", "video", "audio", "source", "track", "object", "embed", "iframe",
];

/// Extract text from XHTML content by traversing the XML tree directly.
///
/// This avoids the double lossy conversion XHTML → markdown → plain-text that
/// previously stripped underscores, asterisks, and numeric content. Instead,
/// text nodes are collected verbatim from the parse tree, with newlines inserted
/// at block-level element boundaries.
pub(super) fn extract_text_from_xhtml(xhtml: &str) -> String {
    // Try direct XML tree traversal first (lossless path).
    if let Some((text, _)) = try_extract_via_roxmltree(xhtml) {
        return text;
    }

    // Fallback: strip HTML tags character-by-character.
    strip_html_tags(xhtml)
}

/// Attempt to extract plain text via `roxmltree` XML parsing.
///
/// Returns `None` if the document cannot be parsed as XML/XHTML.
fn try_extract_via_roxmltree(xhtml: &str) -> Option<(String, bool)> {
    // Remove DOCTYPE declaration to avoid XXE/DTD parsing issues with roxmltree.
    // roxmltree rejects DTDs for security, so we strip them before parsing.
    let sanitized = strip_doctype(xhtml);

    match roxmltree::Document::parse(&sanitized) {
        Ok(doc) => {
            let root = doc.root();

            let mut output = String::with_capacity(xhtml.len() / 2);
            visit_node(root, &mut output);

            // Normalise multiple consecutive blank lines to a single blank line.
            let result = collapse_blank_lines(&output);
            let result = result.trim().to_string();

            if result.is_empty() { None } else { Some((result, true)) }
        }
        Err(_) => None,
    }
}

/// Remove DOCTYPE declaration from XML/XHTML to allow safe parsing with roxmltree.
///
/// DTD declarations can cause security issues (XXE attacks), and roxmltree rejects them.
/// This function safely removes the DOCTYPE declaration while preserving the rest of the document.
fn strip_doctype(xml: &str) -> String {
    // Find and remove <!DOCTYPE ...> declarations. We need to handle nested brackets
    // in the DOCTYPE (e.g., <!DOCTYPE html [internal subset]>)
    let mut result = String::new();
    let mut in_doctype = false;
    let mut bracket_depth = 0;

    let mut chars = xml.chars().peekable();

    while let Some(ch) = chars.next() {
        if !in_doctype && ch == '<' {
            // Check if this starts a DOCTYPE
            let start_pos = result.len();
            result.push(ch);

            if chars.peek() == Some(&'!') {
                result.push(chars.next().unwrap());
                let next_chars: String = chars.clone().take(7).collect();
                if next_chars.starts_with("DOCTYPE") {
                    // This is a DOCTYPE declaration, skip it
                    in_doctype = true;
                    bracket_depth = 0;
                    // Consume the DOCTYPE keyword and everything up to the closing >
                    for c in chars.by_ref() {
                        if c == '[' {
                            bracket_depth += 1;
                        } else if c == ']' {
                            bracket_depth -= 1;
                        } else if c == '>' && bracket_depth == 0 {
                            in_doctype = false;
                            break;
                        }
                    }
                    // Remove what we added to result (the '<!')
                    result.truncate(start_pos);
                } else {
                    // Not a DOCTYPE, keep what we added
                }
            }
        } else if in_doctype {
            // Already in a DOCTYPE, continue consuming
            if ch == '[' {
                bracket_depth += 1;
            } else if ch == ']' {
                bracket_depth -= 1;
            } else if ch == '>' && bracket_depth == 0 {
                in_doctype = false;
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Recursively visit an XML node and append its text to `output`.
fn visit_node(node: roxmltree::Node<'_, '_>, output: &mut String) {
    match node.node_type() {
        roxmltree::NodeType::Text => {
            let text = node.text().unwrap_or("");
            // Normalise whitespace within a text run (collapse runs of
            // whitespace to single spaces) but keep the text itself intact.
            let normalised = normalise_inline_whitespace(text);
            if !normalised.is_empty() {
                // If the output already ends with a newline (or is empty),
                // trim leading spaces from this fragment to avoid spurious
                // indentation; otherwise append as-is.
                let fragment = if output.is_empty() || output.ends_with('\n') {
                    normalised.trim_start().to_string()
                } else {
                    normalised
                };
                if !fragment.is_empty() {
                    output.push_str(&fragment);
                }
            }
        }
        roxmltree::NodeType::Element => {
            let tag = node.tag_name().name().to_ascii_lowercase();

            // Skip elements whose content should never appear in plain text.
            if SKIP_ELEMENTS.iter().any(|&s| s == tag) {
                return;
            }

            // Self-closing elements that produce whitespace.
            if tag == "br" {
                output.push('\n');
                return;
            }
            if tag == "hr" {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                return;
            }

            let is_block = BLOCK_ELEMENTS.iter().any(|&s| s == tag);

            if is_block {
                // Ensure block starts on a new line.
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
            }

            // Recurse into children.
            for child in node.children() {
                visit_node(child, output);
            }

            if is_block {
                // Ensure block ends on a new line.
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
            }
        }
        roxmltree::NodeType::Root => {
            // Visit all children of the document root.
            for child in node.children() {
                visit_node(child, output);
            }
        }
        // Ignore PI, Comment, CDATA, etc.
        _ => {}
    }
}

/// Collapse runs of whitespace (spaces/tabs/newlines) inside a text node to a
/// single space, matching what a browser would render for inline content.
fn normalise_inline_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_ws = false;

    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' || ch == ' ' {
            if !prev_was_ws {
                result.push(' ');
            }
            prev_was_ws = true;
        } else {
            result.push(ch);
            prev_was_ws = false;
        }
    }

    result
}

/// Collapse three or more consecutive newlines into exactly two (one blank line).
fn collapse_blank_lines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut consecutive_newlines: usize = 0;

    for ch in text.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                result.push('\n');
            }
        } else {
            consecutive_newlines = 0;
            result.push(ch);
        }
    }

    result
}

/// Fallback: strip HTML tags without using specialized libraries
pub(super) fn strip_html_tags(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    let mut in_script_style = false;
    let mut tag_name = String::new();

    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
            tag_name.clear();
            continue;
        }

        if ch == '>' {
            in_tag = false;

            let tag_lower = tag_name.to_lowercase();
            if tag_lower.contains("script") || tag_lower.contains("style") {
                in_script_style = !tag_name.starts_with('/');
            }
            continue;
        }

        if in_tag {
            tag_name.push(ch);
            continue;
        }

        if in_script_style {
            continue;
        }

        if ch == '\n' || ch == '\r' || ch == '\t' || ch == ' ' {
            if !text.is_empty() && !text.ends_with(' ') {
                text.push(' ');
            }
        } else {
            text.push(ch);
        }
    }

    let mut result = String::new();
    let mut prev_space = false;
    for ch in text.chars() {
        if ch == ' ' {
            if !prev_space {
                result.push(ch);
            }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_tags_simple() {
        let html = "<html><body><p>Hello World</p></body></html>";
        let text = strip_html_tags(html);
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn test_strip_html_tags_with_scripts() {
        let html = "<body><p>Text</p><script>alert('bad');</script><p>More</p></body>";
        let text = strip_html_tags(html);
        assert!(!text.contains("bad"));
        assert!(text.contains("Text"));
        assert!(text.contains("More"));
    }

    #[test]
    fn test_strip_html_tags_with_styles() {
        let html = "<body><p>Text</p><style>.class { color: red; }</style><p>More</p></body>";
        let text = strip_html_tags(html);
        assert!(!text.to_lowercase().contains("color"));
        assert!(text.contains("Text"));
        assert!(text.contains("More"));
    }

    #[test]
    fn test_strip_html_tags_normalizes_whitespace() {
        let html = "<p>Hello   \n\t   World</p>";
        let text = strip_html_tags(html);
        assert!(text.contains("Hello") && text.contains("World"));
    }

    // --- Direct XHTML extraction tests ---

    #[test]
    fn test_extract_text_from_xhtml_basic() {
        let xhtml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Test</title></head>
  <body>
    <h1>Chapter One</h1>
    <p>This is paragraph text.</p>
  </body>
</html>"#;
        let result = extract_text_from_xhtml(xhtml);
        assert!(result.contains("Chapter One"), "got: {result}");
        assert!(result.contains("This is paragraph text."), "got: {result}");
        // head/title content should not appear in body text
        assert!(!result.contains("Test"), "head title should be excluded, got: {result}");
    }

    #[test]
    fn test_extract_text_from_xhtml_skips_script_style() {
        let xhtml = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <p>Visible text</p>
    <script>var x = 1;</script>
    <style>.c { color: red; }</style>
    <p>More visible</p>
  </body>
</html>"#;
        let result = extract_text_from_xhtml(xhtml);
        assert!(result.contains("Visible text"), "got: {result}");
        assert!(result.contains("More visible"), "got: {result}");
        assert!(!result.contains("var x"), "got: {result}");
        assert!(!result.contains("color"), "got: {result}");
    }

    #[test]
    fn test_extract_text_from_xhtml_preserves_underscores_and_numbers() {
        let xhtml = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <p>The value_count is 1,000 items worth 3.14 each.</p>
    <p>See http://example.com/path_to/resource for details.</p>
  </body>
</html>"#;
        let result = extract_text_from_xhtml(xhtml);
        assert!(result.contains("value_count"), "underscore preserved, got: {result}");
        assert!(result.contains("1,000"), "number preserved, got: {result}");
        assert!(result.contains("3.14"), "decimal preserved, got: {result}");
        assert!(
            result.contains("http://example.com/path_to/resource"),
            "URL preserved, got: {result}"
        );
    }

    #[test]
    fn test_extract_text_from_xhtml_block_elements_add_newlines() {
        let xhtml = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Heading</h1>
    <p>Paragraph one.</p>
    <p>Paragraph two.</p>
    <ul>
      <li>Item A</li>
      <li>Item B</li>
    </ul>
  </body>
</html>"#;
        let result = extract_text_from_xhtml(xhtml);
        assert!(result.contains("Heading"), "got: {result}");
        assert!(result.contains("Paragraph one."), "got: {result}");
        assert!(result.contains("Paragraph two."), "got: {result}");
        assert!(result.contains("Item A"), "got: {result}");
        assert!(result.contains("Item B"), "got: {result}");
        // The two paragraphs should be on different lines
        assert!(result.contains('\n'), "should have newlines, got: {result}");
    }

    #[test]
    fn test_extract_text_from_xhtml_inline_formatting_preserved() {
        let xhtml = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <p>This has <strong>bold</strong> and <em>italic</em> text.</p>
  </body>
</html>"#;
        let result = extract_text_from_xhtml(xhtml);
        // Text content should be preserved; no markdown syntax introduced
        assert!(result.contains("bold"), "got: {result}");
        assert!(result.contains("italic"), "got: {result}");
        assert!(!result.contains("**"), "no markdown bold, got: {result}");
        assert!(!result.contains('_'), "no markdown italic, got: {result}");
    }

    #[test]
    fn test_extract_text_from_xhtml_fallback_for_invalid_xml() {
        // Malformed XHTML that roxmltree cannot parse should fall back to tag stripping.
        let bad_xhtml = "<p>Hello <b>World</b> unclosed <p>second";
        let result = extract_text_from_xhtml(bad_xhtml);
        assert!(result.contains("Hello"), "got: {result}");
        assert!(result.contains("World"), "got: {result}");
    }

    #[test]
    fn test_normalise_inline_whitespace() {
        assert_eq!(normalise_inline_whitespace("hello   world"), "hello world");
        assert_eq!(normalise_inline_whitespace("  leading"), " leading");
        assert_eq!(normalise_inline_whitespace("trailing  "), "trailing ");
        assert_eq!(normalise_inline_whitespace("a\n\t b"), "a b");
    }

    #[test]
    fn test_collapse_blank_lines() {
        let input = "a\n\n\n\nb";
        let result = collapse_blank_lines(input);
        assert_eq!(result, "a\n\nb");

        let input2 = "a\n\nb";
        assert_eq!(collapse_blank_lines(input2), "a\n\nb");
    }
}
