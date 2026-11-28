//! Multi-frame TIFF OCR: every frame is read on its own and becomes one page (#3447).

use super::ImageExtractor;
use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::image_tiff::{TiffIfdWalk, TiffWalkEnd, single_frame_tiff};
use crate::types::internal::{ElementKind, InternalDocument, InternalElementId, RelationshipTarget};
use crate::types::page::{PageStructure, PageUnitType};
use crate::types::{OcrPageFailure, ProcessingWarning};

const WARNING_SOURCE: &str = "tiff_frames";

/// Whether the file is read frame by frame: it holds several frames, or one frame followed by a
/// directory that could not be read (which must be reported rather than dropped).
pub(super) fn reads_by_frame(walk: &TiffIfdWalk) -> bool {
    walk.offsets.len() > 1 || (walk.offsets.len() == 1 && matches!(walk.end, TiffWalkEnd::Broken(_)))
}

/// The warning for a frame chain that ends at an unreadable directory.
pub(super) fn walk_warning(walk: &TiffIfdWalk) -> Option<ProcessingWarning> {
    let TiffWalkEnd::Broken(reason) = &walk.end else {
        return None;
    };
    Some(ProcessingWarning {
        source: std::borrow::Cow::Borrowed(WARNING_SOURCE),
        message: std::borrow::Cow::Owned(format!(
            "TIFF frame {} and any frames after it were not read: {reason}; {} frames were read",
            walk.offsets.len() + 1,
            walk.offsets.len()
        )),
    })
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
    /// every element, table, page and OCR element with its frame number.
    ///
    /// Frames are read one at a time: each holds a full copy of the file and a decoded raster,
    /// so reading them concurrently would multiply the peak by the thread budget. A frame whose
    /// OCR fails is recorded in `ocr_page_failures` and leaves a blank page; the remaining
    /// frames are still read. When every frame fails, the last error is returned.
    pub(super) async fn ocr_tiff_frames(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        walk: &TiffIfdWalk,
    ) -> Result<InternalDocument> {
        let mut doc = InternalDocument::new("image");
        let mut pages = Vec::with_capacity(walk.offsets.len());
        let mut text_pages = Vec::new();
        let mut ocr_elements = Vec::new();
        let mut stats = OcrStats::default();
        let mut last_error = None;
        for (index, offset) in walk.offsets.iter().enumerate() {
            let page_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
            let read = match single_frame_tiff(content, *offset) {
                Some(frame_bytes) => Box::pin(self.ocr_one_image(&frame_bytes, mime_type, config)).await,
                None => Err(crate::XbergError::parsing(format!(
                    "TIFF frame {page_number} has an unreadable directory"
                ))),
            };
            let frame = match read {
                Ok(frame) => frame,
                Err(error @ crate::XbergError::Cancelled) => return Err(error),
                Err(error) => {
                    tracing::warn!(page = page_number, %error, "TIFF frame OCR failed; continuing with the next frame");
                    doc.ocr_page_failures.push(OcrPageFailure {
                        page: page_number,
                        error: error.to_string(),
                        recovered: false,
                    });
                    doc.processing_warnings.push(ProcessingWarning {
                        source: std::borrow::Cow::Borrowed(WARNING_SOURCE),
                        message: std::borrow::Cow::Owned(format!(
                            "TIFF frame {page_number} could not be read: {error}"
                        )),
                    });
                    last_error = Some(error);
                    pages.push(blank_page(page_number));
                    continue;
                }
            };
            stats.add(&frame.metadata);
            if stats.frames == 1 {
                doc.metadata = frame.metadata.clone();
            }
            let mut page = append_frame(&mut doc, frame, page_number, &mut ocr_elements);
            page.page_number = page_number;
            if !page.content.trim().is_empty() {
                text_pages.push(page_number);
            }
            pages.push(page);
        }
        if stats.frames == 0
            && let Some(error) = last_error
        {
            return Err(error);
        }

        stats.write_into(&mut doc.metadata);
        doc.processing_warnings.extend(walk_warning(walk));
        doc.metadata.pages = Some(page_count_metadata(pages.len()));
        doc.prebuilt_pages = Some(pages);
        doc.ocr_text_pages = text_pages;
        doc.prebuilt_ocr_elements = (!ocr_elements.is_empty()).then_some(ocr_elements);
        doc.mime_type = mime_type.to_string();
        Ok(doc)
    }
}

