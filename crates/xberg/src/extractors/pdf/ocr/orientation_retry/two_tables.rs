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
const PROSE_WORK_PASSES: usize = 3;

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
    {
        return None;
    }
    let read = result.ocr_elements.as_deref()?;
    let primary = original.ocr_elements.as_deref()?;
    let remaining_work = remaining_work([first, second], [result, original], read, primary, limits)?;
    if !plain_page_payload(result)
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
    let prose = prose_bands(result, [first, second])?;
    if !structured_prose_matches(result, prose) {
        return None;
    }
    let (tables, bands) = partition_words(read, boxes, dims)?;
    let prose_words = bands.iter().flatten().copied().collect::<Vec<_>>();
    if prose_work(&prose_words, primary) > remaining_work {
        return None;
    }
    let [a, b] = tables;
    if !table_matches_words(&first.cells, &a, row_threshold) || !table_matches_words(&second.cells, &b, row_threshold) {
        return None;
    }
    if !bands
        .iter()
        .zip(prose)
        .all(|(words, text)| super::prose::reads_in_order(words, text, row_threshold))
    {
        return None;
    }
    primary_inside_tables(primary, &prose_words, boxes, dims, correction, thresholds)
        .then(|| flattened_page([first, second], prose))
}

fn plain_page_payload(result: &ExtractedDocument) -> bool {
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
            doc.elements.iter().all(plain_prose_element)
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

fn plain_prose_element(element: &crate::types::internal::InternalElement) -> bool {
    use crate::ocr::hocr_parser::*;
    use crate::types::internal::ElementKind;
    matches!(element.kind, ElementKind::OcrText { .. } | ElementKind::Paragraph)
        && element.depth == 0
        && element.layer == crate::types::ContentLayer::Body
        && element.anchor.is_none()
        && element.annotations.is_empty()
        && element.attributes.as_ref().is_none_or(|attributes| {
            attributes.iter().all(|(name, value)| match name.as_str() {
                HOCR_BOLD_FRACTION_ATTRIBUTE | HOCR_ITALIC_FRACTION_ATTRIBUTE => value.parse::<f64>().ok() == Some(0.0),
                HOCR_BLOCK_ID_ATTRIBUTE
                | HOCR_FONT_SIZE_ATTRIBUTE
                | HOCR_TEXT_ANGLE_ATTRIBUTE
                | HOCR_X_HEIGHT_ATTRIBUTE
                | HOCR_X_ASCENDERS_ATTRIBUTE
                | HOCR_X_DESCENDERS_ATTRIBUTE
                | HOCR_BASELINE_SLOPE_ATTRIBUTE
                | HOCR_BASELINE_CONST_ATTRIBUTE
                | HOCR_LINE_FONT_SIZES_ATTRIBUTE
                | HOCR_LINE_X_HEIGHTS_ATTRIBUTE
                | HOCR_FONT_NAME_ATTRIBUTE => true,
                _ => false,
            })
        })
}

fn structured_prose_matches(result: &ExtractedDocument, bands: [&str; 3]) -> bool {
    result.ocr_internal_document.as_ref().is_none_or(|document| {
        document
            .elements
            .iter()
            .flat_map(|element| element.text.split_whitespace())
            .eq(bands.into_iter().flat_map(str::split_whitespace))
    })
}

fn remaining_work(
    tables: [&Table; 2],
    reads: [&ExtractedDocument; 2],
    words: &[OcrElement],
    primary: &[OcrElement],
    limits: &SecurityLimits,
) -> Option<usize> {
    let elements = words.len().saturating_add(primary.len());
    if elements > MAX_WORDS {
        return None;
    }
    let mut bytes = reads
        .iter()
        .map(|read| read.content.len())
        .fold(0usize, usize::saturating_add);
    if let Some(document) = reads[0].ocr_internal_document.as_ref() {
        if document.elements.len() > MAX_WORDS {
            return None;
        }
        for element in &document.elements {
            bytes = bytes.saturating_add(element.text.len());
            if let Some(attributes) = &element.attributes {
                if attributes.len() > MAX_WORDS {
                    return None;
                }
                bytes = attributes.iter().fold(bytes, |sum, (key, value)| {
                    sum.saturating_add(key.len()).saturating_add(value.len())
                });
            }
        }
    }
    let mut cells = 0usize;
    for table in tables {
        bytes = bytes.saturating_add(table.markdown.len());
        if table.cells.len() > MAX_CELLS {
            return None;
        }
        for row in &table.cells {
            cells = cells.saturating_add(row.len());
            if cells > MAX_CELLS.min(limits.max_table_cells) {
                return None;
            }
            bytes = row.iter().fold(bytes, |sum, cell| sum.saturating_add(cell.len()));
        }
    }
    bytes = words
        .iter()
        .chain(primary)
        .fold(bytes, |sum, word| sum.saturating_add(word.text.len()));
    if bytes > MAX_BYTES.min(limits.max_content_size) {
        return None;
    }
    MAX_WORK
        .min(limits.max_iterations)
        .checked_sub(elements.saturating_mul(cells.saturating_add(1)).saturating_add(bytes))
}

fn prose_work(prose: &[&OcrElement], primary: &[OcrElement]) -> usize {
    let prose_bytes = prose
        .iter()
        .fold(0usize, |sum, word| sum.saturating_add(word.text.len()));
    let primary_bytes = primary
        .iter()
        .fold(0usize, |sum, word| sum.saturating_add(word.text.len()));
    prose
        .len()
        .saturating_mul(prose.len())
        .saturating_mul(PROSE_WORK_PASSES)
        .saturating_add(primary.len().saturating_mul(prose_bytes))
        .saturating_add(prose.len().saturating_mul(primary_bytes))
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

type PartitionedWords<'a> = ([Vec<HocrWord>; 2], [Vec<&'a OcrElement>; 3]);

