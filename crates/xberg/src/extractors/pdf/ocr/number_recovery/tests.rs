use super::*;
use crate::pdf::hierarchy::SegmentData;

fn segment(text: &str) -> SegmentData {
    SegmentData {
        text: text.into(),
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 20.0,
        font_size: 20.0,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y: 0.0,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

fn paragraph(authoritative: &str, located: &str) -> PdfParagraph {
    let line = PdfLine {
        segments: vec![segment(located)],
        native_scripts: Vec::new(),
        baseline_y: 0.0,
        dominant_font_size: 20.0,
        is_bold: false,
        is_monospace: false,
    };
    PdfParagraph {
        text: authoritative.into(),
        word_count: 1,
        lines: vec![line],
        dominant_font_size: 20.0,
        heading_level: None,
        is_bold: false,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: None,
    }
}

fn number(text: &str, x: f32) -> Number {
    Number {
        text: text.into(),
        bbox: Rect::new(x, 0.0, 10.0, 20.0),
        baseline: 0.0,
    }
}

#[test]
fn should_keep_authoritative_text_whose_located_segments_are_incomplete() {
    let mut paragraphs = [paragraph("VOU retained tail", "VOU")];
    assert!(!insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(paragraphs[0].text, "VOU retained tail");
}

#[test]
fn should_not_invent_an_owner_when_two_ocr_lines_overlap() {
    let mut paragraphs = [paragraph("VOU", "VOU"), paragraph("VOU", "VOU")];
    assert!(!insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(
        super::super::document::ocr_paragraphs_plain_text(&paragraphs),
        "VOU\n\nVOU"
    );
}

#[test]
fn should_not_duplicate_a_number_ocr_already_read_on_the_line() {
    let mut paragraphs = [paragraph("០", "០")];
    assert!(!insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(paragraphs[0].text, "០");
}

#[test]
fn should_keep_two_independently_verified_occurrences_on_one_line() {
    let mut paragraphs = [paragraph("VOU", "VOU")];
    assert!(insert(
        vec![number("០", 30.0), number("១", 60.0)],
        &mut paragraphs,
        (1.0, 1.0),
        100
    ));
    let output = super::super::document::ocr_paragraphs_plain_text(&paragraphs);
    assert_eq!(output, "VOU ០ ១");
    assert_eq!(paragraphs[0].word_count, 3);
}

#[test]
fn should_refuse_overlapping_claims_for_one_painted_occurrence() {
    let mut paragraphs = [paragraph("VOU", "VOU")];
    let mut shifted = number("០", 30.0);
    shifted.bbox.y += 0.5;
    shifted.baseline += 0.5;
    assert!(!insert(
        vec![number("០", 30.0), shifted],
        &mut paragraphs,
        (1.0, 1.0),
        100
    ));
    assert_eq!(paragraphs[0].text, "VOU");
}

#[test]
fn should_keep_original_prose_with_descending_x_reading_order() {
    let mut paragraphs = [paragraph("right left", "right")];
    paragraphs[0].lines[0].segments[0].x = 80.0;
    let mut left = segment("left");
    left.width = 40.0;
    paragraphs[0].lines[0].segments.push(left);
    assert!(!insert(vec![number("០", 60.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(paragraphs[0].text, "right left");
    assert_eq!(paragraphs[0].lines[0].segments[0].text, "right");
}

#[test]
fn should_keep_adjacent_native_digits_in_one_numeric_candidate() {
    let chars = [('ប', 0.0), ('១', 20.0), ('២', 40.0), ('ប', 60.0)]
        .into_iter()
        .map(|(character, x)| TextChar::simple(character, Rect::new(x, 0.0, 20.0, 30.0), "Khmer".into(), 30.0))
        .collect();
    let numbers = candidates(chars);
    assert_eq!(numbers.len(), 1);
    assert_eq!(numbers[0].text, "១២");
    assert_eq!(numbers[0].bbox, Rect::new(20.0, 0.0, 40.0, 30.0));
}

#[test]
fn should_honor_the_actual_forced_page_number() {
    let config = ExtractionConfig {
        ocr: Some(crate::core::config::OcrConfig::default()),
        force_ocr_pages: Some(vec![2]),
        ..Default::default()
    };
    assert!(eligible(&config, 0));
    assert!(!eligible(&config, 1));
}

#[test]
fn should_refuse_crops_outside_the_page_or_live_memory_budget() {
    let raster = image::DynamicImage::new_rgb8(100, 100);
    let media = Rect::new(0.0, 0.0, 100.0, 100.0);
    let valid = Rect::new(20.0, 20.0, 10.0, 10.0);
    assert!(crop_number(&raster, media, valid, 0).is_none());
    assert!(crop_number(&raster, media, valid, 100_000).is_some());
    assert!(crop_number(&raster, media, Rect::new(-1.0, 20.0, 10.0, 10.0), 100_000).is_none());
    assert!(crop_number(&raster, media, Rect::new(95.0, 20.0, 10.0, 10.0), 100_000).is_none());
}

fn primary_read() -> crate::types::ExtractedDocument {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    let mut element = InternalElement::text(ElementKind::Paragraph, "VOU", 0);
    element.bbox = Some(crate::types::BoundingBox {
        x0: 10.0,
        y0: 10.0,
        x1: 20.0,
        y1: 20.0,
    });
    let mut doc = InternalDocument::new("ocr");
    doc.elements.push(element);
    let mut read = crate::types::ExtractedDocument {
        content: "VOU".into(),
        ocr_internal_document: Some(doc),
        ..Default::default()
    };
    read.metadata.additional.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
        100.into(),
    );
    read.metadata.additional.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
        100.into(),
    );
    read
}

#[test]
fn should_bind_positive_scale_only_evidence_to_the_selected_geometry() {
    let mut read = primary_read();
    assert!(ReadProof::capture(&read, false).is_none());
    let proof = ReadProof::capture(&read, true).expect("observed primary");
    assert!(proof.matches(&read));
    read.ocr_internal_document.as_mut().unwrap().elements[0]
        .bbox
        .as_mut()
        .unwrap()
        .x0 = 11.0;
    assert!(
        !proof.matches(&read),
        "same text with different geometry cannot reuse the observation"
    );
}

#[test]
fn should_refuse_an_oversized_selected_read_proof() {
    let mut read = primary_read();
    read.content = "x".repeat(300 * 1024);
    assert!(ReadProof::capture(&read, true).is_none());
}
