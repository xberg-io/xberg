use super::{iterator_word, table_matches_words};
use crate::core::config::OcrQualityThresholds;
use crate::extractors::pdf::ocr::document::{
    processed_ocr_layout_dimensions, resolved_ocr_correction_degrees, undo_auto_rotate_point,
};
use crate::extractors::pdf::ocr::scoring::word_count_of;
use crate::extractors::security::SecurityLimits;
use crate::table_core::{HocrWord, table_to_markdown};
use crate::types::{BoundingBox, ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel, Table};

const MAX_CELLS: usize = 1024;
use crate::ocr::tesseract_backend::orientation_evidence::{MAX_BYTES, MAX_WORDS};
const MAX_WORK: usize = MAX_WORDS * MAX_CELLS;

pub(super) fn complete_text(
    result: &ExtractedDocument,
    original: &ExtractedDocument,
    thresholds: &OcrQualityThresholds,
    limits: &SecurityLimits,
    row_threshold: f64,
) -> Option<String> {
    let [first, second] = result.tables.as_slice() else {
        return None;
    };
    let dims = processed_ocr_layout_dimensions(&result.metadata)?;
    let original_dims = processed_ocr_layout_dimensions(&original.metadata)?;
    let correction = resolved_ocr_correction_degrees(&result.metadata)?;
    if !matches!(correction, 90 | 270)
        || original_dims != (dims.1, dims.0)
        || resolved_ocr_correction_degrees(&original.metadata).is_some_and(|angle| angle != 0)
        || !table_only_payload(result)
    {
        return None;
    }
    let read = result.ocr_elements.as_deref()?;
    let primary = original.ocr_elements.as_deref()?;
    if !within_budget([first, second], [result, original], read, primary, limits)
        || !super::recognition::primary_covers_output(original)
        || first.page_number == 0
        || first.page_number != second.page_number
        || read
            .iter()
            .chain(primary)
            .any(|word| word.page_number != first.page_number)
    {
        return None;
    }
    let boxes = [table_box(first, dims)?, table_box(second, dims)?];
    if boxes[0].y1 >= boxes[1].y0
        || [first, second]
            .iter()
            .any(|table| table_to_markdown(&table.cells).trim() != table.markdown.trim())
    {
        return None;
    }
    let words = [first, second]
        .into_iter()
        .flat_map(|table| table.cells.iter().flatten().flat_map(|cell| cell.split_whitespace()));
    if usize::try_from(word_count_of(&result.metadata.additional)?).ok()? != words.clone().count()
        || !(words.clone().eq(result.content.split_whitespace()) || exact_table_markdown(result, first, second))
    {
        return None;
    }
    let [a, b] = partition_words(read, boxes, dims)?;
    if !table_matches_words(&first.cells, &a, row_threshold) || !table_matches_words(&second.cells, &b, row_threshold) {
        return None;
    }
    primary_inside_tables(primary, boxes, dims, correction, thresholds).then(|| words.collect::<Vec<_>>().join(" "))
}

fn table_only_payload(result: &ExtractedDocument) -> bool {
    result.formulas.is_empty()
        && result.document.is_none()
        && result.internal_document.is_none()
        && result.djot_content.is_none()
        && result.elements.as_ref().is_none_or(Vec::is_empty)
        && result.annotations.as_ref().is_none_or(Vec::is_empty)
        && result.uris.as_ref().is_none_or(Vec::is_empty)
        && result.form_fields.is_empty()
        && result.images.as_ref().is_none_or(Vec::is_empty)
        && result.llm_usage.is_none()
        && result.ocr_internal_document.as_ref().is_none_or(|doc| {
            doc.elements.is_empty()
                && doc.tables.is_empty()
                && doc.images.is_empty()
                && doc.relationships.is_empty()
                && doc.uris.is_empty()
                && doc.formulas.is_empty()
                && doc.form_fields.is_empty()
                && doc.diagrams.is_empty()
                && doc.recorded_formulas.is_empty()
                && doc.annotations.is_none()
                && doc.children.is_none()
                && doc.revisions.is_none()
                && doc.prebuilt_pages.is_none()
                && doc.prebuilt_ocr_elements.is_none()
                && doc.pre_rendered_content.is_none()
        })
}

