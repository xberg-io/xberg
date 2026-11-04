use super::*;
use crate::rendering::{PrivacyInlineKind, privacy_inline_tag};
use comrak::arena_tree::NodeEdge;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Strong,
    Emphasis,
    Html(PrivacyInlineKind),
}
struct Frame {
    kind: Kind,
    raw: Range<usize>,
    start: usize,
    owner: usize,
}
#[derive(Debug)]
struct Construct {
    decoded: Range<usize>,
    raw: Range<usize>,
    payload: Range<usize>,
    preserve_envelope: bool,
}
#[derive(Debug)]
pub(super) struct Projection<'s> {
    source: &'s str,
    pub(super) leaf: Leaf,
    constructs: Vec<Construct>,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Mapped {
    pub(super) raw: Range<usize>,
    pub(super) semantic: Range<usize>,
}

fn capacities(source: &str, depth: usize) -> Result<(usize, usize, usize), Error> {
    let lines = source
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        .checked_add(1)
        .ok_or(Error::Budget)?;
    let frames = depth.min(source.len());
    let extra = lines
        .checked_mul(size_of::<usize>())
        .and_then(|n| frames.checked_mul(size_of::<Frame>()).and_then(|v| n.checked_add(v)))
        .and_then(|n| {
            source
                .len()
                .checked_mul(size_of::<Construct>())
                .and_then(|v| n.checked_add(v))
        })
        .ok_or(Error::Budget)?;
    Ok((lines, frames, extra))
}

fn source_range(source: &str, node: &AstNode<'_>, lines: &[usize]) -> Result<Range<usize>, Error> {
    let pos = node.data.borrow().sourcepos;
    let start = lines
        .get(pos.start.line.checked_sub(1).ok_or(Error::InvalidSource)?)
        .and_then(|base| base.checked_add(pos.start.column.checked_sub(1)?))
        .ok_or(Error::InvalidSource)?;
    let end = lines
        .get(pos.end.line.checked_sub(1).ok_or(Error::InvalidSource)?)
        .and_then(|base| base.checked_add(pos.end.column))
        .ok_or(Error::InvalidSource)?;
    source.get(start..end).ok_or(Error::InvalidSource)?;
    Ok(start..end)
}

struct Builder<'s> {
    decoder: Decoder<'s>,
    lines: Vec<usize>,
    frames: Vec<Frame>,
    frame_limit: usize,
    constructs: Vec<Construct>,
    paragraph_end: Option<usize>,
}
impl<'s> Builder<'s> {
    fn prepare(source: &'s str, limits: Limits, depth: usize, work: &mut Work) -> Result<Self, Error> {
        if source.len() > limits.input {
            work.failure = Some("input content limit");
            return Err(Error::Budget);
        }
        work.charge(source.len(), limits)?;
        let (line_count, frame_count, extra) = capacities(source, depth)?;
        let total = reservation(source.len())?
            .checked_add(extra)
            .and_then(|n| work.reserved.checked_add(n))
            .ok_or(Error::Budget)?;
        if total > limits.owned {
            return Err(Error::Budget);
        }
        let decoder = Decoder::prepare(source, limits, work)?;
        let mut lines = Vec::new();
        let mut frames = Vec::new();
        let mut constructs = Vec::new();
        lines.try_reserve_exact(line_count).map_err(|_| Error::Budget)?;
        frames.try_reserve_exact(frame_count).map_err(|_| Error::Budget)?;
        constructs.try_reserve_exact(source.len()).map_err(|_| Error::Budget)?;
        lines.push(0);
        lines.extend(source.match_indices('\n').map(|(index, _)| index + 1));
        work.reserved = total;
        Ok(Self {
            decoder,
            lines,
            frames,
            frame_limit: frame_count,
            constructs,
            paragraph_end: None,
        })
    }

