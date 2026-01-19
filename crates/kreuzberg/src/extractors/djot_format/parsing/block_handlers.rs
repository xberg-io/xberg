//! Block-level container handlers for Djot parsing.

use super::state::{push_block, ExtractionState};
use crate::types::{Attributes, BlockType, FormattedBlock};
use jotdown::Container;

/// Handle start of block containers.
pub(super) fn handle_block_start(
    state: &mut ExtractionState,
    container: &Container,
    _attrs: &jotdown::Attributes,
    parsed_attrs: Option<Attributes>,
    footnotes: &mut Vec<crate::types::Footnote>,
) -> bool {
    match container {
        Container::Heading { level, .. } => {
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::Heading,
                    level: Some(*level as usize),
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::Paragraph => {
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::Paragraph,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::Blockquote => {
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::Blockquote,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::CodeBlock { language } => {
            let lang_str = if language.is_empty() {
                None
            } else {
                Some(language.to_string())
            };
            state.in_code_block = true;
            state.code_content.clear();
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::CodeBlock,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: lang_str,
                    code: Some(String::new()),
                    children: Vec::new(),
                },
            );
            true
        }
        Container::RawBlock { format } => {
            state.in_raw_block = true;
            state.raw_format = Some(format.to_string());
            state.code_content.clear();
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::RawBlock,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: Some(format.to_string()),
                    code: Some(String::new()),
                    children: Vec::new(),
                },
            );
            true
        }
        Container::List { kind, .. } => {
            let block_type = match kind {
                jotdown::ListKind::Ordered { .. } => BlockType::OrderedList,
                jotdown::ListKind::Unordered(_) => BlockType::BulletList,
                jotdown::ListKind::Task(_) => BlockType::TaskList,
            };
            push_block(
                state,
                FormattedBlock {
                    block_type,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::ListItem => {
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::ListItem,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::TaskListItem { checked } => {
            let mut attrs = parsed_attrs.unwrap_or_default();
            attrs.key_values.insert("checked".to_string(), checked.to_string());
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::ListItem,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: Some(attrs),
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::DescriptionList => {
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::DefinitionList,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::DescriptionTerm => {
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::DefinitionTerm,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::DescriptionDetails => {
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::DefinitionDescription,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::Div { .. } => {
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::Div,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::Section { .. } => {
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::Section,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        Container::Footnote { label } => {
            // Start tracking a footnote definition
            footnotes.push(crate::types::Footnote {
                label: label.to_string(),
                content: Vec::new(),
            });
            // We'll collect the content as blocks
            push_block(
                state,
                FormattedBlock {
                    block_type: BlockType::Paragraph,
                    level: None,
                    inline_content: Vec::new(),
                    attributes: parsed_attrs,
                    language: None,
                    code: None,
                    children: Vec::new(),
                },
            );
            true
        }
        _ => false,
    }
}

/// Handle end of block containers.
pub(super) fn handle_block_end(_state: &mut ExtractionState, container: &Container) -> bool {
    matches!(
        container,
        Container::Heading { .. }
            | Container::Paragraph
            | Container::Blockquote
            | Container::CodeBlock { .. }
            | Container::RawBlock { .. }
            | Container::Div { .. }
            | Container::Section { .. }
            | Container::List { .. }
            | Container::ListItem
            | Container::TaskListItem { .. }
            | Container::DescriptionList
            | Container::DescriptionTerm
            | Container::DescriptionDetails
    )
}
