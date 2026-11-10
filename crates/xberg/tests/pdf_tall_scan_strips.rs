#![cfg(all(
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, ExtractionMethod, extract as extract_public};

const TALL_JPEG: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/jpeg-tall.pdf");
const MIXED_JPEG: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/jpeg-mixed.pdf");
const TALL: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/strict-tall.pdf");
const BILEVEL: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/bilevel-tall.pdf");
const TALL_PREDICTOR: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/predictor-mixed.pdf");
const NEUTRAL_RGB: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/neutral-rgb-tall.pdf");
const MIXED: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/strict-mixed.pdf");
const SHORT: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/strict-short.pdf");
const ORACLE: &str = include_str!("../../../fixtures/repros/tall-scan-strips/expected.txt");
const STRIP_WARNING: &str = "source-image strips.";

fn extraction_config() -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "ocr": {"backend": "tesseract", "language": ["eng"]},
        "pages": {"extract_pages": true},
        "concurrency": {"max_threads": 1},
    }))
    .expect("one-thread Tesseract configuration")
}

fn extract(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("integration runtime");
    let mut result = runtime
        .block_on(extract_public(
            ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
            config,
        ))
        .expect("extract with the real Tesseract backend");
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(result.results.len(), 1);
    result.results.pop().expect("one extracted document")
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect()
}

fn strip_warning(document: &ExtractedDocument) -> bool {
    document
        .processing_warnings
        .iter()
        .any(|warning| warning.message.contains(STRIP_WARNING))
}

fn assert_image_text(document: &ExtractedDocument, used_strips: bool) {
    let expected_words = words(ORACLE);
    assert_eq!(expected_words.len(), 440);
    assert_eq!(ORACLE.lines().count(), 40);
    assert_eq!(ORACLE.lines().collect::<std::collections::HashSet<_>>().len(), 40);
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Ocr));
    assert!(document.metadata.ocr_used);
    assert_eq!(strip_warning(document), used_strips);
    assert_eq!(words(&document.content), expected_words);
    let pages = document.pages.as_ref().expect("one original PDF page");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    assert_eq!(words(&pages[0].content), expected_words);
    if used_strips {
        let confidence = pages[0].ocr_confidence.as_ref().expect("one page confidence summary");
        assert_eq!(confidence.word_count, 440);
        assert_eq!(confidence.backend, "tesseract");
    }
}

#[test]
fn tall_scan_recovers_every_source_line_once_in_order() {
    let document = extract(TALL, &extraction_config());
    assert_image_text(&document, true);
    assert_eq!(
        document.metadata.additional.get("ocr_page_segmentation_modes"),
        Some(&serde_json::json!([{ "page_number": 1, "psm": 11 }]))
    );
}

#[test]
fn bilevel_tall_scan_recovers_every_source_line_once_in_order() {
    assert_bilevel_text(&extract(BILEVEL, &extraction_config()));
}

fn assert_bilevel_text(document: &ExtractedDocument) {
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Ocr));
    assert!(strip_warning(document));
    let pages = document.pages.as_ref().unwrap();
    let [page] = pages.as_slice() else {
        panic!("expected one page, got {}", pages.len());
    };
    assert_eq!(page.page_number, 1);
    assert_eq!(page.ocr_confidence.as_ref().unwrap().word_count, 440);
    let expected = words(ORACLE);
    for actual in [words(&document.content), words(&page.content)] {
        assert_eq!(actual.len(), expected.len());
        let (rows, remainder) = actual.as_chunks::<11>();
        assert!(remainder.is_empty());
        assert_eq!(rows.len(), 40);
        for (row, expected_row) in rows.iter().zip(expected.chunks(11)) {
            assert_eq!(&row[..10], &expected_row[..10]);
        }
        let exact_tags = rows
            .iter()
            .zip(expected.chunks(11))
            .filter(|(row, expected_row)| row[10] == expected_row[10])
            .count();
        assert!(exact_tags >= 36, "only {exact_tags} of 40 distinct row tags recognized");
    }
}

