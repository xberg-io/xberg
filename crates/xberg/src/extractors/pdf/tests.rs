use super::*;
#[cfg(feature = "ocr")]
use crate::core::config::OcrQualityThresholds;
#[cfg(all(feature = "pdf", feature = "ocr"))]
use serial_test::serial;

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
mod formula_insertion_tests {
    use super::*;
    use crate::core::config::OutputFormat;
    use crate::types::document_structure::NodeContent;
    use crate::types::internal::{Relationship, RelationshipKind, RelationshipTarget};

    fn paragraph(text: &str, page: u32) -> InternalElement {
        let mut element = InternalElement::text(ElementKind::Paragraph, text, 0).with_page(page);
        element.bbox = Some(crate::types::BoundingBox {
            x0: 0.0,
            y0: 100.0,
            x1: 100.0,
            y1: 150.0,
        });
        element
    }

    fn formula(page: u32) -> crate::types::Formula {
        crate::types::Formula {
            latex: "x^{2}+1".to_string(),
            page: Some(page),
            bbox: Some(crate::types::BoundingBox {
                x0: 0.0,
                y0: 200.0,
                x1: 100.0,
                y1: 250.0,
            }),
        }
    }

    #[test]
    fn consecutive_empty_pages_keep_all_element_ids_unique() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("Source on third page", 3));
        doc.push_element(paragraph("Target on third page", 3));
        let old_ids: Vec<_> = doc.elements.iter().map(|element| element.id.clone()).collect();
        doc.push_relationship(Relationship {
            source: 0,
            target: RelationshipTarget::Index(1),
            kind: RelationshipKind::CrossReference,
        });
        insert_missing_formula_elements(&mut doc, vec![(formula(1), true), (formula(2), true)]);
        let retained: Vec<_> = doc
            .elements
            .iter()
            .filter(|element| element.page == Some(3))
            .map(|element| element.id.clone())
            .collect();
        assert_eq!(retained, old_ids);
        let relationship = &doc.relationships[0];
        assert_eq!(doc.elements[relationship.source as usize].id, old_ids[0]);
        let RelationshipTarget::Index(target) = relationship.target else {
            panic!("indexed target")
        };
        assert_eq!(doc.elements[target as usize].id, old_ids[1]);
        let unique: std::collections::BTreeSet<_> = doc.elements.iter().map(|element| element.id.to_string()).collect();
        assert_eq!(
            unique.len(),
            doc.elements.len(),
            "new page boundaries must have unique IDs"
        );
    }

    #[test]
    fn formula_above_first_page_element_stays_after_its_page_break() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("First page prose", 1));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
        doc.push_element(paragraph("Second page prose", 2));
        insert_missing_formula_elements(&mut doc, vec![(formula(2), true)]);
        let result = crate::extraction::derive::derive_extraction_result(doc, true, OutputFormat::DocTags);
        let rendered = result.formatted_content.as_deref().expect("rendered DocTags");
        let boundary = rendered.find("<page_break>").expect("existing page boundary");
        let math = rendered.find("<formula>").expect("recognized formula");
        assert!(
            boundary < math,
            "DocTags must put page-two math after the page boundary: {}",
            rendered
        );
        assert_eq!(rendered.matches("<page_break>").count(), 1);
        let structure = result.document.expect("derived structure");
        let boundary = structure
            .nodes
            .iter()
            .position(|node| matches!(node.content, NodeContent::PageBreak))
            .unwrap();
        let math = structure
            .nodes
            .iter()
            .position(|node| matches!(node.content, NodeContent::Formula { .. }))
            .unwrap();
        assert!(boundary < math);
        assert_eq!(structure.nodes[math].page, Some(2));
        assert!(rendered.contains("Second page prose"));
    }

    fn list_document(start_page: Option<u32>, first_item_above_math: bool, ordered: bool) -> InternalDocument {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("First page prose", 1));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
        let mut start = InternalElement::text(ElementKind::ListStart { ordered }, "", 0);
        start.page = start_page;
        doc.push_element(start);
        for (index, text) in ["First retained item", "Second retained item"].into_iter().enumerate() {
            let mut item = paragraph(text, 2);
            item.kind = ElementKind::ListItem { ordered };
            item.depth = 1;
            if index == 0 && first_item_above_math {
                item.bbox = Some(crate::types::BoundingBox {
                    x0: 0.0,
                    y0: 300.0,
                    x1: 100.0,
                    y1: 350.0,
                });
            }
            doc.push_element(item);
        }
        doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));
        doc
    }

    fn assert_formula_list_integrity(doc: InternalDocument, math_after_list: bool, ordered: bool) {
        let result = crate::extraction::derive::derive_extraction_result(doc, true, OutputFormat::Markdown);
        let rendered = result.formatted_content.as_deref().expect("rendered Markdown");
        let first = rendered.find("First retained item").expect("first item retained");
        let second = rendered.find("Second retained item").expect("second item retained");
        let math = rendered.find("x^{2}+1").expect("math retained");
        assert_eq!(
            first < math,
            math_after_list,
            "container-boundary placement: {rendered}"
        );
        assert!(first < second, "existing item order must be retained: {rendered}");
        assert_eq!(
            second < math,
            math_after_list,
            "math must stay outside the intact list: {rendered}"
        );
        if ordered {
            assert!(
                rendered.contains("1. First retained item"),
                "first ordinal retained: {rendered}"
            );
            assert!(
                rendered.contains("2. Second retained item"),
                "second ordinal retained: {rendered}"
            );
        }
        let structure = result.document.expect("derived structure");
        let items: Vec<_> = structure
            .nodes
            .iter()
            .filter(|node| matches!(node.content, NodeContent::ListItem { .. }))
            .collect();
        assert_eq!(items.len(), 2);
        for item in items {
            let parent = item.parent.expect("every original item retains a list parent");
            assert!(matches!(
                structure.nodes[parent.0 as usize].content,
                NodeContent::List { .. }
            ));
        }
        let math = structure
            .nodes
            .iter()
            .find(|node| matches!(node.content, NodeContent::Formula { .. }))
            .unwrap();
        assert_eq!(
            math.parent, None,
            "no source evidence assigns standalone detected math to a list item"
        );
        assert_eq!(
            structure
                .nodes
                .iter()
                .filter(|node| matches!(node.content, NodeContent::List { .. }))
                .count(),
            1,
            "the original list must not be split"
        );
        for list in structure
            .nodes
            .iter()
            .filter(|node| matches!(node.content, NodeContent::List { .. }))
        {
            assert!(!list.children.is_empty(), "the original list must retain its children");
            assert!(
                list.children
                    .iter()
                    .all(|index| matches!(structure.nodes[index.0 as usize].content, NodeContent::ListItem { .. }))
            );
        }
    }

    #[test]
    fn formula_between_list_items_preserves_both_items_container_ownership() {
        let mut doc = list_document(None, true, false);
        insert_missing_formula_elements(&mut doc, vec![(formula(2), true)]);
        assert_formula_list_integrity(doc, true, false);
    }

    #[test]
    fn formula_above_page_attributed_list_start_keeps_container_ownership() {
        let mut doc = list_document(Some(2), false, false);
        insert_missing_formula_elements(&mut doc, vec![(formula(2), true)]);
        assert_formula_list_integrity(doc, false, false);
    }

    #[test]
    fn formula_between_ordered_items_preserves_single_list_and_numbering() {
        let mut doc = list_document(Some(2), true, true);
        insert_missing_formula_elements(&mut doc, vec![(formula(2), true)]);
        assert_formula_list_integrity(doc, true, true);
    }

    #[test]
    fn formulas_deferred_after_one_container_keep_recognition_order() {
        let mut doc = list_document(Some(2), true, true);
        let first = formula(2);
        let mut second = formula(2);
        second.latex = "y^{2}+1".to_string();
        second.bbox = Some(crate::types::BoundingBox {
            x0: 0.0,
            y0: 160.0,
            x1: 100.0,
            y1: 190.0,
        });
        doc.formulas = vec![first.clone(), second.clone()];
        let side_channel = serde_json::to_value(&doc.formulas).unwrap();
        insert_missing_formula_elements(&mut doc, vec![(first, true), (second, true)]);
        assert_eq!(serde_json::to_value(&doc.formulas).unwrap(), side_channel);
        assert_formula_list_integrity(doc.clone(), true, true);
        let result = crate::extraction::derive::derive_extraction_result(doc, true, OutputFormat::Markdown);
        assert_eq!(serde_json::to_value(&result.formulas).unwrap(), side_channel);
        let rendered = result.formatted_content.as_deref().expect("rendered Markdown");
        let first = rendered.find("x^{2}+1").expect("first recognized formula");
        let second = rendered.find("y^{2}+1").expect("second recognized formula");
        assert!(
            first < second,
            "shared-boundary insertion must retain recognition order: {rendered}"
        );
        let structure = result.document.expect("derived structure");
        let formulas: Vec<_> = structure
            .nodes
            .iter()
            .filter_map(|node| match &node.content {
                NodeContent::Formula { text } => {
                    assert_eq!(node.parent, None);
                    assert_eq!(node.page, Some(2));
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(formulas, ["x^{2}+1", "y^{2}+1"]);
    }

    #[test]
    fn deferred_formula_batch_keeps_original_following_formula_after_new_math() {
        let mut doc = list_document(Some(2), true, true);
        let mut original = InternalElement::text(ElementKind::Formula, "c^{2}+1", 0).with_page(2);
        original.bbox = Some(crate::types::BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        });
        let original_id = original.id.clone();
        doc.push_element(original);
        insert_missing_formula_elements(&mut doc, vec![(formula(2), true)]);
        assert_formula_list_integrity(doc.clone(), true, true);
        assert_eq!(
            doc.elements
                .iter()
                .find(|element| element.text == "c^{2}+1")
                .unwrap()
                .id,
            original_id
        );
        let result = crate::extraction::derive::derive_extraction_result(doc, true, OutputFormat::Markdown);
        let rendered = result.formatted_content.as_deref().expect("rendered Markdown");
        let recognized = rendered.find("x^{2}+1").expect("new recognized formula");
        let original = rendered.find("c^{2}+1").expect("original formula");
        assert!(
            recognized < original,
            "deferral must not jump over original following math: {rendered}"
        );
        let structure = result.document.expect("derived structure");
        let formulas: Vec<_> = structure
            .nodes
            .iter()
            .filter_map(|node| match &node.content {
                NodeContent::Formula { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(formulas, ["x^{2}+1", "c^{2}+1"]);
    }

    #[test]
    fn formula_inside_cross_page_container_keeps_sidechannel_and_body_unchanged() {
        let mut doc = list_document(Some(2), true, false);
        doc.elements[4].page = Some(3);
        doc.elements
            .insert(4, InternalElement::text(ElementKind::PageBreak, "", 0));
        doc.formulas.push(formula(2));
        doc.push_relationship(Relationship {
            source: 3,
            target: RelationshipTarget::Index(5),
            kind: RelationshipKind::CrossReference,
        });
        let old_elements = serde_json::to_value(&doc.elements).unwrap();
        let old_relationships = doc.relationships.clone();
        let old_formulas = serde_json::to_value(&doc.formulas).unwrap();
        insert_missing_formula_elements(&mut doc, vec![(formula(2), true)]);
        assert_eq!(
            serde_json::to_value(&doc.elements).unwrap(),
            old_elements,
            "ambiguous cross-page ownership must not alter body, IDs, or geometry"
        );
        assert_eq!(doc.relationships, old_relationships);
        assert_eq!(serde_json::to_value(&doc.formulas).unwrap(), old_formulas);
        assert_eq!(doc.processing_warnings.len(), 1);
        assert_eq!(doc.processing_warnings[0].source, "formula_recognition");
        assert!(
            doc.processing_warnings[0]
                .message
                .contains("same-page container boundary")
        );
    }

    #[test]
    fn formula_before_first_list_item_stays_outside_the_list() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("First page prose", 1));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
        doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: false }, "", 0));
        for text in ["First retained item", "Second retained item"] {
            let mut item = paragraph(text, 2);
            item.kind = ElementKind::ListItem { ordered: false };
            item.depth = 1;
            doc.push_element(item);
        }
        doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));
        insert_missing_formula_elements(&mut doc, vec![(formula(2), true)]);
        let result = crate::extraction::derive::derive_extraction_result(doc, true, OutputFormat::Markdown);
        let rendered = result.formatted_content.as_deref().expect("rendered Markdown");
        let math = rendered.find("x^{2}+1").expect("rendered math");
        assert!(math < rendered.find("- First retained item").expect("first list item"));
        assert!(
            rendered.contains("- Second retained item"),
            "both items must remain in the list: {}",
            rendered
        );
        let structure = result.document.expect("derived structure");
        let math = structure
            .nodes
            .iter()
            .find(|node| matches!(node.content, NodeContent::Formula { .. }))
            .unwrap();
        assert_eq!(math.parent, None);
        let list = structure
            .nodes
            .iter()
            .find(|node| matches!(node.content, NodeContent::List { .. }))
            .unwrap();
        assert_eq!(list.children.len(), 2, "the existing list must retain both children");
        assert!(
            list.children
                .iter()
                .all(|index| matches!(structure.nodes[index.0 as usize].content, NodeContent::ListItem { .. }))
        );
    }
}

#[cfg(all(feature = "pdf", feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[tokio::test(flavor = "current_thread")]
async fn native_pdf_blocking_phase_does_not_starve_the_timeout_driver() {
    let work = spawn_pdf_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok::<_, crate::XbergError>(())
    });

    let result = tokio::time::timeout(std::time::Duration::from_millis(10), work).await;

    assert!(
        result.is_err(),
        "the timeout must fire while native PDF work is still running"
    );
}

#[cfg(feature = "pdf")]
#[test]
fn native_pdf_blocking_phase_stops_when_cancelled() {
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let config = ExtractionConfig {
        cancel_token: Some(token),
        ..Default::default()
    };

    let error = ensure_pdf_not_cancelled(&config).expect_err("cancelled native PDF work must stop");

    assert!(matches!(error, crate::XbergError::Cancelled));
}

#[cfg(feature = "pdf")]
#[test]
fn should_suppress_extracted_pdf_metadata_when_disabled() {
    let mut metadata = Metadata {
        output_format: Some("markdown".to_string()),
        ..Default::default()
    };
    let extracted = crate::pdf::metadata::PdfExtractionMetadata {
        title: Some("Document title".to_string()),
        subject: Some("Document subject".to_string()),
        authors: Some(vec!["Author".to_string()]),
        keywords: Some(vec!["keyword".to_string()]),
        created_at: Some("2026-01-01".to_string()),
        modified_at: Some("2026-01-02".to_string()),
        created_by: Some("Producer".to_string()),
        pdf_specific: crate::pdf::metadata::PdfMetadata {
            page_count: Some(2),
            ..Default::default()
        },
        page_structure: Some(crate::types::PageStructure {
            total_count: 2,
            unit_type: crate::types::PageUnitType::Page,
            boundaries: None,
            pages: None,
        }),
    };

    apply_extracted_pdf_metadata(&mut metadata, extracted, false);

    assert_eq!(metadata.output_format.as_deref(), Some("markdown"));
    assert!(metadata.title.is_none());
    assert!(metadata.subject.is_none());
    assert!(metadata.authors.is_none());
    assert!(metadata.keywords.is_none());
    assert!(metadata.created_at.is_none());
    assert!(metadata.modified_at.is_none());
    assert!(metadata.created_by.is_none());
    assert!(metadata.pages.is_none());
    assert!(metadata.format.is_none());
}

#[cfg(feature = "pdf")]
#[tokio::test]
async fn should_validate_direct_pdf_config_before_opening_document() {
    let config = ExtractionConfig {
        pdf_options: Some(crate::core::config::PdfConfig {
            top_margin_fraction: Some(f32::NAN),
            ..Default::default()
        }),
        ..Default::default()
    };

    let error = PdfExtractor::new()
        .extract_content(b"not a PDF", "application/pdf", &config)
        .await
        .expect_err("invalid direct PDF config must fail before parsing");

    assert!(matches!(error, crate::XbergError::Validation { .. }));
    assert!(error.to_string().contains("top_margin_fraction"));
}

fn coverage_native_text() -> String {
    (0..MIN_NATIVE_TOKENS_FOR_STRUCTURE_COVERAGE)
        .map(|index| format!("token{index}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `ExtractedDocument.pages` is derived from `element.page`, so a flat document with no
/// page tags yields `pages: None` -- which a caller cannot distinguish from "this
/// document has no pages". Reported against a 16-page scanned PDF where native mode
/// returned 16 pages and enabling OCR returned `None`, so `list(doc.pages)` raised
/// TypeError on the very mode that exists to give per-page OCR routing.
#[test]
fn should_tag_flat_document_paragraphs_with_their_page_numbers() {
    let text = "Page one body.\n\nPage two body.\n\nPage three body.";
    let first = text.find("Page two").expect("page two");
    let second = text.find("Page three").expect("page three");
    let boundaries = vec![
        crate::types::PageBoundary {
            byte_start: 0,
            byte_end: first,
            page_number: 1,
        },
        crate::types::PageBoundary {
            byte_start: first,
            byte_end: second,
            page_number: 2,
        },
        crate::types::PageBoundary {
            byte_start: second,
            byte_end: text.len(),
            page_number: 3,
        },
    ];

    let doc = flat_pdf_document(text, "application/pdf", Some(&boundaries));

    assert_eq!(doc.elements.len(), 3, "one paragraph per page");
    assert_eq!(
        doc.elements.iter().map(|e| e.page).collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(3)],
        "every paragraph must carry the page its content is on"
    );
}

/// Without boundaries the pre-existing untagged behaviour stands, so a caller that has
/// no page information is no worse off than before.
#[test]
fn should_leave_flat_document_untagged_when_no_boundaries_are_available() {
    let doc = flat_pdf_document("Only body.\n\nSecond para.", "application/pdf", None);

    assert_eq!(doc.elements.len(), 2);
    assert!(
        doc.elements.iter().all(|e| e.page.is_none()),
        "no boundaries means no page attribution, not a guessed one"
    );
}

#[test]
fn should_fall_back_to_native_text_when_structure_drops_substantial_content() {
    let native_text = coverage_native_text();
    let represented = native_text.split_whitespace().take(13).collect::<Vec<_>>().join(" ");
    let mut structured = InternalDocument::new("pdf");
    structured.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, represented, 0));

    let (selected, is_structured) = select_native_pdf_document(&native_text, "application/pdf", Some(structured), None);

    assert!(!is_structured);
    assert_eq!(selected.elements.len(), 1);
    assert_eq!(selected.elements[0].text, native_text);
}

#[test]
fn should_keep_structure_when_native_token_coverage_meets_floor() {
    let native_text = coverage_native_text();
    let represented = native_text.split_whitespace().take(14).collect::<Vec<_>>().join(" ");
    let mut structured = InternalDocument::new("pdf");
    structured.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, represented, 0));

    let (selected, is_structured) = select_native_pdf_document(&native_text, "application/pdf", Some(structured), None);

    assert!(is_structured);
    assert!(matches!(selected.elements[0].kind, ElementKind::Heading { level: 1 }));
}

#[test]
fn should_count_table_cells_as_structured_native_text() {
    let native_text = coverage_native_text();
    let mut structured = InternalDocument::new("pdf");
    structured.tables.push(crate::types::Table {
        cells: vec![native_text.split_whitespace().map(str::to_string).collect()],
        ..Default::default()
    });

    let (_, is_structured) = select_native_pdf_document(&native_text, "application/pdf", Some(structured), None);

    assert!(is_structured);
}

#[test]
fn should_not_apply_structure_coverage_gate_to_trivial_native_text() {
    let mut structured = InternalDocument::new("pdf");
    structured.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0));

    let (selected, is_structured) =
        select_native_pdf_document("Title and body", "application/pdf", Some(structured), None);

    assert!(is_structured);
    assert!(matches!(selected.elements[0].kind, ElementKind::Heading { level: 1 }));
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn gate_decision(run_layout: bool) -> crate::pdf::layout_gate::PageGateDecision {
    crate::pdf::layout_gate::PageGateDecision {
        run_layout,
        reason: if run_layout {
            crate::pdf::layout_gate::GateReason::MultiColumn
        } else {
            crate::pdf::layout_gate::GateReason::PlainText
        },
    }
}

/// Gate-skipped pages carry placeholder rasters; any skip must refuse
/// raster reuse so OCR never reads a blank square.
#[cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
#[test]
fn markdown_layout_rasters_are_reusable_only_when_ungated_and_unrotated() {
    assert!(markdown_layout_reusable_for_ocr(None, &[]));
    assert!(markdown_layout_reusable_for_ocr(
        Some(&[gate_decision(true), gate_decision(true)]),
        &[0, 0]
    ));
    assert!(!markdown_layout_reusable_for_ocr(
        Some(&[gate_decision(true), gate_decision(false)]),
        &[0, 0]
    ));
    for rotation in [90, 180, 270] {
        assert!(!markdown_layout_reusable_for_ocr(None, &[rotation]));
    }
}

#[cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
#[test]
fn gated_markdown_layout_rasters_stay_usable_for_native_unless_rotated() {
    let gated = [gate_decision(true), gate_decision(false)];
    assert!(!markdown_layout_reusable_for_ocr(Some(&gated), &[0, 0]));
    assert!(markdown_layout_usable_for_native(&[0, 360]));
    for rotation in [90, 180, 270] {
        assert!(!markdown_layout_usable_for_native(&[0, rotation]));
    }
}

#[cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
#[test]
fn cpu_layout_retry_override_controls_downstream_ocr_models() {
    use crate::core::config::{
        acceleration::{AccelerationConfig, ExecutionProviderType},
        layout::LayoutDetectionConfig,
    };

    let config = ExtractionConfig {
        layout: Some(LayoutDetectionConfig {
            acceleration: Some(AccelerationConfig {
                provider: ExecutionProviderType::CoreMl,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let effective = config_with_layout_acceleration_override(
        &config,
        Some(AccelerationConfig {
            provider: ExecutionProviderType::Cpu,
            ..Default::default()
        }),
    );

    assert_eq!(
        effective
            .resolved_layout_acceleration()
            .map(|acceleration| acceleration.provider.clone()),
        Some(ExecutionProviderType::Cpu)
    );
    assert_eq!(
        config
            .resolved_layout_acceleration()
            .map(|acceleration| acceleration.provider.clone()),
        Some(ExecutionProviderType::CoreMl)
    );
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
#[test]
fn layout_gate_metadata_reports_one_indexed_skipped_pages_and_all_reasons() {
    let decisions = [gate_decision(true), gate_decision(false), gate_decision(false)];
    let (gated, reasons) = layout_gate_metadata(Some(&decisions));
    assert_eq!(gated, Some(vec![2, 3]));
    assert_eq!(
        reasons,
        Some(vec![
            "multi_column".to_string(),
            "plain_text".to_string(),
            "plain_text".to_string()
        ])
    );

    assert_eq!(layout_gate_metadata(None), (None, None));
}

#[cfg(feature = "pdf")]
fn catalog(
    entries: impl IntoIterator<Item = (&'static str, xberg_native_pdf::object::Object)>,
) -> xberg_native_pdf::object::Object {
    xberg_native_pdf::object::Object::Dictionary(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

#[cfg(feature = "pdf")]
#[test]
fn lopdf_compatibility_pass_skips_pdf_without_relevant_signals() {
    let catalog = catalog([]);

    assert!(!raw_pdf_needs_lopdf_compatibility_pass(
        b"%PDF-1.7\nordinary content\n%%EOF",
    ));
    assert!(!catalog_needs_lopdf_compatibility_pass(Some(&catalog)));
}

#[cfg(feature = "pdf")]
#[test]
fn lopdf_compatibility_pass_keeps_raw_pdf_signals() {
    let catalog = catalog([]);

    assert!(raw_pdf_needs_lopdf_compatibility_pass(
        b"%PDF-1.7\ntrailer << /Prev 42 >>",
    ));
    assert!(raw_pdf_needs_lopdf_compatibility_pass(
        b"%PDF-1.7\n<< /Outlines 7 0 R >>",
    ));
    assert!(!catalog_needs_lopdf_compatibility_pass(Some(&catalog)));
}

#[cfg(feature = "pdf")]
#[test]
fn lopdf_compatibility_pass_keeps_parsed_catalog_outline() {
    let catalog = catalog([("Outlines", xberg_native_pdf::object::Object::Null)]);

    assert!(catalog_needs_lopdf_compatibility_pass(Some(&catalog)));
}

#[cfg(feature = "pdf")]
#[test]
fn lopdf_compatibility_pass_fails_open_without_catalog() {
    assert!(catalog_needs_lopdf_compatibility_pass(None));
}

#[cfg(feature = "pdf")]
fn pdf_test_document(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../test_documents/pdf/{name}"))
}

#[cfg(feature = "pdf")]
fn extraction_method(result: &crate::types::ExtractedDocument) -> Option<ExtractionMethod> {
    result.extraction_method
}

#[cfg(feature = "ocr")]
#[test]
fn should_report_native_when_mixed_ocr_accepts_no_replacements() {
    let replacements = ahash::AHashMap::new();

    assert_eq!(
        extraction_method_after_mixed_ocr(&replacements),
        ExtractionMethod::Native
    );
}

#[cfg(feature = "ocr")]
#[test]
fn should_report_mixed_when_ocr_replacement_survives() {
    let replacements = ahash::AHashMap::from([(2, "authoritative OCR text".to_string())]);

    assert_eq!(
        extraction_method_after_mixed_ocr(&replacements),
        ExtractionMethod::Mixed
    );
}

#[cfg(feature = "ocr")]
#[test]
fn should_build_full_ocr_boundaries_against_exact_output_text() {
    let page_texts = vec![
        "first page  ".to_string(),
        "repeated page".to_string(),
        "repeated page".to_string(),
    ];
    let text = "<page 1>first page  \n\n<page 2>repeated page\n\n<page 3>repeated page";

    let boundaries = boundaries_for_ocr_output(ExtractionMethod::Ocr, text, None, None, Some(&page_texts))
        .expect("full OCR page texts should produce exact boundaries");

    assert_eq!(boundaries.len(), 3);
    for (boundary, expected) in boundaries.iter().zip(&page_texts) {
        assert_eq!(
            &text[boundary.byte_start..boundary.byte_end],
            expected,
            "each boundary must slice the exact OCR page text"
        );
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn source_strip_frames_survive_plain_and_structured_document_selection() {
    let key = "ocr_source_strip_original_frames";
    let frame = serde_json::json!([{
        "page_number": 1, "rendered_width": 450, "rendered_height": 12000,
        "processed_width": 154, "processed_height": 4096,
    }]);
    let mut page = InternalDocument::new("pdf");
    page.push_element(InternalElement::text(ElementKind::Paragraph, "accepted OCR text", 0));
    page.metadata.additional.insert(key.into(), frame.clone());
    let pages = ahash::AHashMap::from([(1, page)]);
    let replacements = ahash::AHashMap::from([(1, "accepted OCR text".to_owned())]);
    let boundaries = [crate::types::PageBoundary {
        page_number: 1,
        byte_start: 0,
        byte_end: "accepted OCR text".len(),
    }];
    for format in [
        crate::core::config::OutputFormat::Plain,
        crate::core::config::OutputFormat::Markdown,
    ] {
        let (document, _, _) = select_pdf_document(
            ExtractionMethod::Mixed,
            "accepted OCR text",
            "application/pdf",
            None,
            None,
            Some(&replacements),
            Some(&pages),
            Some(&boundaries),
            &format,
        );
        assert_eq!(document.metadata.additional.get(key), Some(&frame));
        assert_eq!(
            document
                .metadata
                .additional
                .get(crate::ocr_metadata_keys::OCR_SOURCE_STRIP_ORIGINAL_FRAME_METADATA_KEY),
            Some(&frame[0]),
        );
        assert_eq!(crate::rendering::render_plain(&document).trim(), "accepted OCR text");
    }
    let (native, _, _) = select_pdf_document(
        ExtractionMethod::Native,
        "native text",
        "application/pdf",
        None,
        None,
        Some(&replacements),
        Some(&pages),
        None,
        &crate::core::config::OutputFormat::Plain,
    );
    assert!(!native.metadata.additional.contains_key(key));
}

#[cfg(feature = "ocr")]
#[test]
fn should_collect_requested_elements_when_plain_mixed_output_skips_structure_merge() {
    let mut page = InternalDocument::new("pdf");
    page.prebuilt_ocr_elements = Some(vec![crate::types::OcrElement {
        text: "accepted OCR element".to_string(),
        page_number: 1,
        ..Default::default()
    }]);
    let structured_pages = ahash::AHashMap::from([(1, page)]);
    let replacements = ahash::AHashMap::from([(1, "accepted OCR text".to_string())]);

    let (plain, _, _) = select_pdf_document(
        ExtractionMethod::Mixed,
        "accepted OCR text",
        "application/pdf",
        None,
        None,
        Some(&replacements),
        Some(&structured_pages),
        None,
        &crate::core::config::OutputFormat::Plain,
    );
    assert!(plain.prebuilt_ocr_elements.is_none());

    let elements = accepted_mixed_ocr_elements(&structured_pages);
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].text, "accepted OCR element");
    assert_eq!(elements[0].page_number, 1);
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn should_preserve_tables_without_injecting_text_when_plain_mixed_skips_structure_merge() {
    let table_markdown = "| hidden OCR table |";
    let mut page = InternalDocument::new("pdf");
    page.tables.push(crate::types::Table {
        markdown: table_markdown.to_string(),
        page_number: 2,
        ..Default::default()
    });
    let structured_pages = ahash::AHashMap::from([(2, page)]);
    let replacements = ahash::AHashMap::from([(2, "accepted OCR text".to_string())]);

    let (mut plain, _, _) = select_pdf_document(
        ExtractionMethod::Mixed,
        "accepted OCR text",
        "application/pdf",
        None,
        None,
        Some(&replacements),
        Some(&structured_pages),
        None,
        &crate::core::config::OutputFormat::Plain,
    );
    assert!(plain.tables.is_empty());

    let tables = accepted_mixed_ocr_tables(&structured_pages);
    attach_unrepresented_tables(&mut plain, tables);
    inject_unrepresented_table_elements(&mut plain, false);

    assert_eq!(plain.tables.len(), 1);
    assert_eq!(plain.tables[0].page_number, 2);
    assert_eq!(plain.tables[0].markdown, table_markdown);
    assert!(!crate::rendering::render_plain(&plain).contains(table_markdown));
}

#[cfg(all(feature = "pdf", feature = "ocr", feature = "chunking"))]
fn assert_occurs_once(haystack: &str, needle: &str, projection: &str) {
    assert_eq!(
        haystack.matches(needle).count(),
        1,
        "mixed OCR text must occur exactly once in {projection}: {haystack}"
    );
}

/// A single letter-sized page with an empty `Contents` stream -- no native text at all,
/// so the OCR fallback decision always flags it as a whole-document failure and the gate
/// deterministically returns `OcrGateOutcome::RunFallback`.
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn blank_letter_page_pdf() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! {},
        "Contents" => content_id,
    });
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("fixture PDF must serialize");
    bytes
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
fn mixed_native_and_scanned_pdf() -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let native_content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 18.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
            Operation::new(
                "Tj",
                vec![Object::string_literal(
                    "Issue 1281 native text remains on page one with enough meaningful words to pass the automatic per-page quality gate and preserve mixed routing.",
                )],
            ),
            Operation::new("ET", vec![]),
        ],
    };
    let native_content_id = document.add_object(Stream::new(
        dictionary! {},
        native_content.encode().expect("native PDF content must encode"),
    ));
    let native_page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => native_content_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });

    let image_id = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 1,
            "Height" => 1,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        },
        vec![0],
    ));
    let scanned_content = Content {
        operations: vec![
            Operation::new("q", vec![]),
            Operation::new(
                "cm",
                vec![612.into(), 0.into(), 0.into(), 792.into(), 0.into(), 0.into()],
            ),
            Operation::new("Do", vec![Object::Name(b"Scan".to_vec())]),
            Operation::new("Q", vec![]),
        ],
    };
    let scanned_content_id = document.add_object(Stream::new(
        dictionary! {},
        scanned_content.encode().expect("scanned PDF content must encode"),
    ));
    let scanned_page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => scanned_content_id,
        "Resources" => dictionary! { "XObject" => dictionary! { "Scan" => image_id } },
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![native_page_id.into(), scanned_page_id.into()],
            "Count" => 2,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("mixed PDF fixture must serialize");
    bytes
}

