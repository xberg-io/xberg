use super::{Inventory, Slots, bounds, column_selection, literal_span, overlap};
use crate::types::{
    ExtractedDocument, OcrElement, OcrElementLevel,
    internal::{ElementKind, InternalDocument, InternalElement},
};
use std::{cmp::Reverse, ops::Range};

pub(in crate::extractors::pdf::ocr::column_retry) struct WholeLine {
    block: (usize, InternalElement),
    line: Option<(usize, OcrElement)>,
    words: (usize, Vec<OcrElement>),
}

pub(in crate::extractors::pdf::ocr::column_retry) fn plan(
    primary: &ExtractedDocument,
    retry: &ExtractedDocument,
    old: &Inventory<'_>,
    new: &Inventory<'_>,
    unmatched: &[bool],
) -> Option<Vec<WholeLine>> {
    let source = ReadLines::new(primary, old)?;
    let destination = ReadLines::new(retry, new)?;
    let partners = column_selection::unique_partners(&old.words, &new.words);
    let by_start: std::collections::HashMap<_, _> =
        destination.owned.iter().map(|line| (line.words.start, line)).collect();
    let height = column_selection::median_height(&old.words);
    let mut done = 0;
    let mut insertions = Vec::new();
    for line in &source.owned {
        if !unmatched[line.words.clone()].iter().all(|&missing| missing) {
            continue;
        }
        let above = source.neighbor(line, height, true)?;
        let below = source.neighbor(line, height, false)?;
        let map = |owned: &OwnedLine| {
            let first = partners[owned.words.start]?;
            let mapped = *by_start.get(&first)?;
            (owned.words.len() == mapped.words.len()
                && owned
                    .words
                    .clone()
                    .zip(mapped.words.clone())
                    .all(|(i, j)| partners[i] == Some(j) && old.words[i].text == new.words[j].text))
            .then_some(mapped)
        };
        let mapped_above = map(above)?;
        let mapped_below = map(below)?;
        insertions.push(destination.insertion(&source, line, mapped_above, mapped_below, height)?);
        done += line.words.len();
    }
    (done == unmatched.iter().filter(|&&missing| missing).count()).then_some(insertions)
}

struct OwnedLine {
    block: usize,
    line: usize,
    words: Range<usize>,
    bounds: [u32; 4],
}

struct ReadLines<'a> {
    blocks: &'a [InternalElement],
    public: &'a [OcrElement],
    word_positions: &'a [usize],
    owned: Vec<OwnedLine>,
}

impl<'a> ReadLines<'a> {
    fn new(read: &'a ExtractedDocument, inventory: &'a Inventory<'_>) -> Option<Self> {
        let blocks = &read.ocr_internal_document.as_ref()?.elements;
        let public = read.ocr_elements.as_deref()?;
        let slots = Slots::new(read, &inventory.words)?;
        let owned = slots
            .lines
            .chunk_by(|a, b| a.is_some() && a.as_ref().map(|s| s.element) == b.as_ref().map(|s| s.element))
            .scan(0, |start, chunk| {
                let range = *start..*start + chunk.len();
                *start = range.end;
                Some(range)
            })
            .filter_map(|range| owned_line(&slots, blocks, public, range))
            .collect();
        Some(Self {
            blocks,
            public,
            word_positions: &inventory.indexes,
            owned,
        })
    }

    fn neighbor(&self, target: &OwnedLine, height: f64, above: bool) -> Option<&OwnedLine> {
        let gap = |line: &OwnedLine| vertical_gap(line.bounds, target.bounds, above);
        let eligible = || {
            self.owned.iter().filter(|line| {
                same_column(line.bounds, target.bounds, height)
                    && gap(line).is_some_and(|distance| f64::from(distance) <= height * 3.0)
            })
        };
        let nearest = eligible().filter_map(gap).min()?;
        let mut tied = eligible().filter(|line| gap(line) == Some(nearest));
        let one = tied.next()?;
        if tied.next().is_some() || !self.clear_corridor(target, one, above) {
            return None;
        }
        Some(one)
    }

    fn clear_corridor(&self, target: &OwnedLine, neighbor: &OwnedLine, above: bool) -> bool {
        let (top, bottom) = if above {
            (neighbor.bounds[3], target.bounds[3])
        } else {
            (target.bounds[1], neighbor.bounds[1])
        };
        let corridor = [target.bounds[0], top, target.bounds[2], bottom];
        self.public.iter().enumerate().all(|(index, element)| {
            element.level != OcrElementLevel::Line
                || index == target.line
                || index == neighbor.line
                || bounds(element).is_some_and(|other| overlap(corridor, other).0 == 0)
        })
    }

