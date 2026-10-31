//! A page whose OCR text is judged recognition noise keeps its readable text layer first.

use super::document::{clean_native_page_layers, layer_lines_absent_from_ocr};
use super::scoring::native_layer_is_clean;
use crate::core::config::OcrQualityThresholds;
use crate::types::PageBoundary;

#[test]
fn should_call_a_short_readable_title_a_clean_layer() {
    assert!(native_layer_is_clean("Meeting Notes", &OcrQualityThresholds::default()));
}

#[test]
fn should_not_call_empty_or_symbol_only_text_a_clean_layer() {
    let thresholds = OcrQualityThresholds::default();
    assert!(!native_layer_is_clean("   \n ", &thresholds));
    assert!(!native_layer_is_clean("-- | ~ ..", &thresholds));
}

#[test]
fn should_skip_layer_lines_the_ocr_text_already_holds() {
    let ocr = "intro\nMEETING  notes!\nbody";
    assert_eq!(layer_lines_absent_from_ocr("Meeting Notes", ocr), None);
    assert_eq!(
        layer_lines_absent_from_ocr("Meeting Notes\nBudget 2026", ocr).as_deref(),
        Some("Budget 2026")
    );
}

/// The letters of a layer word that occur in the OCR text only across a word boundary, or in a
/// longer word, are not evidence that the OCR read the word. ~keep
#[test]
fn should_keep_a_layer_word_that_the_ocr_text_holds_only_across_a_word_boundary() {
    assert_eq!(
        layer_lines_absent_from_ocr("Quillmarrow", "the quill marrow road").as_deref(),
        Some("Quillmarrow")
    );
    assert_eq!(
        layer_lines_absent_from_ocr("Quill", "the quillmarrow road").as_deref(),
        Some("Quill")
    );
    assert_eq!(
        layer_lines_absent_from_ocr("Quill Marrow", "the quill marrow road"),
        None
    );
    assert_eq!(
        layer_lines_absent_from_ocr("Plot 100", "plot 1OO").as_deref(),
        Some("Plot 100")
    );
}

#[test]
fn should_collect_layers_only_for_requested_pages_with_one_boundary() {
    let text = "Meeting Notes--|~..Agenda Items";
    let boundaries = [
        PageBoundary {
            byte_start: 0,
            byte_end: 13,
            page_number: 1,
        },
        PageBoundary {
            byte_start: 13,
            byte_end: 19,
            page_number: 2,
        },
        PageBoundary {
            byte_start: 19,
            byte_end: 31,
            page_number: 3,
        },
        PageBoundary {
            byte_start: 19,
            byte_end: 31,
            page_number: 3,
        },
    ];
    let thresholds = OcrQualityThresholds::default();
    let all = clean_native_page_layers(text, &boundaries, None, &thresholds);
    assert_eq!(all.get(&1).map(String::as_str), Some("Meeting Notes"));
    assert!(!all.contains_key(&2), "a symbol-only layer is not clean");
    assert!(!all.contains_key(&3), "a page with two boundaries is skipped");
    let only_two = clean_native_page_layers(
        text,
        &boundaries,
        Some(&std::collections::HashSet::from([2])),
        &thresholds,
    );
    assert!(only_two.is_empty(), "page 1 is not requested and page 2 is not clean");
}

#[test]
fn should_put_the_layer_before_the_structured_page_and_shift_relationships() {
    use super::document::{flat_ocr_page_document, insert_layer_into_document};
    use crate::types::internal::{Relationship, RelationshipKind, RelationshipTarget};

    let mut page = flat_ocr_page_document("first block\n\nsecond block");
    page.relationships.push(Relationship {
        source: 0,
        target: RelationshipTarget::Index(1),
        kind: RelationshipKind::InternalLink,
    });
    insert_layer_into_document(&mut page, None, "Meeting Notes");
    let texts: Vec<&str> = page.elements.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, ["Meeting Notes", "first block", "second block"]);
    assert_eq!(page.relationships[0].source, 1);
    assert!(matches!(page.relationships[0].target, RelationshipTarget::Index(2)));
}