#[test]
fn in_cap_scan_keeps_original_ocr_route_and_text() {
    assert_image_text(&extract(SHORT, &extraction_config()), false);
}

#[test]
fn neutral_rgb_tall_scan_recovers_every_source_line_once_in_order() {
    assert_image_text(&extract(NEUTRAL_RGB, &extraction_config()), true);
}

#[test]
fn mixed_pdf_preserves_the_native_page_and_recovers_the_tall_scan() {
    let document = extract(MIXED, &extraction_config());
    let native = "The native page remains unchanged.\n".repeat(10);
    let pages = document.pages.as_ref().expect("two original PDF pages");
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Mixed));
    assert!(document.metadata.ocr_used);
    assert!(strip_warning(&document));
    assert_eq!(pages.len(), 2);
    assert_eq!((pages[0].page_number, pages[1].page_number), (1, 2));
    assert_eq!(words(&pages[0].content), words(&native));
    assert_eq!(words(&pages[1].content), words(ORACLE));
    assert_eq!(words(&document.content), words(&format!("{native}{ORACLE}")));
    let confidence = pages[1].ocr_confidence.as_ref().expect("mixed OCR page confidence");
    assert_eq!(confidence.word_count, 440);
    assert_eq!(confidence.backend, "tesseract");
}

#[test]
fn disabling_ocr_leaves_the_tall_scan_unrecognized() {
    let result = extract(
        TALL,
        &ExtractionConfig {
            disable_ocr: true,
            ..extraction_config()
        },
    );
    assert!(result.content.is_empty());
    assert!(!result.metadata.ocr_used);
    assert!(!strip_warning(&result));
}

#[test]
fn low_per_page_content_limit_keeps_existing_render_fallback() {
    let mut config = extraction_config();
    config
        .security_limits
        .get_or_insert_with(Default::default)
        .max_content_size = 32 * 1024 * 1024;
    let document = extract(TALL, &config);
    assert!(!strip_warning(&document));
    assert_eq!(document.pages.as_ref().expect("original page record").len(), 1);
}

#[test]
fn tall_gray_jpeg_recovers_every_source_word_after_render_budget_refusal() {
    assert_image_text(&extract(TALL_JPEG, &extraction_config()), true);
}

#[test]
fn tall_gray_jpeg_keeps_the_existing_refusal_for_forced_or_very_low_budget_ocr() {
    let forced = ExtractionConfig {
        force_ocr: true,
        ..extraction_config()
    };
    let mut low = extraction_config();
    low.security_limits
        .get_or_insert_with(Default::default)
        .max_content_size = 16 * 1024 * 1024;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for config in [forced, low] {
        let input = ExtractInput::from_bytes(TALL_JPEG.to_vec(), "application/pdf", None);
        let error = runtime
            .block_on(extract_public(input, &config))
            .expect_err("existing render refusal");
        let xberg::XbergError::Parsing {
            source: Some(source), ..
        } = error
        else {
            panic!("expected typed Native render error, got {error:?}");
        };
        let native = source.downcast_ref::<xberg_native_pdf::Error>().or_else(|| {
            source
                .source()
                .and_then(|cause| cause.downcast_ref::<xberg_native_pdf::Error>())
        });
        assert!(matches!(native, Some(xberg_native_pdf::Error::Image(_))));
    }
}

