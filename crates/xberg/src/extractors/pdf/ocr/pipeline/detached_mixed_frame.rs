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
        && !config.use_cache
        && config.output_format == crate::core::config::OutputFormat::Plain
        && config.content_filter.is_none()
        && config.images.is_none()
        && config.redaction.is_none()
}

#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(super) fn stock_mixed_read(read: &ExtractedDocument) -> bool {
    read.document.is_none()
        && read.raw_ocr_source_frame.is_none()
        && read.formulas.is_empty()
        && read.form_fields.is_empty()
        && read.annotations.as_ref().is_none_or(Vec::is_empty)
        && read.images.as_ref().is_none_or(Vec::is_empty)
        && matches!(read.tables.len(), 1 | 2)
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
        && element
            .attributes
            .as_ref()
            .is_none_or(|attrs| attrs.keys().all(|key| stock_block_attribute(key)))
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
fn stock_block_attribute(key: &str) -> bool {
    use crate::ocr::hocr_parser::{
        HOCR_BASELINE_CONST_ATTRIBUTE, HOCR_BASELINE_SLOPE_ATTRIBUTE, HOCR_BLOCK_ID_ATTRIBUTE,
        HOCR_BOLD_FRACTION_ATTRIBUTE, HOCR_FONT_SIZE_ATTRIBUTE, HOCR_ITALIC_FRACTION_ATTRIBUTE,
        HOCR_LINE_FONT_SIZES_ATTRIBUTE, HOCR_LINE_X_HEIGHTS_ATTRIBUTE, HOCR_X_ASCENDERS_ATTRIBUTE,
        HOCR_X_DESCENDERS_ATTRIBUTE, HOCR_X_HEIGHT_ATTRIBUTE,
    };

    matches!(
        key,
        HOCR_BASELINE_CONST_ATTRIBUTE
            | HOCR_BASELINE_SLOPE_ATTRIBUTE
            | HOCR_BLOCK_ID_ATTRIBUTE
            | HOCR_BOLD_FRACTION_ATTRIBUTE
            | HOCR_FONT_SIZE_ATTRIBUTE
            | HOCR_ITALIC_FRACTION_ATTRIBUTE
            | HOCR_LINE_FONT_SIZES_ATTRIBUTE
            | HOCR_LINE_X_HEIGHTS_ATTRIBUTE
            | HOCR_X_ASCENDERS_ATTRIBUTE
            | HOCR_X_DESCENDERS_ATTRIBUTE
            | HOCR_X_HEIGHT_ATTRIBUTE
    )
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
    if render.0 == 0
        || render.1 == 0
        || (original.width, original.height) != (render.0 as usize, render.1 as usize)
        || (resized.width, resized.height) != (processed.0 as usize, processed.1 as usize)
        || processed == render
        || ![points.0, points.1]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
    {
        return None;
    }
    let text_bbox = read.ocr_internal_document.as_ref()?.elements.first()?.bbox?;
    let boxes = read
        .tables
        .iter()
        .map(|table| table.bounding_box)
        .collect::<Option<Vec<_>>>()?;
    if !matches!(boxes.len(), 1 | 2)
        || !in_frame(text_bbox, processed)
        || boxes.iter().any(|bbox| !in_frame(*bbox, processed))
        || boxes.windows(2).any(|pair| pair[0].y1 >= pair[1].y0)
        || text_bbox.y1 > boxes.first()?.y0
    {
        return None;
    }
    let sx = f64::from(render.0) / f64::from(processed.0);
    let sy = f64::from(render.1) / f64::from(processed.1);
    for (table, bbox) in read.tables.iter_mut().zip(boxes) {
        table.bounding_box = Some(BoundingBox {
            x0: bbox.x0 * sx,
            x1: bbox.x1 * sx,
            y0: bbox.y0 * sy,
            y1: bbox.y1 * sy,
        });
    }
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
    fn default_plain_detached_mixed_read_also_owns_its_frames() {
        let config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };
        assert!(!config.include_document_structure);
        assert!(supports(&config, Some(1)));
    }

    #[test]
    fn explicit_redaction_or_image_extraction_disables_detached_placement() {
        let mut config = ExtractionConfig {
            use_cache: false,
            include_document_structure: true,
            ..Default::default()
        };
        assert!(supports(&config, Some(1)));
        config.redaction = Some(Default::default());
        assert!(!supports(&config, Some(1)));
        config.redaction = None;
        config.images = Some(Default::default());
        assert!(!supports(&config, Some(1)));
    }

    #[test]
    fn non_stock_block_attributes_cannot_claim_detached_geometry() {
        let mut doc = InternalDocument::new("ocr");
        let block = ElementKind::OcrText {
            level: OcrElementLevel::Block,
        };
        let mut text = InternalElement::text(block, "Ledger summary", 0);
        let attrs = text.attributes.get_or_insert_with(Default::default);
        attrs.insert(
            crate::ocr::hocr_parser::HOCR_X_HEIGHT_ATTRIBUTE.to_owned(),
            "12".to_owned(),
        );
        doc.push_element(text);
        assert!(plain_text_block(&doc));
        let attrs = doc.elements[0].attributes.as_mut().unwrap();
        attrs.insert("image_ref".to_owned(), "figure.png".to_owned());
        assert!(!plain_text_block(&doc));
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

    fn two_table_frame() -> ExtractedDocument {
        let mut document = InternalDocument::new("ocr");
        let mut text = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            "Ledger summary",
            0,
        );
        text.bbox = Some(BoundingBox {
            x0: 20.,
            y0: 10.,
            x1: 180.,
            y1: 30.,
        });
        document.push_element(text);
        let mut read = ExtractedDocument {
            ocr_internal_document: Some(document),
            ..Default::default()
        };
        for top in [60., 220.] {
            read.tables.push(Table {
                bounding_box: Some(BoundingBox {
                    x0: 20.,
                    y0: top,
                    x1: 180.,
                    y1: top + 100.,
                }),
                ..Default::default()
            });
        }
        read.metadata.additional.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
            200.into(),
        );
        read.metadata.additional.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
            400.into(),
        );
        read.metadata.image_preprocessing = Some(crate::types::ImagePreprocessingMetadata {
            original_dimensions: (400, 800).into(),
            original_dpi: (150., 150.).into(),
            target_dpi: 75,
            scale_factor: 0.5,
            auto_adjusted: false,
            final_dpi: 75,
            new_dimensions: Some((200, 400).into()),
            resample_method: "test".into(),
            dimension_clamped: false,
            calculated_dpi: None,
            skipped_resize: false,
            resize_error: None,
        });
        read
    }

    #[test]
    fn both_detached_table_boxes_use_the_processed_frame_once() {
        let mut read = two_table_frame();
        assert!(stock_mixed_read(&read));
        assert_eq!(
            place_table_in_render(&mut read, (400, 800), (200., 400.)),
            Some((1., 1.))
        );
        let bounds: Vec<_> = read.tables.iter().map(|table| table.bounding_box.unwrap()).collect();
        assert_eq!(
            bounds,
            [
                BoundingBox {
                    x0: 40.,
                    y0: 120.,
                    x1: 360.,
                    y1: 320.
                },
                BoundingBox {
                    x0: 40.,
                    y0: 440.,
                    x1: 360.,
                    y1: 640.
                },
            ]
        );
    }

    #[test]
    fn an_invalid_second_table_never_partially_rescales_the_first() {
        let mut read = two_table_frame();
        read.tables[1].bounding_box.as_mut().unwrap().x1 = 201.;
        let before = read.tables.iter().map(|table| table.bounding_box).collect::<Vec<_>>();
        assert!(place_table_in_render(&mut read, (400, 800), (200., 400.)).is_none());
        assert_eq!(
            read.tables.iter().map(|table| table.bounding_box).collect::<Vec<_>>(),
            before
        );
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