/// A PDF of `page_count` pages, each a full-page image `XObject` with no text layer, the
/// same scanned-page shape `mixed_native_and_scanned_pdf` uses for its one scanned page,
/// repeated. Letter `MediaBox` on every page (612x792pt) so the render batch peak is
/// uniform and predictable at the default 150 dpi: about 20.3MB per page.
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn all_scanned_pages_pdf(page_count: u32) -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();

    let image_id = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 1,
            "Height" => 1,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        },
        vec![0],
    ));

    let mut page_ids = Vec::new();
    for _ in 0..page_count {
        let scanned_content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![612.into(), 0.into(), 0.into(), 792.into(), 0.into(), 0.into()],
                ),
                Operation::new("Do", vec![Object::Name(b"Scan".to_vec())]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            scanned_content.encode().expect("scanned page content must encode"),
        ));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! { "XObject" => dictionary! { "Scan" => image_id } },
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        page_ids.push(page_id.into());
    }

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids,
            "Count" => i64::from(page_count),
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document
        .save_to(&mut bytes)
        .expect("all-scanned PDF fixture must serialize");
    bytes
}

/// A single-page PDF whose only font is `Type0`/`Identity-H` with no `/CIDSystemInfo`
/// ordering on its descendant `CIDFontType2`, and, when `with_tounicode` is `false`, no
/// `/ToUnicode` either -- so xberg_native_pdf's §9.10.2 mapping cascade has no route to
/// Unicode and falls back to a CID-as-Unicode echo (`MappingProvenance::Fallback`, issue
/// #1254's signal, same font shape as that issue's own reproducer). CIDs are chosen equal
/// to the Unicode codepoints of `text` (big-endian, two bytes each), so the echoed native
/// text reads as ordinary ASCII -- structurally indistinguishable from real prose by every
/// character-class heuristic, which is the shape of issue #1667's defect. No embedded font
/// program: the mapping decision does not depend on one.
///
/// When `with_tounicode` is `true`, an identity `/ToUnicode` CMap over printable ASCII
/// (§9.10.3) is attached, which resolves `MappingProvenance::ToUnicode` -- a mapping tier
/// the font's own file data backs, not a fallback echo -- so the page must NOT be routed
/// to OCR under `Auto` (issue #1696's false-positive control: the fix must not treat every
/// Type0 font as suspect, only ones with no usable mapping tier at all).
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn identity_h_mapping_pdf(text: &str, with_tounicode: bool) -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    let cid_bytes: Vec<u8> = text.chars().flat_map(|c| (c as u32 as u16).to_be_bytes()).collect();

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();

    let descriptor_id = document.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => "Synth+Fallback",
        "Flags" => 4,
        "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 900.into()],
        "ItalicAngle" => 0,
        "Ascent" => 800,
        "Descent" => (-200),
        "CapHeight" => 700,
        "StemV" => 80,
    });
    let descendant_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType2",
        "BaseFont" => "Synth+Fallback",
        "CIDSystemInfo" => dictionary! {
            "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"),
            "Supplement" => 0,
        },
        "FontDescriptor" => descriptor_id,
        "CIDToGIDMap" => "Identity",
        "DW" => 600,
    });

    // Identity mapping over printable ASCII (0x20-0x7E), which is the range `text`'s
    // codepoints are drawn from in every caller. Sequential `beginbfrange` form (ISO
    // 32000-1:2008 §9.10.3): a single dst start value, code and dst increment together. ~keep
    const TO_UNICODE_CMAP: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
            12 dict begin\n\
            begincmap\n\
            /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
            /CMapName /Adobe-Identity-UCS def\n\
            1 begincodespacerange\n\
            <0000> <FFFF>\n\
            endcodespacerange\n\
            1 beginbfrange\n\
            <0020> <007E> <0020>\n\
            endbfrange\n\
            endcmap\n\
            CMapName currentdict /CMap defineresource pop\n\
            end\n\
            end";
    let to_unicode_id =
        with_tounicode.then(|| document.add_object(Stream::new(dictionary! {}, TO_UNICODE_CMAP.to_vec())));

    let mut font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "Synth+Fallback",
        "Encoding" => "Identity-H",
        "DescendantFonts" => vec![descendant_id.into()],
        // Deliberately no /ToUnicode when `with_tounicode` is false: severs every route to
        // Unicode (issue #1254).
    };
    if let Some(id) = to_unicode_id {
        font_dict.set("ToUnicode", id);
    }
    let font_id = document.add_object(font_dict);

    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 18.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
            Operation::new("Tj", vec![Object::string_literal(cid_bytes)]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        content
            .encode()
            .expect("fabricated-provenance fixture content must encode"),
    ));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document
        .save_to(&mut bytes)
        .expect("fabricated-provenance fixture must serialize");
    bytes
}

/// A single-page PDF, modeled on [`identity_h_mapping_pdf`], whose `/ToUnicode` CMap
/// resolves every CID not to itself but to a ROT-`shift`ed letter (`a`-`z`/`A`-`Z` wrap
/// within their own case; every other character passes through unshifted). Unlike
/// [`identity_h_mapping_pdf`]'s `with_tounicode: false` case, this font DOES carry a
/// `/ToUnicode` CMap, so provenance resolves `MappingProvenance::ToUnicode` -- a real,
/// file-backed mapping tier -- not `Fallback`. Issue #1667's provenance signal must
/// therefore NOT fire on this fixture; only issue #1696's language-plausibility detector
/// can catch a `shift != 0` page, which is exactly the gap this builder exists to exercise.
///
/// CIDs are chosen equal to the Unicode codepoints of `text`, mirroring
/// [`identity_h_mapping_pdf`], so `shift: 0` is a structurally identical no-op control that
/// must extract `text` verbatim.
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn shifted_to_unicode_pdf(text: &str, shift: u8) -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    fn rot(c: char, shift: u8) -> char {
        let shift = shift % 26;
        if c.is_ascii_lowercase() {
            ((((c as u8 - b'a') + shift) % 26) + b'a') as char
        } else if c.is_ascii_uppercase() {
            ((((c as u8 - b'A') + shift) % 26) + b'A') as char
        } else {
            c
        }
    }

    let cid_bytes: Vec<u8> = text.chars().flat_map(|c| (c as u32 as u16).to_be_bytes()).collect();

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();

    let descriptor_id = document.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => "Synth+Shifted",
        "Flags" => 4,
        "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 900.into()],
        "ItalicAngle" => 0,
        "Ascent" => 800,
        "Descent" => (-200),
        "CapHeight" => 700,
        "StemV" => 80,
    });
    let descendant_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType2",
        "BaseFont" => "Synth+Shifted",
        "CIDSystemInfo" => dictionary! {
            "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"),
            "Supplement" => 0,
        },
        "FontDescriptor" => descriptor_id,
        "CIDToGIDMap" => "Identity",
        "DW" => 600,
    });

    // Per-character `bfchar` entries rather than a `bfrange`: a ROT shift is not a fixed
    // offset once case wraps (`z` -> `a`, `Z` -> `A`), so each distinct source character
    // needs its own explicit mapping entry.
    let mut distinct: Vec<char> = text.chars().collect();
    distinct.sort_unstable();
    distinct.dedup();
    let mut bfchar_body = String::new();
    for c in &distinct {
        let mapped = rot(*c, shift);
        bfchar_body.push_str(&format!("<{:04X}> <{:04X}>\n", *c as u32, mapped as u32));
    }
    let to_unicode_cmap = format!(
        "/CIDInit /ProcSet findresource begin\n\
             12 dict begin\n\
             begincmap\n\
             /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
             /CMapName /Adobe-Identity-UCS def\n\
             1 begincodespacerange\n\
             <0000> <FFFF>\n\
             endcodespacerange\n\
             {} beginbfchar\n\
             {}\
             endbfchar\n\
             endcmap\n\
             CMapName currentdict /CMap defineresource pop\n\
             end\n\
             end",
        distinct.len(),
        bfchar_body
    );
    let to_unicode_id = document.add_object(Stream::new(dictionary! {}, to_unicode_cmap.into_bytes()));

    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "Synth+Shifted",
        "Encoding" => "Identity-H",
        "DescendantFonts" => vec![descendant_id.into()],
        "ToUnicode" => to_unicode_id,
    };
    let font_id = document.add_object(font_dict);

    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 18.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
            Operation::new("Tj", vec![Object::string_literal(cid_bytes)]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        content.encode().expect("shifted-mapping fixture content must encode"),
    ));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document
        .save_to(&mut bytes)
        .expect("shifted-mapping fixture must serialize");
    bytes
}

/// Genuine English prose, single line, well over 600 characters -- three full
/// `language_detection::CHUNK_SIZE` chunks -- so `shifted_to_unicode_pdf`'s language-
/// plausibility detector has enough prose to render a verdict on (issue #1696). Public
/// domain (US Declaration of Independence, opening), pure ASCII letters/spaces so it
/// round-trips through a two-byte CID exactly.
#[cfg(all(feature = "pdf", feature = "ocr"))]
const WRONG_MAPPING_PROSE: &str = "When in the course of human events it becomes necessary \
        for one people to dissolve the political bands which have connected them with another \
        and to assume among the powers of the earth the separate and equal station to which the \
        laws of nature and of natures god entitle them a decent respect to the opinions of \
        mankind requires that they should declare the causes which impel them to the separation \
        we hold these truths to be self evident that all men are created equal that they are \
        endowed by their creator with certain unalienable rights that among these are life \
        liberty and the pursuit of happiness that to secure these rights governments are \
        instituted among men deriving their just powers from the consent of the governed";

/// ROT-`shift` over ASCII letters only, mirroring `shifted_to_unicode_pdf`'s own mapping so
/// the assertions below can predict the extracted text without re-deriving it.
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn rot_ascii_letters(text: &str, shift: u8) -> String {
    let shift = shift % 26;
    text.chars()
        .map(|c| {
            if c.is_ascii_lowercase() {
                ((((c as u8 - b'a') + shift) % 26) + b'a') as char
            } else if c.is_ascii_uppercase() {
                ((((c as u8 - b'A') + shift) % 26) + b'A') as char
            } else {
                c
            }
        })
        .collect()
}

/// xberg#1696: a `/ToUnicode` CMap that resolves every glyph to a real, file-backed
/// character, but consistently the WRONG one (a ROT-3 shift), reads as ordinary,
/// structurally-clean prose to every character-shape heuristic -- including issue #1667's
/// provenance check, since the mapping tier really is `ToUnicode`, not `Fallback`. Only the
/// language/dictionary-plausibility signal can see that the decoded text is not English (or
/// any real language) at all.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_auto_strategy_routes_wrong_mapping_page_to_ocr() {
    use crate::core::config::{OcrConfig, PageConfig};

    const OCR_TEXT: &str = "issue sixteen ninety six wrong mapping ocr replacement text";
    let _backend = register_mock_ocr_backend("pdf-1696-auto-wrong-mapping-routing", OCR_TEXT);
    let shifted = rot_ascii_letters(WRONG_MAPPING_PROSE, 3);

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "pdf-1696-auto-wrong-mapping-routing".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(
            &shifted_to_unicode_pdf(WRONG_MAPPING_PROSE, 3),
            "application/pdf",
            &config,
        )
        .await
        .expect("wrong-mapping PDF extraction should succeed");
    let derived =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert!(
        !derived.content.contains(&shifted),
        "the wrong-mapped native text must not survive to the final content: {:?}",
        derived.content
    );
    assert_eq!(
        derived.content.matches(OCR_TEXT).count(),
        1,
        "Auto-routed OCR content must occur exactly once: {:?}",
        derived.content
    );
    assert_eq!(
        derived.extraction_method,
        Some(ExtractionMethod::Ocr),
        "Auto strategy must record extraction_method: ocr for a wrong-mapped page"
    );

    let implausible_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
        crate::types::FormatMetadata::Pdf(pdf) => pdf.implausible_text_pages.clone(),
        _ => None,
    });
    assert_eq!(
        implausible_text_pages,
        Some(vec![1]),
        "the wrong-mapped page must be listed in implausible_text_pages"
    );
}

/// `shift: 0` control for the test above: a real `/ToUnicode` CMap whose mapping happens to
/// be the identity (equivalent to a correctly authored font) must never be routed to OCR --
/// the language-plausibility signal must not treat every `/ToUnicode`-bearing font as
/// suspect, only ones whose decoded text does not read as a real language.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_auto_strategy_keeps_native_text_for_identity_to_unicode() {
    use crate::core::config::{OcrConfig, PageConfig};

    const OCR_TEXT: &str = "mock ocr text that must never appear in a clean extraction";
    let _backend = register_mock_ocr_backend("pdf-1696-identity-false-positive", OCR_TEXT);

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "pdf-1696-identity-false-positive".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(
            &shifted_to_unicode_pdf(WRONG_MAPPING_PROSE, 0),
            "application/pdf",
            &config,
        )
        .await
        .expect("identity-mapped PDF extraction should succeed");
    let derived =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert!(
        derived.content.contains(WRONG_MAPPING_PROSE),
        "genuinely mapped native text must survive to the final content: {:?}",
        derived.content
    );
    assert!(
        !derived.content.contains(OCR_TEXT),
        "a page whose ToUnicode mapping happens to be identity must not be routed to OCR: {:?}",
        derived.content
    );
    assert_eq!(
        derived.extraction_method,
        Some(ExtractionMethod::Native),
        "Auto strategy must keep extraction_method: native for a genuinely legible page"
    );

    let implausible_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
        crate::types::FormatMetadata::Pdf(pdf) => pdf.implausible_text_pages.clone(),
        _ => None,
    });
    assert_eq!(
        implausible_text_pages,
        Some(Vec::new()),
        "a page that reads as real English must not be listed as implausible"
    );
}

/// Table rows, not sentences: the line carries five alphabetic words but is over a third
/// ASCII digits, so the prose gate's digit-ratio bound rejects all of it. The mapping is
/// genuine and the text is correct; the document simply holds no prose for a language check
/// to read. This is the shape of an invoice, a form or an agenda packet -- exactly the
/// documents people scan (issue #1709). ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn no_prose_table_text() -> String {
    (1..=25)
        .map(|row| format!("Item {row:04} Qty 12 Unit 45.00 Tax 3.75 Total 48.75"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Issue #1709: the language-plausibility check abstains on a page with too little prose to
/// judge, and an abstention is invisible. `implausible_text_pages: []` means both "every
/// page was checked and is fine" and "no page could be checked at all", so a caller holding
/// a document whose text layer decodes to the wrong letters cannot tell it from a clean one.
///
/// The document here is legitimate and correctly mapped, so it must stay native and must
/// flag no page. What must change is that the caller can see the check did not run.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_document_without_prose_reports_that_plausibility_was_not_evaluated() {
    use crate::core::config::{OcrConfig, PageConfig};

    const OCR_TEXT: &str = "mock ocr text that must never appear for a legitimate table";
    let _backend = register_mock_ocr_backend("pdf-1709-no-prose-abstention", OCR_TEXT);
    let table_text = no_prose_table_text();

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "pdf-1709-no-prose-abstention".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(&shifted_to_unicode_pdf(&table_text, 0), "application/pdf", &config)
        .await
        .expect("table-only PDF extraction should succeed");
    let derived =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert_eq!(
        derived.extraction_method,
        Some(ExtractionMethod::Native),
        "a legitimate table document must stay native: {:?}",
        derived.extraction_method
    );
    assert!(
        !derived.content.contains(OCR_TEXT),
        "a legitimate table document must not be routed to OCR: {:?}",
        derived.content
    );

    let implausible_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
        crate::types::FormatMetadata::Pdf(pdf) => pdf.implausible_text_pages.clone(),
        _ => None,
    });
    assert_eq!(
        implausible_text_pages,
        Some(Vec::new()),
        "no page of a legitimate table document may be flagged as implausible"
    );

    let warnings = &derived.processing_warnings;
    assert!(
        warnings.iter().any(|warning| warning.message.contains("1709")),
        "a document the plausibility check could not judge on any page must say so, so that \
             an empty implausible_text_pages is not read as a clean bill of health: {warnings:?}"
    );
}

/// Forced OCR discards the native text layer, so the caller never receives the text the
/// plausibility check abstained on. Reporting the abstention there would describe content
/// nobody got and would close by advising the very thing the caller already did.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_forced_ocr_does_not_report_a_plausibility_abstention() {
    use crate::core::config::{OcrConfig, PageConfig};

    const OCR_TEXT: &str = "forced ocr replacement text for the table only fixture of issue \
            one thousand seven hundred and nine with plenty of additional alphanumeric content \
            so the native alnum retention guard keeps this replacement in place";
    let _backend = register_mock_ocr_backend("pdf-1709-forced-ocr-abstention", OCR_TEXT);
    let table_text = no_prose_table_text();

    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "pdf-1709-forced-ocr-abstention".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(&shifted_to_unicode_pdf(&table_text, 0), "application/pdf", &config)
        .await
        .expect("forced-OCR extraction of a table-only PDF should succeed");
    let derived =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert_eq!(
        derived.extraction_method,
        Some(ExtractionMethod::Ocr),
        "force_ocr must produce OCR text: {:?}",
        derived.extraction_method
    );

    let warnings = &derived.processing_warnings;
    assert!(
        !warnings.iter().any(|warning| warning.message.contains("1709")),
        "a caller who forced OCR must not be told the native layer could not be checked: \
             {warnings:?}"
    );
}

/// xberg#1338's "explicit OCR config" rule stays intact for the plausibility signal too: a
/// wrong-mapped page must not be silently, automatically OCR'd when the caller never
/// configured `ocr`. The defect must instead be surfaced as a warning, and the page's
/// native (implausible) text is what the caller gets back.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_wrong_mapping_page_without_ocr_config_warns_and_keeps_native() {
    use crate::core::config::PageConfig;

    let shifted = rot_ascii_letters(WRONG_MAPPING_PROSE, 3);

    let config = ExtractionConfig {
        ocr: None,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(
            &shifted_to_unicode_pdf(WRONG_MAPPING_PROSE, 3),
            "application/pdf",
            &config,
        )
        .await
        .expect("wrong-mapping PDF extraction without an ocr config should still succeed");
    let derived =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert!(
        derived.content.contains(&shifted),
        "without an explicit ocr config the wrong-mapped native text must be kept: {:?}",
        derived.content
    );
    assert_eq!(
        derived.extraction_method,
        Some(ExtractionMethod::Native),
        "Auto strategy must not silently OCR a page with no explicit ocr config"
    );

    let warnings = &derived.processing_warnings;
    assert!(
        warnings.iter().any(|warning| warning.message.contains("1696")),
        "the defect must be surfaced as a warning naming issue #1696 when no ocr config is \
             present: {warnings:?}"
    );

    let implausible_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
        crate::types::FormatMetadata::Pdf(pdf) => pdf.implausible_text_pages.clone(),
        _ => None,
    });
    assert_eq!(
        implausible_text_pages,
        Some(vec![1]),
        "the metadata must still record the implausible page even though it was not OCR'd"
    );
}

/// The opt-in `OcrStrategy::ScannedPages` strategy reads `scanned_pages`, which
/// `record_implausible_text_pages` merges the language-plausibility signal into --
/// `scanned_pages_to_ocr` must therefore pick up a wrong-mapped page without needing its
/// own separate consultation of `implausible_text_pages`.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_scanned_pages_strategy_ocrs_implausible_pages() {
    use crate::core::config::{OcrConfig, OcrStrategy, PageConfig};

    // Long enough that its alnum count clears `MIN_OCR_NATIVE_ALNUM_RETENTION_RATIO`
    // against `WRONG_MAPPING_PROSE`'s ~600 native alnum characters: that unrelated guard
    // (`ocr::document`, #1678-era) rejects an OCR replacement that "recovers" less than
    // half of a page the character-shape gate still calls structurally healthy -- which a
    // wrong-but-ordinary-looking ROT-3 mapping does, despite being unreadable. A short mock
    // string would be rejected by that guard before this test ever exercises the routing
    // decision it is meant to check. ~keep
    const OCR_TEXT: &str = "scanned pages strategy ocr replacement text for issue one thousand \
            six hundred ninety six demonstrating that the language plausibility signal correctly \
            routed this wrong mapped page through the mixed native and ocr merge path under the \
            configured scanned pages strategy and confirming the ocr backend actually produced \
            this replacement content for the page under test with plenty of additional \
            alphanumeric characters to clear the native alnum retention guard";
    let _backend = register_mock_ocr_backend("pdf-1696-scanned-pages-strategy", OCR_TEXT);

    let config = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
        ocr: Some(OcrConfig {
            backend: "pdf-1696-scanned-pages-strategy".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(
            &shifted_to_unicode_pdf(WRONG_MAPPING_PROSE, 3),
            "application/pdf",
            &config,
        )
        .await
        .expect("wrong-mapping PDF extraction under ScannedPages should succeed");
    let derived =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert_eq!(
        derived.content.matches(OCR_TEXT).count(),
        1,
        "ScannedPages must route the implausible page to OCR via the merged scanned_pages \
             field: {:?}",
        derived.content
    );
    // The `ScannedPages` strategy always routes through the mixed OCR/native merge path
    // (`extract_mixed_ocr_native`), which reports `Mixed` whenever any page was replaced --
    // never `Ocr`, which is reserved for the whole-document `force_ocr`/`Auto` fallback
    // path. See `test_scanned_page_strategy_automatically_routes_only_the_scan_to_ocr` for
    // the same assertion on an unrelated fixture. ~keep
    assert_eq!(derived.extraction_method, Some(ExtractionMethod::Mixed));
}

/// A single page with a non-origin, negative-origin `MediaBox [10 -100 622 692]` (GH#1653)
/// and two font sizes -- a 24pt heading line and a 10pt body paragraph, both at known raw
/// user-space positions -- so hierarchy clustering assigns a heading level to the first and
/// leaves the second as body text. `rotate` optionally sets `/Rotate` on the page (GH#1654).
#[cfg(feature = "pdf")]
fn coordinate_frame_test_pdf(rotate: Option<i32>) -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 24.into()]),
            Operation::new("Td", vec![82.into(), 500.into()]),
            Operation::new("Tj", vec![Object::string_literal("Coordinate Frame Heading")]),
            Operation::new("ET", vec![]),
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 10.into()]),
            Operation::new("Td", vec![82.into(), 460.into()]),
            Operation::new(
                "Tj",
                vec![Object::string_literal(
                    "This is a body paragraph with enough ordinary words to cluster as body text below the heading.",
                )],
            ),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        content.encode().expect("coordinate frame fixture content must encode"),
    ));

    let mut page_dict = dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "MediaBox" => vec![10.into(), (-100).into(), 622.into(), 692.into()],
    };
    if let Some(rotate) = rotate {
        page_dict.set("Rotate", rotate);
    }
    let page_id = document.add_object(page_dict);

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document
        .save_to(&mut bytes)
        .expect("coordinate frame fixture PDF must serialize");
    bytes
}

#[cfg(feature = "pdf")]
fn coordinate_frame_extraction_config() -> ExtractionConfig {
    use crate::core::config::{HierarchyConfig, PdfConfig};

    ExtractionConfig {
        pdf_options: Some(PdfConfig {
            hierarchy: Some(HierarchyConfig {
                enabled: true,
                ..HierarchyConfig::default()
            }),
            ..PdfConfig::default()
        }),
        ..ExtractionConfig::default()
    }
}

#[tokio::test]
#[cfg(feature = "pdf")]
async fn pdf_page_coordinate_frame_reports_raw_media_box_origin_and_unswapped_extent() {
    let content = coordinate_frame_test_pdf(None);
    let config = coordinate_frame_extraction_config();
    let extractor = PdfExtractor::new();
    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("coordinate frame fixture must extract");

    let frames = result
        .metadata
        .additional
        .get("pdf_page_coordinate_frames")
        .expect("pdf_page_coordinate_frames must be present when a page has hierarchy blocks")
        .as_array()
        .expect("pdf_page_coordinate_frames must be a JSON array");
    assert_eq!(frames.len(), 1, "exactly one page has hierarchy blocks");

    let frame = &frames[0];
    assert_eq!(frame["page_number"], serde_json::json!(1));
    assert_eq!(frame["origin_x"], serde_json::json!(10.0));
    assert_eq!(frame["origin_y"], serde_json::json!(-100.0));
    assert_eq!(frame["width"], serde_json::json!(612.0));
    assert_eq!(frame["height"], serde_json::json!(792.0));
    assert_eq!(frame["unit"], serde_json::json!("point"));
    assert_eq!(frame["origin"], serde_json::json!("bottom_left"));
    assert_eq!(frame["clockwise_rotation"], serde_json::json!(0));
}

#[tokio::test]
#[cfg(feature = "pdf")]
async fn pdf_page_coordinate_frame_reports_rotation_without_swapping_extent() {
    let content = coordinate_frame_test_pdf(Some(90));
    let config = coordinate_frame_extraction_config();
    let extractor = PdfExtractor::new();
    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("rotated coordinate frame fixture must extract");

    let frames = result
        .metadata
        .additional
        .get("pdf_page_coordinate_frames")
        .expect("pdf_page_coordinate_frames must be present when a page has hierarchy blocks")
        .as_array()
        .expect("pdf_page_coordinate_frames must be a JSON array");
    assert_eq!(frames.len(), 1);

    let frame = &frames[0];
    assert_eq!(frame["clockwise_rotation"], serde_json::json!(90));
    // Deliberately un-swapped: this describes raw PDF user space, not the displayed frame.
    assert_eq!(frame["width"], serde_json::json!(612.0));
    assert_eq!(frame["height"], serde_json::json!(792.0));
}

