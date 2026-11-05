//! A replaced page keeps its readable text layer first when its OCR text is judged recognition
//! noise, or when its OCR text confirms a layer that passes the per-page quality check.

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
    use crate::core::config::ExtractionConfig;
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
        super::document::PageLayers {
            lines: &layers,
            boxes: &ahash::AHashMap::default(),
        },
        &[verdict(1, false), verdict(2, true)],
        0,
        &ExtractionConfig::default(),
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
    use crate::core::config::ExtractionConfig;

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
        super::document::PageLayers {
            lines: &layers,
            boxes: &ahash::AHashMap::default(),
        },
        &[verdict(0), verdict(1)],
        0,
        &ExtractionConfig::default(),
    );
    assert_eq!(page_texts, ["xq zv wj", "Meeting Notes\n\nxq zv wj"]);
    assert_eq!(text, "xq zv wj\n\nMeeting Notes\n\nxq zv wj");
    let texts: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, ["xq zv wj", "Meeting Notes", "xq zv wj"]);

    let mut marked_pages = vec!["xq zv wj".to_string(), "xq zv wj".to_string()];
    let marked_config = ExtractionConfig {
        pages: Some(crate::core::config::PageConfig {
            insert_page_markers: true,
            marker_format: "<p{page_num}>".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut marked = "<p1>xq zv wj<p2>xq zv wj".to_string();
    put_layers_first_in_run(
        &mut marked,
        &mut marked_pages,
        None,
        super::document::PageLayers {
            lines: &layers,
            boxes: &ahash::AHashMap::default(),
        },
        &[verdict(1)],
        0,
        &marked_config,
    );
    assert_eq!(marked, "<p1>xq zv wj<p2>Meeting Notes\n\nxq zv wj");
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_keep_the_title_first_when_the_extractor_runs_ocr_on_the_whole_document() {
    use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
    use crate::extractors::pdf::PdfExtractor;
    use crate::plugins::InternalDocumentExtractor;
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-extractor-whole-document-backend";
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
    let internal = PdfExtractor::new()
        .extract_content(&lines_pdf(&[TITLE]), "application/pdf", &config)
        .await;
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();
    let result = crate::extraction::derive::derive_extraction_result(
        internal.expect("the extractor must return a result"),
        true,
        OutputFormat::Plain,
    );

    assert!(
        result.content.contains(NOISY),
        "the OCR text must be in the result, or this test proves nothing: {:?}",
        result.content
    );
    assert!(
        result.content.starts_with(TITLE),
        "the title of the text layer goes first: {:?}",
        result.content
    );
}

/// A page the caller forced to OCR is replaced by definition: it keeps no text layer, however
/// short and readable. The same call without the forced page list puts the title first, which
/// shows the OCR text is judged noise in this test. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_give_a_forced_page_the_ocr_text_without_the_text_layer() {
    use super::pipeline::extract_mixed_ocr_native;
    use crate::core::config::{ExtractionConfig, OcrConfig};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-forced-page-backend";
    const NOISY: &str = "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z A B C D";
    const TITLE: &str = "Meeting Notes";

    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: BACKEND,
        text: NOISY,
        noisy: true,
    }))
    .unwrap();
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let boundary = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: TITLE.len(),
        page_number: 1,
    }];
    let config = |forced: Option<Vec<u32>>| ExtractionConfig {
        use_cache: false,
        force_ocr_pages: forced,
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let automatic = extract_mixed_ocr_native(TITLE, &boundary, &[1], &pdf, &config(None), None)
        .await
        .unwrap();
    let forced = extract_mixed_ocr_native(TITLE, &boundary, &[1], &pdf, &config(Some(vec![1])), None)
        .await
        .unwrap();
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();

    assert_eq!(
        automatic.0,
        format!("{TITLE}\n\n{NOISY}"),
        "control: an automatic page keeps the title first"
    );
    assert_eq!(forced.0, NOISY, "a forced page is the OCR text alone");
    assert_eq!(forced.1.get(&1).map(String::as_str), Some(NOISY));
}

/// The OCR text of a page that the noise gate discards is empty. In a whole-document run the
/// layer does not go in front of that empty text: the page stays empty. Another page of the same
/// run, not discarded, gets its layer, so the layer words differ from the OCR text and the
/// whole-word skip cannot hide the discarded case. ~keep
#[test]
fn should_not_put_a_layer_on_a_page_the_noise_gate_discarded() {
    use super::document::put_layers_first_in_run;
    use super::scoring::OcrPageNoiseVerdict;
    use crate::core::config::ExtractionConfig;

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
    let mut page_texts = vec![String::new(), "xq zv wj".to_string()];
    let mut text = page_texts.join("\n\n");
    let layers = ahash::AHashMap::from([(1, "Meeting Notes".to_string()), (2, "Budget Report".to_string())]);
    put_layers_first_in_run(
        &mut text,
        &mut page_texts,
        None,
        super::document::PageLayers {
            lines: &layers,
            boxes: &ahash::AHashMap::default(),
        },
        &[verdict(0, true), verdict(1, false)],
        0,
        &ExtractionConfig::default(),
    );
    assert_eq!(page_texts, ["", "Budget Report\n\nxq zv wj"]);
    assert_eq!(text, "\n\nBudget Report\n\nxq zv wj");
}

/// On the page route a discarded page has no OCR text to replace the native page, so the native
/// page stays byte for byte whatever its layer. This pins the result a discarded page must give.
/// It passes with or without the discarded-page check that sits before the layer step in the
/// route, because the empty text is refused as a replacement first. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_keep_the_native_page_when_the_noise_gate_discards_its_ocr_text() {
    use super::pipeline::extract_mixed_ocr_native;
    use crate::core::config::{ExtractionConfig, OcrConfig, OcrQualityThresholds};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-discarded-page-backend";
    const NOISY: &str = "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z A B C D";
    const TITLE: &str = "Meeting Notes";

    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: BACKEND,
        text: NOISY,
        noisy: true,
    }))
    .unwrap();
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let boundary = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: TITLE.len(),
        page_number: 1,
    }];
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            quality_thresholds: Some(OcrQualityThresholds {
                discard_suspected_ocr_noise: true,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let run = extract_mixed_ocr_native(TITLE, &boundary, &[1], &pdf, &config, None)
        .await
        .unwrap();
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();

    assert_eq!(run.0, TITLE, "the native page stays");
    assert!(run.1.is_empty(), "no replacement for a discarded page");
}

/// The page-by-page run of a whole document, used for the default classical pipeline, hands the
/// layers of its hints to its pages: a noise page keeps its title first. ~keep
#[cfg(all(paddle_ocr, feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_keep_the_title_first_in_the_page_by_page_whole_document_pipeline() {
    use super::pipeline::{PageOcrHints, extract_full_document_ocr_pipeline_per_page};
    use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-page-by-page-backend";
    const NOISY: &str = "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z A B C D";
    const TITLE: &str = "Meeting Notes";

    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: BACKEND,
        text: NOISY,
        noisy: true,
    }))
    .unwrap();
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let config = ExtractionConfig {
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
    let hints = PageOcrHints {
        source_dpi: None,
        known_full_page_scan: false,
        single_block_pages: None,
        native_page_layers: Some(Arc::new(ahash::AHashMap::from([(1, TITLE.to_string())]))),
    };
    let result = Box::pin(extract_full_document_ocr_pipeline_per_page(
        &pdf,
        &config,
        None,
        Some(hints),
        #[cfg(feature = "layout-detection")]
        None,
    ))
    .await;
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();
    let result = result.expect("the page-by-page run must return a result");

    let expected = format!("{TITLE}\n\n{NOISY}");
    assert_eq!(result.0, expected, "the flat text holds the title first");
    assert_eq!(result.5, [expected.as_str()], "the page text holds the title first");
}

