use super::undo_upright_raster_correction;
use crate::extractors::pdf::ocr::document::{processed_ocr_layout_dimensions, transform_ocr_elements_to_render_space};
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{BoundingBox, ExtractedDocument, Formula, OcrBoundingGeometry, OcrConfidence, OcrElement, Table};

const RENDER_DIMENSIONS: (u32, u32) = (100, 200);
const QUARTER_TURN_INPUT: (u32, u32) = (200, 100);
const SCALED_QUARTER_TURN: (u32, u32) = (400, 200);

fn rectangle(left: u32, top: u32, width: u32, height: u32) -> OcrBoundingGeometry {
    OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    }
}

fn fixture(processed: Option<(u32, u32)>) -> ExtractedDocument {
    let bbox = BoundingBox {
        x0: 20.0,
        y0: 40.0,
        x1: 50.0,
        y1: 60.0,
    };
    let mut document = InternalDocument::new("ocr");
    let mut paragraph = InternalElement::text(ElementKind::Paragraph, "count 17", 0).with_page(1);
    paragraph.bbox = Some(bbox);
    document.push_element(paragraph);
    let mut result = ExtractedDocument {
        content: "count 17".to_owned(),
        ocr_internal_document: Some(document),
        tables: vec![Table {
            bounding_box: Some(bbox),
            ..Default::default()
        }],
        formulas: vec![Formula {
            latex: "x=17".to_owned(),
            bbox: Some(bbox),
            page: Some(1),
        }],
        ocr_elements: Some(vec![OcrElement {
            text: "count 17".to_owned(),
            geometry: rectangle(20, 40, 30, 20),
            confidence: OcrConfidence {
                detection: None,
                recognition: 0.97,
            },
            page_number: 1,
            ..Default::default()
        }]),
        ..Default::default()
    };
    set_processed_dimensions(&mut result, processed);
    result
}

fn set_processed_dimensions(result: &mut ExtractedDocument, processed: Option<(u32, u32)>) {
    if let Some((width, height)) = processed {
        for (key, value) in [
            (OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY, width),
            (OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, height),
        ] {
            result.metadata.additional.insert(key.into(), value.into());
        }
    }
    result
        .metadata
        .additional
        .insert("backend_diagnostic".into(), serde_json::json!({"psm": 3}));
}

fn assert_geometry(result: &ExtractedDocument, expected: (u32, u32, u32, u32)) {
    let (left, top, width, height) = expected;
    let bbox = BoundingBox {
        x0: f64::from(left),
        y0: f64::from(top),
        x1: f64::from(left + width),
        y1: f64::from(top + height),
    };
    let elements = result.ocr_elements.as_ref().expect("private words must survive");
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].geometry, rectangle(left, top, width, height));
    assert_eq!(elements[0].text, "count 17");
    assert_eq!(elements[0].confidence.recognition, 0.97);
    assert_eq!(elements[0].page_number, 1);
    assert_eq!(
        result.ocr_internal_document.as_ref().unwrap().elements[0].bbox,
        Some(bbox)
    );
    assert_eq!(result.tables[0].bounding_box, Some(bbox));
    assert_eq!(result.formulas[0].bbox, Some(bbox));
    assert_eq!(
        result.metadata.additional["backend_diagnostic"],
        serde_json::json!({"psm": 3})
    );
}

fn assert_render_geometry(result: &ExtractedDocument, expected: (u32, u32, u32, u32)) {
    let elements = transform_ocr_elements_to_render_space(
        result.ocr_elements.as_ref().unwrap(),
        &result.metadata,
        RENDER_DIMENSIONS.0,
        RENDER_DIMENSIONS.1,
    )
    .expect("corrected processed geometry must map onto the original table raster");
    assert_eq!(elements.len(), 1);
    assert_eq!(
        elements[0].geometry,
        rectangle(expected.0, expected.1, expected.2, expected.3)
    );
    assert_eq!(elements[0].text, "count 17");
    assert_eq!(elements[0].confidence.recognition, 0.97);
}

