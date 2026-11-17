use super::{HocrWordOccurrence, InternalElement, Line, LineBounds, word_text};
use crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE;

const RIGHT_PEER_LOOKAHEAD: usize = 4;

pub(super) fn bracketed_terminal_fragment(
    lines: &[Line<'_>],
    index: usize,
    elements: &[InternalElement],
    page_width: u32,
) -> Option<()> {
    let (prior, suffix) = (&lines[index], &lines[index + 1]);
    let before = LineBounds::from_words(prior.words)?;
    let after = LineBounds::from_words(suffix.words)?;
    let height = before.height();
    let width = before.right - before.left;
    let start_tolerance = (height / 10).max(1);
    let wrap_gap = after.top.checked_sub(before.bottom)?;
    let outdent = before.left.checked_sub(after.left)?;
    if prior.words.len() < 2
        || before.right > page_width
        || u64::from(width) * 2 < u64::from(page_width)
        || outdent == 0
        || outdent > start_tolerance
        || after.right <= before.left
        || after.right > before.right
        || wrap_gap == 0
        || !after.follows(before)
        || prior.words.last()?.bounds[2] != before.right
        || !separate_blocks(&elements[prior.element], &elements[suffix.element])
    {
        return None;
    }

    let marker = lines.get(index.checked_sub(1)?)?;
    let [glyph] = marker.words else { return None };
    let earlier = lines.get(index.checked_sub(2)?)?;
    let (leading_glyph, body_words) = earlier.words.split_first()?;
    let body = LineBounds::from_words(body_words)?;
    let earlier_gap = before.top.checked_sub(body.bottom)?;
    if !detached_glyph(glyph, before, elements)
        || !detached_glyph(leading_glyph, body, elements)
        || body_words.len() < 2
        || body.left.abs_diff(before.left) > start_tolerance
        || u64::from(body.right - body.left) * 4 < u64::from(width) * 3
        || earlier_gap <= wrap_gap
        || earlier_gap > height
    {
        return None;
    }

    // A resumed body line must be farther from the fragment than its source line. ~keep
    let following = LineBounds::from_words(lines.get(index + 2)?.words)?;
    if following.top.checked_sub(after.bottom)? <= wrap_gap {
        return None;
    }
    contiguous_right_peer(&lines[index + 2..], before, after, start_tolerance).then_some(())
}

fn separate_blocks(prior: &InternalElement, suffix: &InternalElement) -> bool {
    hocr_block_id(prior)
        .zip(hocr_block_id(suffix))
        .is_some_and(|(prior, suffix)| prior != suffix)
}

fn hocr_block_id(element: &InternalElement) -> Option<&str> {
    element
        .attributes
        .as_ref()?
        .get(HOCR_BLOCK_ID_ATTRIBUTE)
        .map(String::as_str)
}

fn detached_glyph(word: &HocrWordOccurrence, body: LineBounds, elements: &[InternalElement]) -> bool {
    let [left, top, right, bottom] = word.bounds;
    let height = body.height();
    left < right
        && top < bottom
        && right - left <= height / 2
        && bottom - top <= height / 2
        && top >= body.top
        && bottom <= body.bottom
        && body.left.checked_sub(right).is_some_and(|gap| gap > 0 && gap <= height)
        && word_text(elements, word).is_some_and(|text| text.chars().count() == 1)
}

fn contiguous_right_peer(
    lines: &[Line<'_>],
    before: LineBounds,
    mut previous: LineBounds,
    start_tolerance: u32,
) -> bool {
    let height = before.height();
    let right_tolerance = (height / 4).max(2);
    for line in lines.iter().take(RIGHT_PEER_LOOKAHEAD) {
        let Some(peer) = LineBounds::from_words(line.words) else {
            return false;
        };
        if peer.left.abs_diff(before.left) > start_tolerance
            || !peer.top.checked_sub(previous.bottom).is_some_and(|gap| gap <= height)
        {
            return false;
        }
        if line.words.len() > 1 && peer.right.abs_diff(before.right) <= right_tolerance {
            return true;
        }
        previous = peer;
    }
    false
}
