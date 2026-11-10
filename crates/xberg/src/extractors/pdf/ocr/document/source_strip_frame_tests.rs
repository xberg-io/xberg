use super::*;
use crate::types::internal::InternalDocument;
use serde_json::json;

use crate::ocr_metadata_keys::{
    OCR_SOURCE_STRIP_ORIGINAL_FRAME_METADATA_KEY as LEGACY, OCR_SOURCE_STRIP_ORIGINAL_FRAMES_METADATA_KEY as KEY,
};
const TEXT: &str = "The orchard committee recorded the northern shipment this morning.";

fn page(width: u32) -> InternalDocument {
    let mut page = flat_ocr_page_document(TEXT);
    page.metadata.additional.insert(
        KEY.into(),
        json!([{
            "page_number": 1, "rendered_width": width, "rendered_height": 12000,
            "processed_width": 154, "processed_height": 4096,
        }]),
    );
    page.metadata
        .additional
        .insert("private_backend_value".into(), json!(true));
    page
}

#[test]
fn source_strip_frames_survive_page_reassembly() {
    let original = page(450);
    let expected = original.metadata.additional[KEY].clone();
    let (assembled, paragraphs, _) =
        assemble_mixed_ocr_page_document(original, false, 1, 1000, 1.0, Default::default());
    assert!(!paragraphs.is_empty(), "exercise the document rebuild");
    assert_eq!(assembled.metadata.additional.get(KEY), Some(&expected));
    assert!(!assembled.metadata.additional.contains_key("private_backend_value"));
}

#[test]
fn source_strip_frames_survive_document_restructuring_payload_handoff() {
    let original = page(450);
    let expected = original.metadata.additional[KEY].clone();
    let mut legacy = original.clone();
    legacy.metadata.additional.remove(KEY);
    legacy.metadata.additional.insert(LEGACY.into(), expected[0].clone());
    for source in [original, legacy] {
        let mut rebuilt = flat_ocr_page_document(TEXT);
        carry_page_ocr_payload_forward(&source, &mut rebuilt);
        assert_eq!(rebuilt.metadata.additional.get(KEY), Some(&expected));
        assert_eq!(rebuilt.metadata.additional.get(LEGACY), Some(&expected[0]));
        assert!(!rebuilt.metadata.additional.contains_key("private_backend_value"));
    }
}

#[test]
fn source_strip_frames_merge_only_accepted_pages_in_document_order() {
    let pages = ahash::AHashMap::from([(4, page(900)), (3, page(300)), (2, page(450)), (5, page(500))]);
    let replacements = ahash::AHashMap::from([(4, TEXT.into()), (2, TEXT.into()), (5, " ".into())]);
    let mut document = InternalDocument::new("pdf");
    merge_structured_ocr_pages_into_internal_document(&mut document, &replacements, &pages);
    assert_eq!(
        document.metadata.additional.get(KEY),
        Some(&json!([
            {"page_number": 2, "rendered_width": 450, "rendered_height": 12000,
             "processed_width": 154, "processed_height": 4096},
            {"page_number": 4, "rendered_width": 900, "rendered_height": 12000,
             "processed_width": 154, "processed_height": 4096},
        ]))
    );
    let expected = document.metadata.additional[KEY].clone();
    assert_eq!(document.metadata.additional.get(LEGACY), Some(&expected[0]));
    merge_structured_ocr_pages_into_internal_document(&mut document, &replacements, &pages);
    assert_eq!(document.metadata.additional.get(KEY), Some(&expected));
    assert!(!document.metadata.additional.contains_key("private_backend_value"));
}
