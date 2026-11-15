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
    let mut claims_by_source = HashMap::<usize, HashMap<usize, usize>>::new();
    for (i, counterpart) in column_selection::unique_partners(old, new).into_iter().enumerate() {
        let Some(j) = counterpart else { continue };
        let (Some(a), Some(b)) = (source[i], destination[j]) else {
            continue;
        };
        // A unique geometric claim can veto a rank even when the retry misread the word.
        let votes = claims_by_source.entry(a).or_default().entry(b).or_default();
        let (area, before, after) = overlap(old[i].bounds, new[j].bounds);
        if old[i].text == new[j].text && area > before.max(after) / 2 {
            *votes += 1;
        }
    }
    let mut incoming = HashMap::<usize, usize>::new();
    for &rank in claims_by_source.values().flat_map(|claims| claims.keys()) {
        *incoming.entry(rank).or_default() += 1;
    }
    Some(
        claims_by_source
            .into_iter()
            .filter_map(|(i, claims)| {
                let (&rank, &count) = claims.iter().next()?;
                if claims.len() != 1 || count < 2 || incoming[&rank] != 1 {
                    return None;
                }
                let element = &elements[i];
                let bounds = bounds(element)?;
                (element.level == OcrElementLevel::Line).then(|| RankedLine {
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
        assert_competing_line_blocks_merged_retry_line("three");
    }

    #[test]
    fn a_mismatched_competing_literal_still_conflicts_with_a_two_vote_merged_retry_line() {
        assert_competing_line_blocks_merged_retry_line("tree");
    }

    fn assert_competing_line_blocks_merged_retry_line(competing_retry_literal: &str) {
        use crate::types::{ExtractedDocument, OcrConfidence, OcrElement, OcrElementLevel};

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
        let read = |words: &[Word<'_>], ranges: &[std::ops::Range<usize>]| {
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
        };
        let primary = read(&words, &[0..2, 2..3, 3..5]);
        let mut retry_words = words.clone();
        retry_words[2].text = competing_retry_literal;
        let merged = read(&retry_words, &[0..3, 3..5]);
        let unchanged = rank_lines(&primary, &primary, &words, &words).unwrap();
        assert_eq!(unchanged.len(), 2);
        let mut split_words = words.clone();
        split_words[0].text = "won";
        let split = read(&split_words, &[0..2, 2..3, 3..5]);
        assert_eq!(rank_lines(&primary, &split, &words, &split_words).unwrap().len(), 1);
        let actual = rank_lines(&primary, &merged, &words, &retry_words).unwrap();
        assert_eq!(
            actual.iter().map(|line| line.literal.as_str()).collect::<Vec<_>>(),
            ["separate line"]
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