/// Six lines of plain prose: a layer that passes the per-page quality check. ~keep
const HARBOUR_LAYER: [&str; 6] = [
    "The committee met on Tuesday to review the annual budget for the harbour district.",
    "Members agreed that the proposal was sound and asked for a revised timetable.",
    "The treasurer will prepare the final report before the next quarterly meeting.",
    "Repairs to the north pier start in March and finish before the summer season.",
    "A public notice about the closure goes to every household along the coast road.",
    "The next meeting takes place in the spring at the library on Market Street.",
];

/// What the OCR backend of the page-route test reads: four of the six layer lines, or all six. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
const HARBOUR_PARTIAL_READ: &str = "\
    The committee met on Tuesday to review the annual budget for the harbour district.\n\
    Members agreed that the proposal was sound and asked for a revised timetable.\n\
    The treasurer will prepare the final report before the next quarterly meeting.\n\
    A public notice about the closure goes to every household along the coast road.";
#[cfg(all(feature = "pdf", feature = "ocr"))]
const HARBOUR_FULL_READ: &str = "\
    The committee met on Tuesday to review the annual budget for the harbour district.\n\
    Members agreed that the proposal was sound and asked for a revised timetable.\n\
    The treasurer will prepare the final report before the next quarterly meeting.\n\
    Repairs to the north pier start in March and finish before the summer season.\n\
    A public notice about the closure goes to every household along the coast road.\n\
    The next meeting takes place in the spring at the library on Market Street.";

/// Shift every ASCII letter three places, as a wrong `/ToUnicode` map does: the shape of the
/// text stays clean and no word is a word of the true text. ~keep
fn rot3(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'a'..='z' => (((c as u8 - b'a' + 3) % 26) + b'a') as char,
            'A'..='Z' => (((c as u8 - b'A' + 3) % 26) + b'A') as char,
            _ => c,
        })
        .collect()
}

#[test]
fn should_confirm_a_layer_when_the_ocr_text_reads_most_of_its_words() {
    use super::document::ocr_confirms_layer;

    let thresholds = OcrQualityThresholds::default();
    let layer = HARBOUR_LAYER.join("\n");
    let partial = [HARBOUR_LAYER[0], HARBOUR_LAYER[1], HARBOUR_LAYER[2], HARBOUR_LAYER[4]].join("\n");
    assert!(ocr_confirms_layer(&layer, &partial, &thresholds));
    assert!(
        !ocr_confirms_layer(&rot3(&layer), &layer, &thresholds),
        "a wrong-letter layer shares no word with the OCR read"
    );
    assert!(
        !ocr_confirms_layer(&layer, HARBOUR_LAYER[5], &thresholds),
        "one line of six is not most of the words"
    );
    assert!(
        !ocr_confirms_layer("Meeting Notes", "Meeting Notes", &thresholds),
        "a layer that fails the per-page quality check is never confirmed"
    );
}