/// A single-page scan takes the whole-document route, which reads no embedded image after a render
/// refusal: Djot output has no strip recovery, so it keeps the refusal as it is.
#[test]
fn tall_gray_jpeg_keeps_the_render_refusal_under_djot_output() {
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "output_format": "djot",
        "ocr": {"backend": "tesseract", "language": ["eng"]},
    }))
    .expect("Djot configuration that bypasses the strip recovery");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let outcome = runtime.block_on(extract_public(
        ExtractInput::from_bytes(TALL_JPEG.to_vec(), "application/pdf", None),
        &config,
    ));
    let error = match outcome {
        Err(error) => error,
        Ok(result) => panic!(
            "the refused page must not be reported as read: {:?}",
            result
                .results
                .first()
                .map(|document| document.content.chars().take(120).collect::<String>())
        ),
    };
    let xberg::XbergError::Parsing {
        message,
        source: Some(source),
    } = error
    else {
        panic!("expected the typed render refusal, got {error:?}");
    };
    assert!(message.contains("image working budget"), "{message}");
    let native = source.downcast_ref::<xberg_native_pdf::Error>().or_else(|| {
        source
            .source()
            .and_then(|cause| cause.downcast_ref::<xberg_native_pdf::Error>())
    });
    assert!(matches!(native, Some(xberg_native_pdf::Error::Image(_))));
}

/// The retry of a refused page on its embedded image runs on the mixed route, where a native page
/// precedes the scan. The dimension clamp leaves the read illegible, so the page stays unread and
/// its failure names the noise instead of reporting the read as the page text.
fn assert_noisy_retry_read_is_a_page_failure(config: &ExtractionConfig) {
    let document = extract(MIXED_JPEG, config);
    let native = "The native page remains unchanged.\n".repeat(10);
    assert_eq!(words(&document.content), words(&native));
    let pages = document.pages.as_ref().expect("two original PDF pages");
    assert_eq!(pages.len(), 2);
    assert!(pages[1].content.trim().is_empty(), "{:?}", pages[1].content);
    let failures = document
        .ocr_page_failures
        .as_ref()
        .expect("the refused page is recorded");
    let [failure] = failures.as_slice() else {
        panic!("expected one page failure, got {failures:?}");
    };
    assert_eq!(failure.page, 2);
    assert!(!failure.recovered);
    for part in ["image working budget", "the retry read was rejected as noise"] {
        assert!(failure.error.contains(part), "{part}: {}", failure.error);
    }
}

#[test]
fn tall_gray_jpeg_page_read_as_noise_is_reported_as_a_failure_not_adopted() {
    assert_noisy_retry_read_is_a_page_failure(&extraction_config());
}

/// An explicit pipeline takes its own retry on the refused page, with its own noise check.
#[test]
fn explicit_pipeline_reports_a_noisy_retry_read_as_a_page_failure_too() {
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "ocr": {
            "backend": "tesseract",
            "language": ["eng"],
            "pipeline": {"stages": [{"backend": "tesseract", "backend_options": {"use_cache": false}}]},
        },
        "pages": {"extract_pages": true},
        "concurrency": {"max_threads": 1},
    }))
    .expect("one-thread explicit-pipeline Tesseract configuration");
    assert_noisy_retry_read_is_a_page_failure(&config);
}

fn assert_exact_tall_source_text(source: &[u8]) {
    assert_exact_tall_source_text_with(source, &extraction_config());
}

fn assert_exact_tall_source_text_with(source: &[u8], config: &ExtractionConfig) {
    let document = extract(source, config);
    assert_image_text(&document, true);
    assert!(document.content.split_whitespace().eq(ORACLE.split_whitespace()));
    let pages = document.pages.as_ref().unwrap();
    assert!(pages[0].content.split_whitespace().eq(ORACLE.split_whitespace()));
}

#[test]
fn tall_png_predictor_recovers_all_source_words_in_order() {
    assert_exact_tall_source_text(TALL_PREDICTOR);
}

