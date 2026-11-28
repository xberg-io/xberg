//! Document structure for VLM OCR output that is already Markdown.
//!
//! Modern OCR VLMs answer in Markdown, with HTML `<table>` for merged cells. Handing that text to
//! the renderers as plain paragraphs escapes it (`\#`, `\<table\>`) and joins its lines, so the
//! page is parsed here with the Markdown and HTML extractors into the elements the renderers
//! understand: headings, list items, tables and paragraphs.

use std::sync::LazyLock;

use pulldown_cmark::Event;
use regex::Regex;

use crate::extractors::MarkdownExtractor;
use crate::types::internal::{ElementKind, InternalDocument, Relationship, RelationshipTarget};

/// `<table ...>` and `</table>` tags, to pair a table's own tags across nested tables.
static TABLE_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<(/?)table(?:[\s>/]|$)").expect("table tag regex is valid"));

/// Indentation, in bytes, from which a line is an indented code block and opens no fence or table.
const CODE_INDENT: usize = 4;

enum Segment<'a> {
    Markdown(&'a str),
    HtmlTable(&'a str),
}

/// Parse the Markdown a VLM returned into an OCR page document.
///
/// Returns `None` when the text carries no block structure beyond plain paragraphs, so a plain
/// prose page keeps the unstructured route it always took, and when anything fails to parse.
pub(super) fn markdown_ocr_document(text: &str) -> Option<InternalDocument> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut document = InternalDocument::new("markdown");
    for segment in split_html_tables(&text) {
        let part = match segment {
            Segment::Markdown(markdown) => parse_markdown(markdown),
            Segment::HtmlTable(html) => parse_html_table(html).or_else(|| parse_markdown(html)),
        }?;
        append(&mut document, part);
    }
    let structured = document
        .elements
        .iter()
        .any(|element| !matches!(element.kind, ElementKind::Paragraph));
    structured.then(|| {
        document.metadata.additional.insert(
            crate::extraction::derive::ADOPTED_OCR_DOCUMENT_KEY.into(),
            serde_json::Value::Bool(true),
        );
        document
    })
}

fn parse_markdown(markdown: &str) -> Option<InternalDocument> {
    if markdown.trim().is_empty() {
        return Some(InternalDocument::new("markdown"));
    }
    let events: Vec<Event> = crate::extractors::markdown_utils::markdown_events(markdown);
    let document = MarkdownExtractor::build_internal_document(&events, &None);
    document.images.is_empty().then_some(document)
}

#[cfg(feature = "html")]
fn parse_html_table(html: &str) -> Option<InternalDocument> {
    use crate::extractors::{HtmlExtractor, SyncExtractor};

    let document = HtmlExtractor::new()
        .extract_sync(
            html.as_bytes(),
            "text/html",
            &crate::core::config::ExtractionConfig::default(),
        )
        .ok()?;
    let has_table = document
        .elements
        .iter()
        .any(|element| matches!(element.kind, ElementKind::Table { .. }));
    (has_table && document.images.is_empty()).then_some(document)
}

#[cfg(not(feature = "html"))]
fn parse_html_table(_html: &str) -> Option<InternalDocument> {
    None
}

/// Split the page into Markdown and whole HTML `<table>` blocks. A table is found only at the
/// start of a line outside a fenced code block, and runs to its own closing tag.
fn split_html_tables(text: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut markdown_start = 0;
    let mut offset = 0;
    let mut fence: Option<(char, usize)> = None;
    while offset < text.len() {
        let line_end = text[offset..].find('\n').map_or(text.len(), |at| offset + at + 1);
        let line = &text[offset..line_end];
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        if indent < CODE_INDENT
            && let Some((marker, length)) = fence_marker(trimmed)
        {
            fence = match fence {
                Some((open, open_length))
                    if open == marker && length >= open_length && trimmed[length..].trim().is_empty() =>
                {
                    None
                }
                Some(open) => Some(open),
                None => Some((marker, length)),
            };
        }
        if fence.is_none()
            && let Some(start) = table_start(line).map(|at| offset + at)
            && let Some(end) = table_end(text, start)
        {
            if markdown_start < start {
                segments.push(Segment::Markdown(&text[markdown_start..start]));
            }
            segments.push(Segment::HtmlTable(&text[start..end]));
            markdown_start = end;
            offset = end;
            continue;
        }
        offset = line_end;
    }
    if markdown_start < text.len() {
        segments.push(Segment::Markdown(&text[markdown_start..]));
    }
    segments
}