#[tokio::test]
#[cfg(feature = "pdf")]
async fn pdf_page_coordinate_frame_omits_page_with_malformed_rotation() {
    let content = coordinate_frame_test_pdf(Some(135));
    let config = coordinate_frame_extraction_config();
    let extractor = PdfExtractor::new();
    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("malformed-rotation fixture must still extract successfully");

    assert!(
        !result.elements.is_empty(),
        "extraction must otherwise be unaffected by the malformed /Rotate"
    );

    match result.metadata.additional.get("pdf_page_coordinate_frames") {
        None => {}
        Some(value) => {
            let frames = value.as_array().expect("pdf_page_coordinate_frames must be an array");
            assert!(
                frames.is_empty(),
                "page with malformed /Rotate must be omitted entirely, got {frames:?}"
            );
        }
    }
}

#[test]
fn flat_plain_text_retains_table_asset_without_duplicate_rendering() {
    const TABLE_TEXT: &str = "Account balance 42";

    let mut doc = InternalDocument::new("pdf");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, TABLE_TEXT, 0));
    let table = crate::types::Table {
        cells: vec![vec!["Account balance".to_string(), "42".to_string()]],
        markdown: "| Account balance | 42 |".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    };

    attach_unrepresented_tables(&mut doc, vec![table]);
    inject_unrepresented_table_elements(&mut doc, false);

    assert_eq!(doc.tables.len(), 1, "table assets must remain available to callers");
    assert!(
        !doc.elements
            .iter()
            .any(|element| matches!(element.kind, ElementKind::Table { .. })),
        "flat plain text must not append a second renderable copy of native table text"
    );

    let result =
        crate::extraction::derive::derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.content.matches(TABLE_TEXT).count(), 1);
    assert_eq!(result.tables.len(), 1);
}

#[test]
fn table_element_injection_remains_available_for_structured_output() {
    let mut doc = InternalDocument::new("pdf");
    let table = crate::types::Table {
        cells: vec![vec!["Heading".to_string(), "Value".to_string()]],
        markdown: "| Heading | Value |".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    };

    attach_unrepresented_tables(&mut doc, vec![table]);
    inject_unrepresented_table_elements(&mut doc, true);

    assert_eq!(doc.tables.len(), 1);
    assert_eq!(
        doc.elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::Table { .. }))
            .count(),
        1,
        "structured and OCR paths must still be able to render attached table assets"
    );
}

/// pdfa_031: the structure-coverage safeguard falls back to `flat_pdf_document`,
/// which carries no `Table` element, so a table reconstructed from prose that is
/// still in the flat text used to be rendered a second time.
#[test]
fn structured_output_skips_tables_already_present_in_the_element_stream() {
    let mut doc = InternalDocument::new("pdf");
    doc.push_element(InternalElement::text(
        ElementKind::Paragraph,
        "Persons committed to the custody of a sheriff shall be confined\nin the facilities designated by law.",
        0,
    ));
    let table = crate::types::Table {
        cells: vec![
            vec![
                "Persons committed to the custody".to_string(),
                "of a sheriff".to_string(),
            ],
            vec![
                "shall be confined in the facilities".to_string(),
                "designated by law".to_string(),
            ],
        ],
        markdown: "| Persons committed to the custody | of a sheriff |".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    };

    attach_unrepresented_tables(&mut doc, vec![table]);
    inject_unrepresented_table_elements(&mut doc, true);

    assert_eq!(doc.tables.len(), 1, "table assets must remain available to callers");
    assert!(
        !doc.elements
            .iter()
            .any(|element| matches!(element.kind, ElementKind::Table { .. })),
        "a table whose cells are already rendered must not be injected a second time"
    );
}

#[test]
fn structured_output_still_injects_tables_missing_from_the_element_stream() {
    let mut doc = InternalDocument::new("pdf");
    doc.push_element(InternalElement::text(
        ElementKind::Paragraph,
        "Narrative prose that shares none of the tabulated content.",
        0,
    ));
    let table = crate::types::Table {
        cells: vec![
            vec!["Region".to_string(), "Revenue".to_string(), "Growth".to_string()],
            vec!["North".to_string(), "1200".to_string(), "4 percent".to_string()],
            vec!["South".to_string(), "980".to_string(), "7 percent".to_string()],
        ],
        markdown: "| Region | Revenue | Growth |".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    };

    attach_unrepresented_tables(&mut doc, vec![table]);
    inject_unrepresented_table_elements(&mut doc, true);

    assert_eq!(
        doc.elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::Table { .. }))
            .count(),
        1,
        "dropped table content must still be recovered by injection"
    );
}

/// The containment check abstains below `MIN_TABLE_TOKENS_FOR_CONTAINMENT`, so a
/// short table cannot be suppressed by an incidental token overlap.
#[test]
fn short_tables_are_injected_even_when_their_tokens_appear_in_the_text() {
    let mut doc = InternalDocument::new("pdf");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "total 42 balance", 0));
    let table = crate::types::Table {
        cells: vec![vec!["Total".to_string(), "42".to_string()]],
        markdown: "| Total | 42 |".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    };

    attach_unrepresented_tables(&mut doc, vec![table]);
    inject_unrepresented_table_elements(&mut doc, true);

    assert_eq!(
        doc.elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::Table { .. }))
            .count(),
        1
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn ocr_tables_replace_native_tables_and_are_sorted() {
    let table = |markdown: &str, page_number| crate::types::Table {
        cells: Vec::new(),
        markdown: markdown.to_string(),
        page_number,
        bounding_box: None,
        ..Default::default()
    };
    let mut tables = vec![table("native", 1)];

    replace_tables_with_ocr_output(&mut tables, vec![table("ocr-page-2", 2), table("ocr-page-1", 1)]);

    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].markdown, "ocr-page-1");
    assert_eq!(tables[1].markdown, "ocr-page-2");
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn empty_ocr_tables_preserve_native_tables() {
    let mut tables = vec![crate::types::Table {
        cells: Vec::new(),
        markdown: "native".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    }];

    replace_tables_with_ocr_output(&mut tables, Vec::new());

    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].markdown, "native");
}

/// GH#1670: a mixed-path document where only page 2 goes to OCR must keep the native
/// tables of every other page. The bug replaced the whole document's table list with only
/// page 2's OCR table, dropping pages 1 and 3 entirely.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn mixed_ocr_tables_replace_only_the_pages_ocr_produced_a_table_for() {
    let native_table = |page_number| crate::types::Table {
        cells: Vec::new(),
        markdown: format!("native-page-{page_number}"),
        page_number,
        bounding_box: None,
        ..Default::default()
    };
    let mut tables = vec![native_table(1), native_table(2), native_table(3)];

    let mut ocr_page = InternalDocument::new("pdf");
    ocr_page.tables.push(crate::types::Table {
        cells: Vec::new(),
        markdown: "ocr-page-2".to_string(),
        page_number: 2,
        bounding_box: None,
        ..Default::default()
    });
    let structured_pages = ahash::AHashMap::from([(2, ocr_page)]);

    merge_mixed_ocr_tables(&mut tables, &structured_pages);

    assert_eq!(
        tables.len(),
        3,
        "pages 1 and 3 were never sent to OCR; their native tables must survive"
    );
    assert_eq!(tables[0].markdown, "native-page-1");
    assert_eq!(
        tables[1].markdown, "ocr-page-2",
        "page 2 went to OCR; its native table is superseded"
    );
    assert_eq!(tables[2].markdown, "native-page-3");
}

/// A page that goes to OCR but whose OCR pass finds no table must keep its native table,
/// the same rule `replace_tables_with_ocr_output` applies when OCR finds nothing anywhere.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_page_ocr_ran_on_but_found_no_table_for_keeps_its_native_table() {
    let mut tables = vec![crate::types::Table {
        cells: Vec::new(),
        markdown: "native-page-1".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    }];

    // Page 1 went to OCR (it is a key in structured_pages) but produced no table there.
    let structured_pages = ahash::AHashMap::from([(1, InternalDocument::new("pdf"))]);

    merge_mixed_ocr_tables(&mut tables, &structured_pages);

    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].markdown, "native-page-1");
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn full_ocr_flat_fallback_never_selects_stale_native_document() {
    let mut native = InternalDocument::new("pdf");
    native.push_element(InternalElement::text(ElementKind::Paragraph, "stale native content", 0));

    let (doc, origin, structured) = select_pdf_document(
        ExtractionMethod::Ocr,
        "authoritative OCR content",
        "application/pdf",
        Some(native),
        None,
        None,
        None,
        None,
        &crate::core::config::OutputFormat::Markdown,
    );

    assert_eq!(origin, PdfDocumentOrigin::Ocr);
    assert!(!structured);
    assert!(
        doc.elements
            .iter()
            .any(|element| element.text == "authoritative OCR content")
    );
    assert!(
        !doc.elements
            .iter()
            .any(|element| element.text == "stale native content")
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn mixed_origin_replaces_only_targeted_structured_page() {
    let mut native = InternalDocument::new("pdf");
    let table = crate::types::Table {
        cells: vec![vec!["native table".to_string()]],
        markdown: "| native table |".to_string(),
        page_number: 2,
        bounding_box: None,
        ..Default::default()
    };
    native.tables.push(table.clone());
    native.push_element(InternalElement::text(ElementKind::Paragraph, "native page one", 0).with_page(1));
    native.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
    native.push_element(InternalElement::text(ElementKind::Paragraph, "stale page two", 0).with_page(2));
    native.push_element(InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0).with_page(2));
    let mut results = ahash::AHashMap::new();
    results.insert(2, "OCR page two".to_string());

    let (mut doc, origin, structured) = select_pdf_document(
        ExtractionMethod::Mixed,
        "flat text must not be selected",
        "application/pdf",
        Some(native),
        None,
        Some(&results),
        None,
        None,
        &crate::core::config::OutputFormat::Markdown,
    );

    assert_eq!(origin, PdfDocumentOrigin::Mixed);
    assert!(structured);
    assert!(doc.elements.iter().any(|element| element.text == "native page one"));
    assert!(doc.elements.iter().any(|element| element.text == "OCR page two"));
    assert!(!doc.elements.iter().any(|element| element.text == "stale page two"));
    attach_unrepresented_tables(&mut doc, vec![table]);
    inject_unrepresented_table_elements(&mut doc, false);
    assert_eq!(doc.tables.len(), 1, "structured table data must remain available");
    assert!(
        !doc.elements
            .iter()
            .any(|element| matches!(element.kind, ElementKind::Table { .. })),
        "target-page table markup must not be re-injected after OCR replacement"
    );
}

/// A fully scanned PDF has no native text layer at all, so
/// `extract_document_structure_from_segments` (native pipeline) runs over zero
/// segments, produces zero elements, and its caller collapses that to
/// `pre_rendered_doc: None` (see `extraction.rs`'s `Ok(_) => None` arm). Before this
/// fix, `ExtractionMethod::Mixed`'s `None` branch ignored `structured_ocr_pages`
/// entirely and returned `flat_pdf_document`, which only ever emits `Paragraph`
/// elements -- discarding every heading the document-global OCR heuristic recovered
/// (matches the `ordinance_2197_scanned.pdf` field observation: `blocks_to_paragraphs`
/// found ~71 headings, but the returned document, and its rendered markdown, had zero).
/// Fails on unfixed code: `doc.elements` has no `Heading` (only the flat `Paragraph`
/// from `text`), and the rendered markdown has no `#` line.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn mixed_without_native_document_still_renders_ocr_structure_as_markdown() {
    let mut results = ahash::AHashMap::new();
    results.insert(1u32, "ANNUAL REPORT OVERVIEW\n\nReport body text.".to_string());

    let mut structured_page = InternalDocument::new("pdf");
    structured_page.push_element(
        InternalElement::text(ElementKind::Heading { level: 1 }, "ANNUAL REPORT OVERVIEW", 0).with_page(1),
    );
    structured_page.push_element(InternalElement::text(ElementKind::Paragraph, "Report body text.", 0).with_page(1));
    let mut structured_pages = ahash::AHashMap::new();
    structured_pages.insert(1u32, structured_page);

    let text = "ANNUAL REPORT OVERVIEW\n\nReport body text.";
    let boundaries = vec![crate::types::PageBoundary {
        byte_start: 0,
        byte_end: text.len(),
        page_number: 1,
    }];

    let (doc, origin, structured) = select_pdf_document(
        ExtractionMethod::Mixed,
        text,
        "application/pdf",
        None, // no native document -- the fully-scanned-PDF case
        None,
        Some(&results),
        Some(&structured_pages),
        Some(&boundaries),
        &crate::core::config::OutputFormat::Markdown,
    );

    assert_eq!(origin, PdfDocumentOrigin::Mixed);
    assert!(
        structured,
        "a document assembled from OCR-recovered structure must be reported as structured"
    );
    assert!(
        doc.elements
            .iter()
            .any(|element| matches!(element.kind, ElementKind::Heading { .. })),
        "expected a Heading element to survive the merge when there is no native document; \
             got element kinds: {:?}",
        doc.elements.iter().map(|element| &element.kind).collect::<Vec<_>>()
    );

    let markdown = crate::rendering::render_markdown(&doc);
    assert!(
        markdown.contains("# ANNUAL REPORT OVERVIEW"),
        "OCR document-global heading structure must reach the rendered markdown even when \
             native xberg_native_pdf extraction produced no structured document: {markdown:?}"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn full_ocr_structured_without_ocr_tables_injects_native_fallback_once() {
    let ocr_doc = InternalDocument::new("pdf");
    let table = crate::types::Table {
        cells: vec![vec!["native fallback".to_string()]],
        markdown: "| native fallback |".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    };
    let (mut doc, origin, structured) = select_pdf_document(
        ExtractionMethod::Ocr,
        "OCR content",
        "application/pdf",
        None,
        Some(ocr_doc),
        None,
        None,
        None,
        &crate::core::config::OutputFormat::Markdown,
    );
    let allow_injection = !structured || (origin == PdfDocumentOrigin::Ocr && doc.tables.is_empty());

    attach_unrepresented_tables(&mut doc, vec![table]);
    inject_unrepresented_table_elements(&mut doc, allow_injection);

    assert_eq!(doc.tables.len(), 1);
    assert_eq!(
        doc.elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::Table { .. }))
            .count(),
        1
    );
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
#[test]
fn preparing_ocr_layout_inputs_transfers_raster_ownership() {
    let image = image::RgbImage::from_pixel(4, 3, image::Rgb([1, 2, 3]));
    let original_pixels = image.as_ptr();
    let detections = vec![crate::layout::DetectionResult {
        page_width: 4,
        page_height: 3,
        detections: Vec::new(),
    }];

    let (images, detections) = prepare_ocr_layout_inputs(vec![image], detections);

    let image::DynamicImage::ImageRgb8(transferred) = &images[0] else {
        panic!("RGB raster must retain its storage type");
    };
    assert_eq!(transferred.as_ptr(), original_pixels);
    assert_eq!((detections[0].page_width, detections[0].page_height), (4, 3));
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
#[test]
fn preparing_ocr_layout_inputs_discards_only_mismatched_page_detections() {
    let images = vec![image::RgbImage::new(4, 3), image::RgbImage::new(8, 6)];
    let detections = vec![
        crate::layout::DetectionResult {
            page_width: 4,
            page_height: 3,
            detections: Vec::new(),
        },
        crate::layout::DetectionResult {
            page_width: 16,
            page_height: 12,
            detections: Vec::new(),
        },
    ];

    let (_, detections) = prepare_ocr_layout_inputs(images, detections);

    assert_eq!((detections[0].page_width, detections[0].page_height), (4, 3));
    assert_eq!((detections[1].page_width, detections[1].page_height), (8, 6));
    assert!(detections[1].detections.is_empty());
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
#[test]
fn preparing_ocr_layout_inputs_repairs_detection_cardinality() {
    let images = vec![image::RgbImage::new(4, 3), image::RgbImage::new(8, 6)];

    let (_, detections) = prepare_ocr_layout_inputs(images, Vec::new());

    assert_eq!(detections.len(), 2);
    assert_eq!((detections[0].page_width, detections[0].page_height), (4, 3));
    assert_eq!((detections[1].page_width, detections[1].page_height), (8, 6));
}

#[cfg(feature = "ocr")]
fn mk_decision(fallback: bool, whole_doc_failure: bool, failing_pages: Vec<u32>) -> ocr::OcrFallbackDecision {
    ocr::OcrFallbackDecision {
        stats: ocr::NativeTextStats::default(),
        avg_non_whitespace: 0.0,
        avg_alnum: 0.0,
        fallback,
        failing_pages,
        whole_doc_failure,
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
struct MockPdfOcrBackend {
    name: &'static str,
    content: &'static str,
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
impl crate::plugins::Plugin for MockPdfOcrBackend {
    fn name(&self) -> &str {
        self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[async_trait::async_trait]
impl crate::plugins::OcrBackend for MockPdfOcrBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    async fn process_image(
        &self,
        _image_bytes: &[u8],
        _config: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        Ok(crate::types::ExtractedDocument {
            content: self.content.to_string(),
            mime_type: std::borrow::Cow::Borrowed("text/plain"),
            ..Default::default()
        })
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
struct RegisteredOcrBackendGuard {
    name: &'static str,
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
impl Drop for RegisteredOcrBackendGuard {
    fn drop(&mut self) {
        let _ = crate::plugins::unregister_ocr_backend(self.name);
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
fn register_mock_ocr_backend(name: &'static str, content: &'static str) -> RegisteredOcrBackendGuard {
    crate::plugins::register_ocr_backend(std::sync::Arc::new(MockPdfOcrBackend { name, content })).unwrap();
    RegisteredOcrBackendGuard { name }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
struct FailingPdfOcrBackend {
    name: &'static str,
    message: &'static str,
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
impl crate::plugins::Plugin for FailingPdfOcrBackend {
    fn name(&self) -> &str {
        self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[async_trait::async_trait]
impl crate::plugins::OcrBackend for FailingPdfOcrBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    async fn process_image(
        &self,
        _image_bytes: &[u8],
        _config: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        Err(crate::XbergError::Plugin {
            message: self.message.to_string(),
            plugin_name: self.name.to_string(),
        })
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
fn register_failing_ocr_backend(name: &'static str, message: &'static str) -> RegisteredOcrBackendGuard {
    crate::plugins::register_ocr_backend(std::sync::Arc::new(FailingPdfOcrBackend { name, message })).unwrap();
    RegisteredOcrBackendGuard { name }
}

/// A backend that reports the two metadata keys `PageContent.ocr_confidence` is built from
/// (`mean_text_conf`, `word_count`) on a declared, calibrated scale -- the shape a real
/// Tesseract page arrives in, without needing tessdata on the machine running the test.
#[cfg(all(feature = "pdf", feature = "ocr"))]
struct ConfidenceReportingOcrBackend {
    name: &'static str,
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
const CONFIDENCE_MOCK_TEXT: &str = "Ordinance number twenty seventeen authorizes the municipal drainage \
                                        improvement program described throughout this recorded document";
#[cfg(all(feature = "pdf", feature = "ocr"))]
const CONFIDENCE_MOCK_MEAN_TEXT_CONF: i64 = 87;
#[cfg(all(feature = "pdf", feature = "ocr"))]
const CONFIDENCE_MOCK_WORD_COUNT: u32 = 16;
#[cfg(all(feature = "pdf", feature = "ocr"))]
const CONFIDENCE_MOCK_SCALE_MAX: f64 = 100.0;

#[cfg(all(feature = "pdf", feature = "ocr"))]
impl crate::plugins::Plugin for ConfidenceReportingOcrBackend {
    fn name(&self) -> &str {
        self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[async_trait::async_trait]
impl crate::plugins::OcrBackend for ConfidenceReportingOcrBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::Legibility {
            scale_max: CONFIDENCE_MOCK_SCALE_MAX,
        }
    }

    async fn process_image(
        &self,
        _image_bytes: &[u8],
        _config: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        let mut document = crate::types::ExtractedDocument {
            content: CONFIDENCE_MOCK_TEXT.to_string(),
            mime_type: std::borrow::Cow::Borrowed("text/plain"),
            ..Default::default()
        };
        document.metadata.additional.insert(
            std::borrow::Cow::Borrowed("mean_text_conf"),
            serde_json::Value::from(CONFIDENCE_MOCK_MEAN_TEXT_CONF),
        );
        document.metadata.additional.insert(
            std::borrow::Cow::Borrowed("word_count"),
            serde_json::Value::from(CONFIDENCE_MOCK_WORD_COUNT),
        );
        Ok(document)
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
fn register_confidence_reporting_ocr_backend(name: &'static str) -> RegisteredOcrBackendGuard {
    crate::plugins::register_ocr_backend(std::sync::Arc::new(ConfidenceReportingOcrBackend { name })).unwrap();
    RegisteredOcrBackendGuard { name }
}

/// Git-tracked, one-page scanned fixture with no native text layer. Embedded rather than
/// read at runtime so a missing file is a compile error, not a silently skipped test. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
const SCANNED_HELLO_PDF: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/ocr/scanned_hello.pdf"
));

/// #1568 -- the force-OCR route (`config.force_ocr` -> `run_ocr_with_layout` ->
/// `extract_with_ocr`). Fails on unfixed code: `ocr_confidence` was `None` on every page
/// because the per-page confidence the OCR loop already computes was never returned.
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial]
async fn force_ocr_route_attaches_page_ocr_confidence() {
    use crate::core::config::{OcrConfig, PageConfig};

    const BACKEND_NAME: &str = "pdf-page-confidence-force-ocr-mock";
    let _backend = register_confidence_reporting_ocr_backend(BACKEND_NAME);

    let config = ExtractionConfig {
        use_cache: false,
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: BACKEND_NAME.to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(SCANNED_HELLO_PDF, "application/pdf", &config)
        .await
        .expect("force-OCR extraction of the scanned fixture must succeed");

    let pages = internal
        .prebuilt_pages
        .as_ref()
        .expect("extract_pages must produce page contents");
    assert_eq!(pages.len(), 1, "the fixture is a single-page scan: {pages:?}");
    let confidence = pages[0]
        .ocr_confidence
        .as_ref()
        .expect("an OCR'd page must carry its confidence summary");
    assert_eq!(confidence.backend, BACKEND_NAME);
    assert_eq!(confidence.word_count, CONFIDENCE_MOCK_WORD_COUNT);
    assert_eq!(
        confidence.score,
        Some(CONFIDENCE_MOCK_MEAN_TEXT_CONF as f64 / CONFIDENCE_MOCK_SCALE_MAX),
        "a calibrated backend's raw confidence must be normalized by its own scale"
    );
}

/// GH#1892: `numeric_repair` on the whole-document route (`force_ocr` -> `run_ocr_with_layout`
/// -> `extract_with_ocr`), which had no repair call at all while the `force_ocr_pages` route
/// repaired from the same flag.
///
/// ~keep This asserts the *wiring*, which the unit tests on
/// `apply_numeric_repair_to_whole_document_ocr` cannot: those call the helper directly and pass
/// identically with both call sites deleted. The `numeric_repair: false` control is what makes
/// the positive assertion mean something -- without it, a fixture whose number the repair never
/// touched would read the same as a working repair.
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial]
async fn force_ocr_route_applies_numeric_repair() {
    use crate::core::config::OcrConfig;

    const BACKEND_NAME: &str = "pdf-1892-force-ocr-numeric-repair";
    const OCR_TEXT: &str = "Total operating revenue 1172 dollars";
    let _backend = register_mock_ocr_backend(BACKEND_NAME, OCR_TEXT);

    let config_for = |numeric_repair: bool| ExtractionConfig {
        use_cache: false,
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: BACKEND_NAME.to_string(),
            numeric_repair,
            ..Default::default()
        }),
        ..Default::default()
    };

    let plain_content = |config: ExtractionConfig| async move {
        let internal = PdfExtractor::new()
            .extract_content(SCANNED_HELLO_PDF, "application/pdf", &config)
            .await
            .expect("force-OCR extraction of the scanned fixture must succeed");
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain)
            .content
    };

    let unrepaired = plain_content(config_for(false)).await;
    let repaired = plain_content(config_for(true)).await;

    assert!(
        unrepaired.contains("1172"),
        "control: with the flag off the mock's bare number must survive, or this test measures \
             nothing: {unrepaired:?}"
    );
    assert!(
        repaired.contains("1,172"),
        "force_ocr takes the whole-document route, so `numeric_repair` must reach it: {repaired:?}"
    );
    assert!(
        !repaired.contains("1172"),
        "and the bare form must be replaced, not merely accompanied: {repaired:?}"
    );
}

/// xberg#1665: the render batch peak scales with the configured thread budget alone, with
/// no notion of `security_limits.max_content_size`. A 4-page batch of Letter (612x792pt)
/// pages at the default 150 dpi estimates to about 4 x 20.3MB = 81MB; with
/// `max_content_size` set to 50MiB (well above any one page, but below the whole batch),
/// the pre-fix code renders and validates the FULL 4-page batch and rejects it outright.
/// `force_ocr_pages` is the explicit-request route (`extract_mixed_ocr_native`), which
/// keeps a real validation failure a hard error rather than a silent native-text fallback
/// (see the `~keep` comment on its call site), so on the base tree this call returns `Err`.
/// The batch is no longer measured against that limit at all: each page's own peak is
/// checked on its own, and 20.3MB clears a 50MiB limit, so all four pages reach OCR in one
/// batch and the thread budget stops being the reason OCR turns off.
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial]
async fn large_thread_budget_does_not_turn_off_ocr_at_a_fixed_content_limit() {
    use crate::core::config::{ConcurrencyConfig, OcrConfig, PageConfig};
    use crate::extractors::security::SecurityLimits;

    const OCR_TEXT: &str = "issue sixteen sixty five recovered scanned page text";
    let _backend = register_mock_ocr_backend("pdf-1665-batch-peak-thread-budget", OCR_TEXT);

    let config = ExtractionConfig {
        concurrency: Some(ConcurrencyConfig {
            max_threads: Some(4),
            max_concurrent_ocr: None,
        }),
        force_ocr_pages: Some(vec![1, 2, 3, 4]),
        security_limits: Some(SecurityLimits {
            max_content_size: 50 * 1024 * 1024,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: "pdf-1665-batch-peak-thread-budget".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result = PdfExtractor::new()
        .extract_content(&all_scanned_pages_pdf(4), "application/pdf", &config)
        .await;

    let internal = result.expect(
        "a thread budget wider than the content limit must still complete: the batch must \
             shrink, not reject every scanned page",
    );
    let pages = internal
        .prebuilt_pages
        .as_ref()
        .expect("extract_pages must produce page contents");
    assert_eq!(pages.len(), 4, "the fixture has four pages: {pages:?}");
    for page in pages {
        assert!(
            page.content.contains(OCR_TEXT),
            "page {} must carry OCR'd text, not an empty native fallback: {:?}",
            page.page_number,
            page.content
        );
    }
}

/// #1568 -- the mixed / scanned-pages route (`config.force_ocr_pages` ->
/// `extract_mixed_ocr_native`). Covered separately from the force-OCR route above because
/// the two dispatch through entirely different code paths, and a fix to one leaves the
/// other silently reporting `None`. Also pins the "absent means not OCR'd" contract: the
/// natively extracted page must keep `ocr_confidence: None`.
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial]
async fn scanned_pages_route_attaches_page_ocr_confidence_only_to_ocred_pages() {
    use crate::core::config::{OcrConfig, PageConfig};

    const BACKEND_NAME: &str = "pdf-page-confidence-mixed-mock";
    let _backend = register_confidence_reporting_ocr_backend(BACKEND_NAME);

    let config = ExtractionConfig {
        use_cache: false,
        force_ocr_pages: Some(vec![2]),
        ocr: Some(OcrConfig {
            backend: BACKEND_NAME.to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
        .await
        .expect("targeted OCR of page 2 must succeed");

    let pages = internal
        .prebuilt_pages
        .as_ref()
        .expect("extract_pages must produce page contents");
    assert_eq!(pages.len(), 2, "the fixture has two pages: {pages:?}");
    assert!(
        pages[0].ocr_confidence.is_none(),
        "page 1 was extracted natively and must not carry a fabricated confidence: {:?}",
        pages[0].ocr_confidence
    );
    let confidence = pages[1]
        .ocr_confidence
        .as_ref()
        .expect("the OCR'd page must carry its confidence summary");
    assert_eq!(confidence.backend, BACKEND_NAME);
    assert_eq!(confidence.word_count, CONFIDENCE_MOCK_WORD_COUNT);
    assert_eq!(
        confidence.score,
        Some(CONFIDENCE_MOCK_MEAN_TEXT_CONF as f64 / CONFIDENCE_MOCK_SCALE_MAX)
    );
}

#[test]
fn test_pdf_extractor_plugin_interface() {
    let extractor = PdfExtractor::new();
    assert_eq!(extractor.name(), "pdf-extractor");
    assert!(extractor.initialize().is_ok());
    assert!(extractor.shutdown().is_ok());
}

/// The condition under which a failed automatic OCR fallback must propagate instead of
/// returning an empty success. Observed on a real scanned document whose pages exceeded
/// `security_limits.max_content_size`: extraction returned `ok`, empty content,
/// `extraction_method: native`, `ocr_used: false`, `quality_score: 0.0`, and the caller had
/// no way to distinguish that from a genuinely empty PDF. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_failed_ocr_fallback_with_no_native_text_is_a_total_loss() {
    assert!(
        failed_ocr_fallback_is_total_loss(""),
        "no native text at all is a total loss"
    );
    assert!(
        failed_ocr_fallback_is_total_loss("\n\n  \t "),
        "whitespace-only native text is as total a loss as none: it carries no content"
    );
    assert!(
        failed_ocr_fallback_is_total_loss("\u{FFFD}\u{FFFD} \u{FFFD}"),
        "text of only lost-glyph markers carries no content either"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_failed_ocr_fallback_with_native_text_still_returns_that_text() {
    assert!(
        !failed_ocr_fallback_is_total_loss("Ordinance No. 2024-17"),
        "native text below the OCR trigger is still worth returning with a warning"
    );
    assert!(
        !failed_ocr_fallback_is_total_loss("  x  "),
        "a single meaningful character is not a total loss"
    );
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial]
async fn automatic_ocr_failure_with_no_native_text_returns_an_error() {
    const BACKEND_NAME: &str = "pdf-total-loss-ocr-failure";
    const FAILURE: &str = "deliberate total-loss OCR failure";
    let _backend = register_failing_ocr_backend(BACKEND_NAME, FAILURE);
    let content = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(crate::core::config::OcrConfig {
            backend: BACKEND_NAME.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let error = PdfExtractor::new()
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect_err("a failed automatic OCR fallback with no native text must not report success");

    match error {
        crate::XbergError::Plugin { plugin_name, message } => {
            assert_eq!(plugin_name, "ocr");
            assert!(
                message.contains(FAILURE),
                "the backend failure must propagate, got: {message}"
            );
        }
        other => panic!("the typed OCR failure must propagate, got: {other}"),
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial]
async fn automatic_ocr_failure_with_native_text_returns_native_content_and_warning() {
    use crate::core::config::{OcrConfig, PageConfig};

    const BACKEND_NAME: &str = "pdf-native-fallback-ocr-failure";
    const FAILURE: &str = "deliberate recoverable OCR failure";
    const NATIVE_TEXT: &str = "Issue 1281 native text remains on page one";
    let _backend = register_failing_ocr_backend(BACKEND_NAME, FAILURE);
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: BACKEND_NAME.to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
        .await
        .expect("native text must remain available when targeted OCR fails");

    assert!(
        internal.content().contains(NATIVE_TEXT),
        "the native page text must survive: {}",
        internal.content()
    );
    assert_eq!(internal.ocr_page_failures.len(), 1);
    assert_eq!(internal.ocr_page_failures[0].page, 2);
    assert!(internal.ocr_page_failures[0].error.contains(FAILURE));
    assert!(!internal.ocr_page_failures[0].recovered);
    let warnings = internal
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "ocr")
        .collect::<Vec<_>>();
    // Three distinct OCR-source warnings belong here, not duplicates: one reports that
    // targeted OCR itself failed for page 2, one records the XObject retry that preceded
    // that failure, and the last says the language-plausibility check (issue #1709) could
    // not judge that page's retained native text at all. Assert on each warning's content
    // rather than a bare count, so a real regression in any one fails loudly. ~keep
    let fallback_failure_warnings = warnings
        .iter()
        .filter(|warning| warning.message.contains(FAILURE))
        .collect::<Vec<_>>();
    assert_eq!(
        fallback_failure_warnings.len(),
        1,
        "expected exactly one OCR fallback-failure warning: {warnings:?}"
    );

    let plausibility_abstention_warnings = warnings
        .iter()
        .filter(|warning| warning.message.contains("could not judge"))
        .collect::<Vec<_>>();
    assert_eq!(
        plausibility_abstention_warnings.len(),
        1,
        "expected exactly one plausibility-check abstention warning: {warnings:?}"
    );

    let xobject_retry_warnings = warnings
        .iter()
        .filter(|warning| {
            warning.message.contains("Page 2 contains 1 image XObject(s)")
                && warning.message.contains("retried on the embedded image bytes")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        xobject_retry_warnings.len(),
        1,
        "expected exactly one page-2 XObject retry warning: {warnings:?}"
    );

    assert_eq!(
        warnings.len(),
        fallback_failure_warnings.len() + plausibility_abstention_warnings.len() + xobject_retry_warnings.len(),
        "unexpected extra OCR-source warning(s): {warnings:?}"
    );

    let result =
        crate::extraction::derive::derive_extraction_result(internal, true, crate::core::config::OutputFormat::Plain);
    assert_eq!(extraction_method(&result), Some(ExtractionMethod::Native));
    assert!(!result.metadata.ocr_used);
}

#[test]
fn test_pdf_extractor_supported_mime_types() {
    let extractor = PdfExtractor::new();
    let mime_types = extractor.supported_mime_types();
    assert_eq!(mime_types.len(), 1);
    assert!(mime_types.contains(&"application/pdf"));
}

#[cfg(feature = "ocr")]
#[test]
fn test_should_fallback_to_ocr_for_empty_text() {
    assert!(ocr::evaluate_native_text_for_ocr("", Some(1), &OcrQualityThresholds::default()).fallback);
}

#[cfg(feature = "ocr")]
#[test]
fn test_should_not_fallback_for_meaningful_text() {
    let sample = "This page has searchable vector text and should avoid OCR.";
    assert!(!ocr::evaluate_native_text_for_ocr(sample, Some(1), &OcrQualityThresholds::default()).fallback);
}

#[cfg(feature = "ocr")]
#[test]
fn test_should_fallback_for_punctuation_only_text() {
    let sample = " . , ; : -- -- ";
    assert!(ocr::evaluate_native_text_for_ocr(sample, Some(2), &OcrQualityThresholds::default()).fallback);
}

#[cfg(feature = "ocr")]
#[test]
fn test_per_page_ocr_no_boundaries_falls_back_to_whole_doc() {
    let text = "This document has enough meaningful words for evaluation purposes here.";
    let decision = ocr::evaluate_per_page_ocr(text, None, Some(1), &OcrQualityThresholds::default());
    assert!(!decision.fallback);
}

#[cfg(feature = "ocr")]
#[test]
fn test_per_page_ocr_empty_boundaries_falls_back_to_whole_doc() {
    let text = "This document has enough meaningful words for evaluation purposes here.";
    let decision = ocr::evaluate_per_page_ocr(text, Some(&[]), Some(1), &OcrQualityThresholds::default());
    assert!(!decision.fallback);
}

#[cfg(feature = "ocr")]
#[test]
fn test_per_page_ocr_all_pages_good() {
    use crate::types::PageBoundary;

    let page1 = "This first page has plenty of meaningful searchable text content here.";
    let page2 = "This second page also has plenty of meaningful searchable text content.";
    let text = format!("{}{}", page1, page2);
    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: page1.len(),
            page_number: 1,
        },
        PageBoundary {
            byte_start: page1.len(),
            byte_end: text.len(),
            page_number: 2,
        },
    ];

    let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
    assert!(!decision.fallback);
}

#[cfg(feature = "ocr")]
#[test]
fn test_per_page_ocr_one_bad_page_triggers_fallback() {
    use crate::types::PageBoundary;

    let good_page = "This page has plenty of meaningful searchable text content for extraction.";
    let bad_page = " . ; ";
    let text = format!("{}{}", good_page, bad_page);
    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: good_page.len(),
            page_number: 1,
        },
        PageBoundary {
            byte_start: good_page.len(),
            byte_end: text.len(),
            page_number: 2,
        },
    ];

    let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
    assert!(decision.fallback);
}

#[cfg(feature = "ocr")]
#[test]
fn test_per_page_ocr_empty_page_triggers_fallback() {
    use crate::types::PageBoundary;

    let good_page = "This page has plenty of meaningful searchable text content for extraction.";
    let empty_page = "";
    let text = format!("{}{}", good_page, empty_page);
    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: good_page.len(),
            page_number: 1,
        },
        PageBoundary {
            byte_start: good_page.len(),
            byte_end: text.len(),
            page_number: 2,
        },
    ];

    let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
    assert!(decision.fallback);
}

#[cfg(feature = "ocr")]
#[test]
fn test_per_page_ocr_preserves_document_stats_on_fallback() {
    use crate::types::PageBoundary;

    let good_page = "This page has plenty of meaningful searchable text content for extraction.";
    let bad_page = " . ; ";
    let text = format!("{}{}", good_page, bad_page);
    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: good_page.len(),
            page_number: 1,
        },
        PageBoundary {
            byte_start: good_page.len(),
            byte_end: text.len(),
            page_number: 2,
        },
    ];

    let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
    assert!(decision.fallback);
    assert!(decision.stats.non_whitespace > 0);
    assert!(decision.stats.meaningful_words > 0);
}

#[cfg(feature = "ocr")]
#[test]
fn test_per_page_ocr_invalid_boundaries_skipped() {
    use crate::types::PageBoundary;

    let text = "This page has plenty of meaningful searchable text content for extraction.";
    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: text.len(),
            page_number: 1,
        },
        PageBoundary {
            byte_start: 999,
            byte_end: 9999,
            page_number: 2,
        },
    ];

    let decision = ocr::evaluate_per_page_ocr(text, Some(&boundaries), Some(1), &OcrQualityThresholds::default());
    assert!(!decision.fallback);
}

#[cfg(feature = "ocr")]
#[test]
fn test_per_page_ocr_multi_page_correct_page_count() {
    let text = "ab cd ef";
    let decision_wrong = ocr::evaluate_native_text_for_ocr(text, None, &OcrQualityThresholds::default());
    let decision_correct = ocr::evaluate_native_text_for_ocr(text, Some(20), &OcrQualityThresholds::default());
    assert!(
        decision_correct.avg_non_whitespace < decision_wrong.avg_non_whitespace,
        "Correct page count should produce lower per-page averages"
    );
}

/// #1336 bug 2: `ScannedPages` must honor its own whole-document-failure
/// signal (OCR every page) rather than discarding it and relying on the
/// caller's fallthrough to the `Auto` gate.
#[cfg(feature = "ocr")]
#[test]
fn test_scanned_pages_to_ocr_returns_full_page_set_on_whole_doc_failure() {
    use crate::core::config::{OcrConfig, OcrStrategy};
    use crate::pdf::metadata::{PdfExtractionMetadata, PdfMetadata};

    let config = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };

    let pdf_metadata = PdfExtractionMetadata {
        title: None,
        subject: None,
        authors: None,
        keywords: None,
        created_at: None,
        modified_at: None,
        created_by: None,
        pdf_specific: PdfMetadata {
            page_count: Some(3),
            scanned_pages: Some(Vec::new()),
            ..Default::default()
        },
        page_structure: None,
    };

    // Empty native text everywhere triggers `whole_doc_failure` in the per-page gate.
    let pages = scanned_pages_to_ocr(&config, &pdf_metadata, "", None);

    assert_eq!(
        pages,
        Some(vec![1, 2, 3]),
        "whole-doc failure under ScannedPages must OCR every page, not discard the signal"
    );
}

/// Whole-doc failure with an unknown page count must not fabricate a page
/// range; falling through to `None` (the `Auto` gate) is the safe default.
#[cfg(feature = "ocr")]
#[test]
fn test_scanned_pages_to_ocr_falls_through_when_page_count_unknown() {
    use crate::core::config::{OcrConfig, OcrStrategy};
    use crate::pdf::metadata::{PdfExtractionMetadata, PdfMetadata};

    let config = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };

    let pdf_metadata = PdfExtractionMetadata {
        title: None,
        subject: None,
        authors: None,
        keywords: None,
        created_at: None,
        modified_at: None,
        created_by: None,
        pdf_specific: PdfMetadata {
            page_count: None,
            scanned_pages: Some(Vec::new()),
            ..Default::default()
        },
        page_structure: None,
    };

    let pages = scanned_pages_to_ocr(&config, &pdf_metadata, "", None);

    assert_eq!(pages, None);
}

/// Metadata for a `ScannedPages` routing test: `page_count` pages, of which
/// `scanned_pages` were flagged by scan detection.
#[cfg(feature = "ocr")]
fn scanned_pages_metadata(
    page_count: Option<u32>,
    scanned_pages: Vec<u32>,
) -> crate::pdf::metadata::PdfExtractionMetadata {
    use crate::pdf::metadata::{PdfExtractionMetadata, PdfMetadata};

    PdfExtractionMetadata {
        title: None,
        subject: None,
        authors: None,
        keywords: None,
        created_at: None,
        modified_at: None,
        created_by: None,
        pdf_specific: PdfMetadata {
            page_count,
            scanned_pages: Some(scanned_pages),
            ..Default::default()
        },
        page_structure: None,
    }
}

/// GH#1752. Leaving `ocr_scanned_page_quality_gate` unset must reproduce the derived
/// answer for every shape of `ocr` block.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn scanned_page_quality_gate_unset_reproduces_the_presence_of_an_ocr_block() {
    use crate::core::config::OcrConfig;

    for ocr in [None, Some(OcrConfig::default())] {
        let config = ExtractionConfig {
            ocr: ocr.clone(),
            ..Default::default()
        };
        assert_eq!(
            config.ocr_scanned_page_quality_gate, None,
            "the setting must default to None"
        );
        assert_eq!(
            scanned_page_quality_gate_enabled(&config),
            ocr.is_some(),
            "with the setting unset the gate must follow the presence of an `ocr` block"
        );
    }
}

/// The same input the whole-document-failure test uses, but with the gate explicitly off:
/// only the pages scan detection flagged may be selected. This is the setting that avoids
/// paying for recognition across a whole mixed document.
#[cfg(feature = "ocr")]
#[test]
fn scanned_page_quality_gate_off_selects_detected_scans_only_despite_an_ocr_block() {
    use crate::core::config::{OcrConfig, OcrStrategy};

    let config = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
        ocr: Some(OcrConfig::default()),
        ocr_scanned_page_quality_gate: Some(false),
        ..Default::default()
    };
    let pdf_metadata = scanned_pages_metadata(Some(3), vec![2]);

    // Empty native text would trigger `whole_doc_failure` and select all three pages if
    // the gate were running. ~keep
    assert_eq!(
        scanned_pages_to_ocr(&config, &pdf_metadata, "", None),
        Some(vec![2]),
        "with the gate off only detected scans may be selected"
    );
}

/// The mirror image: the gate on with no `ocr` block must behave exactly like the gate on
/// with one, reading `OcrConfig::default()`'s thresholds.
#[cfg(feature = "ocr")]
#[test]
fn scanned_page_quality_gate_on_without_an_ocr_block_matches_the_gate_with_one() {
    use crate::core::config::{OcrConfig, OcrStrategy};

    let pdf_metadata = scanned_pages_metadata(Some(3), Vec::new());

    let with_block = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };
    let without_block = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
        ocr: None,
        ocr_scanned_page_quality_gate: Some(true),
        ..Default::default()
    };

    assert_eq!(
        scanned_pages_to_ocr(&without_block, &pdf_metadata, "", None),
        Some(vec![1, 2, 3]),
        "the gate must run on default thresholds when switched on without an `ocr` block"
    );
    assert_eq!(
        scanned_pages_to_ocr(&without_block, &pdf_metadata, "", None),
        scanned_pages_to_ocr(&with_block, &pdf_metadata, "", None),
        "and must reach the same answer as the same gate with a default `ocr` block"
    );
}

/// The pre-GH#1752 pin: with the setting unset and no `ocr` block, `ScannedPages` selects
/// detected scans only, even on input that would fail the gate outright.
#[cfg(feature = "ocr")]
#[test]
fn scanned_page_quality_gate_unset_without_an_ocr_block_selects_detected_scans_only() {
    use crate::core::config::OcrStrategy;

    let config = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
        ocr: None,
        ..Default::default()
    };

    assert_eq!(
        scanned_pages_to_ocr(&config, &scanned_pages_metadata(Some(3), vec![2]), "", None),
        Some(vec![2]),
    );
    assert_eq!(
        scanned_pages_to_ocr(&config, &scanned_pages_metadata(Some(3), Vec::new()), "", None),
        None,
        "no detected scan and no gate means nothing to select"
    );
}

/// GH#1752 F1: mirrors `near_empty_fallback_unset_reproduces_the_1338_carve_out`'s oracle
/// pattern below, which already compares `near_empty_ocr_fallback_applies`'s `Some(true)`
/// arm against `automatic_ocr_backend_is_registered()` directly rather than forcing a
/// particular build's registry state. Switched on without an `ocr` block,
/// `scanned_page_quality_gate_enabled` must require a registered automatic backend the
/// same way; before a fix it returned `true` unconditionally, which let
/// `scanned_pages_to_ocr` enter the per-page gate (and its `whole_doc_failure` branch,
/// selecting every page) with no backend able to run OCR on any of them.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn scanned_page_quality_gate_on_without_an_ocr_block_requires_a_registered_backend() {
    let config = ExtractionConfig {
        ocr: None,
        ocr_scanned_page_quality_gate: Some(true),
        ..Default::default()
    };
    assert_eq!(
        scanned_page_quality_gate_enabled(&config),
        automatic_ocr_backend_is_registered(),
        "switched on without an `ocr` block, the gate must require a registered automatic \
             backend, exactly like its sibling `near_empty_ocr_fallback_applies`'s `Some(true)` arm"
    );
}

/// GH#1752 / #1338. `ocr_near_empty_fallback` unset must reproduce the derived condition:
/// always with an `ocr` block, and without one only for genuinely absent native text.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn near_empty_fallback_unset_reproduces_the_1338_carve_out() {
    use crate::core::config::OcrConfig;

    let with_block = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };
    let without_block = ExtractionConfig::default();

    assert_eq!(with_block.ocr_near_empty_fallback, None);
    assert_eq!(without_block.ocr_near_empty_fallback, None);

    assert!(
        near_empty_ocr_fallback_applies(&with_block, "a page with plenty of real native text"),
        "an explicit `ocr` block keeps its full per-page gate behaviour"
    );
    assert!(
        !near_empty_ocr_fallback_applies(&without_block, "a page with plenty of real native text"),
        "without a block, a legitimately sparse or short PDF must stay native (#1338)"
    );
    assert_eq!(
        near_empty_ocr_fallback_applies(&without_block, "   \n  "),
        automatic_ocr_backend_is_registered(),
        "without a block, genuinely absent native text reaches OCR iff a backend is registered"
    );
}

/// Text made only of U+FFFD holds no content: every glyph of the page lost its Unicode value.
/// Without an `ocr` block the fallback must treat it like empty text, or a page that is
/// fully undecodable stops reaching OCR once the extractor keeps the U+FFFD. A page with
/// real text beside the U+FFFD stays native.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
#[serial_test::serial]
fn near_empty_fallback_treats_undecodable_only_text_as_absent_text() {
    use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
    use std::sync::Arc;

    struct StandInBackend(String);
    impl Plugin for StandInBackend {
        fn name(&self) -> &str {
            &self.0
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }
    #[async_trait::async_trait]
    impl OcrBackend for StandInBackend {
        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
        fn supports_language(&self, _: &str) -> bool {
            true
        }
        async fn process_image(
            &self,
            _: &[u8],
            _: &crate::core::config::OcrConfig,
        ) -> crate::Result<crate::types::ExtractedDocument> {
            Ok(crate::types::ExtractedDocument::default())
        }
    }

    let name = crate::core::config::OcrConfig::default().backend;
    let registered_here = !automatic_ocr_backend_is_registered()
        && crate::plugins::register_ocr_backend(Arc::new(StandInBackend(name.clone()))).is_ok();
    let without_block = ExtractionConfig::default();

    let lost = near_empty_ocr_fallback_applies(&without_block, "\u{FFFD}\u{FFFD} \n\u{FFFD}");
    let mixed = near_empty_ocr_fallback_applies(&without_block, "A real word \u{FFFD}");
    let backend_present = automatic_ocr_backend_is_registered();
    if registered_here {
        crate::plugins::unregister_ocr_backend(&name).unwrap();
    }

    assert!(backend_present, "the test needs an automatic backend to be registered");
    assert!(lost, "text of only U+FFFD reaches OCR like empty text");
    assert!(!mixed, "text with real words beside U+FFFD stays native");
}

/// Switched on, the fallback stops requiring *completely* empty text -- that is the whole
/// point: a scanned page carrying only a page label or scanner stamp must reach OCR. It
/// still consults the backend registry exactly as the derived condition does.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn near_empty_fallback_on_without_an_ocr_block_drops_the_empty_text_requirement() {
    let opted_in = ExtractionConfig {
        ocr: None,
        ocr_near_empty_fallback: Some(true),
        ..Default::default()
    };
    let derived = ExtractionConfig::default();

    assert_eq!(
        near_empty_ocr_fallback_applies(&opted_in, "Page 3 of 412"),
        automatic_ocr_backend_is_registered(),
        "the stamp-only page must now be judged by the quality gate, not by the empty-text test"
    );
    assert_eq!(
        near_empty_ocr_fallback_applies(&opted_in, "Page 3 of 412"),
        near_empty_ocr_fallback_applies(&derived, ""),
        "and the backend requirement is unchanged: only the text test widened"
    );
}

/// Switched off, the fallback never runs, even with an explicit `ocr` block.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn near_empty_fallback_off_suppresses_the_branch_even_with_an_ocr_block() {
    use crate::core::config::OcrConfig;

    let config = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        ocr_near_empty_fallback: Some(false),
        ..Default::default()
    };

    assert!(!near_empty_ocr_fallback_applies(&config, ""));
    assert!(!near_empty_ocr_fallback_applies(&config, "real native text"));
}

