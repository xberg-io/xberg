use super::{
    MIN_GUTTER_JUMPS, Word, bounds, column_selection, document, interleaving, inventory, line_slots, overlap, pipeline,
    stop_retry, supported,
};
use crate::{
    cancellation::CancellationToken,
    pdf::structure::types::{LayoutHintClass, PdfLine, PdfParagraph},
    types::ExtractedDocument,
};
use std::{collections::HashMap, ops::Range};

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

struct LineOwners {
    by_word: Vec<Option<usize>>,
    spans: Vec<Option<Range<usize>>>,
}

fn complete_line_owners(read: &ExtractedDocument, words: &[Word<'_>]) -> Option<LineOwners> {
    let elements = read.ocr_elements.as_ref()?;
    let slots = line_slots(read, words)?;
    let mut owners = vec![Vec::new(); elements.len()];
    for (word, slot) in slots.iter().enumerate() {
        if let Some(slot) = slot {
            owners[slot.element].push((word, slot.range.clone()));
        }
    }
    let mut by_word = vec![None; words.len()];
    let mut spans = vec![None; elements.len()];
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
        let first = owned[0].0;
        let span = first..first + owned.len();
        by_word[span.clone()].fill(Some(index));
        spans[index] = Some(span);
    }
    Some(LineOwners { by_word, spans })
}

#[derive(Clone, Default)]
struct LineClaims {
    owners: [Option<usize>; 3],
    unresolved: bool,
}

impl LineClaims {
    fn record(&mut self, line: usize) {
        if !self.owners.contains(&Some(line))
            && let Some(slot) = self.owners.iter_mut().find(|slot| slot.is_none())
        {
            *slot = Some(line);
        }
    }

    fn unique(&self) -> Option<usize> {
        match (self.unresolved, self.owners) {
            (false, [Some(line), None, None]) => Some(line),
            _ => None,
        }
    }
}

struct OrderEvidence<'a, 'text> {
    old: &'a [Word<'text>],
    new: &'a [Word<'text>],
    source: LineOwners,
    destination: LineOwners,
    partners: Vec<Option<usize>>,
    outgoing: Vec<LineClaims>,
    incoming: Vec<LineClaims>,
    votes: Vec<usize>,
}

impl<'a, 'text> OrderEvidence<'a, 'text> {
    fn new(old: &'a [Word<'text>], new: &'a [Word<'text>], source: LineOwners, destination: LineOwners) -> Self {
        // Three distinct claims suffice to reject a rank and keep the witness bounded.
        let mut outgoing = vec![LineClaims::default(); source.spans.len()];
        let mut incoming = vec![LineClaims::default(); destination.spans.len()];
        let partners =
            column_selection::geometric_partners(old, new, |i, j| match (source.by_word[i], destination.by_word[j]) {
                (Some(a), Some(b)) => {
                    outgoing[a].record(b);
                    incoming[b].record(a);
                }
                (Some(a), None) => outgoing[a].unresolved = true,
                (None, Some(b)) => incoming[b].unresolved = true,
                (None, None) => {}
            });
        let mut votes = vec![0; source.spans.len()];
        for (i, &partner) in partners.iter().enumerate() {
            let Some(j) = partner else { continue };
            if let (Some(a), Some(b)) = (source.by_word[i], destination.by_word[j])
                && outgoing[a].unique() == Some(b)
                && old[i].text == new[j].text
                && strong_overlap(old[i], new[j])
            {
                votes[a] += 1;
            }
        }
        Self {
            old,
            new,
            source,
            destination,
            partners,
            outgoing,
            incoming,
            votes,
        }
    }

    fn exact_suffix(&self, line: usize, rank: usize, prefix: usize, votes: usize) -> Option<()> {
        let suffix = self.source.spans[line].as_ref()?;
        let first_word = self.source.spans[prefix].as_ref()?;
        let target = self.destination.spans[rank].as_ref()?;
        if votes != suffix.len()
            || target.len() != suffix.len() + 1
            || first_word.len() != 1
            || first_word.end != suffix.start
            || self.outgoing[prefix].unique() != Some(rank)
            || self.partners[first_word.start] != Some(target.start)
            || !self.partners[suffix.clone()]
                .iter()
                .copied()
                .eq((target.start + 1..target.end).map(Some))
        {
            return None;
        }
        let primary_prefix = self.old[first_word.start];
        let retry_prefix = self.new[target.start];
        let suffix_first = self.old[suffix.start];
        let extension = retry_prefix.text.strip_prefix(primary_prefix.text)?;
        let [_, top, right, bottom] = primary_prefix.bounds;
        let [left, suffix_top, _, suffix_bottom] = suffix_first.bounds;
        let vertical_overlap = bottom.min(suffix_bottom).saturating_sub(top.max(suffix_top));
        // The prefix retains its own primary rank barrier. Only the disjoint exact suffix supplies votes.
        (extension.len() == 1
            && extension.as_bytes()[0].is_ascii_punctuation()
            && strong_overlap(primary_prefix, retry_prefix)
            && right <= left
            && vertical_overlap > (bottom - top).max(suffix_bottom - suffix_top) / 2)
            .then_some(())
    }

    fn rank(&self, line: usize) -> Option<usize> {
        let rank = self.outgoing[line].unique()?;
        let votes = self.votes[line];
        let incoming = &self.incoming[rank];
        if votes < 2 || incoming.unresolved {
            return None;
        }
        match incoming.owners {
            [Some(_), None, None] => Some(rank),
            [Some(a), Some(b), None] => {
                let prefix = if a == line { b } else { a };
                self.exact_suffix(line, rank, prefix, votes)?;
                Some(rank)
            }
            _ => None,
        }
    }
}

