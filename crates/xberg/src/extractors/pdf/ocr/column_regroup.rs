//! Regroup a sparse read's words into the column order an automatic-segmentation read witnessed.
//!
//! A sparse (PSM 11) read of a two-column page joins words from both columns into one line,
//! which no paragraph reorder can separate. Here every primary word is assigned to the retry
//! line its box falls in, each primary line is split where its words change retry line, and
//! the pieces are ordered by retry line. Only the primary read's own words are kept, so the
//! token set (and text F1) is unchanged; only their order and line grouping move.

use super::{MIN_GUTTER_JUMPS, Word, interleaving, inventory, overlap, rectangle, slots, supported};
use super::{document, pipeline};
use crate::types::{BoundingBox, ExtractedDocument, OcrElementLevel};

/// One piece of a primary line: the words (by inventory index, in text order) that fall in a
/// single retry line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Piece {
    pub(super) element: usize,
    pub(super) rank: usize,
    pub(super) words: Vec<usize>,
}

/// Rewrites `primary`'s structured OCR document (and its flat text, when that text is the
/// document's serialization) into the column order `retry` witnessed. Returns whether it did;
/// `false` leaves `primary` untouched.
pub(super) fn regroup(
    primary: &mut ExtractedDocument,
    retry: &ExtractedDocument,
    backend: &str,
    frame: (u32, u32),
) -> bool {
    if !comparable(primary, retry, backend, frame) {
        return false;
    }
    let Some(plan) = plan(primary, retry, frame) else {
        return false;
    };
    apply(primary, &plan);
    tracing::debug!(
        pieces = plan.pieces.len(),
        "Regrouped sparse OCR words into witnessed column order"
    );
    true
}

struct Plan {
    pieces: Vec<Piece>,
    texts: Vec<String>,
    bounds: Vec<[u32; 4]>,
}

fn comparable(primary: &ExtractedDocument, retry: &ExtractedDocument, backend: &str, frame: (u32, u32)) -> bool {
    supported(primary)
        && supported(retry)
        && pipeline::effective_tesseract_psm(&primary.metadata, backend) == Some(11)
        && pipeline::effective_tesseract_psm(&retry.metadata, backend) == Some(3)
        && document::processed_ocr_layout_dimensions(&retry.metadata) == Some(frame)
        && document::resolved_ocr_correction_degrees(&retry.metadata).is_none_or(|angle| angle == 0)
        && serde_json::to_value(&primary.metadata.image_preprocessing).ok()
            == serde_json::to_value(&retry.metadata.image_preprocessing).ok()
}