fn partition_words(elements: &[OcrElement], boxes: [BoundingBox; 2], dims: (u32, u32)) -> Option<PartitionedWords<'_>> {
    let mut tables = [Vec::new(), Vec::new()];
    let mut prose = [Vec::new(), Vec::new(), Vec::new()];
    for element in elements.iter().filter(|element| element.level == OcrElementLevel::Word) {
        let rect = iterator_rect(element, dims)?;
        if let Some(index) = boxes.iter().position(|bbox| contains(*bbox, rect)) {
            tables[index].push(iterator_word(element, boxes[index])?);
        } else {
            let band = if rect.y1 <= boxes[0].y0 {
                0
            } else if rect.y0 >= boxes[0].y1 && rect.y1 <= boxes[1].y0 {
                1
            } else if rect.y0 >= boxes[1].y1 {
                2
            } else {
                return None;
            };
            prose[band].push(element);
        }
    }
    Some((tables, prose))
}

fn primary_inside_tables(
    elements: &[OcrElement],
    prose: &[&OcrElement],
    boxes: [BoundingBox; 2],
    dims: (u32, u32),
    correction: u16,
    thresholds: &OcrQualityThresholds,
) -> bool {
    let rotated = boxes.map(|bbox| original_frame(bbox, dims, correction));
    let mut used = std::collections::HashSet::new();
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
            rotated.iter().any(|b| contains(*b, rect))
                || claim_primary_prose(word, rect, prose, dims, correction, &mut used)
        })
}

fn contains(outer: BoundingBox, inner: BoundingBox) -> bool {
    outer.x0 <= inner.x0 && inner.x1 <= outer.x1 && outer.y0 <= inner.y0 && inner.y1 <= outer.y1
}

fn original_frame(bbox: BoundingBox, dims: (u32, u32), correction: u16) -> BoundingBox {
    let undo = |x, y| undo_auto_rotate_point(x, y, correction, f64::from(dims.0), f64::from(dims.1));
    let (x0, y0) = undo(bbox.x0, bbox.y0);
    let (x1, y1) = undo(bbox.x1, bbox.y1);
    BoundingBox {
        x0: x0.min(x1),
        y0: y0.min(y1),
        x1: x0.max(x1),
        y1: y0.max(y1),
    }
}

