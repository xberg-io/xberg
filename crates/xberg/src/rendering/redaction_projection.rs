use crate::extractors::security::SecurityLimits;
use comrak::nodes::{AstNode, NodeCode, NodeMath, NodeValue};
use std::ops::Range;

#[derive(Clone)]
struct Budget {
    bytes: usize,
    operations: usize,
    failed: bool,
}

impl Budget {
    fn spend(&mut self, bytes: usize, operations: usize) -> bool {
        let remaining = self
            .bytes
            .checked_sub(bytes)
            .zip(self.operations.checked_sub(operations));
        if !self.failed
            && let Some((bytes, operations)) = remaining
        {
            self.bytes = bytes;
            self.operations = operations;
            true
        } else {
            self.failed = true;
            false
        }
    }
}

struct Origin {
    visible: Range<usize>,
    source: Option<Range<usize>>,
    identity: bool,
    discarded_generated: bool,
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

#[derive(Clone, Copy)]
enum PlainAtomKind {
    DefaultAlert,
    FootnoteReference,
    FootnoteDefinition,
    DisplayMath,
}

struct PlainAtom {
    visible: Range<usize>,
    kind: PlainAtomKind,
}

pub(crate) struct VisibleMarkdown {
    pub text: String,
    origins: Vec<Origin>,
    atoms: Vec<PlainAtom>,
    budget: Budget,
}

impl VisibleMarkdown {
    pub fn parse(markdown: &str, limits: &SecurityLimits) -> Option<Self> {
        if markdown.len() > limits.max_content_size {
            return None;
        }
        let mut budget = Budget {
            bytes: limits.max_content_size,
            operations: limits.max_iterations,
            failed: false,
        };
        let line_count = markdown.bytes().filter(|byte| *byte == b'\n').count().checked_add(1)?;
        if !budget.spend(line_count.checked_mul(size_of::<usize>())?, markdown.len()) {
            return None;
        }
        let arena = comrak::Arena::new();
        let root = comrak::parse_document(&arena, markdown, &super::markdown::comrak_options());
        super::inline_html::decode_standard_inline_html(root, &arena);
        let mut lines = vec![0];
        lines.extend(markdown.match_indices('\n').map(|(index, _)| index + 1));
        let mut result = Self {
            text: String::new(),
            origins: Vec::new(),
            atoms: Vec::new(),
            budget,
        };
        for node in root.descendants() {
            let depth = node.ancestors().take(limits.max_nesting_depth.checked_add(1)?).count();
            if depth > limits.max_nesting_depth || !result.budget.spend(0, depth.checked_add(1)?) {
                return None;
            }
            result.append(node, markdown, &lines)?;
            if result.budget.failed {
                return None;
            }
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
            NodeValue::Math(math) => self.inline_math(node, math, markdown, lines),
            NodeValue::FootnoteReference(reference) => {
                self.label(&reference.name, PlainAtomKind::FootnoteReference);
                Some(())
            }
            NodeValue::CodeBlock(block) => {
                self.block_boundary();
                self.definition_label(node);
                let text = block.literal.strip_suffix('\n').unwrap_or(&block.literal);
                if block.info == "math" && block.fenced && block.closed {
                    self.atom(text, PlainAtomKind::DisplayMath);
                } else {
                    self.unmapped(text);
                }
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
            value => self.structure(node, value),
        }
    }

    fn structure(&mut self, node: &AstNode<'_>, value: &NodeValue) -> Option<()> {
        match value {
            NodeValue::Paragraph => {
                self.block_boundary();
                self.definition_label(node);
            }
            NodeValue::FootnoteDefinition(_) => {
                node.first_child().filter(|child| match &child.data.borrow().value {
                    NodeValue::Paragraph => true,
                    NodeValue::CodeBlock(block) => block.info == "math",
                    _ => false,
                })?;
            }
            NodeValue::Alert(alert) => {
                self.block_boundary();
                if let Some(title) = &alert.title {
                    self.unmapped(title);
                } else {
                    self.label(alert.alert_type.default_title(), PlainAtomKind::DefaultAlert);
                }
            }
            value => {
                if structural(value)? {
                    self.block_boundary();
                }
            }
        }
        Some(())
    }

    fn definition_label(&mut self, node: &AstNode<'_>) {
        if node.previous_sibling().is_none()
            && let Some(parent) = node.parent()
            && let NodeValue::FootnoteDefinition(definition) = &parent.data.borrow().value
        {
            self.label(&definition.name, PlainAtomKind::FootnoteDefinition);
            self.unmapped(" ");
        }
    }

    fn inline_math(&mut self, node: &AstNode<'_>, math: &NodeMath, markdown: &str, lines: &[usize]) -> Option<()> {
        let (opening, closing) = if !math.dollar_math {
            ("$`", "`$")
        } else if math.display_math {
            ("$$", "$$")
        } else {
            ("$", "$")
        };
        let source = source_range(node, markdown, lines).and_then(|range| {
            let raw = markdown.get(range.clone())?;
            let payload = raw.strip_prefix(opening)?.strip_suffix(closing)?;
            let start = range.start + opening.len();
            if payload == math.literal {
                return Some(start..range.end - closing.len());
            }
            // Code-math removes exactly one edge-space pair only when the payload is not all spaces. ~keep
            (!math.dollar_math
                && !payload.contains(['\r', '\n'])
                && payload.bytes().any(|byte| byte != b' ')
                && payload.strip_prefix(' ')?.strip_suffix(' ')? == math.literal)
                .then_some(start + 1..range.end - closing.len() - 1)
        });
        if let Some(source) = source {
            self.leaf(&math.literal, markdown, source, false)
        } else {
            self.unmapped(&math.literal);
            Some(())
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
            if self.budget.failed {
                return None;
            }
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
        let merge = identity
            && self.origins.last().is_some_and(|last| {
                last.identity
                    && last.visible.end == start
                    && last
                        .source
                        .as_ref()
                        .zip(source.as_ref())
                        .is_some_and(|(previous, next)| previous.end == next.start)
            });
        let Some(bytes) = text.len().checked_add(if merge { 0 } else { size_of::<Origin>() }) else {
            self.budget.failed = true;
            return;
        };
        if !self.budget.spend(bytes, text.len()) {
            return;
        }
        self.text.push_str(text);
        if merge {
            let last = self.origins.last_mut().expect("merge checked the last origin");
            last.source.as_mut().expect("merge checked the source").end =
                source.as_ref().expect("merge checked source").end;
            last.visible.end = self.text.len();
        } else {
            self.origins.push(Origin {
                visible: start..self.text.len(),
                source,
                identity,
                discarded_generated: false,
            });
        }
    }

    fn label(&mut self, text: &str, kind: PlainAtomKind) {
        let start = self.text.len();
        self.unmapped(text);
        if !text.is_empty() && !text.chars().any(char::is_whitespace) {
            if !self.budget.spend(size_of::<PlainAtom>(), 1) {
                return;
            }
            self.atoms.push(PlainAtom {
                visible: start..self.text.len(),
                kind,
            });
        }
    }

    fn atom(&mut self, text: &str, kind: PlainAtomKind) {
        let start = self.text.len();
        self.unmapped(text);
        if !text.is_empty() {
            if !self.budget.spend(size_of::<PlainAtom>(), 1) {
                return;
            }
            self.atoms.push(PlainAtom {
                visible: start..self.text.len(),
                kind,
            });
        }
    }

    fn discarded_label(&mut self, text: &str) {
        let start = self.text.len();
        let Some(bytes) = text.len().checked_add(size_of::<Origin>()) else {
            self.budget.failed = true;
            return;
        };
        if !self.budget.spend(bytes, text.len()) {
            return;
        }
        self.text.push_str(text);
        // This generated enum title disappears in Plain; its actual source kind is redacted separately. ~keep
        self.origins.push(Origin {
            visible: start..self.text.len(),
            source: None,
            identity: false,
            discarded_generated: true,
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
                    origin.discarded_generated
                        || origin.source_intersection(finding).is_some_and(|source| {
                            let index = selected.partition_point(|range| range.end <= source.start);
                            selected
                                .get(index)
                                .is_some_and(|range| range.start <= source.start && range.end >= source.end)
                        })
                })
        })
    }

