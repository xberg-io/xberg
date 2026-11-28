//! Multi-frame TIFF OCR: every frame is read on its own and becomes one page (#3447).

use super::ImageExtractor;
use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::types::internal::{ElementKind, InternalDocument, RelationshipTarget};
use crate::types::page::{PageStructure, PageUnitType};

/// The directory offsets of a TIFF's frames when it holds more than one, `None` otherwise.
pub(super) fn multi_frame_offsets(content: &[u8], mime_type: &str) -> Option<Vec<u64>> {
    if !mime_type.to_lowercase().contains("tiff") {
        return None;
    }
    crate::extraction::image::tiff_frame_ifd_offsets(content)
        .ok()
        .filter(|offsets| offsets.len() > 1)
}

/// A page count without byte ranges, for a document whose units are all pages.
pub(super) fn page_count_metadata(total: usize) -> PageStructure {
    PageStructure {
        total_count: u32::try_from(total).unwrap_or(u32::MAX),
        unit_type: PageUnitType::Page,
        boundaries: None,
        pages: None,
    }
}

impl ImageExtractor {
    /// OCR each frame as its own single-frame TIFF and join the results in frame order, tagging
    /// every element, page and OCR element with its frame number.
    pub(super) async fn ocr_tiff_frames(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        offsets: &[u64],
    ) -> Result<InternalDocument> {
        let mut merged: Option<InternalDocument> = None;
        let mut pages = Vec::with_capacity(offsets.len());
        let mut text_pages = Vec::new();
        let mut ocr_elements = Vec::new();
        for (index, offset) in offsets.iter().enumerate() {
            let page_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
            if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
                return Err(crate::XbergError::Cancelled);
            }
            let frame_bytes = crate::extraction::image::single_frame_tiff(content, *offset).ok_or_else(|| {
                crate::XbergError::parsing(format!("TIFF frame {page_number} has an unreadable directory"))
            })?;
            let mut frame = Box::pin(self.ocr_one_image(&frame_bytes, mime_type, config)).await?;
            drop(frame_bytes);

            let target = merged.get_or_insert_with(|| {
                let mut base = InternalDocument::new("image");
                base.metadata = std::mem::take(&mut frame.metadata);
                base
            });
            let mut page = append_frame(target, frame, page_number, &mut ocr_elements);
            page.page_number = page_number;
            if !page.content.trim().is_empty() {
                text_pages.push(page_number);
            }
            pages.push(page);
        }

        let mut doc = merged.unwrap_or_else(|| InternalDocument::new("image"));
        doc.metadata.pages = Some(page_count_metadata(pages.len()));
        doc.prebuilt_pages = Some(pages);
        doc.ocr_text_pages = text_pages;
        doc.prebuilt_ocr_elements = (!ocr_elements.is_empty()).then_some(ocr_elements);
        doc.mime_type = mime_type.to_string();
        Ok(doc)
    }
}

/// Appends one frame's content to `target`, tagged with its page number, and returns its page.
fn append_frame(
    target: &mut InternalDocument,
    mut frame: InternalDocument,
    page_number: u32,
    ocr_elements: &mut Vec<crate::types::ocr_elements::OcrElement>,
) -> crate::types::PageContent {
    let element_base = target.elements.len() as u32;
    let table_base = target.tables.len() as u32;
    let image_base = target.images.len() as u32;
    let frame_has_images = !frame.images.is_empty();

    for mut element in frame.elements {
        element.page = Some(page_number);
        match &mut element.kind {
            ElementKind::Table { table_index } => *table_index += table_base,
            ElementKind::Image { image_index } if frame_has_images => *image_index += image_base,
            _ => {}
        }
        target.push_element(element);
    }
    for mut relationship in frame.relationships {
        relationship.source += element_base;
        if let RelationshipTarget::Index(target_index) = &mut relationship.target {
            *target_index += element_base;
        }
        target.push_relationship(relationship);
    }
    target.tables.append(&mut frame.tables);
    target.images.append(&mut frame.images);
    target.formulas.append(&mut frame.formulas);
    target.processing_warnings.append(&mut frame.processing_warnings);
    for mut failure in frame.ocr_page_failures {
        failure.page = page_number;
        target.ocr_page_failures.push(failure);
    }
    if let Some(usage) = frame.llm_usage {
        target.llm_usage.get_or_insert_with(Vec::new).extend(usage);
    }
    for mut element in frame.prebuilt_ocr_elements.into_iter().flatten() {
        element.page_number = page_number;
        ocr_elements.push(element);
    }

    let frame_page = frame
        .prebuilt_pages
        .and_then(|frame_pages| frame_pages.into_iter().next());
    frame_page.unwrap_or_else(|| blank_page(page_number))
}

fn blank_page(page_number: u32) -> crate::types::PageContent {
    crate::types::PageContent {
        page_number,
        content: String::new(),
        tables: vec![],
        image_indices: vec![],
        image_preprocessing: None,
        hierarchy: None,
        is_blank: Some(true),
        layout_regions: None,
        speaker_notes: None,
        section_name: None,
        sheet_name: None,
        ocr_confidence: None,
        native_content: None,
        geometry: None,
        viewports: None,
    }
}
