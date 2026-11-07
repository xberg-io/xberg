#![cfg(feature = "pdf")]

use super::super::document::{LayerFirst, LineAt, put_layer_first_in_document};
use crate::types::document_structure::{AnnotationKind, TextAnnotation};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

fn repeated_rows(count: usize) -> (InternalDocument, LayerFirst, InternalElement) {
    let mut document = InternalDocument::new("pdf");
    let mut source = Vec::new();
    for row in 0..=count {
        let bbox = super::row_box(if row == count { count + 5 } else { row });
        let mut element = InternalElement::text(ElementKind::Paragraph, "Total 1OO", 0);
        element.page = Some(1);
        element.bbox = Some(crate::types::BoundingBox {
            x0: f64::from(bbox.left),
            y0: f64::from(bbox.top),
            x1: f64::from(bbox.right),
            y1: f64::from(bbox.bottom),
        });
        if row == count {
            element.annotations.push(TextAnnotation {
                start: 0,
                end: 5,
                kind: AnnotationKind::Bold,
            });
        }
        document.push_element(element);
        source.push(LineAt::new("Total 1OO", bbox));
    }
    let unowned = document.elements.last().unwrap().clone();
    let native = vec!["Total 100"; count];
    let boxes = super::page_boxes(&native, &source);
    let text = vec!["Total 1OO"; count + 1].join("\n");
    let first = super::super::document::layer_lines_to_put_first(
        &native.join("\n"),
        &text,
        false,
        &crate::core::config::OcrQualityThresholds::default(),
        Some(&boxes),
    )
    .expect("each native occurrence lacks literal OCR coverage");
    (document, first, unowned)
}

#[test]
fn should_preserve_all_equal_key_distinct_geometry_ocr_rows_while_restoring_native_rows() {
    let mut jobs = 0;
    for count in [250usize, 2000] {
        for sample in 0..3 {
            let start = std::time::Instant::now();
            let (mut document, first, unowned) = repeated_rows(count);
            assert_eq!(document.elements.len(), count + 1);
            assert_eq!(first.lines.lines().count(), count);
            let original = document.elements.clone();
            put_layer_first_in_document(&mut document, Some(1), &first);
            let elapsed_ns = start.elapsed().as_nanos();
            assert_eq!(document.elements.len(), count + 2);
            assert_eq!(
                document.elements[0].text.lines().collect::<Vec<_>>(),
                vec!["Total 100"; count]
            );
            assert_eq!(document.elements[1..], original);
            assert_eq!(document.elements.last().unwrap(), &unowned);
            assert!(elapsed_ns > 0);
            println!("STRUCTURED_RETENTION count={count} sample={sample} elapsed_ns={elapsed_ns} retained={count}");
            jobs += 1;
        }
    }
    assert_eq!(jobs, 6);
}

fn preserved_payload_case(kind: ElementKind, extra: bool, annotated: bool) {
    use super::super::document::{layer_lines_to_put_first, text_with_layer_first};
    use crate::core::config::OcrQualityThresholds;
    let layer = super::MINUTES.join("\n");
    let mut read = super::MINUTES[..7].to_vec();
    if extra {
        read.push("Paid today");
        assert!(!layer.split_whitespace().any(|word| word == "Paid" || word == "today"));
    } else {
        assert!(
            layer.starts_with(&read.join("\n")),
            "literal coverage alone cannot discard rich payload"
        );
    }
    let text = read.join("\n");
    let first = layer_lines_to_put_first(&layer, &text, false, &OcrQualityThresholds::default(), None).unwrap();
    let mut document = InternalDocument::new("pdf");
    for (index, text) in read.iter().enumerate() {
        let mut element = InternalElement::text(if index == 0 { kind } else { ElementKind::Paragraph }, *text, 0);
        element.page = Some(1);
        if annotated && index == if extra { read.len() - 1 } else { 0 } {
            element.annotations.push(TextAnnotation {
                start: 0,
                end: 4,
                kind: AnnotationKind::Bold,
            });
            element.bbox = Some(crate::types::BoundingBox {
                x0: 0.0,
                y0: 100.0,
                x1: 100.0,
                y1: 112.0,
            });
        }
        document.push_element(element);
    }
    if matches!(kind, ElementKind::Table { .. }) {
        document.tables.push(crate::types::Table {
            cells: vec![vec!["Harbour".into(), "committee".into(), "minutes".into()]],
            page_number: 1,
            ..Default::default()
        });
    }
    let expected_plain = matches!(kind, ElementKind::Paragraph) && !annotated;
    assert_eq!(
        super::super::document::plain_ocr_page_can_yield_to_native(&document, &text),
        expected_plain
    );
    let original = document.elements.clone();
    let tables = serde_json::to_value(&document.tables).unwrap();
    put_layer_first_in_document(&mut document, Some(1), &first);
    assert_eq!(first.lines, super::MINUTES[7]);
    assert_eq!(document.elements[1..], original);
    assert_eq!(serde_json::to_value(&document.tables).unwrap(), tables);
    assert_eq!(
        text_with_layer_first(&text, &first),
        format!("{}\n\n{text}", super::MINUTES[7])
    );
}

