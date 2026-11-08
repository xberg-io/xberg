use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use crate::extractors::security::SecurityLimits;
use crate::ocr::preprocessing::geometry_evidence::WordInventory;
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::types::document_structure::ContentLayer;
use crate::types::formats::OcrTableBoundingBox;
use crate::types::internal::{ElementKind, InternalDocument, RelationshipTarget};
use crate::types::{BoundingBox, ExtractedDocument, OcrElement, OcrElementLevel, Table};

use super::caption_grid_proof::{CaptionPaintProof, MAX_ELEMENTS, certify_figure_caption_rows};

pub(super) struct CaptionRewrite {
    grid: String,
    prose: String,
    cells: Vec<Vec<String>>,
    bounds: BoundingBox,
    frame: (u32, u32),
}

pub(super) fn eligible(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "layout-types")]
    if config.layout.is_some() {
        return false;
    }
    !config.use_cache
        && config.output_format == OutputFormat::Plain
        && config.redaction.is_none()
        && config.images.is_none()
        && config.ocr.as_ref().is_some_and(|ocr| {
            ocr.tesseract_config.is_none()
                && ocr.element_config.is_none()
                && !ocr.numeric_repair
                && !ocr.auto_rotate
                && ocr.effective_tesseract_language() == ["eng"]
        })
}

pub(super) fn proof_limits(config: &ExtractionConfig, ocr: &OcrConfig) -> SecurityLimits {
    let mut limits = config.security_limits.clone().unwrap_or_default();
    if let Some(nested) = &ocr.security_limits {
        limits.max_content_size = limits.max_content_size.min(nested.max_content_size);
        limits.max_iterations = limits.max_iterations.min(nested.max_iterations);
    }
    limits
}

pub(super) fn capture(
    result: &ExtractedDocument,
    frame: (u32, u32),
    limits: &mut SecurityLimits,
    inventory: WordInventory,
) -> Option<(CaptionRewrite, CaptionPaintProof)> {
    let [table] = result.tables.as_slice() else { return None };
    if result.document.is_some()
        || result.raw_ocr_source_frame.is_some()
        || !result.formulas.is_empty()
        || super::document::resolved_ocr_correction_degrees(&result.metadata).is_some_and(|angle| angle != 0)
    {
        return None;
    }
    let metadata = &result.metadata.additional;
    if metadata.get("psm")?.as_str()? != "11"
        || metadata.get(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)?.as_u64()? != u64::from(frame.0)
        || metadata.get(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)?.as_u64()? != u64::from(frame.1)
    {
        return None;
    }
    let bounds = table.bounding_box.as_ref()?;
    let pixel =
        |value: f64| (value >= 0.0 && value <= f64::from(u32::MAX) && value.fract() == 0.0).then_some(value as u32);
    let region = OcrTableBoundingBox {
        left: pixel(bounds.x0)?,
        top: pixel(bounds.y0)?,
        right: pixel(bounds.x1)?,
        bottom: pixel(bounds.y1)?,
    };
    let elements = result.ocr_elements.as_deref()?;
    if !has_complete_iterator_words(elements, inventory, &mut limits.max_iterations) {
        return None;
    }
    let page = elements.first()?.page_number;
    let proof = certify_figure_caption_rows(&table.cells, page, region, elements, frame, limits)?;
    const MAX_CAPTION_BYTES: usize = 16 * 1024;
    let grid = table.markdown.trim_end_matches('\n');
    if grid.is_empty() || grid.len() > limits.max_content_size.min(MAX_CAPTION_BYTES) {
        return None;
    }
    let prose = proof.lines().join("\n");
    (prose.len() <= grid.len()).then(|| {
        (
            CaptionRewrite {
                grid: grid.to_owned(),
                prose,
                cells: table.cells.clone(),
                bounds: *bounds,
                frame,
            },
            proof.paint,
        )
    })
}

pub(super) async fn verify_paint(
    candidate: Option<(CaptionRewrite, CaptionPaintProof)>,
    raster: std::sync::Arc<Vec<u8>>,
    retained: usize,
    limits: SecurityLimits,
    cancel: Option<crate::cancellation::CancellationToken>,
) -> Option<CaptionRewrite> {
    let (rewrite, proof) = candidate?;
    let retained = retained.checked_add(raster.capacity())?;
    match tokio::task::spawn_blocking(move || {
        !cancel.is_some_and(|token| token.is_cancelled()) && proof.certify(&raster, rewrite.frame, retained, &limits)
    })
    .await
    {
        Ok(true) => Some(rewrite),
        Ok(false) => None,
        Err(error) => {
            tracing::warn!(%error, "Unable to verify PDF caption source pixels");
            None
        }
    }
}

