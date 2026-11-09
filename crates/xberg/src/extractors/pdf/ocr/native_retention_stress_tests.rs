#![cfg(feature = "pdf")]

use super::super::document::{LayerFirst, LineAt, put_layer_first_in_document};
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use crate::extractors::pdf::unmapped_glyphs::{LOST_SYMBOL_GLYPH_MARKER, is_lost_glyph};
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
        crate::extractors::pdf::ocr::document::OcrPayload::Plain,
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
    let first = layer_lines_to_put_first(
        &layer,
        &text,
        false,
        &OcrQualityThresholds::default(),
        None,
        crate::extractors::pdf::ocr::document::OcrPayload::Plain,
    )
    .unwrap();
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
        let first = layer_lines_to_put_first(
            &layer,
            &read,
            false,
            &thresholds,
            Some(&boxes),
            crate::extractors::pdf::ocr::document::OcrPayload::Plain,
        )
        .unwrap();
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
        crate::extractors::pdf::ocr::document::OcrPayload::Plain,
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

/// A readable page that lost six glyphs: the lost-glyph markers sit between its words.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn readable_page_with_lost_glyphs() -> String {
    let words = "the committee reviewed the annual budget and approved the design of the new archive reader \
                 for every branch library after the long public meeting about the shared reading room";
    let mut page = String::new();
    for (at, word) in words.split(' ').enumerate() {
        if at % 3 == 0 && at < 18 {
            page.push('\u{FFFD}');
            page.push(' ');
        }
        page.push_str(word);
        page.push(' ');
    }
    assert_eq!(page.matches('\u{FFFD}').count(), 6, "the fixture holds six markers");
    page
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn one_page(page: &str) -> Vec<crate::types::PageBoundary> {
    vec![crate::types::PageBoundary {
        byte_start: 0,
        byte_end: page.len(),
        page_number: 1,
    }]
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_marked_page_still_routes_to_ocr_but_keeps_its_readable_layer() {
    use super::super::document::clean_native_page_layers;
    use super::super::scoring::evaluate_native_text_for_ocr;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let page = readable_page_with_lost_glyphs();
    assert!(
        evaluate_native_text_for_ocr(&page, Some(1), &thresholds).fallback,
        "control: the routing gate still counts the markers"
    );
    let layers = clean_native_page_layers(&page, &one_page(&page), None, &thresholds);
    assert!(layers.contains_key(&1), "the readable words of the page stay a layer");
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn ocr_that_drops_the_readable_words_of_a_marked_page_is_not_accepted() {
    use super::super::document::accepted_ocr_page_replacements;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let read: ahash::AHashMap<u32, String> = [(1, "two stray words only".to_string())].into_iter().collect();
    let page = readable_page_with_lost_glyphs();
    let accepted = accepted_ocr_page_replacements(&page, &one_page(&page), &read, &thresholds, true);
    assert!(accepted.is_empty(), "OCR keeps under half of the readable words");
    let all_lost = "\u{FFFD} ".repeat(20);
    let accepted = accepted_ocr_page_replacements(&all_lost, &one_page(&all_lost), &read, &thresholds, true);
    assert!(
        accepted.contains_key(&1),
        "control: a page with no readable word keeps no protection"
    );
}

const ROW_WORDS: [&str; 20] = [
    "committee",
    "budget",
    "archive",
    "library",
    "reader",
    "harbour",
    "treasurer",
    "notice",
    "pier",
    "repairs",
    "minutes",
    "hours",
    "spring",
    "public",
    "design",
    "branch",
    "reading",
    "room",
    "annual",
    "meeting",
];

/// Forty distinct readable rows. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn forty_rows() -> Vec<String> {
    (0..40)
        .map(|n| {
            format!(
                "section {n} the {} {} approved the {} {} after the {}",
                ROW_WORDS[n % 20],
                ROW_WORDS[(n * 7 + 3) % 20],
                ROW_WORDS[(n * 3 + 1) % 20],
                ROW_WORDS[(n * 11 + 5) % 20],
                ROW_WORDS[(n * 13 + 2) % 20],
            )
        })
        .collect()
}

/// The ways a page loses glyphs, each on every fourth row: the native rows with markers, and the
/// rows as OCR reads them whole. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn marked_cases() -> Vec<(&'static str, Vec<String>, Vec<String>)> {
    type Loss = fn(usize, &str) -> (String, String);
    let cases: [(&str, Loss); 5] = [
        ("between, OCR reads no glyph", |_, row| {
            (row.replacen(" approved", " \u{FFFD} approved", 1), row.to_string())
        }),
        ("between, OCR reads the glyph", |_, row| {
            (
                row.replacen(" approved", " \u{FFFD} approved", 1),
                row.replacen(" approved", " - approved", 1),
            )
        }),
        ("inside a word", |_, row| {
            (row.replacen("approved", "appr\u{FFFD}ved", 1), row.to_string())
        }),
        ("inside a number", |n, row| {
            let from = format!("section {n} ");
            (
                row.replacen(&from, &format!("section {n}\u{FFFD}5 "), 1),
                row.replacen(&from, &format!("section {n}35 "), 1),
            )
        }),
        ("a whole word", |_, row| {
            (row.replacen("approved", &"\u{FFFD}".repeat(8), 1), row.to_string())
        }),
    ];
    cases
        .into_iter()
        .map(|(name, loss)| {
            let (native, read) = forty_rows()
                .iter()
                .enumerate()
                .map(|(n, row)| {
                    if n % 4 == 0 {
                        loss(n, row)
                    } else {
                        (row.clone(), row.clone())
                    }
                })
                .unzip();
            (name, native, read)
        })
        .collect()
}