    fn associate_atom(&self, label: &PlainAtom, plain: &str, start: usize, result: &mut Self) -> Option<usize> {
        let text = self.text.get(label.visible.clone())?;
        let (payload, end) = match label.kind {
            PlainAtomKind::DefaultAlert => {
                let kind = plain.get(start..)?.split_whitespace().next()?;
                let alert = super::comrak_bridge::admonition_alert_type(kind)?;
                (alert.default_title() == text).then_some(())?;
                result.discarded_label(text);
                return start.checked_add(kind.len());
            }
            PlainAtomKind::DisplayMath => math_plain_payload(plain, start, text)?,
            kind @ (PlainAtomKind::FootnoteReference | PlainAtomKind::FootnoteDefinition) => {
                let suffix = if matches!(kind, PlainAtomKind::FootnoteDefinition) {
                    ":"
                } else {
                    ""
                };
                let marker = format!("[^{text}]{suffix}");
                let end = start.checked_add(marker.len())?;
                (plain.get(start..end)? == marker).then_some(())?;
                let payload_start = start.checked_add("[^".len())?;
                (payload_start..payload_start.checked_add(text.len())?, end)
            }
        };
        for (offset, character) in text.char_indices() {
            let source = payload.start + offset..payload.start + offset + character.len_utf8();
            let identity = plain.get(source.clone())?.starts_with(character);
            result.push(character, Some(source), identity);
        }
        Some(end)
    }

