//! A sideways read of one table with text above or below it, accepted only word by word.

use super::{iterator_word, single_table, table_matches_words};
use crate::extractors::pdf::ocr::scoring::word_count_of;
use crate::ocr::tesseract_backend::orientation_evidence::{MAX_BYTES, MAX_WORDS};
use crate::table_core::{HocrWord, detect_rows, find_row_index};
use crate::types::{BoundingBox, ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};

/// Returns the page text when every word of `result.content` is owned by the table or by
/// a row of independently read words wholly above or below it, in geometric reading order.
pub(super) fn complete_text(result: &ExtractedDocument, row_threshold: f64) -> Option<String> {
    let (table, bbox, (width, height)) = single_table(result)?;
    let markdown = table.markdown.trim();
    let content = result.content.trim();
    let start = content.find(markdown).filter(|_| !markdown.is_empty())?;
    if content[start + markdown.len()..].contains(markdown) {
        return None;
    }
    let (before, after) = (&content[..start], &content[start + markdown.len()..]);
    if before.trim().is_empty() && after.trim().is_empty() {
        return None;
    }
    let evidence = result.ocr_elements.as_deref()?;
    let table_words = table.cells.iter().flatten().flat_map(|cell| cell.split_whitespace());
    let total = table_words.clone().count() + before.split_whitespace().count() + after.split_whitespace().count();
    if evidence.len() > MAX_WORDS
        || content.len() > MAX_BYTES
        || usize::try_from(word_count_of(&result.metadata.additional)?).ok()? != total
    {
        return None;
    }
    let (mut inside, mut above, mut below) = (Vec::new(), Vec::new(), Vec::new());
    for element in evidence.iter().filter(|element| element.level == OcrElementLevel::Word) {
        let rect = word_rect(element, (width, height))?;
        if bbox.x0 <= rect.x0 && rect.x1 <= bbox.x1 && bbox.y0 <= rect.y0 && rect.y1 <= bbox.y1 {
            inside.push(element);
        } else if rect.y1 <= bbox.y0 {
            above.push(element);
        } else if rect.y0 >= bbox.y1 {
            below.push(element);
        } else {
            return None;
        }
    }
    let inside = inside
        .into_iter()
        .map(|element| iterator_word(element, bbox))
        .collect::<Option<Vec<_>>>()?;
    if !table_matches_words(&table.cells, &inside, row_threshold)
        || !reads_in_order(&above, before, row_threshold)
        || !reads_in_order(&below, after, row_threshold)
    {
        return None;
    }
    Some(
        before
            .split_whitespace()
            .chain(table_words)
            .chain(after.split_whitespace())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(super) fn reads_in_order(elements: &[&OcrElement], text: &str, row_threshold: f64) -> bool {
    let mut words = Vec::with_capacity(elements.len());
    for element in elements {
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = element.geometry
        else {
            return false;
        };
        words.push(HocrWord {
            text: element.text.clone(),
            left,
            top,
            width,
            height,
            confidence: element.confidence.recognition,
        });
    }
    let rows = detect_rows(&words, row_threshold);
    let mut keyed = Vec::with_capacity(words.len());
    for word in &words {
        let Some(row) = find_row_index(&rows, word) else {
            return false;
        };
        keyed.push((row, word.left, word.text.as_str()));
    }
    keyed.sort_unstable_by_key(|&(row, left, _)| (row, left));
    keyed
        .windows(2)
        .all(|pair| pair[0].0 != pair[1].0 || pair[0].1 != pair[1].1)
        && keyed
            .iter()
            .flat_map(|(_, _, word)| word.split_whitespace())
            .eq(text.split_whitespace())
}

pub(super) fn word_rect(element: &OcrElement, (width, height): (u32, u32)) -> Option<BoundingBox> {
    let OcrBoundingGeometry::Rectangle {
        left,
        top,
        width: w,
        height: h,
    } = element.geometry
    else {
        return None;
    };
    let (right, bottom) = (left.checked_add(w)?, top.checked_add(h)?);
    (element.backend_metadata.get("backend")?.as_str()? == "tesseract-iterator"
        && (0.0..=1.0).contains(&element.confidence.recognition)
        && left < right
        && top < bottom
        && right <= width
        && bottom <= height)
        .then_some(BoundingBox {
            x0: left.into(),
            y0: top.into(),
            x1: right.into(),
            y1: bottom.into(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table_core::table_to_markdown;

    fn word(text: &str, left: u32, top: u32) -> OcrElement {
        OcrElement {
            text: text.into(),
            level: OcrElementLevel::Word,
            geometry: OcrBoundingGeometry::Rectangle {
                left,
                top,
                width: 10,
                height: 10,
            },
            backend_metadata: std::collections::HashMap::from([("backend".into(), "tesseract-iterator".into())]),
            ..Default::default()
        }
    }

    fn candidate() -> ExtractedDocument {
        let mut candidate = super::super::tests::table_candidate();
        let markdown = table_to_markdown(&candidate.tables[0].cells);
        candidate.tables[0].markdown = markdown.clone();
        candidate.content = format!("Ledger Summary\n\n{markdown}\n\nTotal 9");
        candidate.metadata.additional.insert("word_count".into(), 13.into());
        let words = candidate.ocr_elements.as_mut().unwrap();
        words.extend([
            word("Summary", 60, 10),
            word("Ledger", 30, 10),
            word("9", 60, 180),
            word("Total", 30, 180),
        ]);
        candidate
    }

    #[test]
    fn accepts_text_above_and_below_a_proven_table_in_reading_order() {
        let candidate = candidate();
        assert_eq!(
            complete_text(&candidate, 0.5).as_deref(),
            Some("Ledger Summary Cash 100 31,290 Rent 107 77,078 Fuel 114 71,433 Total 9")
        );
    }

    #[test]
    fn refuses_non_finite_or_out_of_range_iterator_confidence() {
        assert!(complete_text(&candidate(), 0.5).is_some());
        for invalid in [f64::NAN, f64::INFINITY, -0.01, 1.01] {
            let mut read = candidate();
            read.ocr_elements.as_mut().unwrap()[0].confidence.recognition = invalid;
            assert_eq!(complete_text(&read, 0.5), None, "invalid confidence {invalid}");
        }
    }

    #[test]
    fn refuses_text_that_is_not_owned_word_for_word() {
        let swapped = {
            let mut candidate = candidate();
            candidate.content = candidate.content.replace("Ledger Summary", "Summary Ledger");
            candidate
        };
        let beside = {
            let mut candidate = candidate();
            candidate.ocr_elements.as_mut().unwrap().push(word("Side", 190, 90));
            candidate.content.push_str(" Side");
            candidate.metadata.additional.insert("word_count".into(), 14.into());
            candidate
        };
        let unread = {
            let mut candidate = candidate();
            candidate.content = candidate.content.replace("Total 9", "Total 9 Extra");
            candidate.metadata.additional.insert("word_count".into(), 14.into());
            candidate
        };
        let miscounted = {
            let mut candidate = candidate();
            candidate.metadata.additional.insert("word_count".into(), 12.into());
            candidate
        };
        let unproven_table = {
            let mut candidate = candidate();
            candidate
                .ocr_elements
                .as_mut()
                .unwrap()
                .retain(|word| word.text != "77,078");
            candidate
        };
        for (name, case) in [
            ("reordered heading", swapped),
            ("word beside the table", beside),
            ("content word without evidence", unread),
            ("word count mismatch", miscounted),
            ("table cell without evidence", unproven_table),
        ] {
            assert_eq!(complete_text(&case, 0.5), None, "{name}");
        }
    }

    fn rewrite(candidate: &mut ExtractedDocument, text: &str, geometry: impl Fn(&mut OcrBoundingGeometry)) {
        let element = candidate
            .ocr_elements
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|element| element.text == text)
            .unwrap();
        geometry(&mut element.geometry);
    }

    fn tall_candidate() -> ExtractedDocument {
        let mut candidate = candidate();
        for key in [
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
        ] {
            candidate.metadata.additional.insert(key.into(), 400.into());
        }
        candidate
    }

    #[test]
    fn a_table_repeated_in_the_markdown_is_not_a_unique_table() {
        let mut read = tall_candidate();
        let markdown = read.tables[0].markdown.clone();
        read.content = format!("{markdown}\n\n{markdown}");
        let echoed: Vec<_> = markdown.lines().map(str::to_owned).collect();
        let mut count = 9;
        let elements = read.ocr_elements.as_mut().unwrap();
        elements.retain(|element| element.geometry.to_aabb().1 >= 30 && element.geometry.to_aabb().1 < 160);
        for (row, line) in echoed.iter().enumerate() {
            for (column, token) in line.split_whitespace().enumerate() {
                elements.push(word(token, 20 + 15 * column as u32, 200 + 15 * row as u32));
                count += 1;
            }
        }
        read.metadata.additional.insert("word_count".into(), count.into());
        assert_eq!(complete_text(&read, 0.5), None);
    }

    #[test]
    fn a_read_with_no_text_outside_the_table_is_left_to_the_table_route() {
        let mut read = candidate();
        read.content = read.tables[0].markdown.clone();
        read.ocr_elements
            .as_mut()
            .unwrap()
            .retain(|element| !["Ledger", "Summary", "Total", "9"].contains(&element.text.as_str()));
        read.metadata.additional.insert("word_count".into(), 9.into());
        assert_eq!(complete_text(&read, 0.5), None);
    }

    #[test]
    fn a_word_straddling_the_table_edge_refuses_the_read() {
        let mut read = candidate();
        read.ocr_elements.as_mut().unwrap().push(word("edge", 190, 155));
        assert_eq!(complete_text(&read, 0.5), None);
        let mut inside_x = candidate();
        inside_x.ocr_elements.as_mut().unwrap().push(word("edge", 60, 155));
        assert_eq!(complete_text(&inside_x, 0.5), None);
    }

    #[test]
    fn text_outside_the_table_needs_iterator_evidence_from_the_tesseract_iterator() {
        let mut read = candidate();
        read.ocr_elements
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|element| element.text == "Ledger")
            .unwrap()
            .backend_metadata
            .insert("backend".into(), "hocr".into());
        assert_eq!(complete_text(&read, 0.5), None);
        let mut unnamed = candidate();
        unnamed
            .ocr_elements
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|element| element.text == "Ledger")
            .unwrap()
            .backend_metadata
            .clear();
        assert_eq!(complete_text(&unnamed, 0.5), None);
    }

    #[test]
    fn word_boxes_outside_the_image_or_without_area_refuse_the_read() {
        type Edit = fn(&mut OcrBoundingGeometry);
        let cases: [(&str, &str, Edit); 5] = [
            ("zero width", "Ledger", |geometry| {
                if let OcrBoundingGeometry::Rectangle { width, .. } = geometry {
                    *width = 0;
                }
            }),
            ("zero height", "Ledger", |geometry| {
                if let OcrBoundingGeometry::Rectangle { height, .. } = geometry {
                    *height = 0;
                }
            }),
            ("past the right edge", "Summary", |geometry| {
                if let OcrBoundingGeometry::Rectangle { left, .. } = geometry {
                    *left = 195;
                }
            }),
            ("past the bottom edge", "9", |geometry| {
                if let OcrBoundingGeometry::Rectangle { top, .. } = geometry {
                    *top = 195;
                }
            }),
            ("coordinate overflow", "Ledger", |geometry| {
                if let OcrBoundingGeometry::Rectangle { left, .. } = geometry {
                    *left = u32::MAX - 5;
                }
            }),
        ];
        assert!(complete_text(&candidate(), 0.5).is_some());
        for (name, text, edit) in cases {
            let mut read = candidate();
            rewrite(&mut read, text, edit);
            assert_eq!(complete_text(&read, 0.5), None, "{name}");
        }
    }

    #[test]
    fn two_words_at_one_position_in_a_row_refuse_the_read() {
        for (first, second, content) in [
            ("Ledger", "Summary", "Ledger Summary"),
            ("Summary", "Ledger", "Summary Ledger"),
        ] {
            let mut read = candidate();
            read.content = read.content.replace("Ledger Summary", content);
            let elements = read.ocr_elements.as_mut().unwrap();
            elements.retain(|element| !["Ledger", "Summary"].contains(&element.text.as_str()));
            elements.extend([word(first, 30, 10), word(second, 30, 10)]);
            assert_eq!(complete_text(&read, 0.5), None, "{content}");
        }
        let mut apart = candidate();
        let elements = apart.ocr_elements.as_mut().unwrap();
        elements.retain(|element| !["Ledger", "Summary"].contains(&element.text.as_str()));
        elements.extend([word("Ledger", 30, 10), word("Summary", 60, 10)]);
        assert!(complete_text(&apart, 0.5).is_some());
    }

    #[test]
    fn the_implicit_retry_accepts_text_around_a_table_only_with_a_consistent_word_count() {
        use crate::core::config::{ExtractionConfig, OcrQualityThresholds};
        use crate::plugins::ConfidenceSemantics;

        let retry = super::super::OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let mut counted = candidate();
        assert!(retry.accepts(&counted, confidence, 0.5));
        counted.metadata.additional.insert("word_count".into(), 12.into());
        assert!(!retry.accepts(&counted, confidence, 0.5));
        let mut unrotated = candidate();
        unrotated.metadata.additional.insert(
            crate::ocr_metadata_keys::OCR_ORIENTATION_DEGREES_METADATA_KEY.into(),
            0.into(),
        );
        assert!(!retry.accepts(&unrotated, confidence, 0.5));
    }
}
