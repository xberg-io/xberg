use super::super::document::{layer_lines_to_put_first, put_layer_first_in_document};
use super::*;
use crate::types::document_structure::{AnnotationKind, TextAnnotation};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

#[test]
fn a_second_native_occurrence_without_an_ocr_box_is_retained() {
    let mut layer = MINUTES.to_vec();
    layer.push("Total 100");
    let rows = (0..MINUTES.len()).collect::<Vec<_>>();
    let merged = merge_at_rows(&layer, &MINUTES.join("\n"), &rows);
    assert_eq!(count_line(&merged, "Total 100"), 2, "{merged}");
}

#[test]
fn an_owned_copy_does_not_delete_a_different_equal_line_and_its_annotation() {
    let mut ocr = MINUTES.to_vec();
    ocr[3] = "Total 1OO";
    ocr.insert(0, "Total 1OO");
    let text = ocr.join("\n");
    let boxes = page_boxes(&MINUTES, &lines_at_rows(&text, &[9, 0, 1, 2, 3, 4, 5, 6, 7]));
    let first = layer_lines_to_put_first(
        &MINUTES.join("\n"),
        &text,
        false,
        &OcrQualityThresholds::default(),
        Some(&boxes),
        crate::extractors::pdf::ocr::document::PlainOcrRows::every(),
    )
    .expect("the spatially owned misread requires restoration");
    assert_eq!(first.lines, "Total 100");
    let annotation = TextAnnotation {
        start: 0,
        end: 5,
        kind: AnnotationKind::Bold,
    };
    let mut document = InternalDocument::new("pdf");
    for (index, text) in ocr.iter().enumerate() {
        let mut element = InternalElement::text(ElementKind::Paragraph, *text, 0);
        element.page = Some(1);
        if index == 0 {
            element.annotations.push(annotation.clone());
        }
        document.push_element(element);
    }
    put_layer_first_in_document(&mut document, Some(1), &first);
    assert!(
        document
            .elements
            .iter()
            .any(|element| { element.text == "Total 1OO" && element.annotations == [annotation.clone()] })
    );
}

fn equal_ocr_occurrences(with_boxes: bool) -> (InternalDocument, super::super::document::LayerFirst) {
    let mut ocr = MINUTES.to_vec();
    ocr[3] = "Total 1OO";
    ocr.insert(0, "Total 1OO");
    let text = ocr.join("\n");
    let rows = [9, 0, 1, 2, 3, 4, 5, 6, 7];
    let boxes = page_boxes(&MINUTES, &lines_at_rows(&text, &rows));
    let first = layer_lines_to_put_first(
        &MINUTES.join("\n"),
        &text,
        false,
        &OcrQualityThresholds::default(),
        Some(&boxes),
        crate::extractors::pdf::ocr::document::PlainOcrRows::every(),
    )
    .unwrap();
    let mut document = InternalDocument::new("pdf");
    for (index, text) in ocr.iter().enumerate() {
        let mut element = InternalElement::text(ElementKind::Paragraph, *text, 0);
        element.page = Some(1);
        if with_boxes {
            let bbox = row_box(rows[index]);
            element.bbox = Some(crate::types::BoundingBox {
                x0: f64::from(bbox.left),
                y0: f64::from(bbox.top),
                x1: f64::from(bbox.right),
                y1: f64::from(bbox.bottom),
            });
        }
        element.annotations.push(TextAnnotation {
            start: 0,
            end: 5,
            kind: if index == 0 {
                AnnotationKind::Bold
            } else {
                AnnotationKind::Italic
            },
        });
        document.push_element(element);
    }
    (document, first)
}

#[test]
fn should_preserve_both_equal_ocr_alternatives_and_their_geometry() {
    let (mut document, first) = equal_ocr_occurrences(true);
    let original = document.elements.clone();
    put_layer_first_in_document(&mut document, Some(1), &first);
    assert_eq!(document.elements[0].text, "Total 100");
    assert_eq!(document.elements[1..], original);
}

