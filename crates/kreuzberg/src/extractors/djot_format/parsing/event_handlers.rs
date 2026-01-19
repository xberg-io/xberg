//! Event handlers for special Djot elements.

use super::state::{pop_block, ExtractionState};
use crate::extractors::djot_format::attributes::parse_jotdown_attributes;
use crate::types::{BlockType, FormattedBlock, InlineElement, InlineType};
use std::collections::HashMap;

/// Handle footnote reference event.
pub(super) fn handle_footnote_reference(state: &mut ExtractionState, label: &str) {
    state.flush_text();

    let mut meta = HashMap::new();
    meta.insert("label".to_string(), label.to_string());

    state.current_inline_elements.push(InlineElement {
        element_type: InlineType::FootnoteRef,
        content: label.to_string(),
        attributes: None,
        metadata: Some(meta),
    });
}

/// Handle symbol event.
pub(super) fn handle_symbol(state: &mut ExtractionState, sym: &str) {
    state.flush_text();

    state.current_inline_elements.push(InlineElement {
        element_type: InlineType::Symbol,
        content: sym.to_string(),
        attributes: None,
        metadata: None,
    });
}

/// Handle thematic break event.
pub(super) fn handle_thematic_break(state: &mut ExtractionState, attrs: &jotdown::Attributes, blocks: &mut Vec<FormattedBlock>) {
    state.flush_text();

    let parsed_attrs = if attrs.is_empty() {
        None
    } else {
        Some(parse_jotdown_attributes(attrs))
    };

    let hr_block = FormattedBlock {
        block_type: BlockType::ThematicBreak,
        level: None,
        inline_content: Vec::new(),
        attributes: parsed_attrs,
        language: None,
        code: None,
        children: Vec::new(),
    };

    if let Some(parent) = state.block_stack.last_mut() {
        parent.children.push(hr_block);
    } else {
        blocks.push(hr_block);
    }
}

/// Handle end of footnote definition.
pub(super) fn handle_footnote_end(state: &mut ExtractionState, footnotes: &mut Vec<crate::types::Footnote>) {
    state.flush_text();
    // Pop the footnote content block and add to the last footnote
    if let Some(mut block) = state.block_stack.pop() {
        block.inline_content.append(&mut state.current_inline_elements);
        if let Some(footnote) = footnotes.last_mut() {
            footnote.content.push(block);
        }
    }
}

/// Finalize block element content and pop from stack.
pub(super) fn finalize_block_element(state: &mut ExtractionState, blocks: &mut Vec<FormattedBlock>) {
    // Flush any remaining text
    state.flush_text();

    // For code blocks, set the accumulated code content
    if state.in_code_block {
        if let Some(block) = state.block_stack.last_mut() {
            block.code = Some(std::mem::take(&mut state.code_content));
        }
        state.in_code_block = false;
    }

    // For raw blocks
    if state.in_raw_block {
        if let Some(block) = state.block_stack.last_mut() {
            block.code = Some(std::mem::take(&mut state.code_content));
        }
        state.in_raw_block = false;
        state.raw_format = None;
    }

    pop_block(state, blocks);
}