/// Page route (#2439): a page with a correct layer goes to OCR, and the OCR text reads four of its
/// six lines. The replacement keeps the two lines the OCR text lacks, first, so every word of the
/// layer stays. A wrong-letter layer gets the OCR text alone, as before. Both the single-backend
/// route and the pipeline route are driven. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_keep_every_layer_word_when_page_ocr_confirms_the_layer_and_reads_less() {
    use super::pipeline::extract_mixed_ocr_native;
    use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage};
    use std::sync::Arc;

    const PARTIAL: &str = "layer-test-confirmed-partial-backend";
    const FULL: &str = "layer-test-confirmed-full-backend";
    let partial_text = HARBOUR_PARTIAL_READ;
    let full_text = HARBOUR_LAYER.join("\n");
    for (name, text) in [(PARTIAL, HARBOUR_PARTIAL_READ), (FULL, HARBOUR_FULL_READ)] {
        crate::plugins::register_ocr_backend(Arc::new(StageBackend {
            name,
            text,
            noisy: false,
        }))
        .unwrap();
    }
    let single_backend = |name: &str| ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: name.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let pipeline = |name: &str| ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            pipeline: Some(OcrPipelineConfig {
                stages: vec![OcrPipelineStage {
                    backend: name.to_string(),
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
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let boundary = |text: &str| {
        [crate::types::PageBoundary {
            byte_start: 0,
            byte_end: text.len(),
            page_number: 1,
        }]
    };
    let layer = HARBOUR_LAYER.join("\n");
    let wrong = rot3(&layer);
    let mut results = Vec::new();
    for (route, config) in [
        ("single backend", [single_backend(PARTIAL), single_backend(FULL)]),
        ("pipeline", [pipeline(PARTIAL), pipeline(FULL)]),
    ] {
        let confirmed = extract_mixed_ocr_native(&layer, &boundary(&layer), &[1], &pdf, &config[0], None)
            .await
            .unwrap();
        let wrong_letters = extract_mixed_ocr_native(&wrong, &boundary(&wrong), &[1], &pdf, &config[1], None)
            .await
            .unwrap();
        results.push((route, confirmed.0, confirmed.1, wrong_letters.0));
    }
    for name in [PARTIAL, FULL] {
        crate::plugins::unregister_ocr_backend(name).unwrap();
    }

    for (route, confirmed_text, replaced, wrong_text) in results {
        assert!(
            replaced.contains_key(&1),
            "{route}: the OCR text must replace the page, or this test proves nothing"
        );
        assert_eq!(
            confirmed_text,
            format!("{}\n{}\n\n{partial_text}", HARBOUR_LAYER[3], HARBOUR_LAYER[5]),
            "{route}: the two layer lines the OCR text lacks go first"
        );
        assert_eq!(
            wrong_text, full_text,
            "{route}: a wrong-letter layer gets the OCR text alone"
        );
    }
}

/// A whole-document run (#2439): a page with no noise verdict whose OCR text confirms its layer
/// gets the layer lines the OCR text lacks first. A wrong-letter layer and a layer the OCR text
/// does not confirm stay as the OCR text. ~keep
#[test]
fn should_put_a_confirmed_layer_first_on_a_page_of_a_whole_document_run() {
    use super::document::put_layers_first_in_run;
    use crate::core::config::ExtractionConfig;

    let layer = HARBOUR_LAYER.join("\n");
    let partial = [HARBOUR_LAYER[0], HARBOUR_LAYER[1], HARBOUR_LAYER[2], HARBOUR_LAYER[4]].join("\n");
    let mut page_texts = vec![partial.clone(), layer.clone(), HARBOUR_LAYER[5].to_string()];
    let mut text = page_texts.join("\n\n");
    let layers = ahash::AHashMap::from([(1, layer.clone()), (2, rot3(&layer)), (3, layer.clone())]);
    put_layers_first_in_run(
        &mut text,
        &mut page_texts,
        None,
        super::document::PageLayers {
            lines: &layers,
            boxes: &ahash::AHashMap::default(),
        },
        &[],
        0,
        &ExtractionConfig::default(),
    );
    let first = format!("{}\n{}\n\n{partial}", HARBOUR_LAYER[3], HARBOUR_LAYER[5]);
    assert_eq!(page_texts, [first.clone(), layer.clone(), HARBOUR_LAYER[5].to_string()]);
    assert_eq!(text, format!("{first}\n\n{layer}\n\n{}", HARBOUR_LAYER[5]));
}

/// A whole-document run (#2439): a page whose noise verdict discarded its OCR text keeps no layer,
/// even when text sits on it later and that text would confirm the layer. ~keep
#[test]
fn should_not_put_a_confirmed_layer_on_a_page_the_noise_gate_discarded() {
    use super::document::put_layers_first_in_run;
    use super::scoring::OcrPageNoiseVerdict;
    use crate::core::config::ExtractionConfig;

    let partial = [HARBOUR_LAYER[0], HARBOUR_LAYER[1], HARBOUR_LAYER[2], HARBOUR_LAYER[4]].join("\n");
    let mut page_texts = vec![partial.clone()];
    let mut text = partial.clone();
    let layers = ahash::AHashMap::from([(1, HARBOUR_LAYER.join("\n"))]);
    let discarded = OcrPageNoiseVerdict {
        page_index: 0,
        low_confidence: true,
        fragmented_noise: false,
        dictionary_noise: false,
        fragmented_word_ratio: 0.0,
        word_count: 3,
        mean_confidence: Some(0.2),
        dict_invalid_word_ratio: None,
        discarded: true,
    };
    put_layers_first_in_run(
        &mut text,
        &mut page_texts,
        None,
        super::document::PageLayers {
            lines: &layers,
            boxes: &ahash::AHashMap::default(),
        },
        &[discarded],
        0,
        &ExtractionConfig::default(),
    );
    assert_eq!(page_texts, [partial.as_str()]);
    assert_eq!(text, partial);
}

/// A line with its longest word misread: one letter added. ~keep
fn misread_longest_word(line: &str) -> String {
    let stem = |word: &str| word.trim_end_matches(|c: char| !c.is_alphanumeric()).to_string();
    let longest = line
        .split(' ')
        .map(stem)
        .max_by_key(|word| word.len())
        .unwrap_or_default();
    line.replacen(&longest, &format!("{longest}x"), 1)
}

/// The box of a line on row `row` of a page: 12 units per row, 10 high, the page's width. ~keep
fn row_box(row: usize) -> super::document::LineBox {
    let top = row as f32 * 12.0;
    super::document::LineBox {
        left: 0.0,
        top,
        right: 500.0,
        bottom: top + 10.0,
    }
}

/// The non-empty lines of a text, each on the row of `rows` with its index. ~keep
fn lines_at_rows(text: &str, rows: &[usize]) -> Vec<super::document::LineAt> {
    let lines: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(lines.len(), rows.len(), "one row per non-empty line");
    lines
        .iter()
        .zip(rows)
        .map(|(line, row)| super::document::LineAt::new(line, row_box(*row)))
        .collect()
}

/// The boxes of a page whose layer lines sit on rows 0.. in order, and whose OCR lines are given. ~keep
fn page_boxes(layer: &[&str], ocr: &[super::document::LineAt]) -> super::document::PageLineBoxes {
    super::document::PageLineBoxes {
        layer: layer
            .iter()
            .enumerate()
            .map(|(row, line)| super::document::LineAt::new(line, row_box(row)))
            .collect(),
        ocr: ocr.to_vec(),
    }
}

/// A whole-document run (#2439): OCR misreads one word on every line of a correct layer. The page
/// holds each line once, with the layer's words, and not the OCR copy beside it. An OCR line that
/// the layer lacks stays, and so does an exact read. The structured document drops the copies
/// too, keeps the other elements and the relationships among them. ~keep
#[test]
fn should_keep_each_line_once_when_ocr_misreads_one_word_per_line() {
    use super::document::{flat_ocr_page_document, put_layers_first_in_run};
    use crate::core::config::ExtractionConfig;
    use crate::types::internal::{Relationship, RelationshipKind, RelationshipTarget};

    let layer = HARBOUR_LAYER.join("\n");
    let misread: Vec<String> = HARBOUR_LAYER.iter().map(|line| misread_longest_word(line)).collect();
    let layers = ahash::AHashMap::from([(1, layer.clone())]);
    let run = |ocr_text: &str, ocr_rows: &[usize], doc: Option<&mut crate::types::internal::InternalDocument>| {
        let mut page_texts = vec![ocr_text.to_string()];
        let mut text = ocr_text.to_string();
        let boxes = ahash::AHashMap::from([(1, page_boxes(&HARBOUR_LAYER, &lines_at_rows(ocr_text, ocr_rows)))]);
        put_layers_first_in_run(
            &mut text,
            &mut page_texts,
            doc,
            super::document::PageLayers {
                lines: &layers,
                boxes: &boxes,
            },
            &[],
            0,
            &ExtractionConfig::default(),
        );
        assert_eq!(page_texts[0], text, "the flat text and the page text agree");
        text
    };

    assert_eq!(
        run(&misread.join("\n"), &[0, 1, 2, 3, 4, 5], None),
        layer,
        "every line once, with the layer's words"
    );

    let mixed = format!(
        "{}\n{}\n{}\n{}\n\nStamped received",
        HARBOUR_LAYER[0], misread[1], misread[2], HARBOUR_LAYER[3]
    );
    let mut doc = flat_ocr_page_document(&format!("{}\n\n{}\n{}", misread[1], HARBOUR_LAYER[0], misread[2]));
    doc.push_element(crate::types::internal::InternalElement::text(
        crate::types::internal::ElementKind::OcrText {
            level: crate::types::ocr_elements::OcrElementLevel::Block,
        },
        "Stamped received",
        0,
    ));
    for element in &mut doc.elements {
        element.page = Some(1);
    }
    doc.relationships.push(Relationship {
        source: 2,
        target: RelationshipTarget::Index(1),
        kind: RelationshipKind::InternalLink,
    });
    doc.relationships.push(Relationship {
        source: 2,
        target: RelationshipTarget::Index(0),
        kind: RelationshipKind::InternalLink,
    });
    // The OCR-only stamp has a row of its own, below every layer line. ~keep
    let text = run(&mixed, &[0, 1, 2, 3, 9], Some(&mut doc));
    let lines: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();
    let mut sorted = lines.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(lines.len(), sorted.len(), "no line twice: {text:?}");
    assert_eq!(lines.len(), 7, "six layer lines and the OCR-only line: {text:?}");
    assert!(text.ends_with("Stamped received"), "the OCR-only line stays: {text:?}");
    for copy in &misread[1..3] {
        assert!(!text.contains(copy.as_str()), "the OCR copy {copy:?} goes: {text:?}");
    }
    let layer_first = [HARBOUR_LAYER[1], HARBOUR_LAYER[2], HARBOUR_LAYER[4], HARBOUR_LAYER[5]].join("\n");
    let texts: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, [layer_first.as_str(), HARBOUR_LAYER[0], "Stamped received"]);
    assert_eq!(doc.relationships.len(), 1, "the relationship to a dropped element goes");
    assert_eq!(doc.relationships[0].source, 2);
    assert!(matches!(doc.relationships[0].target, RelationshipTarget::Index(1)));
}

/// Six lines of numbers: a layer that passes the per-page quality check and has no word with a
/// letter. ~keep
const LEDGER_LAYER: [&str; 6] = [
    "1042 2087 3315 4471 5530 6694",
    "7120 8243 9356 1468 2579 3681",
    "4792 5804 6915 7026 8137 9248",
    "1359 2460 3571 4682 5793 6804",
    "7915 8026 9137 1248 2359 3460",
    "4571 5682 6793 7804 8915 9026",
];

/// A whole-document run (#2439): a layer of numbers only. The OCR text reads three of the six
/// lines, so it confirms the layer and the other three lines come back. Numbers that the OCR text
/// does not hold, and too few numbers, never confirm a layer. ~keep
#[test]
fn should_confirm_a_number_only_layer_against_the_ocr_text() {
    use super::document::{ocr_confirms_layer, put_layers_first_in_run};
    use crate::core::config::ExtractionConfig;

    let thresholds = OcrQualityThresholds::default();
    let layer = LEDGER_LAYER.join("\n");
    let read = LEDGER_LAYER[..3].join("\n");
    assert!(
        ocr_confirms_layer(&layer, &read, &thresholds),
        "the OCR text holds half the numbers"
    );
    let other = "9911 8822 7733 6644 5555 4466\n3377 2288 1199 9900 8811 7722";
    assert!(
        !ocr_confirms_layer(&layer, other, &thresholds),
        "garbage shares no number"
    );
    assert!(
        !ocr_confirms_layer(&layer, &LEDGER_LAYER[5].replace("4571", "1111"), &thresholds),
        "one line of six is not half of the numbers"
    );
    assert!(
        !ocr_confirms_layer("2024 17", "2024 17", &thresholds),
        "two numbers are too few to confirm a layer"
    );

    let mut page_texts = vec![read.clone()];
    let mut text = read.clone();
    let layers = ahash::AHashMap::from([(1, layer)]);
    put_layers_first_in_run(
        &mut text,
        &mut page_texts,
        None,
        super::document::PageLayers {
            lines: &layers,
            boxes: &ahash::AHashMap::default(),
        },
        &[],
        0,
        &ExtractionConfig::default(),
    );
    assert_eq!(
        text,
        format!(
            "{}\n{}\n{}\n\n{read}",
            LEDGER_LAYER[3], LEDGER_LAYER[4], LEDGER_LAYER[5]
        )
    );
}

/// A layer of numbers with only two distinct numbers on six full lines: it passes the per-page
/// quality check, and the OCR text holds both numbers by chance. Two numbers never confirm a layer
/// (#2439), and three do. ~keep
#[test]
fn should_need_three_distinct_numbers_to_confirm_a_number_only_layer() {
    use super::document::ocr_confirms_layer;

    let thresholds = OcrQualityThresholds::default();
    let line = |numbers: &str| [numbers; 6].join(" ");
    let two = vec![line("1042 2087"); 6].join("\n");
    let three = vec![line("1042 2087 3315"); 6].join("\n");
    assert!(
        !ocr_confirms_layer(&two, &two, &thresholds),
        "two distinct numbers are too few to confirm a layer"
    );
    assert!(
        ocr_confirms_layer(&three, &three, &thresholds),
        "three distinct numbers confirm a layer"
    );
}

/// A whole-document run (#2439): an OCR line that reads one layer line exactly is never the copy of
/// another layer line that differs from it by one word. The layer holds a line and a near twin of
/// it; the OCR text reads the first exactly and lacks the twin. The twin comes back, and the exact
/// OCR line stays. ~keep
#[test]
fn should_not_drop_an_exact_ocr_line_as_the_copy_of_its_near_twin_in_the_layer() {
    use super::document::layer_lines_to_put_first;

    let thresholds = OcrQualityThresholds::default();
    let twin = misread_longest_word(HARBOUR_LAYER[0]);
    let layer = format!("{}\n{twin}", HARBOUR_LAYER.join("\n"));
    let ocr_text = HARBOUR_LAYER[..4].join("\n");
    // The twin sits on the first line's row: it owns the exact OCR line's place. ~keep
    let mut boxes = page_boxes(&HARBOUR_LAYER, &lines_at_rows(&ocr_text, &[0, 1, 2, 3]));
    boxes.layer.push(super::document::LineAt::new(&twin, row_box(0)));
    let first = layer_lines_to_put_first(&layer, &ocr_text, false, &thresholds, Some(&boxes))
        .expect("the OCR text confirms the layer, and the twin and two more lines are absent from it");
    assert!(
        first.lines.lines().any(|line| line == twin),
        "the twin comes back: {:?}",
        first.lines
    );
    assert!(
        first.ocr_copies.is_empty(),
        "an exact OCR line is not the copy of the twin: {:?}",
        first.ocr_copies
    );
}

/// A page of common nouns in no order that the language check calls implausible: a correct
/// layer that the check sends to OCR (#2439). ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
const NOUN_LINES: [&str; 15] = [
    "Gather Pebble Lantern",
    "marble fiddle orchard sprocket ladle quiver tundra bramble",
    "pewter saddle cobbler thimble garnet basin hollow ferment",
    "rafter nozzle meadow scaffold parsnip tinder ledger ripple",
    "kettle bauble furrow gimlet canopy trowel muslin quarry",
    "lintel spindle hazel pommel barley dowel sorrel wicket",
    "copper flagon trellis mortar skein gusset paddle rivet",
    "almond burrow cistern tassel freckle hamper pigeon funnel",
    "walnut grommet shutter beacon dapple girder lattice nutmeg",
    "turnip anvil drizzle buckle cinder pantry heron welter",
    "gravel tumbler lagoon pivot sorghum brisket jigsaw thistle",
    "velvet cobble ferrule mantle quince sedge ratchet pulley",
    "wicker tallow ember swivel cudgel prism oxbow kindling",
    "mitten chisel gourd rampart slate bobbin culvert wren",
    "pumice clasp jetty mullion gable hearth sluice vellum",
];

/// What the OCR backend of the extractor test reads: the first ten lines of [`NOUN_LINES`] and one
/// line that the layer lacks. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
const NOUN_READ: &str = "Gather Pebble Lantern\n\
    marble fiddle orchard sprocket ladle quiver tundra bramble\n\
    pewter saddle cobbler thimble garnet basin hollow ferment\n\
    rafter nozzle meadow scaffold parsnip tinder ledger ripple\n\
    kettle bauble furrow gimlet canopy trowel muslin quarry\n\
    lintel spindle hazel pommel barley dowel sorrel wicket\n\
    copper flagon trellis mortar skein gusset paddle rivet\n\
    almond burrow cistern tassel freckle hamper pigeon funnel\n\
    walnut grommet shutter beacon dapple girder lattice nutmeg\n\
    turnip anvil drizzle buckle cinder pantry heron welter\n\
    Stamped received";

/// The structured document a line-level OCR backend returns for the page of [`lines_pdf`]: one
/// `OcrText { level: Line }` element per line of `text`, with its box in the pixels of a
/// `width` by `height` raster of the page (top-left origin, as hOCR gives it), at the place the
/// text layer draws that line. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn ocr_line_document(text: &str, width: f64, height: f64) -> crate::types::internal::InternalDocument {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::ocr_elements::OcrElementLevel;

    let (sx, sy) = (width / 612.0, height / 792.0);
    let mut doc = InternalDocument::new("pdf");
    for (i, line) in text.lines().enumerate() {
        let baseline = 730.0 - 16.0 * i as f64;
        let mut element = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Line,
            },
            line,
            0,
        );
        element.bbox = Some(crate::types::BoundingBox {
            x0: 72.0 * sx,
            y0: (792.0 - baseline - 10.0) * sy,
            x1: (72.0 + 6.5 * line.len() as f64) * sx,
            y1: (792.0 - baseline + 3.0) * sy,
        });
        doc.push_element(element);
    }
    doc
}