#[test]
fn should_put_the_layer_first_on_the_noise_page_of_a_whole_document_run_only() {
    use super::document::{flat_ocr_page_document, put_layers_first_in_run};
    use super::scoring::OcrPageNoiseVerdict;
    use crate::types::internal::{Relationship, RelationshipKind, RelationshipTarget};

    let verdict = |page_index: usize, discarded: bool| OcrPageNoiseVerdict {
        page_index,
        low_confidence: true,
        fragmented_noise: false,
        dictionary_noise: false,
        fragmented_word_ratio: 0.0,
        word_count: 3,
        mean_confidence: Some(0.2),
        dict_invalid_word_ratio: None,
        discarded,
    };
    let mut page_texts = vec![
        "page one text".to_string(),
        "xq zv wj".to_string(),
        "page three text".to_string(),
    ];
    let mut text = page_texts.join("\n\n");
    let mut doc = flat_ocr_page_document("page one text");
    for (page, body) in [(2u32, "xq zv wj"), (3, "page three text")] {
        let mut part = flat_ocr_page_document(body);
        part.elements[0].page = Some(page);
        doc.elements.append(&mut part.elements);
    }
    doc.elements[0].page = Some(1);
    doc.relationships.push(Relationship {
        source: 2,
        target: RelationshipTarget::Index(0),
        kind: RelationshipKind::InternalLink,
    });
    let layers = ahash::AHashMap::from([
        (1, "Page One".to_string()),
        (2, "Meeting Notes".to_string()),
        (3, "Page Three".to_string()),
    ]);
    // Page 1 has no verdict, page 3's verdict was discarded: only page 2 gets its layer.
    put_layers_first_in_run(
        &mut text,
        &mut page_texts,
        Some(&mut doc),
        &layers,
        &[verdict(1, false), verdict(2, true)],
        0,
        None,
    );
    assert_eq!(page_texts[1], "Meeting Notes\n\nxq zv wj");
    assert_eq!(page_texts[0], "page one text");
    assert_eq!(page_texts[2], "page three text");
    assert_eq!(text, "page one text\n\nMeeting Notes\n\nxq zv wj\n\npage three text");
    let texts: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, ["page one text", "Meeting Notes", "xq zv wj", "page three text"]);
    assert_eq!(doc.elements[1].page, Some(2));
    assert_eq!(
        doc.relationships[0].source, 3,
        "a relationship past the insertion point shifts"
    );
    assert!(
        matches!(doc.relationships[0].target, RelationshipTarget::Index(0)),
        "one before it stays"
    );
}

/// The two existing text judgements (language plausibility and the recognition-noise ratio)
/// both abstain on a one-line layer, so neither can tell invented words from a real title.
/// The layer gate therefore cannot reuse them to reject a short invented layer. ~keep
#[test]
fn should_show_that_the_existing_judgements_abstain_on_a_one_line_layer() {
    use super::plausibility::{PlausibilityVerdict, evaluate_text_plausibility};
    use super::scoring::is_ocr_recognition_noise;

    let thresholds = OcrQualityThresholds::default();
    for line in ["Meeting Notes", "Qxzv Kjwq Zxcv Wqjx Vkzq Jqxz"] {
        assert_eq!(
            evaluate_text_plausibility(line, &thresholds),
            PlausibilityVerdict::NotEvaluated
        );
        assert!(!is_ocr_recognition_noise(line, &thresholds));
    }
}