    fn insertion(
        &self,
        source: &Self,
        target: &OwnedLine,
        above: &OwnedLine,
        below: &OwnedLine,
        height: f64,
    ) -> Option<WholeLine> {
        let block = &source.blocks[target.block];
        let source_line = &source.public[target.line];
        if above.block + 1 != below.block
            || above.words.end != below.words.start
            || above.line >= below.line
            || [above, below].into_iter().any(|line| {
                let anchor = &self.blocks[line.block];
                anchor.layer != block.layer
                    || anchor.page != block.page
                    || anchor.depth != block.depth
                    || self.public[line.line].page_number != source_line.page_number
                    || !same_column(line.bounds, target.bounds, height)
            })
            || target.bounds[1] < above.bounds[3]
            || target.bounds[3] > below.bounds[1]
            || self
                .public
                .iter()
                .any(|element| bounds(element).is_none_or(|other| overlap(target.bounds, other).0 != 0))
        {
            return None;
        }
        let mut words: Vec<_> = source.word_positions[target.words.clone()]
            .iter()
            .map(|&index| source.public[index].clone())
            .collect();
        if words
            .iter()
            .any(|word| !unlinked(word) || word.page_number != source_line.page_number)
        {
            return None;
        }
        let before_words = self.word_positions[below.words.start];
        let last_above = self.word_positions[above.words.end - 1];
        let (line, index) = if (last_above..before_words).contains(&below.line) {
            words.insert(0, source_line.clone());
            (None, below.line)
        } else if below.line < self.word_positions[0] {
            (Some((below.line, source_line.clone())), before_words)
        } else {
            return None;
        };
        Some(WholeLine {
            block: (below.block, block.clone()),
            line,
            words: (index, words),
        })
    }
}

fn owned_line(
    slots: &Slots,
    blocks: &[InternalElement],
    public: &[OcrElement],
    words: Range<usize>,
) -> Option<OwnedLine> {
    let (block, block_range) = literal_span(&slots.blocks, words.clone())?;
    let (line, line_range) = literal_span(&slots.lines, words.clone())?;
    let element = &blocks[block];
    let public_line = &public[line];
    let bounds = bounds(public_line)?;
    let bbox = super::bbox(bounds);
    if element.kind != ElementKind::Paragraph
        || !element.annotations.is_empty()
        || element.anchor.is_some()
        || !unlinked(public_line)
        || element.bbox != Some(bbox)
        || block_range != (0..element.text.len())
        || line_range != (0..public_line.text.len())
        || element.text != public_line.text
        || public_line.text.split_whitespace().count() != words.len()
    {
        return None;
    }
    Some(OwnedLine {
        block,
        line,
        words,
        bounds,
    })
}

fn unlinked(element: &OcrElement) -> bool {
    element.parent_id.is_none() && element.element_id().is_none()
}

fn same_column(a: [u32; 4], b: [u32; 4], height: f64) -> bool {
    let shared = a[2].min(b[2]).saturating_sub(a[0].max(b[0]));
    f64::from(a[0].abs_diff(b[0])) <= height && shared > (a[2] - a[0]).max(b[2] - b[0]) / 2
}

fn vertical_gap(a: [u32; 4], b: [u32; 4], above: bool) -> Option<u32> {
    if above {
        b[1].checked_sub(a[3])
    } else {
        a[1].checked_sub(b[3])
    }
}

pub(in crate::extractors::pdf::ocr::column_retry) fn apply(
    lines: Vec<WholeLine>,
    doc: &mut InternalDocument,
    public: &mut Vec<OcrElement>,
) {
    let mut blocks = Vec::new();
    let mut elements = Vec::new();
    for WholeLine { block, line, words } in lines {
        blocks.push(block);
        if let Some((index, line)) = line {
            elements.push((index, vec![line]));
        }
        elements.push(words);
    }
    blocks.sort_unstable_by_key(|(index, _)| Reverse(*index));
    elements.sort_unstable_by_key(|(index, _)| Reverse(*index));
    for (index, block) in blocks {
        doc.elements.insert(index, block);
    }
    for (index, values) in elements {
        drop(public.splice(index..index, values));
    }
}