#[test]
fn should_keep_genuine_ocr_only_words_and_annotations_beside_restored_native_text() {
    preserved_payload_case(ElementKind::Paragraph, true, true);
}

#[test]
fn should_keep_ocr_heading_payload_even_when_native_text_covers_every_ocr_token() {
    preserved_payload_case(ElementKind::Heading { level: 2 }, false, false);
}

#[test]
fn should_keep_ocr_table_payload_even_when_native_text_covers_every_ocr_token() {
    preserved_payload_case(ElementKind::Table { table_index: 0 }, false, false);
}

#[test]
#[ignore = "requires the captured hash-pinned public recipe native-control output"]
fn should_report_public_recipe_native_quality_and_language_routing() {
    use super::super::scoring::{
        apply_flagged_pages, evaluate_native_text_for_ocr, evaluate_per_page_ocr, native_layer_is_clean,
    };
    use crate::core::config::OcrQualityThresholds;
    let root = std::path::PathBuf::from(std::env::var_os("XBERG_2439_RECIPE_ROOT").expect("public recipe root"));
    let native: crate::types::ExtractedDocument =
        serde_json::from_slice(&std::fs::read(root.join("candidate-native.json")).unwrap()).unwrap();
    let pages = native.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 8);
    let thresholds = OcrQualityThresholds::default();
    let p8 = &pages[7].content;
    let clean = native_layer_is_clean(p8, &thresholds);
    let structural = evaluate_native_text_for_ocr(p8, Some(1), &thresholds);
    let mut text = String::new();
    let mut bounds = Vec::new();
    for page in pages {
        let start = text.len();
        text.push_str(&page.content);
        bounds.push(crate::types::PageBoundary {
            page_number: page.page_number,
            byte_start: start,
            byte_end: text.len(),
        });
        text.push_str("\n\n");
    }
    let mut gate = evaluate_per_page_ocr(&text, Some(&bounds), Some(8), &thresholds);
    let shape_pages = gate.failing_pages.clone();
    let plausibility = super::super::plausibility::scan_text_plausibility(&text, Some(&bounds), Some(8), &thresholds);
    apply_flagged_pages(&mut gate, &plausibility.implausible, true, Some(8));
    let report = serde_json::json!({"input":"saved actual public native-control page content",
        "page8_clean":clean,"page8_structural_fallback":structural.fallback,"page8_words":structural.stats.word_count,
        "shape_failing_pages":shape_pages,"implausible_pages":plausibility.implausible,
        "selected_pages":gate.failing_pages,"whole_document_failure":gate.whole_doc_failure});
    std::fs::write(
        root.join("native-route-recomputed.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    assert!(clean);
    assert!(!structural.fallback);
    assert_eq!(structural.stats.word_count, 115);
    assert_eq!(shape_pages, [1, 2, 3, 4]);
    assert_eq!(plausibility.implausible, [8]);
    assert_eq!(gate.failing_pages, [1, 2, 3, 4, 8]);
    assert!(!gate.whole_doc_failure);
}

#[test]
fn should_keep_plain_ocr_only_words_even_without_rich_payload() {
    preserved_payload_case(ElementKind::Paragraph, true, false);
}

#[test]
fn should_keep_ocr_annotations_when_native_text_covers_every_ocr_token() {
    preserved_payload_case(ElementKind::Paragraph, false, true);
}

#[cfg(all(feature = "ocr", feature = "bundle-tessdata-eng"))]
#[tokio::test]
#[ignore = "requires public recipe; captures actual pre-render internal payload for preservation qualification"]
async fn should_capture_the_actual_public_recipe_page8_internal_payload() {
    use crate::plugins::InternalDocumentExtractor;
    let root = std::path::PathBuf::from(std::env::var_os("XBERG_2439_RECIPE_ROOT").expect("public recipe root"));
    let metrics: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("candidate-metrics.json")).unwrap()).unwrap();
    let mut config: crate::core::config::ExtractionConfig = serde_json::from_value(metrics["config"].clone()).unwrap();
    let tessdata = tempfile::tempdir().unwrap();
    std::fs::write(
        tessdata.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().unwrap(),
    )
    .unwrap();
    config.ocr.as_mut().unwrap().tessdata_path = Some(tessdata.path().to_path_buf());
    let bytes = std::fs::read(root.join("generated/scan_first_half.pdf")).unwrap();
    let _ = crate::extract(
        crate::ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
        &crate::ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await
    .unwrap();
    let internal = crate::extractors::pdf::PdfExtractor::new()
        .extract_content(&bytes, "application/pdf", &config)
        .await
        .unwrap();
    std::fs::write(
        root.join("candidate-internal.json"),
        serde_json::to_vec_pretty(&internal).unwrap(),
    )
    .unwrap();
    let page: Vec<_> = internal
        .elements
        .iter()
        .filter(|element| element.page == Some(8))
        .collect();
    assert!(!page.is_empty());
    std::fs::write(
        root.join("candidate-page8-elements.json"),
        serde_json::to_vec_pretty(&page).unwrap(),
    )
    .unwrap();
}