fn has_complete_iterator_words(elements: &[OcrElement], inventory: WordInventory, work: &mut usize) -> bool {
    if elements.len() > MAX_ELEMENTS
        || inventory.skipped != 0
        || inventory.count == 0
        || inventory.count > elements.len()
    {
        return false;
    }
    let Some(remaining) = work.checked_sub(elements.len()) else {
        return false;
    };
    *work = remaining;
    let mut count = 0;
    for word in elements.iter().filter(|element| element.level == OcrElementLevel::Word) {
        if word.backend_metadata.get("backend").and_then(serde_json::Value::as_str) != Some("tesseract-iterator") {
            return false;
        }
        count += 1;
    }
    count == inventory.count
}

impl CaptionRewrite {
    pub(super) fn bind_page(mut self, raster: (u32, u32), points: (f32, f32)) -> Option<Self> {
        if self.frame != raster
            || raster.0 == 0
            || raster.1 == 0
            || !points.0.is_finite()
            || !points.1.is_finite()
            || points.0 <= 0.0
            || points.1 <= 0.0
        {
            return None;
        }
        let mut geometry = Table {
            bounding_box: Some(self.bounds),
            ..Default::default()
        };
        super::document::rescale_ocr_bboxes_to_page_points(
            None,
            std::slice::from_mut(&mut geometry),
            raster.0,
            raster.1,
            points.0,
            points.1,
        );
        self.bounds = geometry.bounding_box?;
        Some(self)
    }

    pub(super) fn matches_selected(&self, result: &ExtractedDocument) -> bool {
        matches!(result.tables.as_slice(), [table] if self.owns(table))
            && result.raw_ocr_source_frame.is_none()
            && super::document::resolved_ocr_correction_degrees(&result.metadata).is_none_or(|angle| angle == 0)
    }

    fn owns(&self, table: &Table) -> bool {
        table.bounding_box == Some(self.bounds)
            && table.cells == self.cells
            && table.markdown.trim_end_matches('\n') == self.grid
            && table
                .columns
                .as_ref()
                .is_none_or(|columns| self.cells.first() == Some(columns))
            && table.cell_styles.is_empty()
    }

    fn grid_range(&self, text: &str) -> Option<std::ops::Range<usize>> {
        if self.grid.is_empty() {
            return None;
        }
        let mut matches = text.match_indices(&self.grid);
        let (start, matched) = matches.next()?;
        let end = start + matched.len();
        (matches.next().is_none()
            && (start == 0 || text.as_bytes()[start - 1] == b'\n')
            && (end == text.len() || text.as_bytes()[end] == b'\n'))
            .then_some(start..end)
    }