fn within_budget(
    tables: [&Table; 2],
    reads: [&ExtractedDocument; 2],
    words: &[OcrElement],
    primary: &[OcrElement],
    limits: &SecurityLimits,
) -> bool {
    let elements = words.len().saturating_add(primary.len());
    if elements > MAX_WORDS {
        return false;
    }
    let mut bytes = reads
        .iter()
        .map(|read| read.content.len())
        .fold(0usize, usize::saturating_add);
    let mut cells = 0usize;
    for table in tables {
        bytes = bytes.saturating_add(table.markdown.len());
        if table.cells.len() > MAX_CELLS {
            return false;
        }
        for row in &table.cells {
            cells = cells.saturating_add(row.len());
            if cells > MAX_CELLS.min(limits.max_table_cells) {
                return false;
            }
            bytes = row.iter().fold(bytes, |sum, cell| sum.saturating_add(cell.len()));
        }
    }
    bytes = words
        .iter()
        .chain(primary)
        .fold(bytes, |sum, word| sum.saturating_add(word.text.len()));
    bytes <= MAX_BYTES.min(limits.max_content_size)
        && elements.saturating_mul(cells.saturating_add(1)).saturating_add(bytes) <= MAX_WORK.min(limits.max_iterations)
}

fn table_box(table: &Table, (width, height): (u32, u32)) -> Option<BoundingBox> {
    let bbox = table.bounding_box?;
    let columns = table.cells.first()?.len();
    (0.0 <= bbox.x0
        && bbox.x0 < bbox.x1
        && bbox.x1 <= f64::from(width)
        && 0.0 <= bbox.y0
        && bbox.y0 < bbox.y1
        && bbox.y1 <= f64::from(height)
        && table.cells.len() >= 3
        && columns >= 2
        && table
            .cells
            .iter()
            .all(|row| row.len() == columns && row.iter().all(|cell| !cell.trim().is_empty())))
    .then_some(bbox)
}

fn exact_table_markdown(result: &ExtractedDocument, first: &Table, second: &Table) -> bool {
    let a = first.markdown.trim();
    let b = second.markdown.trim();
    !a.is_empty()
        && !b.is_empty()
        && result.content.trim().strip_prefix(a).is_some_and(|tail| {
            let after_space = tail.trim_start();
            tail.len() > after_space.len() && after_space == b
        })
}

fn partition_words(elements: &[OcrElement], boxes: [BoundingBox; 2], dims: (u32, u32)) -> Option<[Vec<HocrWord>; 2]> {
    let mut tables = [Vec::new(), Vec::new()];
    for element in elements.iter().filter(|element| element.level == OcrElementLevel::Word) {
        let rect = iterator_rect(element, dims)?;
        let index = boxes
            .iter()
            .position(|bbox| bbox.x0 <= rect.x0 && rect.x1 <= bbox.x1 && bbox.y0 <= rect.y0 && rect.y1 <= bbox.y1)?;
        tables[index].push(iterator_word(element, boxes[index])?);
    }
    Some(tables)
}

fn primary_inside_tables(
    elements: &[OcrElement],
    boxes: [BoundingBox; 2],
    dims: (u32, u32),
    correction: u16,
    thresholds: &OcrQualityThresholds,
) -> bool {
    let rotated = boxes.map(|bbox| {
        let undo = |x, y| undo_auto_rotate_point(x, y, correction, f64::from(dims.0), f64::from(dims.1));
        let (x0, y0) = undo(bbox.x0, bbox.y0);
        let (x1, y1) = undo(bbox.x1, bbox.y1);
        BoundingBox {
            x0: x0.min(x1),
            y0: y0.min(y1),
            x1: x0.max(x1),
            y1: y0.max(y1),
        }
    });
    let mut words = elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .peekable();
    words.peek().is_some()
        && words.all(|word| {
            let Some(rect) = iterator_rect(word, (dims.1, dims.0)) else {
                return false;
            };
            if !super::recognition::protected(
                word,
                thresholds,
                thresholds
                    .min_meaningful_word_len
                    .min(super::recognition::MIN_UNIT_LETTERS),
            ) {
                return true;
            }
            rotated
                .iter()
                .any(|b| b.x0 <= rect.x0 && rect.x1 <= b.x1 && b.y0 <= rect.y0 && rect.y1 <= b.y1)
        })
}

