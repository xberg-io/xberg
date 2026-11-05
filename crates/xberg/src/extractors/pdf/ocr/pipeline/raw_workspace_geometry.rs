use super::super::document::processed_ocr_layout_dimensions;
use super::undo_ocr_element_geometry;
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::types::{BoundingBox, ExtractedDocument};

/// Map the downscaled raw OCR geometry back to the PDF render frame after undoing rotation.
pub(super) fn normalize(result: &mut ExtractedDocument, render: (u32, u32), axes_swapped: bool) -> bool {
    let Some(source) = result.raw_ocr_source_frame.take() else {
        return false;
    };
    let source = if axes_swapped {
        (source.dimensions.1, source.dimensions.0)
    } else {
        source.dimensions
    };
    let Some(processed) = processed_ocr_layout_dimensions(&result.metadata) else {
        return false;
    };
    if source != render || processed.0 >= render.0 || processed.1 >= render.1 {
        return false;
    }
    scale_payloads(
        result,
        f64::from(render.0) / f64::from(processed.0),
        f64::from(render.1) / f64::from(processed.1),
    );
    for (key, size) in [
        (OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY, render.0),
        (OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, render.1),
    ] {
        result.metadata.additional.insert(key.into(), size.into());
    }
    true
}

fn scale_payloads(result: &mut ExtractedDocument, scale_x: f64, scale_y: f64) {
    let scale_point = |x, y| (x * scale_x, y * scale_y);
    let scale_bbox = |bbox: &mut BoundingBox| {
        (bbox.x0, bbox.y0) = scale_point(bbox.x0, bbox.y0);
        (bbox.x1, bbox.y1) = scale_point(bbox.x1, bbox.y1);
    };
    if let Some(doc) = result.ocr_internal_document.as_mut() {
        for element in &mut doc.elements {
            if let Some(bbox) = element.bbox.as_mut() {
                scale_bbox(bbox);
            }
            if let Some(geometry) = element.ocr_geometry.as_mut() {
                undo_ocr_element_geometry(geometry, scale_point);
            }
        }
        crate::types::internal::table_grid::transform_retained_grid_bboxes(doc, &result.tables, scale_bbox);
        for bbox in doc.tables.iter_mut().filter_map(|table| table.bounding_box.as_mut()) {
            scale_bbox(bbox);
        }
    }
    for bbox in result.tables.iter_mut().filter_map(|table| table.bounding_box.as_mut()) {
        scale_bbox(bbox);
    }
    for bbox in result.formulas.iter_mut().filter_map(|formula| formula.bbox.as_mut()) {
        scale_bbox(bbox);
    }
    for element in result.ocr_elements.iter_mut().flatten() {
        undo_ocr_element_geometry(&mut element.geometry, scale_point);
    }
}
