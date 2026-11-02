use comrak::Arena;
use comrak::nodes::{AstNode, NodeValue};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Superscript,
    Underline,
}

pub(crate) fn tag(node: &AstNode<'_>) -> Option<(Kind, bool)> {
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
#[cfg(any(feature = "office", feature = "pdf"))]
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

#[cfg(feature = "redaction")]
fn original_tag_range(node: &AstNode<'_>, text: &str, lines: &[usize]) -> Option<std::ops::Range<usize>> {
    let data = node.data.borrow();
    let NodeValue::HtmlInline(tag) = &data.value else {
        return None;
    };
    let position = data.sourcepos;
    if position.start.line != position.end.line {
        return None;
    }
    let start = lines
        .get(position.start.line.checked_sub(1)?)?
        .checked_add(position.start.column.checked_sub(1)?)?;
    let end = start.checked_add(tag.len())?;
    (text.get(start..end) == Some(tag.as_str())).then_some(start..end)
}

#[cfg(feature = "redaction")]
pub(crate) fn standard_inline_html_ranges(text: &str) -> Option<Vec<std::ops::Range<usize>>> {
    if !text.contains("<sup>") && !text.contains("<u>") {
        return Some(Vec::new());
    }
    let arena = Arena::new();
    let options = super::markdown::comrak_options();
    let root = comrak::parse_document(&arena, text, &options);
    let mut lines = vec![0];
    lines.extend(text.match_indices('\n').map(|(index, _)| index + 1));
    let mut ranges = Vec::new();
    for parent in root.descendants() {
        let Some(pairs) = pairs(parent) else {
            continue;
        };
        for (opening, closing, _) in pairs {
            ranges.push(original_tag_range(opening, text, &lines)?);
            ranges.push(original_tag_range(closing, text, &lines)?);
        }
    }
    ranges.sort_by_key(|range| range.start);
    Some(ranges)
}

#[cfg(all(test, feature = "redaction"))]
mod redaction_tests;
