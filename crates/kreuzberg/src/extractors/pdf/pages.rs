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
                page_hierarchies.entry(page_num).or_insert_with(Vec::new).push(block);
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
                page_hierarchies.entry(page_num).or_insert_with(Vec::new).push(block);
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

/// Populate `layout_regions` on each page from layout detection results.
///
/// Maps each `PageLayoutResult` to its corresponding `PageContent` by page index,
/// converting internal `PageLayoutRegion` values to the public `LayoutRegion` type.
///
/// Only populated when layout detection is enabled and results are available.
/// Pages without matching layout results retain `layout_regions: None`.
#[cfg(feature = "layout-detection")]
pub(crate) fn assign_layout_regions_to_pages(
    page_contents: &mut Option<Vec<PageContent>>,
    layout_results: &[crate::pdf::structure::types::PageLayoutResult],
) {
    let Some(pages) = page_contents.as_mut() else {
        return;
    };

    for result in layout_results {
        let page_number = result.page_index + 1; // page_index is 0-based, page_number is 1-based
        if let Some(page) = pages.iter_mut().find(|p| p.page_number == page_number) {
            let page_area = f64::from(result.page_width_pts) * f64::from(result.page_height_pts);
            let regions: Vec<crate::types::LayoutRegion> = result
                .regions
                .iter()
                .map(|r| {
                    let region_area = f64::from(r.bbox.width()) * f64::from(r.bbox.height());
                    let area_fraction = if page_area > 0.0 { region_area / page_area } else { 0.0 };
                    crate::types::LayoutRegion {
                        class_name: r.class_name.name().to_string(),
                        confidence: f64::from(r.confidence),
                        bounding_box: crate::types::BoundingBox {
                            x0: f64::from(r.bbox.left),
                            y0: f64::from(r.bbox.bottom),
                            x1: f64::from(r.bbox.right),
                            y1: f64::from(r.bbox.top),
                        },
                        area_fraction,
                    }
                })
                .collect();
            if !regions.is_empty() {
                page.layout_regions = Some(regions);
            }
        }
    }
}
