use comrak::nodes::{AstNode, NodeValue};
use std::{mem::size_of, ops::Range};

const UTF8_BYTES: usize = char::MAX.len_utf8();
const SENTINEL_BYTES: usize = "az".len();

#[derive(Debug, PartialEq, Eq)]
enum Error {
    Budget,
    InvalidSource,
    MissingLeaf,
    UnsupportedAtom,
}

#[derive(Clone, Copy)]
struct Limits {
    input: usize,
    owned: usize,
    operations: usize,
}

#[derive(Debug, Default)]
struct Work {
    operations: usize,
    parses: usize,
    reserved: usize,
}

impl Work {
    fn charge(&mut self, amount: usize, limits: Limits) -> Result<(), Error> {
        self.operations = self.operations.checked_add(amount).ok_or(Error::Budget)?;
        (self.operations <= limits.operations)
            .then_some(())
            .ok_or(Error::Budget)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Origin {
    decoded: Range<usize>,
    source: Range<usize>,
}

#[derive(Debug)]
struct Leaf {
    text: String,
    origins: Vec<Origin>,
    source: Range<usize>,
}

struct Decoder<'s> {
    source: &'s str,
    leaf: Leaf,
    atom_input: String,
    atom_output: String,
    limits: Limits,
}

fn reservation(bytes: usize) -> Result<usize, Error> {
    // Three strings reserve decoded UTF8, a source atom plus sentinels, and decoded atom UTF8. ~keep
    bytes
        .checked_mul(size_of::<Origin>() + UTF8_BYTES * 2 + 1)
        .and_then(|n| n.checked_add(SENTINEL_BYTES * 2))
        .ok_or(Error::Budget)
}

impl<'s> Decoder<'s> {
    fn prepare(source: &'s str, limits: Limits, work: &mut Work) -> Result<Self, Error> {
        let reserved = work
            .reserved
            .checked_add(reservation(source.len())?)
            .ok_or(Error::Budget)?;
        if source.len() > limits.input || reserved > limits.owned {
            return Err(Error::Budget);
        }
        work.charge(source.len(), limits)?;
        work.reserved = reserved;
        let bytes = source.len();
        let mut decoder = Self {
            source,
            leaf: Leaf {
                text: String::new(),
                origins: Vec::new(),
                source: 0..0,
            },
            atom_input: String::new(),
            atom_output: String::new(),
            limits,
        };
        decoder
            .leaf
            .text
            .try_reserve_exact(bytes * UTF8_BYTES)
            .map_err(|_| Error::Budget)?;
        decoder
            .leaf
            .origins
            .try_reserve_exact(bytes)
            .map_err(|_| Error::Budget)?;
        decoder
            .atom_input
            .try_reserve_exact(bytes + SENTINEL_BYTES)
            .map_err(|_| Error::Budget)?;
        decoder
            .atom_output
            .try_reserve_exact(bytes * UTF8_BYTES + SENTINEL_BYTES)
            .map_err(|_| Error::Budget)?;
        Ok(decoder)
    }

    fn emit(&mut self, text: &str, source: Range<usize>, work: &mut Work) -> Result<(), Error> {
        work.charge(text.len().checked_add(source.len()).ok_or(Error::Budget)?, self.limits)?;
        for ch in text.chars() {
            if self.leaf.origins.len() == self.source.len()
                || self.leaf.text.len() + ch.len_utf8() > self.source.len() * UTF8_BYTES
            {
                return Err(Error::Budget);
            }
            let start = self.leaf.text.len();
            self.leaf.text.push(ch);
            self.leaf.origins.push(Origin {
                decoded: start..self.leaf.text.len(),
                source: source.clone(),
            });
        }
        Ok(())
    }

    fn text(&mut self, range: Range<usize>, work: &mut Work) -> Result<(), Error> {
        let mut i = range.start;
        while i < range.end {
            let ch = self.source[i..range.end].chars().next().ok_or(Error::InvalidSource)?;
            if ch == '\\' && self.source.as_bytes().get(i + 1).is_some_and(u8::is_ascii_punctuation) {
                self.emit(&self.source[i + 1..i + 2], i..i + 2, work)?;
                i += 2;
            } else if ch == '&' {
                let end = self.entity_end(i, range.end, work)?;
                if let Some(end) = end {
                    self.entity(i..end, work)?;
                    i = end;
                } else {
                    self.emit("&", i..i + 1, work)?;
                    i += 1;
                }
            } else {
                self.emit(&self.source[i..i + ch.len_utf8()], i..i + ch.len_utf8(), work)?;
                i += ch.len_utf8();
            }
        }
        Ok(())
    }

    fn entity_end(&self, start: usize, end: usize, work: &mut Work) -> Result<Option<usize>, Error> {
        let mut i = start + 1;
        while i < end && (self.source.as_bytes()[i].is_ascii_alphanumeric() || self.source.as_bytes()[i] == b'#') {
            work.charge(1, self.limits)?;
            i += 1;
        }
        Ok((i < end && self.source.as_bytes()[i] == b';').then_some(i + 1))
    }