/// A one-page PDF whose text layer holds `lines`, one line each, in Helvetica. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn lines_pdf(lines: &[&str]) -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let mut operations = vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 13.into()]),
        Operation::new("TL", vec![16.into()]),
        Operation::new("Td", vec![72.into(), 730.into()]),
    ];
    for line in lines {
        operations.push(Operation::new("Tj", vec![Object::string_literal(*line)]));
        operations.push(Operation::new("T*", vec![]));
    }
    operations.push(Operation::new("ET", vec![]));
    let content = Content { operations };
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        content.encode().expect("the page content must encode"),
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
    let catalog_id = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    document.trailer.set("Root", catalog_id);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("the page PDF must serialize");
    bytes
}

/// The issue's route (#2439), through the extractor in automatic mode: the language check flags a
/// page whose layer is correct, OCR reads ten of its fifteen lines plus one line the layer lacks,
/// and the OCR text replaces the page. The content and the page text keep every word of the
/// layer, and the OCR line joins them. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_keep_every_layer_word_of_a_page_the_language_check_sends_to_ocr() {
    use super::plausibility::{PlausibilityVerdict, evaluate_text_plausibility};
    use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};
    use crate::extractors::pdf::PdfExtractor;
    use crate::plugins::InternalDocumentExtractor;
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-language-check-backend";
    const OCR_ONLY: &str = "Stamped received";
    assert_eq!(
        evaluate_text_plausibility(&NOUN_LINES.join("\n"), &OcrQualityThresholds::default()),
        PlausibilityVerdict::Implausible,
        "the language check must flag the layer, or this test proves nothing"
    );
    assert!(NOUN_READ.starts_with(&NOUN_LINES[..10].join("\n")) && NOUN_READ.ends_with(OCR_ONLY));
    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: BACKEND,
        text: NOUN_READ,
        noisy: false,
    }))
    .unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let internal = PdfExtractor::new()
        .extract_content(&lines_pdf(&NOUN_LINES), "application/pdf", &config)
        .await;
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();
    let result = crate::extraction::derive::derive_extraction_result(
        internal.expect("the extractor must return a result"),
        true,
        OutputFormat::Plain,
    );

    let page = result
        .pages
        .as_ref()
        .and_then(|pages| pages.first())
        .map(|page| page.content.clone())
        .unwrap_or_default();
    for (output, text) in [("content", &result.content), ("page", &page)] {
        assert!(
            text.contains(OCR_ONLY),
            "{output}: the OCR text must replace the page, or this test proves nothing: {text:?}"
        );
        let have: std::collections::HashSet<String> = text.split_whitespace().map(str::to_lowercase).collect();
        let lost: Vec<&str> = NOUN_LINES
            .iter()
            .flat_map(|line| line.split_whitespace())
            .filter(|word| !have.contains(&word.to_lowercase()))
            .collect();
        assert!(lost.is_empty(), "{output} lost layer words {lost:?}: {text:?}");
    }
}