    // Consume complete streams, never search for a repeated value. Whitespace runs are
    // associated without changing either string supplied to the configured matchers. ~keep
    pub fn on_plain(&self, plain: &str) -> Option<Self> {
        let mut source = plain.char_indices().peekable();
        let mut visible = self.text.char_indices().peekable();
        let mut result = Self {
            text: String::new(),
            origins: Vec::new(),
            atoms: Vec::new(),
            budget: self.budget.clone(),
        };
        let mut atoms = self.atoms.iter().peekable();
        while let Some((visible_start, character)) = visible.peek().copied() {
            if result.budget.failed {
                return None;
            }
            let (start, actual) = source.peek().copied()?;
            if let Some(label) = atoms.next_if(|label| label.visible.start == visible_start) {
                let end = self.associate_atom(label, plain, start, &mut result)?;
                while source.next_if(|(index, _)| *index < end).is_some() {}
                while visible.next_if(|(index, _)| *index < label.visible.end).is_some() {}
                continue;
            }
            if character.is_whitespace() && actual.is_whitespace() {
                let mut end = start;
                while let Some((index, next)) = source.next_if(|(_, next)| next.is_whitespace()) {
                    end = index + next.len_utf8();
                }
                let next_atom = atoms.peek().map(|atom| atom.visible.start);
                while visible
                    .next_if(|(index, next)| next.is_whitespace() && next_atom.is_none_or(|start| *index < start))
                    .is_some()
                {}
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
        (!result.budget.failed && atoms.next().is_none() && source.next().is_none()).then_some(result)
    }
}

fn math_plain_payload(plain: &str, start: usize, text: &str) -> Option<(Range<usize>, usize)> {
    let direct_end = start.checked_add(text.len())?;
    if plain.get(start..direct_end) == Some(text) {
        return Some((start..direct_end, direct_end));
    }
    let payload_start = start.checked_add("$$".len())?;
    let payload_end = payload_start.checked_add(text.len())?;
    let end = payload_end.checked_add("$$".len())?;
    (plain.get(start..payload_start) == Some("$$")
        && plain.get(payload_start..payload_end) == Some(text)
        && plain.get(payload_end..end) == Some("$$"))
    .then_some((payload_start..payload_end, end))
}

fn structural(value: &NodeValue) -> Option<bool> {
    match value {
        NodeValue::Heading(_) | NodeValue::TableCell => Some(true),
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