    fn construct(
        &mut self,
        decoded: Range<usize>,
        raw: Range<usize>,
        payload: Range<usize>,
        preserve_envelope: bool,
    ) -> Result<(), Error> {
        if payload.start < raw.start || payload.end > raw.end || self.decoder.source.get(payload.clone()).is_none() {
            return Err(Error::InvalidSource);
        }
        if self.constructs.len() == self.decoder.source.len() {
            return Err(Error::Budget);
        }
        self.constructs.push(Construct {
            decoded,
            raw,
            payload,
            preserve_envelope,
        });
        Ok(())
    }

    fn open(&mut self, kind: Kind, raw: Range<usize>, owner: usize) -> Result<(), Error> {
        if self.frames.len() == self.frame_limit {
            return Err(Error::Budget);
        }
        self.frames.push(Frame {
            kind,
            raw,
            start: self.decoder.leaf.text.len(),
            owner,
        });
        Ok(())
    }

    fn close(&mut self, kind: Kind, raw: Range<usize>, owner: usize) -> Result<(), Error> {
        let frame = self.frames.pop().ok_or(Error::Unsupported)?;
        if frame.kind != kind || frame.owner != owner {
            return Err(Error::Unsupported);
        }
        let end = self.decoder.leaf.text.len();
        let payload = match kind {
            Kind::Strong => frame.raw.start + 2..raw.end - 2,
            Kind::Emphasis => frame.raw.start + 1..raw.end - 1,
            Kind::Html(_) => frame.raw.end..raw.start,
        };
        self.construct(frame.start..end, frame.raw.start..raw.end, payload, false)
    }

    fn delimiter(&self, range: Range<usize>, strong: bool) -> Result<(), Error> {
        let source = self.decoder.source.get(range).ok_or(Error::InvalidSource)?;
        let delimiters: &[&str] = if strong { &["**", "__"] } else { &["*", "_"] };
        delimiters
            .iter()
            .any(|d| source.len() >= d.len() * 2 && source.starts_with(*d) && source.ends_with(*d))
            .then_some(())
            .ok_or(Error::InvalidSource)
    }

    fn whitespace_gap(&self, gap: Range<usize>, work: &mut Work) -> Result<(), Error> {
        let bytes = self.decoder.source.get(gap).ok_or(Error::InvalidSource)?;
        work.charge(bytes.len(), self.decoder.limits)?;
        if !bytes.bytes().all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n')) {
            return Err(Error::Unsupported);
        }
        Ok(())
    }

    fn paragraph(&mut self, node: &AstNode<'_>, work: &mut Work) -> Result<(), Error> {
        let parent = node.parent().ok_or(Error::Unsupported)?;
        if !matches!(parent.data.borrow().value, NodeValue::Document)
            || parent.parent().is_some()
            || !self.frames.is_empty()
        {
            return Err(Error::Unsupported);
        }
        let range = source_range(self.decoder.source, node, &self.lines)?;
        let gap = self.paragraph_end.unwrap_or(0)..range.start;
        self.whitespace_gap(gap.clone(), work)?;
        if self.paragraph_end.is_some() {
            self.decoder.text(gap, work)?;
        }
        self.paragraph_end = Some(range.end);
        Ok(())
    }

    fn start(&mut self, node: &AstNode<'_>, work: &mut Work) -> Result<(), Error> {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Document => Ok(()),
            NodeValue::Paragraph => self.paragraph(node, work),
            NodeValue::Text(expected) => {
                let range = source_range(self.decoder.source, node, &self.lines)?;
                let start = self.decoder.leaf.text.len();
                self.decoder.text(range, work)?;
                (self.decoder.leaf.text.get(start..) == Some(expected.as_ref()))
                    .then_some(())
                    .ok_or(Error::InvalidSource)
            }
            NodeValue::Code(code) => {
                let range = source_range(self.decoder.source, node, &self.lines)?;
                let start = self.decoder.leaf.text.len();
                let origin_start = self.decoder.leaf.origins.len();
                self.decoder
                    .code(range.clone(), code.num_backticks, &code.literal, work)?;
                // The code decoder owns both tick runs and any padding it trimmed. ~keep
                let origins = &self.decoder.leaf.origins[origin_start..];
                let payload = match (origins.first(), origins.last()) {
                    (Some(first), Some(last)) => first.source.start..last.source.end,
                    _ => range.clone(),
                };
                self.construct(start..self.decoder.leaf.text.len(), range, payload, true)
            }
            NodeValue::Strong | NodeValue::Emph => {
                let strong = matches!(data.value, NodeValue::Strong);
                let range = source_range(self.decoder.source, node, &self.lines)?;
                self.delimiter(range.clone(), strong)?;
                self.open(
                    if strong { Kind::Strong } else { Kind::Emphasis },
                    range,
                    node as *const _ as usize,
                )
            }
            NodeValue::HtmlInline(tag) => {
                let (kind, opening) = privacy_inline_tag(node).ok_or(Error::Unsupported)?;
                let range = source_range(self.decoder.source, node, &self.lines)?;
                if self.decoder.source.get(range.clone()) != Some(tag.as_str()) {
                    return Err(Error::InvalidSource);
                }
                let owner = node.parent().ok_or(Error::Unsupported)? as *const _ as usize;
                if opening {
                    self.open(Kind::Html(kind), range, owner)
                } else {
                    self.close(Kind::Html(kind), range, owner)
                }
            }
            _ => Err(Error::Unsupported),
        }
    }

    fn end(&mut self, node: &AstNode<'_>) -> Result<(), Error> {
        let value = &node.data.borrow().value;
        let kind = match value {
            NodeValue::Paragraph => return self.frames.is_empty().then_some(()).ok_or(Error::Unsupported),
            NodeValue::Strong => Kind::Strong,
            NodeValue::Emph => Kind::Emphasis,
            _ => return Ok(()),
        };
        let range = source_range(self.decoder.source, node, &self.lines)?;
        self.close(kind, range, node as *const _ as usize)
    }
}

