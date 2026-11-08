use crate::ocr_metadata_keys::{
    OCR_AUTO_ROTATED_METADATA_KEY, OCR_ORIENTATION_DEGREES_METADATA_KEY, OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
    OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY, OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY,
    OCR_SOURCE_STRIP_ORIGINAL_FRAME_METADATA_KEY, OCR_SOURCE_STRIP_ORIGINAL_FRAMES_METADATA_KEY,
};
use crate::types::ExtractedDocument;

pub(super) fn select_read_width(original: &ExtractedDocument, (width, height): (u32, u32)) -> Option<u32> {
    let metadata = &original.metadata.additional;
    if [OCR_AUTO_ROTATED_METADATA_KEY, OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY]
        .iter()
        .any(|key| metadata.get(*key).is_some_and(|value| value.as_bool() != Some(false)))
        || metadata
            .get(OCR_ORIENTATION_DEGREES_METADATA_KEY)
            .is_some_and(|value| value.as_f64() != Some(0.0))
    {
        return None;
    }
    let dimension = |key, bound| {
        metadata
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .filter(|&value| value > 0 && value <= bound)
    };
    let Some(read_width) = dimension(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY, width) else {
        return Some(width);
    };
    let Some(read_height) = dimension(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, height) else {
        return Some(width);
    };
    // One-pixel rounding of an aspect-preserving resize bounds this cross-product error. ~keep
    let error = (u64::from(read_width) * u64::from(height)).abs_diff(u64::from(read_height) * u64::from(width));
    (error <= u64::from(width.max(height))).then_some(read_width)
}

pub(super) fn record_original_frame(original: &mut ExtractedDocument, (width, height): (u32, u32), page: usize) {
    let metadata = &mut original.metadata.additional;
    let frame = serde_json::json!({
        "page_number": page + 1,
        "rendered_width": width,
        "rendered_height": height,
        "processed_width": metadata.get(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY),
        "processed_height": metadata.get(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY),
    });
    metadata.insert(OCR_SOURCE_STRIP_ORIGINAL_FRAME_METADATA_KEY.into(), frame.clone());
    metadata.insert(
        OCR_SOURCE_STRIP_ORIGINAL_FRAMES_METADATA_KEY.into(),
        serde_json::json!([frame]),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn read(width: Option<serde_json::Value>, height: Option<serde_json::Value>) -> ExtractedDocument {
        let mut doc = ExtractedDocument::default();
        if let Some(width) = width {
            doc.metadata
                .additional
                .insert(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), width);
        }
        if let Some(height) = height {
            doc.metadata
                .additional
                .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), height);
        }
        doc
    }

    #[test]
    fn read_width_selection_falls_back_for_absent_zero_wide_and_non_integer_values() {
        for value in [
            None,
            Some(json!(0)),
            Some(json!(1001)),
            Some(json!(500.5)),
            Some(json!(-1)),
            Some(json!("500")),
            Some(json!(u64::MAX)),
        ] {
            assert_eq!(
                select_read_width(&read(value, Some(json!(4000))), (1000, 8000)),
                Some(1000)
            );
        }
        assert_eq!(
            select_read_width(&read(Some(json!(500)), Some(json!(4000))), (1000, 8000)),
            Some(500)
        );
        assert_eq!(
            select_read_width(&read(Some(json!(1000)), Some(json!(8000))), (1000, 8000)),
            Some(1000)
        );
    }

    #[test]
    fn an_applied_rotation_cannot_supply_a_narrower_unrotated_read_width() {
        for width in [400, 8000] {
            let mut doc = read(Some(json!(width)), Some(json!(50)));
            doc.metadata
                .additional
                .insert(OCR_AUTO_ROTATED_METADATA_KEY.into(), json!(true));
            doc.metadata
                .additional
                .insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), json!(90));
            assert_eq!(select_read_width(&doc, (1000, 8000)), None);
        }
    }

    #[test]
    fn transposed_then_shrunk_frame_is_not_a_downscale_of_the_rendered_frame() {
        assert_eq!(
            select_read_width(&read(Some(json!(400)), Some(json!(50))), (1000, 8000)),
            None
        );
        assert_eq!(
            select_read_width(&read(Some(json!(512)), Some(json!(4096))), (1000, 8000)),
            Some(512)
        );
    }

    #[test]
    fn missing_or_invalid_height_cannot_claim_a_proven_downscale() {
        for height in [None, Some(json!(0)), Some(json!(8001)), Some(json!(4000.5))] {
            assert_eq!(
                select_read_width(&read(Some(json!(500)), height), (1000, 8000)),
                Some(1000)
            );
        }
    }

    #[test]
    fn rotation_provenance_is_not_erased_by_geometry_restoration() {
        let mut doc = read(Some(json!(500)), Some(json!(4000)));
        doc.metadata
            .additional
            .insert(OCR_AUTO_ROTATED_METADATA_KEY.into(), json!(false));
        doc.metadata
            .additional
            .insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), json!(0));
        assert_eq!(select_read_width(&doc, (1000, 8000)), Some(500));
        doc.metadata
            .additional
            .insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), json!(90));
        doc.metadata
            .additional
            .insert("ocr_backend_rotation_undone".into(), json!(true));
        assert_eq!(select_read_width(&doc, (1000, 8000)), None);
        doc.metadata
            .additional
            .insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), json!(0));
        doc.metadata
            .additional
            .insert(OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY.into(), json!(true));
        assert_eq!(select_read_width(&doc, (1000, 8000)), None);
    }

    #[test]
    fn fractional_pixel_rounding_preserves_the_downscaled_aspect_ratio() {
        assert_eq!(
            select_read_width(&read(Some(json!(154)), Some(json!(4096))), (450, 12000)),
            Some(154)
        );
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn original_frame_diagnostic_keeps_observed_dimensions_and_page_identity() {
        let mut original = ExtractedDocument::default();
        original
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), json!(154));
        original
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), json!(4096));
        record_original_frame(&mut original, (450, 12000), 1);
        assert_eq!(
            original.metadata.additional[OCR_SOURCE_STRIP_ORIGINAL_FRAME_METADATA_KEY],
            json!({
                "page_number": 2, "rendered_width": 450, "rendered_height": 12000,
                "processed_width": 154, "processed_height": 4096,
            })
        );
        assert_eq!(
            original.metadata.additional[OCR_SOURCE_STRIP_ORIGINAL_FRAMES_METADATA_KEY],
            json!([original.metadata.additional[OCR_SOURCE_STRIP_ORIGINAL_FRAME_METADATA_KEY]])
        );
        assert_eq!(
            original.metadata.additional[OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY],
            154
        );
    }

    #[test]
    fn rotated_pdf_page_is_refused_even_when_reported_width_would_admit_it() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/repros/tall-scan-strips/large-text-150.pdf"
        ));
        let base = xberg_native_pdf::PdfDocument::from_bytes(bytes.to_vec()).unwrap();
        assert!(super::super::certify(&base, 0, Some(154), 100 * 1024 * 1024).is_some());
        for rotation in [90, 180, 270] {
            let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
            let page = pdf.get_pages()[&1];
            pdf.get_object_mut(page)
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("Rotate", rotation);
            let mut bytes = Vec::new();
            pdf.save_to(&mut bytes).unwrap();
            let rotated = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
            assert_eq!(
                rotated.get_page_rotation_status(0).unwrap(),
                xberg_native_pdf::document::PageRotation::Valid(rotation)
            );
            assert!(super::super::certify(&rotated, 0, Some(154), 100 * 1024 * 1024).is_none());
        }
    }
}
