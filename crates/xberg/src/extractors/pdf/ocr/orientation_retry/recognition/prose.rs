//! Admission for a single complete recognition-only prose region.

use super::*;

pub(super) fn complete_region(
    original: &ExtractedDocument,
    candidate: &ExtractedDocument,
    dimensions: (u32, u32),
    thresholds: &OcrQualityThresholds,
) -> Option<BoundingBox> {
    if !primary_supported(candidate) {
        return None;
    }
    let [paragraph] = candidate.ocr_internal_document.as_ref()?.elements.as_slice() else {
        return None;
    };
    let elements = candidate.ocr_elements.as_deref()?;
    let words = checked_words(candidate)?;
    let bbox = paragraph_region(paragraph, original, &words)?;
    let tokens = words.iter().map(|word| word.text.as_str());
    if !tokens.clone().eq(candidate.content.split_whitespace())
        || !tokens.eq(paragraph.text.split_whitespace())
        || !words
            .iter()
            .all(|word| word.confidence.recognition * 100.0 >= thresholds.min_ocr_mean_confidence)
    {
        return None;
    }

    let words = words
        .iter()
        .map(|word| iterator_word(word, bbox))
        .collect::<Option<Vec<_>>>()?;
    let keys = iterator_line_keys(elements, &words)?;
    let groups = line_lengths(&paragraph.text, &keys)?;
    let frames = [90, 270]
        .into_iter()
        .filter(|&angle| coherent_frame(&words, &groups, dimensions, angle))
        .count();
    (frames == 1).then_some(bbox)
}

fn paragraph_region(
    paragraph: &InternalElement,
    original: &ExtractedDocument,
    words: &[&OcrElement],
) -> Option<BoundingBox> {
    let (left, top, width, height) = paragraph.ocr_geometry.as_ref()?.to_aabb();
    let bounds = BoundingBox {
        x0: f64::from(left),
        y0: f64::from(top),
        x1: f64::from(left.checked_add(width)?),
        y1: f64::from(top.checked_add(height)?),
    };
    paragraph
        .attributes
        .as_ref()?
        .get(crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE)?;
    let primary = checked_words(original)?;
    if words.len() <= primary.len()
        || !words.iter().all(|word| region_contains(word, bounds))
        || !primary.iter().all(|word| region_contains(word, bounds))
    {
        return None;
    }
    let union = words.iter().fold((u32::MAX, u32::MAX, 0, 0), |(x0, y0, x1, y1), word| {
        let (left, top, width, height) = word.geometry.to_aabb();
        (x0.min(left), y0.min(top), x1.max(left + width), y1.max(top + height))
    });
    (union == (left, top, left + width, top + height)).then_some(bounds)
}

fn line_lengths(paragraph: &str, keys: &[Option<usize>]) -> Option<Vec<usize>> {
    let counts: Vec<_> = keys.chunk_by(|left, right| left == right).map(<[_]>::len).collect();
    let paragraph_lines = paragraph.lines().map(|line| line.split_whitespace().count());
    (counts.iter().copied().eq(paragraph_lines)
        && counts.iter().filter(|&&count| count > 1).count() >= 2
        && counts.iter().take(counts.len() - 1).all(|&count| count > 1))
    .then_some(counts)
}

// Median line thickness of the region; a box thinner than a third of it is a mark, not a word. ~keep
fn text_sized(words: &[HocrWord], dimensions: (u32, u32), angle: u16) -> bool {
    let Some(mut heights) = words
        .iter()
        .map(|word| project_word(word, dimensions, angle).map(|word| word.height))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    heights.sort_unstable();
    let median = heights[heights.len() / 2];
    heights.iter().all(|&height| height.saturating_mul(3) >= median)
}

fn coherent_frame(words: &[HocrWord], groups: &[usize], dimensions: (u32, u32), angle: u16) -> bool {
    if !text_sized(words, dimensions, angle) {
        return false;
    }
    let mut start = 0;
    let mut previous: Option<(u32, u32, u32)> = None;
    for &length in groups {
        let Some((left, top, bottom)) = projected_line(&words[start..start + length], dimensions, angle) else {
            return false;
        };
        if let Some((previous_left, previous_top, previous_bottom)) = previous {
            let line_height = (bottom - top).max(previous_bottom - previous_top);
            if top < previous_bottom
                || top - previous_bottom > line_height
                || left.abs_diff(previous_left) > line_height
            {
                return false;
            }
        }
        previous = Some((left, top, bottom));
        start += length;
    }
    true
}

