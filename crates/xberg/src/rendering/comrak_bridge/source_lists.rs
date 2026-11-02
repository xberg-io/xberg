//! Preserve source decimal runs without renumbering incompatible list items.

use super::{AstNode, ContainerEntry, ElementKind, InternalElement, NodeList, NodeValue, is_body_element, mk};
use comrak::nodes::ListType;
use std::collections::HashSet;

const MAX_COMMONMARK_ORDINAL_DIGITS: usize = 9;
const MAX_COMMONMARK_ORDINAL: usize = 999_999_999;

pub(super) fn decimal_ordinal(label: &str) -> Option<usize> {
    let digits = label.strip_suffix('.')?;
    if digits.is_empty()
        || digits.len() > MAX_COMMONMARK_ORDINAL_DIGITS
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || digits.len() > 1 && digits.starts_with('0')
    {
        return None;
    }
    digits.parse().ok()
}

fn previous_ordinal<'a>(parent: &'a AstNode<'a>) -> Option<usize> {
    let item = parent.last_child()?;
    match item.data.borrow().value {
        NodeValue::Item(list) => Some(list.start),
        _ => None,
    }
}

fn compatible(existing: NodeList, requested: NodeList, ordinal: Option<usize>, previous: Option<usize>) -> bool {
    let next = previous
        .and_then(|value| value.checked_add(1))
        .filter(|value| *value <= MAX_COMMONMARK_ORDINAL);
    existing.list_type == requested.list_type
        && (requested.list_type == ListType::Bullet
            || next.is_some_and(|next| ordinal.is_none_or(|ordinal| ordinal == next)))
}

pub(super) fn item_parent<'a>(
    arena: &'a comrak::Arena<'a>,
    parent: &'a AstNode<'a>,
    stack: &mut [ContainerEntry<'a>],
    mut requested: NodeList,
    ordinal: Option<usize>,
) -> (&'a AstNode<'a>, NodeList) {
    let existing = match parent.data.borrow().value {
        NodeValue::List(list) => Some(list),
        _ => None,
    };
    let Some(existing) = existing else {
        let list = mk(arena, NodeValue::List(requested));
        parent.append(list);
        return (list, requested);
    };
    if parent.first_child().is_none() {
        parent.data.borrow_mut().value = NodeValue::List(requested);
        return (parent, requested);
    }
    let previous = previous_ordinal(parent);
    if compatible(existing, requested, ordinal, previous) {
        // Item ordinals track only direct siblings; nested children never advance an outer run. ~keep
        if requested.list_type == ListType::Ordered {
            requested.start = previous
                .and_then(|value| value.checked_add(1))
                .unwrap_or(existing.start);
        }
        return (parent, requested);
    }
    let list = mk(arena, NodeValue::List(requested));
    parent.insert_after(list);
    if let Some(entry) = stack.last_mut().filter(|entry| entry.node.same_node(parent)) {
        entry.node = list;
    }
    (list, requested)
}

#[derive(Default)]
struct SourceList {
    alphabetic_items: Vec<usize>,
    decimal: bool,
    flat: bool,
}

fn alphabetic_label(label: &str) -> bool {
    matches!(label.as_bytes(), [letter, b'.'] if letter.is_ascii_alphabetic())
}

pub(super) fn paragraph_items(elements: &[InternalElement]) -> HashSet<usize> {
    let mut lists: Vec<SourceList> = Vec::new();
    let mut paragraphs = HashSet::new();
    for (index, element) in elements
        .iter()
        .enumerate()
        .filter(|(_, element)| is_body_element(element))
    {
        match element.kind {
            ElementKind::ListStart { ordered } => {
                if let Some(parent) = lists.last_mut() {
                    parent.flat = false;
                }
                lists.push(SourceList {
                    flat: ordered && lists.is_empty(),
                    ..Default::default()
                });
            }
            ElementKind::ListEnd => {
                if let Some(list) = lists.pop().filter(|list| list.flat && list.decimal) {
                    paragraphs.extend(list.alphabetic_items);
                }
            }
            ElementKind::ListItem { ordered: true } => {
                if let Some(list) = lists.last_mut()
                    && let Some(label) = element.list_item_source_label()
                {
                    list.decimal |= decimal_ordinal(label).is_some();
                    if alphabetic_label(label) {
                        list.alphabetic_items.push(index);
                    }
                }
            }
            ElementKind::ListItem { ordered: false } => {}
            _ => {
                if let Some(list) = lists.last_mut() {
                    list.flat = false;
                }
            }
        }
    }
    paragraphs
}

pub(super) fn paragraph<'a>(
    arena: &'a comrak::Arena<'a>,
    parent: &'a AstNode<'a>,
    stack: &mut [ContainerEntry<'a>],
) -> &'a AstNode<'a> {
    let cursor = if parent.first_child().is_none() {
        parent
    } else {
        let cursor = mk(arena, parent.data.borrow().value.clone());
        parent.insert_after(cursor);
        cursor
    };
    if let Some(entry) = stack.last_mut().filter(|entry| entry.node.same_node(parent)) {
        entry.node = cursor;
        entry.empty_list_cursor = true;
    }
    // The empty list anchors later items after the literal paragraphs, without inventing nesting. ~keep
    let paragraph = mk(arena, NodeValue::Paragraph);
    cursor.insert_before(paragraph);
    paragraph
}
