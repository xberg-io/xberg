use comrak::nodes::{AstNode, NodeCode, NodeValue};
use std::ops::Range;

struct Origin {
    visible: Range<usize>,
    source: Option<Range<usize>>,
    identity: bool,
}

impl Origin {
    fn source_intersection(&self, finding: &Range<usize>) -> Option<Range<usize>> {
        let source = self.source.as_ref()?;
        if self.identity {
            let start = finding.start.max(self.visible.start) - self.visible.start;
            let end = finding.end.min(self.visible.end) - self.visible.start;
            Some(source.start.checked_add(start)?..source.start.checked_add(end)?)
        } else {
            Some(source.clone())
        }
    }
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
            NodeValue::HtmlInline(text) | NodeValue::Raw(text) => {
                self.leaf(text, markdown, source_range(node, markdown, lines)?, false)
            }
            NodeValue::EscapedTag(text) => self.leaf(text, markdown, source_range(node, markdown, lines)?, false),
            NodeValue::Code(code) => self.inline_code(node, code, markdown, lines),
            NodeValue::CodeBlock(block) => {
                self.block_boundary();
                self.unmapped(block.literal.strip_suffix('\n').unwrap_or(&block.literal));
                Some(())
            }
            NodeValue::HtmlBlock(block) => {
                self.block_boundary();
                self.unmapped(block.literal.strip_suffix('\n').unwrap_or(&block.literal));
                Some(())
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => {
                self.unmapped(if matches!(data.value, NodeValue::SoftBreak) {
                    " "
                } else {
                    "\n"
                });
                Some(())
            }
            value => {
                if structural(value)? {
                    self.block_boundary();
                }
                Some(())
            }
        }
    }

    fn inline_code(&mut self, node: &AstNode<'_>, code: &NodeCode, markdown: &str, lines: &[usize]) -> Option<()> {
        let source = source_range(node, markdown, lines).and_then(|range| {
            Some(range.start.checked_add(code.num_backticks)?..range.end.checked_sub(code.num_backticks)?)
        });
        if let Some(source) = source.filter(|range| markdown.get(range.clone()) == Some(code.literal.as_str())) {
            self.leaf(&code.literal, markdown, source, false)
        } else {
            self.unmapped(&code.literal);
            Some(())
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
                && raw.get(offset + 1..)?.starts_with(character)
            {
                offset += 1;
            }
            let next = raw.get(offset..)?.chars().next()?;
            if next != character {
                return None;
            }
            offset += next.len_utf8();
            self.push(
                character,
                Some(source.start + start..source.start + offset),
                offset - start == character.len_utf8(),
            );
        }
        (offset == raw.len()).then_some(())
    }

    fn push(&mut self, character: char, source: Option<Range<usize>>, identity: bool) {
        self.span(character.encode_utf8(&mut [0; 4]), source, identity);
    }

    fn span(&mut self, text: &str, source: Option<Range<usize>>, identity: bool) {
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        if identity
            && let Some(last) = self.origins.last_mut()
            && last.identity
            && last.visible.end == start
            && let (Some(previous), Some(next)) = (last.source.as_mut(), source.as_ref())
            && previous.end == next.start
        {
            previous.end = next.end;
            last.visible.end = self.text.len();
            return;
        }
        self.origins.push(Origin {
            visible: start..self.text.len(),
            source,
            identity,
        });
    }

    fn unmapped(&mut self, text: &str) {
        self.span(text, None, false);
    }

    fn block_boundary(&mut self) {
        if !self.text.is_empty() {
            self.unmapped("\n\n");
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
                    origin.source_intersection(finding).is_some_and(|source| {
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
        while let Some((visible_start, character)) = visible.peek().copied() {
            let (start, actual) = source.peek().copied()?;
            if character.is_whitespace() && actual.is_whitespace() {
                let mut end = start;
                while let Some((index, next)) = source.next_if(|(_, next)| next.is_whitespace()) {
                    end = index + next.len_utf8();
                }
                while visible.next_if(|(_, next)| next.is_whitespace()).is_some() {}
                let visible_end = visible.peek().map_or(self.text.len(), |(index, _)| *index);
                let text = &self.text[visible_start..visible_end];
                let identity = text.len() == character.len_utf8() && plain.get(start..end) == Some(text);
                result.span(text, Some(start..end), identity);
            } else {
                if actual != character {
                    return None;
                }
                source.next();
                visible.next();
                result.push(character, Some(start..start + actual.len_utf8()), true);
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