#[test]
fn should_allow_only_finite_internal_measured_typography_as_plain_ocr_payload() {
    use super::super::document::plain_ocr_page_can_yield_to_native;
    let mut document = InternalDocument::new("pdf");
    document.push_element(InternalElement::text(
        ElementKind::Paragraph,
        "Harbour committee minutes",
        0,
    ));
    assert!(plain_ocr_page_can_yield_to_native(
        &document,
        "Harbour committee minutes"
    ));
    document.elements[0].set_measured_font_size(13.0);
    assert!(plain_ocr_page_can_yield_to_native(
        &document,
        "Harbour committee minutes"
    ));
    document.elements[0]
        .attributes
        .as_mut()
        .unwrap()
        .insert("semantic-role".into(), "author".into());
    assert!(!plain_ocr_page_can_yield_to_native(
        &document,
        "Harbour committee minutes"
    ));
}

fn coverage_fixture() -> (String, String, super::super::document::PageLineBoxes) {
    let layer = super::MINUTES.join("\n");
    let read = super::MINUTES[..7].join("\n");
    let boxes = super::page_boxes(
        &super::MINUTES,
        &super::lines_at_rows(&read, &(0..7).collect::<Vec<_>>()),
    );
    (layer, read, boxes)
}

#[test]
fn should_certify_only_literal_owned_complete_row_inventories() {
    use super::super::document::native_page_covers_ocr;
    let (layer, read, boxes) = coverage_fixture();
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    assert!(native_page_covers_ocr(&layer, &read, &thresholds, Some(&boxes)));
    assert!(!native_page_covers_ocr(&layer, &read, &thresholds, None));
    let mut unowned = boxes.clone();
    unowned.ocr[3].bbox = super::row_box(30);
    assert!(!native_page_covers_ocr(&layer, &read, &thresholds, Some(&unowned)));
    let mut incomplete = boxes.clone();
    incomplete.layer.pop();
    assert!(!native_page_covers_ocr(&layer, &read, &thresholds, Some(&incomplete)));
    let mut unclaimed = boxes.clone();
    unclaimed.ocr.pop();
    assert!(!native_page_covers_ocr(&layer, &read, &thresholds, Some(&unclaimed)));
    let changed = read.replace("Total 100", "Total -100");
    let changed_boxes = super::page_boxes(
        &super::MINUTES,
        &super::lines_at_rows(&changed, &(0..7).collect::<Vec<_>>()),
    );
    assert!(!native_page_covers_ocr(
        &layer,
        &changed,
        &thresholds,
        Some(&changed_boxes)
    ));
}

#[test]
fn should_reject_ambiguous_literal_occurrences_even_with_distinct_geometry() {
    use super::super::document::native_page_covers_ocr;
    let rows = super::MINUTES[..7]
        .iter()
        .chain(super::MINUTES[..7].iter())
        .copied()
        .collect::<Vec<_>>();
    let layer = rows.join("\n");
    let read = super::MINUTES[..7].join("\n");
    let boxes = super::page_boxes(&rows, &super::lines_at_rows(&read, &(7..14).collect::<Vec<_>>()));
    assert!(!native_page_covers_ocr(
        &layer,
        &read,
        &crate::core::config::OcrQualityThresholds::default(),
        Some(&boxes)
    ));
}

