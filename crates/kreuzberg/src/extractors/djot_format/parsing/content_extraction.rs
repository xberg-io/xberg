//! Complete Djot content extraction.
//!
//! Handles extraction of rich DjotContent structures from Djot events.

use super::block_handlers::{handle_block_end, handle_block_start};
use super::event_handlers::{
    finalize_block_element, handle_footnote_end, handle_footnote_reference, handle_symbol, handle_thematic_break,
};
use super::inline_handlers::{
    finalize_inline_element, handle_image_end, handle_inline_end, handle_inline_start, handle_link_end, handle_math_end,
};
use super::state::{pop_block, ExtractionState};
use super::text_extraction::extract_text_from_events;
use crate::extractors::djot_format::attributes::parse_jotdown_attributes;
use crate::types::{Attributes, DjotContent, DjotImage, DjotLink, FormattedBlock};
use jotdown::{Container, Event};
use std::collections::HashMap;

/// Extract complete djot content with 100% feature extraction.
///
/// Processes ALL djot events to build a rich DjotContent structure including:
/// - Block structure (headings, lists, blockquotes, divs, sections, code blocks)
/// - Inline formatting (strong, emphasis, highlight, subscript, superscript, insert, delete)
/// - Attributes (classes, IDs, key-value pairs)
/// - Links and images with full metadata (href, src, alt, title)
/// - Math blocks (inline & display)
/// - Definition lists (term/description pairs)
/// - Task lists with checked state
/// - Raw blocks (HTML/LaTeX)
/// - Footnotes (references and definitions)
/// - Captions
/// - Smart punctuation
/// - All other djot features
pub fn extract_complete_djot_content(
    events: &[Event],
    metadata: crate::types::Metadata,
    tables: Vec<crate::types::Table>,
) -> DjotContent {
    let plain_text = extract_text_from_events(events);

    let mut blocks = Vec::new();
    let mut images = Vec::new();
    let mut links = Vec::new();
    let mut footnotes = Vec::new();
    let attributes_map: HashMap<String, Attributes> = HashMap::new();

    let mut state = ExtractionState::new();

    for event in events {
        match event {
            Event::Start(container, attrs) => {
                handle_start_event(&mut state, container, attrs, &mut blocks, &mut images, &mut links, &mut footnotes);
            }
            Event::End(container) => {
                handle_end_event(&mut state, container, &mut blocks, &mut images, &mut links, &mut footnotes);
            }
            Event::Str(s) => {
                if state.in_code_block || state.in_raw_block {
                    state.code_content.push_str(s);
                } else if state.in_math {
                    state.math_content.push_str(s);
                } else {
                    state.current_text.push_str(s);
                }
            }
            Event::FootnoteReference(label) => {
                handle_footnote_reference(&mut state, label);
            }
            Event::Symbol(sym) => {
                handle_symbol(&mut state, sym);
            }
            Event::Attributes(attrs) => {
                // Store attributes to be applied to the next element
                state.pending_attributes = Some(parse_jotdown_attributes(attrs));
            }
            Event::Softbreak => {
                if state.in_math {
                    state.math_content.push(' ');
                } else if !state.inline_type_stack.is_empty() {
                    state.current_text.push(' ');
                } else {
                    state.current_text.push('\n');
                }
            }
            Event::Hardbreak => {
                if state.in_math {
                    state.math_content.push('\n');
                } else {
                    state.current_text.push('\n');
                }
            }
            Event::NonBreakingSpace => {
                state.current_text.push(' ');
            }
            Event::Blankline => {
                // Blank lines are typically ignored in block processing
            }
            Event::ThematicBreak(attrs) => {
                handle_thematic_break(&mut state, attrs, &mut blocks);
            }
            // Smart punctuation events
            Event::LeftSingleQuote => {
                state.current_text.push('\'');
            }
            Event::RightSingleQuote => {
                state.current_text.push('\'');
            }
            Event::LeftDoubleQuote => {
                state.current_text.push('"');
            }
            Event::RightDoubleQuote => {
                state.current_text.push('"');
            }
            Event::Ellipsis => {
                state.current_text.push_str("...");
            }
            Event::EnDash => {
                state.current_text.push_str("--");
            }
            Event::EmDash => {
                state.current_text.push_str("---");
            }
            Event::Escape => {
                // Escape is a marker, doesn't produce output
            }
        }
    }

    // Finalize any remaining content
    state.flush_text();

    // Pop any remaining blocks
    while !state.block_stack.is_empty() {
        pop_block(&mut state, &mut blocks);
    }

    // Add any remaining inline elements to the last block if exists
    if !state.current_inline_elements.is_empty()
        && let Some(last_block) = blocks.last_mut()
    {
        last_block.inline_content.append(&mut state.current_inline_elements);
    }

    DjotContent {
        plain_text,
        blocks,
        metadata,
        tables,
        images,
        links,
        footnotes,
        attributes: attributes_map,
    }
}

/// Handle start of a container event.
fn handle_start_event(
    state: &mut ExtractionState,
    container: &Container,
    attrs: &jotdown::Attributes,
    _blocks: &mut Vec<FormattedBlock>,
    images: &mut Vec<DjotImage>,
    links: &mut Vec<DjotLink>,
    footnotes: &mut Vec<crate::types::Footnote>,
) {
    // Parse attributes from jotdown's Attributes type
    let parsed_attrs = if attrs.is_empty() {
        state.pending_attributes.take()
    } else {
        Some(parse_jotdown_attributes(attrs))
    };

    // Try block handlers first
    if handle_block_start(state, container, attrs, parsed_attrs.clone(), footnotes) {
        return;
    }

    // Try inline handlers
    if handle_inline_start(state, container, parsed_attrs, images, links) {
        return;
    }

    // Handle remaining containers (tables, link definitions, etc.)
    match container {
        Container::Table | Container::TableRow { .. } | Container::TableCell { .. } | Container::Caption => {
            // Tables are extracted separately
        }
        Container::LinkDefinition { .. } => {
            // Link definitions are resolved by jotdown, not needed in output
        }
        _ => {}
    }
}

/// Handle end of a container event.
fn handle_end_event(
    state: &mut ExtractionState,
    container: &Container,
    blocks: &mut Vec<FormattedBlock>,
    images: &mut Vec<DjotImage>,
    links: &mut Vec<DjotLink>,
    footnotes: &mut Vec<crate::types::Footnote>,
) {
    // Check if it's a block container
    if handle_block_end(state, container) {
        finalize_block_element(state, blocks);
        return;
    }

    // Handle special cases
    match container {
        Container::Footnote { .. } => {
            handle_footnote_end(state, footnotes);
        }
        Container::Math { display } => {
            handle_math_end(state, *display);
        }
        Container::Link(url, _) => {
            handle_link_end(state, url, links);
        }
        Container::Image(src, _) => {
            handle_image_end(state, src, images);
        }
        _ => {
            // Check if it's an inline element
            if handle_inline_end(state, container) {
                finalize_inline_element(state, container);
            }
        }
    }

    // Handle remaining containers (tables, link definitions, etc.)
    match container {
        Container::Table | Container::TableRow { .. } | Container::TableCell { .. } | Container::Caption => {
            // Tables are handled separately
        }
        Container::LinkDefinition { .. } => {
            // Link definitions don't produce output
        }
        _ => {}
    }
}
