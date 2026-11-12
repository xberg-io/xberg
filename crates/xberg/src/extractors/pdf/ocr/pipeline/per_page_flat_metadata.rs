//! Flat backend metadata for the per-page route, matching the whole-document route.

use crate::ocr_metadata_keys::{
    OCR_AUTO_ROTATED_METADATA_KEY, OCR_ORIENTATION_CONFIDENCE_METADATA_KEY, OCR_ORIENTATION_DEGREES_METADATA_KEY,
    OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY, OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
    OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
};
use crate::types::internal::InternalDocument;

const ORIENTATION_KEYS: [&str; 5] = [
    OCR_AUTO_ROTATED_METADATA_KEY,
    OCR_ORIENTATION_DEGREES_METADATA_KEY,
    OCR_ORIENTATION_CONFIDENCE_METADATA_KEY,
    OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
    OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
];

/// The whole-document route copies the backend metadata of the first page it reads onto the
/// document (`backend_additional_metadata`). The per-page route reads each page on its own, so
/// the first accepted Tesseract-read page (the first entry of `ocr_page_segmentation_modes`, which
/// lists only pages whose read text was accepted, where the whole-document route also counts a page
/// with an empty read)
/// stands for the run: its `psm` and the orientation keys its read carries. A page another
/// backend read supplies nothing, and a run with no Tesseract page sets no key (GH#3007). ~keep
pub(super) fn lift(document: &mut InternalDocument, structured_pages: &ahash::AHashMap<u32, InternalDocument>) {
    let Some(first) = document
        .metadata
        .additional
        .get(OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
        .and_then(serde_json::Value::as_array)
        .and_then(|modes| modes.first())
    else {
        return;
    };
    let page = first
        .get("page_number")
        .and_then(serde_json::Value::as_u64)
        .and_then(|page| u32::try_from(page).ok());
    let psm = first.get("psm").and_then(serde_json::Value::as_i64);
    if let Some(psm) = psm {
        document
            .metadata
            .additional
            .insert("psm".into(), serde_json::Value::String(psm.to_string()));
    }
    if let Some(read) = page.and_then(|page| structured_pages.get(&page)) {
        for key in ORIENTATION_KEYS {
            if let Some(value) = read.metadata.additional.get(key) {
                document.metadata.additional.insert(key.into(), value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::attach_page_segmentation_modes;
    use super::*;

    fn read_page(page: u32, psm: Option<i32>, rotated: bool) -> InternalDocument {
        let mut doc = InternalDocument::new("pdf");
        if let Some(psm) = psm {
            attach_page_segmentation_modes(&mut doc, [(page, psm)]);
        }
        if rotated {
            doc.metadata
                .additional
                .insert(OCR_AUTO_ROTATED_METADATA_KEY.into(), true.into());
            doc.metadata
                .additional
                .insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), 90.into());
        }
        doc
    }

    fn lifted(pages: Vec<(u32, InternalDocument)>) -> InternalDocument {
        let structured: ahash::AHashMap<u32, InternalDocument> = pages.into_iter().collect();
        let texts: ahash::AHashMap<u32, String> = structured.keys().map(|page| (*page, "text".to_string())).collect();
        let mut document = InternalDocument::new("pdf");
        crate::extractors::pdf::ocr::document::merge_page_segmentation_modes(&mut document, &texts, &structured);
        lift(&mut document, &structured);
        document
    }

    fn flag(document: &InternalDocument, key: &str) -> Option<serde_json::Value> {
        document.metadata.additional.get(key).cloned()
    }

    #[test]
    fn a_rotated_first_page_sets_the_flag_and_its_orientation() {
        let document = lifted(vec![
            (1, read_page(1, Some(3), true)),
            (2, read_page(2, Some(3), false)),
        ]);
        assert_eq!(flag(&document, "auto_rotated"), Some(true.into()));
        assert_eq!(flag(&document, "orientation_degrees"), Some(90.into()));
        assert_eq!(flag(&document, "psm"), Some("3".into()));
    }

    #[test]
    fn an_upright_first_page_leaves_the_flag_unset_whichever_page_rotated_later() {
        let document = lifted(vec![
            (1, read_page(1, Some(3), false)),
            (2, read_page(2, Some(3), true)),
        ]);
        assert_eq!(flag(&document, "auto_rotated"), None);
        assert_eq!(flag(&document, "orientation_degrees"), None);
        assert_eq!(flag(&document, "psm"), Some("3".into()));
    }

    #[test]
    fn a_page_another_backend_read_supplies_neither_psm_nor_rotation() {
        let document = lifted(vec![(1, read_page(1, None, true)), (2, read_page(2, Some(6), false))]);
        assert_eq!(flag(&document, "auto_rotated"), None);
        assert_eq!(flag(&document, "psm"), Some("6".into()));
    }

    #[test]
    fn a_run_without_a_tesseract_page_sets_no_flat_key() {
        let document = lifted(vec![(1, read_page(1, None, true)), (2, read_page(2, None, false))]);
        assert_eq!(flag(&document, "psm"), None);
        assert_eq!(flag(&document, "auto_rotated"), None);
    }
}