/// A whole-document pipeline run (#2439) whose first stage the pipeline accepts: the OCR text
/// reads four of the six lines of a correct layer, so the two other lines go first. The
/// per-page quality check reads the thresholds of the run's config: a config that makes the
/// layer fail that check gets the OCR text alone. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_put_a_confirmed_layer_first_on_the_accepted_stage_of_a_pipeline_run() {
    use super::pipeline::{PageOcrHints, run_ocr_pipeline_for_page};
    use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-accepted-stage-backend";
    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: BACKEND,
        text: HARBOUR_PARTIAL_READ,
        noisy: false,
    }))
    .unwrap();
    let pipeline = OcrPipelineConfig {
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
        quality_thresholds: OcrQualityThresholds {
            pipeline_min_quality: 0.0,
            ..Default::default()
        },
    };
    let config = |thresholds: Option<OcrQualityThresholds>| ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            pipeline: Some(pipeline.clone()),
            quality_thresholds: thresholds,
            ..Default::default()
        }),
        ..Default::default()
    };
    let images = [image::DynamicImage::new_rgb8(16, 16)];
    let run = |config: ExtractionConfig| {
        let images = &images;
        let pipeline = pipeline.clone();
        async move {
            let hints = PageOcrHints {
                source_dpi: None,
                known_full_page_scan: false,
                single_block_pages: None,
                native_page_layers: Some(Arc::new(ahash::AHashMap::from([(1, HARBOUR_LAYER.join("\n"))]))),
            };
            run_ocr_pipeline_for_page(
                None,
                Some(images),
                #[cfg(feature = "layout-detection")]
                None,
                &config,
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
    let confirmed = run(config(None)).await;
    let strict = OcrQualityThresholds {
        min_total_non_whitespace: 100_000,
        min_non_whitespace_per_page: 100_000.0,
        ..Default::default()
    };
    let failing = run(config(Some(strict))).await;
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();

    assert_eq!(
        confirmed,
        format!("{}\n{}\n\n{HARBOUR_PARTIAL_READ}", HARBOUR_LAYER[3], HARBOUR_LAYER[5]),
        "the accepted stage keeps the two layer lines that the OCR text lacks"
    );
    assert_eq!(
        failing, HARBOUR_PARTIAL_READ,
        "a layer that fails the quality check of the run's config is not confirmed"
    );
}

/// An OCR backend that returns fixed text and its structured document with one line element per
/// line, with its box in the pixels of the image it reads, as the built-in backends do. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
struct StructuredBackend {
    name: &'static str,
    text: String,
}
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[async_trait::async_trait]
impl crate::plugins::OcrBackend for StructuredBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(
        &self,
        image: &[u8],
        _: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        let (width, height) = image::load_from_memory(image)
            .map(|image| (f64::from(image.width()), f64::from(image.height())))
            .unwrap_or((612.0, 792.0));
        Ok(crate::types::ExtractedDocument {
            content: self.text.clone(),
            ocr_internal_document: Some(ocr_line_document(&self.text, width, height)),
            ..Default::default()
        })
    }
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::Legibility { scale_max: 100.0 }
    }
}
#[cfg(all(feature = "pdf", feature = "ocr"))]
impl crate::plugins::Plugin for StructuredBackend {
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

/// Page route (#2439): OCR misreads one word on every line of a correct layer. The structured page
/// that the other output formats render holds each layer line once, and not the OCR copy beside
/// it, on both the single-backend route and the pipeline route. The page's text layer has
/// positions and the OCR backend returns one line element per line with its box, as the built-in
/// backends do: the merge removes a copy only where the layer line owns its place. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_drop_the_ocr_copies_from_the_structured_page_of_a_replaced_page() {
    use super::pipeline::extract_mixed_ocr_native;
    use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-structured-copies-backend";
    let misread: Vec<String> = HARBOUR_LAYER.iter().map(|line| misread_longest_word(line)).collect();
    crate::plugins::register_ocr_backend(Arc::new(StructuredBackend {
        name: BACKEND,
        text: misread.join("\n"),
    }))
    .unwrap();
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
    let pdf = lines_pdf(&HARBOUR_LAYER);
    let layer = HARBOUR_LAYER.join("\n");
    let boundary = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: layer.len(),
        page_number: 1,
    }];
    let mut results = Vec::new();
    for (route, config) in [("single backend", &single_backend), ("pipeline", &pipeline)] {
        let output = extract_mixed_ocr_native(&layer, &boundary, &[1], &pdf, config, None)
            .await
            .unwrap();
        results.push((route, output.1, output.2));
    }
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();

    for (route, replaced, structured) in results {
        assert!(
            replaced.contains_key(&1),
            "{route}: the OCR text must replace the page, or this test proves nothing"
        );
        let page = structured
            .get(&1)
            .unwrap_or_else(|| panic!("{route}: the replaced page has a structured document"));
        assert!(
            !page.elements.is_empty(),
            "{route}: the structured page holds the OCR lines, or this test proves nothing"
        );
        let text: Vec<&str> = page.elements.iter().map(|element| element.text.as_str()).collect();
        let text = text.join("\n");
        for line in HARBOUR_LAYER {
            assert_eq!(
                text.matches(line).count(),
                1,
                "{route}: layer line {line:?} once: {text:?}"
            );
        }
        for copy in &misread {
            assert!(
                !text.contains(copy.as_str()),
                "{route}: the OCR copy {copy:?} goes: {text:?}"
            );
        }
    }
}