fn projected_line(words: &[HocrWord], dimensions: (u32, u32), angle: u16) -> Option<(u32, u32, u32)> {
    let first = project_word(words.first()?, dimensions, angle)?;
    let (mut top, mut bottom) = (first.top, first.top + first.height);
    let mut previous = first.clone();
    for word in words.iter().skip(1) {
        let word = project_word(word, dimensions, angle)?;
        let previous_right = previous.left + previous.width;
        if word.left < previous_right
            || word.left - previous_right > previous.height.max(word.height)
            || word.top >= previous.top + previous.height
            || previous.top >= word.top + word.height
        {
            return None;
        }
        top = top.min(word.top);
        bottom = bottom.max(word.top + word.height);
        previous = word;
    }
    Some((first.left, top, bottom))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OcrBoundingGeometry, OcrConfidence};

    fn word(text: &str, left: u32, top: u32) -> OcrElement {
        word_of_width(text, left, top, 9)
    }

    fn word_of_width(text: &str, left: u32, top: u32, width: u32) -> OcrElement {
        OcrElement::new(
            text,
            OcrBoundingGeometry::Rectangle {
                left: top,
                top: 200 - left - 12,
                width,
                height: 12,
            },
            OcrConfidence {
                recognition: 0.95,
                detection: None,
            },
        )
        .with_level(OcrElementLevel::Word)
        .with_metadata("backend", "tesseract-iterator".into())
    }

    fn result(elements: Vec<OcrElement>, content: &str, count: u32) -> ExtractedDocument {
        let mut result = ExtractedDocument {
            content: content.into(),
            ocr_elements: Some(elements),
            ..Default::default()
        };
        let additional = &mut result.metadata.additional;
        additional.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
            200.into(),
        );
        additional.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
            200.into(),
        );
        additional.insert("word_count".into(), count.into());
        additional.insert("mean_text_conf".into(), 95.into());
        result
    }

    fn candidate() -> ExtractedDocument {
        candidate_with(["Complete", "first", "Clear", "second", "ending"], [9; 5])
    }

    fn candidate_with(words: [&str; 5], widths: [u32; 5]) -> ExtractedDocument {
        let text = format!("{} {}\n{} {}\n{}", words[0], words[1], words[2], words[3], words[4]);
        let text = text.as_str();
        let elements = crate::ocr::conversion::tesseract_elements_with_lines(vec![
            (0, word_of_width(words[0], 15, 40, widths[0])),
            (0, word_of_width(words[1], 30, 40, widths[1])),
            (1, word_of_width(words[2], 15, 53, widths[2])),
            (1, word_of_width(words[3], 30, 53, widths[3])),
            (2, word_of_width(words[4], 15, 67, widths[4])),
        ]);
        let mut candidate = result(elements, text, 5);
        let mut paragraph = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            text,
            0,
        );
        paragraph.attributes = Some(ahash::AHashMap::from_iter([(
            crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE.into(),
            "native-line-block".into(),
        )]));
        paragraph.ocr_geometry = Some(OcrBoundingGeometry::Rectangle {
            left: 40,
            top: 158,
            width: 36,
            height: 27,
        });
        candidate.ocr_internal_document = Some(InternalDocument {
            elements: vec![paragraph],
            ..Default::default()
        });
        candidate
    }

    fn selected(original: &ExtractedDocument, candidate: &ExtractedDocument, allow_prose: bool) -> bool {
        accepts(
            original,
            candidate,
            &OcrQualityThresholds::default(),
            ConfidenceSemantics::Legibility { scale_max: 100.0 },
            0.5,
            allow_prose,
        )
    }

    #[test]
    fn prose_requires_literal_confident_inventory_and_preserves_primary_occurrences() {
        let candidate = candidate();
        let original = result(vec![word("Clear", 15, 53)], "Clear", 1);
        assert!(selected(&original, &candidate, true));
        assert!(!selected(&original, &candidate, false), "detached route");
        let protected = result(vec![word("ID99", 15, 53)], "ID99", 1);
        assert!(
            !selected(&protected, &candidate, true),
            "protected occurrence at the same position"
        );
        let outside = result(vec![word("O", 15, 95)], "O", 1);
        assert!(
            !selected(&outside, &candidate, true),
            "unprotected ink outside the candidate block"
        );

        let mut missing = candidate.clone();
        missing.ocr_elements.as_mut().unwrap().remove(0);
        assert!(!selected(&original, &missing, true), "missing line");
        let mut stray = candidate.clone();
        stray
            .ocr_elements
            .as_mut()
            .unwrap()
            .last_mut()
            .unwrap()
            .confidence
            .recognition = 0.4;
        assert!(!selected(&original, &stray, true), "weak trailing word");
        let mut invalid = candidate;
        invalid.content.push_str(" speckle");
        assert!(!selected(&original, &invalid, true), "unowned rendered token");
    }

    #[test]
    fn prose_admission_follows_word_boxes_not_word_letters() {
        for words in [
            ["Complete", "first", "Clear", "second", "ending"],
            ["Total,", "3.5", "a", "no.", "O'Neil-2"],
        ] {
            let candidate = candidate_with(words, [9; 5]);
            let original = result(vec![word(words[2], 15, 53)], words[2], 1);
            assert!(selected(&original, &candidate, true), "{words:?}");
        }
    }

    #[test]
    fn prose_refuses_a_token_whose_box_is_not_text_sized() {
        for tiny in [",", "x"] {
            let candidate = candidate_with(["Complete", "first", "Clear", tiny, "ending"], [9, 9, 9, 2, 9]);
            let original = result(vec![word("Clear", 15, 53)], "Clear", 1);
            assert!(!selected(&original, &candidate, true), "{tiny:?}");
        }
    }
}
