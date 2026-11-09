use super::{InternalDocument, accepted_mixed_ocr_tables};

/// Replace native tables only on pages for which OCR supplied accepted tables (GH#1670).
/// Keep independently extracted native tables elsewhere, including pages whose text OCR replaced:
/// no OCR table is not evidence that native geometry is invalid. Such pages may expose OCR text
/// alongside native table cells; mixed extraction does not guarantee agreement between the two.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn merge_mixed_ocr_tables(
    tables: &mut Vec<crate::types::Table>,
    structured_pages: &ahash::AHashMap<u32, InternalDocument>,
) {
    let mut ocr_tables = accepted_mixed_ocr_tables(structured_pages);
    if ocr_tables.is_empty() {
        return;
    }

    ocr_tables.sort_by_key(|table| table.page_number);
    let ocr_pages: std::collections::HashSet<u32> = ocr_tables.iter().map(|table| table.page_number).collect();
    tables.retain(|table| !ocr_pages.contains(&table.page_number));
    tables.extend(ocr_tables);
    tables.sort_by_key(|table| table.page_number);
}