pub(super) fn project<'s>(
    source: &'s str,
    limits: Limits,
    depth_limit: usize,
    work: &mut Work,
) -> Result<Projection<'s>, Error> {
    let mut builder = Builder::prepare(source, limits, depth_limit, work)?;
    let arena = comrak::Arena::new();
    work.parses = work.parses.checked_add(1).ok_or(Error::Budget)?;
    work.document_parses = work.document_parses.checked_add(1).ok_or(Error::Budget)?;
    let root = comrak::parse_document(&arena, source, &crate::rendering::comrak_options());
    let mut depth = 0usize;
    for edge in root.traverse() {
        work.charge(1, limits)?;
        match edge {
            NodeEdge::Start(node) => {
                depth = depth.checked_add(1).ok_or(Error::Budget)?;
                if depth > depth_limit {
                    work.failure = Some("nesting depth limit");
                    return Err(Error::Budget);
                }
                builder.start(node, work)?;
            }
            NodeEdge::End(node) => {
                builder.end(node)?;
                depth = depth.checked_sub(1).ok_or(Error::InvalidSource)?;
            }
        }
    }
    if !builder.frames.is_empty() {
        return Err(Error::Unsupported);
    }
    builder.whitespace_gap(builder.paragraph_end.unwrap_or(0)..source.len(), work)?;
    builder.decoder.leaf.source = 0..source.len();
    let projection = Projection {
        source,
        leaf: builder.decoder.leaf,
        constructs: builder.constructs,
    };
    projection.validate(limits, work)?;
    Ok(projection)
}

