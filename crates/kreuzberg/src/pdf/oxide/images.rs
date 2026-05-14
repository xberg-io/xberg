//! Image extraction using the pdf_oxide backend.
//!
//! Extracts embedded images from PDF pages via pdf_oxide, including
//! actual image data and metadata.

use super::OxideDocument;
use crate::pdf::error::{PdfError, Result};
use bytes::Bytes;
use std::borrow::Cow;

/// Extract image positions from all pages for interleaving into the assembly pipeline.
///
/// Calls `doc.doc.extract_images(page_idx)` for each page and creates an
/// `(page_number, image_index)` pair for each image found. These positions
/// are used by the assembly pipeline to insert image placeholders into the
/// document structure.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the oxide document
///
/// # Returns
///
/// A `Vec<(u32, u32)>` of (1-indexed page number, global image index) pairs.
pub(crate) fn extract_image_positions(doc: &mut OxideDocument) -> Result<Vec<(u32, u32)>> {
    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("pdf_oxide: failed to get page count: {e}")))?;

    let mut positions = Vec::new();
    let mut global_index = 0u32;

    for page_idx in 0..page_count {
        let oxide_images = match doc.doc.extract_images(page_idx) {
            Ok(images) => images,
            Err(e) => {
                tracing::debug!(
                    page = page_idx,
                    "pdf_oxide: failed to extract images for positions: {e}"
                );
                continue;
            }
        };

        let page_number = (page_idx + 1) as u32; // Kreuzberg uses 1-indexed page numbers

        for _img in &oxide_images {
            positions.push((page_number, global_index));
            global_index += 1;
        }
    }

    Ok(positions)
}

/// Extract full image data from all pages of a PDF.
///
/// Returns a `Vec<ExtractedImage>` with complete image data and metadata.
/// When image extraction is disabled or no images are found, returns an empty vec.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the oxide document
/// * `max_images_per_page` - Optional limit on images per page
///
/// # Returns
///
/// A `Vec<ExtractedImage>` containing all extracted images with their data.
pub(crate) fn extract_images_with_data(
    doc: &mut OxideDocument,
    max_images_per_page: Option<u32>,
) -> Result<Vec<crate::types::ExtractedImage>> {
    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("pdf_oxide: failed to get page count: {e}")))?;

    let mut all_images = Vec::new();
    let mut global_index = 0u32;

    for page_idx in 0..page_count {
        let oxide_images = match doc.doc.extract_images(page_idx) {
            Ok(images) => images,
            Err(e) => {
                tracing::debug!(page = page_idx, "pdf_oxide: failed to extract images: {e}");
                continue;
            }
        };

        let page_number = (page_idx + 1) as u32; // Kreuzberg uses 1-indexed page numbers
        let limit = max_images_per_page.unwrap_or(u32::MAX) as usize;
        let count = oxide_images.len().min(limit);

        for oxide_img in oxide_images.iter().take(count) {
            // Extract image data - use raw ImageData directly to avoid expensive conversions
            let (data, format) = match oxide_img.data() {
                pdf_oxide::extractors::ImageData::Jpeg(jpeg_bytes) => {
                    (Bytes::copy_from_slice(jpeg_bytes), Cow::Borrowed("jpeg"))
                }
                pdf_oxide::extractors::ImageData::Raw { pixels, format: _ } => {
                    (Bytes::copy_from_slice(pixels), Cow::Borrowed("raw"))
                }
            };

            let extracted_img = crate::types::ExtractedImage {
                data,
                format,
                image_index: global_index,
                page_number: Some(page_number),
                width: Some(oxide_img.width()),
                height: Some(oxide_img.height()),
                colorspace: Some(format!("{:?}", oxide_img.color_space())),
                bits_per_component: Some(oxide_img.bits_per_component() as u32),
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: None,
                source_path: None,
                image_kind: None,
                kind_confidence: None,
                cluster_id: None,
            };

            all_images.push(extracted_img);
            global_index += 1;
        }
    }

    Ok(all_images)
}