fn strong_overlap(old: Word<'_>, new: Word<'_>) -> bool {
    let (area, before, after) = overlap(old.bounds, new.bounds);
    area > before.max(after) / 2
}

fn rank_lines(
    primary: &ExtractedDocument,
    retry: &ExtractedDocument,
    old: &[Word<'_>],
    new: &[Word<'_>],
) -> Option<Vec<RankedLine>> {
    let source = complete_line_owners(primary, old)?;
    let destination = complete_line_owners(retry, new)?;
    let evidence = OrderEvidence::new(old, new, source, destination);
    let elements = primary.ocr_elements.as_ref()?;
    Some(
        (0..elements.len())
            .filter_map(|line| {
                let rank = evidence.rank(line)?;
                let element = &elements[line];
                let bounds = bounds(element)?;
                Some(RankedLine {
                    literal: element.text.clone(),
                    bounds,
                    rank,
                })
            })
            .collect(),
    )
}

impl LineOrder {
    pub(in crate::extractors::pdf::ocr) fn before_grouping(
        &self,
        lines: &[PdfParagraph],
        metadata: &crate::types::Metadata,
        frame: (u32, u32),
    ) -> Option<Vec<usize>> {
        if self.frame != frame || pipeline::effective_tesseract_psm(metadata, "tesseract") != Some(11) {
            return None;
        }
        let keys: Vec<_> = lines.iter().map(|line| self.paragraph_rank(line)).collect();
        let order = paragraph_order(&keys)?;
        tracing::debug!(
            moved = order.iter().enumerate().filter(|(i, j)| *i != **j).count(),
            "Reordered primary OCR paragraphs using column order before grouping"
        );
        Some(order)
    }

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
    use super::{LineOrder, RankedLine, Word, paragraph_order, rank_lines};

    #[test]
    fn a_one_vote_source_still_conflicts_with_a_two_vote_merged_retry_line() {
        assert_competing_line_blocks_merged_retry_line(&["three"]);
    }

    #[test]
    fn a_mismatched_competing_literal_still_conflicts_with_a_two_vote_merged_retry_line() {
        assert_competing_line_blocks_merged_retry_line(&["tree"]);
    }

    #[test]
    fn split_retry_words_still_veto_a_merged_retry_line() {
        assert_competing_line_blocks_merged_retry_line(&["th", "ree"]);
    }

    #[test]
    fn a_word_crossing_its_line_box_vetoes_a_rank_on_either_read() {
        let words = [
            ("one", [0, 0, 12, 10]),
            ("two", [20, 0, 32, 10]),
            ("three", [40, 0, 52, 10]),
            ("separate", [0, 30, 12, 40]),
            ("line", [20, 30, 32, 40]),
        ]
        .map(|(text, bounds)| Word {
            text,
            bounds,
            confidence: 1.0,
        });
        let intact = line_read(&words, &[0..3, 3..5]);
        let mut clipped = line_read(&words, &[0..2, 2..3, 3..5]);
        clipped.ocr_elements.as_mut().unwrap()[1].geometry = super::super::rectangle([41, 0, 52, 10]);

        for (primary, retry) in [(&clipped, &intact), (&intact, &clipped)] {
            let ranked = rank_lines(primary, retry, &words, &words).unwrap();
            assert_eq!(
                ranked.iter().map(|line| line.literal.as_str()).collect::<Vec<_>>(),
                ["separate line"]
            );
        }
    }

    fn assert_competing_line_blocks_merged_retry_line(competing_retry_literals: &[&str]) {
        let words: Vec<_> = ["one", "two", "three", "separate", "line"]
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                let left = (i % 3) as u32 * 20;
                let top = (i / 3) as u32 * 30;
                Word {
                    text,
                    bounds: [left, top, left + 12, top + 10],
                    confidence: 1.0,
                }
            })
            .collect();
        let primary = line_read(&words, &[0..2, 2..3, 3..5]);
        let mut retry_words = words[..2].to_vec();
        let [left, top, right, bottom] = words[2].bounds;
        let fragments = competing_retry_literals.len() as u32;
        for (index, &text) in competing_retry_literals.iter().enumerate() {
            let index = index as u32;
            retry_words.push(Word {
                text,
                bounds: [
                    left + (right - left) * index / fragments,
                    top,
                    left + (right - left) * (index + 1) / fragments,
                    bottom,
                ],
                confidence: 1.0,
            });
        }
        let end = retry_words.len();
        retry_words.extend_from_slice(&words[3..]);
        let merged = line_read(&retry_words, &[0..end, end..retry_words.len()]);
        let unchanged = rank_lines(&primary, &primary, &words, &words).unwrap();
        assert_eq!(unchanged.len(), 2);
        let mut split_words = words.clone();
        split_words[0].text = "won";
        let split = line_read(&split_words, &[0..2, 2..3, 3..5]);
        assert_eq!(rank_lines(&primary, &split, &words, &split_words).unwrap().len(), 1);
        let actual = rank_lines(&primary, &merged, &words, &retry_words).unwrap();
        assert_eq!(
            actual.iter().map(|line| line.literal.as_str()).collect::<Vec<_>>(),
            ["separate line"]
        );
    }

    fn line_read(words: &[Word<'_>], ranges: &[std::ops::Range<usize>]) -> crate::types::ExtractedDocument {
        use crate::types::{ExtractedDocument, OcrConfidence, OcrElement, OcrElementLevel};
        let mut elements = Vec::new();
        for range in ranges {
            let first = words[range.start].bounds;
            let last = words[range.end - 1].bounds;
            let text = words[range.clone()]
                .iter()
                .map(|word| word.text)
                .collect::<Vec<_>>()
                .join(" ");
            elements.push(
                OcrElement::new(
                    text,
                    super::super::rectangle([first[0], first[1], last[2], last[3]]),
                    OcrConfidence::from_tesseract(100.0),
                )
                .with_level(OcrElementLevel::Line),
            );
        }
        ExtractedDocument {
            ocr_elements: Some(elements),
            ..Default::default()
        }
    }

    #[test]
    fn ranks_an_exact_suffix_after_a_separately_owned_punctuation_prefix() {
        let words: Vec<_> = ["one", "two", "three"]
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                let x = i as u32 * 20;
                Word {
                    text,
                    bounds: [x, 0, x + 12, 10],
                    confidence: 1.0,
                }
            })
            .collect();
        let primary = line_read(&words, &[0..1, 1..3]);
        let mut retry = words.clone();
        retry[0].text = "one.";
        let merged = line_read(&retry, &[0..3]);
        let actual = rank_lines(&primary, &merged, &words, &retry).unwrap();
        assert_eq!(
            actual.iter().map(|line| line.literal.as_str()).collect::<Vec<_>>(),
            ["two three"]
        );
    }

    #[test]
    fn binds_primary_lines_before_grouping_and_leaves_the_header_in_place() {
        use crate::pdf::structure::{
            adapters::{OcrFontSizeScale, ocr_doc_to_layout_paragraphs_ordered},
            types::{LayoutHint, LayoutHintClass},
        };
        use crate::types::{
            OcrElementLevel,
            internal::{ElementKind, InternalDocument, InternalElement},
        };
        let rows = [
            ("First left", [10, 60, 110, 80]),
            ("First right", [230, 60, 330, 80]),
            ("Second left", [10, 80, 110, 100]),
            ("Header", [10, 20, 330, 40]),
            ("Second right", [230, 80, 330, 100]),
            ("Third right", [230, 100, 330, 120]),
        ];
        let mut doc = InternalDocument::new("test");
        for &(text, bounds) in &rows {
            let mut element = InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Line,
                },
                text,
                0,
            );
            element.bbox = Some(super::super::bbox(bounds));
            doc.push_element(element);
        }
        let ranks = [0, 2, 1, 3, 5, 4];
        let order = LineOrder {
            frame: super::super::tests::FRAME,
            lines: rows
                .iter()
                .zip(ranks)
                .map(|(&(text, bounds), rank)| RankedLine {
                    literal: text.into(),
                    bounds,
                    rank,
                })
                .collect(),
        };
        let read = super::super::tests::page_read(11, "", &[]);
        let mut rank = |lines: &[crate::pdf::structure::types::PdfParagraph]| {
            order.before_grouping(lines, &read.metadata, super::super::tests::FRAME)
        };
        let result = ocr_doc_to_layout_paragraphs_ordered(
            &doc,
            500,
            &[LayoutHint {
                class_name: LayoutHintClass::PageHeader,
                confidence: 1.0,
                left: 10.0,
                bottom: 460.0,
                right: 330.0,
                top: 480.0,
            }],
            0.5,
            0.2,
            OcrFontSizeScale::uniform(1.0),
            Some(&mut rank),
        );
        let text: Vec<_> = result.iter().flat_map(|para| para.text.lines()).collect();
        assert_eq!(
            text,
            [
                "First left",
                "Second left",
                "First right",
                "Header",
                "Third right",
                "Second right"
            ]
        );
        assert_ordered_layout_retains_furniture(result);
    }

    fn assert_ordered_layout_retains_furniture(paragraphs: Vec<crate::pdf::structure::types::PdfParagraph>) {
        use crate::types::{BoundingBox, ContentLayer};

        let mut original = paragraphs.clone();
        let config = crate::core::config::ExtractionConfig::default();
        assert!(!super::super::pipeline::original_ocr_layout_blocks_reconstruction(
            &mut original,
            &config
        ));
        original
            .iter_mut()
            .find(|p| p.text.contains("First left"))
            .unwrap()
            .is_list_item = true;
        let blocked = super::super::pipeline::original_ocr_layout_blocks_reconstruction(&mut original, &config);
        assert!(blocked);
        assert!(super::super::pipeline::ocr_heuristic_input(&paragraphs, blocked).is_empty());
        assert_eq!(
            super::super::pipeline::ocr_heuristic_input(&paragraphs, false).len(),
            paragraphs.len()
        );
        let doc =
            crate::pdf::structure::assemble_internal_document(vec![paragraphs], &[], None, &[], &Default::default());
        let doc = super::super::document::build_pipeline_ocr_page_document(
            Some(doc),
            Vec::new(),
            Vec::new(),
            "",
            1,
            (1000, 500),
            (500.0, 250.0),
        )
        .unwrap();
        let header = doc.elements.iter().find(|el| el.text == "Header").unwrap();
        assert_eq!(header.layer, ContentLayer::Header);
        assert_eq!(
            header.bbox,
            Some(BoundingBox {
                x0: 5.0,
                y0: 230.0,
                x1: 165.0,
                y1: 240.0
            })
        );
    }

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
