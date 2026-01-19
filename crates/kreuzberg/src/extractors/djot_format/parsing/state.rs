//! State management for Djot content extraction.
//!
//! Provides extraction state and helper functions for parsing Djot events.

use crate::types::{Attributes, FormattedBlock, InlineElement, InlineType};

/// Enhanced state tracking using a block stack for proper nesting.
pub(super) struct ExtractionState {
    pub block_stack: Vec<FormattedBlock>,   // Stack for nested blocks
    pub inline_type_stack: Vec<InlineType>, // Stack for nested inline element types
    pub current_text: String,               // Text accumulator
    pub pending_attributes: Option<Attributes>,
    pub code_content: String, // Accumulator for code blocks
    pub in_code_block: bool,
    pub in_math: bool,
    pub math_display: bool,
    pub math_content: String,
    pub current_link_index: Option<usize>,
    pub current_image_index: Option<usize>,
    pub in_raw_block: bool,
    pub raw_format: Option<String>,
    pub current_inline_elements: Vec<InlineElement>,
}

impl ExtractionState {
    /// Create a new extraction state.
    pub fn new() -> Self {
        Self {
            block_stack: Vec::new(),
            inline_type_stack: Vec::new(),
            current_text: String::new(),
            pending_attributes: None,
            code_content: String::new(),
            in_code_block: false,
            in_math: false,
            math_display: false,
            math_content: String::new(),
            current_link_index: None,
            current_image_index: None,
            in_raw_block: false,
            raw_format: None,
            current_inline_elements: Vec::new(),
        }
    }

    /// Flush current text to inline elements if any text is pending.
    pub fn flush_text(&mut self) {
        if !self.current_text.is_empty() {
            self.current_inline_elements.push(InlineElement {
                element_type: InlineType::Text,
                content: std::mem::take(&mut self.current_text),
                attributes: None,
                metadata: None,
            });
        }
    }
}

/// Helper to create a new block and push to stack.
pub(super) fn push_block(state: &mut ExtractionState, block: FormattedBlock) {
    state.block_stack.push(block);
}

/// Helper to pop a block from the stack and add to parent or blocks list.
pub(super) fn pop_block(state: &mut ExtractionState, blocks: &mut Vec<FormattedBlock>) {
    if let Some(mut block) = state.block_stack.pop() {
        // Add any pending inline elements to the block
        if !state.current_inline_elements.is_empty() {
            block.inline_content.append(&mut state.current_inline_elements);
        }
        // If there's a parent block, add as child; otherwise add to top-level blocks
        if let Some(parent) = state.block_stack.last_mut() {
            parent.children.push(block);
        } else {
            blocks.push(block);
        }
    }
}
