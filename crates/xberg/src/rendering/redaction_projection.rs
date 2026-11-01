use comrak::nodes::{AstNode, NodeValue};
use std::ops::Range;

struct Origin {
    visible: Range<usize>,
    source: Option<Range<usize>>,
}

pub(crate) struct VisibleMarkdown {
    pub text: String,
    origins: Vec<Origin>,
}

impl VisibleMarkdown {
    pub fn parse(markdown: &str) -> Option<Self> {
        let arena = comrak::Arena::new();
        let root = comrak::parse_document(&arena, markdown, &super::markdown::comrak_options());
        super::inline_html::decode_standard_inline_html(root, &arena);
        let mut lines = vec![0];
        lines.extend(markdown.match_indices('\n').map(|(index, _)| index + 1));
        let mut result = Self {
            text: String::new(),
            origins: Vec::new(),
        };
        for node in root.descendants() {
            result.append(node, markdown, &lines)?;
        }
        Some(result)
    }

    fn append(&mut self, node: &AstNode<'_>, markdown: &str, lines: &[usize]) -> Option<()> {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Text(text) => self.leaf(text, markdown, source_range(node, markdown, lines)?, true),
            NodeValue::HtmlInline(text) | NodeValue::Raw(text) | NodeValue::EscapedTag(text) => {
                self.leaf(text, markdown, source_range(node, markdown, lines)?, false)
            }
            NodeValue::Code(code) => {
                let range = source_range(node, markdown, lines)?;
                let source = range.start.checked_add(code.num_backticks)?..range.end.checked_sub(code.num_backticks)?;
                (markdown.get(source.clone())? == code.literal).then_some(())?;
                self.leaf(&code.literal, markdown, source, false)
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => {
                self.synthetic(if matches!(data.value, NodeValue::SoftBreak) {
                    " "
                } else {
                    "\n"
                });
                Some(())
            }
            value => {
                if structural(value)? && !self.text.is_empty() {
                    self.synthetic("\n\n");
                }
                Some(())
            }
        }
    }

    fn leaf(&mut self, visible: &str, markdown: &str, source: Range<usize>, escapes: bool) -> Option<()> {
        let raw = markdown.get(source.clone())?;
        let mut offset = 0;
        for character in visible.chars() {
            let start = offset;
            if escapes
                && raw.as_bytes().get(offset) == Some(&b'\\')
                && character.is_ascii_punctuation()
                && raw.get(offset + 1..)?.chars().next() == Some(character)
            {
                offset += 1;
            }
            let next = raw.get(offset..)?.chars().next()?;
            if next != character {
                return None;
            }
            offset += next.len_utf8();
            self.push(character, Some(source.start + start..source.start + offset));
        }
        (offset == raw.len()).then_some(())
    }

    fn push(&mut self, character: char, source: Option<Range<usize>>) {
        let start = self.text.len();
        self.text.push(character);
        self.origins.push(Origin {
            visible: start..self.text.len(),
            source,
        });
    }

    fn synthetic(&mut self, text: &str) {
        for character in text.chars() {
            self.push(character, None);
        }
    }

    pub fn covered(&self, sensitive: &[Range<usize>], selected: &[Range<usize>]) -> bool {
        sensitive.iter().all(|finding| {
            let start = self
                .origins
                .partition_point(|origin| origin.visible.end <= finding.start);
            let end = self
                .origins
                .partition_point(|origin| origin.visible.start < finding.end);
            start < end
                && self.origins[start..end].iter().all(|origin| {
                    origin.source.as_ref().is_some_and(|source| {
                        let index = selected.partition_point(|range| range.end <= source.start);
                        selected
                            .get(index)
                            .is_some_and(|range| range.start <= source.start && range.end >= source.end)
                    })
                })
        })
    }

    // Consume complete streams, never search for a repeated value. Whitespace runs are
    // associated without changing either string supplied to the configured matchers. ~keep
    pub fn on_plain(&self, plain: &str) -> Option<Self> {
        let mut source = plain.char_indices().peekable();
        let mut visible = self.text.char_indices().peekable();
        let mut result = Self {
            text: String::new(),
            origins: Vec::new(),
        };
        while let Some((_, character)) = visible.peek().copied() {
            let (start, actual) = source.peek().copied()?;
            if character.is_whitespace() && actual.is_whitespace() {
                let mut end = start;
                while let Some((index, next)) = source.next_if(|(_, next)| next.is_whitespace()) {
                    end = index + next.len_utf8();
                }
                while let Some((_, next)) = visible.next_if(|(_, next)| next.is_whitespace()) {
                    result.push(next, Some(start..end));
                }
            } else {
                if actual != character {
                    return None;
                }
                source.next();
                visible.next();
                result.push(character, Some(start..start + actual.len_utf8()));
            }
        }
        source.next().is_none().then_some(result)
    }
}

fn structural(value: &NodeValue) -> Option<bool> {
    match value {
        NodeValue::Paragraph | NodeValue::Heading(_) | NodeValue::TableCell => Some(true),
        NodeValue::Document
        | NodeValue::BlockQuote
        | NodeValue::List(_)
        | NodeValue::Item(_)
        | NodeValue::DescriptionList
        | NodeValue::DescriptionItem(_)
        | NodeValue::DescriptionTerm
        | NodeValue::DescriptionDetails
        | NodeValue::Table(_)
        | NodeValue::TableRow(_)
        | NodeValue::Emph
        | NodeValue::Strong
        | NodeValue::Strikethrough
        | NodeValue::Underline
        | NodeValue::Insert
        | NodeValue::Highlight
        | NodeValue::Superscript
        | NodeValue::Subscript
        | NodeValue::Link(_)
        | NodeValue::Image(_)
        | NodeValue::ThematicBreak => Some(false),
        _ => None,
    }
}

fn source_range(node: &AstNode<'_>, markdown: &str, lines: &[usize]) -> Option<Range<usize>> {
    let position = node.data.borrow().sourcepos;
    let start = lines
        .get(position.start.line.checked_sub(1)?)?
        .checked_add(position.start.column.checked_sub(1)?)?;
    let end = lines
        .get(position.end.line.checked_sub(1)?)?
        .checked_add(position.end.column)?;
    markdown.get(start..end)?;
    Some(start..end)
}