/// GH#1890. `record_implausible_text_pages` warns that implausible pages were not routed to
/// OCR; the `else if` in `extract_core_native` decides whether they actually are. Those
/// are one fact, and the warning used to re-derive it, which made them disagree for two
/// configurations. This is an oracle test, not a table of expectations: it compares the
/// observable warning against `near_empty_ocr_fallback_applies` itself, so any future
/// divergence fails here regardless of which side moved. `#[serial]` because the predicate's
/// `Some(true)`-without-a-block arm consults the process-global OCR backend registry that the
/// mock-backend tests in this module mutate. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[test]
#[serial]
fn the_unrouted_implausible_page_warning_tracks_the_routing_predicate() {
    use crate::core::config::OcrConfig;

    let implausible = rot_ascii_letters(WRONG_MAPPING_PROSE, 3);
    let boundaries = vec![crate::types::PageBoundary {
        page_number: 1,
        byte_start: 0,
        byte_end: implausible.len(),
    }];

    for ocr_block in [None, Some(OcrConfig::default())] {
        for near_empty_fallback in [None, Some(true), Some(false)] {
            let config = ExtractionConfig {
                ocr: ocr_block.clone(),
                ocr_near_empty_fallback: near_empty_fallback,
                ..Default::default()
            };
            let mut pdf_metadata = scanned_pages_metadata(Some(1), Vec::new());
            let mut warnings = Vec::new();

            let branch_runs = near_empty_ocr_fallback_applies(&config, &implausible);
            record_implausible_text_pages(
                &config,
                &mut pdf_metadata,
                &implausible,
                Some(&boundaries),
                &mut warnings,
            );

            assert_eq!(
                pdf_metadata.pdf_specific.implausible_text_pages,
                Some(vec![1]),
                "the ROT-3 page must be flagged as implausible, or this case asserts nothing \
                     (ocr_block: {}, ocr_near_empty_fallback: {near_empty_fallback:?})",
                ocr_block.is_some()
            );

            let warned = warnings
                .iter()
                .any(|warning| warning.message.contains("do not read as any real detectable language"));
            assert_eq!(
                warned,
                !branch_runs,
                "the not-routed-to-OCR warning must be emitted exactly when the automatic OCR \
                     branch will not run, and suppressed exactly when it will (ocr_block: {}, \
                     ocr_near_empty_fallback: {near_empty_fallback:?}, branch_runs: {branch_runs})",
                ocr_block.is_some()
            );
        }
    }
}

#[tokio::test]
#[cfg(feature = "pdf")]
async fn test_pdf_batch_mode_validates_page_config_enabled() {
    use crate::core::config::PageConfig;

    let extractor = PdfExtractor::new();

    let config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            insert_page_markers: false,
            marker_format: "<!-- PAGE {page_num} -->".to_string(),
            keep_native_content: false,
        }),
        ..Default::default()
    };

    let pdf_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/google_doc_document.pdf");
    if let Ok(content) = std::fs::read(pdf_path) {
        let result = extractor.extract_content(&content, "application/pdf", &config).await;
        assert!(
            result.is_ok(),
            "Failed to extract PDF with page config: {:?}",
            result.err()
        );

        let extraction_result = result.unwrap();
        let extraction_result = crate::extraction::derive::derive_extraction_result(
            extraction_result,
            true,
            crate::core::config::OutputFormat::Plain,
        );
        assert!(
            !extraction_result.content.is_empty(),
            "Content should be extracted from PDF"
        );
    }
}

#[tokio::test]
#[cfg(feature = "pdf")]
async fn test_pdf_batch_mode_derives_pages_with_page_config_absent() {
    let extractor = PdfExtractor::new();
    let config = ExtractionConfig::default();

    let pdf_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/google_doc_document.pdf");
    if let Ok(content) = std::fs::read(pdf_path) {
        let result = extractor.extract_content(&content, "application/pdf", &config).await;
        assert!(
            result.is_ok(),
            "Failed to extract PDF without page config: {:?}",
            result.err()
        );

        let extraction_result = result.unwrap();
        let extraction_result = crate::extraction::derive::derive_extraction_result(
            extraction_result,
            true,
            crate::core::config::OutputFormat::Plain,
        );
        assert!(
            extraction_result.pages.is_some(),
            "Pages should always be derived: the default config leaves annotation extraction \
                 off, which forces per-page tracking for the annotation fallback regardless of an \
                 explicit pages config"
        );
    }
}

/// GH CI E2E `test_pdf_hierarchy_config`: `pdf_options.hierarchy.enabled` used to be a silent
/// no-op. A `PageHierarchy` can only hang off a `PageContent`, and `page_contents` is produced
/// only when `pages.extract_pages` is set -- which `PageConfig::default()` leaves `false` and
/// nothing in the hierarchy config points at. Headings were detected, then dropped.
#[tokio::test]
#[cfg(feature = "pdf")]
async fn test_pdf_hierarchy_enabled_populates_page_hierarchy_without_explicit_page_config() {
    use crate::core::config::{HierarchyConfig, PdfConfig};

    let pdf_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/embedded_images_tables.pdf");
    let Ok(content) = std::fs::read(pdf_path) else {
        return;
    };

    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            hierarchy: Some(HierarchyConfig {
                enabled: true,
                ..HierarchyConfig::default()
            }),
            ..PdfConfig::default()
        }),
        ..ExtractionConfig::default()
    };

    let extractor = PdfExtractor::new();
    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("hierarchy extraction should succeed");
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

    let pages = result.pages.as_ref().expect("hierarchy request must produce pages");
    let hierarchy = pages[0]
        .hierarchy
        .as_ref()
        .expect("pages[0].hierarchy must be populated when hierarchy.enabled is true");
    assert!(
        !hierarchy.blocks.is_empty(),
        "hierarchy must carry at least one block; got block_count={}",
        hierarchy.block_count
    );
}

#[tokio::test]
#[cfg(feature = "pdf")]
async fn test_pdf_page_marker_validation() {
    use crate::core::config::PageConfig;

    let extractor = PdfExtractor::new();

    let config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            insert_page_markers: true,
            marker_format: "\n\n<!-- PAGE {page_num} -->\n\n".to_string(),
            keep_native_content: false,
        }),
        ..Default::default()
    };

    let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/multi_page.pdf");
    if let Ok(content) = std::fs::read(pdf_path) {
        let result = extractor.extract_content(&content, "application/pdf", &config).await;
        assert!(
            result.is_ok(),
            "Failed to extract PDF with page markers: {:?}",
            result.err()
        );

        let extraction_result = result.unwrap();
        let extraction_result = crate::extraction::derive::derive_extraction_result(
            extraction_result,
            true,
            crate::core::config::OutputFormat::Plain,
        );
        let marker_placeholder = "<!-- PAGE ";
        if extraction_result.content.len() > 100 {
            assert!(
                extraction_result.content.contains(marker_placeholder),
                "Page markers should be inserted when configured and document has multiple pages"
            );
        }
    }
}

