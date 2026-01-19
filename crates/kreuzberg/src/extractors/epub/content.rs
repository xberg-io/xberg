//! EPUB content extraction and text processing.
//!
//! Handles extraction of text content from XHTML files in spine order,
//! with markdown conversion and HTML cleaning utilities.

use crate::Result;
use std::io::Cursor;
use zip::ZipArchive;

use super::metadata::parse_opf;
use super::parsing::{read_file_from_zip, resolve_path};

/// Extract text content from an EPUB document by reading in spine order
pub(super) fn extract_content(
    archive: &mut ZipArchive<Cursor<Vec<u8>>>,
    opf_path: &str,
    manifest_dir: &str,
) -> Result<String> {
    let opf_xml = read_file_from_zip(archive, opf_path)?;
    let (_, spine_hrefs) = parse_opf(&opf_xml)?;

    let mut content = String::new();

    for (index, href) in spine_hrefs.iter().enumerate() {
        let file_path = resolve_path(manifest_dir, href);

        match read_file_from_zip(archive, &file_path) {
            Ok(xhtml_content) => {
                let text = extract_text_from_xhtml(&xhtml_content);
                if !text.is_empty() {
                    if index > 0 && !content.ends_with('\n') {
                        content.push('\n');
                    }
                    content.push_str(&text);
                    content.push('\n');
                }
            }
            Err(_) => {
                continue;
            }
        }
    }

    Ok(content.trim().to_string())
}

/// Extract text from XHTML content using html-to-markdown-rs
pub(super) fn extract_text_from_xhtml(xhtml: &str) -> String {
    match crate::extraction::html::convert_html_to_markdown(xhtml, None) {
        Ok(markdown) => {
            let text = markdown_to_plain_text(&markdown);
            remove_html_comments(&text)
        }
        Err(_) => strip_html_tags(xhtml),
    }
}

/// Remove HTML comments from text
pub(super) fn remove_html_comments(text: &str) -> String {
    let mut result = String::new();
    let mut in_comment = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if !in_comment && ch == '<' {
            if chars.peek() == Some(&'!') {
                chars.next();
                if chars.peek() == Some(&'-') {
                    chars.next();
                    if chars.peek() == Some(&'-') {
                        chars.next();
                        in_comment = true;
                        continue;
                    } else {
                        result.push('<');
                        result.push('!');
                        result.push('-');
                        continue;
                    }
                } else {
                    result.push('<');
                    result.push('!');
                    continue;
                }
            } else {
                result.push(ch);
            }
        } else if in_comment {
            if ch == '-' && chars.peek() == Some(&'-') {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    in_comment = false;
                    result.push('\n');
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Convert markdown output to plain text by removing markdown syntax
pub(super) fn markdown_to_plain_text(markdown: &str) -> String {
    let mut text = String::new();
    let mut in_code_block = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            continue;
        }

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            text.push_str(trimmed);
            text.push('\n');
            continue;
        }

        let cleaned = if let Some(stripped) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            stripped
        } else if let Some(stripped) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
            if let Some(rest) = stripped.strip_prefix(". ") {
                rest
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        let cleaned = cleaned.trim_start_matches('#').trim();

        let cleaned = cleaned
            .replace("**", "")
            .replace("__", "")
            .replace("*", "")
            .replace("_", "");

        let cleaned = remove_markdown_links(&cleaned);

        if !cleaned.is_empty() {
            text.push_str(&cleaned);
            text.push('\n');
        }
    }

    text.trim().to_string()
}

/// Remove markdown links [text](url) -> text
pub(super) fn remove_markdown_links(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' {
            let mut link_text = String::new();
            let mut depth = 1;

            while let Some(&next_ch) = chars.peek() {
                chars.next();
                if next_ch == '[' {
                    depth += 1;
                    link_text.push(next_ch);
                } else if next_ch == ']' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    link_text.push(next_ch);
                } else {
                    link_text.push(next_ch);
                }
            }

            if let Some(&'(') = chars.peek() {
                chars.next();
                let mut paren_depth = 1;
                while let Some(&next_ch) = chars.peek() {
                    chars.next();
                    if next_ch == '(' {
                        paren_depth += 1;
                    } else if next_ch == ')' {
                        paren_depth -= 1;
                        if paren_depth == 0 {
                            break;
                        }
                    }
                }
            }

            result.push_str(&link_text);
        } else {
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

    #[test]
    fn test_remove_markdown_links() {
        let text = "This is a [link](http://example.com) in text";
        let result = remove_markdown_links(text);
        assert!(result.contains("link"));
        assert!(!result.contains("http://"));
    }

    #[test]
    fn test_markdown_to_plain_text_removes_formatting() {
        let markdown = "# Heading\n\nThis is **bold** text with _italic_ emphasis.";
        let result = markdown_to_plain_text(markdown);
        assert!(result.contains("Heading"));
        assert!(result.contains("bold"));
        assert!(!result.contains("**"));
    }

    #[test]
    fn test_markdown_to_plain_text_removes_list_markers() {
        let markdown = "- Item 1\n- Item 2\n* Item 3";
        let result = markdown_to_plain_text(markdown);
        assert!(result.contains("Item 1"));
        assert!(result.contains("Item 2"));
        assert!(result.contains("Item 3"));
    }
}