/// An OCR backend that returns fixed text; a noisy one reports a low mean confidence.
#[cfg(all(feature = "pdf", feature = "ocr"))]
struct StageBackend {
    name: &'static str,
    text: &'static str,
    noisy: bool,
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[async_trait::async_trait]
impl crate::plugins::OcrBackend for StageBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(
        &self,
        _: &[u8],
        _: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        let mut metadata = crate::types::Metadata::default();
        if self.noisy {
            metadata
                .additional
                .insert("mean_text_conf".into(), serde_json::json!(18.0));
        }
        Ok(crate::types::ExtractedDocument {
            content: self.text.to_string(),
            metadata,
            ..Default::default()
        })
    }
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::Legibility { scale_max: 100.0 }
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
impl crate::plugins::Plugin for StageBackend {
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

/// A pipeline scores every stage on its OCR text. A text layer put before noisy OCR text must not
/// raise that stage above a better stage, so the winner is the one the base picks without any
/// layer, and only the chosen stage gets its layer. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_pick_the_same_pipeline_stage_with_and_without_a_text_layer() {
    use super::pipeline::{PageOcrHints, run_ocr_pipeline_for_page};
    use super::scoring::{compute_quality_score, pipeline_stage_score};
    use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage};
    use std::sync::Arc;

    const NOISY: &str = "layer-test-noisy-stage";
    const PLAIN: &str = "layer-test-plain-stage";
    const NOISY_TEXT: &str = "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z A B C D";
    const PLAIN_TEXT: &str = "Total due on receipt";
    const LAYER: &str = "The committee met on Tuesday to review the annual budget. Members agreed that the proposal \
        was sound and asked the treasurer to prepare the final report. The next meeting takes place in the spring.";

    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: NOISY,
        text: NOISY_TEXT,
        noisy: true,
    }))
    .unwrap();
    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: PLAIN,
        text: PLAIN_TEXT,
        noisy: false,
    }))
    .unwrap();

    let stage = |backend: &str, priority: u32| OcrPipelineStage {
        backend: backend.to_string(),
        priority,
        language: None,
        tesseract_config: None,
        paddle_ocr_config: None,
        paddle_ocr_settings: None,
        vlm_config: None,
        backend_options: None,
    };
    // The accept bar sits between the noisy stage scored alone and scored with the layer in front:
    // only a pipeline that scores the layer text accepts the noisy stage on the first try. ~keep
    let defaults = crate::core::config::OcrQualityThresholds::default();
    let noisy_conf = Some(0.18);
    let score_alone = pipeline_stage_score(compute_quality_score(NOISY_TEXT, &defaults), noisy_conf);
    let score_layered = pipeline_stage_score(
        compute_quality_score(&format!("{LAYER}\n\n{NOISY_TEXT}"), &defaults),
        noisy_conf,
    );
    let score_plain = pipeline_stage_score(compute_quality_score(PLAIN_TEXT, &defaults), None);
    let accept_at = (score_alone + score_layered) / 2.0;
    assert!(
        score_alone < accept_at && accept_at <= score_layered && accept_at <= score_plain,
        "the accept bar must separate the stages: alone {score_alone}, layered {score_layered}, plain {score_plain}"
    );
    let pipeline = OcrPipelineConfig {
        stages: vec![stage(NOISY, 120), stage(PLAIN, 100)],
        quality_thresholds: crate::core::config::OcrQualityThresholds {
            pipeline_min_quality: accept_at,
            ..Default::default()
        },
    };
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            pipeline: Some(pipeline.clone()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let images = [image::DynamicImage::new_rgb8(16, 16)];
    let hints = |layers: Option<ahash::AHashMap<u32, String>>| PageOcrHints {
        source_dpi: None,
        known_full_page_scan: false,
        single_block_pages: None,
        native_page_layers: layers.map(Arc::new),
    };
    let run = |pipeline: OcrPipelineConfig, hints: PageOcrHints| {
        let (config, images) = (&config, &images);
        async move {
            run_ocr_pipeline_for_page(
                None,
                Some(images),
                #[cfg(feature = "layout-detection")]
                None,
                config,
                &pipeline,
                None,
                0,
                true,
                None,
                Some(hints),
                0,
                None,
            )
            .await
            .map(|output| output.0)
            .expect("the pipeline must return a stage result")
        }
    };

    let layers = || Some(ahash::AHashMap::from([(1, LAYER.to_string())]));
    let noisy_only = OcrPipelineConfig {
        stages: vec![stage(NOISY, 120)],
        quality_thresholds: pipeline.quality_thresholds.clone(),
    };
    let noisy_with_layer = run(noisy_only.clone(), hints(layers())).await;
    let without_layer = run(pipeline.clone(), hints(None)).await;
    let with_layer = run(pipeline.clone(), hints(layers())).await;
    crate::plugins::unregister_ocr_backend(NOISY).unwrap();
    crate::plugins::unregister_ocr_backend(PLAIN).unwrap();

    assert!(
        noisy_with_layer.contains(LAYER) && noisy_with_layer.ends_with(NOISY_TEXT),
        "the noisy stage must carry the layer before its text, or this test proves nothing: {noisy_with_layer:?}"
    );
    assert_eq!(
        without_layer, PLAIN_TEXT,
        "without a layer the plain stage scores higher"
    );
    assert_eq!(with_layer, PLAIN_TEXT, "a layer must not change which stage wins");
}