/// Word-weighted OCR figures across the frames that were read.
#[derive(Default)]
struct OcrStats {
    frames: usize,
    words: u64,
    confidence_weight: f64,
    confidence_sum: f64,
}

impl OcrStats {
    fn add(&mut self, metadata: &crate::types::Metadata) {
        self.frames += 1;
        let words = metadata
            .additional
            .get("word_count")
            .and_then(serde_json::Value::as_u64);
        self.words += words.unwrap_or(0);
        // -1 is Tesseract's "no confidence available" sentinel.
        let confidence = metadata
            .additional
            .get("mean_text_conf")
            .and_then(serde_json::Value::as_f64)
            .filter(|confidence| *confidence >= 0.0);
        if let Some(confidence) = confidence {
            let weight = words.unwrap_or(1).max(1) as f64;
            self.confidence_weight += weight;
            self.confidence_sum += confidence * weight;
        }
    }

    /// Replace the first frame's word count and mean confidence with the whole document's.
    fn write_into(&self, metadata: &mut crate::types::Metadata) {
        if metadata.additional.contains_key("word_count") {
            metadata
                .additional
                .insert("word_count".into(), serde_json::Value::from(self.words));
        }
        if self.confidence_weight > 0.0 && metadata.additional.contains_key("mean_text_conf") {
            metadata.additional.insert(
                "mean_text_conf".into(),
                serde_json::Value::from(self.confidence_sum / self.confidence_weight),
            );
        }
        metadata
            .additional
            .insert("ocr_frames_read".into(), serde_json::Value::from(self.frames as u64));
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
    // Region-only layout OCR frames carry text elements but no prebuilt page.
    let fallback_content = frame
        .prebuilt_pages
        .is_none()
        .then(|| crate::rendering::render_plain(&frame));

    for mut element in frame.elements {
        element.page = Some(page_number);
        match &mut element.kind {
            ElementKind::Table { table_index } => *table_index += table_base,
            ElementKind::Image { image_index } if frame_has_images => *image_index += image_base,
            _ => {}
        }
        let merged_index = target.elements.len() as u32;
        element.id =
            InternalElementId::generate(element.kind.discriminant(), &element.text, element.page, merged_index);
        target.push_element(element);
    }
    for mut relationship in frame.relationships {
        relationship.source += element_base;
        if let RelationshipTarget::Index(target_index) = &mut relationship.target {
            *target_index += element_base;
        }
        target.push_relationship(relationship);
    }
    for table in &mut frame.tables {
        table.page_number = page_number;
    }
    target.tables.append(&mut frame.tables);
    target.images.append(&mut frame.images);
    for formula in &mut frame.formulas {
        formula.page = Some(page_number);
    }
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
    let mut page = match (frame_page, fallback_content) {
        (Some(page), _) => page,
        (None, Some(content)) => {
            let mut page = blank_page(page_number);
            page.is_blank = Some(content.trim().is_empty());
            page.content = content;
            page
        }
        (None, None) => blank_page(page_number),
    };
    page.tables = page
        .tables
        .iter()
        .map(|table| {
            let mut table = crate::types::Table::clone(table);
            table.page_number = page_number;
            std::sync::Arc::new(table)
        })
        .collect();
    if frame_has_images {
        for index in &mut page.image_indices {
            *index += image_base;
        }
    }
    page
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

#[cfg(test)]
#[path = "image_tiff_frames_tests.rs"]
mod tests;
