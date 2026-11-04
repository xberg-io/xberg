use super::{InternalDocument, finish_markdown};
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Options, format_commonmark};
use std::ops::Range;

const TAGS: [(&str, &str); 4] = [
    ("<sup>", "<SUP>"),
    ("</sup>", "</SUP>"),
    ("<u>", "<U>"),
    ("</u>", "</U>"),
];

// A failed proof leaves the original rendering and its old authority unchanged. ~keep
pub(super) fn track<'a>(
    tracked: &mut Option<Vec<&'a AstNode<'a>>>,
    budget: Option<&mut usize>,
    open: &'a AstNode<'a>,
    close: &'a AstNode<'a>,
) {
    if let (Some(nodes), Some(budget)) = (tracked.as_mut(), budget) {
        const TAG_STORAGE: usize = 2 * size_of::<usize>() + size_of::<Range<usize>>() + "</SUP>".len();
        let cost = 2 * TAG_STORAGE;
        if *budget < cost || nodes.try_reserve(2).is_err() {
            *tracked = None;
        } else {
            *budget -= cost;
            nodes.extend([open, close]);
        }
    }
}

pub(super) fn verify<'a>(
    root: &'a AstNode<'a>,
    options: &Options<'_>,
    doc: &InternalDocument,
    canonical: (&str, usize),
    nodes: &[&'a AstNode<'a>],
    budget: &mut usize,
) -> Option<Vec<Range<usize>>> {
    if nodes.is_empty() {
        return None;
    }
    let (output, raw_len) = canonical;
    let buffers = raw_len.checked_add(output.len())?.checked_mul(2)?;
    let Some(remaining) = budget.checked_sub(buffers) else {
        *budget = 0;
        return None;
    };
    let visited = root.descendants().take(remaining.saturating_add(1)).count();
    *budget = remaining.saturating_sub(visited);
    if visited > remaining {
        return None;
    }
    for node in nodes {
        let mut data = node.data.borrow_mut();
        let NodeValue::HtmlInline(tag) = &mut data.value else {
            return None;
        };
        let (_, alternate) = TAGS.iter().find(|(canonical, _)| *canonical == tag.as_str())?;
        *tag = (*alternate).into();
    }
    let mut alternate = String::new();
    alternate.try_reserve_exact(raw_len).ok()?;
    format_commonmark(root, options, &mut alternate).ok()?;
    if alternate.len() != raw_len {
        return None;
    }
    let alternate = finish_markdown(alternate, doc);
    ranges(output.as_bytes(), alternate.as_bytes(), nodes.len())
}

fn ranges(output: &[u8], alternate: &[u8], expected: usize) -> Option<Vec<Range<usize>>> {
    if output.len() != alternate.len() {
        return None;
    }
    let mut ranges = Vec::new();
    ranges.try_reserve_exact(expected).ok()?;
    let mut cursor = 0;
    while cursor < output.len() {
        if let Some((tag, _)) = TAGS.iter().find(|(tag, changed)| {
            output[cursor..].starts_with(tag.as_bytes()) && alternate[cursor..].starts_with(changed.as_bytes())
        }) {
            if ranges.len() == expected {
                return None;
            }
            ranges.push(cursor..cursor + tag.len());
            cursor += tag.len();
        } else if output[cursor] == alternate[cursor] {
            cursor += 1;
        } else {
            return None;
        }
    }
    (ranges.len() == expected).then_some(ranges)
}

#[cfg(test)]
mod tests {
    use super::ranges;

    #[test]
    fn should_ignore_identical_source_tags_and_reject_unexpected_formatter_changes() {
        let output = b"<sup>2</sup> <SUP> <sup>";
        assert_eq!(ranges(output, b"<SUP>2</SUP> <SUP> <sup>", 2), Some(vec![0..5, 6..12]));
        assert_eq!(ranges(output, b"<SUP>3</SUP> <SUP> <sup>", 2), None);
        assert_eq!(ranges(output, b"<SUP>2</SUP> <SUP> <SUP>", 2), None);
    }
}