fn claim_primary_prose(
    primary: &OcrElement,
    rect: BoundingBox,
    prose: &[&OcrElement],
    dims: (u32, u32),
    correction: u16,
    used: &mut std::collections::HashSet<usize>,
) -> bool {
    let center_inside = |outer: BoundingBox, inner: BoundingBox| {
        (outer.x0..=outer.x1).contains(&((inner.x0 + inner.x1) / 2.0))
            && (outer.y0..=outer.y1).contains(&((inner.y0 + inner.y1) / 2.0))
    };
    let mut matching = prose.iter().enumerate().filter(|(_, word)| {
        if primary.page_number != word.page_number || !primary.text.split_whitespace().eq(word.text.split_whitespace())
        {
            return false;
        }
        iterator_rect(word, dims).is_some_and(|candidate| {
            let candidate = original_frame(candidate, dims, correction);
            center_inside(rect, candidate) && center_inside(candidate, rect)
        })
    });
    let Some((at, _)) = matching.next() else { return false };
    matching.next().is_none() && used.insert(at)
}

fn prose_bands<'a>(result: &'a ExtractedDocument, tables: [&Table; 2]) -> Option<[&'a str; 3]> {
    let cell_words = tables
        .iter()
        .flat_map(|table| table.cells.iter().flatten().flat_map(|cell| cell.split_whitespace()));
    let content = result.content.trim();
    let bands =
        if cell_words.clone().eq(content.split_whitespace()) || exact_table_markdown(result, tables[0], tables[1]) {
            ["", "", ""]
        } else {
            let (above, rest) = content.split_once(tables[0].markdown.trim())?;
            let (between, below) = rest.split_once(tables[1].markdown.trim())?;
            [above, between, below]
        };
    let count = cell_words.count() + bands.iter().map(|text| text.split_whitespace().count()).sum::<usize>();
    (usize::try_from(word_count_of(&result.metadata.additional)?).ok()? == count).then_some(bands)
}

fn flattened_page(tables: [&Table; 2], bands: [&str; 3]) -> String {
    bands[0]
        .split_whitespace()
        .chain(table_words(tables[0]))
        .chain(bands[1].split_whitespace())
        .chain(table_words(tables[1]))
        .chain(bands[2].split_whitespace())
        .collect::<Vec<_>>()
        .join(" ")
}

