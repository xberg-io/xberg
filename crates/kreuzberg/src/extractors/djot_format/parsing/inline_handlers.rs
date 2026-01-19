//! Inline-level element handlers for Djot parsing.

use super::state::ExtractionState;
use crate::types::{DjotImage, DjotLink, InlineElement, InlineType};
use jotdown::Container;
use std::collections::HashMap;

/// Handle start of inline elements.
pub(super) fn handle_inline_start(
    state: &mut ExtractionState,
    container: &Container,
    parsed_attrs: Option<crate::types::Attributes>,
    images: &mut Vec<DjotImage>,
    links: &mut Vec<DjotLink>,
) -> bool {
    match container {
        Container::Math { display } => {
            state.in_math = true;
            state.math_display = *display;
            state.math_content.clear();
            state.inline_type_stack.push(InlineType::Math);
            true
        }
        Container::Strong => {
            state.inline_type_stack.push(InlineType::Strong);
            state.flush_text();
            true
        }
        Container::Emphasis => {
            state.inline_type_stack.push(InlineType::Emphasis);
            state.flush_text();
            true
        }
        Container::Mark => {
            state.inline_type_stack.push(InlineType::Highlight);
            state.flush_text();
            true
        }
        Container::Subscript => {
            state.inline_type_stack.push(InlineType::Subscript);
            state.flush_text();
            true
        }
        Container::Superscript => {
            state.inline_type_stack.push(InlineType::Superscript);
            state.flush_text();
            true
        }
        Container::Insert => {
            state.inline_type_stack.push(InlineType::Insert);
            state.flush_text();
            true
        }
        Container::Delete => {
            state.inline_type_stack.push(InlineType::Delete);
            state.flush_text();
            true
        }
        Container::Verbatim => {
            state.inline_type_stack.push(InlineType::Code);
            state.flush_text();
            true
        }
        Container::Span => {
            state.inline_type_stack.push(InlineType::Span);
            state.flush_text();
            true
        }
        Container::RawInline { format } => {
            state.inline_type_stack.push(InlineType::RawInline);
            state.raw_format = Some(format.to_string());
            state.flush_text();
            true
        }
        Container::Link(url, _link_type) => {
            state.inline_type_stack.push(InlineType::Link);
            links.push(DjotLink {
                url: url.to_string(),
                text: String::new(),
                title: None,
                attributes: parsed_attrs,
            });
            state.current_link_index = Some(links.len() - 1);
            state.flush_text();
            true
        }
        Container::Image(src, _link_type) => {
            state.inline_type_stack.push(InlineType::Image);
            images.push(DjotImage {
                src: src.to_string(),
                alt: String::new(),
                title: None,
                attributes: parsed_attrs,
            });
            state.current_image_index = Some(images.len() - 1);
            state.flush_text();
            true
        }
        _ => false,
    }
}

/// Handle end of inline elements.
pub(super) fn handle_inline_end(_state: &mut ExtractionState, container: &Container) -> bool {
    matches!(
        container,
        Container::Strong
            | Container::Emphasis
            | Container::Mark
            | Container::Subscript
            | Container::Superscript
            | Container::Insert
            | Container::Delete
            | Container::Verbatim
            | Container::Span
            | Container::RawInline { .. }
    )
}

/// Handle end of math element.
pub(super) fn handle_math_end(state: &mut ExtractionState, display: bool) {
    state.in_math = false;
    let math_text = std::mem::take(&mut state.math_content);
    state.inline_type_stack.pop();

    let mut meta = HashMap::new();
    meta.insert("display".to_string(), display.to_string());

    state.current_inline_elements.push(InlineElement {
        element_type: InlineType::Math,
        content: math_text,
        attributes: state.pending_attributes.take(),
        metadata: Some(meta),
    });
}

/// Finalize inline element content.
pub(super) fn finalize_inline_element(state: &mut ExtractionState, container: &Container) {
    if let Some(inline_type) = state.inline_type_stack.pop() {
        let content = std::mem::take(&mut state.current_text);
        let mut meta = None;

        // For raw inline, include the format
        if matches!(container, Container::RawInline { .. })
            && let Some(fmt) = state.raw_format.take()
        {
            let mut m = HashMap::new();
            m.insert("format".to_string(), fmt);
            meta = Some(m);
        }

        state.current_inline_elements.push(InlineElement {
            element_type: inline_type,
            content,
            attributes: state.pending_attributes.take(),
            metadata: meta,
        });
    }
}

/// Handle end of link element.
pub(super) fn handle_link_end(state: &mut ExtractionState, url: &str, links: &mut Vec<DjotLink>) {
    if let Some(idx) = state.current_link_index.take() {
        let text = std::mem::take(&mut state.current_text);
        if let Some(link) = links.get_mut(idx) {
            link.text = text.clone();
        }
        state.inline_type_stack.pop();

        let mut meta = HashMap::new();
        meta.insert("href".to_string(), url.to_string());

        state.current_inline_elements.push(InlineElement {
            element_type: InlineType::Link,
            content: text,
            attributes: state.pending_attributes.take(),
            metadata: Some(meta),
        });
    }
}

/// Handle end of image element.
pub(super) fn handle_image_end(state: &mut ExtractionState, src: &str, images: &mut Vec<DjotImage>) {
    if let Some(idx) = state.current_image_index.take() {
        let alt = std::mem::take(&mut state.current_text);
        if let Some(image) = images.get_mut(idx) {
            image.alt = alt.clone();
        }
        state.inline_type_stack.pop();

        let mut meta = HashMap::new();
        meta.insert("src".to_string(), src.to_string());

        state.current_inline_elements.push(InlineElement {
            element_type: InlineType::Image,
            content: alt,
            attributes: state.pending_attributes.take(),
            metadata: Some(meta),
        });
    }
}
