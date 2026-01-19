//! Text extraction from Djot events.
//!
//! Handles parsing of jotdown events into plain text.

use jotdown::Event;

/// Extract plain text from Djot events.
///
/// Processes djot events and extracts plain text content, handling:
/// - Text content
/// - Line breaks (soft, hard, blank)
/// - Smart punctuation (quotes, dashes, ellipsis)
/// - Special symbols and footnote references
pub fn extract_text_from_events(events: &[Event]) -> String {
    let mut text = String::new();

    for event in events {
        match event {
            Event::Str(s) => {
                text.push_str(s.as_ref());
            }
            Event::Softbreak | Event::Hardbreak | Event::Blankline => {
                text.push('\n');
            }
            Event::NonBreakingSpace => {
                text.push(' ');
            }
            Event::LeftSingleQuote | Event::RightSingleQuote => {
                text.push('\'');
            }
            Event::LeftDoubleQuote | Event::RightDoubleQuote => {
                text.push('"');
            }
            Event::Ellipsis => {
                text.push_str("...");
            }
            Event::EnDash => {
                text.push_str("--");
            }
            Event::EmDash => {
                text.push_str("---");
            }
            Event::FootnoteReference(s) => {
                text.push('[');
                text.push_str(s.as_ref());
                text.push(']');
            }
            Event::Symbol(s) => {
                text.push(':');
                text.push_str(s.as_ref());
                text.push(':');
            }
            Event::ThematicBreak(_) => {
                text.push_str("\n---\n");
            }
            Event::Start(_, _) | Event::End(_) | Event::Escape | Event::Attributes(_) => {}
        }
    }

    text
}