/// The marker and run length of a code fence line (three or more backticks or tildes).
fn fence_marker(trimmed: &str) -> Option<(char, usize)> {
    let marker = trimmed.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let length = trimmed.chars().take_while(|c| *c == marker).count();
    (length >= 3).then_some((marker, length))
}

fn table_start(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    if indent >= CODE_INDENT {
        return None;
    }
    let opens = TABLE_TAG.captures(&line[indent..]).filter(|caps| {
        caps.get(0).is_some_and(|tag| tag.start() == 0) && caps.get(1).is_some_and(|slash| slash.as_str().is_empty())
    });
    opens.map(|_| indent)
}

/// The end of the table that opens at `start`: just past its matching `</table>`.
fn table_end(text: &str, start: usize) -> Option<usize> {
    let mut depth = 0usize;
    for tag in TABLE_TAG.captures_iter(&text[start..]) {
        let whole = tag.get(0)?;
        if tag.get(1)?.as_str().is_empty() {
            depth += 1;
        } else {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                let close = start + whole.end();
                if whole.as_str().ends_with('>') {
                    return Some(close);
                }
                return Some(text[close..].find('>').map_or(text.len(), |at| close + at + 1));
            }
        }
    }
    None
}

/// Append `part` to `document`, moving its tables and relationships to their new indices.
fn append(document: &mut InternalDocument, part: InternalDocument) {
    let element_base = document.elements.len() as u32;
    let table_base = document.tables.len() as u32;
    document.tables.extend(part.tables);
    for uri in part.uris {
        document.push_uri(uri);
    }
    document.uris_dropped += part.uris_dropped;
    for mut element in part.elements {
        if let ElementKind::Table { table_index } = &mut element.kind {
            *table_index += table_base;
        }
        let index = document.elements.len() as u32;
        document.push_element(element.with_index(index));
    }
    for relationship in part.relationships {
        document.push_relationship(Relationship {
            source: relationship.source + element_base,
            target: match relationship.target {
                RelationshipTarget::Index(target) => RelationshipTarget::Index(target + element_base),
                key @ RelationshipTarget::Key(_) => key,
            },
            kind: relationship.kind,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_prose_has_no_structure() {
        assert!(markdown_ocr_document("One line.\n\nAnother paragraph, 5 EUR.").is_none());
    }

    #[test]
    fn headings_lists_and_tables_make_a_structured_document() {
        let document = markdown_ocr_document("# Title\n\n- a\n- b\n\n| H |\n|---|\n| v |").expect("structure");
        let kinds: Vec<_> = document.elements.iter().map(|element| element.kind).collect();
        assert!(matches!(kinds[0], ElementKind::Heading { level: 1 }));
        assert!(kinds.iter().any(|kind| matches!(kind, ElementKind::ListItem { .. })));
        assert_eq!(document.tables.len(), 1);
    }

    #[test]
    fn a_table_tag_inside_a_code_fence_stays_code() {
        let text = "```html\n<table><tr><td>x</td></tr></table>\n```";
        let segments = split_html_tables(text);
        assert!(matches!(segments.as_slice(), [Segment::Markdown(whole)] if *whole == text));
    }

    #[test]
    fn a_shorter_fence_inside_a_longer_one_does_not_end_it() {
        let text = "````\n```\n<table><tr><td>x</td></tr></table>\n````";
        assert!(matches!(split_html_tables(text).as_slice(), [Segment::Markdown(whole)] if *whole == text));
    }

    #[test]
    fn links_in_a_parsed_page_keep_their_uris() {
        let document = markdown_ocr_document("# T\n\nSee [site](https://example.com/a).").expect("structure");
        assert!(
            document.uris.iter().any(|uri| uri.url == "https://example.com/a"),
            "{:?}",
            document.uris
        );
    }

    #[test]
    fn an_indented_marker_does_not_close_a_fence() {
        let text = "```\n    ```\n<table><tr><td>x</td></tr></table>\n```";
        assert!(matches!(split_html_tables(text).as_slice(), [Segment::Markdown(whole)] if *whole == text));
    }

    #[test]
    fn nested_tables_are_one_segment_to_the_outer_closing_tag() {
        let text = "<table><tr><td><table><tr><td>x</td></tr></table></td></tr></table>\nafter";
        let segments = split_html_tables(text);
        assert!(matches!(
            segments.as_slice(),
            [Segment::HtmlTable(table), Segment::Markdown("\nafter")] if table.ends_with("</table></td></tr></table>")
        ));
    }

    #[test]
    fn an_unclosed_table_stays_markdown() {
        let segments = split_html_tables("<table><tr><td>x");
        assert!(matches!(segments.as_slice(), [Segment::Markdown(_)]));
    }
}