#[test]
fn tall_png_predictor_recovers_all_source_words_with_default_parameters() {
    let full = b"/DecodeParms << /Predictor 12 /Columns 1275 /Colors 1 /BitsPerComponent 8 >>";
    let short = b"/DecodeParms << /Predictor 12 /Columns 1275 >>";
    let positions = TALL_PREDICTOR
        .windows(full.len())
        .enumerate()
        .filter_map(|(index, window)| (window == full).then_some(index))
        .collect::<Vec<_>>();
    let [start] = positions.as_slice() else {
        panic!("the predictor fixture must have exactly one matching DecodeParms dictionary");
    };
    let mut abbreviated = TALL_PREDICTOR.to_vec();
    abbreviated[*start..*start + short.len()].copy_from_slice(short);
    abbreviated[*start + short.len()..*start + full.len()].fill(b' ');
    assert_exact_tall_source_text(&abbreviated);
}

#[test]
fn tall_lower_density_png_predictor_recovers_all_source_words_in_order() {
    assert_exact_tall_source_text(&lower_density_predictor());
}

fn lower_density_predictor() -> Vec<u8> {
    lower_density(TALL_PREDICTOR, (765, 7200))
}

/// Keep the exact encoded image and xref offsets; only the page and paint size change.
fn lower_density(source: &[u8], (width, height): (u32, u32)) -> Vec<u8> {
    let swaps = [
        ("/MediaBox [0 0 612 5760]", format!("/MediaBox [0 0 {width} {height}]")),
        ("q 612 0 0 5760 0 0 cm", format!("q {width} 0 0 {height} 0 0 cm")),
    ];
    let mut lower_density = source.to_vec();
    for (old, new) in swaps {
        let (old, new) = (old.as_bytes(), new.as_bytes());
        assert_eq!(old.len(), new.len());
        let mut positions = source
            .windows(old.len())
            .enumerate()
            .filter_map(|(index, window)| (window == old).then_some(index));
        let start = positions.next().expect("exact source geometry");
        assert!(positions.next().is_none());
        lower_density[start..start + new.len()].copy_from_slice(new);
    }
    lower_density
}

#[test]
fn tall_lower_density_plain_gray_flate_recovers_all_source_words_in_order() {
    const SOURCE_PIXELS: usize = 1275 * 12000;
    let mut pdf = lopdf::Document::load_mem(&lower_density_predictor()).expect("read exact synthetic source");
    let stream = pdf
        .get_object_mut((5, 0))
        .and_then(lopdf::Object::as_stream_mut)
        .expect("the source image stream");
    assert_eq!(stream.dict.get(b"Subtype").unwrap().as_name().unwrap(), b"Image");
    let samples = stream
        .get_plain_content_with_limit(SOURCE_PIXELS + 12000)
        .expect("decode the exact predictor source samples");
    assert_eq!(samples.len(), SOURCE_PIXELS);
    stream.set_plain_content(samples);
    stream.compress().expect("losslessly encode without prediction");
    assert_eq!(stream.dict.get(b"Filter").unwrap().as_name().unwrap(), b"FlateDecode");
    assert!(stream.dict.get(b"DecodeParms").is_err());
    let mut plain = Vec::new();
    pdf.save_to(&mut plain).expect("serialize the unpredicted source");
    assert_exact_tall_source_text(&plain);
}

#[test]
fn tall_inverted_indexed_scan_recovers_all_source_words_in_order() {
    let source = include_bytes!("../../../fixtures/repros/tall-scan-strips/indexed-inverted.pdf");
    assert_exact_tall_source_text(source);
}

const BELOW_FLOOR_DENSITY: (u32, u32) = (918, 8640);

fn configured(extra: serde_json::Value) -> ExtractionConfig {
    let mut value = serde_json::to_value(extraction_config()).expect("serialize base configuration");
    let (base, extra) = (value.as_object_mut().unwrap(), extra.as_object().unwrap());
    for (key, field) in extra {
        match (base.get_mut(key), field) {
            (Some(serde_json::Value::Object(base)), serde_json::Value::Object(field)) => base.extend(field.clone()),
            _ => {
                base.insert(key.clone(), field.clone());
            }
        }
    }
    serde_json::from_value(value).expect("merged configuration")
}