/// Page route: a replacement is decided on the OCR text alone. A noise page whose OCR text keeps
/// under half of the letters of a substantial clean layer stays native, byte for byte, even
/// though the layer put before the OCR text would pass the same check. A short layer that the
/// check does not protect is replaced, with the layer first. Both the single-backend route and
/// the pipeline route are driven. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_decide_a_page_replacement_on_the_ocr_text_before_the_layer_goes_first() {
    use super::pipeline::extract_mixed_ocr_native;
    use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-page-route-backend";
    const NOISY: &str = "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z A B C D";
    const LONG_LAYER: &str = "The committee met on Tuesday to review the annual budget. Members agreed that the proposal \
        was sound and asked the treasurer to prepare the final report. The next meeting takes place in the spring.";
    const TITLE: &str = "Meeting Notes";

    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: BACKEND,
        text: NOISY,
        noisy: true,
    }))
    .unwrap();
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let single_backend = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let pipeline = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            pipeline: Some(OcrPipelineConfig {
                stages: vec![OcrPipelineStage {
                    backend: BACKEND.to_string(),
                    priority: 100,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: None,
                    backend_options: None,
                }],
                quality_thresholds: crate::core::config::OcrQualityThresholds {
                    pipeline_min_quality: 0.0,
                    ..Default::default()
                },
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let boundary = |text: &str| {
        [crate::types::PageBoundary {
            byte_start: 0,
            byte_end: text.len(),
            page_number: 1,
        }]
    };
    let mut results = Vec::new();
    for (route, config) in [("single backend", &single_backend), ("pipeline", &pipeline)] {
        let long = extract_mixed_ocr_native(LONG_LAYER, &boundary(LONG_LAYER), &[1], &pdf, config, None)
            .await
            .unwrap();
        let short = extract_mixed_ocr_native(TITLE, &boundary(TITLE), &[1], &pdf, config, None)
            .await
            .unwrap();
        results.push((route, long.0, long.1, short.0));
    }
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();

    for (route, long_text, long_replaced, short_text) in results {
        assert_eq!(
            long_text, LONG_LAYER,
            "{route}: a page the OCR text alone does not replace stays native"
        );
        assert!(
            long_replaced.is_empty(),
            "{route}: no replacement for the substantial layer"
        );
        assert_eq!(
            short_text,
            format!("{TITLE}\n\n{NOISY}"),
            "{route}: the short layer goes first"
        );
    }
}

/// A whole-document run chooses its output on the OCR text, then puts the layer first on a noise
/// page, in the flat text, the page text and the structured document. A page without a layer, and
/// a layer whose words the OCR text already holds, stay as the OCR text. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_put_the_layer_first_on_a_noise_page_of_a_single_backend_whole_document_run() {
    use super::pipeline::{PageOcrHints, extract_with_ocr_with_page_hints};
    use crate::core::config::{ExtractionConfig, OcrConfig};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-whole-document-backend";
    const NOISY: &str = "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z A B C D";
    const TITLE: &str = "Meeting Notes";

    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: BACKEND,
        text: NOISY,
        noisy: true,
    }))
    .unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let images = [image::DynamicImage::new_rgb8(16, 16)];
    let run = |layer: &'static str| {
        let (config, images) = (&config, &images);
        async move {
            let hints = PageOcrHints {
                source_dpi: None,
                known_full_page_scan: false,
                single_block_pages: None,
                native_page_layers: Some(Arc::new(ahash::AHashMap::from([(1, layer.to_string())]))),
            };
            extract_with_ocr_with_page_hints(
                None,
                Some(images),
                #[cfg(feature = "layout-detection")]
                None,
                config,
                None,
                Some(hints),
            )
            .await
            .expect("the whole-document run must return a result")
        }
    };
    let with_title = run(TITLE).await;
    let with_ocr_words = run("a b c").await;
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();

    let expected = format!("{TITLE}\n\n{NOISY}");
    assert_eq!(with_title.0, expected, "the flat text holds the layer first");
    assert_eq!(with_title.6, [expected.as_str()], "the page text holds the layer first");
    let document = with_title.4.expect("the run must return a document");
    assert!(
        document.elements.iter().any(|element| element.text == TITLE),
        "the structured document holds the layer"
    );
    assert_eq!(with_ocr_words.0, NOISY, "a layer made of OCR words adds nothing");
}