    pub(super) fn apply(&self, document: &mut InternalDocument, text: &mut String, page: u32) -> Option<()> {
        let range = self.grid_range(text)?;
        let mut candidates = document.tables.iter().enumerate().filter(|(_, table)| self.owns(table));
        let (index, table) = candidates.next()?;
        if candidates.next().is_some()
            || table.page_number != page
            || table.table_id.as_ref().is_some_and(|id| {
                document
                    .tables
                    .iter()
                    .enumerate()
                    .any(|(other, table)| other != index && table.table_id.as_ref() == Some(id))
            })
        {
            return None;
        }
        let mut owners = document.elements.iter().enumerate().filter(
            |(_, element)| matches!(element.kind, ElementKind::Table { table_index } if table_index as usize == index),
        );
        let (owner, node) = owners.next()?;
        if owners.next().is_some()
            || node.page != Some(page)
            || node.bbox != Some(self.bounds)
            || node.layer != ContentLayer::Body
            || node.depth != 0
            || !node.text.is_empty()
            || !node.annotations.is_empty()
            || node.attributes.is_some()
            || node.anchor.is_some()
            || document.relationships.iter().any(|relationship| {
                relationship.source as usize == owner
                    || matches!(relationship.target, RelationshipTarget::Index(target) if target as usize == owner)
            })
        {
            return None;
        }
        let flattened = crate::rendering::common::render_table_plain(&table.cells);
        let plain = flattened.strip_suffix('\n')?.to_owned();
        text.replace_range(range, &self.prose);
        document.tables.remove(index);
        let node = &mut document.elements[owner];
        node.kind = ElementKind::Paragraph;
        node.text = plain;
        for element in &mut document.elements {
            if let ElementKind::Table { table_index } = &mut element.kind
                && *table_index as usize > index
            {
                *table_index -= 1;
            }
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal::InternalElement;

    fn document() -> (InternalDocument, CaptionRewrite, String) {
        let cells = vec![
            vec!["Figure 4:".into(), "A caption".into()],
            vec!["Continues here.".into(), String::new()],
        ];
        let grid = "| Figure 4: | A caption |\n| --- | --- |\n| Continues here. |  |";
        let caption = CaptionRewrite {
            grid: grid.to_owned(),
            prose: "Figure 4: A caption\nContinues here.".into(),
            cells: cells.clone(),
            bounds: BoundingBox {
                x0: 5.0,
                y0: 10.0,
                x1: 50.0,
                y1: 20.0,
            },
            frame: (100, 100),
        };
        let mut document = InternalDocument::new("pdf");
        let table_index = document.push_table(Table {
            cells,
            markdown: format!("{grid}\n"),
            page_number: 9,
            bounding_box: Some(caption.bounds),
            ..Default::default()
        });
        let mut element = InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(9);
        element.bbox = Some(caption.bounds);
        document.push_element(element);
        let table_index = document.push_table(Table {
            cells: vec![
                vec!["Invoice".into(), "Total".into()],
                vec!["1042".into(), "385.20".into()],
                vec!["3124".into(), "15.00".into()],
            ],
            page_number: 9,
            ..Default::default()
        });
        document.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(9));
        (document, caption, format!("Before\n\n{grid}\n\nAfter"))
    }

    #[test]
    fn replaces_caption_in_place_and_keeps_a_distinct_numeric_table_and_plain_bytes() {
        let (mut doc, proof, mut page) = document();
        let initial = crate::rendering::render_plain(&doc);
        let invoice = serde_json::to_value(&doc.tables[1]).unwrap();
        assert!(proof.apply(&mut doc, &mut page, 9).is_some());
        assert_eq!(page, "Before\n\nFigure 4: A caption\nContinues here.\n\nAfter");
        assert_eq!(crate::rendering::render_plain(&doc), initial);
        assert_eq!(doc.tables.len(), 1);
        assert_eq!(serde_json::to_value(&doc.tables[0]).unwrap(), invoice);
        assert!(matches!(doc.elements[0].kind, ElementKind::Paragraph));
        assert!(matches!(doc.elements[1].kind, ElementKind::Table { table_index: 0 }));
    }

    fn captured_result() -> (ExtractedDocument, WordInventory, (u32, u32)) {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ocr_caption_2530/p9-iterator.json"
        ))
        .unwrap();
        let ocr_table: crate::types::formats::OcrTable = serde_json::from_value(fixture["table"].clone()).unwrap();
        let mut elements: Vec<OcrElement> = serde_json::from_value(fixture["elements"].clone()).unwrap();
        let mut count = 0;
        for word in elements
            .iter_mut()
            .filter(|element| element.level == OcrElementLevel::Word)
        {
            word.backend_metadata
                .insert("backend".into(), "tesseract-iterator".into());
            count += 1;
        }
        let frame = (2586, 3346);
        let region = ocr_table.bounding_box.unwrap();
        let table = Table {
            cells: ocr_table.cells,
            markdown: ocr_table.markdown,
            page_number: ocr_table.page_number,
            bounding_box: Some(BoundingBox {
                x0: f64::from(region.left),
                y0: f64::from(region.top),
                x1: f64::from(region.right),
                y1: f64::from(region.bottom),
            }),
            ..Default::default()
        };
        let mut result = ExtractedDocument {
            tables: vec![table],
            ocr_elements: Some(elements),
            ..Default::default()
        };
        let additional = &mut result.metadata.additional;
        additional.insert("psm".into(), "11".into());
        additional.insert(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), frame.0.into());
        additional.insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), frame.1.into());
        (result, WordInventory { count, skipped: 0 }, frame)
    }

    #[test]
    fn captures_a_certified_wrapped_caption_as_its_source_lines() {
        let (result, inventory, frame) = captured_result();
        let mut limits = SecurityLimits::default();
        let (rewrite, _) = capture(&result, frame, &mut limits, inventory)
            .expect("a certified wrapped caption must reach the conversion");
        let lines: Vec<_> = result
            .ocr_elements
            .as_ref()
            .unwrap()
            .iter()
            .filter(|element| element.level == OcrElementLevel::Line)
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(lines.len(), 6);
        assert_eq!(rewrite.prose, lines.join("\n"));
        assert_eq!(rewrite.grid, result.tables[0].markdown.trim_end_matches('\n'));
        assert_eq!(rewrite.cells, result.tables[0].cells);

        let mut untrusted = result.clone();
        untrusted.metadata.additional.insert("psm".into(), "3".into());
        assert!(capture(&untrusted, frame, &mut SecurityLimits::default(), inventory).is_none());
    }

    #[test]
    fn rejects_omitted_or_untrusted_iterator_words_and_typed_skips() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ocr_caption_2530/p9-iterator.json"
        ))
        .unwrap();
        let mut elements: Vec<OcrElement> = serde_json::from_value(fixture["elements"].clone()).unwrap();
        // The compact geometry fixture omits per-word backend metadata. Give its
        // test words the stock provenance marker before testing transport loss.
        let mut count = 0;
        for word in elements
            .iter_mut()
            .filter(|element| element.level == OcrElementLevel::Word)
        {
            word.backend_metadata
                .insert("backend".into(), "tesseract-iterator".into());
            count += 1;
        }
        let complete = WordInventory { count, skipped: 0 };
        assert_eq!(count, 85);
        let mut work = elements.len();
        assert!(has_complete_iterator_words(&elements, complete, &mut work));
        assert_eq!(work, 0);
        assert!(!has_complete_iterator_words(&elements, complete, &mut work));
        let oversized = vec![elements[0].clone(); MAX_ELEMENTS + 1];
        let mut work = MAX_ELEMENTS + 1;
        assert!(!has_complete_iterator_words(&oversized, complete, &mut work));
        assert_eq!(work, MAX_ELEMENTS + 1);
        let mut work = SecurityLimits::default().max_iterations;
        assert!(!has_complete_iterator_words(
            &elements,
            WordInventory { count, skipped: 1 },
            &mut work,
        ));
        let endpoint = elements.iter().rposition(|e| e.level == OcrElementLevel::Word).unwrap();
        let mut endpoint_word = elements.remove(endpoint);
        assert!(!has_complete_iterator_words(&elements, complete, &mut work));
        endpoint_word.backend_metadata.remove("backend");
        elements.insert(endpoint, endpoint_word);
        assert!(!has_complete_iterator_words(&elements, complete, &mut work));
    }

    #[test]
    fn caption_budget_uses_both_nested_limits_and_outer_defaults() {
        let mut config = ExtractionConfig::default();
        let defaults = SecurityLimits::default();
        let ocr = OcrConfig {
            security_limits: Some(SecurityLimits {
                max_content_size: defaults.max_content_size + 1,
                max_iterations: 10,
                ..defaults.clone()
            }),
            ..Default::default()
        };
        let effective = proof_limits(&config, &ocr);
        assert_eq!(effective.max_content_size, defaults.max_content_size);
        assert_eq!(effective.max_iterations, 10);

        config.security_limits = Some(SecurityLimits {
            max_content_size: 4096,
            ..defaults
        });
        assert_eq!(proof_limits(&config, &ocr).max_content_size, 4096);
        assert_eq!(proof_limits(&config, &OcrConfig::default()).max_content_size, 4096);
    }

    #[test]
    fn binds_only_to_the_selected_ocr_table_in_its_original_frame() {
        let (mut doc, proof, _) = document();
        let mut selected = ExtractedDocument {
            tables: vec![doc.tables.remove(0)],
            ..Default::default()
        };
        assert!(proof.matches_selected(&selected));
        selected.tables[0].bounding_box.as_mut().unwrap().y0 += 1.0;
        assert!(!proof.matches_selected(&selected));
    }

    #[test]
    fn rejects_displaced_table_asset_and_node_without_modifying_either() {
        for (table_dx, node_dx) in [(5.0, 0.0), (0.0, 5.0), (5.0, 5.0)] {
            let (mut doc, proof, mut page) = document();
            doc.tables[0].bounding_box.as_mut().unwrap().x0 += table_dx;
            doc.elements[0].bbox.as_mut().unwrap().x0 += node_dx;
            let original = serde_json::to_value(&doc).unwrap();
            let original_page = page.clone();
            assert!(proof.apply(&mut doc, &mut page, 9).is_none());
            assert_eq!(serde_json::to_value(&doc).unwrap(), original);
            assert_eq!(page, original_page);
        }
    }

    #[test]
    fn refuses_a_shared_table_asset_identifier_or_relationship() {
        for shared_identifier in [false, true] {
            let (mut doc, proof, mut page) = document();
            if shared_identifier {
                doc.tables[0].table_id = Some("table-1".into());
                doc.tables[1].table_id = Some("table-1".into());
            } else {
                doc.relationships.push(crate::types::internal::Relationship {
                    source: 1,
                    target: RelationshipTarget::Index(0),
                    kind: crate::types::internal::RelationshipKind::Caption,
                });
            }
            let original = serde_json::to_value(&doc).unwrap();
            let original_page = page.clone();
            assert!(proof.apply(&mut doc, &mut page, 9).is_none());
            assert_eq!(serde_json::to_value(&doc).unwrap(), original);
            assert_eq!(page, original_page);
        }
    }

    #[test]
    fn leaves_ambiguous_page_text_and_a_different_attempt_unchanged() {
        for different_attempt in [false, true] {
            let (mut doc, proof, mut page) = document();
            if different_attempt {
                doc.tables[0].cells[1][0].push('!');
            } else {
                page.push_str("\n\n");
                page.push_str(&proof.grid);
            }
            let original = serde_json::to_value(&doc).unwrap();
            let original_page = page.clone();
            assert!(proof.apply(&mut doc, &mut page, 9).is_none());
            assert_eq!(serde_json::to_value(&doc).unwrap(), original);
            assert_eq!(page, original_page);
        }
    }
}
