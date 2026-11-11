//! Pins for the detached mixed frame's placement guards (xberg-io/xberg#2896). Each test
//! constructs the minimal read a single guard refuses, so removing only that guard turns the
//! test red.

use super::detached_mixed_frame::place_table_in_render;
use crate::types::{
    BoundingBox, ExtractedDocument, Table,
    internal::{ElementKind, InternalDocument, InternalElement},
    ocr_elements::OcrElementLevel,
};

fn two_table_frame() -> ExtractedDocument {
    let mut document = InternalDocument::new("ocr");
    let mut text = InternalElement::text(
        ElementKind::OcrText {
            level: OcrElementLevel::Block,
        },
        "Ledger summary",
        0,
    );
    text.bbox = Some(BoundingBox {
        x0: 20.,
        y0: 10.,
        x1: 180.,
        y1: 30.,
    });
    document.push_element(text);
    let mut read = ExtractedDocument {
        ocr_internal_document: Some(document),
        ..Default::default()
    };
    for top in [60., 220.] {
        read.tables.push(Table {
            bounding_box: Some(BoundingBox {
                x0: 20.,
                y0: top,
                x1: 180.,
                y1: top + 100.,
            }),
            ..Default::default()
        });
    }
    read.metadata.additional.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
        200.into(),
    );
    read.metadata.additional.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
        400.into(),
    );
    read.metadata.image_preprocessing = Some(crate::types::ImagePreprocessingMetadata {
        original_dimensions: (400, 800).into(),
        original_dpi: (150., 150.).into(),
        target_dpi: 75,
        scale_factor: 0.5,
        auto_adjusted: false,
        final_dpi: 75,
        new_dimensions: Some((200, 400).into()),
        resample_method: "test".into(),
        dimension_clamped: false,
        calculated_dpi: None,
        skipped_resize: false,
        resize_error: None,
    });
    read
}

/// Only the zero render size guard refuses these placements: the preprocessing block agrees
/// with the processed frame and the boxes, so a zero render dimension is the only refusal.
#[test]
fn a_zero_render_dimension_refuses_detached_table_placement() {
    let mut read = two_table_frame();
    assert_eq!(
        place_table_in_render(&mut read, (400, 800), (200., 400.)),
        Some((1., 1.)),
        "the matching render must place the tables"
    );
    for (original, render) in [
        ((0usize, 800usize), (0u32, 800u32)),
        ((400usize, 0usize), (400u32, 0u32)),
    ] {
        let mut read = two_table_frame();
        read.metadata.image_preprocessing.as_mut().unwrap().original_dimensions = original.into();
        let before = read.clone();
        assert_eq!(
            place_table_in_render(&mut read, render, (200., 400.)),
            None,
            "a zero render dimension must refuse placement"
        );
        assert_eq!(
            serde_json::to_value(&read).unwrap(),
            serde_json::to_value(&before).unwrap(),
            "a refused placement must not rescale any table"
        );
    }
}

/// Only the table order guard refuses this placement: both boxes sit inside the processed
/// frame, the text is above both, and the refused placement leaves the read untouched.
#[test]
fn reversed_table_order_refuses_detached_table_placement() {
    let mut read = two_table_frame();
    read.tables.reverse();
    let before = read.clone();
    assert_eq!(
        place_table_in_render(&mut read, (400, 800), (200., 400.)),
        None,
        "tables must be placed in vertical reading order"
    );
    assert_eq!(
        serde_json::to_value(&read).unwrap(),
        serde_json::to_value(&before).unwrap(),
        "a refused placement must not rescale any table"
    );
}