macro_rules! fixture_120 {
    ($name:literal) => {
        include_bytes!(concat!("../../../fixtures/repros/tall-scan-strips/", $name)).as_slice()
    };
}

#[test]
fn indexed_scan_rendered_at_120_dpi_recovers_every_source_word() {
    assert_exact_tall_source_text(fixture_120!("indexed-120.pdf"));
}

#[test]
fn bilevel_rgb_and_jpeg_scans_rendered_at_120_dpi_keep_the_standard_floor_and_say_so() {
    for source in [
        fixture_120!("bilevel-120.pdf"),
        fixture_120!("neutral-rgb-120.pdf"),
        fixture_120!("jpeg-120.pdf"),
    ] {
        let document = extract(source, &extraction_config());
        assert!(!strip_warning(&document));
        assert!(
            document
                .processing_warnings
                .iter()
                .any(|warning| warning.message.contains("source-image strips were not used")),
            "{:?}",
            document.processing_warnings
        );
    }
}

#[test]
fn tall_scans_below_the_density_floor_say_recovery_was_not_used() {
    for source in [TALL_PREDICTOR, NEUTRAL_RGB, BILEVEL] {
        let document = extract(&lower_density(source, BELOW_FLOOR_DENSITY), &extraction_config());
        assert!(!strip_warning(&document));
        assert!(
            document
                .processing_warnings
                .iter()
                .any(|warning| warning.message.contains("source-image strips were not used")),
            "{:?}",
            document.processing_warnings
        );
    }
}

#[test]
fn markdown_output_recovers_the_tall_scan_in_every_certified_encoding() {
    let config = configured(serde_json::json!({"output_format": "markdown"}));
    for source in [TALL, NEUTRAL_RGB, TALL_JPEG, TALL_PREDICTOR] {
        assert_exact_tall_source_text_with(source, &config);
    }
}

#[test]
fn strips_join_as_separate_paragraphs_and_markdown_reports_flat_text() {
    let plain = extract(TALL, &extraction_config());
    assert!(strip_warning(&plain));
    assert_eq!(plain.content.split("\n\n").count(), 40, "{:?}", plain.content);
    let markdown = extract(TALL, &configured(serde_json::json!({"output_format": "markdown"})));
    assert!(strip_warning(&markdown));
    assert!(
        markdown
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("Markdown structure")),
        "{:?}",
        markdown.processing_warnings
    );
    assert!(
        !plain
            .processing_warnings
            .iter()
            .any(|w| w.message.contains("Markdown structure"))
    );
}

#[test]
fn markdown_output_with_an_uncertifiable_jpeg_keeps_the_typed_render_refusal() {
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "output_format": "markdown",
        "ocr": {"backend": "tesseract", "language": ["eng"]},
    }))
    .expect("Markdown configuration");
    let below_floor = lower_density(TALL_JPEG, BELOW_FLOOR_DENSITY);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let error = runtime
        .block_on(extract_public(
            ExtractInput::from_bytes(below_floor, "application/pdf", None),
            &config,
        ))
        .expect_err("a JPEG below the density floor cannot be certified");
    let xberg::XbergError::Parsing {
        source: Some(source), ..
    } = error
    else {
        panic!("expected the typed render refusal, got {error:?}");
    };
    let native = source.downcast_ref::<xberg_native_pdf::Error>().or_else(|| {
        source
            .source()
            .and_then(|cause| cause.downcast_ref::<xberg_native_pdf::Error>())
    });
    assert!(matches!(native, Some(xberg_native_pdf::Error::Image(_))));
}

#[test]
fn an_explicit_tesseract_page_mode_recovers_the_tall_scan() {
    let config = configured(serde_json::json!({"ocr": {"tesseract_config": {"psm": 3}}}));
    for source in [TALL, NEUTRAL_RGB, TALL_PREDICTOR] {
        let document = extract(source, &config);
        assert_image_text(&document, true);
        assert_eq!(
            document.metadata.additional.get("ocr_page_segmentation_modes"),
            Some(&serde_json::json!([{ "page_number": 1, "psm": 3 }]))
        );
    }
}