/// Two pages with identical OCR text and a layer only on the later page: the layer goes on page 2
/// in the flat text, the page texts and the document alike, never on page 1. ~keep
#[test]
fn should_put_the_layer_on_the_later_of_two_identical_ocr_pages() {
    use super::document::{flat_ocr_page_document, put_layers_first_in_run};
    use super::scoring::OcrPageNoiseVerdict;

    let verdict = |page_index: usize| OcrPageNoiseVerdict {
        page_index,
        low_confidence: true,
        fragmented_noise: false,
        dictionary_noise: false,
        fragmented_word_ratio: 0.0,
        word_count: 3,
        mean_confidence: Some(0.2),
        dict_invalid_word_ratio: None,
        discarded: false,
    };
    let mut page_texts = vec!["xq zv wj".to_string(), "xq zv wj".to_string()];
    let mut text = page_texts.join("\n\n");
    let mut doc = flat_ocr_page_document("xq zv wj");
    doc.elements[0].page = Some(1);
    let mut second = flat_ocr_page_document("xq zv wj");
    second.elements[0].page = Some(2);
    doc.elements.append(&mut second.elements);
    let layers = ahash::AHashMap::from([(2, "Meeting Notes".to_string())]);
    put_layers_first_in_run(
        &mut text,
        &mut page_texts,
        Some(&mut doc),
        &layers,
        &[verdict(0), verdict(1)],
        0,
        None,
    );
    assert_eq!(page_texts, ["xq zv wj", "Meeting Notes\n\nxq zv wj"]);
    assert_eq!(text, "xq zv wj\n\nMeeting Notes\n\nxq zv wj");
    let texts: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, ["xq zv wj", "Meeting Notes", "xq zv wj"]);

    let mut marked_pages = vec!["xq zv wj".to_string(), "xq zv wj".to_string()];
    let mut marked = "<p1>xq zv wj<p2>xq zv wj".to_string();
    put_layers_first_in_run(
        &mut marked,
        &mut marked_pages,
        None,
        &layers,
        &[verdict(1)],
        0,
        Some("<p{page_num}>"),
    );
    assert_eq!(marked, "<p1>xq zv wj<p2>Meeting Notes\n\nxq zv wj");
}
