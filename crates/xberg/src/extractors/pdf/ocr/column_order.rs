use super::{
    MIN_GUTTER_JUMPS, Word, bounds, column_selection, document, interleaving, inventory, line_slots, overlap, pipeline,
    stop_retry, supported,
};
use crate::{
    cancellation::CancellationToken,
    pdf::structure::types::{LayoutHintClass, PdfLine, PdfParagraph},
    types::{ExtractedDocument, OcrElementLevel},
};
use std::collections::HashMap;

#[derive(Debug)]
struct RankedLine {
    literal: String,
    bounds: [u32; 4],
    rank: usize,
}

#[derive(Debug)]
pub(in crate::extractors::pdf::ocr) struct LineOrder {
    frame: (u32, u32),
    lines: Vec<RankedLine>,
}

pub(super) fn plan(
    primary: &ExtractedDocument,
    retry: &ExtractedDocument,
    backend: &str,
    frame: (u32, u32),
    cancel: Option<&CancellationToken>,
) -> Option<LineOrder> {
    if !supported(retry)
        || pipeline::effective_tesseract_psm(&retry.metadata, backend) != Some(3)
        || document::processed_ocr_layout_dimensions(&retry.metadata) != Some(frame)
        || document::resolved_ocr_correction_degrees(&retry.metadata).is_some_and(|angle| angle != 0)
        || serde_json::to_value(&primary.metadata.image_preprocessing).ok()?
            != serde_json::to_value(&retry.metadata.image_preprocessing).ok()?
    {
        return None;
    }
    let old = inventory(primary)?;
    let new = inventory(retry)?;
    if !retained_order_improves(&old.words, &new.words, frame) || stop_retry(cancel) {
        return None;
    }
    let lines = rank_lines(primary, retry, &old.words, &new.words)?;
    (lines.len() >= 2 && !stop_retry(cancel)).then_some(LineOrder { frame, lines })
}

fn retained_order_improves(old: &[Word<'_>], new: &[Word<'_>], frame: (u32, u32)) -> bool {
    if new.len() * 100 < old.len() * 92 || new.len() * 100 > old.len() * 112 {
        return false;
    }
    let shared: Vec<_> = old
        .iter()
        .copied()
        .filter(|word| {
            new.iter().any(|other| {
                let (area, a, b) = overlap(word.bounds, other.bounds);
                area > a.min(b) / 2
            })
        })
        .collect();
    let jumps = interleaving(&shared, frame);
    shared.len() * 100 >= old.len() * 92 && jumps >= MIN_GUTTER_JUMPS && interleaving(new, frame) * 100 <= jumps * 45
}