    fn entity(&mut self, range: Range<usize>, work: &mut Work) -> Result<(), Error> {
        work.charge(
            range.len().checked_add(SENTINEL_BYTES).ok_or(Error::Budget)?,
            self.limits,
        )?;
        self.atom_input.clear();
        self.atom_input.push('a');
        self.atom_input.push_str(&self.source[range.clone()]);
        self.atom_input.push('z');
        self.atom_output.clear();
        let arena = comrak::Arena::new();
        work.parses = work.parses.checked_add(1).ok_or(Error::Budget)?;
        let root = comrak::parse_document(&arena, &self.atom_input, &crate::rendering::comrak_options());
        for node in root.descendants() {
            work.charge(1, self.limits)?;
            match &node.data.borrow().value {
                NodeValue::Document | NodeValue::Paragraph => {}
                NodeValue::Text(text) => {
                    if self.atom_output.len() + text.len() > self.source.len() * UTF8_BYTES + SENTINEL_BYTES {
                        return Err(Error::Budget);
                    }
                    self.atom_output.push_str(text);
                }
                _ => return Err(Error::UnsupportedAtom),
            }
        }
        let mut output = std::mem::take(&mut self.atom_output);
        let value = output
            .strip_prefix('a')
            .and_then(|s| s.strip_suffix('z'))
            .ok_or(Error::InvalidSource)?;
        self.emit(value, range, work)?;
        output.clear();
        self.atom_output = output;
        Ok(())
    }

    fn code(&mut self, range: Range<usize>, ticks: usize, expected: &str, work: &mut Work) -> Result<(), Error> {
        let delimiter_bytes = ticks.checked_mul(2).ok_or(Error::InvalidSource)?;
        if ticks == 0 || range.len() < delimiter_bytes {
            return Err(Error::InvalidSource);
        }
        let start = range.start + ticks;
        let end = range.end - ticks;
        if !self.source[range.start..start].bytes().all(|b| b == b'`')
            || !self.source[end..range.end].bytes().all(|b| b == b'`')
        {
            return Err(Error::InvalidSource);
        }
        let mut i = start;
        while i < end {
            let ch = self.source[i..end].chars().next().ok_or(Error::InvalidSource)?;
            let length = ch.len_utf8();
            // Comrak retains CR before LF; reconstruction must equal the actual leaf. ~keep
            if ch == '\n' || (ch == '\r' && self.source.as_bytes().get(i + 1) != Some(&b'\n')) {
                self.emit(" ", i..i + length, work)?;
            } else {
                self.emit(&self.source[i..i + length], i..i + length, work)?;
            }
            i += length;
        }
        if self.leaf.text != expected {
            if self.leaf.text.strip_prefix(' ').and_then(|s| s.strip_suffix(' ')) != Some(expected) {
                return Err(Error::InvalidSource);
            }
            work.charge(self.leaf.text.len(), self.limits)?;
            self.leaf.text.pop();
            self.leaf.text.remove(0);
            self.leaf.origins.pop();
            self.leaf.origins.remove(0);
            for origin in &mut self.leaf.origins {
                origin.decoded.start -= 1;
                origin.decoded.end -= 1;
            }
        }
        Ok(())
    }

    fn source_range(&self, node: &AstNode<'_>, work: &mut Work) -> Result<Range<usize>, Error> {
        work.charge(self.source.len().checked_mul(2).ok_or(Error::Budget)?, self.limits)?;
        let pos = node.data.borrow().sourcepos;
        let offset = |line: usize, column: usize| {
            let mut base = 0usize;
            for (index, text) in self.source.split_inclusive('\n').enumerate() {
                if index + 1 == line {
                    return base.checked_add(column);
                }
                base += text.len();
            }
            None
        };
        let start = offset(
            pos.start.line,
            pos.start.column.checked_sub(1).ok_or(Error::InvalidSource)?,
        )
        .ok_or(Error::InvalidSource)?;
        let end = offset(pos.end.line, pos.end.column).ok_or(Error::InvalidSource)?;
        self.source.get(start..end).ok_or(Error::InvalidSource)?;
        Ok(start..end)
    }
}

// This selects an actual parser leaf; it grants no grammar admission or whole-document coverage. ~keep
fn decode_leaf(source: &str, ordinal: usize, limits: Limits, work: &mut Work) -> Result<Leaf, Error> {
    let mut decoder = Decoder::prepare(source, limits, work)?;
    let arena = comrak::Arena::new();
    work.parses = work.parses.checked_add(1).ok_or(Error::Budget)?;
    let root = comrak::parse_document(&arena, source, &crate::rendering::comrak_options());
    let mut leaf_index = 0;
    for node in root.descendants() {
        work.charge(1, limits)?;
        let data = node.data.borrow();
        if !matches!(data.value, NodeValue::Text(_) | NodeValue::Code(_)) {
            continue;
        }
        if leaf_index != ordinal {
            leaf_index += 1;
            continue;
        }
        let range = decoder.source_range(node, work)?;
        match &data.value {
            NodeValue::Text(expected) => {
                decoder.text(range.clone(), work)?;
                if decoder.leaf.text.as_str() != expected.as_ref() {
                    return Err(Error::InvalidSource);
                }
            }
            NodeValue::Code(code) => decoder.code(range.clone(), code.num_backticks, &code.literal, work)?,
            _ => return Err(Error::InvalidSource),
        }
        decoder.leaf.source = range;
        return Ok(decoder.leaf);
    }
    Err(Error::MissingLeaf)
}

#[cfg(test)]
mod tests;
