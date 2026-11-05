use super::*;

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
    use super::super::pipeline::extract_mixed_ocr_native;
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
    use super::super::document::put_layers_first_in_run;
    use super::super::scoring::OcrPageNoiseVerdict;
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
        super::super::document::PageLayers {
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
    use super::super::pipeline::extract_mixed_ocr_native;
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
    use super::super::pipeline::{PageOcrHints, extract_full_document_ocr_pipeline_per_page};
    use crate::core::config::{ExtractionConfig, OcrConfig};
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
            pipeline: Some(zero_quality_pipeline(BACKEND)),
            ..Default::default()
        }),
        ..Default::default()
    };
    let hints = PageOcrHints {
        source_dpi: None,
        #[cfg(feature = "layout-detection")]
        raster_dpis: None,
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

#[test]
fn should_confirm_a_layer_when_the_ocr_text_reads_most_of_its_words() {
    use super::super::document::ocr_confirms_layer;

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
    use super::super::pipeline::extract_mixed_ocr_native;
    use crate::core::config::{ExtractionConfig, OcrConfig};
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
            pipeline: Some(zero_quality_pipeline(name)),
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
    use super::super::document::put_layers_first_in_run;
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
        super::super::document::PageLayers {
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
    use super::super::document::put_layers_first_in_run;
    use super::super::scoring::OcrPageNoiseVerdict;
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
        super::super::document::PageLayers {
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

#[test]
fn should_restore_native_rows_without_deleting_unproved_ocr_alternatives() {
    use super::super::document::{layer_lines_to_put_first, text_with_layer_first};
    let layer = HARBOUR_LAYER.join("\n");
    let misread: Vec<String> = HARBOUR_LAYER.iter().map(|line| misread_longest_word(line)).collect();
    let read = misread.join("\n");
    let boxes = page_boxes(&HARBOUR_LAYER, &lines_at_rows(&read, &[0, 1, 2, 3, 4, 5]));
    let first = layer_lines_to_put_first(&layer, &read, false, &OcrQualityThresholds::default(), Some(&boxes))
        .expect("all source rows lack complete literal OCR coverage");
    assert_eq!(first.lines, layer);
    assert_eq!(text_with_layer_first(&read, &first), format!("{layer}\n\n{read}"));
}

/// A whole-document run (#2439): a layer of numbers only. The OCR text reads three of the six
/// lines, so it confirms the layer and the other three lines come back. Numbers that the OCR text
/// does not hold, and too few numbers, never confirm a layer. ~keep
#[test]
fn should_confirm_a_number_only_layer_against_the_ocr_text() {
    use super::super::document::{ocr_confirms_layer, put_layers_first_in_run};
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
        super::super::document::PageLayers {
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
    use super::super::document::ocr_confirms_layer;

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

/// another layer line that differs from it by one word. The layer holds a line and a near twin of
/// it; the OCR text reads the first exactly and lacks the twin. The twin comes back, and the exact
/// OCR line stays. ~keep
#[test]
fn should_not_drop_an_exact_ocr_line_as_the_copy_of_its_near_twin_in_the_layer() {
    use super::super::document::layer_lines_to_put_first;

    let thresholds = OcrQualityThresholds::default();
    let twin = misread_longest_word(HARBOUR_LAYER[0]);
    let layer = format!("{}\n{twin}", HARBOUR_LAYER.join("\n"));
    let ocr_text = HARBOUR_LAYER[..4].join("\n");
    // The twin sits on the first line's row: it owns the exact OCR line's place. ~keep
    let mut boxes = page_boxes(&HARBOUR_LAYER, &lines_at_rows(&ocr_text, &[0, 1, 2, 3]));
    boxes.layer.push(super::super::document::LineAt::new(&twin, row_box(0)));
    let first = layer_lines_to_put_first(&layer, &ocr_text, false, &thresholds, Some(&boxes))
        .expect("the OCR text confirms the layer, and the twin and two more lines are absent from it");
    assert!(
        first.lines.lines().any(|line| line == twin),
        "the twin comes back: {:?}",
        first.lines
    );
    assert!(super::super::document::text_with_layer_first(&ocr_text, &first).ends_with(&ocr_text));
}

/// The issue's route (#2439), through the extractor in automatic mode: the language check flags a
/// page whose layer is correct, OCR reads ten of its fifteen lines plus one line the layer lacks,
/// and the OCR text replaces the page. The content and the page text keep every word of the
/// layer, and the OCR line joins them. ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_keep_every_layer_word_of_a_page_the_language_check_sends_to_ocr() {
    use super::super::plausibility::{PlausibilityVerdict, evaluate_text_plausibility};
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
    use super::super::pipeline::{PageOcrHints, run_ocr_pipeline_for_page};
    use crate::core::config::{ExtractionConfig, OcrConfig};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-accepted-stage-backend";
    crate::plugins::register_ocr_backend(Arc::new(StageBackend {
        name: BACKEND,
        text: HARBOUR_PARTIAL_READ,
        noisy: false,
    }))
    .unwrap();
    let pipeline = zero_quality_pipeline(BACKEND);
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
                ..Default::default()
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
