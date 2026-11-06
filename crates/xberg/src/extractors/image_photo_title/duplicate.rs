use std::ops::Range;

use super::{GrayImage, MaskWord, PRIMARY_PROTECTION_CONFIDENCE, RING_RADIUS, Rect, text_lines};
use crate::types::internal::{ElementKind, InternalElement};
use crate::types::{OcrElement, OcrElementLevel};

const MAX_PRIMARY_GLYPH_MARGIN: u32 = 2 * RING_RADIUS;

fn foreground_columns(mask: &GrayImage, word: &MaskWord) -> Option<Vec<Rect>> {
    let mut runs = Vec::<Rect>::with_capacity(word.text.len());
    for x in word.bounds.left..word.bounds.right {
        let mut foreground = (word.bounds.top..word.bounds.bottom).filter(|&y| mask.get_pixel(x, y).0[0] == 0);
        let Some(top) = foreground.next() else { continue };
        let bottom = foreground.next_back().unwrap_or(top) + 1;
        if let Some(run) = runs.last_mut().filter(|run| run.right == x) {
            run.right += 1;
            run.top = run.top.min(top);
            run.bottom = run.bottom.max(bottom);
        } else {
            if runs.len() == word.text.len() {
                return None;
            }
            runs.push(Rect {
                left: x,
                top,
                right: x + 1,
                bottom,
            });
        }
    }
    (runs.len() == word.text.len()).then_some(runs)
}

fn unique_primary_glyph(original: &[OcrElement], glyph: &str) -> Option<Rect> {
    let matching = |level| {
        original
            .iter()
            .filter(move |element| element.level == level && element.text.trim() == glyph)
    };
    let mut lines = matching(OcrElementLevel::Line);
    let mut words = matching(OcrElementLevel::Word);
    let line = lines.next()?;
    let word = words.next()?;
    let bounds = Rect::from_word(word)?;
    (lines.next().is_none()
        && words.next().is_none()
        && Rect::from_word(line) == Some(bounds)
        && !original.iter().any(|other| {
            other.level == OcrElementLevel::Word
                && !std::ptr::eq(other, word)
                && Rect::from_word(other).is_some_and(|other_bounds| other_bounds.intersects(bounds))
        })
        && (0.0..PRIMARY_PROTECTION_CONFIDENCE).contains(&line.confidence.recognition)
        && (0.0..PRIMARY_PROTECTION_CONFIDENCE).contains(&word.confidence.recognition))
    .then_some(bounds)
}

fn is_isolated_solid_glyph(mask: &GrayImage, glyph: Rect, primary: Rect) -> bool {
    primary.right <= mask.width()
        && primary.bottom <= mask.height()
        && (primary.left..primary.right)
            .all(|x| (primary.top..primary.bottom).all(|y| (mask.get_pixel(x, y).0[0] == 0) == glyph.contains(x, y)))
}

fn duplicates_certified_glyph(
    masks: [&GrayImage; 2],
    crop: Rect,
    original: &[OcrElement],
    glyph: &str,
    bounds: Rect,
) -> Option<()> {
    let source = bounds.translate(crop)?;
    let primary = unique_primary_glyph(original, glyph)?;
    if primary.top != source.top
        || primary.bottom != source.bottom
        || source.left.checked_sub(primary.left)? > MAX_PRIMARY_GLYPH_MARGIN
        || primary.right.checked_sub(source.right)? > MAX_PRIMARY_GLYPH_MARGIN
    {
        return None;
    }
    let local = Rect {
        left: primary.left.checked_sub(crop.left)?,
        top: bounds.top,
        right: primary.right.checked_sub(crop.left)?,
        bottom: bounds.bottom,
    };
    masks
        .iter()
        .all(|mask| is_isolated_solid_glyph(mask, bounds, local))
        .then_some(())
}

pub(super) fn duplicate_title_glyph(
    words: &[MaskWord],
    masks: [&GrayImage; 2],
    crop: Rect,
    original: &[OcrElement],
) -> Option<char> {
    let mut duplicate = None;
    for word in words {
        // A shared run count lets the two masks align separated foreground with the certified letters.
        let Some(runs) = foreground_columns(masks[0], word) else {
            continue;
        };
        if foreground_columns(masks[1], word).as_ref() != Some(&runs) {
            continue;
        }
        for (glyph, bounds) in word.text.bytes().map(char::from).zip(runs) {
            if duplicates_certified_glyph(masks, crop, original, glyph.encode_utf8(&mut [0; 4]), bounds).is_some()
                && duplicate.replace(glyph).is_some()
            {
                return None;
            }
        }
    }
    duplicate
}

fn unique_line<'a, Position: Copy>(lines: impl Iterator<Item = (Position, &'a str)>, glyph: &str) -> Option<Position> {
    let mut matching = lines.filter_map(|(position, line)| (line == glyph).then_some(position));
    let position = matching.next()?;
    matching.next().is_none().then_some(position)
}