#[test]
fn should_reject_ambiguous_geometry_despite_complete_literal_sequence_coverage() {
    use super::super::document::native_page_covers_ocr;
    let mut rows = super::MINUTES.to_vec();
    rows.push(super::MINUTES[0]);
    let text = rows.join("\n");
    let mut boxes = super::page_boxes(&rows, &super::lines_at_rows(&text, &(0..9).collect::<Vec<_>>()));
    boxes.layer[8].bbox = super::row_box(0);
    assert!(!native_page_covers_ocr(
        &text,
        &text,
        &crate::core::config::OcrQualityThresholds::default(),
        Some(&boxes)
    ));
}

#[test]
fn should_reject_count_preserving_final_text_edits_and_semantic_metadata() {
    use super::super::document::plain_ocr_page_can_yield_to_native;
    let mut document = InternalDocument::new("pdf");
    document.push_element(InternalElement::text(
        ElementKind::Paragraph,
        "Harbour committee minutes",
        0,
    ));
    assert!(plain_ocr_page_can_yield_to_native(
        &document,
        "Harbour committee minutes"
    ));
    document.elements[0].text = "Paid committee minutes".into();
    assert!(!plain_ocr_page_can_yield_to_native(
        &document,
        "Harbour committee minutes"
    ));
    document.elements[0].text = "Harbour committee minutes".into();
    document.metadata.title = Some("OCR-only title".into());
    assert!(!plain_ocr_page_can_yield_to_native(
        &document,
        "Harbour committee minutes"
    ));
}