#[test]
fn a_configuration_that_blocks_strip_recovery_is_named_in_the_result() {
    let config = configured(serde_json::json!({"include_document_structure": true}));
    let document = extract(TALL, &config);
    assert!(!strip_warning(&document));
    let warning = document
        .processing_warnings
        .iter()
        .find(|warning| warning.message.contains("source-image strips were not used"))
        .expect("the skipped recovery is reported");
    assert!(warning.message.contains("document structure"), "{}", warning.message);
}

fn not_used_warnings(document: &ExtractedDocument) -> Vec<&str> {
    document
        .processing_warnings
        .iter()
        .map(|warning| warning.message.as_ref())
        .filter(|message| message.contains("source-image strips were not used"))
        .collect()
}

/// The mixed route retries nothing for a scan its configuration keeps from strip recovery, and says why.
#[test]
fn the_mixed_route_names_the_configuration_that_blocked_strip_recovery() {
    let blocked = extract(
        MIXED,
        &configured(serde_json::json!({"include_document_structure": true})),
    );
    assert_eq!(blocked.extraction_method, Some(ExtractionMethod::Mixed));
    assert!(!strip_warning(&blocked));
    assert_eq!(
        not_used_warnings(&blocked),
        vec![
            "Page 2 was read from a render downscaled to 130 DPI and its source-image strips were not used: \
             per-element results or document structure were requested. The text may be incomplete."
        ]
    );
    let allowed = extract(MIXED, &extraction_config());
    assert!(strip_warning(&allowed));
    assert_eq!(not_used_warnings(&allowed), Vec::<&str>::new());
}

/// A layout request whose pass soft-failed (its models are unavailable) has no detections to lose:
/// strip recovery stays on. The extraction runs in a child process with an empty offline model
/// cache, so the cache variables reach no other test.
#[cfg(feature = "layout-detection")]
#[test]
fn a_layout_request_whose_models_are_unavailable_still_recovers_the_tall_scan() {
    const CHILD: &str = "XBERG_TEST_LAYOUT_UNAVAILABLE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let empty_cache = tempfile::tempdir().expect("empty model cache");
        let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "a_layout_request_whose_models_are_unavailable_still_recovers_the_tall_scan",
            ])
            .args(["--test-threads=1", "--nocapture"])
            .env(CHILD, "1")
            .env_remove("HUGGINGFACE_HUB_CACHE")
            .env("HF_HOME", empty_cache.path())
            .env("HF_HUB_CACHE", empty_cache.path())
            .env("HF_HUB_OFFLINE", "1")
            .output()
            .expect("run the child test");
        let report = String::from_utf8_lossy(&output.stdout).into_owned() + &String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{report}");
        assert!(
            report.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
            "the child test did not run: {report}"
        );
        return;
    }
    let document = extract(TALL, &configured(serde_json::json!({"layout": {}})));
    assert!(
        document
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "layout" && warning.message.contains("layout detection failed")),
        "{:?}",
        document.processing_warnings
    );
    assert_image_text(&document, true);
    assert_eq!(not_used_warnings(&document), Vec::<&str>::new());
}

/// The render-failure fallback has no layout detections in scope, so a layout request alone keeps it
/// from replacing the refusal with a source-image read.
#[cfg(feature = "layout-detection")]
#[test]
fn a_layout_request_keeps_the_gray_jpeg_render_refusal() {
    let config = configured(serde_json::json!({"layout": {}}));
    let document = extract(TALL_JPEG, &config);
    assert!(!strip_warning(&document));
    assert!(
        words(&document.content).len() < words(ORACLE).len() / 2,
        "{:?}",
        document.content.chars().take(80).collect::<String>()
    );
    assert!(
        document
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "layout" && warning.message.contains("image working budget")),
        "{:?}",
        document.processing_warnings
    );
}