/// Whole-document route (#2439): the run renders the page itself, so the OCR line boxes are in the
/// pixels of its raster while the text layer's are in points. The run carries each page's factor
/// to points into the merge: the structured document and the flat text hold each layer line once
/// and not the OCR copy beside it. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_drop_the_ocr_copies_from_the_structured_page_of_a_whole_document_run() {
    use super::pipeline::{PageOcrHints, extract_with_ocr_with_page_hints};
    use crate::core::config::{ExtractionConfig, OcrConfig};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-structured-copies-whole-document-backend";
    let misread: Vec<String> = HARBOUR_LAYER.iter().map(|line| misread_longest_word(line)).collect();
    crate::plugins::register_ocr_backend(Arc::new(StructuredBackend {
        name: BACKEND,
        text: misread.join("\n"),
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
    let pdf = lines_pdf(&HARBOUR_LAYER);
    let hints = PageOcrHints {
        source_dpi: None,
        known_full_page_scan: false,
        single_block_pages: None,
        native_page_layers: Some(Arc::new(ahash::AHashMap::from([(1, HARBOUR_LAYER.join("\n"))]))),
    };
    let output = extract_with_ocr_with_page_hints(
        Some(&pdf),
        None,
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
        Some(hints),
    )
    .await;
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();
    let output = output.expect("the whole-document run must return a result");

    let document = output.4.expect("the whole-document run returns a structured document");
    assert!(
        !document.elements.is_empty(),
        "the structured document holds the OCR lines, or this test proves nothing"
    );
    let structured: Vec<&str> = document.elements.iter().map(|element| element.text.as_str()).collect();
    for (route, text) in [("structured document", structured.join("\n")), ("flat text", output.0)] {
        for line in HARBOUR_LAYER {
            assert_eq!(
                text.matches(line).count(),
                1,
                "{route}: layer line {line:?} once: {text:?}"
            );
        }
        for copy in &misread {
            assert!(
                !text.contains(copy.as_str()),
                "{route}: the OCR copy {copy:?} goes: {text:?}"
            );
        }
    }
}

/// A whole-document run on images renders no PDF page, so no side has line boxes: the layer lines
/// the OCR text lacks go first and every OCR line stays, misreads included (rule 3 of the merge:
/// a duplicate line is the accepted cost, a lost distinct line is not). ~keep
#[tokio::test]
async fn should_keep_every_ocr_line_of_an_image_run_that_renders_no_pdf() {
    use super::pipeline::{PageOcrHints, extract_with_ocr_with_page_hints};
    use crate::core::config::{ExtractionConfig, OcrConfig};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-image-run-keeps-ocr-lines-backend";
    let misread: Vec<String> = HARBOUR_LAYER.iter().map(|line| misread_longest_word(line)).collect();
    crate::plugins::register_ocr_backend(Arc::new(StructuredBackend {
        name: BACKEND,
        text: misread.join("\n"),
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
    let hints = PageOcrHints {
        source_dpi: None,
        known_full_page_scan: false,
        single_block_pages: None,
        native_page_layers: Some(Arc::new(ahash::AHashMap::from([(1, HARBOUR_LAYER.join("\n"))]))),
    };
    let output = extract_with_ocr_with_page_hints(
        None,
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
        Some(hints),
    )
    .await;
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();
    let output = output.expect("the image run must return a result");

    let document = output.4.expect("the image run returns a structured document");
    let structured: Vec<&str> = document.elements.iter().map(|element| element.text.as_str()).collect();
    for (route, text) in [("structured document", structured.join("\n")), ("flat text", output.0)] {
        for line in HARBOUR_LAYER {
            assert_eq!(
                text.matches(line).count(),
                1,
                "{route}: layer line {line:?} once: {text:?}"
            );
        }
        for copy in &misread {
            assert_eq!(
                text.matches(copy.as_str()).count(),
                1,
                "{route}: without boxes the OCR line {copy:?} stays: {text:?}"
            );
        }
    }
}

/// The flat text of a page (#2439): a dropped OCR copy leaves no blank line behind, at the start of
/// the OCR text, between two paragraphs or at its end. ~keep
#[test]
fn should_leave_no_blank_line_where_an_ocr_copy_was() {
    use super::document::{LayerFirst, text_with_layer_first};

    let first = LayerFirst {
        lines: "Layer line".to_string(),
        ocr_copies: vec!["copy line".to_string()],
    };
    assert_eq!(
        text_with_layer_first("First part\n\nCopy line\n\nLast part", &first),
        "Layer line\n\nFirst part\n\nLast part"
    );
    assert_eq!(
        text_with_layer_first("Copy line\n\nLast part", &first),
        "Layer line\n\nLast part"
    );
    assert_eq!(
        text_with_layer_first("First part\n\nCopy line", &first),
        "Layer line\n\nFirst part"
    );
    assert_eq!(text_with_layer_first("Copy line", &first), "Layer line");
}

/// The structured page (#2439): a paragraph that loses an OCR copy keeps its other lines and the
/// annotations of the retained text, shifted by the removed range; an annotation inside the copy
/// goes. A paragraph that holds no copy keeps its annotations. ~keep
#[test]
fn should_shift_the_annotations_of_a_paragraph_that_loses_an_ocr_copy() {
    use super::document::{LayerFirst, put_layer_first_in_document};
    use crate::types::document_structure::{AnnotationKind, TextAnnotation};
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

    let note = |start: u32, end: u32| TextAnnotation {
        start,
        end,
        kind: AnnotationKind::Bold,
    };
    let mut doc = InternalDocument::new("pdf");
    // "Copy line\n" is bytes 0..10; "Kept" is 10..14 and "line" 15..19. ~keep
    let mut element = InternalElement::text(ElementKind::Paragraph, "Copy line\nKept line", 0);
    element
        .annotations
        .extend([note(0, 4), note(10, 14), note(15, 19), note(5, 14)]);
    doc.push_element(element);
    let mut other = InternalElement::text(ElementKind::Paragraph, "Other paragraph", 0);
    other.annotations.push(note(0, 5));
    doc.push_element(other);
    let first = LayerFirst {
        lines: "Layer line".to_string(),
        ocr_copies: vec!["copy line".to_string()],
    };
    put_layer_first_in_document(&mut doc, None, &first);
    let texts: Vec<&str> = doc.elements.iter().map(|element| element.text.as_str()).collect();
    assert_eq!(texts, ["Layer line", "Kept line", "Other paragraph"]);
    assert_eq!(
        doc.elements[1].annotations,
        [note(0, 4), note(5, 9), note(0, 4)],
        "the copy's annotation goes, the kept line's shift, one across keeps its kept part"
    );
    assert_eq!(
        doc.elements[2].annotations,
        [note(0, 5)],
        "an untouched paragraph keeps them"
    );
}

/// The element of one OCR line (hOCR `Line`) that is a copy goes whole, with its relationship; the
/// line elements around it keep their annotations untouched. ~keep
#[test]
fn should_remove_a_one_line_element_that_is_an_ocr_copy_whole() {
    use super::document::{LayerFirst, put_layer_first_in_document};
    use crate::types::document_structure::{AnnotationKind, TextAnnotation};
    use crate::types::internal::{
        ElementKind, InternalDocument, InternalElement, Relationship, RelationshipKind, RelationshipTarget,
    };
    use crate::types::ocr_elements::OcrElementLevel;

    let note = TextAnnotation {
        start: 0,
        end: 4,
        kind: AnnotationKind::Bold,
    };
    let mut doc = InternalDocument::new("pdf");
    for text in ["Kept line", "Copy line", "Last line"] {
        let mut element = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Line,
            },
            text,
            0,
        );
        element.annotations.push(note.clone());
        doc.push_element(element);
    }
    doc.relationships.push(Relationship {
        source: 1,
        target: RelationshipTarget::Index(2),
        kind: RelationshipKind::InternalLink,
    });
    doc.relationships.push(Relationship {
        source: 0,
        target: RelationshipTarget::Index(2),
        kind: RelationshipKind::InternalLink,
    });
    let first = LayerFirst {
        lines: "Layer line".to_string(),
        ocr_copies: vec!["copy line".to_string()],
    };
    put_layer_first_in_document(&mut doc, None, &first);
    let texts: Vec<&str> = doc.elements.iter().map(|element| element.text.as_str()).collect();
    assert_eq!(texts, ["Layer line", "Kept line", "Last line"]);
    assert!(
        doc.elements[1..]
            .iter()
            .all(|element| element.annotations == [note.clone()])
    );
    assert_eq!(doc.relationships.len(), 1, "the copy's relationship goes");
    assert_eq!(
        (doc.relationships[0].source, &doc.relationships[0].target),
        (1, &RelationshipTarget::Index(2))
    );
}

/// A whole-document run (#2439): two OCR lines misread the same layer line. The first in reading
/// order is the copy and goes; the second stays, so no OCR line is taken for a copy twice. ~keep
#[test]
fn should_take_the_first_of_two_ocr_misreads_of_one_layer_line_as_its_copy() {
    use super::document::{layer_lines_to_put_first, text_with_layer_first};

    let thresholds = OcrQualityThresholds::default();
    let layer = HARBOUR_LAYER.join("\n");
    let early = misread_longest_word(HARBOUR_LAYER[5]);
    let late = HARBOUR_LAYER[5].replacen("library", "librarv", 1);
    let ocr_text = format!(
        "{}\n{early}\n{}\n{late}",
        HARBOUR_LAYER[..3].join("\n"),
        HARBOUR_LAYER[3..5].join("\n")
    );
    // The early misread sits on the layer line's row; the late one on a row of its own. ~keep
    let boxes = page_boxes(&HARBOUR_LAYER, &lines_at_rows(&ocr_text, &[0, 1, 2, 5, 3, 4, 6]));
    let first = layer_lines_to_put_first(&layer, &ocr_text, false, &thresholds, Some(&boxes))
        .expect("the OCR text confirms the layer and lacks its last line");
    let text = text_with_layer_first(&ocr_text, &first);
    assert!(!text.contains(early.as_str()), "the first misread goes: {text:?}");
    assert!(text.contains(late.as_str()), "the second misread stays: {text:?}");
    assert_eq!(
        text.matches(HARBOUR_LAYER[5]).count(),
        1,
        "the layer line once: {text:?}"
    );
}

/// A layer with one word with a letter among numbers that the OCR text reads differently (#2439):
/// one shared word does not confirm it, because the OCR text must hold half of all its words. The
/// same layer is confirmed when the OCR text holds half of its numbers too. ~keep
#[test]
fn should_not_confirm_a_layer_of_numbers_on_its_one_lettered_word() {
    use super::document::ocr_confirms_layer;

    let thresholds = OcrQualityThresholds::default();
    let layer = format!("Total\n{}", LEDGER_LAYER.join("\n"));
    let other_numbers: Vec<String> = LEDGER_LAYER
        .iter()
        .map(|line| line.split(' ').map(|n| format!("{n}9")).collect::<Vec<_>>().join(" "))
        .collect();
    let misread = format!("Total\n{}", other_numbers.join("\n"));
    assert!(
        !ocr_confirms_layer(&layer, &misread, &thresholds),
        "one lettered word of a layer of numbers does not confirm it"
    );
    let half = format!("Total\n{}", LEDGER_LAYER[..3].join("\n"));
    assert!(
        ocr_confirms_layer(&layer, &half, &thresholds),
        "the OCR text holds the word and half the numbers: confirmed"
    );
}

/// The structured page (#2439): a title, a heading, an image and a table of several rows that hold
/// an OCR copy of a layer line keep their text and their kind, and the table data stays. A plain
/// OCR text block that holds a copy loses it. ~keep
#[test]
fn should_keep_a_title_heading_image_or_table_that_holds_an_ocr_copy() {
    use super::document::{LayerFirst, put_layer_first_in_document};
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

    let copy = "Copy line";
    let rows = format!("Region Total\n{copy}\nNorth 1204\nSouth 3381");
    let mut doc = InternalDocument::new("pdf");
    doc.tables.push(crate::types::Table {
        cells: vec![
            vec!["Region".into(), "Total".into()],
            vec!["Copy".into(), "line".into()],
            vec!["North".into(), "1204".into()],
        ],
        ..Default::default()
    });
    let kinds = [
        (ElementKind::Title, copy.to_string()),
        (ElementKind::Heading { level: 2 }, copy.to_string()),
        (ElementKind::Image { image_index: 0 }, copy.to_string()),
        (ElementKind::Table { table_index: 0 }, rows.clone()),
    ];
    for (kind, text) in kinds.iter().cloned() {
        doc.push_element(InternalElement::text(kind, text, 0));
    }
    doc.push_element(InternalElement::text(
        ElementKind::OcrText {
            level: crate::types::ocr_elements::OcrElementLevel::Block,
        },
        format!("{copy}\nRead line"),
        0,
    ));
    let tables_before = format!("{:?}", doc.tables);
    let first = LayerFirst {
        lines: "Layer line".to_string(),
        ocr_copies: vec!["copy line".to_string(); 5],
    };
    put_layer_first_in_document(&mut doc, None, &first);
    assert_eq!(
        doc.elements.len(),
        6,
        "the layer element and every element: {:?}",
        doc.elements
    );
    assert_eq!(doc.elements[0].text, "Layer line");
    for (element, (kind, text)) in doc.elements[1..5].iter().zip(kinds.iter()) {
        assert_eq!(&element.kind, kind);
        assert_eq!(&element.text, text, "a {kind:?} keeps its text");
    }
    assert_eq!(doc.elements[5].text, "Read line", "the OCR text block loses the copy");
    assert_eq!(format!("{:?}", doc.tables), tables_before, "the table data stays");
}

// ---- Ownership rule (rulings/2439.md): an OCR line goes only when a layer line owns its place. ----

/// Eight lines of invented minutes: the layer of the ownership tests. ~keep
const MINUTES: [&str; 8] = [
    "Harbour committee minutes",
    "Members present at the meeting",
    "Repairs to the north pier",
    "Total 100",
    "Treasurer report accepted",
    "Library hours extended",
    "Public notice posted",
    "Next meeting in spring",
];

/// The merge of one confirmed page whose OCR lines sit on `ocr_rows`, as the page text. ~keep
fn merge_at_rows(layer: &[&str], ocr: &str, ocr_rows: &[usize]) -> String {
    use super::document::put_layers_first_in_run;
    let mut page_texts = vec![ocr.to_string()];
    let mut text = ocr.to_string();
    let layers = ahash::AHashMap::from([(1u32, layer.join("\n"))]);
    let boxes = ahash::AHashMap::from([(1u32, page_boxes(layer, &lines_at_rows(ocr, ocr_rows)))]);
    put_layers_first_in_run(
        &mut text,
        &mut page_texts,
        None,
        super::document::PageLayers {
            lines: &layers,
            boxes: &boxes,
        },
        &[],
        0,
        &crate::core::config::ExtractionConfig::default(),
    );
    page_texts.remove(0)
}

fn count_line(page: &str, needle: &str) -> usize {
    page.lines().filter(|line| line.trim() == needle).count()
}

/// Row A: a distinct OCR-only "Total 900" on its own row before the misread "Total 1OO" that sits
/// on the layer line's row. The OCR-only line stays; the misread goes. ~keep
#[test]
fn should_keep_an_ocr_only_line_that_shares_a_label_with_a_misread_layer_line() {
    let mut ocr: Vec<&str> = MINUTES.to_vec();
    ocr[3] = "Total 1OO";
    ocr.insert(2, "Total 900");
    let page = merge_at_rows(&MINUTES, &ocr.join("\n"), &[0, 1, 9, 2, 3, 4, 5, 6, 7]);
    assert_eq!(
        (count_line(&page, "Total 900"), count_line(&page, "Total 1OO")),
        (1, 0),
        "{page:?}"
    );
    assert_eq!(count_line(&page, "Total 100"), 1, "{page:?}");
}

/// Row B: an OCR-only line with the words of a layer line reordered, on its own row. It stays; the
/// misread on the layer line's row goes; the layer line comes back. ~keep
#[test]
fn should_keep_an_ocr_only_line_that_reorders_the_words_of_a_misread_layer_line() {
    let mut ocr: Vec<&str> = MINUTES.to_vec();
    ocr[2] = "Repairs to the north pler";
    ocr.insert(0, "the north pier to repairs");
    let page = merge_at_rows(&MINUTES, &ocr.join("\n"), &[9, 0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(count_line(&page, "the north pier to repairs"), 1, "{page:?}");
    assert_eq!(count_line(&page, "Repairs to the north pler"), 0, "{page:?}");
    assert_eq!(count_line(&page, "Repairs to the north pier"), 1, "{page:?}");
}

/// Row C: the layer holds "Total 100" twice, each misread on its own row. Both come back, no
/// misread stays: each layer line spends one box. ~keep
#[test]
fn should_remove_both_misreads_of_a_layer_line_that_occurs_twice() {
    let mut layer: Vec<&str> = MINUTES.to_vec();
    layer.push("Total 100");
    let mut ocr = layer.clone();
    ocr[3] = "Total 1OO";
    ocr[8] = "Total 1O0";
    let page = merge_at_rows(&layer, &ocr.join("\n"), &[0, 1, 2, 3, 4, 5, 6, 7, 8]);
    let misreads = count_line(&page, "Total 1OO") + count_line(&page, "Total 1O0");
    assert_eq!((count_line(&page, "Total 100"), misreads), (2, 0), "{page:?}");
}

/// Row D: the layer holds "Total 100" and "Total 900"; OCR reads 900 exactly. Both totals once. ~keep
#[test]
fn should_keep_an_exactly_read_second_total_beside_a_misread_first() {
    let mut layer: Vec<&str> = MINUTES.to_vec();
    layer.insert(4, "Total 900");
    let mut ocr = layer.clone();
    ocr[3] = "Total 1OO";
    let page = merge_at_rows(&layer, &ocr.join("\n"), &[0, 1, 2, 3, 4, 5, 6, 7, 8]);
    let counts = (
        count_line(&page, "Total 100"),
        count_line(&page, "Total 900"),
        count_line(&page, "Total 1OO"),
    );
    assert_eq!(counts, (1, 1, 0), "{page:?}");
}

/// Row E: as row A with the OCR-only line after the misread. The rule does not depend on order. ~keep
#[test]
fn should_keep_an_ocr_only_line_after_the_misread_too() {
    let mut ocr: Vec<&str> = MINUTES.to_vec();
    ocr[3] = "Total 1OO";
    ocr.insert(6, "Total 900");
    let page = merge_at_rows(&MINUTES, &ocr.join("\n"), &[0, 1, 2, 3, 4, 5, 9, 6, 7]);
    assert_eq!(
        (count_line(&page, "Total 900"), count_line(&page, "Total 1OO")),
        (1, 0),
        "{page:?}"
    );
}

/// A misread on a far row is not owned: it stays beside the layer line (a duplicate, never a lost
/// line). A page without boxes removes nothing either. ~keep
#[test]
fn should_remove_nothing_without_ownership_or_without_boxes() {
    use super::document::layer_lines_to_put_first;
    let mut ocr: Vec<&str> = MINUTES.to_vec();
    ocr[3] = "Total 1OO";
    let page = merge_at_rows(&MINUTES, &ocr.join("\n"), &[0, 1, 2, 9, 4, 5, 6, 7]);
    assert_eq!(
        (count_line(&page, "Total 100"), count_line(&page, "Total 1OO")),
        (1, 1),
        "{page:?}"
    );
    let thresholds = OcrQualityThresholds::default();
    let first = layer_lines_to_put_first(&MINUTES.join("\n"), &ocr.join("\n"), false, &thresholds, None)
        .expect("the OCR text confirms the layer and lacks one line");
    assert_eq!(first.lines, "Total 100");
    assert!(first.ocr_copies.is_empty(), "no boxes, no copy: {:?}", first.ocr_copies);
}

/// Ownership needs half of the shorter height and half of the shorter width. ~keep
#[test]
fn should_own_a_box_on_half_of_its_height_and_width() {
    use super::document::LineBox;
    let owner = LineBox {
        left: 0.0,
        top: 0.0,
        right: 100.0,
        bottom: 10.0,
    };
    let at = |left: f32, top: f32| LineBox {
        left,
        top,
        right: left + 100.0,
        bottom: top + 10.0,
    };
    assert!(owner.owns(&at(0.0, 5.0)), "half the height shared");
    assert!(!owner.owns(&at(0.0, 5.5)), "less than half the height");
    assert!(owner.owns(&at(50.0, 0.0)), "half the width shared");
    assert!(!owner.owns(&at(51.0, 0.0)), "less than half the width: another column");
}

/// The row index bounds the matching work: with N layer lines and N OCR lines on N rows, each
/// layer line meets its band only, and the merge of 2000 lines takes a small part of a second
/// (67 s at the PR head). The band of a line on row 500 holds its row and its neighbours. ~keep
#[test]
fn should_bound_the_matching_work_to_the_band_of_each_layer_line() {
    use super::document::{OcrRowIndex, layer_lines_to_put_first};
    let thresholds = OcrQualityThresholds::default();
    let mut ms_by_n = Vec::new();
    for n in [250usize, 2000] {
        let layer: Vec<String> = (1..=n)
            .map(|k| format!("Harbour record {k} repairs accepted tonight"))
            .collect();
        let ocr: Vec<String> = (1..=n)
            .map(|k| format!("Harbour record {k} repalrs accepled tonight"))
            .collect();
        let rows: Vec<usize> = (0..n).collect();
        let layer_refs: Vec<&str> = layer.iter().map(String::as_str).collect();
        let ocr_text = ocr.join("\n");
        let boxes = page_boxes(&layer_refs, &lines_at_rows(&ocr_text, &rows));
        let index = OcrRowIndex::new(&boxes.ocr);
        let band: Vec<usize> = index.band(&row_box(n / 2)).collect();
        assert!(
            band.len() <= 3 && band.contains(&(n / 2)),
            "the band holds the row and its neighbours: {band:?}"
        );
        let start = std::time::Instant::now();
        let first = layer_lines_to_put_first(&layer.join("\n"), &ocr_text, false, &thresholds, Some(&boxes))
            .expect("every layer line is missing");
        let ms = start.elapsed().as_millis();
        println!(
            "SCALING n={n} missing={} copies={} ms={ms}",
            first.lines.lines().count(),
            first.ocr_copies.len()
        );
        assert_eq!(first.ocr_copies.len(), n, "every misread sits on its layer line's row");
        ms_by_n.push(ms);
    }
    assert!(ms_by_n[1] < 2000, "2000 lines in under two seconds: {ms_by_n:?} ms");
}