/// The stored layer of a one-page native text, which keeps its markers. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn stored_layer(native: &str) -> String {
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let layers = super::super::document::clean_native_page_layers(native, &one_page(native), None, &thresholds);
    let layer = layers
        .get(&1)
        .expect("the readable words of the marked page stay a layer")
        .clone();
    assert_eq!(layer, native.trim(), "a stored layer keeps its markers");
    layer
}

/// The page the caller gets after the rows the layer owns are put first: the production steps of
/// the mixed route, from the native text of the page to the merged text. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn merged_page(native: &str, read: &str, boxes: Option<&super::super::document::PageLineBoxes>) -> String {
    use super::super::document::{layer_lines_to_put_first, text_with_layer_first};
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    layer_lines_to_put_first(
        &stored_layer(native),
        read,
        false,
        &thresholds,
        boxes,
        crate::extractors::pdf::ocr::document::OcrPayload::Plain,
    )
    .map_or_else(|| read.to_string(), |first| text_with_layer_first(read, &first))
}

/// The line boxes of a page whose native rows and OCR rows lie on the same baselines. Each native
/// row with a marker comes as two segments that break before its first marker, as a text layer
/// can give it. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn row_boxes(native: &[String], read: &[String]) -> super::super::document::PageLineBoxes {
    let read: Vec<(String, usize)> = read.iter().map(|row| (row.clone(), 1)).collect();
    spanning_row_boxes(native, &read)
}

