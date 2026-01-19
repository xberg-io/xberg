//! Page content management for PDF extraction.
//!
//! Handles assignment of tables and images to specific pages.

use crate::types::PageContent;

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

    Some(updated_pages)
}