#[test]
fn should_preserve_both_equal_ocr_occurrences_when_structured_geometry_is_absent() {
    let (mut document, first) = equal_ocr_occurrences(false);
    let original = document.elements.clone();
    put_layer_first_in_document(&mut document, Some(1), &first);
    assert_eq!(document.elements[1..], original);
    assert_eq!(document.elements[0].text, "Total 100");
}

#[test]
fn should_preserve_the_original_flat_source_order_beside_restored_native_rows() {
    use super::super::document::text_with_layer_first;
    let (document, first) = equal_ocr_occurrences(true);
    let text = document
        .elements
        .iter()
        .map(|e| e.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(text_with_layer_first(&text, &first), format!("Total 100\n\n{text}"));
}

#[test]
fn should_preserve_ocr_occurrences_when_multiple_rows_claim_one_native_box() {
    let mut ocr = MINUTES.to_vec();
    ocr[3] = "Total 1OO";
    ocr.insert(4, "Total 1O0");
    let merged = merge_at_rows(&MINUTES, &ocr.join("\n"), &[0, 1, 2, 3, 3, 4, 5, 6, 7]);
    assert_eq!(merged, format!("Total 100\n\n{}", ocr.join("\n")));
}

#[test]
fn should_preserve_ocr_lines_when_the_flat_source_order_cannot_be_certified() {
    use super::super::document::text_with_layer_first;
    let (document, first) = equal_ocr_occurrences(true);
    let text = document
        .elements
        .iter()
        .rev()
        .map(|e| e.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(text_with_layer_first(&text, &first), format!("Total 100\n\n{text}"));
}

#[test]
fn should_retain_repeated_native_occurrences_when_line_geometry_is_unavailable() {
    use super::super::document::text_with_layer_first;
    let mut layer = MINUTES.to_vec();
    layer.push("Total 100");
    let ocr = MINUTES.join("\n");
    let first = layer_lines_to_put_first(
        &layer.join("\n"),
        &ocr,
        false,
        &OcrQualityThresholds::default(),
        None,
        crate::extractors::pdf::ocr::document::PlainOcrRows::every(),
    )
    .expect("one OCR occurrence cannot cover two native occurrences");
    assert_eq!(first.lines, "Total 100");

    assert_eq!(text_with_layer_first(&ocr, &first), format!("Total 100\n\n{ocr}"));
}

const SEMANTIC_ROWS: [(&str, &str); 5] = [
    ("Total -100", "Total 100"),
    ("Total 12%", "Total 12"),
    ("Total −100", "Total 100"),
    ("Total 0.15", "Total 0,15"),
    ("Total 12 kg/m²", "Total 12 kg-m²"),
];

#[test]
fn should_restore_signed_values_and_units_at_the_owned_ocr_occurrence() {
    let mut checked = 0;
    for (native, read) in SEMANTIC_ROWS {
        let mut layer = MINUTES.to_vec();
        layer[3] = native;
        let mut ocr = layer.clone();
        ocr[3] = read;
        let merged = merge_at_rows(&layer, &ocr.join("\n"), &(0..8).collect::<Vec<_>>());
        assert_eq!(merged, format!("{native}\n\n{}", ocr.join("\n")));
        checked += 1;
    }
    assert_eq!(checked, 5);
}

#[test]
fn should_preserve_signed_native_values_and_all_ocr_payload_without_geometry() {
    use super::super::document::text_with_layer_first;
    let mut checked = 0;
    for (native, read) in SEMANTIC_ROWS {
        let mut layer = MINUTES.to_vec();
        layer[3] = native;
        let mut ocr = layer.clone();
        ocr[3] = read;
        let text = ocr.join("\n");
        let first = layer_lines_to_put_first(
            &layer.join("\n"),
            &text,
            false,
            &OcrQualityThresholds::default(),
            None,
            crate::extractors::pdf::ocr::document::PlainOcrRows::every(),
        )
        .expect("semantic native value was not read");
        assert_eq!(first.lines, native);

        assert_eq!(text_with_layer_first(&text, &first), format!("{native}\n\n{text}"));
        checked += 1;
    }
    assert_eq!(checked, 5);
}

#[test]
fn should_not_borrow_pieces_of_a_missing_native_row_from_other_ocr_rows() {
    use super::super::document::text_with_layer_first;
    let mut ocr = MINUTES.to_vec();
    ocr[3] = "Total\n100";
    let text = ocr.join("\n");
    let first = layer_lines_to_put_first(
        &MINUTES.join("\n"),
        &text,
        false,
        &OcrQualityThresholds::default(),
        None,
        crate::extractors::pdf::ocr::document::PlainOcrRows::every(),
    )
    .expect("no OCR row contains the native source row");
    assert_eq!(first.lines, "Total 100");

    assert_eq!(text_with_layer_first(&text, &first), format!("Total 100\n\n{text}"));
}

fn suffix_ocr_row(read: &str) -> (String, super::super::document::PageLineBoxes, InternalDocument) {
    let mut ocr = MINUTES.to_vec();
    ocr[3] = read;
    let text = ocr.join("\n");
    let mut boxes = page_boxes(&MINUTES, &lines_at_rows(&text, &(0..8).collect::<Vec<_>>()));
    boxes.layer[3].bbox.right = 100.0;
    boxes.ocr[3].bbox.right = 200.0;
    let mut document = InternalDocument::new("pdf");
    let mut element = InternalElement::text(ElementKind::Paragraph, read, 0);
    element.page = Some(1);
    element.bbox = Some(crate::types::BoundingBox {
        x0: 0.0,
        y0: 36.0,
        x1: 200.0,
        y1: 46.0,
    });
    element.annotations.push(TextAnnotation {
        start: 10,
        end: read.len() as u32,
        kind: AnnotationKind::Bold,
    });
    document.push_element(element);
    (text, boxes, document)
}

#[test]
fn should_restore_native_value_without_discarding_ocr_only_suffix_or_its_payload() {
    use super::super::document::text_with_layer_first;
    let (text, boxes, mut document) = suffix_ocr_row("Total 1OO paid today");
    let first = layer_lines_to_put_first(
        &MINUTES.join("\n"),
        &text,
        false,
        &OcrQualityThresholds::default(),
        Some(&boxes),
        crate::extractors::pdf::ocr::document::PlainOcrRows::every(),
    )
    .unwrap();
    assert_eq!(first.lines, "Total 100");

    let original = document.elements.clone();
    put_layer_first_in_document(&mut document, Some(1), &first);
    assert_eq!(document.elements[1..], original);
    assert_eq!(document.elements[0].text, "Total 100");
    assert_eq!(text_with_layer_first(&text, &first), format!("Total 100\n\n{text}"));
}

#[test]
fn should_keep_a_native_covered_ocr_row_and_harmless_suffix_unchanged() {
    let (text, boxes, document) = suffix_ocr_row("Total 100 paid today");
    let original = document.elements.clone();
    assert!(
        layer_lines_to_put_first(
            &MINUTES.join("\n"),
            &text,
            false,
            &OcrQualityThresholds::default(),
            Some(&boxes),
            crate::extractors::pdf::ocr::document::PlainOcrRows::every(),
        )
        .is_none()
    );
    assert_eq!(document.elements, original);
}

#[test]
fn should_reject_nonfinite_ownership_bounds_and_keep_negative_finite_coordinates_valid() {
    use super::super::document::LineBox;
    let finite = LineBox {
        left: -100.0,
        top: -20.0,
        right: -10.0,
        bottom: -5.0,
    };
    assert!(finite.owns(&finite));
    let unbounded = LineBox {
        left: f32::NEG_INFINITY,
        right: f32::INFINITY,
        ..finite
    };
    assert!(!finite.owns(&unbounded));
    let mut checked = 0;
    for axis in 0..4 {
        for value in [f32::NEG_INFINITY, f32::INFINITY, f32::NAN] {
            let mut invalid = finite;
            match axis {
                0 => invalid.left = value,
                1 => invalid.top = value,
                2 => invalid.right = value,
                3 => invalid.bottom = value,
                _ => unreachable!(),
            }
            assert!(!finite.owns(&invalid));
            assert!(!invalid.owns(&finite));
            checked += 2;
        }
    }
    assert_eq!(checked, 24);
}
