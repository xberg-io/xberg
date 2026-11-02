//! Preserve source decimal runs without renumbering incompatible list items.

use super::{AstNode, ContainerEntry, NodeList, NodeValue, mk};
use comrak::nodes::ListType;

const MAX_COMMONMARK_ORDINAL_DIGITS: usize = 9;

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
    existing.list_type == requested.list_type
        && (requested.list_type == ListType::Bullet
            || ordinal.is_none()
            || previous.and_then(|value| value.checked_add(1)) == ordinal)
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
