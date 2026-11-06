use super::RunPageParagraphs;

#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
use crate::types::{BoundingBox, ExtractedDocument};

#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(super) fn supports(config: &crate::core::config::ExtractionConfig, page_count: Option<usize>) -> bool {
    #[cfg(feature = "layout-types")]
    if config.layout.is_some() {
        return false;
    }
    page_count == Some(1)
        && config.include_document_structure
        && !config.use_cache
        && config.output_format == crate::core::config::OutputFormat::Plain
        && config.content_filter.is_none()
}

#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(super) fn stock_mixed_read(read: &ExtractedDocument) -> bool {
    read.document.is_none()
        && read.raw_ocr_source_frame.is_none()
        && read.formulas.is_empty()
        && read.form_fields.is_empty()
        && read.annotations.as_ref().is_none_or(Vec::is_empty)
        && read.images.as_ref().is_none_or(Vec::is_empty)
        && read.tables.len() == 1
        && read.ocr_internal_document.as_ref().is_some_and(plain_text_block)
        && super::super::document::resolved_ocr_correction_degrees(&read.metadata).unwrap_or(0) == 0
}

#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn plain_text_block(doc: &crate::types::internal::InternalDocument) -> bool {
    use crate::types::{ContentLayer, internal::ElementKind, ocr_elements::OcrElementLevel};

    let [element] = doc.elements.as_slice() else {
        return false;
    };
    let text = element.text.trim();
    matches!(
        element.kind,
        ElementKind::OcrText {
            level: OcrElementLevel::Block
        }
    ) && element.depth == 0
        && element.layer == ContentLayer::Body
        && element.anchor.is_none()
        && element.annotations.is_empty()
        && !text.is_empty()
        && text.lines().count() == 1
        && !crate::pdf::structure::pipeline::looks_like_list_item(text)
        && !doc.ocr_text_claimed_by_tables
        && doc.relationships.is_empty()
        && doc.images.is_empty()
        && doc.tables.is_empty()
        && doc.diagrams.is_empty()
        && doc.uris.is_empty()
        && doc.annotations.as_ref().is_none_or(Vec::is_empty)
        && doc.form_fields.is_empty()
        && doc.formulas.is_empty()
        && doc.recorded_formulas.is_empty()
}

#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(super) fn place_table_in_render(
    read: &mut ExtractedDocument,
    render: (u32, u32),
    points: (f32, f32),
) -> Option<(f32, f32)> {
    let preprocessing = read.metadata.image_preprocessing.as_ref()?;
    let processed = super::super::document::processed_ocr_layout_dimensions(&read.metadata)?;
    let resized = preprocessing.new_dimensions?;
    let original = preprocessing.original_dimensions;
    if (original.width, original.height) != (render.0 as usize, render.1 as usize)
        || (resized.width, resized.height) != (processed.0 as usize, processed.1 as usize)
        || processed == render
        || ![points.0, points.1]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
    {
        return None;
    }
    let text_bbox = read.ocr_internal_document.as_ref()?.elements.first()?.bbox?;
    let table_bbox = read.tables.first_mut()?.bounding_box.as_mut()?;
    if !in_frame(text_bbox, processed) || !in_frame(*table_bbox, processed) || text_bbox.y1 > table_bbox.y0 {
        return None;
    }
    let sx = f64::from(render.0) / f64::from(processed.0);
    let sy = f64::from(render.1) / f64::from(processed.1);
    table_bbox.x0 *= sx;
    table_bbox.x1 *= sx;
    table_bbox.y0 *= sy;
    table_bbox.y1 *= sy;
    Some((points.0 / processed.0 as f32, points.1 / processed.1 as f32))
}

#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn in_frame(bbox: BoundingBox, (width, height): (u32, u32)) -> bool {
    0.0 <= bbox.x0
        && bbox.x0 < bbox.x1
        && bbox.x1 <= f64::from(width)
        && 0.0 <= bbox.y0
        && bbox.y0 < bbox.y1
        && bbox.y1 <= f64::from(height)
}

impl RunPageParagraphs {
    /// Detached paragraph geometry is carried separately from the already assembled document.
    pub(super) fn place_detached_in_points(&mut self) -> bool {
        let Some((sx, sy)) = self.detached_ocr_scale.take() else {
            return false;
        };
        for paragraph in &mut self.paragraphs {
            if let Some((left, bottom, right, top)) = paragraph.block_bbox {
                paragraph.block_bbox = Some((left * sx, bottom * sy, right * sx, top * sy));
            }
            for line in &mut paragraph.lines {
                line.baseline_y *= sy;
                for segment in &mut line.segments {
                    segment.x *= sx;
                    segment.width *= sx;
                    segment.y *= sy;
                    segment.height *= sy;
                    segment.baseline_y *= sy;
                }
            }
        }
        true
    }
}

#[cfg(all(test, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::core::config::{ExtractionConfig, OutputFormat};
    use crate::types::{
        Table,
        internal::{ElementKind, InternalDocument, InternalElement},
        ocr_elements::OcrElementLevel,
    };

    #[test]
    fn only_uncached_single_page_plain_structure_is_supported() {
        let mut config = ExtractionConfig {
            use_cache: false,
            include_document_structure: true,
            ..Default::default()
        };
        assert!(supports(&config, Some(1)));
        assert!(!supports(&config, Some(2)));
        assert!(!supports(&config, None));
        config.use_cache = true;
        assert!(!supports(&config, Some(1)));
        config.use_cache = false;
        config.output_format = OutputFormat::Markdown;
        assert!(!supports(&config, Some(1)));
    }

    #[test]
    fn single_rich_block_cannot_claim_detached_geometry() {
        let mut doc = InternalDocument::new("ocr");
        let block = ElementKind::OcrText {
            level: OcrElementLevel::Block,
        };
        doc.push_element(InternalElement::text(block, "Ledger summary", 0));
        let mut read = ExtractedDocument {
            ocr_internal_document: Some(doc),
            tables: vec![Table::default()],
            ..Default::default()
        };
        assert!(stock_mixed_read(&read));
        let element = &mut read.ocr_internal_document.as_mut().unwrap().elements[0];
        element.anchor = Some("ledger".to_string());
        assert!(!stock_mixed_read(&read));
        let element = &mut read.ocr_internal_document.as_mut().unwrap().elements[0];
        element.anchor = None;
        element.kind = ElementKind::Heading { level: 1 };
        assert!(!stock_mixed_read(&read));
        let element = &mut read.ocr_internal_document.as_mut().unwrap().elements[0];
        element.kind = block;
        element.text = "1. Listed item".to_string();
        assert!(!stock_mixed_read(&read));
    }

    #[test]
    fn detached_point_placement_never_consumes_the_lazy_pdf_scale() {
        let mut page = RunPageParagraphs {
            ocr_scale: Some((0.24, 0.24)),
            ..Default::default()
        };
        assert!(!page.place_detached_in_points());
        assert_eq!(page.ocr_scale, Some((0.24, 0.24)));
        page.detached_ocr_scale = Some((0.24, 0.24));
        assert!(page.place_detached_in_points());
        assert!(!page.place_detached_in_points());
    }
}