#[test]
fn should_map_scaled_ninety_degree_words_to_the_original_table_raster() {
    let mut result = fixture(Some(SCALED_QUARTER_TURN));
    let preprocessing = serde_json::json!({
        "original_dimensions": {"width": 200, "height": 100},
        "original_dpi": {"horizontal": 150.0, "vertical": 150.0},
        "target_dpi": 300, "scale_factor": 2.0, "auto_adjusted": false,
        "final_dpi": 300, "new_dimensions": {"width": 400, "height": 200},
        "resample_method": "CATMULLROM", "dimension_clamped": false,
        "calculated_dpi": 300, "skipped_resize": false, "resize_error": null,
    });
    result.metadata.image_preprocessing = Some(serde_json::from_value(preprocessing.clone()).unwrap());
    undo_upright_raster_correction(&mut result, 90, QUARTER_TURN_INPUT.0, QUARTER_TURN_INPUT.1);
    assert_eq!(
        serde_json::to_value(&result.metadata.image_preprocessing).unwrap(),
        preprocessing
    );
    assert_geometry(&result, (40, 350, 20, 30));
    assert_eq!(processed_ocr_layout_dimensions(&result.metadata), Some((200, 400)));
    assert_render_geometry(&result, (20, 175, 10, 15));
}

#[test]
fn should_map_scaled_two_seventy_degree_quadrilaterals_to_the_original_table_raster() {
    let mut result = fixture(Some(SCALED_QUARTER_TURN));
    let mut quadrilateral = result.ocr_elements.as_ref().unwrap()[0].clone();
    quadrilateral.geometry = OcrBoundingGeometry::Quadrilateral {
        points: [(20, 40), (50, 40), (50, 60), (20, 60)]
            .into_iter()
            .map(Into::into)
            .collect(),
    };
    undo_upright_raster_correction(&mut result, 270, QUARTER_TURN_INPUT.0, QUARTER_TURN_INPUT.1);
    assert_geometry(&result, (140, 20, 20, 30));
    let mut quad_result = fixture(Some(SCALED_QUARTER_TURN));
    quad_result.ocr_elements = Some(vec![quadrilateral]);
    undo_upright_raster_correction(&mut quad_result, 270, QUARTER_TURN_INPUT.0, QUARTER_TURN_INPUT.1);
    let transformed = transform_ocr_elements_to_render_space(
        quad_result.ocr_elements.as_ref().unwrap(),
        &quad_result.metadata,
        100,
        200,
    )
    .expect("quadrilateral words must retain the table's render frame");
    assert_eq!(
        transformed[0].geometry,
        OcrBoundingGeometry::Quadrilateral {
            points: [(80, 10), (80, 25), (70, 25), (70, 10)]
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    );
    assert_eq!(processed_ocr_layout_dimensions(&result.metadata), Some((200, 400)));
    assert_render_geometry(&result, (70, 10, 10, 15));
}

#[test]
fn should_use_processed_translation_for_a_scaled_half_turn() {
    let mut result = fixture(Some((200, 400)));
    undo_upright_raster_correction(&mut result, 180, RENDER_DIMENSIONS.0, RENDER_DIMENSIONS.1);
    assert_geometry(&result, (150, 340, 30, 20));
    assert_eq!(processed_ocr_layout_dimensions(&result.metadata), Some((200, 400)));
    assert_render_geometry(&result, (75, 170, 15, 10));
}

#[test]
fn should_preserve_unscaled_legacy_geometry_without_inventing_processed_dimensions() {
    for (rotation, expected) in [(90, (40, 150, 20, 30)), (270, (40, 20, 20, 30))] {
        let mut result = fixture(None);
        undo_upright_raster_correction(&mut result, rotation, QUARTER_TURN_INPUT.0, QUARTER_TURN_INPUT.1);
        assert_geometry(&result, expected);
        assert_eq!(processed_ocr_layout_dimensions(&result.metadata), None);
        assert!(
            transform_ocr_elements_to_render_space(result.ocr_elements.as_ref().unwrap(), &result.metadata, 100, 200,)
                .is_none()
        );
    }
}

#[test]
fn should_leave_zero_correction_geometry_and_metadata_unchanged() {
    let mut result = fixture(Some((200, 400)));
    let before = serde_json::to_value(&result).unwrap();
    undo_upright_raster_correction(&mut result, 0, RENDER_DIMENSIONS.0, RENDER_DIMENSIONS.1);
    assert_eq!(serde_json::to_value(&result).unwrap(), before);
    assert_geometry(&result, (20, 40, 30, 20));
    assert_render_geometry(&result, (10, 20, 15, 10));
}

#[test]
fn should_reject_missing_invalid_or_unrelated_processed_metadata_for_table_mapping() {
    for dimensions in [None, Some((0, 200)), Some((200, 0)), Some((200, 200))] {
        let mut result = fixture(dimensions);
        undo_upright_raster_correction(&mut result, 90, QUARTER_TURN_INPUT.0, QUARTER_TURN_INPUT.1);
        assert!(
            transform_ocr_elements_to_render_space(result.ocr_elements.as_ref().unwrap(), &result.metadata, 100, 200,)
                .is_none(),
            "untrustworthy dimensions {dimensions:?} must not become a valid table frame"
        );
    }
}
