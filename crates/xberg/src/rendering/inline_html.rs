use comrak::Arena;
use comrak::nodes::{AstNode, NodeValue};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Superscript,
    Underline,
}

fn tag(node: &AstNode<'_>) -> Option<(Kind, bool)> {
    let data = node.data.borrow();
    let NodeValue::HtmlInline(html) = &data.value else {
        return None;
    };
    match html.as_str() {
        "<sup>" => Some((Kind::Superscript, true)),
        "</sup>" => Some((Kind::Superscript, false)),
        "<u>" => Some((Kind::Underline, true)),
        "</u>" => Some((Kind::Underline, false)),
        _ => None,
    }
}

type Pair<'a> = (&'a AstNode<'a>, &'a AstNode<'a>, Kind);

fn pairs<'a>(parent: &'a AstNode<'a>) -> Option<Vec<Pair<'a>>> {
    let mut stack = Vec::new();
    let mut pairs = Vec::new();
    for child in parent.children() {
        match tag(child) {
            Some((kind, true)) => stack.push((child, kind)),
            Some((kind, false)) => {
                let (opening, expected) = stack.pop()?;
                if kind != expected {
                    return None;
                }
                pairs.push((opening, child, kind));
            }
            None => {}
        }
    }
    stack.is_empty().then_some(pairs)
}

// Binary writers parse the postprocessed Markdown, so only balanced, bare formatting tags
// become semantic nodes. Text escapes and code literals never enter this HtmlInline path. ~keep
pub(super) fn decode_standard_inline_html<'a>(root: &'a AstNode<'a>, arena: &'a Arena<'a>) {
    for parent in root.descendants().collect::<Vec<_>>() {
        let Some(pairs) = pairs(parent) else {
            continue;
        };
        for (opening, closing, kind) in pairs {
            let value = match kind {
                Kind::Superscript => NodeValue::Superscript,
                Kind::Underline => NodeValue::Underline,
            };
            let wrapper = arena.alloc(AstNode::from(value));
            opening.insert_before(wrapper);
            while let Some(child) = opening.next_sibling() {
                if std::ptr::eq(child, closing) {
                    break;
                }
                wrapper.append(child);
            }
            opening.detach();
            closing.detach();
        }
    }
}
