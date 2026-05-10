//! Page content management for PDF extraction.
//!
//! Handles assignment of tables, images, layout regions, and hierarchy to specific pages.

use crate::types::internal::{ElementKind, InternalDocument};
use crate::types::{HierarchicalBlock, PageContent, PageHierarchy};

/// Extract hierarchy information from an InternalDocument and assign to pages.
///
/// Examines all heading elements in the document, maps them to pages using the
/// `page` field, and creates PageHierarchy structures for each page.
///
/// Only processes ElementKind::Heading elements and ignores other element types.
pub(crate) fn assign_hierarchy_to_pages(pages: &mut [PageContent], doc: &InternalDocument) {
    // Group heading/block elements by page number
    let mut page_hierarchies: std::collections::HashMap<usize, Vec<HierarchicalBlock>> =
        std::collections::HashMap::new();

    for element in &doc.elements {
        let page_num = match element.page {
            Some(p) => p as usize,
            None => continue,
        };

        match element.kind {
            ElementKind::Heading { level } => {
                let block = HierarchicalBlock {
                    text: element.text.clone(),
                    font_size: 12.0, // Default font size (not available in InternalElement)
                    level: format!("h{}", level),
                    bbox: element
                        .bbox
                        .map(|b| (b.x0 as f32, b.y0 as f32, b.x1 as f32, b.y1 as f32)),
                };
                page_hierarchies.entry(page_num).or_default().push(block);
            }
            ElementKind::Paragraph => {
                let block = HierarchicalBlock {
                    text: element.text.clone(),
                    font_size: 12.0,
                    level: "body".to_string(),
                    bbox: element
                        .bbox
                        .map(|b| (b.x0 as f32, b.y0 as f32, b.x1 as f32, b.y1 as f32)),
                };
                page_hierarchies.entry(page_num).or_default().push(block);
            }
            _ => {}
        }
    }

    // Assign hierarchy to each page
    for page in pages.iter_mut() {
        if let Some(blocks) = page_hierarchies.remove(&page.page_number) {
            let block_count = blocks.len();
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

    for image in images {
        if let Some(page_num) = image.page_number
            && let Some(page) = updated_pages.iter_mut().find(|p| p.page_number == page_num)
        {
            page.images.push(std::sync::Arc::new(image.clone()));
        }
    }

    // Refine is_blank: pages that gained tables or images are not blank
    for page in &mut updated_pages {
        if !page.tables.is_empty() || !page.images.is_empty() {
            page.is_blank = Some(false);
        }
    }

    Some(updated_pages)
}