pub(super) struct DuplicateRemoval {
    page: Range<usize>,
    element_index: usize,
}

pub(super) fn duplicate_removal(page: &str, elements: &[InternalElement], glyph: char) -> Option<DuplicateRemoval> {
    let mut bytes = [0; 4];
    let glyph: &str = glyph.encode_utf8(&mut bytes);
    let page_offset = unique_line(text_lines(page), glyph)?;
    let page_suffix = page.get(page_offset..)?.strip_prefix(glyph)?.strip_prefix("\n\n")?;
    if page_offset != 0 && !page[..page_offset].ends_with("\n\n") {
        return None;
    }
    let element_lines = elements
        .iter()
        .enumerate()
        .flat_map(|(index, element)| text_lines(&element.text).map(move |(offset, line)| ((index, offset), line)));
    let (index, _) = unique_line(element_lines, glyph)?;
    let element = &elements[index];
    if element.text != glyph
        || !matches!(element.kind, ElementKind::Paragraph | ElementKind::Heading { .. })
        || element.depth != 0
        || element.page.is_some()
        || !element.annotations.is_empty()
        || element.attributes.is_some()
        || element.anchor.as_deref().is_some_and(|anchor| {
            !matches!(element.kind, ElementKind::Heading { .. }) || !anchor.eq_ignore_ascii_case(glyph)
        })
    {
        return None;
    }
    Some(DuplicateRemoval {
        page: page_offset..page.len() - page_suffix.len(),
        element_index: index,
    })
}

pub(super) fn remove_duplicate(
    page: &mut String,
    elements: &mut Vec<InternalElement>,
    removal: DuplicateRemoval,
    insertion: &mut (usize, (usize, usize)),
) {
    let (page_offset, (index, _)) = insertion;
    let remove_before = removal.page.end <= *page_offset;
    if removal.page.contains(page_offset)
        || removal.element_index == *index
        || remove_before != (removal.element_index < *index)
    {
        return;
    }
    // Both projections must agree about the duplicate's order relative to the insertion.
    if remove_before {
        *page_offset -= removal.page.len();
        *index -= 1;
    }
    page.replace_range(removal.page, "");
    elements.remove(removal.element_index);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OcrBoundingGeometry, OcrConfidence};

    const CROP: Rect = Rect {
        left: 100,
        top: 200,
        right: 280,
        bottom: 250,
    };

    fn primary(text: &str, left: u32, width: u32, confidence: f64) -> OcrElement {
        OcrElement::new(
            text,
            OcrBoundingGeometry::Rectangle {
                left,
                top: 200,
                width,
                height: 25,
            },
            OcrConfidence::from_tesseract(confidence),
        )
        .with_level(OcrElementLevel::Word)
    }

    #[test]
    fn duplicate_glyph_requires_identical_isolated_support_and_unique_primary_occurrence() {
        let bounds = Rect {
            left: 10,
            top: 0,
            right: 13,
            bottom: 25,
        };
        let mask = GrayImage::from_fn(180, 50, |x, y| {
            image::Luma([if bounds.contains(x, y) { 0 } else { 255 }])
        });
        let word = primary("I", 109, 5, 75.0);
        let line = word.clone().with_level(OcrElementLevel::Line);
        let original = [word.clone(), line.clone()];
        assert_eq!(
            duplicates_certified_glyph([&mask, &mask], CROP, &original, "I", bounds),
            Some(())
        );
        assert_eq!(
            duplicates_certified_glyph(
                [&mask, &mask],
                CROP,
                &[word.clone(), line.clone(), primary("42", 112, 5, 60.0)],
                "I",
                bounds,
            ),
            None
        );
        let mut separate_foreground = mask.clone();
        separate_foreground.put_pixel(9, 12, image::Luma([0]));
        assert_eq!(
            duplicates_certified_glyph([&mask, &separate_foreground], CROP, &original, "I", bounds),
            None
        );
        assert_eq!(
            duplicates_certified_glyph(
                [&mask, &mask],
                CROP,
                &[word, line, primary("I", 270, 5, 60.0)],
                "I",
                bounds
            ),
            None
        );
    }

    #[test]
    fn refuses_duplicate_removal_when_either_projection_repeats_the_same_label() {
        let mut heading = InternalElement::text(ElementKind::Heading { level: 1 }, "I", 0);
        heading.anchor = Some("i".into());
        let paragraph = InternalElement::text(ElementKind::Paragraph, "Unrelated\nI", 0);
        assert!(duplicate_removal("I\n\nBelow", &[heading.clone()], 'I').is_some());
        assert!(duplicate_removal("I\n\nBelow\n\nI", &[heading.clone()], 'I').is_none());
        assert!(duplicate_removal("I\n\nBelow", &[heading, paragraph], 'I').is_none());
    }
}