fn table_words(table: &Table) -> impl Iterator<Item = &str> {
    table.cells.iter().flatten().flat_map(|cell| cell.split_whitespace())
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
    fn table_only_reads_do_not_pay_for_absent_prose_matching() {
        let (candidate, mut primary) = two_table_candidate();
        let word = primary.ocr_elements.as_ref().unwrap()[0].clone();
        primary.ocr_elements = Some(vec![word; 600]);
        primary.content = vec!["49"; 600].join(" ");
        primary.metadata.additional.insert("word_count".into(), 600.into());
        assert_eq!(
            complete_text(&candidate, &primary, &Default::default(), &Default::default(), 0.5).as_deref(),
            Some(candidate.content.as_str()),
            "bounded primary table evidence needs no quadratic prose search"
        );
    }

    fn covering_primary(content: &str, evidence: &[&str]) -> ExtractedDocument {
        let (_, mut primary) = two_table_candidate();
        let template = primary.ocr_elements.as_ref().unwrap()[0].clone();
        primary.ocr_elements = Some(
            evidence
                .iter()
                .map(|text| {
                    let mut word = template.clone();
                    word.text = (*text).into();
                    word
                })
                .collect(),
        );
        primary.content = content.into();
        let reported = content.split_whitespace().count();
        primary.metadata.additional.insert("word_count".into(), reported.into());
        primary
    }

    #[test]
    fn covering_primary_counts_tokens_and_bounds_the_extra_iterator_words() {
        use super::super::recognition::primary_covers_output;
        let four = ["aa", "bb", "cc", "dd"];
        assert!(primary_covers_output(&covering_primary("aa bb cc dd", &four)));
        let at_cap = ["aa", "bb", "cc", "dd", "zz"];
        assert!(primary_covers_output(&covering_primary("aa bb cc dd", &at_cap)));
        let over_cap = ["aa", "bb", "cc", "dd", "zz", "yy"];
        assert!(!primary_covers_output(&covering_primary("aa bb cc dd", &over_cap)));
        let mut under_reported = covering_primary("aa bb cc dd", &four);
        under_reported.metadata.additional.insert("word_count".into(), 5.into());
        assert!(!primary_covers_output(&under_reported));
        let mut missing = covering_primary("aa bb cc dd", &four);
        missing.content = "aa bb cc ee".into();
        assert!(!primary_covers_output(&missing));
        let duplicate = covering_primary("aa aa bb cc", &["aa", "bb", "cc", "dd"]);
        assert!(
            !primary_covers_output(&duplicate),
            "a repeated token needs a repeated word"
        );
        assert!(primary_covers_output(&covering_primary(
            "aa aa bb cc",
            &["aa", "aa", "bb", "cc"]
        )));
    }

    #[test]
    fn a_confident_primary_word_outside_both_tables_still_blocks_a_covered_read() {
        let (candidate, mut primary) = two_table_candidate();
        let prove = |primary: &ExtractedDocument| {
            complete_text(&candidate, primary, &Default::default(), &Default::default(), 0.5)
        };
        let mut outside = primary.ocr_elements.as_ref().unwrap()[0].clone();
        outside.confidence.recognition = 0.95;
        outside.text = "77".into();
        let OcrBoundingGeometry::Rectangle { left, .. } = &mut outside.geometry else {
            unreachable!()
        };
        *left = 200;
        primary.content = "49 77".into();
        primary.metadata.additional.insert("word_count".into(), 2.into());
        primary.ocr_elements.as_mut().unwrap().push(outside);
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

    fn mixed_two_table_candidate() -> (ExtractedDocument, ExtractedDocument) {
        let (mut candidate, mut primary) = two_table_candidate();
        candidate.content = format!(
            "Overview 7\n\n{}\n\nSubtotal 8\n\n{}\n\nBalance 9",
            candidate.tables[0].markdown.trim(),
            candidate.tables[1].markdown.trim()
        );
        candidate.metadata.additional.insert("word_count".into(), 24.into());
        let prototype = candidate.ocr_elements.as_ref().unwrap()[0].clone();
        for (text, left, top) in [
            ("Overview", 30, 10),
            ("7", 60, 10),
            ("Subtotal", 30, 180),
            ("8", 60, 180),
            ("Balance", 30, 370),
            ("9", 60, 370),
        ] {
            let mut word = prototype.clone();
            word.text = text.into();
            word.geometry = OcrBoundingGeometry::Rectangle {
                left,
                top,
                width: 10,
                height: 10,
            };
            candidate.ocr_elements.as_mut().unwrap().push(word);
        }
        let mut heading = prototype;
        heading.text = "Overview".into();
        heading.confidence.recognition = 0.95;
        heading.geometry = OcrBoundingGeometry::Rectangle {
            left: 10,
            top: 160,
            width: 10,
            height: 10,
        };
        primary.ocr_elements.as_mut().unwrap().push(heading);
        primary.content = "49 Overview".into();
        primary.metadata.additional.insert("word_count".into(), 2.into());
        (candidate, primary)
    }

    #[test]
    fn mixed_two_tables_prove_prose_above_between_and_below_the_grids() {
        let (candidate, primary) = mixed_two_table_candidate();
        assert_eq!(
            complete_text(&candidate, &primary, &Default::default(), &Default::default(), 0.5).as_deref(),
            Some(
                "Overview 7 Cash 100 31,290 Rent 107 77,078 Fuel 114 71,433 Subtotal 8 Cash 100 31,290 Rent 107 77,078 Fuel 114 71,433 Balance 9"
            )
        );
    }

    #[test]
    fn mixed_two_tables_keep_only_matching_plain_ocr_prose_payload() {
        use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
        let (mut candidate, primary) = mixed_two_table_candidate();
        let mut document = InternalDocument::new("ocr");
        for text in ["Overview 7", "Subtotal 8", "Balance 9"] {
            document.push_element(InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Block,
                },
                text,
                0,
            ));
        }
        candidate.ocr_internal_document = Some(document);
        let prove =
            |read: &ExtractedDocument| complete_text(read, &primary, &Default::default(), &Default::default(), 0.5);
        assert!(
            prove(&candidate).is_some(),
            "independently owned plain prose remains structured"
        );
        let mut changed = candidate.clone();
        changed.ocr_internal_document.as_mut().unwrap().elements[0].text = "Other 7".into();
        assert!(prove(&changed).is_none(), "all output representations must agree");
        let mut anchored = candidate.clone();
        anchored.ocr_internal_document.as_mut().unwrap().elements[0].anchor = Some("owned-anchor".into());
        assert!(
            prove(&anchored).is_none(),
            "rich payload cannot use the plain-prose proof"
        );
    }

    #[test]
    fn mixed_two_tables_refuse_unowned_missing_crossing_and_reordered_prose() {
        let (candidate, primary) = mixed_two_table_candidate();
        let prove = |candidate: &ExtractedDocument| {
            complete_text(candidate, &primary, &Default::default(), &Default::default(), 0.5)
        };
        assert!(
            prove(&candidate).is_some(),
            "valid mixed evidence must reach the guards"
        );
        let mut reordered = candidate.clone();
        reordered.content = reordered.content.replace("Overview 7", "7 Overview");
        let mut missing = candidate.clone();
        missing
            .ocr_elements
            .as_mut()
            .unwrap()
            .retain(|word| word.text != "Subtotal");
        let mut crossing = candidate.clone();
        let word = crossing
            .ocr_elements
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|word| word.text == "Subtotal")
            .unwrap();
        word.geometry = OcrBoundingGeometry::Rectangle {
            left: 30,
            top: 155,
            width: 10,
            height: 10,
        };
        let mut beside = candidate.clone();
        let word = beside
            .ocr_elements
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|word| word.text == "Subtotal")
            .unwrap();
        word.geometry = OcrBoundingGeometry::Rectangle {
            left: 190,
            top: 90,
            width: 10,
            height: 10,
        };
        for (reason, invalid) in [
            ("reordered", reordered),
            ("missing", missing),
            ("crossing", crossing),
            ("beside", beside),
        ] {
            assert!(prove(&invalid).is_none(), "{reason}");
        }
    }

    #[test]
    fn mixed_two_tables_preserve_each_protected_primary_prose_occurrence() {
        let (candidate, primary) = mixed_two_table_candidate();
        let prove = |primary: &ExtractedDocument| {
            complete_text(&candidate, primary, &Default::default(), &Default::default(), 0.5)
        };
        assert!(
            prove(&primary).is_some(),
            "valid primary prose must reach occurrence protection"
        );
        let mut changed = primary.clone();
        changed.ocr_elements.as_mut().unwrap()[1].text = "Other".into();
        changed.content = "49 Other".into();
        let mut displaced = primary.clone();
        displaced.ocr_elements.as_mut().unwrap()[1].geometry = OcrBoundingGeometry::Rectangle {
            left: 200,
            top: 190,
            width: 10,
            height: 10,
        };
        let mut duplicate = primary.clone();
        let word = duplicate.ocr_elements.as_ref().unwrap()[1].clone();
        duplicate.ocr_elements.as_mut().unwrap().push(word);
        duplicate.content.push_str(" Overview");
        duplicate.metadata.additional.insert("word_count".into(), 3.into());
        for (reason, invalid) in [("changed", changed), ("displaced", displaced), ("duplicate", duplicate)] {
            assert!(prove(&invalid).is_none(), "{reason}");
        }
    }
}
