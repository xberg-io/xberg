//! Page content management for PDF extraction.
//!
//! Handles assignment of tables, images, layout regions, and hierarchy to specific pages.

use crate::types::internal::{ElementKind, InternalDocument};
use crate::types::{HierarchicalBlock, PageContent, PageHierarchy};

/// Fallback font size (in points) reported for a hierarchy block when no measured
/// font size was recorded on the element.
///
/// This is the pre-fix hardcoded default that every block used to report
/// unconditionally; it is kept as a fallback so output stays stable for elements
/// that predate font-size measurement (e.g. non-PDF extractors) rather than
/// reporting a nonsensical `0.0`.
const FALLBACK_HIERARCHY_FONT_SIZE_PT: f32 = 12.0;

/// Extract hierarchy information from an InternalDocument and assign to pages.
///
/// Examines all heading and paragraph elements in the document, maps them to
/// pages using the `page` field, and creates PageHierarchy structures for each
/// page. Each block reports the element's measured dominant font size when one
/// was recorded, falling back to [`FALLBACK_HIERARCHY_FONT_SIZE_PT`] otherwise.
///
/// Only processes ElementKind::Heading and ElementKind::Paragraph elements and
/// ignores other element types.
pub(crate) fn assign_hierarchy_to_pages(pages: &mut [PageContent], doc: &InternalDocument) {
    let mut page_hierarchies: std::collections::HashMap<u32, Vec<HierarchicalBlock>> = std::collections::HashMap::new();

    for element in &doc.elements {
        let page_num = match element.page {
            Some(p) => p,
            None => continue,
        };

        // An OCR text anchor is an empty paragraph until the pipeline gives it the text of
        // its picture; it is no block of the page's native text. ~keep
        #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
        if element.image_ocr_text_anchor_index().is_some() {
            continue;
        }

        let level = match element.kind {
            ElementKind::Heading { level } => format!("h{}", level),
            ElementKind::Paragraph => "body".to_string(),
            _ => continue,
        };

        let block = HierarchicalBlock {
            text: element.text.clone(),
            font_size: element.measured_font_size().unwrap_or(FALLBACK_HIERARCHY_FONT_SIZE_PT),
            level,
            bbox: element
                .bbox
                .map(|b| (b.x0 as f32, b.y0 as f32, b.x1 as f32, b.y1 as f32).into()),
        };
        page_hierarchies.entry(page_num).or_default().push(block);
    }

    for page in pages.iter_mut() {
        if let Some(blocks) = page_hierarchies.remove(&page.page_number) {
            let block_count = blocks.len() as u32;
            page.hierarchy = Some(PageHierarchy { block_count, blocks });
        }
    }
}

/// Helper function to assign tables and images to pages.
///
/// If page_contents is None, returns None (no per-page tracking enabled).
/// Otherwise, iterates through tables and images, assigning them to pages based on page_number.
///
/// # Performance
///
/// Uses Arc::new to wrap tables and images, avoiding expensive copies.
/// This reduces memory overhead by enabling zero-copy sharing of table/image data
/// across multiple references (e.g., when the same table appears on multiple pages).
///
/// # Arguments
///
/// * `page_contents` - Optional vector of page contents to populate
/// * `tables` - Slice of tables to assign to pages
/// * `images` - Slice of images to assign to pages
///
/// # Returns
///
/// Updated page contents with tables and images assigned, or None if page tracking disabled
pub(crate) fn assign_tables_and_images_to_pages(
    mut page_contents: Option<Vec<PageContent>>,
    tables: &[crate::types::Table],
    images: &[crate::types::ExtractedImage],
) -> Option<Vec<PageContent>> {
    let pages = page_contents.take()?;

    let mut updated_pages = pages;

    for table in tables {
        if let Some(page) = updated_pages.iter_mut().find(|p| p.page_number == table.page_number) {
            page.tables.push(std::sync::Arc::new(table.clone()));
        }
    }

    for (idx, image) in images.iter().enumerate() {
        if let Some(page_num) = image.page_number
            && let Some(page) = updated_pages.iter_mut().find(|p| p.page_number == page_num)
        {
            page.image_indices.push(idx as u32);
        }
    }

    for page in &mut updated_pages {
        if !page.tables.is_empty() || !page.image_indices.is_empty() {
            page.is_blank = Some(false);
        }
    }

    Some(updated_pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal_builder::InternalDocumentBuilder;

    fn page(page_number: u32) -> PageContent {
        PageContent {
            page_number,
            content: String::new(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        }
    }

    #[test]
    fn should_report_each_elements_own_measured_font_size() {
        let mut builder = InternalDocumentBuilder::new("pdf");
        let heading_idx = builder.push_heading(1, "Title", Some(1), None);
        builder.set_measured_font_size(heading_idx, 24.0);
        let paragraph_idx = builder.push_paragraph("Body text.", vec![], Some(1), None);
        builder.set_measured_font_size(paragraph_idx, 11.5);
        let doc = builder.build();

        let mut pages = vec![page(1)];
        assign_hierarchy_to_pages(&mut pages, &doc);

        let blocks = &pages[0].hierarchy.as_ref().expect("hierarchy present").blocks;
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].font_size, 24.0, "heading should report its own measured size");
        assert_eq!(
            blocks[1].font_size, 11.5,
            "paragraph should report its own measured size, not the heading's"
        );
    }

    #[test]
    fn should_fall_back_to_default_font_size_when_none_was_measured() {
        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_paragraph("Body text.", vec![], Some(1), None);
        let doc = builder.build();

        let mut pages = vec![page(1)];
        assign_hierarchy_to_pages(&mut pages, &doc);

        let blocks = &pages[0].hierarchy.as_ref().expect("hierarchy present").blocks;
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].font_size, FALLBACK_HIERARCHY_FONT_SIZE_PT);
    }

    #[test]
    fn should_fall_back_to_default_font_size_for_non_finite_or_non_positive_measurements() {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -12.0] {
            let mut builder = InternalDocumentBuilder::new("pdf");
            let idx = builder.push_paragraph("Body text.", vec![], Some(1), None);
            builder.set_measured_font_size(idx, invalid);
            let doc = builder.build();

            let mut pages = vec![page(1)];
            assign_hierarchy_to_pages(&mut pages, &doc);

            let blocks = &pages[0].hierarchy.as_ref().expect("hierarchy present").blocks;
            assert_eq!(
                blocks[0].font_size, FALLBACK_HIERARCHY_FONT_SIZE_PT,
                "invalid measurement {invalid} must not propagate"
            );
        }
    }

    #[test]
    fn should_skip_elements_with_no_page() {
        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_paragraph("Body text.", vec![], None, None);
        let doc = builder.build();

        let mut pages = vec![page(1)];
        assign_hierarchy_to_pages(&mut pages, &doc);

        assert!(
            pages[0].hierarchy.is_none(),
            "an element with no page must not be assigned"
        );
    }
}