/// As `row_boxes`, with each OCR row given as its text and the number of native rows its box
/// spans, from the row after the one before it. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn spanning_row_boxes(native: &[String], read: &[(String, usize)]) -> super::super::document::PageLineBoxes {
    use crate::pdf::hierarchy::SegmentData;
    use crate::pdf::structure::types::{PdfLine, PdfParagraph};
    let tall_segment = |text: &str, row: usize, x: f32, width: f32, rows: usize| SegmentData {
        text: text.to_string(),
        x,
        y: row as f32 * 12.0,
        width,
        height: rows as f32 * 12.0 - 2.0,
        font_size: 10.0,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y: row as f32 * 12.0,
        rotation_degrees: 0.0,
        assigned_role: None,
    };
    let segment = |text: &str, row: usize, x: f32, width: f32| tall_segment(text, row, x, width, 1);
    let native: Vec<SegmentData> = native
        .iter()
        .enumerate()
        .flat_map(|(row, text)| match text.find(is_lost_glyph) {
            Some(at) if at > 0 => vec![
                segment(&text[..at], row, 0.0, 250.0),
                segment(&text[at..], row, 250.0, 250.0),
            ],
            _ => vec![segment(text, row, 0.0, 500.0)],
        })
        .collect();
    let mut next_row = 0;
    let lines = read
        .iter()
        .map(|(text, rows)| {
            let row = next_row;
            next_row += rows;
            (text, row, *rows)
        })
        .map(|(text, row, rows)| PdfLine {
            segments: vec![tall_segment(text, row, 0.0, 500.0, rows)],
            native_scripts: Vec::new(),
            baseline_y: row as f32 * 12.0,
            dominant_font_size: 10.0,
            is_bold: false,
            is_monospace: false,
        })
        .collect();
    let paragraph = PdfParagraph {
        text: String::new(),
        lines,
        dominant_font_size: 10.0,
        heading_level: None,
        is_bold: false,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: None,
        word_count: 0,
    };
    let boxes =
        super::super::document::page_line_boxes(&native, &[paragraph], (1.0, 1.0)).expect("both sides have rows");
    assert!(
        boxes.layer.iter().all(|row| !row.key.contains(is_lost_glyph)),
        "a native box is keyed on its readable text"
    );
    boxes
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn marked_rows_that_ocr_read_whole_are_not_printed_twice() {
    for (name, native, read) in marked_cases() {
        assert_eq!(
            native.iter().filter(|row| row.contains('\u{FFFD}')).count(),
            10,
            "{name}"
        );
        let read = read.join("\n");
        assert_eq!(merged_page(&native.join("\n"), &read, None), read, "{name}");
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn marked_rows_that_ocr_read_whole_are_not_printed_twice_with_row_geometry() {
    for (name, native, read) in marked_cases() {
        let boxes = row_boxes(&native, &read);
        let read = read.join("\n");
        assert_eq!(merged_page(&native.join("\n"), &read, Some(&boxes)), read, "{name}");
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_marked_row_is_judged_at_its_own_ocr_row_not_at_a_row_elsewhere() {
    let mut native = forty_rows();
    native[3] = "total 1\u{FFFD}0 for the year".to_string();
    native[9] = "total 100 for the year".to_string();
    let mut read = native.clone();
    read[3] = "total 1oo for the year".to_string();
    let boxes = row_boxes(&native, &read);
    let read = read.join("\n");
    assert_eq!(
        merged_page(&native.join("\n"), &read, Some(&boxes)),
        format!("total 1 0 for the year\n\n{read}"),
        "the OCR row at the marked row misreads it, so the row is put first"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_row_without_a_native_box_of_its_text_comes_from_the_ocr_row_that_holds_it_whole() {
    let native = forty_rows();
    let mut segments = native.clone();
    segments[3] = format!("{} and more", segments[3]);
    let boxes = row_boxes(&segments, &native);
    let read = native.join("\n");
    assert_eq!(merged_page(&read, &read, Some(&boxes)), read);
    let mut misread = native.clone();
    misread[3] = misread[3].replacen(" the ", " then ", 1);
    let boxes = row_boxes(&segments, &misread);
    let misread = misread.join("\n");
    assert_eq!(
        merged_page(&read, &misread, Some(&boxes)),
        format!("{}\n\n{misread}", native[3]),
        "a row that no OCR row holds whole is put first; the OCR row read a word the native lacks, so it stays"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn an_unplaced_native_row_takes_the_ocr_row_that_misreads_it() {
    let (native, read) = measure_rows();
    let mut segments = native.clone();
    segments[3] = format!("{} and more", segments[3]);
    let boxes = row_boxes(&segments, &read);
    let mut kept = read.clone();
    kept.remove(3);
    assert_eq!(
        merged_page(&native.join("\n"), &read.join("\n"), Some(&boxes)),
        format!("{}\n\n{}", native[3], kept.join("\n")),
        "a row without a native box is put first, and the OCR row that misreads it is not printed"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn native_rows_that_ocr_reads_as_one_line_are_not_put_first() {
    let native = forty_rows();
    let mut read: Vec<(String, usize)> = native.iter().map(|row| (row.clone(), 1)).collect();
    let joined = format!("{} {}", native[5], native[6]);
    read.splice(5..7, [(joined, 2)]);
    let boxes = spanning_row_boxes(&native, &read);
    let read: Vec<String> = read.into_iter().map(|(row, _)| row).collect();
    let read = read.join("\n");
    assert_eq!(merged_page(&native.join("\n"), &read, Some(&boxes)), read);
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn an_ocr_row_covers_each_equal_native_row_once() {
    let mut native = forty_rows();
    native[20] = native[2].clone();
    let mut segments = native.clone();
    segments[20] = format!("{} and more", segments[20]);
    let mut read = native.clone();
    read.remove(20);
    let mut spans: Vec<(String, usize)> = read.iter().map(|row| (row.clone(), 1)).collect();
    spans[19].1 = 2;
    let boxes = spanning_row_boxes(&segments, &spans);
    let read = read.join("\n");
    assert_eq!(
        merged_page(&native.join("\n"), &read, Some(&boxes)),
        format!("{}\n\n{read}", native[2]),
        "the second equal row finds the words of the first taken"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_marked_row_that_ocr_reads_with_another_word_is_put_first() {
    for (name, native, read) in marked_cases() {
        let mut read = read;
        read[8] = read[8].replacen(" the ", " then ", 1);
        let boxes = row_boxes(&native, &read);
        let read = read.join("\n");
        for boxes in [None, Some(&boxes)] {
            let merged = merged_page(&native.join("\n"), &read, boxes);
            assert!(merged.ends_with(&format!("\n\n{read}")), "{name}: {merged}");
            assert_eq!(
                merged.lines().count(),
                42,
                "row 8 is put first once, the OCR row stays: {name}: {merged}"
            );
            assert!(!merged.contains('\u{FFFD}'), "{name}: {merged}");
        }
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_marked_row_the_ocr_read_lacks_is_put_first_once_and_counts_only_its_own_markers() {
    use super::super::document::layer_lines_to_put_first;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let (_, native, read) = marked_cases().swap_remove(2);
    let mut read = read;
    read.remove(4);
    let read = read.join("\n");
    let layer = stored_layer(&native.join("\n"));
    let first = layer_lines_to_put_first(
        &layer,
        &read,
        false,
        &thresholds,
        None,
        crate::extractors::pdf::ocr::document::OcrPayload::Plain,
    )
    .expect("row 4 is missing");
    assert_eq!(first.lines, forty_rows()[4].replacen("approved", "appr ved", 1));
    assert_eq!(
        first.lost_glyphs, 1,
        "the nine marked rows that OCR read are not counted"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_row_of_markers_alone_is_not_put_first() {
    use super::super::document::layer_lines_to_put_first;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let clean = forty_rows();
    let native = format!("{}\n\u{FFFD}\u{FFFD}", clean.join("\n"));
    let read = clean[..39].join("\n");
    let first = layer_lines_to_put_first(
        &stored_layer(&native),
        &read,
        false,
        &thresholds,
        None,
        crate::extractors::pdf::ocr::document::OcrPayload::Plain,
    )
    .expect("row 39 is missing");
    assert_eq!(first.lines, clean[39]);
    assert_eq!(first.lost_glyphs, 0);
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn an_equal_row_takes_its_ocr_row_before_a_marked_row_does() {
    use super::super::document::layer_lines_to_put_first;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let clean = forty_rows();
    let marked = clean[0].replacen("approved", "appr\u{FFFD}ved", 1);
    let native = format!("{marked}\n{}", clean.join("\n"));
    let read = clean.join("\n");
    let first = layer_lines_to_put_first(
        &stored_layer(&native),
        &read,
        false,
        &thresholds,
        None,
        crate::extractors::pdf::ocr::document::OcrPayload::Plain,
    )
    .expect("one of the two rows 0 is missing");
    assert_eq!(first.lines, clean[0].replacen("approved", "appr ved", 1));
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_marker_inside_a_word_does_not_fuse_the_word_the_ocr_confirms() {
    use super::super::document::ocr_confirms_layer;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let split = "caf\u{FFFD}e ba\u{FFFD}nk li\u{FFFD}brary pa\u{FFFD}rk sc\u{FFFD}hool ma\u{FFFD}rket";
    let between = "caf \u{FFFD} e ba \u{FFFD} nk li \u{FFFD} brary pa \u{FFFD} rk sc \u{FFFD} hool ma \u{FFFD} rket";
    let whole = "cafe bank library park school market";
    assert!(
        !ocr_confirms_layer(split, whole, &thresholds),
        "caf<m>e is not the word cafe"
    );
    assert!(
        !ocr_confirms_layer(between, whole, &thresholds),
        "a marker between letters is a hole too"
    );
    let numbers = "1\u{FFFD}2 3\u{FFFD}4 5\u{FFFD}6 7\u{FFFD}8";
    assert!(
        !ocr_confirms_layer(numbers, "12 34 56 78", &thresholds),
        "1<m>2 is not the number 12"
    );
    let control = "the \u{FFFD} committee reviewed the \u{FFFD} annual budget and approved the \u{FFFD} design of the \
                   \u{FFFD} new archive reader for every \u{FFFD} branch library";
    let read = "the committee reviewed the annual budget and approved the design of the new archive reader for every \
                branch library";
    assert!(
        ocr_confirms_layer(control, read, &thresholds),
        "control: markers between words confirm"
    );
}

/// A page whose first twenty rows read and whose other half is lost glyphs, one for each letter or
/// digit of the readable half: the rows, the native text and its readable half. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn half_lost_page() -> (Vec<String>, String, String) {
    let clean = forty_rows();
    let readable = clean[..20].join("\n");
    let lost: String = "\u{FFFD}".repeat(readable.chars().filter(|c| c.is_alphanumeric()).count());
    let native = format!("{readable}\n{lost}");
    (clean, native, readable)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn half_a_page_lost_to_markers_is_judged_against_the_whole_page() {
    use super::super::document::accepted_ocr_page_replacements;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let (clean, native, readable) = half_lost_page();
    // The read holds 80% of the readable half: over the ratio against the layer, under it against
    // the whole page. ~keep
    let alnum = |text: &str| text.chars().filter(|c| c.is_alphanumeric()).count();
    let layer_alnum = alnum(&readable);
    let rows = (1..=20)
        .find(|&rows| alnum(&clean[..rows].join("\n")) * 10 >= layer_alnum * 8)
        .expect("twenty rows hold the layer");
    let ocr_reads_forty_percent = clean[..rows].join("\n");
    let read_alnum = alnum(&ocr_reads_forty_percent);
    assert!(
        read_alnum * 2 >= layer_alnum && read_alnum < layer_alnum,
        "fixture: the read holds over half of the layer and under half of the page"
    );
    let accepted = accepted_ocr_page_replacements(
        &native,
        &one_page(&native),
        &[(1, ocr_reads_forty_percent.clone())].into_iter().collect(),
        &thresholds,
        true,
    );
    assert!(
        accepted.is_empty(),
        "40% of the real page is not a replacement for 50% of it"
    );
    let ocr_reads_all = clean[..20].join("\n");
    let accepted = accepted_ocr_page_replacements(
        &native,
        &one_page(&native),
        &[(1, ocr_reads_all)].into_iter().collect(),
        &thresholds,
        true,
    );
    assert!(
        accepted.contains_key(&1),
        "control: OCR that reads the readable half is accepted"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn ocr_that_reads_sixty_percent_of_a_half_lost_page_is_accepted() {
    use super::super::document::accepted_ocr_page_replacements;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let (clean, native, readable) = half_lost_page();
    let alnum = |text: &str| text.chars().filter(|c| c.is_alphanumeric()).count();
    let page_alnum = alnum(&readable) * 2;
    let rows = (1..=40)
        .find(|&rows| alnum(&clean[..rows].join("\n")) * 10 >= page_alnum * 6)
        .expect("forty rows hold the page");
    let read = clean[..rows].join("\n");
    let share = alnum(&read) as f64 / page_alnum as f64;
    assert!(
        (0.6..0.65).contains(&share),
        "fixture: the read holds 60% of the page, well over the 0.5 line, not on it: {share}"
    );
    let accepted = accepted_ocr_page_replacements(
        &native,
        &one_page(&native),
        &[(1, read)].into_iter().collect(),
        &thresholds,
        true,
    );
    assert!(
        accepted.contains_key(&1),
        "OCR that reads 60% of the real page replaces a layer that holds 50% of it"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn only_the_marked_page_of_a_document_keeps_its_native_text() {
    use super::super::document::accepted_ocr_page_replacements;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let clean = forty_rows();
    let (_, marked, _) = marked_cases().swap_remove(0);
    let page_one = format!("{}\n", marked[..20].join("\n"));
    let page_two = format!("{}\n", clean[20..].join("\n"));
    let native = format!("{page_one}{page_two}");
    let boundaries = vec![
        crate::types::PageBoundary {
            byte_start: 0,
            byte_end: page_one.len(),
            page_number: 1,
        },
        crate::types::PageBoundary {
            byte_start: page_one.len(),
            byte_end: native.len(),
            page_number: 2,
        },
    ];
    let stray = "two stray words only".to_string();
    let read = [(1, stray.clone()), (2, stray)].into_iter().collect();
    let accepted = accepted_ocr_page_replacements(&native, &boundaries, &read, &thresholds, true);
    assert!(
        accepted.is_empty(),
        "both pages are readable, so OCR that keeps a few words replaces neither"
    );
    let page_two_lost = "\u{FFFD} ".repeat(60);
    let native = format!("{page_one}{page_two_lost}");
    let boundaries = vec![
        crate::types::PageBoundary {
            byte_start: 0,
            byte_end: page_one.len(),
            page_number: 1,
        },
        crate::types::PageBoundary {
            byte_start: page_one.len(),
            byte_end: native.len(),
            page_number: 2,
        },
    ];
    let accepted = accepted_ocr_page_replacements(&native, &boundaries, &read, &thresholds, true);
    assert_eq!(
        accepted.keys().copied().collect::<Vec<_>>(),
        vec![2],
        "only the page with nothing readable is replaced"
    );
}

/// Forty rows whose row 3 ends in a measure, and the same rows as OCR reads them: the measure's
/// digit as a quote. The OCR row keeps every word of the native row. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn measure_rows() -> (Vec<String>, Vec<String>) {
    let mut native = forty_rows();
    native[3] = format!("{} 12 cm3", native[3]);
    let mut read = native.clone();
    read[3] = format!("{} 12 cm\u{2019}", forty_rows()[3]);
    (native, read)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn an_ocr_row_that_misreads_a_native_row_put_first_is_not_printed() {
    let (native, read) = measure_rows();
    let boxes = row_boxes(&native, &read);
    let mut kept = read.clone();
    kept.remove(3);
    let read = read.join("\n");
    for boxes in [None, Some(&boxes)] {
        assert_eq!(
            merged_page(&native.join("\n"), &read, boxes),
            format!("{}\n\n{}", native[3], kept.join("\n")),
            "geometry {}",
            boxes.is_some()
        );
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn an_ocr_row_with_a_word_the_native_row_lacks_stays_beside_it() {
    let (native, mut read) = measure_rows();
    read[3] = format!("{} paid", read[3]);
    let boxes = row_boxes(&native, &read);
    let read = read.join("\n");
    for boxes in [None, Some(&boxes)] {
        assert_eq!(
            merged_page(&native.join("\n"), &read, boxes),
            format!("{}\n\n{read}", native[3]),
            "geometry {}",
            boxes.is_some()
        );
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn an_ocr_row_with_a_number_the_native_row_lacks_stays_beside_it() {
    let native = forty_rows();
    let mut read = native.clone();
    read[3] = read[3].replacen("section 3 ", "section 35 ", 1);
    let boxes = row_boxes(&native, &read);
    let read = read.join("\n");
    for boxes in [None, Some(&boxes)] {
        assert_eq!(
            merged_page(&native.join("\n"), &read, boxes),
            format!("{}\n\n{read}", native[3]),
            "geometry {}",
            boxes.is_some()
        );
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn an_ocr_row_that_drops_the_sign_of_a_number_stays_beside_it() {
    let mut native = forty_rows();
    native[3] = format!("{} total -100", native[3]);
    let mut read = native.clone();
    read[3] = format!("{} total 100", forty_rows()[3]);
    let boxes = row_boxes(&native, &read);
    let read = read.join("\n");
    for boxes in [None, Some(&boxes)] {
        assert_eq!(
            merged_page(&native.join("\n"), &read, boxes),
            format!("{}\n\n{read}", native[3]),
            "geometry {}",
            boxes.is_some()
        );
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn an_ocr_row_too_unlike_the_native_row_stays_beside_it() {
    let native = forty_rows();
    let mut read = native.clone();
    read[3] = read[3].replacen(" after the ", " ", 1);
    assert!(native[3].contains(" after the "), "row 3 loses two words");
    let boxes = row_boxes(&native, &read);
    let read = read.join("\n");
    for boxes in [None, Some(&boxes)] {
        assert_eq!(
            merged_page(&native.join("\n"), &read, boxes),
            format!("{}\n\n{read}", native[3]),
            "geometry {}",
            boxes.is_some()
        );
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_page_of_rich_ocr_payload_keeps_the_ocr_row_that_misreads_a_native_row() {
    use super::super::document::{OcrPayload, layer_lines_to_put_first, text_with_layer_first};
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let (native, read) = measure_rows();
    let boxes = row_boxes(&native, &read);
    let read = read.join("\n");
    let layer = stored_layer(&native.join("\n"));
    for boxes in [None, Some(&boxes)] {
        let first = layer_lines_to_put_first(&layer, &read, false, &thresholds, boxes, OcrPayload::Rich)
            .expect("row 3 is put first");
        assert_eq!(first.lines, native[3]);
        assert!(
            first.misread_ocr_rows.is_empty(),
            "no OCR row of a rich page is dropped"
        );
        assert_eq!(text_with_layer_first(&read, &first), format!("{}\n\n{read}", native[3]));
    }
}

/// The misread OCR row is a plain paragraph of a page that also holds a table: the row leaves
/// by the element that holds it, not by the payload of the page. The whole-document run route,
/// without a noise verdict, so the OCR text has to confirm the layer first. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn should_print_a_row_once_when_its_ocr_misread_is_a_paragraph_on_a_page_with_a_table() {
    use super::super::document::{PageLayers, put_layers_first_in_run};
    use crate::core::config::ExtractionConfig;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    let (native, read) = measure_rows();
    let table_row = "| Budget | 12 | 45 |";
    let mut doc = InternalDocument::new("pdf");
    for row in &read {
        let mut element = InternalElement::text(ElementKind::Paragraph, row.as_str(), 0);
        element.page = Some(1);
        doc.push_element(element);
    }
    let mut table = InternalElement::text(ElementKind::Table { table_index: 0 }, table_row, 0);
    table.page = Some(1);
    doc.push_element(table);
    let layers = ahash::AHashMap::from([(1, stored_layer(&native.join("\n")))]);
    let mut page_texts = vec![format!("{}\n{table_row}", read.join("\n"))];
    let mut text = page_texts[0].clone();
    put_layers_first_in_run(
        &mut text,
        &mut page_texts,
        Some(&mut doc),
        PageLayers {
            lines: &layers,
            boxes: &ahash::AHashMap::default(),
        },
        &[],
        0,
        &ExtractionConfig::default(),
    );
    let mut kept = read.clone();
    kept.remove(3);
    assert_eq!(
        text,
        format!("{}\n\n{}\n{table_row}", native[3], kept.join("\n")),
        "the row prints once, as the native read it; the table row stays"
    );
    assert_eq!(page_texts[0], text);
    let texts: Vec<&str> = doc.elements.iter().map(|element| element.text.as_str()).collect();
    let mut expected = vec![native[3].as_str()];
    expected.extend(kept.iter().map(String::as_str));
    expected.push(table_row);
    assert_eq!(texts, expected, "the misread paragraph leaves the page; the table element stays");
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_misread_ocr_row_leaves_the_structured_page_once_and_an_emptied_element_leaves_it() {
    use super::super::document::{LayerFirst, put_layer_first_in_document};
    use crate::types::internal::{
        ElementKind, InternalDocument, InternalElement, Relationship, RelationshipKind, RelationshipTarget,
    };

    let mut doc = InternalDocument::new("pdf");
    for text in ["total 12 cm\u{2019}", "alpha beta\ntotal 12 cm\u{2019}", "gamma"] {
        let mut element = InternalElement::text(ElementKind::Paragraph, text, 0);
        element.page = Some(1);
        doc.push_element(element);
    }
    doc.relationships.push(Relationship {
        source: 2,
        target: RelationshipTarget::Index(1),
        kind: RelationshipKind::InternalLink,
    });
    let first = LayerFirst {
        lines: "total 12 cm3".to_string(),
        lost_glyphs: 0,
        misread_ocr_rows: vec!["total 12 cm\u{2019}".to_string()],
    };
    put_layer_first_in_document(&mut doc, Some(1), &first);
    let texts: Vec<&str> = doc.elements.iter().map(|element| element.text.as_str()).collect();
    assert_eq!(
        texts,
        ["total 12 cm3", "alpha beta\ntotal 12 cm\u{2019}", "gamma"],
        "one native row takes one OCR row; the second equal row stays"
    );
    assert_eq!(doc.relationships.len(), 1);
    assert_eq!(
        doc.relationships[0].source, 2,
        "the source moves down one for the removed element, then up one for the layer"
    );
    assert!(
        matches!(doc.relationships[0].target, RelationshipTarget::Index(1)),
        "the target moves down one, then up one"
    );
    let text =
        super::super::document::text_with_layer_first("total 12 cm\u{2019}\nalpha beta\ntotal 12 cm\u{2019}", &first);
    assert_eq!(text, "total 12 cm3\n\nalpha beta\ntotal 12 cm\u{2019}");
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_whole_document_run_drops_the_misread_row_on_a_plain_page_only() {
    use super::super::document::{PageLayers, flat_ocr_page_document, put_layers_first_in_run};
    use super::super::scoring::OcrPageNoiseVerdict;
    use crate::core::config::ExtractionConfig;
    use crate::types::document_structure::{AnnotationKind, TextAnnotation};

    let verdict = OcrPageNoiseVerdict {
        page_index: 0,
        low_confidence: true,
        fragmented_noise: false,
        dictionary_noise: false,
        fragmented_word_ratio: 0.0,
        word_count: 4,
        mean_confidence: Some(0.2),
        dict_invalid_word_ratio: None,
        discarded: false,
    };
    let layers = ahash::AHashMap::from([(1, "total 12 cm3 of water".to_string())]);
    for rich in [false, true] {
        let mut page_texts = vec!["total 12 cm\u{2019} of water".to_string()];
        let mut text = page_texts[0].clone();
        let mut doc = flat_ocr_page_document(&text);
        doc.elements[0].page = Some(1);
        if rich {
            doc.elements[0].annotations.push(TextAnnotation {
                start: 0,
                end: 5,
                kind: AnnotationKind::Bold,
            });
        }
        put_layers_first_in_run(
            &mut text,
            &mut page_texts,
            Some(&mut doc),
            PageLayers {
                lines: &layers,
                boxes: &ahash::AHashMap::default(),
            },
            std::slice::from_ref(&verdict),
            0,
            &ExtractionConfig::default(),
        );
        let texts: Vec<&str> = doc.elements.iter().map(|element| element.text.as_str()).collect();
        if rich {
            assert_eq!(
                text, "total 12 cm3 of water\n\ntotal 12 cm\u{2019} of water",
                "a rich page keeps its OCR row"
            );
            assert_eq!(texts, ["total 12 cm3 of water", "total 12 cm\u{2019} of water"]);
        } else {
            assert_eq!(
                text, "total 12 cm3 of water",
                "a plain page prints the row once, as the native read it"
            );
            assert_eq!(texts, ["total 12 cm3 of water"]);
        }
        assert_eq!(page_texts[0], text);
    }
}

/// Forty rows whose row 0 begins with a lost glyph (`marker`, a checkbox before the text), and the
/// rows as OCR reads them, with `glyph` for what OCR made of the checkbox. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn checkbox_rows(marker: char, glyph: &str) -> (Vec<String>, Vec<String>) {
    let (mut native, mut read) = (forty_rows(), forty_rows());
    native[0] = format!("{marker}{}", native[0]);
    read[0] = format!("{glyph}{}", read[0]);
    (native, read)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_symbol_glyph_the_ocr_glued_to_a_word_puts_the_native_row_first_without_the_glued_row() {
    let (native, read) = checkbox_rows(LOST_SYMBOL_GLYPH_MARKER, "o");
    let boxes = row_boxes(&native, &read);
    let expected = format!("{}\n\n{}", forty_rows()[0], read[1..].join("\n"));
    for boxes in [None, Some(&boxes)] {
        assert_eq!(
            merged_page(&native.join("\n"), &read.join("\n"), boxes),
            expected,
            "geometry: {}",
            boxes.is_some()
        );
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_symbol_glyph_the_ocr_read_as_its_own_token_or_not_at_all_is_a_whole_read() {
    for glyph in ["o ", "", "☐ "] {
        let (native, read) = checkbox_rows(LOST_SYMBOL_GLYPH_MARKER, glyph);
        let boxes = row_boxes(&native, &read);
        let read = read.join("\n");
        for boxes in [None, Some(&boxes)] {
            assert_eq!(merged_page(&native.join("\n"), &read, boxes), read, "glyph {glyph:?}");
        }
    }
}

/// A lost glyph whose font is unknown may be a letter of the word, so the OCR read wins. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_lost_glyph_of_an_unknown_font_glued_by_the_ocr_is_a_whole_read() {
    let (native, read) = checkbox_rows('\u{FFFD}', "o");
    let boxes = row_boxes(&native, &read);
    let read = read.join("\n");
    for boxes in [None, Some(&boxes)] {
        assert_eq!(merged_page(&native.join("\n"), &read, boxes), read);
    }
}

/// The native text lost a ligature inside a word; OCR read the word whole, so the page is the OCR
/// read, whichever font the lost glyph had. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_word_with_a_lost_glyph_inside_comes_from_the_ocr_that_read_it_whole() {
    let mut native = forty_rows();
    native[0] = native[0].replacen("approved", "appr\u{FFFD}ved", 1);
    native[1] = native[1].replacen("approved", "appr\u{FFFF}ved", 1);
    let read = forty_rows();
    let boxes = row_boxes(&native, &read);
    let read = read.join("\n");
    for boxes in [None, Some(&boxes)] {
        assert_eq!(merged_page(&native.join("\n"), &read, boxes), read);
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn a_marked_page_whose_remainder_is_garbage_keeps_no_layer() {
    use super::super::document::{accepted_ocr_page_replacements, clean_native_page_layers};
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let page = "\u{FFFF} #%& \u{FFFD} @!* ~~ \u{FFFF}\u{FFFF} |||\n".repeat(30);
    assert!(
        !clean_native_page_layers(&page, &one_page(&page), None, &thresholds).contains_key(&1),
        "nothing readable remains to keep"
    );
    let read = [(1, "two stray words only".to_string())].into_iter().collect();
    let accepted = accepted_ocr_page_replacements(&page, &one_page(&page), &read, &thresholds, true);
    assert!(accepted.contains_key(&1), "the OCR text replaces the page");
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn the_marked_page_judgement_is_off_under_a_configured_pipeline() {
    use super::super::document::accepted_ocr_page_replacements;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let read: ahash::AHashMap<u32, String> = [(1, "two stray words only".to_string())].into_iter().collect();
    let page = readable_page_with_lost_glyphs();
    let accepted = accepted_ocr_page_replacements(&page, &one_page(&page), &read, &thresholds, false);
    assert!(
        accepted.contains_key(&1),
        "a configured pipeline judges the marked page as before: its markers are garble"
    );
    let clean = format!("{}\n", forty_rows()[..20].join("\n"));
    for judge in [false, true] {
        let accepted = accepted_ocr_page_replacements(&clean, &one_page(&clean), &read, &thresholds, judge);
        assert!(
            accepted.is_empty(),
            "control: a clean page keeps its text under either arm ({judge})"
        );
    }
}

/// A mixed-route config whose OCR runs the named fake backend alone, or as a configured pipeline. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn mixed_route_config(backend: &str, pipeline: bool) -> crate::core::config::ExtractionConfig {
    use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds};
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: backend.to_string(),
            pipeline: pipeline.then(|| OcrPipelineConfig {
                stages: vec![OcrPipelineStage {
                    backend: backend.to_string(),
                    priority: 100,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: None,
                    backend_options: None,
                }],
                quality_thresholds: OcrQualityThresholds {
                    pipeline_min_quality: 0.0,
                    ..Default::default()
                },
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// The mixed route judges a marked page on its readable text: a weak OCR read does not replace the
/// page under a single backend, and a pipeline the user configured replaces it as before. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn the_mixed_route_keeps_a_marked_page_unless_the_user_configured_a_pipeline() {
    use super::super::pipeline::extract_mixed_ocr_native;
    const BACKEND: &str = "marked-page-route-backend";
    const READ: &str = "two stray words only";
    crate::plugins::register_ocr_backend(std::sync::Arc::new(super::StageBackend {
        name: BACKEND,
        text: READ,
        noisy: false,
    }))
    .unwrap();
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let page = readable_page_with_lost_glyphs();
    let mut results = Vec::new();
    for pipeline in [false, true] {
        let config = mixed_route_config(BACKEND, pipeline);
        let run = extract_mixed_ocr_native(&page, &one_page(&page), &[1], &pdf, &config, None).await;
        results.push((pipeline, run));
    }
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();
    for (pipeline, run) in results {
        let (text, replaced, ..) = run.unwrap();
        if pipeline {
            assert!(
                replaced.get(&1).is_some_and(|text| text.contains(READ)),
                "a configured pipeline replaces the marked page: {text:?}"
            );
        } else {
            assert!(
                replaced.is_empty(),
                "a single backend keeps the marked page: {replaced:?}"
            );
            assert_eq!(text, page, "the native text of the kept page is unchanged");
        }
    }
}

/// The premise of the route test: under the `full` build the default config has an effective
/// pipeline (the paddle fallback) that the user did not configure, so the gate reads `pipeline`. ~keep
#[cfg(all(paddle_ocr, feature = "ocr"))]
#[test]
fn the_full_build_makes_a_pipeline_the_user_did_not_configure() {
    let config = crate::core::config::OcrConfig::default();
    assert!(config.pipeline.is_none());
    assert!(
        config.effective_pipeline().is_some(),
        "the default backend gets a paddle fallback stage"
    );
}
