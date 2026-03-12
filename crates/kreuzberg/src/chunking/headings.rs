//! Heading extraction for Markdown chunk metadata.
//!
//! Parses markdown text to build a heading map, then resolves
//! which headings a chunk falls under based on its byte offset.

use crate::types::{HeadingContext, HeadingLevel};
use pulldown_cmark::{Event, Options, Parser, TagEnd};

/// An entry in the heading map: `(byte_offset, level, text)`.
type HeadingEntry = (usize, u8, String);

/// Build a heading map from markdown text.
///
/// Returns a sorted Vec of `(byte_offset, level, heading_text)` for each heading found.
pub fn build_heading_map(markdown: &str) -> Vec<HeadingEntry> {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut headings = Vec::new();
    let mut current_heading: Option<(usize, u8)> = None;
    let mut heading_text = String::new();

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(pulldown_cmark::Tag::Heading { level, .. }) => {
                current_heading = Some((range.start, heading_level_to_u8(level)));
                heading_text.clear();
            }
            Event::Text(text) if current_heading.is_some() => {
                heading_text.push_str(&text);
            }
            Event::Code(code) if current_heading.is_some() => {
                heading_text.push_str(&code);
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((offset, level)) = current_heading.take() {
                    headings.push((offset, level, heading_text.clone()));
                }
            }
            _ => {}
        }
    }
    headings
}

/// Resolve the heading context for a chunk at the given byte offset.
///
/// Walks the heading map to find all headings that precede `byte_start`,
/// building a proper hierarchy stack (h1 > h2 > h3, etc.).
pub fn resolve_heading_context(byte_start: usize, heading_map: &[HeadingEntry]) -> Option<HeadingContext> {
    let mut stack: Vec<HeadingLevel> = Vec::new();

    for &(offset, level, ref text) in heading_map {
        if offset > byte_start {
            break;
        }
        // Pop headings at same or deeper level (they've been superseded).
        while stack.last().is_some_and(|h| h.level >= level) {
            stack.pop();
        }
        stack.push(HeadingLevel {
            level,
            text: text.clone(),
        });
    }

    if stack.is_empty() {
        None
    } else {
        Some(HeadingContext { headings: stack })
    }
}

fn heading_level_to_u8(level: pulldown_cmark::HeadingLevel) -> u8 {
    match level {
        pulldown_cmark::HeadingLevel::H1 => 1,
        pulldown_cmark::HeadingLevel::H2 => 2,
        pulldown_cmark::HeadingLevel::H3 => 3,
        pulldown_cmark::HeadingLevel::H4 => 4,
        pulldown_cmark::HeadingLevel::H5 => 5,
        pulldown_cmark::HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_heading_map_basic() {
        let md = "# Title\n\nSome text.\n\n## Section 1\n\nContent.\n\n## Section 2\n\nMore content.";
        let map = build_heading_map(md);
        assert_eq!(map.len(), 3);
        assert_eq!(map[0], (0, 1, "Title".to_string()));
        assert_eq!(map[1].1, 2);
        assert_eq!(map[1].2, "Section 1");
        assert_eq!(map[2].1, 2);
        assert_eq!(map[2].2, "Section 2");
    }

    #[test]
    fn test_build_heading_map_nested() {
        let md = "# H1\n\n## H2\n\n### H3\n\nText.";
        let map = build_heading_map(md);
        assert_eq!(map.len(), 3);
        assert_eq!(map[0].1, 1);
        assert_eq!(map[1].1, 2);
        assert_eq!(map[2].1, 3);
    }

    #[test]
    fn test_build_heading_map_no_headings() {
        let md = "Just plain text without any headings.";
        let map = build_heading_map(md);
        assert!(map.is_empty());
    }

    #[test]
    fn test_build_heading_map_with_code_in_heading() {
        let md = "# Title with `code`\n\nText.";
        let map = build_heading_map(md);
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].2, "Title with code");
    }

    #[test]
    fn test_resolve_heading_context_under_h2() {
        let map = vec![
            (0, 1, "Title".to_string()),
            (10, 2, "Section A".to_string()),
            (30, 2, "Section B".to_string()),
        ];
        // Chunk at byte 15 is under h1 > h2("Section A")
        let ctx = resolve_heading_context(15, &map).unwrap();
        assert_eq!(ctx.headings.len(), 2);
        assert_eq!(ctx.headings[0].level, 1);
        assert_eq!(ctx.headings[0].text, "Title");
        assert_eq!(ctx.headings[1].level, 2);
        assert_eq!(ctx.headings[1].text, "Section A");
    }

    #[test]
    fn test_resolve_heading_context_root() {
        let map = vec![(10, 1, "Title".to_string())];
        // Chunk at byte 0 is before any heading
        let ctx = resolve_heading_context(0, &map);
        assert!(ctx.is_none());
    }

    #[test]
    fn test_resolve_heading_context_superseded() {
        let map = vec![
            (0, 1, "Title".to_string()),
            (10, 2, "Section A".to_string()),
            (20, 3, "Subsection".to_string()),
            (30, 2, "Section B".to_string()), // This supersedes h3 "Subsection"
        ];
        // Chunk at byte 35 should be h1 > h2("Section B"), no h3
        let ctx = resolve_heading_context(35, &map).unwrap();
        assert_eq!(ctx.headings.len(), 2);
        assert_eq!(ctx.headings[1].text, "Section B");
    }

    #[test]
    fn test_resolve_heading_context_deep_nesting() {
        let map = vec![
            (0, 1, "H1".to_string()),
            (5, 2, "H2".to_string()),
            (10, 3, "H3".to_string()),
            (15, 4, "H4".to_string()),
        ];
        let ctx = resolve_heading_context(20, &map).unwrap();
        assert_eq!(ctx.headings.len(), 4);
        assert_eq!(ctx.headings[3].level, 4);
    }
}