#[test]
fn should_preserve_explicit_forced_ocr_and_element_requests() {
    use super::super::document::native_preservation_is_automatic;
    let mut config = crate::ExtractionConfig {
        ocr: Some(crate::OcrConfig {
            enabled: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(native_preservation_is_automatic(&config, 8));
    config.force_ocr = true;
    assert!(!native_preservation_is_automatic(&config, 8));
    config.force_ocr = false;
    config.force_ocr_pages = Some(vec![8]);
    assert!(!native_preservation_is_automatic(&config, 8));
    assert!(native_preservation_is_automatic(&config, 9));
    config.force_ocr_pages = None;
    config.ocr.as_mut().unwrap().element_config = Some(Default::default());
    assert!(!native_preservation_is_automatic(&config, 8));
}

#[test]
fn should_reject_sparse_native_unmapped_markers_without_stripping_ocr_payload() {
    use super::super::document::{
        layer_lines_to_put_first, native_page_covers_ocr, plain_ocr_page_can_yield_to_native, text_with_layer_first,
    };
    let mut native_rows = super::MINUTES.to_vec();
    native_rows.push("\u{fffd}");
    let layer = native_rows.join("\n");
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    assert!(
        super::super::scoring::native_layer_is_clean(&layer, &thresholds),
        "sparse marker is below legacy garble thresholds"
    );
    let mut checked = 0;
    for marker_read in [false, true] {
        let mut read_rows = super::MINUTES[..7].to_vec();
        let mut positions = (0..7).collect::<Vec<_>>();
        if marker_read {
            read_rows.push("\u{fffd}");
            positions.push(8);
        }
        let read = read_rows.join("\n");
        let boxes = super::page_boxes(&native_rows, &super::lines_at_rows(&read, &positions));
        let mut document = InternalDocument::new("pdf");
        document.push_element(InternalElement::text(ElementKind::Paragraph, &read, 0));
        assert!(plain_ocr_page_can_yield_to_native(&document, &read));
        assert!(!native_page_covers_ocr(&layer, &read, &thresholds, Some(&boxes)));
        let first = layer_lines_to_put_first(&layer, &read, false, &thresholds, Some(&boxes)).unwrap();
        let merged = text_with_layer_first(&read, &first);
        assert!(merged.ends_with(&read));
        if marker_read {
            assert!(merged.ends_with('\u{fffd}'));
        }
        checked += 1;
    }
    assert_eq!(checked, 2);
}

type LexicalCopyCase = (super::super::document::LayerFirst, InternalDocument);

fn lexical_copy_case(native: &str, read: &str) -> LexicalCopyCase {
    use super::super::document::layer_lines_to_put_first;
    let mut layer = super::MINUTES.to_vec();
    layer[3] = native;
    let mut ocr = layer.clone();
    ocr[3] = read;
    let text = ocr.join("\n");
    let boxes = super::page_boxes(&layer, &super::lines_at_rows(&text, &(0..8).collect::<Vec<_>>()));
    let first = layer_lines_to_put_first(
        &layer.join("\n"),
        &text,
        false,
        &crate::core::config::OcrQualityThresholds::default(),
        Some(&boxes),
    )
    .unwrap();
    let mut document = InternalDocument::new("pdf");
    for (index, line) in ocr.iter().enumerate() {
        let mut element = InternalElement::text(ElementKind::Paragraph, *line, 0);
        element.page = Some(1);
        let bbox = super::row_box(index);
        element.bbox = Some(crate::types::BoundingBox {
            x0: f64::from(bbox.left),
            y0: f64::from(bbox.top),
            x1: f64::from(bbox.right),
            y1: f64::from(bbox.bottom),
        });
        if index == 3 {
            element.annotations.push(TextAnnotation {
                start: 0,
                end: line.len() as u32,
                kind: AnnotationKind::Bold,
            });
            element.annotations.push(TextAnnotation {
                start: 0,
                end: line.len() as u32,
                kind: AnnotationKind::Link {
                    url: "https://example.invalid/status".into(),
                    title: None,
                },
            });
            element.anchor = Some("ocr-alternative".into());
        }
        document.push_element(element);
    }
    document.relationships.push(crate::types::internal::Relationship {
        source: 3,
        target: crate::types::internal::RelationshipTarget::Index(0),
        kind: crate::types::internal::RelationshipKind::InternalLink,
    });
    (first, document)
}

#[test]
fn should_not_claim_a_same_length_lexical_alternative_is_a_false_ocr_copy() {
    for (native, read) in [("Status pending", "Status approved"), ("Total 100", "Total paid")] {
        let (first, document) = lexical_copy_case(native, read);
        assert_eq!(first.lines, native);
        let text = document
            .elements
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            super::super::document::text_with_layer_first(&text, &first),
            format!("{native}\n\n{text}")
        );
    }
}

#[test]
fn should_keep_lexical_alternative_geometry_annotations_and_source_order() {
    for (native, read) in [("Status pending", "Status approved"), ("Total 100", "Total paid")] {
        let (first, mut document) = lexical_copy_case(native, read);
        let original = document.elements.clone();
        put_layer_first_in_document(&mut document, Some(1), &first);
        assert_eq!(document.elements[0].text, native);
        assert_eq!(document.relationships.len(), 1);
        assert_eq!(document.relationships[0].source, 4);
        assert!(matches!(
            document.relationships[0].target,
            crate::types::internal::RelationshipTarget::Index(1)
        ));
        assert_eq!(
            document.relationships[0].kind,
            crate::types::internal::RelationshipKind::InternalLink
        );
        assert_eq!(
            document.elements[1..],
            original,
            "retain {read:?} and all original payload"
        );
    }
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[test]
fn type0_retention_counts_meaningful_literal_occurrences_and_requires_markers() {
    use super::super::document::native_survives_type0_ink_loss;
    let native = "budget budget budget design design archive reader source \u{FFFD}";
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    assert!(native_survives_type0_ink_loss(
        native,
        "budget budget budget budget budget",
        &thresholds
    ));
    assert!(!native_survives_type0_ink_loss(
        native,
        "budget design archive reader",
        &thresholds
    ));
    assert!(!native_survives_type0_ink_loss(
        native,
        "budget budget budget design design archive reader source",
        &thresholds
    ));
    assert!(!native_survives_type0_ink_loss(
        "budget budget budget design archive reader",
        "wrong",
        &thresholds
    ));
    assert!(!native_survives_type0_ink_loss(
        "\u{FFFD} \u{FFFD} \u{FFFD}",
        "wrong",
        &thresholds
    ));
}

#[test]
fn a_layer_no_read_confirms_comes_first_only_with_raster_proof() {
    use super::super::document::layer_lines_to_put_first_with_proof;
    use crate::core::config::OcrQualityThresholds;

    let layer = "ប០ប";
    let read = "VOU\n\nThe office keeps the current monthly service records.";
    let thresholds = OcrQualityThresholds::default();
    assert!(layer_lines_to_put_first_with_proof(layer, read, false, false, &thresholds, None).is_none());
    let first = layer_lines_to_put_first_with_proof(layer, read, false, true, &thresholds, None).unwrap();
    assert_eq!(first.lines, layer);
    assert!(
        layer_lines_to_put_first_with_proof(layer, "ប០ប\n\nVOU", false, true, &thresholds, None).is_none(),
        "a row the read already holds is not restored"
    );
}