impl Projection<'_> {
    pub(super) fn drop_ranges(
        &self,
        matches: impl ExactSizeIterator<Item = Range<usize>>,
        limits: Limits,
        work: &mut Work,
    ) -> Result<Vec<Range<usize>>, Error> {
        drop::ranges(self, matches, limits, work)
    }

    fn validate(&self, limits: Limits, work: &mut Work) -> Result<(), Error> {
        work.charge(
            self.leaf
                .text
                .len()
                .checked_add(self.leaf.origins.len())
                .and_then(|n| n.checked_add(self.constructs.len()))
                .ok_or(Error::Budget)?,
            limits,
        )?;
        let mut decoded_end = 0;
        let mut previous = 0..0;
        for origin in &self.leaf.origins {
            let scalar = self.leaf.text.get(origin.decoded.clone()).ok_or(Error::InvalidSource)?;
            if origin.decoded.start != decoded_end
                || scalar.chars().count() != 1
                || origin.source.is_empty()
                || origin.source.start < previous.start
                || origin.source.end < previous.end
                || (origin.source.start < previous.end && origin.source != previous)
                || self.source.get(origin.source.clone()).is_none()
            {
                return Err(Error::InvalidSource);
            }
            decoded_end = origin.decoded.end;
            previous = origin.source.clone();
        }
        if decoded_end != self.leaf.text.len() {
            return Err(Error::InvalidSource);
        }
        for construct in &self.constructs {
            if self.leaf.text.get(construct.decoded.clone()).is_none()
                || construct.raw.is_empty()
                || self.source.get(construct.raw.clone()).is_none()
            {
                return Err(Error::InvalidSource);
            }
        }
        Ok(())
    }

    fn contiguous_identity(
        &self,
        origins: &[Origin],
        raw: &Range<usize>,
        expected: &str,
        limits: Limits,
        work: &mut Work,
    ) -> Result<bool, Error> {
        work.charge(
            expected
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(origins.len()))
                .ok_or(Error::Budget)?,
            limits,
        )?;
        if self.source.get(raw.clone()).ok_or(Error::InvalidSource)? != expected {
            return Ok(false);
        }
        let mut end = raw.start;
        for origin in origins {
            let source = self.source.get(origin.source.clone()).ok_or(Error::InvalidSource)?;
            let decoded = self.leaf.text.get(origin.decoded.clone()).ok_or(Error::InvalidSource)?;
            if origin.source.start != end || source != decoded {
                return Ok(false);
            }
            end = origin.source.end;
        }
        Ok(end == raw.end)
    }

    pub(super) fn map(
        &self,
        semantic: Range<usize>,
        expected: &str,
        limits: Limits,
        work: &mut Work,
    ) -> Result<Mapped, Error> {
        if semantic.is_empty() {
            return Err(Error::InvalidSource);
        }
        work.charge(
            self.leaf
                .origins
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(self.constructs.len()))
                .and_then(|n| n.checked_add(expected.len()))
                .ok_or(Error::Budget)?,
            limits,
        )?;
        if self.leaf.text.get(semantic.clone()) != Some(expected) {
            return Err(Error::InvalidSource);
        }
        let first = self
            .leaf
            .origins
            .partition_point(|origin| origin.decoded.end <= semantic.start);
        let end = self
            .leaf
            .origins
            .partition_point(|origin| origin.decoded.start < semantic.end);
        let origins = self
            .leaf
            .origins
            .get(first..end)
            .filter(|s| !s.is_empty())
            .ok_or(Error::InvalidSource)?;
        if origins[0].decoded.start != semantic.start || origins[origins.len() - 1].decoded.end != semantic.end {
            return Err(Error::InvalidSource);
        }
        let mut raw = origins[0].source.start..origins[origins.len() - 1].source.end;
        let contiguous = self.contiguous_identity(origins, &raw, expected, limits, work)?;
        for construct in &self.constructs {
            if !construct.decoded.is_empty()
                && semantic.start <= construct.decoded.start
                && semantic.end >= construct.decoded.end
                && (!contiguous || construct.preserve_envelope)
            {
                raw.start = raw.start.min(construct.raw.start);
                raw.end = raw.end.max(construct.raw.end);
            }
        }
        self.source.get(raw.clone()).ok_or(Error::InvalidSource)?;
        Ok(Mapped { raw, semantic })
    }
}

#[cfg(test)]
mod tests;

mod drop;