/// #1451: `max_pages` must reject a document once its page count is known, before
/// per-page work (OCR/layout/rendering) starts. Against the unfixed extractor there is
/// no `max_pages` field to set, so this test fails to compile; once the field exists but
/// enforcement is missing, `extract_content` would return `Ok` here instead of the
/// expected `SecurityError::TooManyPages`.
#[tokio::test]
#[cfg(feature = "pdf")]
async fn should_reject_document_exceeding_max_pages() {
    let pdf_path = pdf_test_document("multi_page.pdf");
    let Ok(content) = std::fs::read(&pdf_path) else {
        return;
    };
    let real_page_count = xberg_native_pdf::PdfDocument::from_bytes(content.clone())
        .expect("fixture must parse")
        .page_count()
        .expect("fixture must expose a page count");
    assert!(
        real_page_count > 1,
        "fixture must have more than one page to exercise the limit"
    );

    let extractor = PdfExtractor::new();
    let config = ExtractionConfig {
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_pages: Some(real_page_count - 1),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extractor.extract_content(&content, "application/pdf", &config).await;
    let error = result.expect_err("a document over max_pages must be rejected");
    let message = error.to_string();
    assert!(
        message.contains("too many pages") || message.contains("max_pages"),
        "error must name the limit that was hit: {message}"
    );
}

/// A base64-encoded, 3-page, AES-256 (R6) encrypted PDF whose user password is the
/// obviously-fake literal `xberg-test-fake-password-1451`. No corpus fixture requires a
/// real user password to open: every encrypted PDF under `test_documents/pdf/` (including
/// `password_protected.pdf`, despite the name) authenticates with the empty password and
/// only restricts copy/edit permissions, so none of them exercise the password-required
/// path this test targets. Built with `qpdf --encrypt ... 256` from a minimal 3-page
/// document and verified with `qpdf --check` (fails with no password or the wrong one,
/// succeeds and reports 3 pages with the correct one) before being embedded here.
#[cfg(feature = "pdf")]
const ENCRYPTED_THREE_PAGE_PDF_BASE64: &str = concat!(
    "JVBERi0xLjcKJb/3ov4KMSAwIG9iago8PCAvRXh0ZW5zaW9ucyA8PCAvQURCRSA8PCAvQmFzZVZlcnNpb24gLzEuNyAv",
    "RXh0ZW5zaW9uTGV2ZWwgOCA+PiA+PiAvUGFnZXMgMiAwIFIgL1R5cGUgL0NhdGFsb2cgPj4KZW5kb2JqCjIgMCBvYmoK",
    "PDwgL0NvdW50IDMgL0tpZHMgWyAzIDAgUiA0IDAgUiA1IDAgUiBdIC9UeXBlIC9QYWdlcyA+PgplbmRvYmoKMyAwIG9i",
    "ago8PCAvQ29udGVudHMgNiAwIFIgL01lZGlhQm94IFsgMCAwIDIwMCAyMDAgXSAvUGFyZW50IDIgMCBSIC9SZXNvdXJj",
    "ZXMgPDwgL0ZvbnQgPDwgL0YxIDcgMCBSID4+ID4+IC9UeXBlIC9QYWdlID4+CmVuZG9iago0IDAgb2JqCjw8IC9Db250",
    "ZW50cyA4IDAgUiAvTWVkaWFCb3ggWyAwIDAgMjAwIDIwMCBdIC9QYXJlbnQgMiAwIFIgL1Jlc291cmNlcyA8PCAvRm9u",
    "dCA8PCAvRjEgNyAwIFIgPj4gPj4gL1R5cGUgL1BhZ2UgPj4KZW5kb2JqCjUgMCBvYmoKPDwgL0NvbnRlbnRzIDkgMCBS",
    "IC9NZWRpYUJveCBbIDAgMCAyMDAgMjAwIF0gL1BhcmVudCAyIDAgUiAvUmVzb3VyY2VzIDw8IC9Gb250IDw8IC9GMSA3",
    "IDAgUiA+PiA+PiAvVHlwZSAvUGFnZSA+PgplbmRvYmoKNiAwIG9iago8PCAvRmlsdGVyIC9GbGF0ZURlY29kZSAvTGVu",
    "Z3RoIDgwID4+CnN0cmVhbQq/uprLnpDr1L1WYuji7aoo8FC/AmNCG9HIRQLXttVpBBc7g+Wbs7gcxVkaZNEjLHKAT0E4",
    "zK38JxdAJjfL7XkpkCbptuaCZCGvvfQ0dyrxlWVuZHN0cmVhbQplbmRvYmoKNyAwIG9iago8PCAvQmFzZUZvbnQgL0hl",
    "bHZldGljYSAvU3VidHlwZSAvVHlwZTEgL1R5cGUgL0ZvbnQgPj4KZW5kb2JqCjggMCBvYmoKPDwgL0ZpbHRlciAvRmxh",
    "dGVEZWNvZGUgL0xlbmd0aCA4MCA+PgpzdHJlYW0K4JE0iPOQNpJRjCvrfuVuZ8G+7e3bbkuci4qETQEnA0RfeOqEZe5q",
    "L81EqqU/h7+KsfI+uoIT6tBO4uAdnj/i0054F0Q6VoQWb0PkpOiMLy1lbmRzdHJlYW0KZW5kb2JqCjkgMCBvYmoKPDwg",
    "L0ZpbHRlciAvRmxhdGVEZWNvZGUgL0xlbmd0aCA4MCA+PgpzdHJlYW0KKqDRR85TmYv59t1N7OdBCttbZfgU8lsnHRFY",
    "MueyVRSwbBDvnujXiRLztgcPFiS4XexkTW/ikEXnPHq9uFq1VDpPVQNpaRQlZKf9xlAUveVlbmRzdHJlYW0KZW5kb2Jq",
    "CjEwIDAgb2JqCjw8IC9DRiA8PCAvU3RkQ0YgPDwgL0F1dGhFdmVudCAvRG9jT3BlbiAvQ0ZNIC9BRVNWMyAvTGVuZ3Ro",
    "IDMyID4+ID4+IC9GaWx0ZXIgL1N0YW5kYXJkIC9MZW5ndGggMjU2IC9PIDw0ZmQzOTFhNjY0MzdlZjFkYzEwNDMxOGVj",
    "OTZhNDY1OTM5OWNhMzY2YTY1MTBmZjhkNTI1N2FjMmJiOGRmMGNkMGRiZjkzYTc4MDgzYTkzOGRhYzBlMjcwMzliMTMw",
    "ZmM+IC9PRSA8NDkwZTZlNzI0OGM2NmZkMjZjOTdjMzE3MWVlNjVlNjc0Yjk1YWM2ZWI1MTI1ZjVlZmVkMDBmOTI3NmQw",
    "Y2YxMj4gL1AgLTQgL1Blcm1zIDw1NmI5YWRkMWJiMWI2YjUwZGYyMjg3NWM1Njc5YTJmMz4gL1IgNiAvU3RtRiAvU3Rk",
    "Q0YgL1N0ckYgL1N0ZENGIC9VIDw0NTNlNjI2Y2JmZTI1MGE5N2FmNDllZWEzMmYwOWFmOWMyZDk4Y2MzMWM2ZGQ1NDdj",
    "NjkwYzU1NTMwZDhjN2ViYjA2MzdlN2JjZmZmMDdkZThmYjc2NmFmNWQ4ZDM1NWY+IC9VRSA8MTA0YWEyZjAzNDBiMTEw",
    "ZGEzZTFmMGQ0ODQ3NmQyOWMyZTExNmYxOTRlZTBjMjFmYWFiMzkzNDFiZmU2NjMyZT4gL1YgNSA+PgplbmRvYmoKeHJl",
    "ZgowIDExCjAwMDAwMDAwMDAgNjU1MzUgZiAKMDAwMDAwMDAxNSAwMDAwMCBuIAowMDAwMDAwMTMwIDAwMDAwIG4gCjAw",
    "MDAwMDAyMDEgMDAwMDAgbiAKMDAwMDAwMDMyOSAwMDAwMCBuIAowMDAwMDAwNDU3IDAwMDAwIG4gCjAwMDAwMDA1ODUg",
    "MDAwMDAgbiAKMDAwMDAwMDczNSAwMDAwMCBuIAowMDAwMDAwODA1IDAwMDAwIG4gCjAwMDAwMDA5NTUgMDAwMDAgbiAK",
    "MDAwMDAwMTEwNSAwMDAwMCBuIAp0cmFpbGVyIDw8IC9Sb290IDEgMCBSIC9TaXplIDExIC9JRCBbPGU4ODhiODFiZjdj",
    "YzRhODE3NzQ3YmM4ZDZlZTgxYzgxPjxmYzI0MzUwMTY1NzA4NjdhNDYzNTZiYTFiYWE5ZGVhND5dIC9FbmNyeXB0IDEw",
    "IDAgUiA+PgpzdGFydHhyZWYKMTY1MwolJUVPRgo=",
);

#[cfg(feature = "pdf")]
const ENCRYPTED_THREE_PAGE_PDF_PASSWORD: &str = "xberg-test-fake-password-1451";

/// An obviously-fake literal that is not this fixture's password, and is not empty.
#[cfg(feature = "pdf")]
const WRONG_PDF_PASSWORD: &str = "xberg-test-fake-wrong-password-1879";

/// `PdfDocument::authenticate` reports a rejected password as `Ok(false)`, not as an `Err`,
/// so `open_pdf_document` -- which mapped the error and dropped the bool -- accepted every
/// wrong password. That is why no `pdf_page_count` fixture could prove a binding forwards
/// its password argument: the page tree reads unauthenticated, and authentication itself
/// never failed either. The extraction path (`open_bytes_with_passwords`) always read the
/// bool and is unaffected. GH#1879.
#[test]
#[cfg(feature = "pdf")]
fn should_reject_a_wrong_pdf_password_rather_than_ignore_it() {
    use base64::Engine as _;

    let content = base64::engine::general_purpose::STANDARD
        .decode(ENCRYPTED_THREE_PAGE_PDF_BASE64)
        .expect("fixture must be valid base64");

    let error = crate::pdf_page_count(&content, Some(WRONG_PDF_PASSWORD))
        .expect_err("a wrong non-empty user password must not authenticate");
    assert!(
        error.to_string().contains("authenticate"),
        "the error must name authentication as the cause: {error}"
    );

    assert_eq!(
        crate::pdf_page_count(&content, Some(ENCRYPTED_THREE_PAGE_PDF_PASSWORD))
            .expect("the correct user password must authenticate"),
        3,
        "the correct password must still open the document"
    );
    assert_eq!(
        crate::pdf_page_count(&content, None).expect("an unauthenticated page-tree read must still succeed"),
        3,
        "the page tree is structure, not encrypted data, so omitting the password is not an error"
    );
}

/// The corpus fixture whose user password is empty is still the right document for a
/// binding-level wrong-password test: the empty password authenticates, and any non-empty
/// password is rejected. So the e2e fixture needs no new corpus object. GH#1879.
#[test]
#[cfg(feature = "pdf")]
fn should_reject_a_non_empty_password_on_a_pdf_whose_user_password_is_empty() {
    let Some(content) = crate::utils::read_test_fixture("pdf/password_protected.pdf") else {
        return;
    };

    assert_eq!(
        crate::pdf_page_count(&content, None).expect("an encrypted page tree reads without a password"),
        2
    );
    assert_eq!(
        crate::pdf_page_count(&content, Some("")).expect("the empty user password authenticates"),
        2
    );
    let error = crate::pdf_page_count(&content, Some(WRONG_PDF_PASSWORD))
        .expect_err("a non-empty password is neither the user nor the owner password");
    assert!(
        error.to_string().contains("authenticate"),
        "the error must name authentication as the cause: {error}"
    );
}

/// An encrypted document is subject to `max_pages` like any other. This is a
/// characterization test, NOT a regression test — it passes against the pre-fix code
/// too, and it is kept because the behaviour is worth pinning, not because it catches
/// anything.
///
/// It was originally written believing `xberg_native_pdf` refuses to count an unauthenticated
/// document's pages, so an encrypted PDF would skip the cap. That is false, and measuring
/// it is what settled the design: a PDF's page tree is structure, not string or stream
/// data, so `xberg_native_pdf::PdfDocument::from_bytes(..).page_count()` returns 3 for this
/// AES-256/R6 fixture with no password at all. A sweep of all 488 PDFs in
/// `test_documents/` found exactly one document the unauthenticated count cannot handle
/// (`corrupt_truncated.pdf`), which `lopdf` also cannot read and which fails extraction
/// anyway. So `enforce_page_limit` needs no password handling.
///
/// The fixture: 3 pages, AES-256/R6, built with `qpdf --encrypt ... 256` and verified
/// with `qpdf --check` before embedding. Its password is the obviously-fake literal
/// `xberg-test-fake-password-1451`. No corpus fixture requires a real user password —
/// every encrypted PDF under `test_documents/pdf/` (including `password_protected.pdf`,
/// despite the name) opens with the empty password and only restricts copy/edit.
#[tokio::test]
#[cfg(feature = "pdf")]
async fn encrypted_document_over_max_pages_is_rejected_like_any_other() {
    use crate::core::config::pdf::PdfConfig;
    use base64::Engine as _;

    let content = base64::engine::general_purpose::STANDARD
        .decode(ENCRYPTED_THREE_PAGE_PDF_BASE64)
        .expect("fixture must be valid base64");

    let extractor = PdfExtractor::new();
    let config = ExtractionConfig {
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_pages: Some(2),
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            passwords: Some(vec![ENCRYPTED_THREE_PAGE_PDF_PASSWORD.to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extractor.extract_content(&content, "application/pdf", &config).await;
    let error = result.expect_err("an encrypted document over max_pages must be rejected like any other");
    let message = error.to_string();
    assert!(
        message.contains("too many pages") || message.contains("max_pages"),
        "error must name the limit that was hit: {message}"
    );
}

/// A document exactly at the configured `max_pages` ceiling must extract in full — a
/// limit that rejects the boundary case too is not the fix the issue asked for.
#[tokio::test]
#[cfg(feature = "pdf")]
async fn should_extract_fully_when_page_count_is_at_max_pages() {
    let pdf_path = pdf_test_document("multi_page.pdf");
    let Ok(content) = std::fs::read(&pdf_path) else {
        return;
    };
    let real_page_count = xberg_native_pdf::PdfDocument::from_bytes(content.clone())
        .expect("fixture must parse")
        .page_count()
        .expect("fixture must expose a page count");

    let extractor = PdfExtractor::new();
    let config = ExtractionConfig {
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_pages: Some(real_page_count),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extractor.extract_content(&content, "application/pdf", &config).await;
    let doc = result.expect("a document exactly at max_pages must extract fully, not be rejected");
    assert!(
        !doc.elements.is_empty(),
        "extraction at the boundary must still produce content, not an empty truncated result"
    );
}

/// The default `SecurityLimits` (no override) must extract a multi-page document exactly
/// as before #1451: `max_pages` defaulting to anything other than `None` (unlimited) would
/// silently start rejecting existing callers' documents.
#[tokio::test]
#[cfg(feature = "pdf")]
async fn should_extract_normally_with_default_security_limits() {
    let pdf_path = pdf_test_document("multi_page.pdf");
    let Ok(content) = std::fs::read(&pdf_path) else {
        return;
    };

    let extractor = PdfExtractor::new();
    let config = ExtractionConfig::default();

    let result = extractor.extract_content(&content, "application/pdf", &config).await;
    assert!(
        result.is_ok(),
        "default security limits must not reject a normal multi-page document: {:?}",
        result.err()
    );
}

/// Proves the `PdfBackend::Pdfium` dispatch branch in `extract_core` (#702):
/// selecting pdfium must fail with a specific, actionable error naming the
/// backend and telling the caller what to do, rather than the native parser's
/// generic "corrupt file" error. Content is deliberately not a valid PDF --
/// dispatch must happen before any parsing, so a real pdfium rejection never
/// reaches the parser at all.
///
/// Fails against unfixed code: `extract_core` unconditionally calls
/// `extract_core_native` and never reads `PdfConfig::backend`, so `content`
/// gets handed straight to `xberg_native_pdf`. The native parser rejects the garbage
/// bytes with an `XbergError::Parsing` whose message describes a corrupt/invalid
/// PDF and never contains the word "pdfium" -- the
/// `message.contains("pdfium")` assertion below fails.
///
/// Gated on `not(feature = "pdf-pdfium")`: this exercises the *stub* rejection
/// wording. With `pdf-pdfium` enabled the real engine runs instead (see
/// `pdfium_engine`), whose error text this assertion does not describe.
#[tokio::test]
#[cfg(all(feature = "pdf", not(feature = "pdf-pdfium")))]
async fn pdfium_backend_is_rejected_with_actionable_error_not_generic_parse_failure() {
    let extractor = PdfExtractor::new();
    let mut config = ExtractionConfig::default();
    config.pdf_options = Some(crate::core::config::PdfConfig {
        backend: crate::core::config::PdfBackend::Pdfium,
        ..Default::default()
    });

    let result = extractor
        .extract_content(b"not a real pdf", "application/pdf", &config)
        .await;

    let error = result.expect_err("selecting pdfium must fail, not silently extract via xberg_native_pdf");
    let message = error.to_string();
    assert!(
        message.contains("pdfium"),
        "error must name the requested backend 'pdfium', got: {message}"
    );
    assert!(
        message.contains("native"),
        "error must tell the caller what to do (use native instead), got: {message}"
    );
}

/// Proves there is no silent fallback (#702's core danger): given a document
/// `xberg_native_pdf` can extract perfectly well, selecting `PdfBackend::Pdfium` must
/// still fail rather than quietly returning `xberg_native_pdf`'s output mislabeled as
/// pdfium. Uses a real, valid multi-page PDF specifically to rule out "it failed
/// because the input was bad" as an explanation for the error.
///
/// Fails against unfixed code: `extract_core` ignores `PdfConfig::backend`
/// entirely and always calls `extract_core_native`, so `result.is_ok()` is `true`
/// pre-fix (the exact silent-fallback failure mode #702 exists to close) instead
/// of the `false` asserted here.
///
/// Gated on `not(feature = "pdf-pdfium")`: with the real engine enabled, a valid
/// document legitimately extracting via pdfium is the whole point (see
/// `pdfium_engine_serializes_concurrent_extractions_when_the_library_is_available` below), not
/// a silent fallback -- this test's premise (pdfium selection must always fail)
/// only holds for the stub.
#[tokio::test]
#[cfg(all(feature = "pdf", not(feature = "pdf-pdfium")))]
async fn pdfium_backend_never_silently_falls_back_to_xberg_native_pdf_output() {
    let pdf_path = pdf_test_document("multi_page.pdf");
    let Ok(content) = std::fs::read(&pdf_path) else {
        return;
    };

    let extractor = PdfExtractor::new();
    let mut config = ExtractionConfig::default();
    config.pdf_options = Some(crate::core::config::PdfConfig {
        backend: crate::core::config::PdfBackend::Pdfium,
        ..Default::default()
    });

    let result = extractor.extract_content(&content, "application/pdf", &config).await;
    assert!(
        result.is_err(),
        "pdfium selected on a document xberg_native_pdf could extract fine must still error, \
             not silently return xberg_native_pdf's output under the caller's pdfium selection"
    );
}

/// Proves the `pdf-pdfium` engine (#702) actually extracts text and metadata
/// through pdfium rather than reusing `xberg_native_pdf` output. `multi_page.pdf` is a
/// 5-page document whose first page's text is known ahead of time (verified
/// with `pdftotext` against the fixture). Four requests run concurrently to
/// regression-test serialization of Pdfium's process-global C API.
///
/// Requires a real `libpdfium` on the system library search path (or
/// `PDFIUM_DYNAMIC_LIB_PATH` pointing at one) -- this repository provisions
/// neither today, so this test skips (rather than fails) when
/// `pdfium_engine::bind_once` cannot load the library. That is an environment
/// gap, not a code defect; asserting the error is specifically
/// `XbergError::MissingDependency` (rather than string-matching the message)
/// still proves the failure came from library loading and not from the stub
/// rejection or some other error being swallowed.
///
/// Fails against unfixed code: the stub `extract_core_pdfium` unconditionally
/// returns `XbergError::Validation` (never attempting to load pdfium), so on a
/// machine without `libpdfium` installed the `matches!(err,
/// XbergError::MissingDependency(_))` assertion below fails -- the error variant
/// is `Validation`, not `MissingDependency` -- and on a machine *with*
/// `libpdfium` installed the stub still returns `Validation` unconditionally, so
/// `result.is_ok()` never becomes `true` and the `.expect`-equivalent match's
/// `Ok` arm is never reached; either way the real extraction assertions further
/// down never run against unfixed code.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "pdf-pdfium"))]
async fn pdfium_engine_serializes_concurrent_extractions_when_the_library_is_available() {
    // Skip-on-absence is right for a developer without the bucket-fetched corpus, but it must
    // never be how CI passes. `XBERG_REQUIRE_PDFIUM` (set by the `pdfium-engine` job) turns
    // every skip in this test into a failure, following the same truthy-env convention as
    // `XBERG_REQUIRE_MODELS` in tests/candle_backends.rs. Without it this test reports success
    // whenever libpdfium cannot be loaded -- which is exactly how the engine ended up with no
    // executed coverage at all. ~keep
    let require_pdfium = matches!(
        std::env::var("XBERG_REQUIRE_PDFIUM").as_deref(),
        Ok("1" | "true" | "yes")
    );

    let pdf_path = pdf_test_document("multi_page.pdf");
    let content = match std::fs::read(&pdf_path) {
        Ok(content) => content,
        Err(err) => {
            assert!(
                !require_pdfium,
                "XBERG_REQUIRE_PDFIUM is set but the fixture at {} could not be read ({err}); \
                     a fixture-fetch failure is a checkout bug, never a reason to report success",
                pdf_path.display()
            );
            return;
        }
    };

    let extractor = PdfExtractor::new();
    let mut config = ExtractionConfig::default();
    config.pdf_options = Some(crate::core::config::PdfConfig {
        backend: crate::core::config::PdfBackend::Pdfium,
        ..Default::default()
    });

    let (first, second, third, fourth) = tokio::join!(
        extractor.extract_content(&content, "application/pdf", &config),
        extractor.extract_content(&content, "application/pdf", &config),
        extractor.extract_content(&content, "application/pdf", &config),
        extractor.extract_content(&content, "application/pdf", &config),
    );
    let results = [first, second, third, fourth];

    if let Some(err) = results.iter().find_map(|result| result.as_ref().err()) {
        if matches!(err, crate::XbergError::MissingDependency(_)) {
            assert!(
                !require_pdfium,
                "XBERG_REQUIRE_PDFIUM is set, so a real libpdfium was supposed to be loadable, \
                     but binding failed: {err:?}. Returning here would report success without ever \
                     exercising the engine."
            );
            return;
        }

        panic!("concurrent pdfium extraction failed: {err:?}");
    }

    for document in results.into_iter().flatten() {
        let result = crate::extraction::derive::derive_extraction_result(
            document,
            true,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            result.content.contains("The Evolution of the Word Processor"),
            "expected the fixture's real first-page heading in pdfium output, got: {:?}",
            result.content
        );
        assert!(
            result.content.contains("Christopher Latham Sholes"),
            "expected real body text from the fixture in pdfium output, got: {:?}",
            result.content
        );

        let page_count = result.metadata.format.as_ref().and_then(|format| match format {
            crate::types::FormatMetadata::Pdf(pdf) => pdf.page_count,
            _ => None,
        });
        assert_eq!(page_count, Some(5), "multi_page.pdf has 5 pages");
    }
}

#[tokio::test]
#[cfg(feature = "pdf")]
async fn test_pdf_exposes_native_extraction_method() {
    let extractor = PdfExtractor::new();
    let config = ExtractionConfig::default();
    let pdf_path = pdf_test_document("google_doc_document.pdf");

    if let Ok(content) = std::fs::read(pdf_path) {
        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("native PDF extraction should succeed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(extraction_method(&result), Some(ExtractionMethod::Native));
    }
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_pdf_exposes_ocr_extraction_method() {
    use crate::core::config::OcrConfig;

    let _backend = register_mock_ocr_backend("pdf-extraction-method-ocr", "mock OCR text");
    let extractor = PdfExtractor::new();
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "pdf-extraction-method-ocr".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pdf_path = pdf_test_document("multi_page.pdf");

    if let Ok(content) = std::fs::read(pdf_path) {
        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("forced OCR extraction should succeed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(extraction_method(&result), Some(ExtractionMethod::Ocr));
    }
}

/// #1575: a backend warning and its `additional` metadata (psm/language) must survive the
/// whole `PdfExtractor::extract_content` call, not just `extract_with_ocr_for_page`'s own
/// return value. `doc.metadata` is fully replaced by a fresh `Metadata { .. }` literal
/// partway through this extractor (see the `#1575` comment above that assignment) --
/// without the take-then-restore fix there, this end-to-end test would see an empty
/// `metadata.additional` even though the lower-level pipeline test already passes.
#[cfg(all(feature = "pdf", feature = "ocr"))]
struct WarningMetadataPdfOcrBackend {
    name: &'static str,
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
impl crate::plugins::Plugin for WarningMetadataPdfOcrBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[async_trait::async_trait]
impl crate::plugins::OcrBackend for WarningMetadataPdfOcrBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }
    fn supports_language(&self, _lang: &str) -> bool {
        true
    }
    async fn process_image(
        &self,
        _image_bytes: &[u8],
        _config: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        let mut additional = ahash::AHashMap::new();
        additional.insert(std::borrow::Cow::Borrowed("psm"), serde_json::json!("11"));
        additional.insert(std::borrow::Cow::Borrowed("language"), serde_json::json!("eng"));
        Ok(crate::types::ExtractedDocument {
            content: "mock OCR text".to_string(),
            metadata: crate::types::Metadata {
                additional,
                ..Default::default()
            },
            processing_warnings: vec![crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("tesseract"),
                message: std::borrow::Cow::Borrowed(
                    "Tesseract removed 1 OCR line(s) because their dictionary-check ratio exceeded 0.60.",
                ),
            }],
            ..Default::default()
        })
    }
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_pdf_ocr_route_surfaces_backend_warning_and_metadata() {
    const BACKEND_NAME: &str = "pdf-warning-metadata-ocr";
    crate::plugins::register_ocr_backend(std::sync::Arc::new(WarningMetadataPdfOcrBackend { name: BACKEND_NAME }))
        .unwrap();
    let _guard = RegisteredOcrBackendGuard { name: BACKEND_NAME };

    let extractor = PdfExtractor::new();
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(crate::core::config::OcrConfig {
            backend: BACKEND_NAME.to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pdf_path = pdf_test_document("multi_page.pdf");
    let content = std::fs::read(pdf_path).expect("fixture must be present");

    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("forced OCR extraction should succeed");

    assert_eq!(
        result.processing_warnings.len(),
        1,
        "expected exactly the backend's own warning: {:?}",
        result.processing_warnings
    );
    assert_eq!(result.processing_warnings[0].source, "tesseract");
    assert_eq!(
        result.processing_warnings[0].message,
        "Tesseract removed 1 OCR line(s) because their dictionary-check ratio exceeded 0.60."
    );
    assert_eq!(
        result.metadata.additional.get("psm"),
        Some(&serde_json::json!("11")),
        "psm must survive the `doc.metadata = Metadata {{ .. }}` reassignment: {:?}",
        result.metadata.additional
    );
    assert_eq!(
        result.metadata.additional.get("language"),
        Some(&serde_json::json!("eng")),
        "language must survive the `doc.metadata = Metadata {{ .. }}` reassignment: {:?}",
        result.metadata.additional
    );
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
/// #1576: any `ImageExtractionConfig` -- default fields included -- must not suppress
/// document-level page OCR on a scanned PDF. Before the fix, `ImageExtractionConfig::
/// default()` (`run_ocr_on_images: true`) left `pages[0].content` empty because the
/// `RunFallback` gate read `run_ocr_on_images` as "skip page OCR", even though that field
/// only controls whether separately extracted images get their own OCR pass. ~keep
async fn images_config_does_not_suppress_scanned_page_ocr() {
    use crate::core::config::{ImageExtractionConfig, OcrConfig, PageConfig};

    const BACKEND_NAME: &str = "pdf-1576-images-config-does-not-suppress-ocr";
    const RECOVERED_TEXT: &str = "recovered scan text";
    let _backend = register_mock_ocr_backend(BACKEND_NAME, RECOVERED_TEXT);
    // A single blank page (no `Contents` text) has empty native text, which the OCR
    // fallback decision (`ocr::scoring::OcrFallbackDecision`) always flags as a whole-
    // document failure -- deterministically exercising `OcrGateOutcome::RunFallback`,
    // unlike `force_ocr` which bypasses this gate entirely (see the comment on
    // `failed_ocr_fallback_is_total_loss` above).
    let content = blank_letter_page_pdf();

    let base_config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: BACKEND_NAME.to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            insert_page_markers: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    for (label, images) in [
        ("no images config", None),
        (
            "ImageExtractionConfig::default()",
            Some(ImageExtractionConfig::default()),
        ),
    ] {
        let config = ExtractionConfig {
            images,
            ..base_config.clone()
        };
        let internal = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .unwrap_or_else(|e| panic!("{label}: scanned-page OCR extraction should succeed: {e}"));
        let result = crate::extraction::derive::derive_extraction_result(
            internal,
            true,
            crate::core::config::OutputFormat::Plain,
        );
        let pages = result
            .pages
            .as_ref()
            .unwrap_or_else(|| panic!("{label}: pages must be present"));
        assert!(!pages.is_empty(), "{label}: at least one page must be present");
        assert!(
            pages[0].content.contains(RECOVERED_TEXT),
            "{label}: pages[0].content must carry the OCR text, got {:?}",
            pages[0].content
        );
        assert_eq!(
            extraction_method(&result),
            Some(ExtractionMethod::Ocr),
            "{label}: a fully scanned page must report ExtractionMethod::Ocr"
        );
    }
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn should_report_native_when_forced_page_ocr_is_rejected() {
    use crate::core::config::{OcrConfig, PageConfig};

    let _backend = register_mock_ocr_backend("pdf-rejected-mixed-method", "a");
    let content =
        std::fs::read(pdf_test_document("multi_page.pdf")).expect("mixed extraction-method fixture must be available");
    let config = ExtractionConfig {
        force_ocr_pages: Some(vec![1]),
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: "pdf-rejected-mixed-method".to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("rejected page OCR should preserve the native result");
    let result =
        crate::extraction::derive::derive_extraction_result(internal, true, crate::core::config::OutputFormat::Plain);

    assert_eq!(extraction_method(&result), Some(ExtractionMethod::Native));
    assert!(!result.metadata.ocr_used);
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr", feature = "chunking"))]
#[serial]
async fn test_mixed_pdf_ocr_survives_full_postprocessing() {
    use crate::core::config::{ChunkingConfig, OcrConfig, PageConfig};

    // Must recover at least MIN_OCR_NATIVE_ALNUM_RETENTION_RATIO (0.5) of page one's 1897
    // alphanumeric characters, or `accepted_ocr_page_replacements` vetoes the replacement as
    // destructive (`ocr/scoring.rs`, guard added by 10680fca4e6) and the page silently stays
    // native. This text carries 1151, a ratio of 0.61. A short marker string cannot exercise
    // this test's subject at all -- it is rejected before any substitution happens. ~keep
    const OCR_TEXT: &str = "Issue 1281 authoritative OCR replacement on page one. The Evolution of the Word \
            Processor. The concept of the word processor predates modern computers and has evolved through several \
            technological milestones. Pre-Digital Era, nineteenth to early twentieth century. The origins of word \
            processing can be traced back to the invention of the typewriter in the mid nineteenth century. Patented \
            in 1868 by Christopher Latham Sholes, the typewriter revolutionized written communication. It enabled \
            people to produce legible, professional documents far more efficiently than handwriting ever allowed. \
            During this period the term word processing did not yet exist, but the typewriter laid the groundwork. \
            Later advances such as carbon paper for duplicates and the electric typewriter introduced by IBM in 1935 \
            improved speed. Together these refinements steadily increased the convenience and reliability of everyday \
            document creation. Mechanical composition gave way to electronic storage as magnetic media made revision \
            practical for the first time. Dedicated word processing machines briefly dominated offices before general \
            purpose computers absorbed the role. By the late twentieth century the printed page had become an artifact \
            of software rather than of mechanism. That transition is the through line connecting every milestone \
            described on the remainder of this page.";
    const RETAINED_PAGE_TWO_MARKER: &str = "Other notable software from this era included WordPerfect";
    let _backend = register_mock_ocr_backend("pdf-extraction-method-mixed", OCR_TEXT);
    let extractor = PdfExtractor::new();
    let pdf_path = pdf_test_document("multi_page.pdf");
    let content = std::fs::read(pdf_path).expect("mixed OCR fixture must be available");
    let native_config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let native = extractor
        .extract_content(&content, "application/pdf", &native_config)
        .await
        .expect("native PDF extraction should succeed");
    let native =
        crate::extraction::derive::derive_extraction_result(native, true, crate::core::config::OutputFormat::Plain);
    let native_pages = native.pages.expect("native extraction must expose pages");
    let stale_page_one = native_pages[0].content.clone();

    let config = ExtractionConfig {
        force_ocr_pages: Some(vec![1]),
        output_format: crate::core::config::OutputFormat::Markdown,
        include_document_structure: true,
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: "pdf-extraction-method-mixed".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        chunking: Some(ChunkingConfig {
            max_characters: OCR_TEXT.chars().count() + 1,
            overlap: 0,
            ..Default::default()
        }),
        ..Default::default()
    };
    let internal = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("mixed OCR/native extraction should succeed");
    let derived = crate::extraction::derive::derive_extraction_result(
        internal.clone(),
        true,
        crate::core::config::OutputFormat::Markdown,
    );
    // Substitution happens inside `extract_content`, not in `run_pipeline`, so the OCR text is
    // already in place here; the assertions after `run_pipeline` cover the later stages
    // (chunking, document structure) rather than the substitution itself. The old form of this
    // check sliced `derived.content` with `metadata.pages.boundaries`, which index the raw
    // concatenated text and not the rendered string -- see `core::pipeline::features`. ~keep
    let derived_pages = derived.pages.as_ref().expect("derived extraction must expose pages");
    assert!(
        derived_pages[0].content.contains(OCR_TEXT),
        "page one must carry the accepted OCR replacement"
    );
    assert!(
        derived_pages[1].content.contains(RETAINED_PAGE_TWO_MARKER),
        "page-two derived content must retain native text"
    );
    assert_occurs_once(&derived.content, OCR_TEXT, "derived plain content");
    assert_occurs_once(
        derived
            .formatted_content
            .as_deref()
            .expect("derived Markdown output must exist"),
        OCR_TEXT,
        "derived Markdown output",
    );
    let result = crate::core::pipeline::run_pipeline(internal, &config)
        .await
        .expect("mixed OCR post-processing should succeed");

    assert_eq!(extraction_method(&result), Some(ExtractionMethod::Mixed));
    assert_occurs_once(&result.content, OCR_TEXT, "post-processed Markdown content");
    assert!(!result.content.contains(&stale_page_one));
    assert!(result.content.contains(RETAINED_PAGE_TWO_MARKER));

    let pages = result.pages.as_ref().expect("mixed extraction must expose pages");
    assert_occurs_once(&pages[0].content, OCR_TEXT, "page one");
    assert!(!pages[0].content.contains(&stale_page_one));
    assert_occurs_once(&pages[1].content, RETAINED_PAGE_TWO_MARKER, "retained page two");
    assert!(!pages[1].content.contains(OCR_TEXT));

    let document = result.document.as_ref().expect("document structure must be derived");
    let ocr_nodes: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| node.content.text().is_some_and(|text| text.contains(OCR_TEXT)))
        .collect();
    assert_eq!(ocr_nodes.len(), 1, "document structure must not duplicate OCR text");
    assert_eq!(ocr_nodes[0].page, Some(1));
    assert!(
        !document
            .nodes
            .iter()
            .any(|node| node.content.text().is_some_and(|text| text.contains(&stale_page_one)))
    );

    let chunks = result.chunks.as_ref().expect("chunking must run");
    let ocr_chunks: Vec<_> = chunks.iter().filter(|chunk| chunk.content.contains(OCR_TEXT)).collect();
    assert_eq!(ocr_chunks.len(), 1, "chunks must not duplicate OCR text");
    assert_eq!(ocr_chunks[0].metadata.first_page, Some(1));
    assert_eq!(ocr_chunks[0].metadata.last_page, Some(1));
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr", feature = "chunking"))]
#[serial]
async fn test_scanned_page_strategy_automatically_routes_only_the_scan_to_ocr() {
    use crate::core::config::{ChunkingConfig, OcrConfig, OcrStrategy, PageConfig};

    const NATIVE_TEXT: &str = "Issue 1281 native text remains on page one";
    const OCR_TEXT: &str = "Issue 1281 automatic OCR replacement on page two.";
    let _backend = register_mock_ocr_backend("pdf-automatic-mixed-routing", OCR_TEXT);
    let config = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
        output_format: crate::core::config::OutputFormat::Markdown,
        include_document_structure: true,
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: "pdf-automatic-mixed-routing".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        chunking: Some(ChunkingConfig {
            max_characters: OCR_TEXT.chars().count() + 1,
            overlap: 0,
            ..Default::default()
        }),
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
        .await
        .expect("automatic mixed PDF extraction should succeed");
    let derived = crate::extraction::derive::derive_extraction_result(
        internal.clone(),
        true,
        crate::core::config::OutputFormat::Markdown,
    );
    assert_occurs_once(&derived.content, OCR_TEXT, "automatic derived content");
    assert_occurs_once(
        derived
            .formatted_content
            .as_deref()
            .expect("automatic derived Markdown output must exist"),
        OCR_TEXT,
        "automatic derived Markdown output",
    );
    let result = crate::core::pipeline::run_pipeline(internal, &config)
        .await
        .expect("automatic mixed PDF post-processing should succeed");

    assert_eq!(extraction_method(&result), Some(ExtractionMethod::Mixed));
    assert_occurs_once(&result.content, NATIVE_TEXT, "automatic mixed Markdown content");
    assert_occurs_once(&result.content, OCR_TEXT, "automatic mixed Markdown content");
    let pages = result
        .pages
        .as_ref()
        .expect("automatic mixed extraction must expose pages");
    assert_eq!(pages.len(), 2);
    assert!(pages[0].content.contains(NATIVE_TEXT));
    assert!(!pages[0].content.contains(OCR_TEXT));
    assert_occurs_once(&pages[1].content, OCR_TEXT, "automatically OCR'd page two");
    assert!(!pages[1].content.contains(NATIVE_TEXT));

    let document = result.document.as_ref().expect("document structure must be derived");
    let ocr_nodes: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| node.content.text().is_some_and(|text| text.contains(OCR_TEXT)))
        .collect();
    assert_eq!(ocr_nodes.len(), 1);
    assert_eq!(ocr_nodes[0].page, Some(2));

    let chunks = result.chunks.as_ref().expect("automatic mixed chunking must run");
    let ocr_chunks: Vec<_> = chunks.iter().filter(|chunk| chunk.content.contains(OCR_TEXT)).collect();
    assert_eq!(ocr_chunks.len(), 1, "automatic chunks must not duplicate OCR text");
    assert_eq!(ocr_chunks[0].metadata.first_page, Some(2));
    assert_eq!(ocr_chunks[0].metadata.last_page, Some(2));
}

/// GH#2064: when the automatic OCR of the detected scanned pages fails as a whole, each
/// selected page gets a record beside the route-level warning.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn route_level_failure_records_each_scanned_page() {
    use crate::core::config::{OcrConfig, OcrStrategy};

    const NATIVE_TEXT: &str = "Issue 1281 native text remains on page one";
    let config = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: "pdf-2064-backend-that-is-not-registered".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
        .await
        .expect("a failed automatic OCR run must keep the native text");
    let result =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert!(
        result.content.contains(NATIVE_TEXT),
        "the native page must survive: {:?}",
        result.content
    );
    let route_warning = result
        .processing_warnings
        .iter()
        .find(|warning| {
            warning
                .message
                .starts_with("Automatic OCR of detected scanned pages [2] failed (")
        })
        .unwrap_or_else(|| {
            panic!(
                "the route-level warning must be present: {:?}",
                result.processing_warnings
            )
        });
    let failures = result.ocr_page_failures.as_deref().unwrap_or_default();
    assert_eq!(failures.len(), 1, "only the scanned page is recorded: {failures:?}");
    let failure = &failures[0];
    assert_eq!(failure.page, 2);
    assert!(!failure.recovered, "the scanned page has no native text to keep");
    assert_eq!(
        route_warning.message,
        format!(
            "Automatic OCR of detected scanned pages [2] failed ({}); those pages retain their native text, \
                 which may be empty or incomplete.",
            failure.error
        ),
        "the record carries the same error as the warning"
    );
}

/// GH#2064: the same records when the targeted fallback of the default `Auto` strategy fails
/// as a whole.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn targeted_fallback_failure_records_each_selected_page() {
    use crate::core::config::OcrConfig;

    const NATIVE_TEXT: &str = "Issue 1281 native text remains on page one";
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: "pdf-2064-backend-that-is-not-registered".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
        .await
        .expect("a failed targeted OCR fallback must keep the native text");
    let result =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert!(
        result.content.contains(NATIVE_TEXT),
        "the native page must survive: {:?}",
        result.content
    );
    let route_warning = result
        .processing_warnings
        .iter()
        .find(|warning| warning.message.starts_with("Targeted OCR fallback failed ("))
        .unwrap_or_else(|| {
            panic!(
                "the route-level warning must be present: {:?}",
                result.processing_warnings
            )
        });
    let failures = result.ocr_page_failures.as_deref().unwrap_or_default();
    assert_eq!(failures.len(), 1, "only the selected page is recorded: {failures:?}");
    let failure = &failures[0];
    assert_eq!(failure.page, 2);
    assert!(!failure.recovered, "the selected page has no native text to keep");
    assert_eq!(
        route_warning.message,
        format!(
            "Targeted OCR fallback failed ({}) for pages [2]; those pages retain native text that was below the \
                 OCR-trigger quality threshold and may be empty or incomplete.",
            failure.error
        ),
        "the record carries the same error as the warning"
    );
}

/// xberg#1667: a native text layer produced by a fabricated character mapping
/// (`MappingProvenance::Fallback`) reads as ordinary, structurally-clean ASCII prose, so
/// `NativeTextStats`' character-class checks pass it at `quality_score: 1.0` and OCR never
/// fires -- under the DEFAULT `OcrStrategy::Auto`, not the opt-in `ScannedPages` strategy the
/// existing scanned-page-routing tests use. The fix reads the fabricated-provenance signal
/// (already computed for issue #1254) from the `Auto` gate too, so this page must route to
/// OCR even though every character-shape heuristic would call it clean.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_auto_strategy_routes_fabricated_provenance_page_to_ocr() {
    use crate::core::config::{OcrConfig, PageConfig};

    const OCR_TEXT: &str = "issue sixteen sixty seven automatic ocr replacement text";
    let _backend = register_mock_ocr_backend("pdf-1667-auto-provenance-routing", OCR_TEXT);

    // Echoes back as ordinary ASCII prose (CID == Unicode codepoint under the
    // CID-as-Unicode fallback), well past every `OcrQualityThresholds` default: > 64
    // non-whitespace chars, several words at or above the 4-char "meaningful" length, no
    // fragmentation, no repeats. A clean-looking page, on paper.
    const FABRICATED_NATIVE_TEXT: &str = "synthetic fabricated text used only to confirm that automatic routing reaches the configured ocr backend correctly";

    let config = ExtractionConfig {
        // `OcrStrategy::Auto` is `#[default]`; left unset deliberately, this is the path
        // every caller who does not opt into `ScannedPages` takes.
        ocr: Some(OcrConfig {
            backend: "pdf-1667-auto-provenance-routing".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(
            &identity_h_mapping_pdf(FABRICATED_NATIVE_TEXT, false),
            "application/pdf",
            &config,
        )
        .await
        .expect("fabricated-provenance PDF extraction should succeed");
    let derived =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert!(
        !derived.content.contains(FABRICATED_NATIVE_TEXT),
        "the fabricated native text must not survive to the final content: {:?}",
        derived.content
    );
    assert_eq!(
        derived.content.matches(OCR_TEXT).count(),
        1,
        "Auto-routed OCR content must occur exactly once: {:?}",
        derived.content
    );
    assert_eq!(
        derived.extraction_method,
        Some(ExtractionMethod::Ocr),
        "Auto strategy must record extraction_method: ocr for a fabricated-provenance page"
    );
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn fabricated_mapping_page_records_effective_block_psm_in_page_metadata() {
    use crate::core::config::{OcrConfig, PageConfig};

    const FABRICATED_NATIVE_TEXT: &str =
        "synthetic fabricated text used to verify the per page segmentation mode recorded after automatic ocr routing";
    let config = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(
            &identity_h_mapping_pdf(FABRICATED_NATIVE_TEXT, false),
            "application/pdf",
            &config,
        )
        .await
        .expect("fabricated-provenance PDF extraction should succeed");
    assert_eq!(
        internal
            .metadata
            .additional
            .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY),
        Some(&serde_json::json!([{ "page_number": 1, "psm": 6 }]))
    );
}

/// `identity_h_mapping_pdf`'s unmapped text drawn invisibly over an inset scan: a text
/// raster covering half the page, like a scanned sheet painted with margins, under an
/// unreadable text layer.
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn inset_scan_with_unmapped_text_layer_pdf(text: &str) -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document =
        Document::load_mem(&identity_h_mapping_pdf(text, false)).expect("fabricated-provenance fixture must load");
    let page_id = *document.get_pages().get(&1).expect("fixture has one page");
    let scan = image::load_from_memory(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ocr/lone_quantity_invoice.png"
    )))
    .expect("scan fixture must decode")
    .resize(1275, 1650, image::imageops::FilterType::Triangle)
    .to_luma8();
    let (width, height) = scan.dimensions();
    let image_id = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => width,
            "Height" => height,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        },
        scan.into_raw(),
    ));

    // 432 x 559 pt on a Letter page is half its area: an inset scan, not a figure.
    let mut content = b"q 432 0 0 559 90 116 cm /Scan Do Q\n3 Tr\n".to_vec();
    content.extend(document.get_page_content(page_id));
    document
        .change_page_content(page_id, content)
        .expect("replace fixture page content");
    document
        .get_object_mut(page_id)
        .and_then(Object::as_dict_mut)
        .expect("page dictionary")
        .get_mut(b"Resources")
        .and_then(Object::as_dict_mut)
        .expect("page resources")
        .set("XObject", dictionary! { "Scan" => image_id });

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("fixture PDF must serialize");
    bytes
}

/// Block mode is for a page whose unmapped text is what OCR reads. A scan whose text layer
/// is unmapped is still a scan, and keeps the segmentation mode a scan gets: block mode
/// loses the table reconstruction on a scanned table.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn scanned_page_with_unmapped_text_layer_keeps_the_scan_segmentation_mode() {
    use crate::core::config::{OcrConfig, PageConfig};

    const FABRICATED_NATIVE_TEXT: &str = "synthetic fabricated text layer drawn invisibly over an inset scan so the page is routed by its unmapped characters";
    let config = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(
            &inset_scan_with_unmapped_text_layer_pdf(FABRICATED_NATIVE_TEXT),
            "application/pdf",
            &config,
        )
        .await
        .expect("inset scan with an unmapped text layer should extract");
    let fabricated_text_pages = internal.metadata.format.as_ref().and_then(|format| match format {
        crate::types::FormatMetadata::Pdf(pdf) => pdf.fabricated_text_pages.clone(),
        _ => None,
    });
    assert_eq!(
        fabricated_text_pages,
        Some(vec![1]),
        "the fixture must be routed by its unmapped text layer"
    );
    let entries = internal
        .metadata
        .additional
        .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
        .and_then(serde_json::Value::as_array)
        .expect("the OCR'd page must record its effective segmentation mode");

    assert_eq!(entries.len(), 1, "one OCR'd page: {entries:?}");
    assert_eq!(entries[0]["page_number"], serde_json::json!(1));
    assert_ne!(
        entries[0]["psm"],
        serde_json::json!(6),
        "a scan must not take block mode because its text layer is unmapped"
    );
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn explicit_force_ocr_does_not_apply_the_automatic_block_mode() {
    use crate::core::config::{OcrConfig, PageConfig};

    const FABRICATED_NATIVE_TEXT: &str = "synthetic fabricated text used to verify explicit whole document ocr retains the caller controlled segmentation mode";
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(
            &identity_h_mapping_pdf(FABRICATED_NATIVE_TEXT, false),
            "application/pdf",
            &config,
        )
        .await
        .expect("explicit whole-document OCR should succeed");
    let entries = internal
        .metadata
        .additional
        .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
        .and_then(serde_json::Value::as_array);

    assert!(entries.is_none_or(|entries| entries.iter().all(|entry| entry["psm"] != serde_json::json!(6))));
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn explicit_force_ocr_pages_does_not_apply_the_automatic_block_mode() {
    use crate::core::config::{OcrConfig, PageConfig};

    const FABRICATED_NATIVE_TEXT: &str = "synthetic fabricated text used to verify explicit selected page ocr retains the caller controlled segmentation mode";
    let config = ExtractionConfig {
        force_ocr_pages: Some(vec![1]),
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(
            &identity_h_mapping_pdf(FABRICATED_NATIVE_TEXT, false),
            "application/pdf",
            &config,
        )
        .await
        .expect("explicit selected-page OCR should succeed");
    let entries = internal
        .metadata
        .additional
        .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
        .and_then(serde_json::Value::as_array);

    assert!(entries.is_none_or(|entries| entries.iter().all(|entry| entry["psm"] != serde_json::json!(6))));
}

/// xberg#1696's false-positive control: a Type0/Identity-H font that DOES carry a
/// `/ToUnicode` CMap resolves `MappingProvenance::ToUnicode`, never `Fallback`, so
/// `apply_flagged_pages` (issue #1667's fix) must not route it to OCR --
/// the fix targets fonts with no usable mapping tier at all, not every Type0 font.
/// Reported alongside #1667's own test since both extract the same fixture shape through
/// the real `run.rs`/`font_dict.rs` provenance path, differing only in `/ToUnicode`
/// presence.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_auto_strategy_keeps_native_text_when_page_carries_tounicode_map() {
    use crate::core::config::{OcrConfig, PageConfig};

    const OCR_TEXT: &str = "mock ocr text that must never appear in a clean extraction";
    let _backend = register_mock_ocr_backend("pdf-1696-tounicode-false-positive", OCR_TEXT);

    // > 64 non-whitespace chars (`OcrQualityThresholds::min_total_non_whitespace`'s
    // default): a shorter fixture never reaches the fabricated-ratio check at all,
    // which would make the mutation-testing control below pass for the wrong reason. ~keep
    const NATIVE_TEXT: &str = "genuinely mapped native text read from the ToUnicode character map identity range covering printable ascii glyphs correctly";

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "pdf-1696-tounicode-false-positive".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let internal = PdfExtractor::new()
        .extract_content(&identity_h_mapping_pdf(NATIVE_TEXT, true), "application/pdf", &config)
        .await
        .expect("ToUnicode-mapped PDF extraction should succeed");
    let derived =
        crate::extraction::derive::derive_extraction_result(internal, false, crate::core::config::OutputFormat::Plain);

    assert!(
        derived.content.contains(NATIVE_TEXT),
        "genuinely mapped native text must survive to the final content: {:?}",
        derived.content
    );
    assert!(
        !derived.content.contains(OCR_TEXT),
        "a page with a real ToUnicode mapping must not be routed to OCR: {:?}",
        derived.content
    );
    assert_eq!(
        derived.extraction_method,
        Some(ExtractionMethod::Native),
        "Auto strategy must keep extraction_method: native for a genuinely mapped page"
    );

    let fabricated_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
        crate::types::FormatMetadata::Pdf(pdf) => pdf.fabricated_text_pages.clone(),
        _ => None,
    });
    assert_eq!(
        fabricated_text_pages,
        Some(Vec::new()),
        "a page whose mapping resolves via ToUnicode must not be listed as fabricated"
    );
}

#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
async fn test_pdf_force_ocr_without_ocr_config() {
    use crate::core::config::ExtractionConfig;

    let extractor = PdfExtractor::new();

    let config = ExtractionConfig {
        force_ocr: true,
        ocr: None,
        ..Default::default()
    };

    let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/multi_page.pdf");
    if let Ok(content) = std::fs::read(pdf_path) {
        let result = extractor.extract_content(&content, "application/pdf", &config).await;

        if let Err(e) = result {
            assert!(
                !e.to_string().contains("OCR config required for force_ocr"),
                "Should not require manual OCR config when force_ocr is true"
            );
        }
    }
}

/// Verifies that per-page OCR text segments correctly override native page
/// content in each `PageContent` entry (#928).
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
async fn test_ocr_page_texts_override_native_page_content() {
    use crate::core::config::OcrConfig;
    use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
    use crate::types::ExtractedDocument;
    use std::sync::Arc;

    struct PerPageMockBackend;

    #[async_trait::async_trait]
    impl OcrBackend for PerPageMockBackend {
        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
        fn supports_language(&self, _: &str) -> bool {
            true
        }
        async fn process_image(&self, data: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
            let width = image::load_from_memory(data)
                .expect("mock input must be an image")
                .width();
            let target_dpi = if width == 1 { 150 } else { 300 };
            Ok(ExtractedDocument {
                content: format!("ocr-page-{width}"),
                metadata: crate::types::Metadata {
                    image_preprocessing: Some(crate::types::ImagePreprocessingMetadata {
                        original_dimensions: (100, 200).into(),
                        original_dpi: (72.0, 72.0).into(),
                        target_dpi,
                        scale_factor: 1.0,
                        auto_adjusted: false,
                        final_dpi: target_dpi,
                        new_dimensions: None,
                        resample_method: "LANCZOS3".to_string(),
                        dimension_clamped: false,
                        calculated_dpi: None,
                        skipped_resize: true,
                        resize_error: None,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            })
        }
        fn supports_document_processing(&self) -> bool {
            false
        }
    }

    impl Plugin for PerPageMockBackend {
        fn name(&self) -> &str {
            "per-page-ocr-mock-928"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    crate::plugins::register_ocr_backend(Arc::new(PerPageMockBackend)).unwrap();

    use image::ImageEncoder as _;
    let make_png = |width| {
        let img = image::DynamicImage::new_rgb8(width, 1);
        let rgb = img.to_rgb8();
        let (w, h) = rgb.dimensions();
        let mut buf = std::io::Cursor::new(Vec::new());
        image::codecs::png::PngEncoder::new(&mut buf)
            .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
            .unwrap();
        image::load_from_memory(&buf.into_inner()).unwrap()
    };
    let images = vec![make_png(1), make_png(2)];

    let config = crate::core::config::ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "per-page-ocr-mock-928".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = super::ocr::extract_with_ocr(
        None,
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
    )
    .await;

    crate::plugins::unregister_ocr_backend("per-page-ocr-mock-928").unwrap();

    let (_text, _conf, _tables, _elems, _doc, _llm, page_texts, _rasters, _formulas, preprocessing, _) =
        result.expect("extract_with_ocr should succeed");

    assert_eq!(page_texts.len(), 2, "expected one entry per page");
    assert!(page_texts[0].starts_with("ocr-page-"), "page 0 should have OCR text");
    assert!(page_texts[1].starts_with("ocr-page-"), "page 1 should have OCR text");
    assert_ne!(page_texts[0], page_texts[1], "each page should get unique OCR text");
    let mut target_dpis = preprocessing
        .into_iter()
        .map(|(page, metadata)| (page, metadata.target_dpi))
        .collect::<Vec<_>>();
    target_dpis.sort_unstable();
    assert_eq!(target_dpis, vec![(1, 150), (2, 300)]);
}

/// Verifies that when a VLM returns a single string for a multi-page PDF,
/// the guard clears stale native text on secondary pages (#928).
#[cfg(feature = "ocr")]
#[test]
fn test_vlm_single_string_guard_clears_secondary_pages() {
    use crate::types::PageContent;

    let vlm_text = "whole-doc VLM summary".to_string();
    let pts = vec![vlm_text.clone()];

    let mut pages: Vec<PageContent> = (1u32..=3u32)
        .map(|n| PageContent {
            page_number: n,
            content: format!("native page {n}"),
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
        })
        .collect();

    replace_pages_with_ocr_page_texts(&mut pages, &pts, false);

    assert_eq!(pages[0].content, vlm_text, "page 1 should carry the VLM text");
    assert!(pages[1].content.is_empty(), "page 2 must be cleared by VLM guard");
    assert!(pages[2].content.is_empty(), "page 3 must be cleared by VLM guard");
    assert_eq!(
        pages[0].is_blank,
        Some(false),
        "page 1 has VLM content so must not be blank"
    );
    assert_eq!(pages[1].is_blank, Some(true), "page 2 was cleared so must be blank");
    assert_eq!(pages[2].is_blank, Some(true), "page 3 was cleared so must be blank");
}

#[cfg(feature = "ocr")]
fn text_layer_page(page_number: u32, content: &str) -> crate::types::PageContent {
    crate::types::PageContent {
        page_number,
        content: content.to_string(),
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

#[cfg(feature = "ocr")]
#[test]
fn ocr_text_replaces_the_text_layer_and_keeps_it_only_when_asked() {
    let mut kept = text_layer_page(1, "Ref KX-204");
    replace_page_content_with_ocr_text(&mut kept, "Ref KX-2O4", true);
    assert_eq!(kept.content, "Ref KX-2O4");
    assert_eq!(kept.native_content.as_deref(), Some("Ref KX-204"));
    assert_eq!(kept.is_blank, Some(false));

    let mut dropped = text_layer_page(1, "Ref KX-204");
    replace_page_content_with_ocr_text(&mut dropped, "Ref KX-2O4", false);
    assert_eq!(dropped.content, "Ref KX-2O4");
    assert_eq!(dropped.native_content, None);
}

#[cfg(feature = "ocr")]
#[test]
fn ocr_text_equal_to_the_text_layer_is_not_stored_twice() {
    let mut same = text_layer_page(1, "Ref KX-204");
    replace_page_content_with_ocr_text(&mut same, "Ref KX-204", true);
    assert_eq!(same.content, "Ref KX-204");
    assert_eq!(same.native_content, None);

    let mut different = text_layer_page(1, "Ref KX-204");
    replace_page_content_with_ocr_text(&mut different, "Ref KX-204 ", true);
    assert_eq!(different.native_content.as_deref(), Some("Ref KX-204"));
}

#[cfg(feature = "ocr")]
#[test]
fn a_blank_text_layer_is_not_kept() {
    for blank in ["", " \n\t "] {
        let mut page = text_layer_page(1, blank);
        replace_page_content_with_ocr_text(&mut page, "scanned words", true);
        assert_eq!(page.content, "scanned words");
        assert_eq!(page.native_content, None, "text layer {blank:?}");
    }

    let mut one_character = text_layer_page(1, "7");
    replace_page_content_with_ocr_text(&mut one_character, "scanned words", true);
    assert_eq!(one_character.native_content.as_deref(), Some("7"));
}

#[cfg(feature = "ocr")]
#[test]
fn pages_cleared_by_a_single_ocr_text_keep_their_text_layer() {
    let pts = vec!["whole document text".to_string()];
    let mut pages: Vec<_> = (1u32..=3u32)
        .map(|n| text_layer_page(n, &format!("native page {n}")))
        .collect();

    replace_pages_with_ocr_page_texts(&mut pages, &pts, true);

    assert_eq!(pages[0].content, "whole document text");
    assert_eq!(pages[0].native_content.as_deref(), Some("native page 1"));
    for (page, native) in pages
        .iter()
        .zip(["native page 1", "native page 2", "native page 3"])
        .skip(1)
    {
        assert!(page.content.is_empty(), "page {} must be cleared", page.page_number);
        assert_eq!(page.is_blank, Some(true));
        assert_eq!(page.native_content.as_deref(), Some(native));
    }

    let mut not_kept: Vec<_> = (1u32..=2u32)
        .map(|n| text_layer_page(n, &format!("native page {n}")))
        .collect();
    replace_pages_with_ocr_page_texts(&mut not_kept, &pts, false);
    assert!(not_kept[1].content.is_empty());
    assert_eq!(not_kept[1].native_content, None);
}

/// Regression for #1095: when OCR texts are written into existing PageContent entries,
/// is_blank must be recalculated from the new content, not left stale from native extraction.
///
/// Simulates scanned PDF pages (all blank natively) receiving OCR content.
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_page_texts_update_is_blank_on_existing_pages() {
    use crate::types::PageContent;

    let pts = vec!["page one content".to_string(), "page two content".to_string()];

    let mut pages: Vec<PageContent> = (1u32..=2u32)
        .map(|n| PageContent {
            page_number: n,
            content: String::new(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: Some(true),
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        })
        .collect();

    replace_pages_with_ocr_page_texts(&mut pages, &pts, false);

    assert_eq!(
        pages[0].is_blank,
        Some(false),
        "page with OCR content must not be blank"
    );
    assert_eq!(
        pages[1].is_blank,
        Some(false),
        "page with OCR content must not be blank"
    );
}

/// Regression for #1095: pages built from OCR texts when no native pages exist
/// must have is_blank derived from the OCR content, not left as None.
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_scratch_pages_is_blank_set_from_content() {
    use crate::extraction::blank_detection::is_page_text_blank;
    use crate::types::PageContent;

    let pts = vec!["substantial ocr content".to_string(), String::new()];

    let page_contents: Vec<PageContent> = pts
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let content = crate::pdf::text::fix_pdf_control_chars(&text).into_owned();
            let is_blank = Some(is_page_text_blank(&content));
            PageContent {
                page_number: (i + 1) as u32,
                content,
                tables: Vec::new(),
                image_indices: vec![],
                image_preprocessing: None,
                hierarchy: None,
                is_blank,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: None,
            }
        })
        .collect();

    assert_eq!(
        page_contents[0].is_blank,
        Some(false),
        "page with content must not be blank"
    );
    assert_eq!(
        page_contents[1].is_blank,
        Some(true),
        "page with empty content must be blank"
    );
}

/// Integration regression for #1095: force_ocr on a scanned (non-searchable) PDF with
/// extract_pages=true must produce pages where is_blank reflects OCR content, not stale
/// native-extraction state.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_force_ocr_sets_is_blank_from_ocr_content() {
    use crate::core::config::{OcrConfig, PageConfig};

    let _backend = register_mock_ocr_backend("is-blank-fix-1095", "extracted ocr text content here");
    let extractor = PdfExtractor::new();
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "is-blank-fix-1095".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let pdf_path = pdf_test_document("non_searchable.pdf");
    let content = std::fs::read(&pdf_path).unwrap_or_else(|e| panic!("non_searchable.pdf must be readable: {e}"));
    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("extraction should succeed");
    let result =
        crate::extraction::derive::derive_extraction_result(result, false, crate::core::config::OutputFormat::Plain);

    let pages = result.pages.expect("pages must be present when extract_pages=true");
    assert!(!pages.is_empty(), "must have at least one page");
    for page in &pages {
        assert_eq!(
            page.is_blank,
            Some(false),
            "page {} has OCR content, is_blank must be Some(false) (issue #1095)",
            page.page_number
        );
    }
}

/// Regression for #1293: `metadata.pages.pages[].is_blank` must agree with the
/// top-level `pages[].is_blank` for OCR'd pages. Previously `PageInfo.is_blank`
/// was computed once from native content in `build_page_structure` and never
/// refreshed after OCR text overwrote `PageContent`, so a scanned page with no
/// native text but real OCR text reported `is_blank=true` in metadata while the
/// top-level page reported `is_blank=false`.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_metadata_page_info_is_blank_matches_page_content_after_ocr() {
    use crate::core::config::{OcrConfig, PageConfig};

    let _backend = register_mock_ocr_backend("is-blank-metadata-fix-1293", "extracted ocr text content here");
    let extractor = PdfExtractor::new();
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "is-blank-metadata-fix-1293".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let pdf_path = pdf_test_document("non_searchable.pdf");
    let content = std::fs::read(&pdf_path).unwrap_or_else(|e| panic!("non_searchable.pdf must be readable: {e}"));
    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("extraction should succeed");
    let result =
        crate::extraction::derive::derive_extraction_result(result, false, crate::core::config::OutputFormat::Plain);

    let pages = result
        .pages
        .clone()
        .expect("pages must be present when extract_pages=true");
    let metadata_pages = result
        .metadata
        .pages
        .as_ref()
        .and_then(|structure| structure.pages.as_ref())
        .expect("metadata.pages.pages must be present when extract_pages=true");

    assert!(!pages.is_empty(), "must have at least one page");
    assert_eq!(
        pages.len(),
        metadata_pages.len(),
        "page counts must match between the two sources"
    );

    for page in &pages {
        let info = metadata_pages
            .iter()
            .find(|p| p.number == page.page_number)
            .unwrap_or_else(|| panic!("metadata.pages.pages missing entry for page {}", page.page_number));
        assert_eq!(
            info.is_blank, page.is_blank,
            "page {}: metadata.pages.pages[].is_blank ({:?}) must match top-level pages[].is_blank ({:?}) (issue #1293)",
            page.page_number, info.is_blank, page.is_blank
        );
        assert_eq!(
            info.is_blank,
            Some(false),
            "page {} has OCR content, metadata is_blank must be Some(false) (issue #1293)",
            page.page_number
        );
    }
}

/// ocr_inline_images=true on a text-only PDF (no embedded images) must succeed
/// and return an empty images list, not panic or error.
#[tokio::test]
// `ocr_inline_images` is rejected by config validation unless an OCR BACKEND is
// compiled in, so the `pdf`-only feature leg cannot run this test -- gating it on
// `pdf` alone left that leg permanently red for a reason unrelated to PDF.
// `ocr`, not `ocr-pipeline`: `ocr = ["ocr-pipeline", ...]`, so the pipeline can be
// enabled with no backend registered, which fails one step later instead. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
async fn test_pdf_ocr_inline_images_no_images_in_document() {
    use crate::core::config::ExtractionConfig;
    use crate::core::config::pdf::PdfConfig;

    let extractor = PdfExtractor::new();

    let pdf_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/code_and_formula.pdf");

    if !pdf_path.exists() {
        panic!("missing test fixture: {pdf_path:?}");
    }

    let content = std::fs::read(pdf_path).expect("Failed to read PDF");

    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            ocr_inline_images: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("Extraction should succeed even when there are no images to OCR");

    for img in &result.images {
        assert!(img.ocr_result.is_none(), "text-only PDF should produce no OCR results");
    }
}

/// ocr_inline_images=true with config.ocr=None must use TesseractConfig::default()
/// and not panic.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
async fn test_pdf_ocr_inline_images_no_ocr_config() {
    use crate::core::config::ExtractionConfig;
    use crate::core::config::pdf::PdfConfig;

    let extractor = PdfExtractor::new();

    let pdf_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/embedded_images_tables.pdf");

    assert!(
        pdf_path.exists(),
        "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
    );

    let content = std::fs::read(pdf_path).expect("Failed to read PDF");

    let config = ExtractionConfig {
        ocr: None,
        pdf_options: Some(PdfConfig {
            ocr_inline_images: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let _result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("Extraction with ocr=None and ocr_inline_images=true must not panic");
}

/// Regression for issue #917: a mixed document with good aggregate text but a
/// scanned page must still reach OCR. Before the fix, `has_substantive_doc=true`
/// alone suppressed OCR even when `decision.fallback=true`.
///
/// Tests `evaluate_ocr_skip_gate` directly — the function that the production
/// path delegates to — so that reverting `&& !decision_fallback` breaks this test.
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_gate_runs_ocr_when_substantive_doc_but_fallback_needed() {
    let thresholds = OcrQualityThresholds::default();

    let outcome = ocr::evaluate_ocr_skip_gate(true, 500, 0.9, &mk_decision(true, true, vec![]), &thresholds);
    assert_eq!(
        outcome,
        ocr::OcrGateOutcome::RunFallback,
        "substantive doc must not suppress OCR when per-page fallback is needed (issue #917)"
    );
}

/// Counterpart: when no per-page fallback is needed, a substantive doc correctly
/// skips OCR. Ensures the fix doesn't over-correct and always run OCR.
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_gate_skips_when_substantive_doc_and_no_fallback() {
    let thresholds = OcrQualityThresholds::default();

    let outcome = ocr::evaluate_ocr_skip_gate(true, 500, 0.9, &mk_decision(false, false, vec![]), &thresholds);
    assert_eq!(
        outcome,
        ocr::OcrGateOutcome::SkipSubstantive,
        "OCR should be skipped when doc is substantive and no per-page fallback is needed"
    );
}

/// Regression for #987: image placeholders must appear in Markdown output when
/// `force_ocr` is used and `config.images.inject_placeholders = true`.
///
/// Uses a mock OCR backend so the test is independent of tessdata availability.
/// Image elements are appended after text on the OCR path; positional interleaving
/// is a known follow-up (tracked separately).
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_inject_placeholders_present_on_force_ocr_path() {
    use crate::core::config::{ImageExtractionConfig, OcrConfig, OutputFormat};

    let _backend = register_mock_ocr_backend("inject-placeholder-ocr", "mock page text");
    let extractor = PdfExtractor::new();

    let pdf_path = pdf_test_document("embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
    );
    let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Markdown,
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "inject-placeholder-ocr".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        images: Some(ImageExtractionConfig {
            extract_images: true,
            inject_placeholders: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("force_ocr extraction with images should succeed");

    let result = crate::extraction::derive::derive_extraction_result(result, true, OutputFormat::Markdown);

    if result.images.as_ref().is_some_and(|imgs| !imgs.is_empty()) {
        assert!(
            result.formatted_content.as_deref().unwrap_or("").contains("![") || result.content.contains("!["),
            "Markdown must contain image placeholders on the force_ocr path when inject_placeholders=true"
        );
    }
}

/// Verifies that `inject_placeholders` defaults to false when only
/// `pdf_options.extract_images` is set and `config.images` is absent,
/// so callers who never touched `config.images` do not get unexpected placeholders.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_inject_placeholders_absent_when_only_pdf_options_set() {
    use crate::core::config::{OcrConfig, OutputFormat, pdf::PdfConfig};

    let _backend = register_mock_ocr_backend("inject-placeholder-absent-ocr", "mock page text");
    let extractor = PdfExtractor::new();

    let pdf_path = pdf_test_document("embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
    );
    let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Markdown,
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "inject-placeholder-absent-ocr".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            extract_images: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("force_ocr extraction with pdf_options should succeed");

    let result = crate::extraction::derive::derive_extraction_result(result, true, OutputFormat::Markdown);

    let markdown = result.formatted_content.as_deref().unwrap_or(&result.content);
    assert!(
        !markdown.contains("!["),
        "Markdown must NOT contain image placeholders when config.images is absent (inject_placeholders defaults to false)"
    );
}

/// GH#1668 end to end: `include_document_structure` alone, with `output_format`
/// left at its `Plain` default and no hierarchy config, must trigger the
/// structured native pass. Before the fix, this config silently took the flat
/// path (`flat_pdf_document`, paragraph-only elements), so the structure tree
/// this flag asks for came back holding nothing but `paragraph` nodes even
/// though `embedded_images_tables.pdf` has a native table `counts.tables`
/// reports correctly either way.
#[tokio::test]
#[cfg(feature = "pdf")]
async fn test_include_document_structure_alone_triggers_the_structured_path() {
    use crate::core::config::OutputFormat;

    let extractor = PdfExtractor::new();
    let pdf_path = pdf_test_document("embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
    );
    let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

    let config = crate::core::config::ExtractionConfig {
        include_document_structure: true,
        ..Default::default()
    };

    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("native extraction with include_document_structure should succeed");

    let result = crate::extraction::derive::derive_extraction_result(result, true, OutputFormat::Plain);

    let structure = result
        .document
        .as_ref()
        .expect("include_document_structure=true must populate the document field");
    assert!(
        structure.node_types.iter().any(|kind| kind != "paragraph"),
        "structure must hold a non-paragraph node kind on a document with a native table; got {:?}",
        structure.node_types
    );
}

/// GH#1668 review follow-up: `test_include_document_structure_alone_triggers_the_structured_path`
/// only asserts `node_types`, never `content`, so it cannot catch a change that keeps the
/// structure tree correct while reverting the plain-text rendering the changelog also
/// promises. `mod.rs`'s `select_pdf_document` Mixed-branch merge already gates a similar
/// merge on `output_format != Plain` (see the comment there); a later change of that shape
/// applied to the native path would silently revert this fix without failing the existing
/// test. This test pins the plain-text side directly: with `include_document_structure` set,
/// `Plain` output must still come from the structured document, so each table row renders as
/// its own line (`render_table_plain` is `row.join(" ")` per row), and that output must differ
/// from the flag left off.
#[tokio::test]
#[cfg(feature = "pdf")]
async fn plain_output_with_document_structure_renders_each_table_row_on_its_own_line() {
    use crate::core::config::OutputFormat;

    let extractor = PdfExtractor::new();
    let pdf_path = pdf_test_document("embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
    );
    let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

    let flat_config = crate::core::config::ExtractionConfig {
        include_document_structure: false,
        ..Default::default()
    };
    let flat_result = extractor
        .extract_content(&content, "application/pdf", &flat_config)
        .await
        .expect("native extraction without include_document_structure should succeed");
    let flat_result = crate::extraction::derive::derive_extraction_result(flat_result, true, OutputFormat::Plain);

    let structured_config = crate::core::config::ExtractionConfig {
        include_document_structure: true,
        ..Default::default()
    };
    let structured_result = extractor
        .extract_content(&content, "application/pdf", &structured_config)
        .await
        .expect("native extraction with include_document_structure should succeed");
    let structured_result =
        crate::extraction::derive::derive_extraction_result(structured_result, true, OutputFormat::Plain);

    assert_eq!(
        flat_result.tables.len(),
        1,
        "fixture must carry exactly one native table with include_document_structure off"
    );
    assert_eq!(
        structured_result.tables.len(),
        1,
        "fixture must carry exactly one native table with include_document_structure on"
    );

    let table = &structured_result.tables[0];
    for row in &table.cells {
        let rendered_row = row.join(" ");
        assert!(
            structured_result.content.lines().any(|line| line == rendered_row),
            "expected table row {rendered_row:?} to render as its own line in the structured Plain content; got {:?}",
            structured_result.content
        );
    }

    assert_ne!(
        flat_result.content, structured_result.content,
        "include_document_structure must change the rendered Plain content for a document with a native table"
    );
}

/// Regression for #1355: when `force_ocr` renders a page blank (as xberg_native_pdf does
/// for a page whose only visible content is an image XObject it silently failed to
/// decode) but the page actually carries image XObjects, OCR must be retried on the
/// embedded image bytes and the recovery must be surfaced as a `ProcessingWarning`.
///
/// The mock backend returns an empty string for its first invocation (standing in
/// for the blank whole-page render) and real text for every subsequent call. Because
/// `embedded_images_tables.pdf` has exactly one page, the whole-page OCR call is
/// always the first call the backend receives, so the fallback calls that follow in
/// the same per-page loop deterministically observe call index >= 1.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_force_ocr_image_xobject_fallback_recovers_blank_page_and_warns() {
    use crate::core::config::OcrConfig;
    use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
    use crate::types::ExtractedDocument;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const BACKEND_NAME: &str = "blank-then-image-fallback-mock-1355";

    struct BlankThenImageMockBackend {
        call_count: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl OcrBackend for BlankThenImageMockBackend {
        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
        fn supports_language(&self, _lang: &str) -> bool {
            true
        }
        async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> crate::Result<ExtractedDocument> {
            let call_index = self.call_count.fetch_add(1, Ordering::SeqCst);
            let content = if call_index == 0 {
                String::new()
            } else {
                "IMG_TEXT".to_string()
            };
            Ok(ExtractedDocument {
                content,
                ..Default::default()
            })
        }
        fn supports_document_processing(&self) -> bool {
            false
        }
    }

    impl Plugin for BlankThenImageMockBackend {
        fn name(&self) -> &str {
            BACKEND_NAME
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    crate::plugins::register_ocr_backend(Arc::new(BlankThenImageMockBackend {
        call_count: AtomicUsize::new(0),
    }))
    .unwrap();
    let _guard = RegisteredOcrBackendGuard { name: BACKEND_NAME };

    let extractor = PdfExtractor::new();
    let pdf_path = pdf_test_document("embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
    );
    let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

    let config = crate::core::config::ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: BACKEND_NAME.to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("force_ocr extraction should succeed");
    let result =
        crate::extraction::derive::derive_extraction_result(result, false, crate::core::config::OutputFormat::Plain);

    assert!(
        !crate::extraction::blank_detection::is_page_text_blank(&result.content),
        "page content must be recovered from the embedded-image OCR fallback, not left blank; got {:?}",
        result.content
    );
    assert!(
        result.content.contains("IMG_TEXT"),
        "recovered content must come from the per-image fallback OCR call; got {:?}",
        result.content
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|w| w.source == "ocr" && w.message.contains("image XObject")),
        "expected a ProcessingWarning with source 'ocr' mentioning 'image XObject'; got: {:?}",
        result.processing_warnings
    );
}

/// False-positive guard for #1355: when the whole-page OCR result is not blank, the
/// image-XObject fallback must never fire, even on a page that has real embedded
/// images (`embedded_images_tables.pdf`) — no fallback warning, and page text
/// unchanged from whatever the (mock) OCR backend returned.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_force_ocr_image_xobject_fallback_does_not_fire_on_non_blank_page() {
    use crate::core::config::OcrConfig;

    const BACKEND_NAME: &str = "non-blank-no-fallback-mock-1355";
    const SENTINEL: &str = "genuine OCR text, not blank";

    let _backend = register_mock_ocr_backend(BACKEND_NAME, SENTINEL);

    let extractor = PdfExtractor::new();
    let pdf_path = pdf_test_document("embedded_images_tables.pdf");
    let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

    let config = crate::core::config::ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: BACKEND_NAME.to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extractor
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("force_ocr extraction should succeed");
    let result =
        crate::extraction::derive::derive_extraction_result(result, false, crate::core::config::OutputFormat::Plain);

    // The fixture also carries real tables that get merged into the final content
    // independently of OCR (unrelated to this fix), so assert the OCR-produced text
    // is present unchanged rather than an exact match on the whole document body.
    assert!(
        result.content.starts_with(SENTINEL),
        "non-blank OCR output must pass through unchanged when the fallback never fires; got {:?}",
        result.content
    );
    assert!(
        !result.content.contains("IMG_TEXT"),
        "the image fallback must never run when the whole-page OCR result was not blank; got {:?}",
        result.content
    );
    assert!(
        !result
            .processing_warnings
            .iter()
            .any(|w| w.message.contains("image XObject")),
        "no image-fallback warning must be added when the whole-page OCR result was not blank; got: {:?}",
        result.processing_warnings
    );
}

/// Fallback must win over the non-text skip even with a pre-rendered document. ~keep
/// Otherwise scanned ToC-like pages are suppressed. ~keep
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_gate_fallback_overrides_non_textual_skip() {
    let thresholds = OcrQualityThresholds::default();

    let outcome = ocr::evaluate_ocr_skip_gate(true, 500, 0.1, &mk_decision(true, false, vec![]), &thresholds);
    assert_eq!(
        outcome,
        ocr::OcrGateOutcome::RunFallback,
        "fallback must route to OCR rather than being suppressed as non-text"
    );

    let control = ocr::evaluate_ocr_skip_gate(true, 500, 0.1, &mk_decision(false, false, vec![]), &thresholds);
    assert_eq!(
        control,
        ocr::OcrGateOutcome::SkipNonText,
        "genuinely non-textual content without a fallback decision must still skip OCR"
    );
}

/// Hybrid PDF: per-page check fires on specific pages while the whole-document
/// quality check passes. Gate must route to RunFallbackOnPages, not RunFallback.
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_gate_targets_specific_pages_on_hybrid_pdf() {
    let thresholds = OcrQualityThresholds::default();

    let outcome = ocr::evaluate_ocr_skip_gate(false, 500, 0.9, &mk_decision(true, false, vec![3, 7]), &thresholds);
    assert_eq!(
        outcome,
        ocr::OcrGateOutcome::RunFallbackOnPages(vec![3, 7]),
        "hybrid PDF with specific failing pages must route to targeted OCR"
    );
}

/// Whole-document failure with no per-page list → full document OCR (existing behaviour).
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_gate_full_document_when_whole_doc_failure() {
    let thresholds = OcrQualityThresholds::default();
    let outcome = ocr::evaluate_ocr_skip_gate(false, 500, 0.9, &mk_decision(true, true, vec![]), &thresholds);
    assert_eq!(outcome, ocr::OcrGateOutcome::RunFallback);
}

/// Edge case: whole-doc failure is true AND per-page list is populated.
/// Whole-doc failure dominates (the document is fundamentally bad).
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_gate_whole_doc_failure_dominates_per_page_list() {
    let thresholds = OcrQualityThresholds::default();
    let outcome = ocr::evaluate_ocr_skip_gate(false, 500, 0.9, &mk_decision(true, true, vec![1, 2, 3]), &thresholds);
    assert_eq!(
        outcome,
        ocr::OcrGateOutcome::RunFallback,
        "whole-doc failure must trigger full OCR even when per-page list is populated"
    );
}

/// evaluate_per_page_ocr must collect ALL failing pages, not short-circuit on the first.
/// This is the core regression the original implementation had.
#[cfg(feature = "ocr")]
#[test]
fn test_evaluate_per_page_ocr_collects_all_failing_pages() {
    use crate::types::PageBoundary;

    let good = "This page has plenty of meaningful searchable text content for extraction.";
    let bad = " . ; ";
    let text = format!("{}{}{}{}", good, bad, good, bad);
    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: good.len(),
            page_number: 1,
        },
        PageBoundary {
            byte_start: good.len(),
            byte_end: good.len() + bad.len(),
            page_number: 2,
        },
        PageBoundary {
            byte_start: good.len() + bad.len(),
            byte_end: 2 * good.len() + bad.len(),
            page_number: 3,
        },
        PageBoundary {
            byte_start: 2 * good.len() + bad.len(),
            byte_end: text.len(),
            page_number: 4,
        },
    ];

    let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(4), &OcrQualityThresholds::default());
    assert!(decision.fallback);
    assert_eq!(
        decision.failing_pages,
        vec![2, 4],
        "all failing pages must be collected, not just the first"
    );
    assert!(
        !decision.whole_doc_failure,
        "whole-doc check should pass when half the document has good text"
    );
}

#[cfg(feature = "ocr")]
#[test]
fn test_per_page_ocr_keeps_good_page_when_document_average_fails() {
    use crate::types::PageBoundary;

    let good = "Native words remain searchable with the page table.";
    let mut boundaries = vec![PageBoundary {
        byte_start: 0,
        byte_end: good.len(),
        page_number: 1,
    }];
    boundaries.extend((2..=10).map(|page_number| PageBoundary {
        byte_start: good.len(),
        byte_end: good.len(),
        page_number,
    }));

    let decision = ocr::evaluate_per_page_ocr(good, Some(&boundaries), Some(10), &OcrQualityThresholds::default());

    assert_eq!(decision.failing_pages, (2..=10).collect::<Vec<_>>());
    assert!(!decision.whole_doc_failure);
    assert_eq!(
        ocr::evaluate_ocr_skip_gate(false, good.len(), 0.1, &decision, &OcrQualityThresholds::default()),
        ocr::OcrGateOutcome::RunFallbackOnPages((2..=10).collect())
    );
}

/// When every page fails the per-page quality check, the gate must route to
/// RunFallback (ExtractionMethod::Ocr), not RunFallbackOnPages (ExtractionMethod::Mixed).
/// A document where every page needs OCR is not a mixed document.
#[cfg(feature = "ocr")]
#[test]
fn test_all_pages_failing_routes_to_run_fallback_not_mixed() {
    use crate::types::PageBoundary;

    let bad = " . ; ";
    let text = format!("{}{}", bad, bad);
    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: bad.len(),
            page_number: 1,
        },
        PageBoundary {
            byte_start: bad.len(),
            byte_end: text.len(),
            page_number: 2,
        },
    ];

    let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
    assert!(decision.fallback);
    assert_eq!(decision.failing_pages, vec![1, 2]);
    assert!(
        decision.whole_doc_failure,
        "all pages failing must set whole_doc_failure so gate routes to RunFallback"
    );

    let outcome = ocr::evaluate_ocr_skip_gate(false, text.len(), 0.1, &decision, &OcrQualityThresholds::default());
    assert_eq!(
        outcome,
        ocr::OcrGateOutcome::RunFallback,
        "all-pages-failing must produce RunFallback (Ocr), not RunFallbackOnPages (Mixed)"
    );
}

/// Mock backend that records the OcrConfig it receives so tests can assert
/// that fields like output_format are forwarded correctly.
#[cfg(all(feature = "pdf", feature = "ocr"))]
struct ConfigCapturingBackend {
    name: &'static str,
    sentinel: &'static str,
    received_config: std::sync::Arc<std::sync::Mutex<Option<crate::core::config::OcrConfig>>>,
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
impl crate::plugins::Plugin for ConfigCapturingBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "0.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[async_trait::async_trait]
impl crate::plugins::OcrBackend for ConfigCapturingBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(
        &self,
        _image_bytes: &[u8],
        config: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        *self.received_config.lock().unwrap() = Some(config.clone());
        Ok(crate::types::ExtractedDocument {
            content: self.sentinel.to_string(),
            mime_type: std::borrow::Cow::Borrowed("text/plain"),
            ocr_elements: Some(vec![crate::types::OcrElement {
                text: "backend element".to_string(),
                page_number: 1,
                ..Default::default()
            }]),
            ..Default::default()
        })
    }
}

/// Regression for #1088: ocr_inline_images must call the backend named in
/// OcrConfig.backend, not always Tesseract via OcrProcessor.
///
/// Uses the existing register_mock_ocr_backend helper. The sentinel string
/// appearing in img.ocr_result.content proves which backend ran — no separate
/// AtomicBool needed.
///
/// Fixture note: with_images.pdf is used here (not embedded_images_tables.pdf)
/// because xberg_native_pdf reliably extracts its single raster XObject.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_ocr_inline_images_uses_configured_backend() {
    const BACKEND_NAME: &str = "mock-inline-ocr-routing-1088";
    const SENTINEL: &str = "__inline_ocr_sentinel_1088__";
    let _guard = register_mock_ocr_backend(BACKEND_NAME, SENTINEL);

    let pdf_path = pdf_test_document("with_images.pdf");
    assert!(pdf_path.exists(), "missing test fixture: {pdf_path:?}");
    let content = std::fs::read(&pdf_path).expect("read fixture");

    let config = crate::core::config::ExtractionConfig {
        ocr: Some(crate::core::config::OcrConfig {
            backend: BACKEND_NAME.to_string(),
            ..Default::default()
        }),
        pdf_options: Some(crate::core::config::pdf::PdfConfig {
            ocr_inline_images: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = PdfExtractor::new()
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("extraction must not fail");

    assert!(
        !result.images.is_empty(),
        "with_images.pdf must yield at least one embedded image; \
             fixture may need replacing if xberg_native_pdf no longer extracts from it"
    );

    let images_with_ocr: Vec<_> = result.images.iter().filter(|img| img.ocr_result.is_some()).collect();

    assert!(
        !images_with_ocr.is_empty(),
        "at least one image must have an ocr_result when ocr_inline_images=true"
    );

    for img in &images_with_ocr {
        let nested = img.ocr_result.as_ref().unwrap();
        let content = nested.content.as_str();
        assert!(
            content.contains(SENTINEL),
            "ocr_result content '{content}' does not contain sentinel — \
                 backend routing is still going through hardcoded Tesseract"
        );
        assert!(
            nested.ocr_elements.is_none(),
            "a custom inline-image backend must not bypass the caller's absent element policy"
        );
    }
}

/// Verifies that the extraction-level output_format is forwarded to the backend
/// via OcrConfig.output_format. This mirrors the standalone image extractor
/// (image.rs) and allows backends that produce format-aware output (e.g. Markdown
/// table rendering) to behave correctly for inline PDF images.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_ocr_inline_images_forwards_output_format() {
    use std::sync::{Arc, Mutex};
    const BACKEND_NAME: &str = "mock-inline-ocr-format-1088";
    const SENTINEL: &str = "__format_sentinel_1088__";

    let received_config = Arc::new(Mutex::new(None));
    let backend = Arc::new(ConfigCapturingBackend {
        name: BACKEND_NAME,
        sentinel: SENTINEL,
        received_config: Arc::clone(&received_config),
    });
    crate::plugins::register_ocr_backend(backend).unwrap();
    let _guard = RegisteredOcrBackendGuard { name: BACKEND_NAME };

    let pdf_path = pdf_test_document("with_images.pdf");
    assert!(pdf_path.exists(), "missing test fixture: {pdf_path:?}");
    let content = std::fs::read(&pdf_path).expect("read fixture");

    let config = crate::core::config::ExtractionConfig {
        output_format: crate::core::config::OutputFormat::Markdown,
        ocr: Some(crate::core::config::OcrConfig {
            backend: BACKEND_NAME.to_string(),
            ..Default::default()
        }),
        pdf_options: Some(crate::core::config::pdf::PdfConfig {
            ocr_inline_images: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = PdfExtractor::new()
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("extraction must not fail");

    if result.images.is_empty() {
        panic!("with_images.pdf must yield images; fixture may need replacing");
    }

    let captured = received_config.lock().unwrap();
    let captured_config = captured
        .as_ref()
        .expect("backend was never called — no images were processed");
    assert_eq!(
        captured_config.output_format,
        Some(crate::core::config::OutputFormat::Markdown),
        "output_format was not forwarded to the inline-image OCR backend"
    );
}

/// When ocr_inline_images is false the mock backend must NOT be called even
/// though it is registered as the configured backend.
#[tokio::test]
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[serial]
async fn test_ocr_inline_images_disabled_does_not_call_backend() {
    const BACKEND_NAME: &str = "mock-inline-ocr-disabled-1088";
    let _guard = register_mock_ocr_backend(BACKEND_NAME, "should-never-appear");

    let pdf_path = pdf_test_document("with_images.pdf");
    assert!(pdf_path.exists(), "missing test fixture: {pdf_path:?}");
    let content = std::fs::read(&pdf_path).expect("read fixture");

    let config = crate::core::config::ExtractionConfig {
        ocr: Some(crate::core::config::OcrConfig {
            backend: BACKEND_NAME.to_string(),
            ..Default::default()
        }),
        pdf_options: Some(crate::core::config::pdf::PdfConfig {
            ocr_inline_images: false,
            extract_images: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = PdfExtractor::new()
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("extraction must not fail");

    assert!(
        !result.images.is_empty(),
        "with_images.pdf must yield images; fixture may need replacing"
    );
    for img in &result.images {
        assert!(
            img.ocr_result.is_none(),
            "img {} on page {:?} has ocr_result even though ocr_inline_images=false",
            img.image_index,
            img.page_number,
        );
    }
}

/// Tests form field extraction from a PDF.
/// Uses an existing test PDF rather than creating one programmatically.
/// This is a simple smoke test to verify that form field extraction works
/// and doesn't panic or crash the extraction pipeline.
/// Path to the vendored fillable-form fixture (AcroForm with text, button,
/// and choice fields).
#[cfg(feature = "pdf")]
fn form_test_document() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/vendored/pdfium-render/form-test.pdf")
}

/// End-to-end: a real AcroForm PDF yields populated, correctly-typed
/// `form_fields` on the extractor's `InternalDocument` carrier.
#[tokio::test]
#[cfg(feature = "pdf")]
async fn test_form_field_extraction_enabled() {
    let content = std::fs::read(form_test_document()).expect("read form-test.pdf fixture");

    let config = crate::core::config::ExtractionConfig {
        pdf_options: Some(crate::core::config::pdf::PdfConfig {
            extract_form_fields: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let internal_doc = PdfExtractor::new()
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("extraction must not fail");

    assert!(
        !internal_doc.form_fields.is_empty(),
        "AcroForm PDF must yield form fields, got none"
    );
    assert!(
        internal_doc.form_fields.iter().all(|f| !f.full_name.is_empty()),
        "every extracted field must have a full_name"
    );
    assert!(
        internal_doc
            .form_fields
            .iter()
            .any(|f| f.field_type == crate::types::FormFieldType::Text),
        "fixture has text fields; at least one must map to FormFieldType::Text"
    );
}

/// With `extract_form_fields = false`, the same AcroForm PDF yields no fields.
#[tokio::test]
#[cfg(feature = "pdf")]
async fn test_form_field_extraction_disabled() {
    let content = std::fs::read(form_test_document()).expect("read form-test.pdf fixture");

    let config = crate::core::config::ExtractionConfig {
        pdf_options: Some(crate::core::config::pdf::PdfConfig {
            extract_form_fields: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let internal_doc = PdfExtractor::new()
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("extraction must not fail");

    assert!(
        internal_doc.form_fields.is_empty(),
        "form fields must be empty when extract_form_fields is disabled"
    );
}

/// Tests that form fields are properly extracted and carried through the pipeline
/// using the InternalDocument.form_fields carrier (not metadata.additional).
/// This test manually constructs an InternalDocument with form_fields to verify
/// the carrier pattern works end-to-end, without relying on a complex PDF fixture.
#[test]
#[cfg(feature = "pdf")]
fn test_form_fields_carrier_via_internal_document() {
    let mut doc = InternalDocument::new("pdf");
    doc.mime_type = "application/pdf".to_string();

    doc.form_fields = vec![crate::types::PdfFormField {
        name: "full_name".to_string(),
        full_name: "form.full_name".to_string(),
        field_type: crate::types::FormFieldType::Text,
        value: Some("Ada Lovelace".to_string()),
        default_value: Some("Default Name".to_string()),
        flags: 0,
        page: None,
        bbox: None,
        max_length: None,
        tooltip: None,
    }];

    assert_eq!(doc.form_fields.len(), 1);
    let field = &doc.form_fields[0];
    assert_eq!(field.name, "full_name");
    assert_eq!(field.value, Some("Ada Lovelace".to_string()));
    assert_eq!(field.field_type, crate::types::FormFieldType::Text);

    assert!(
        doc.metadata.additional.get("_pdf_form_fields").is_none(),
        "metadata.additional should not contain _pdf_form_fields (leak check)"
    );

    let result =
        crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

    assert_eq!(result.form_fields.len(), 1);
    assert_eq!(result.form_fields[0].name, "full_name");
    assert_eq!(result.form_fields[0].value, Some("Ada Lovelace".to_string()));
}

/// Tests that with form_fields extraction disabled, the result contains no form fields.
#[test]
#[cfg(feature = "pdf")]
fn test_form_fields_disabled_no_leak() {
    let mut doc = InternalDocument::new("pdf");
    doc.mime_type = "application/pdf".to_string();

    assert!(doc.form_fields.is_empty());

    assert!(doc.metadata.additional.get("_pdf_form_fields").is_none());

    let result =
        crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

    assert!(result.form_fields.is_empty());
}

/// Two stroked boxes joined by a line that ends in a filled arrowhead, with a
/// label centred in each box. Mirrors the fixture in
/// `tests/diagram_dot_pdf.rs`, kept local so this unit test can inspect
/// `InternalDocument::diagrams` directly rather than through the renderer.
#[cfg(feature = "pdf")]
fn two_boxes_and_an_arrow_pdf() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let content = b"1 w 0 0 0 RG
10 150 80 30 re S
10 20 80 30 re S
50 150 m 50 56 l S
46 56 m 54 56 l 50 50 l h f
BT /F1 12 Tf 30 160 Td (Alpha) Tj ET
BT /F1 12 Tf 30 30 Td (Beta) Tj ET
"
    .to_vec();

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let content_id = document.add_object(Stream::new(dictionary! {}, content));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
    });
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("fixture PDF must save");
    bytes
}

/// #579 defect: recovery ran on every PDF regardless of the requested
/// output format, so an ordinary document paid for a second full text
/// pass on any page that happened to look graph-shaped. `diagrams` is
/// internal, so this has to be a unit test here rather than in the
/// integration suite, which can only see the rendered `content` string —
/// and `content` looks identical whether recovery ran and was discarded
/// or never ran at all, so it cannot tell the two apart.
#[cfg(feature = "pdf")]
#[tokio::test]
async fn recovery_is_skipped_unless_dot_output_is_requested() {
    let bytes = two_boxes_and_an_arrow_pdf();
    let extractor = PdfExtractor::new();

    let plain_doc = extractor
        .extract_content(&bytes, "application/pdf", &ExtractionConfig::default())
        .await
        .expect("plain extraction must succeed");
    assert_eq!(
        plain_doc.diagrams.len(),
        0,
        "diagram recovery must not run for non-dot output"
    );

    let dot_config = ExtractionConfig {
        output_format: crate::core::config::OutputFormat::Custom("dot".to_string()),
        ..Default::default()
    };
    let dot_doc = extractor
        .extract_content(&bytes, "application/pdf", &dot_config)
        .await
        .expect("dot extraction must succeed");
    assert_eq!(dot_doc.diagrams.len(), 1, "diagram recovery must run for dot output");
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn should_attach_distinct_preprocessing_metadata_to_each_pdf_page() {
    fn preprocessing(target_dpi: i32, final_dpi: i32) -> crate::types::ImagePreprocessingMetadata {
        crate::types::ImagePreprocessingMetadata {
            original_dimensions: (100, 200).into(),
            original_dpi: (72.0, 72.0).into(),
            target_dpi,
            scale_factor: 1.0,
            auto_adjusted: false,
            final_dpi,
            new_dimensions: None,
            resample_method: "LANCZOS3".to_string(),
            dimension_clamped: false,
            calculated_dpi: None,
            skipped_resize: true,
            resize_error: None,
        }
    }

    let mut pages = Some(vec![
        crate::types::PageContent {
            page_number: 1,
            content: "first".to_string(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: Some(false),
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        },
        crate::types::PageContent {
            page_number: 2,
            content: "second".to_string(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: Some(false),
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        },
    ]);
    let by_page = ahash::AHashMap::from([(1, preprocessing(150, 150)), (2, preprocessing(300, 300))]);
    let root_metadata = super::attach_pdf_preprocessing_metadata(&mut pages, &by_page);

    let pages = pages.expect("PDF OCR pages must remain present");
    assert_eq!(
        pages[0]
            .image_preprocessing
            .as_ref()
            .map(|metadata| metadata.target_dpi),
        Some(150)
    );
    assert_eq!(
        pages[1]
            .image_preprocessing
            .as_ref()
            .map(|metadata| metadata.target_dpi),
        Some(300)
    );
    assert_eq!(
        root_metadata.as_ref().map(|metadata| metadata.target_dpi),
        Some(150),
        "the singular compatibility field must deterministically report the first preprocessed page"
    );
}

/// A minimal 2-page PDF carrying real text content, for tests that only need page
/// structure and must not depend on the `test_documents/` corpus (see `read_test_fixture`).
#[cfg(feature = "pdf")]
fn build_two_page_text_pdf() -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Contents 5 0 R /Resources << /Font << /F1 7 0 R >> >> >>\nendobj\n",
    );

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Contents 6 0 R /Resources << /Font << /F1 7 0 R >> >> >>\nendobj\n",
    );

    let stream1 = "BT /F1 12 Tf 72 700 Td (Page one has real text content here.) Tj ET\n";
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            stream1.len(),
            stream1
        )
        .as_bytes(),
    );

    let stream2 = "BT /F1 12 Tf 72 700 Td (Page two has different text content here.) Tj ET\n";
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            stream2.len(),
            stream2
        )
        .as_bytes(),
    );

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let xref_pos = pdf.len();
    let total_objs = offsets.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {}\n", total_objs).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f\r\n");
    for &off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs, xref_pos
        )
        .as_bytes(),
    );
    pdf
}

/// GH#1752: `metadata.pages` must be populated when `ocr_near_empty_fallback = Some(true)`
/// is set with no `ocr` block, at parity with what an `ocr` block alone already produced.
///
/// Before the fix, `page_structure_was_implicitly_tracked` in `extraction.rs` discarded the
/// page structure whenever `config.ocr` was `None`, regardless of the two settings GH#1752
/// gave the near-empty-fallback and scanned-page-quality-gate behaviours. Both configs here
/// set `pdf_options: None` and `pages: None` so `force_annotation_page_tracking` is `true`,
/// the only condition under which this site is reachable. Config X (an `ocr` block) is
/// checked first as a control -- asserting it is `Some` before checking config Y rules out a
/// fixture that simply never produces page structure at all, which would make the Y
/// assertion pass vacuously.
#[cfg(feature = "pdf")]
#[tokio::test]
async fn metadata_pages_populated_for_near_empty_fallback_without_ocr_block() {
    use crate::core::config::OcrConfig;

    let pdf = build_two_page_text_pdf();
    let extractor = PdfExtractor::new();

    let config_x = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        pdf_options: None,
        pages: None,
        ..ExtractionConfig::default()
    };
    let result_x = extractor
        .extract_content(&pdf, "application/pdf", &config_x)
        .await
        .expect("extraction with an `ocr` block must succeed");
    let result_x =
        crate::extraction::derive::derive_extraction_result(result_x, false, crate::core::config::OutputFormat::Plain);
    let pages_x = result_x
        .metadata
        .pages
        .as_ref()
        .expect("control: an `ocr` block alone must populate metadata.pages");

    let config_y = ExtractionConfig {
        ocr: None,
        ocr_near_empty_fallback: Some(true),
        pdf_options: None,
        pages: None,
        ..ExtractionConfig::default()
    };
    let result_y = extractor
        .extract_content(&pdf, "application/pdf", &config_y)
        .await
        .expect("extraction with ocr_near_empty_fallback must succeed");
    let result_y =
        crate::extraction::derive::derive_extraction_result(result_y, false, crate::core::config::OutputFormat::Plain);
    let pages_y = result_y.metadata.pages.as_ref().expect(
        "ocr_near_empty_fallback = Some(true) without an `ocr` block must also populate \
             metadata.pages (GH#1752)",
    );

    assert_eq!(
        pages_y.total_count, pages_x.total_count,
        "page counts must match between the two configurations"
    );
}

/// `scanned_math.pdf` plus a prose page the `auto` layout gate skips.
#[cfg(all(feature = "ocr", feature = "formula-recognition", feature = "layout-detection"))]
fn scanned_math_then_prose_pdf() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr/scanned_math.pdf");
    let bytes = std::fs::read(path).expect("read scanned_math.pdf fixture");
    let mut document = Document::load_mem(&bytes).expect("scanned_math.pdf must parse");
    let pages_id = document
        .catalog()
        .and_then(|catalog| catalog.get(b"Pages"))
        .and_then(Object::as_reference)
        .expect("fixture has a page tree");
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let mut content = String::from("BT /F1 10 Tf 72 720 Td 14 TL\n");
    for line in 0..45 {
        content.push_str(&format!(
            "(Line {line} of steady single column prose with enough characters to pass the floor.) Tj T*\n"
        ));
    }
    content.push_str("ET");
    let content_id = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "Contents" => content_id,
    });
    let pages = document
        .get_object_mut(pages_id)
        .and_then(Object::as_dict_mut)
        .expect("page tree root is a dictionary");
    pages
        .get_mut(b"Kids")
        .and_then(Object::as_array_mut)
        .expect("page tree root has kids")
        .push(page_id.into());
    pages.set("Count", 2);

    let mut out = Vec::new();
    document.save_to(&mut out).expect("fixture PDF must serialize");
    out
}

#[cfg(all(feature = "ocr", feature = "formula-recognition", feature = "layout-detection"))]
#[tokio::test]
#[serial]
#[ignore = "runs the RT-DETR layout and RapidLaTeXOCR models (~310 MB, downloaded on first use)"]
async fn auto_gate_skipping_a_page_keeps_formula_recognition_on_the_pages_it_ran() {
    let content = scanned_math_then_prose_pdf();
    let config = ExtractionConfig {
        use_layout_for_markdown: true,
        disable_ocr: true,
        layout: Some(crate::core::config::LayoutDetectionConfig {
            strategy: crate::core::config::layout::LayoutStrategy::Auto,
            formula_model: Some(crate::core::config::layout::FormulaModel::LatexOcr),
            ..Default::default()
        }),
        ..Default::default()
    };

    let doc = PdfExtractor::new()
        .extract_content(&content, "application/pdf", &config)
        .await
        .expect("extraction must not fail");

    assert!(
        doc.formulas.iter().any(|formula| formula.page == Some(1)),
        "the gate ran layout on page 1, so its formula regions must be recognised; got {:?}",
        doc.formulas.iter().map(|f| (f.page, &f.latex)).collect::<Vec<_>>()
    );
}

/// ~keep A mixed PDF whose native page and scanned page each hold a table, extracted with the layout
/// model and without the table model. The native pass and the OCR pass each raise the
/// `pdf_tables` warning, and the returned document must carry it once.
///
/// The layout model is real. The child process gets a private Hugging Face cache that holds the
/// layout model only and runs offline, so the table model fails to load on both passes. Set
/// `XBERG_SKIP_LIVE_HF=1` to skip the test where the layout model cannot be fetched.
#[cfg(all(feature = "pdf", feature = "ocr", feature = "layout-detection"))]
mod mixed_pdf_table_model_warning {
    use super::super::PdfExtractor;
    use crate::core::config::layout::LayoutDetectionConfig;
    use crate::core::config::{ExtractionConfig, OcrConfig, OcrStrategy};
    use crate::plugins::InternalDocumentExtractor;
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    const CACHE_ROOT: &str = "XBERG_TABLE_MODEL_WARNING_TEST_HF_CACHE";
    const BACKEND_NAME: &str = "mixed-pdf-table-model-warning-test-backend";
    const LAYOUT_MODEL_REPO_DIR: &str = "models--xberg-io--layout-models";
    const ROWS: [[&str; 4]; 7] = [
        ["Region", "Q1 revenue", "Q2 revenue", "Change"],
        ["North", "12,400", "13,050", "+5.2%"],
        ["South", "9,870", "9,410", "-4.7%"],
        ["East", "15,230", "16,880", "+10.8%"],
        ["West", "11,005", "11,640", "+5.8%"],
        ["Central", "7,320", "7,515", "+2.7%"],
        ["Total", "55,825", "58,495", "+4.8%"],
    ];

    /// ~keep A ruled table of `ROWS` with a title above it, drawn as native text and lines.
    fn table_page_operations() -> Vec<Operation> {
        const LEFT: f32 = 72.0;
        const TOP: f32 = 640.0;
        const COLUMN_WIDTH: f32 = 117.0;
        const ROW_HEIGHT: f32 = 28.0;
        let width = COLUMN_WIDTH * ROWS[0].len() as f32;
        let height = ROW_HEIGHT * ROWS.len() as f32;
        let mut operations = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 16.into()]),
            Operation::new("Td", vec![LEFT.into(), (TOP + 30.0).into()]),
            Operation::new("Tj", vec![Object::string_literal("Quarterly revenue by region")]),
            Operation::new("ET", vec![]),
            Operation::new("w", vec![1.into()]),
        ];
        for row in 0..=ROWS.len() {
            let y = TOP - ROW_HEIGHT * row as f32;
            operations.push(Operation::new("m", vec![LEFT.into(), y.into()]));
            operations.push(Operation::new("l", vec![(LEFT + width).into(), y.into()]));
        }
        for column in 0..=ROWS[0].len() {
            let x = LEFT + COLUMN_WIDTH * column as f32;
            operations.push(Operation::new("m", vec![x.into(), TOP.into()]));
            operations.push(Operation::new("l", vec![x.into(), (TOP - height).into()]));
        }
        operations.push(Operation::new("S", vec![]));
        for (row_index, row) in ROWS.iter().enumerate() {
            for (column_index, cell) in row.iter().enumerate() {
                let x = LEFT + COLUMN_WIDTH * column_index as f32 + 8.0;
                let y = TOP - ROW_HEIGHT * row_index as f32 - 19.0;
                operations.push(Operation::new("BT", vec![]));
                operations.push(Operation::new("Tf", vec!["F1".into(), 11.into()]));
                operations.push(Operation::new("Td", vec![x.into(), y.into()]));
                operations.push(Operation::new("Tj", vec![Object::string_literal(*cell)]));
                operations.push(Operation::new("ET", vec![]));
            }
        }
        operations
    }

    fn add_scan(document: &mut Document, scan: &image::RgbImage) -> lopdf::ObjectId {
        document.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => scan.width() as i64,
                "Height" => scan.height() as i64,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
            },
            scan.as_raw().clone(),
        ))
    }

    fn page_content(
        page: &Option<Vec<Operation>>,
        font_id: lopdf::ObjectId,
        scan_id: Option<lopdf::ObjectId>,
    ) -> (Vec<Operation>, lopdf::Dictionary) {
        match page {
            Some(operations) => (
                operations.clone(),
                dictionary! { "Font" => dictionary! { "F1" => font_id } },
            ),
            None => (
                vec![
                    Operation::new("q", vec![]),
                    Operation::new(
                        "cm",
                        vec![612.into(), 0.into(), 0.into(), 792.into(), 0.into(), 0.into()],
                    ),
                    Operation::new("Do", vec![Object::Name(b"Scan".to_vec())]),
                    Operation::new("Q", vec![]),
                ],
                dictionary! {
                    "XObject" => dictionary! { "Scan" => scan_id.expect("a scanned page needs a raster") }
                },
            ),
        }
    }

    /// ~keep A PDF of the given page contents. Each page is a US Letter page; `scan` is the raster a
    /// page without operations shows across its whole area.
    fn pdf(pages: &[Option<Vec<Operation>>], scan: Option<&image::RgbImage>) -> Vec<u8> {
        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let scan_id = scan.map(|scan| add_scan(&mut document, scan));
        let mut kids: Vec<Object> = Vec::new();
        for page in pages {
            let (operations, resources) = page_content(page, font_id, scan_id);
            let content_id = document.add_object(Stream::new(
                dictionary! {},
                Content { operations }.encode().expect("page content must encode"),
            ));
            let page_id = document.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => resources,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            });
            kids.push(page_id.into());
        }
        let count = kids.len() as i64;
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
        );
        let catalog_id = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        document.trailer.set("Root", catalog_id);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("the PDF fixture must serialize");
        bytes
    }

    /// ~keep The number of `pdf_tables` warnings on the returned document, and all of its warnings.
    async fn table_model_warning_count(content: &[u8]) -> (usize, Vec<crate::types::ProcessingWarning>) {
        let config = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig::default()),
            use_layout_for_markdown: true,
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let doc = PdfExtractor::new()
            .extract_content(content, "application/pdf", &config)
            .await
            .expect("the extraction must succeed");
        let count = doc
            .processing_warnings
            .iter()
            .filter(|warning| warning.source == "pdf_tables")
            .count();
        (count, doc.processing_warnings)
    }

    fn run_without_table_model(child: &str) {
        if std::env::var("XBERG_SKIP_LIVE_HF").ok().as_deref() == Some("1") {
            eprintln!("XBERG_SKIP_LIVE_HF=1, skipping");
            return;
        }
        let layout_model = crate::layout::LayoutModelManager::new(None)
            .ensure_rtdetr_model()
            .expect("the layout model must be available");
        let repo_dir = layout_model
            .ancestors()
            .find(|dir| dir.file_name().is_some_and(|name| name == LAYOUT_MODEL_REPO_DIR))
            .expect("the layout model must sit in the Hugging Face cache layout");
        let relative = layout_model
            .strip_prefix(repo_dir.parent().expect("the repository directory has a parent"))
            .expect("the layout model sits under its repository directory");
        let cache = tempfile::TempDir::new().unwrap();
        let staged = cache.path().join(relative);
        std::fs::create_dir_all(staged.parent().expect("the staged model has a parent")).unwrap();
        std::fs::copy(&layout_model, &staged).expect("copy the layout model into the private cache");

        crate::utils::test_support::run_child_test(child, CACHE_ROOT, cache.path(), |command| {
            command
                .env("HF_HUB_CACHE", cache.path())
                .env("HF_HUB_OFFLINE", "1")
                .env_remove("HUGGINGFACE_HUB_OFFLINE");
        });
    }

    #[test]
    fn mixed_pdf_without_the_table_model_returns_one_table_model_warning() {
        run_without_table_model(
            "extractors::pdf::tests::mixed_pdf_table_model_warning::mixed_pdf_without_the_table_model_child",
        );
    }

    #[test]
    fn force_ocr_pdf_without_the_table_model_returns_one_table_model_warning() {
        run_without_table_model(
            "extractors::pdf::tests::mixed_pdf_table_model_warning::force_ocr_pdf_without_the_table_model_child",
        );
    }

    #[test]
    fn image_without_the_table_model_returns_one_table_model_warning() {
        run_without_table_model(
            "extractors::pdf::tests::mixed_pdf_table_model_warning::image_without_the_table_model_child",
        );
    }

    #[tokio::test]
    #[ignore = "run in an isolated subprocess by the force-OCR warning parent"]
    async fn force_ocr_pdf_without_the_table_model_child() {
        if crate::utils::test_support::child_test_root(CACHE_ROOT).is_none() {
            return;
        }
        let _backend = super::register_mock_ocr_backend(BACKEND_NAME, "Region Q1 revenue Q2 revenue Change");
        let config = ExtractionConfig {
            force_ocr: true,
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig::default()),
            ..Default::default()
        };
        let content = pdf(&[Some(table_page_operations())], None);
        let (native_count, native_warnings) = table_model_warning_count(&content).await;
        assert_eq!(
            native_count, 1,
            "control: the same table fixture must raise the native warning: {native_warnings:?}"
        );
        let doc = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("force-OCR extraction must succeed with the missing table model");
        let warnings: Vec<_> = doc
            .processing_warnings
            .iter()
            .filter(|warning| warning.source == "pdf_tables")
            .collect();
        assert_eq!(warnings.len(), 1, "got {:?}", doc.processing_warnings);
    }

    #[tokio::test]
    #[ignore = "run in an isolated subprocess by image_without_the_table_model_returns_one_table_model_warning"]
    async fn image_without_the_table_model_child() {
        if crate::utils::test_support::child_test_root(CACHE_ROOT).is_none() {
            return;
        }
        let _backend = super::register_mock_ocr_backend(BACKEND_NAME, "Region Q1 revenue Q2 revenue Change");
        let native = pdf(&[Some(table_page_operations())], None);
        let (native_count, native_warnings) = table_model_warning_count(&native).await;
        assert_eq!(
            native_count, 1,
            "control: the same table fixture must raise the native warning: {native_warnings:?}"
        );
        let png = crate::pdf::render::render_pdf_page_to_png(&native, 0, Some(150), None)
            .expect("the native table page must render");
        let config = ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig {
                element_config: Some(crate::types::OcrElementConfig {
                    include_elements: true,
                    min_level: crate::types::OcrElementLevel::Word,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig::default()),
            ..Default::default()
        };
        let doc = crate::extractors::image::ImageExtractor::new()
            .extract_content(&png, "image/png", &config)
            .await
            .expect("image extraction must succeed with real OCR and the missing table model");
        assert!(
            doc.prebuilt_ocr_elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty()),
            "the real OCR pass must produce positioned elements before table recognition is tested"
        );
        let warnings: Vec<_> = doc
            .processing_warnings
            .iter()
            .filter(|warning| warning.source == "image_tables")
            .collect();
        assert_eq!(warnings.len(), 1, "got {:?}", doc.processing_warnings);
    }

    #[tokio::test]
    #[ignore = "run in an isolated subprocess by mixed_pdf_without_the_table_model_returns_one_table_model_warning"]
    async fn mixed_pdf_without_the_table_model_child() {
        if crate::utils::test_support::child_test_root(CACHE_ROOT).is_none() {
            return;
        }
        let _backend = super::register_mock_ocr_backend(BACKEND_NAME, "Region Q1 revenue Q2 revenue Change");

        let native_only = pdf(&[Some(table_page_operations())], None);
        let png = crate::pdf::render::render_pdf_page_to_png(&native_only, 0, Some(150), None)
            .expect("the native table page must render");
        let scan = image::load_from_memory(&png)
            .expect("the rendered page must decode")
            .to_rgb8();
        let scanned_only = pdf(&[None], Some(&scan));
        let mixed = pdf(&[Some(table_page_operations()), None], Some(&scan));

        let (native_count, native_warnings) = table_model_warning_count(&native_only).await;
        assert_eq!(
            native_count, 1,
            "control: the native page must raise the warning: {native_warnings:?}"
        );
        let (scanned_count, scanned_warnings) = table_model_warning_count(&scanned_only).await;
        assert_eq!(
            scanned_count, 1,
            "control: the scanned page must raise the warning: {scanned_warnings:?}"
        );

        let (mixed_count, mixed_warnings) = table_model_warning_count(&mixed).await;
        assert_eq!(
            mixed_count, 1,
            "the mixed PDF must carry the warning once: {mixed_warnings:?}"
        );
    }
}