fn complete_line_owners(read: &ExtractedDocument, words: &[Word<'_>]) -> Option<Vec<Option<usize>>> {
    let elements = read.ocr_elements.as_ref()?;
    let slots = line_slots(read, words)?;
    let mut owners = vec![Vec::new(); elements.len()];
    for (word, slot) in slots.iter().enumerate() {
        if let Some(slot) = slot {
            owners[slot.element].push((word, slot.range.clone()));
        }
    }
    let mut result = vec![None; words.len()];
    for (index, owned) in owners.iter().enumerate() {
        let text = &elements[index].text;
        if owned.is_empty()
            || !owned
                .windows(2)
                .all(|pair| pair[0].0 + 1 == pair[1].0 && pair[0].1.end < pair[1].1.start)
            || !text.split_whitespace().eq(owned.iter().map(|&(i, _)| words[i].text))
        {
            continue;
        }
        for &(word, _) in owned {
            result[word] = Some(index);
        }
    }
    Some(result)
}

fn rank_lines(
    primary: &ExtractedDocument,
    retry: &ExtractedDocument,
    old: &[Word<'_>],
    new: &[Word<'_>],
) -> Option<Vec<RankedLine>> {
    let source = complete_line_owners(primary, old)?;
    let destination = complete_line_owners(retry, new)?;
    let elements = primary.ocr_elements.as_ref()?;
    let mut votes = HashMap::<usize, HashMap<usize, usize>>::new();
    for (i, counterpart) in column_selection::unique_partners(old, new).into_iter().enumerate() {
        let Some(j) = counterpart else { continue };
        let (Some(a), Some(b)) = (source[i], destination[j]) else {
            continue;
        };
        let (area, before, after) = overlap(old[i].bounds, new[j].bounds);
        if old[i].text == new[j].text && area > before.max(after) / 2 {
            *votes.entry(a).or_default().entry(b).or_default() += 1;
        }
    }
    let unique: Vec<_> = votes
        .into_iter()
        .filter_map(|(index, ranks)| {
            let (&rank, &count) = ranks.iter().next()?;
            (ranks.len() == 1 && count >= 2).then_some((index, rank))
        })
        .collect();
    let mut incoming = HashMap::<usize, usize>::new();
    for &(_, rank) in &unique {
        *incoming.entry(rank).or_default() += 1;
    }
    Some(
        unique
            .into_iter()
            .filter_map(|(i, rank)| {
                let element = &elements[i];
                let bounds = bounds(element)?;
                (incoming[&rank] == 1 && element.level == OcrElementLevel::Line).then(|| RankedLine {
                    literal: element.text.clone(),
                    bounds,
                    rank,
                })
            })
            .collect(),
    )
}

impl LineOrder {
    pub(in crate::extractors::pdf::ocr) fn reorder(
        &self,
        paragraphs: &[PdfParagraph],
        metadata: &crate::types::Metadata,
        frame: (u32, u32),
    ) -> Option<Vec<PdfParagraph>> {
        if self.frame != frame || pipeline::effective_tesseract_psm(metadata, "tesseract") != Some(11) {
            return None;
        }
        let keys: Vec<_> = paragraphs
            .iter()
            .map(|paragraph| self.paragraph_rank(paragraph))
            .collect();
        let indices = paragraph_order(&keys)?;
        tracing::debug!(
            moved = indices.iter().enumerate().filter(|(i, j)| *i != **j).count(),
            "Reordered primary OCR paragraphs using column order"
        );
        Some(indices.into_iter().map(|index| paragraphs[index].clone()).collect())
    }

    fn paragraph_rank(&self, paragraph: &PdfParagraph) -> Option<usize> {
        let (first, lines) = paragraph.lines.split_first()?;
        if paragraph.is_page_furniture
            || paragraph.is_list_item
            || paragraph.is_code_block
            || paragraph.is_formula
            || paragraph.heading_level.is_some()
            || paragraph.caption_for.is_some()
            || !matches!(paragraph.layout_class, None | Some(LayoutHintClass::Text))
            || paragraph.layout_region_path.is_some()
        {
            return None;
        }
        let rank = self.line_rank(first)?;
        if !lines.is_empty() {
            let following = lines.iter().map(|line| self.line_rank(line));
            if !following.eq((rank + 1..).map(Some).take(lines.len())) {
                return None;
            }
        }
        Some(rank)
    }

    fn line_rank(&self, line: &PdfLine) -> Option<usize> {
        let [segment] = line.segments.as_slice() else {
            return None;
        };
        let actual = [
            segment.x,
            self.frame.1 as f32 - segment.y - segment.height,
            segment.x + segment.width,
            self.frame.1 as f32 - segment.y,
        ];
        let mut matches = self.lines.iter().filter(|witness| {
            witness.literal.trim() == segment.text.trim()
                && witness
                    .bounds
                    .into_iter()
                    .zip(actual)
                    .all(|(a, b)| (a as f32 - b).abs() <= 1.0)
        });
        let rank = matches.next()?.rank;
        matches.next().is_none().then_some(rank)
    }
}

fn paragraph_order(keys: &[Option<usize>]) -> Option<Vec<usize>> {
    let mut counts = HashMap::<usize, usize>::new();
    for rank in keys.iter().flatten() {
        *counts.entry(*rank).or_default() += 1;
    }
    let mut indices: Vec<_> = (0..keys.len()).collect();
    let mut start = 0;
    for (end, rank) in keys.iter().copied().chain([None]).enumerate() {
        if rank.is_some_and(|key| counts[&key] == 1) {
            continue;
        }
        indices[start..end].sort_by_key(|&index| keys[index]);
        start = end + 1;
    }
    indices.iter().copied().ne(0..keys.len()).then_some(indices)
}

#[cfg(test)]
mod tests {
    use super::paragraph_order;

    #[test]
    fn reorders_only_between_structure_barriers() {
        let keys = [Some(4), Some(3), None, Some(2), Some(1), None, Some(0)];
        assert_eq!(paragraph_order(&keys), Some(vec![1, 0, 2, 4, 3, 5, 6]));
    }

    #[test]
    fn duplicate_order_witnesses_are_barriers() {
        let keys = [Some(4), Some(3), Some(4), Some(2), Some(1)];
        assert_eq!(paragraph_order(&keys), Some(vec![0, 1, 2, 4, 3]));
        assert_eq!(paragraph_order(&[Some(1), None, Some(0)]), None);
    }
}