fn plan(primary: &ExtractedDocument, retry: &ExtractedDocument, frame: (u32, u32)) -> Option<Plan> {
    let doc = primary.ocr_internal_document.as_ref()?;
    if doc
        .elements
        .iter()
        .any(|element| element.text.contains('\n') || element.bbox.is_none())
    {
        return None;
    }
    let words = inventory(primary)?.words;
    let owners = slots(
        doc.elements
            .iter()
            .enumerate()
            .filter_map(|(i, element)| element.bbox.map(|bounds| (i, element.text.as_str(), bounds))),
        &words,
    );
    let mut by_element = vec![Vec::new(); doc.elements.len()];
    for (word, slot) in owners.iter().enumerate() {
        let slot = slot.as_ref()?;
        by_element[slot.element].push((slot.range.start, word));
    }
    for (element, owned) in doc.elements.iter().zip(&mut by_element) {
        owned.sort_unstable();
        if owned.len() != element.text.split_whitespace().count() {
            return None;
        }
    }
    let witness: Vec<[u32; 4]> = retry
        .ocr_elements
        .as_deref()?
        .iter()
        .filter(|element| element.level == OcrElementLevel::Line)
        .map(super::bounds)
        .collect::<Option<_>>()?;
    let ranks = word_ranks(&words, &witness);
    let element_words: Vec<Vec<usize>> = by_element
        .into_iter()
        .map(|owned| owned.into_iter().map(|(_, word)| word).collect())
        .collect();
    let pieces = pieces(&element_words, &ranks, &words);
    let reordered: Vec<Word<'_>> = pieces
        .iter()
        .flat_map(|piece| piece.words.iter().map(|&word| words[word]))
        .collect();
    let jumps = interleaving(&words, frame);
    if jumps < MIN_GUTTER_JUMPS || interleaving(&reordered, frame) * 100 > jumps * 45 {
        return None;
    }
    let texts = pieces
        .iter()
        .map(|piece| {
            piece
                .words
                .iter()
                .map(|&word| words[word].text)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    let bounds = pieces
        .iter()
        .map(|piece| union_bounds(piece.words.iter().map(|&word| words[word].bounds)))
        .collect::<Option<_>>()?;
    Some(Plan { pieces, texts, bounds })
}

/// The retry line each word falls in (more than half its area inside), or, for a word no line
/// holds, the line of the word before it in the same primary order (`None` when none precedes).
pub(super) fn word_ranks(words: &[Word<'_>], witness: &[[u32; 4]]) -> Vec<Option<usize>> {
    words
        .iter()
        .map(|word| {
            let (best, area) = witness
                .iter()
                .enumerate()
                .map(|(rank, line)| (rank, overlap(word.bounds, *line).0))
                .max_by_key(|&(rank, area)| (area, std::cmp::Reverse(rank)))?;
            let (_, own, _) = overlap(word.bounds, word.bounds);
            (area * 2 > own).then_some(best)
        })
        .collect()
}

/// Split each primary line where its words change retry line and order the pieces by retry
/// line, then by left edge, then by primary position. A word no retry line holds stays with
/// the piece before it; a line with no held word follows the piece before it.
pub(super) fn pieces(element_words: &[Vec<usize>], ranks: &[Option<usize>], words: &[Word<'_>]) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::new();
    let mut previous_rank = 0;
    for (element, owned) in element_words.iter().enumerate() {
        let mut current: Option<Piece> = None;
        for &word in owned {
            let rank = ranks[word]
                .or(current.as_ref().map(|piece| piece.rank))
                .or_else(|| owned.iter().find_map(|&other| ranks[other]))
                .unwrap_or(previous_rank);
            match current.as_mut() {
                Some(piece) if piece.rank == rank => piece.words.push(word),
                _ => {
                    pieces.extend(current.take());
                    current = Some(Piece {
                        element,
                        rank,
                        words: vec![word],
                    });
                }
            }
        }
        if let Some(piece) = current {
            previous_rank = piece.rank;
            pieces.push(piece);
        }
    }
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_by_key(|&index| {
        let piece = &pieces[index];
        (piece.rank, words[piece.words[0]].bounds[0], index)
    });
    order.into_iter().map(|index| pieces[index].clone()).collect()
}

fn union_bounds(mut boxes: impl Iterator<Item = [u32; 4]>) -> Option<[u32; 4]> {
    let first = boxes.next()?;
    Some(boxes.fold(first, |a, b| {
        [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
    }))
}

fn apply(primary: &mut ExtractedDocument, plan: &Plan) {
    let Some(doc) = primary.ocr_internal_document.as_mut() else {
        return;
    };
    let serialized = serialize(&doc.elements);
    let original = std::mem::take(&mut doc.elements);
    for (index, ((piece, text), bounds)) in plan.pieces.iter().zip(&plan.texts).zip(&plan.bounds).enumerate() {
        let mut element = original[piece.element].clone();
        element.text = text.clone();
        let [x0, y0, x1, y1] = bounds.map(f64::from);
        element.bbox = Some(BoundingBox { x0, y0, x1, y1 });
        element.ocr_geometry = Some(rectangle(*bounds));
        doc.elements.push(element.with_index(index as u32));
    }
    if primary.content == serialized {
        primary.content = serialize(&doc.elements);
    }
}

/// The flat text the OCR backend builds from its structured blocks: non-empty texts joined by a
/// blank line (`ocr::processor::execution::flatten_hocr_elements_to_text`).
fn serialize(elements: &[crate::types::internal::InternalElement]) -> String {
    elements
        .iter()
        .filter(|element| {
            !element.text.is_empty() && !matches!(element.kind, crate::types::internal::ElementKind::PageBreak)
        })
        .map(|element| element.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}
