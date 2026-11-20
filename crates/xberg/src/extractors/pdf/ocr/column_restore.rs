use super::{
    Inventory, Slot, Word, bbox, bounds, column_selection, document_slots, line_slots, overlap, rectangle, union,
};
use crate::types::{ExtractedDocument, OcrElement, internal::InternalDocument};
use std::{cmp::Reverse, ops::Range};

#[path = "column_restore_whole.rs"]
mod whole;
pub(super) use whole::{WholeLine, apply as apply_whole, plan as plan_whole};

pub(super) struct Span {
    pub(super) block: (Slot, String),
    pub(super) line: (Slot, String),
    index: usize,
    words: Vec<OcrElement>,
    bounds: [u32; 4],
}

pub(super) fn plan(
    primary: &ExtractedDocument,
    retry: &ExtractedDocument,
    old: &Inventory<'_>,
    new: &Inventory<'_>,
    unmatched: &[bool],
) -> Option<Vec<Span>> {
    let source = Slots::new(primary, &old.words)?;
    let destination = Slots::new(retry, &new.words)?;
    let partners = column_selection::unique_partners(&old.words, &new.words);
    let height = column_selection::median_height(&old.words);
    let mut result = Vec::new();
    let mut start = 0;
    while let Some(offset) = unmatched[start..].iter().position(|&missing| missing) {
        start += offset;
        let end = unmatched[start..]
            .iter()
            .position(|&missing| !missing)
            .map_or(unmatched.len(), |n| start + n);
        let left = start.checked_sub(1)?;
        let before = partners[left]?;
        let after = *partners.get(end)?.as_ref()?;
        let row = &old.words[left..=end];
        if before + 1 != after
            || row.first()?.text != new.words[before].text
            || row.last()?.text != new.words[after].text
            || !row
                .windows(2)
                .all(|pair| column_selection::same_line(pair[0], pair[1], height))
            || !fits_gap(&old.words[start..end], &new.words[before..=after], height)
            || !source.owns_span(primary, left..end + 1, row)
        {
            return None;
        }
        result.push(destination.insertion(primary, retry, old, new, start..end, before..after + 1)?);
        start = end;
    }
    Some(result)
}

fn fits_gap(words: &[Word<'_>], anchors: &[Word<'_>], height: f64) -> bool {
    column_selection::same_line(anchors[0], words[0], height)
        && column_selection::same_line(words[words.len() - 1], anchors[1], height)
}

struct Slots {
    blocks: Vec<Option<Slot>>,
    lines: Vec<Option<Slot>>,
}

impl Slots {
    fn new(read: &ExtractedDocument, words: &[Word<'_>]) -> Option<Self> {
        Some(Self {
            blocks: document_slots(read, words)?,
            lines: line_slots(read, words)?,
        })
    }

    fn owns_span(&self, read: &ExtractedDocument, span: Range<usize>, words: &[Word<'_>]) -> bool {
        let Some((block, block_range)) = literal_span(&self.blocks, span.clone()) else {
            return false;
        };
        let Some((line, line_range)) = literal_span(&self.lines, span) else {
            return false;
        };
        let block = &read.ocr_internal_document.as_ref().expect("validated slots").elements[block];
        let line = &read.ocr_elements.as_ref().expect("validated slots")[line];
        block.annotations.is_empty()
            && block.anchor.is_none()
            && literal_text(&block.text[block_range], words)
            && literal_text(&line.text[line_range], words)
    }

    fn insertion(
        &self,
        primary: &ExtractedDocument,
        retry: &ExtractedDocument,
        old: &Inventory<'_>,
        new: &Inventory<'_>,
        missing: Range<usize>,
        anchors: Range<usize>,
    ) -> Option<Span> {
        if !self.owns_span(retry, anchors.clone(), &new.words[anchors.clone()])
            || old.words[missing.clone()].iter().any(|source| {
                new.words
                    .iter()
                    .any(|candidate| overlap(source.bounds, candidate.bounds).0 != 0)
            })
        {
            return None;
        }
        let insertion = |slots: &[Option<Slot>]| {
            let right = slots[anchors.end - 1].as_ref()?;
            Some(Slot {
                element: right.element,
                range: right.range.start..right.range.start,
            })
        };
        let public = primary.ocr_elements.as_ref()?;
        let words: Vec<_> = old.indexes[missing]
            .iter()
            .map(|&index| public[index].clone())
            .collect();
        let text = words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            + " ";
        let destination = &retry.ocr_elements.as_ref()?[self.lines[anchors.start].as_ref()?.element];
        if destination.element_id().is_some()
            || destination.parent_id.is_some()
            || words.iter().any(|word| {
                word.element_id().is_some() || word.parent_id.is_some() || word.page_number != destination.page_number
            })
        {
            return None;
        }
        let span_bounds = union(words.iter().filter_map(bounds))?;
        Some(Span {
            block: (insertion(&self.blocks)?, text.clone()),
            line: (insertion(&self.lines)?, text),
            index: new.indexes[anchors.end - 1],
            words,
            bounds: span_bounds,
        })
    }
}

fn literal_span(slots: &[Option<Slot>], span: Range<usize>) -> Option<(usize, Range<usize>)> {
    let first = slots[span.start].as_ref()?;
    let mut end = first.range.start;
    for slot in slots.get(span)? {
        let slot = slot.as_ref()?;
        if slot.element != first.element || slot.range.start < end {
            return None;
        }
        end = slot.range.end;
    }
    Some((first.element, first.range.start..end))
}

fn literal_text(text: &str, words: &[Word<'_>]) -> bool {
    !text.contains(['\n', '\r']) && text.split_whitespace().eq(words.iter().map(|word| word.text))
}

pub(super) fn apply(mut spans: Vec<Span>, doc: &mut InternalDocument, elements: &mut Vec<OcrElement>) {
    spans.sort_unstable_by_key(|span| Reverse(span.index));
    for span in &spans {
        let block = &mut doc.elements[span.block.0.element];
        let merged = block.bbox.as_ref().map(|current| {
            let added = bbox(span.bounds);
            crate::types::BoundingBox {
                x0: current.x0.min(added.x0),
                y0: current.y0.min(added.y0),
                x1: current.x1.max(added.x1),
                y1: current.y1.max(added.y1),
            }
        });
        // The two proven anchors already bound the insertion horizontally.
        // Expand the block for any ascenders or descenders from the source.
        if let Some(merged) = merged {
            block.bbox = Some(merged);
        }
        if let Some(crate::types::OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        }) = block.ocr_geometry.as_ref()
        {
            let existing = [*left, *top, left.saturating_add(*width), top.saturating_add(*height)];
            block.ocr_geometry = union([existing, span.bounds].into_iter()).map(rectangle);
        }
        block.ocr_confidence = None;
        if let Some(attributes) = block.attributes.as_mut() {
            attributes.remove(crate::ocr::hocr_parser::HOCR_LINE_FONT_SIZES_ATTRIBUTE);
            attributes.remove(crate::ocr::hocr_parser::HOCR_LINE_X_HEIGHTS_ATTRIBUTE);
        }
        let line = &mut elements[span.line.0.element];
        if let Some(merged) = union(bounds(line).into_iter().chain([span.bounds])) {
            line.geometry = rectangle(merged);
        }
    }
    for span in spans {
        drop(elements.splice(span.index..span.index, span.words));
    }
}
