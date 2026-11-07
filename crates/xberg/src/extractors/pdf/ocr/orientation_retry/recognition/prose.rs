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

/// Text-sized evidence for a region, all measured in the upright frame.
/// - The median line thickness must reach a page-relative floor, so uniformly tiny boxes (dust, grain) fail.
/// - A box thinner than a third of the median is a mark (`-`, `.`, `•`); a few are tolerated, a share is not.
/// - Most boxes must be at least as long as they are thick, as words are and isolated strokes (`l I |`) are not.
/// - Most tokens must carry a letter, so a grid of digits or marks is not read as prose.
///
/// A region whose majority is a large heading font sets the median and thins the body words, so it is refused.
/// ~keep
fn text_sized(words: &[HocrWord], dimensions: (u32, u32), angle: u16) -> bool {
    const FLOOR_DIVISOR: u32 = 300;
    const THIN_SHARE: usize = 8;
    let Some(projected) = words
        .iter()
        .map(|word| project_word(word, dimensions, angle))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    let mut thickness: Vec<u32> = projected.iter().map(|word| word.height).collect();
    thickness.sort_unstable();
    let Some(&median) = thickness.get(thickness.len() / 2) else {
        return false;
    };
    let floor = (dimensions.0.max(dimensions.1) / FLOOR_DIVISOR).max(1);
    let count = projected.len();
    let thin = projected
        .iter()
        .filter(|word| word.height.saturating_mul(3) < median)
        .count();
    let stroke = projected.iter().filter(|word| word.width < word.height).count();
    let lettered = words
        .iter()
        .filter(|word| word.text.chars().any(char::is_alphabetic))
        .count();
    median >= floor && thin * THIN_SHARE < count && stroke * 2 <= count && lettered * 2 >= count
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

    fn upright_words(tokens: &[(&str, u32, u32)]) -> Vec<HocrWord> {
        tokens
            .iter()
            .map(|&(text, thickness, length)| HocrWord {
                text: text.into(),
                left: 10,
                top: 10,
                width: thickness,
                height: length,
                confidence: 95.0,
            })
            .collect()
    }

    #[test]
    fn prose_refuses_a_token_whose_box_is_not_text_sized() {
        let original = result(vec![word("Clear", 15, 53)], "Clear", 1);
        let thresholds = OcrQualityThresholds::default();
        let sized = |candidate: &ExtractedDocument| {
            let words = checked_words(candidate).unwrap();
            let paragraph = &candidate.ocr_internal_document.as_ref().unwrap().elements[0];
            let bbox = paragraph_region(paragraph, &original, &words).unwrap();
            let words: Vec<_> = words.iter().map(|word| iterator_word(word, bbox).unwrap()).collect();
            [90, 270].into_iter().any(|angle| text_sized(&words, (200, 200), angle))
        };
        let accepted = candidate_with(["Complete", "first", "Clear", "second", "ending"], [9; 5]);
        assert!(sized(&accepted) && complete_region(&original, &accepted, (200, 200), &thresholds).is_some());
        for tiny in [",", "x"] {
            let candidate = candidate_with(["Complete", "first", "Clear", tiny, "ending"], [9, 9, 9, 2, 9]);
            assert!(!sized(&candidate), "{tiny:?} must fail text_sized itself");
            assert!(!selected(&original, &candidate, true), "{tiny:?}");
        }
    }

    #[test]
    fn thin_box_boundary_is_a_third_of_the_median() {
        let words = |thin: u32| {
            let mut tokens = vec![("end", thin, 20)];
            tokens.extend(["alpha", "beta", "gamma", "delta"].map(|t| (t, 9, 20)));
            upright_words(&tokens)
        };
        assert!(text_sized(&words(9), (200, 200), 90), "full-size reference");
        assert!(
            text_sized(&words(3), (200, 200), 90),
            "exactly a third of 9 is still a word"
        );
        assert!(
            !text_sized(&words(2), (200, 200), 90),
            "below a third of 9 is a mark, and one in five is a share"
        );
    }

    #[test]
    fn a_few_thin_punctuation_tokens_do_not_refuse_a_real_paragraph() {
        let mut tokens = vec![("Between", 9, 40), ("10", 9, 20), ("\u{2013}", 2, 12), ("20", 9, 20)];
        tokens.extend(["units", "were", "counted", "in", "each", "of", "the", "rows"].map(|t| (t, 9, 30)));
        assert!(
            text_sized(&upright_words(&tokens), (200, 200), 90),
            "one dash in twelve tokens"
        );
        tokens.extend([("-", 2, 12), ("\u{2022}", 2, 12), (".", 2, 12)]);
        assert!(
            !text_sized(&upright_words(&tokens), (200, 200), 90),
            "a share of marks is not prose"
        );
    }

    #[test]
    fn a_dominant_heading_size_sets_the_median_and_refuses_small_body_words() {
        let heading = [
            ("Annual", 30, 90),
            ("Report", 30, 90),
            ("Summary", 30, 90),
            ("Plan", 30, 90),
        ];
        let body = [("with", 6, 20), ("notes", 6, 20), ("below", 6, 20)];
        let mixed: Vec<_> = heading.into_iter().chain(body).collect();
        assert!(!text_sized(&upright_words(&mixed), (200, 200), 90));
        assert!(text_sized(&upright_words(&heading), (200, 200), 90));
    }

    #[test]
    fn uniformly_tiny_or_stroke_like_reads_are_refused() {
        let grain = [
            ("a", 4, 5),
            ("an", 4, 6),
            ("on", 4, 6),
            ("it", 4, 5),
            ("so", 4, 6),
            ("up", 4, 6),
        ];
        assert!(
            !text_sized(&upright_words(&grain), (3000, 3000), 90),
            "below the page floor"
        );
        assert!(
            text_sized(&upright_words(&grain), (1200, 1200), 90),
            "same boxes on a small page"
        );
        let strokes = [
            ("l", 12, 5),
            ("I", 12, 5),
            ("|", 12, 4),
            ("i", 12, 5),
            (",", 12, 4),
            ("l", 12, 5),
        ];
        assert!(
            !text_sized(&upright_words(&strokes), (3000, 3000), 90),
            "isolated strokes"
        );
        let digits = [
            ("10", 12, 20),
            ("20", 12, 20),
            ("30", 12, 20),
            ("40", 12, 20),
            ("5.5", 12, 24),
        ];
        assert!(
            !text_sized(&upright_words(&digits), (3000, 3000), 90),
            "a grid of numbers"
        );
        assert!(!text_sized(&[], (3000, 3000), 90), "no words");
    }
}
