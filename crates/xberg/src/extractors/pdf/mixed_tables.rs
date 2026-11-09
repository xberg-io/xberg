use super::{InternalDocument, accepted_mixed_ocr_tables};

/// Merge OCR tables into the mixed path's table list, replacing only the tables of pages OCR
/// itself produced a table for.
///
/// Unlike [`super::replace_tables_with_ocr_output`], which is right for full-document OCR (every page
/// went through OCR, so there is no native page left to preserve), the mixed path sends only
/// SOME pages to OCR. Before this, the mixed path called `replace_tables_with_ocr_output` too,
/// which replaced the WHOLE document's table list with only the OCR pages' tables: on a long
/// document where a handful of scanned pages produced even one OCR table, every native table on
/// every page OCR never touched was silently dropped (GH#1670). A page whose OCR pass found no
/// table keeps its native tables untouched, the same as `replace_tables_with_ocr_output` does
/// when it finds none anywhere.
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