fn iterator_rect(element: &OcrElement, (w, h): (u32, u32)) -> Option<BoundingBox> {
    let OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    } = element.geometry
    else {
        return None;
    };
    let (right, bottom) = (left.checked_add(width)?, top.checked_add(height)?);
    (element.backend_metadata.get("backend")?.as_str()? == "tesseract-iterator"
        && (0.0..=1.0).contains(&element.confidence.recognition)
        && left < right
        && top < bottom
        && right <= w
        && bottom <= h)
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
    use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};

    fn two_table_candidate() -> (ExtractedDocument, ExtractedDocument) {
        let mut candidate = super::super::tests::table_candidate();
        candidate.tables[0].page_number = 1;
        candidate.tables[0].markdown = table_to_markdown(&candidate.tables[0].cells);
        let mut second = candidate.tables[0].clone();
        let bounds = second.bounding_box.as_mut().unwrap();
        bounds.y0 += 180.;
        bounds.y1 += 180.;
        candidate.tables.push(second);
        candidate.content = format!("{} {}", candidate.content, candidate.content);
        candidate
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), 400.into());
        candidate.metadata.additional.insert("word_count".into(), 18.into());
        let words = candidate.ocr_elements.as_mut().unwrap();
        for word in words.iter_mut() {
            word.page_number = 1;
        }
        let mut lower = words.clone();
        for word in &mut lower {
            if let OcrBoundingGeometry::Rectangle { top, .. } = &mut word.geometry {
                *top += 180;
            }
        }
        words.extend(lower);
        let mut supported = words[0].clone();
        supported.text = "49".into();
        supported.confidence.recognition = 0.95;
        supported.geometry = OcrBoundingGeometry::Rectangle {
            left: 300,
            top: 70,
            width: 10,
            height: 10,
        };
        let mut primary = ExtractedDocument {
            content: "49".into(),
            ocr_elements: Some(vec![supported]),
            ..Default::default()
        };
        primary
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), 400.into());
        primary
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), 200.into());
        primary.metadata.additional.insert("word_count".into(), 1.into());
        (candidate, primary)
    }

    #[test]
    fn protects_credible_primary_words_outside_both_rotated_tables() {
        let (candidate, mut primary) = two_table_candidate();
        let prove = |primary: &ExtractedDocument| {
            complete_text(&candidate, primary, &Default::default(), &Default::default(), 0.5)
        };
        assert_eq!(prove(&primary).as_deref(), Some(candidate.content.as_str()));
        let word = &mut primary.ocr_elements.as_mut().unwrap()[0];
        let OcrBoundingGeometry::Rectangle { left, .. } = &mut word.geometry else {
            unreachable!()
        };
        *left = 200;
        assert_eq!(prove(&primary), None);
        primary.ocr_elements.as_mut().unwrap()[0].confidence.recognition = 0.10;
        assert_eq!(prove(&primary).as_deref(), Some(candidate.content.as_str()));
        primary
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), 201.into());
        assert_eq!(prove(&primary), None);
    }

    #[test]
    fn accepts_primary_evidence_that_covers_a_filtered_output_and_refuses_uncovered_text() {
        let (candidate, mut primary) = two_table_candidate();
        let prove = |primary: &ExtractedDocument| {
            complete_text(&candidate, primary, &Default::default(), &Default::default(), 0.5)
        };
        let mut dropped = primary.ocr_elements.as_ref().unwrap()[0].clone();
        dropped.text = "zz".into();
        dropped.confidence.recognition = 0.10;
        primary.ocr_elements.as_mut().unwrap().push(dropped.clone());
        assert_eq!(prove(&primary).as_deref(), Some(candidate.content.as_str()));
        let mut confident_outside = dropped;
        confident_outside.confidence.recognition = 0.95;
        confident_outside.text = "77".into();
        let OcrBoundingGeometry::Rectangle { left, .. } = &mut confident_outside.geometry else {
            unreachable!()
        };
        *left = 200;
        primary.ocr_elements.as_mut().unwrap().push(confident_outside);
        assert_eq!(prove(&primary), None);
        primary.ocr_elements.as_mut().unwrap().pop();
        primary.content = "49 50".into();
        primary.metadata.additional.insert("word_count".into(), 2.into());
        assert_eq!(prove(&primary), None);
    }

    #[test]
    fn refuses_overlapping_tables_unowned_candidate_words_and_exhausted_budget() {
        let (candidate, primary) = two_table_candidate();
        let prove = |candidate: &ExtractedDocument, limits: &SecurityLimits| {
            complete_text(candidate, &primary, &Default::default(), limits, 0.5)
        };
        assert!(prove(&candidate, &Default::default()).is_some());

        let mut overlap = candidate.clone();
        overlap.tables[1].bounding_box.as_mut().unwrap().y0 = 150.;
        assert_eq!(prove(&overlap, &Default::default()), None);
        let mut foreign_word = candidate.clone();
        foreign_word.ocr_elements.as_mut().unwrap()[0].page_number = 2;
        assert_eq!(prove(&foreign_word, &Default::default()), None);
        let mut displaced = candidate.clone();
        let word = &mut displaced.ocr_elements.as_mut().unwrap()[0];
        let OcrBoundingGeometry::Rectangle { top, .. } = &mut word.geometry else {
            unreachable!()
        };
        *top += 180;
        assert_eq!(prove(&displaced, &Default::default()), None);
        let mut forged_markdown = candidate.clone();
        forged_markdown.tables[0].markdown = "Other text".into();
        forged_markdown.content = format!(
            "{}\n\n{}",
            forged_markdown.tables[0].markdown, forged_markdown.tables[1].markdown
        );
        assert_eq!(prove(&forged_markdown, &Default::default()), None);
        let tight = SecurityLimits {
            max_table_cells: 11,
            ..Default::default()
        };
        assert_eq!(prove(&candidate, &tight), None);
    }
    #[test]
    fn selector_keeps_primary_with_unproved_counts_rich_content_or_boundary_crossing() {
        use crate::extractors::pdf::ocr::orientation_retry::OrientationRetry;
        use crate::plugins::ConfidenceSemantics;
        let (candidate, primary) = two_table_candidate();
        let select = |primary| {
            OrientationRetry::new(&Default::default(), Default::default())
                .select_implicit(
                    Ok(candidate.clone()),
                    primary,
                    &Default::default(),
                    ConfidenceSemantics::Legibility { scale_max: 100.0 },
                    0.5,
                )
                .unwrap()
        };
        assert_eq!(select(primary.clone()).tables.len(), 2);
        let mut incomplete = primary.clone();
        incomplete.metadata.additional.insert("word_count".into(), 2.into());
        let mut rich = primary.clone();
        let mut document = crate::extractors::pdf::ocr::document::flat_ocr_page_document("49");
        document.elements[0].anchor = Some("retained-link-target".into());
        rich.ocr_internal_document = Some(document);
        let mut crosses = primary.clone();
        crosses.ocr_elements.as_mut().unwrap()[0].geometry = OcrBoundingGeometry::Rectangle {
            left: 215,
            top: 70,
            width: 12,
            height: 10,
        };
        let mut incomplete_iterator = primary;
        incomplete_iterator.ocr_elements = None;
        for unproved in [incomplete, rich, crosses, incomplete_iterator] {
            let result = select(unproved);
            assert!(result.tables.is_empty());
            assert_eq!(result.content, "49");
        }
    }

    #[test]
    fn scales_both_recovered_table_boxes_into_the_same_raster() {
        let (mut candidate, _) = two_table_candidate();
        super::super::scale_recovered_table_to_raster(&mut candidate, (100, 100));
        let boxes = candidate
            .tables
            .iter()
            .map(|table| table.bounding_box.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            boxes,
            [
                BoundingBox {
                    x0: 10.,
                    y0: 10.,
                    x1: 90.,
                    y1: 40.
                },
                BoundingBox {
                    x0: 10.,
                    y0: 55.,
                    x1: 90.,
                    y1: 85.
                },
            ]
        );
    }
}
