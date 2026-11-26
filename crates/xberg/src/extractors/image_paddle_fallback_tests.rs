use super::*;
use crate::core::extractor::cacheability;
use crate::plugins::{OcrBackendType, Plugin};
use crate::types::{OcrBoundingGeometry, OcrConfidence, OcrElement};
use std::sync::atomic::{AtomicUsize, Ordering};

fn word(text: &str, recognition: f64) -> OcrElement {
    OcrElement {
        text: text.into(),
        geometry: OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width: 10,
            height: 10,
        },
        confidence: OcrConfidence {
            detection: None,
            recognition,
        },
        level: OcrElementLevel::Word,
        rotation: None,
        page_number: 1,
        parent_id: None,
        backend_metadata: Default::default(),
    }
}

/// A Tesseract read of `words` words: `confident` plausible numbers read at 0.9, the rest
/// misread fragments at 0.2.
fn tesseract_read(words: usize, confident: usize) -> ExtractedDocument {
    let words: Vec<(String, f64)> = (0..words)
        .map(|index| {
            if index < confident {
                (format!("{}", 1000 + index), 0.9)
            } else {
                (format!("x{index}"), 0.2)
            }
        })
        .collect();
    ExtractedDocument {
        content: words
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        ocr_elements: Some(words.iter().map(|(text, conf)| word(text, *conf)).collect()),
        ..Default::default()
    }
}

/// A PaddleOCR read of `plausible` receipt words and amounts plus `implausible` fragments, two
/// tokens a line.
fn paddle_read(plausible: usize, implausible: usize) -> ExtractedDocument {
    let tokens: Vec<String> = (0..plausible)
        .map(|index| {
            if index % 2 == 0 {
                "total".to_string()
            } else {
                format!("{}500", index)
            }
        })
        .chain((0..implausible).map(|index| format!("qx{index}")))
        .collect();
    ExtractedDocument {
        content: tokens
            .chunks(2)
            .map(|line| line.join(" "))
            .collect::<Vec<_>>()
            .join("\n"),
        ..Default::default()
    }
}

/// Measured on the benchmark photos (2026-11-26, `ocr-images-fast`): usable Tesseract words,
/// plausible tokens among Tesseract words read at >= 0.70, plausible and all tokens of the
/// PaddleOCR text. Word F1 against ground truth, Tesseract -> PaddleOCR: .51->.95, .00->1.00,
/// .43->.83, .49->1.00, .24->.80, .00->.67.
const MEASURED_PHOTOS: &[(&str, usize, usize, usize, usize)] = &[
    ("cord_receipt_01", 34, 13, 28, 34),
    ("cord_receipt_02", 13, 3, 11, 16),
    ("cord_receipt_03", 27, 10, 25, 29),
    ("cord_receipt_04", 35, 12, 14, 17),
    ("textocr_scene_01", 38, 7, 50, 61),
    ("textocr_scene_03", 40, 1, 34, 37),
];

#[test]
fn the_measured_photos_switch_to_the_paddle_read() {
    for &(name, words, trusted, plausible, all) in MEASURED_PHOTOS {
        let primary = tesseract_read(words, trusted);
        assert!(sparse_unreliable_read(&primary), "{name}: the photo must be eligible");
        assert_eq!(trusted_tesseract_tokens(&primary), trusted, "{name}");
        let candidate = paddle_read(plausible, all - plausible);
        assert_eq!(plausible_tokens(&candidate.content), plausible, "{name}");
        assert_eq!(tokens(&candidate.content).count(), all, "{name}");
        assert!(
            paddle_read_wins(&primary, &candidate, &OcrQualityThresholds::default()),
            "{name}"
        );
    }
}

#[test]
fn plausible_tokens_are_words_and_numbers_not_fragments() {
    for token in ["total", "CASH", "91000", "Widget", "Ärger", "12"] {
        assert!(is_plausible_token(token), "{token}");
    }
    for token in ["x", "1", "rn", "CoE7", "x7Q", "sss", "aaaaaaaaaaaaaaaaaaaaaaaaa"] {
        assert!(!is_plausible_token(token), "{token}");
    }
}

#[test]
fn the_word_bound_is_inclusive_at_80() {
    for (words, eligible) in [(80, true), (81, false)] {
        let primary = tesseract_read(words, words / 2);
        assert!(!has_robust_word_confidence_distribution(&primary), "{words}");
        assert_eq!(sparse_unreliable_read(&primary), eligible, "{words}");
    }
}

#[test]
fn a_dense_scan_with_rule_line_noise_is_never_read_again() {
    // 150 words, a third of them rule-line noise: unreliable, but not photo-shaped. ~keep
    let dense = tesseract_read(150, 100);
    assert!(!has_robust_word_confidence_distribution(&dense));
    assert!(!sparse_unreliable_read(&dense));
}

#[test]
fn a_paddle_read_that_loses_tesseract_words_loses() {
    let primary = tesseract_read(70, 40);
    let thresholds = OcrQualityThresholds::default();
    assert!(!paddle_read_wins(&primary, &paddle_read(39, 0), &thresholds));
    assert!(paddle_read_wins(&primary, &paddle_read(40, 0), &thresholds));
}

#[test]
fn a_read_without_text_is_never_read_again() {
    assert!(!sparse_unreliable_read(&tesseract_read(0, 0)));
}

#[test]
fn detector_noise_never_becomes_the_text() {
    let thresholds = OcrQualityThresholds::default();
    let primary = tesseract_read(5, 0);
    assert!(sparse_unreliable_read(&primary));

    assert!(
        !paddle_read_wins(&primary, &paddle_read(3, 0), &thresholds),
        "too few tokens"
    );
    assert!(paddle_read_wins(&primary, &paddle_read(4, 0), &thresholds));
    assert!(
        !paddle_read_wins(&primary, &paddle_read(6, 5), &thresholds),
        "mostly fragments (6 of 11 plausible)"
    );
    assert!(
        paddle_read_wins(&primary, &paddle_read(6, 3), &thresholds),
        "6 of 9 plausible"
    );

    // Sixty repeats of one short word: plausible tokens, but fragmented and repetitive. ~keep
    let noise = ExtractedDocument {
        content: ["on on on on on on on on on on"; 6].join("\n"),
        ..Default::default()
    };
    assert!(
        crate::extractors::pdf::ocr::compute_quality_score(&noise.content, &thresholds)
            < thresholds.pipeline_min_quality,
        "the fixture must fail the default quality gate"
    );
    assert!(plausible_tokens(&noise.content) >= MIN_PLAUSIBLE_TOKENS);
    assert!(
        !paddle_read_wins(&primary, &noise, &thresholds),
        "below the quality gate"
    );

    let blank = ExtractedDocument {
        content: "  \n ".to_string(),
        ..Default::default()
    };
    assert!(!paddle_read_wins(&primary, &blank, &thresholds), "empty text");
}

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 80)
        .encode_image(&image::DynamicImage::ImageRgb8(image::RgbImage::new(width, height)))
        .expect("test JPEG encode");
    bytes
}

#[test]
fn only_camera_photos_qualify() {
    let photo = jpeg(960, 1280);
    assert!(is_camera_photo(&photo, "image/jpeg"));
    assert!(is_camera_photo(&photo, "IMAGE/JPEG"));
    for mime in ["image/png", "image/bmp", "image/gif", "image/tiff"] {
        assert!(!is_camera_photo(&photo, mime), "{mime}");
    }
    // US Letter at 150 dpi: a page scan saved as JPEG. ~keep
    assert!(!is_camera_photo(&jpeg(1275, 1650), "image/jpeg"));
    assert!(!is_camera_photo(b"not an image", "image/jpeg"));
}

fn default_extraction() -> ExtractionConfig {
    ExtractionConfig::default()
}

#[test]
fn the_default_tesseract_request_is_eligible() {
    assert!(eligible(&default_extraction(), &OcrConfig::default(), "image/jpeg"));
    let cache_only = OcrConfig {
        backend_options: Some(serde_json::json!({"use_cache": false})),
        ..Default::default()
    };
    assert!(eligible(&default_extraction(), &cache_only, "image/jpeg"));
}

#[test]
fn any_tesseract_config_is_an_explicit_choice() {
    for json in [
        r#"{}"#,
        r#"{"language":["eng"]}"#,
        r#"{"use_cache":false}"#,
        r#"{"psm":11}"#,
    ] {
        let ocr = OcrConfig {
            tesseract_config: Some(serde_json::from_str(json).expect("valid tesseract_config")),
            ..Default::default()
        };
        assert!(!eligible(&default_extraction(), &ocr, "image/jpeg"), "{json}");
    }
    let tessdata = OcrConfig {
        tessdata_path: Some("/opt/tessdata".into()),
        ..Default::default()
    };
    let bytes = OcrConfig {
        tessdata_bytes: Some(Default::default()),
        ..Default::default()
    };
    let numeric = OcrConfig {
        numeric_repair: true,
        ..Default::default()
    };
    for (name, ocr) in [
        ("tessdata_path", tessdata),
        ("tessdata_bytes", bytes),
        ("numeric_repair", numeric),
    ] {
        assert!(!eligible(&default_extraction(), &ocr, "image/jpeg"), "{name}");
    }
}

#[test]
fn an_explicit_other_backend_is_never_replaced() {
    for backend in ["paddle-ocr", "paddleocr", "sceptre", "vlm", "custom-ocr"] {
        let ocr = OcrConfig {
            backend: backend.to_string(),
            ..Default::default()
        };
        assert!(
            !eligible(&default_extraction(), &ocr, "image/jpeg"),
            "backend: {backend}"
        );
    }
}

#[test]
fn the_option_turns_the_fallback_off() {
    let ocr = OcrConfig {
        image_paddle_fallback: false,
        ..Default::default()
    };
    assert!(!eligible(&default_extraction(), &ocr, "image/jpeg"));
}

#[test]
fn multi_backend_policies_layout_languages_and_tiff_keep_the_tesseract_read() {
    let options = OcrConfig {
        backend_options: Some(serde_json::json!({"mode": "fast"})),
        ..Default::default()
    };
    let pipeline = OcrConfig {
        pipeline: Some(crate::core::config::OcrPipelineConfig {
            stages: vec![],
            quality_thresholds: Default::default(),
        }),
        ..Default::default()
    };
    let two_languages = OcrConfig {
        language: vec!["eng".to_string(), "deu".to_string()],
        ..Default::default()
    };
    for (name, ocr) in [
        ("backend_options", options),
        ("pipeline", pipeline),
        ("two languages", two_languages),
    ] {
        assert!(!eligible(&default_extraction(), &ocr, "image/jpeg"), "{name}");
    }
    assert!(!eligible(&default_extraction(), &OcrConfig::default(), "image/tiff"));
    #[cfg(feature = "layout-types")]
    {
        let layout = ExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        };
        assert!(!eligible(&layout, &OcrConfig::default(), "image/jpeg"));
    }
}

#[test]
fn an_adopted_paddle_read_skips_the_tesseract_recoveries() {
    let paddle: Arc<dyn OcrBackend> = MockPaddle::new("paddle-ocr", None);
    assert_eq!(tesseract_recoveries(None, true, Some("title")), (true, Some("title")));
    assert_eq!(tesseract_recoveries(None, false, None::<&str>), (false, None));
    assert_eq!(tesseract_recoveries(Some(&paddle), true, Some("title")), (false, None));
}

struct MockPaddle {
    name: &'static str,
    read: Option<ExtractedDocument>,
    calls: AtomicUsize,
}

impl MockPaddle {
    fn new(name: &'static str, read: Option<ExtractedDocument>) -> Arc<Self> {
        Arc::new(Self {
            name,
            read,
            calls: AtomicUsize::new(0),
        })
    }
}

impl Plugin for MockPaddle {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for MockPaddle {
    async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        assert_eq!(config.backend, PADDLE_BACKEND);
        assert!(config.tesseract_config.is_none());
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.read.clone().expect("this case must not read the image again"))
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::PaddleOCR
    }
}

async fn run(
    paddle: Option<Arc<MockPaddle>>,
    primary: ExtractedDocument,
    models: bool,
) -> (ExtractedDocument, bool, bool) {
    let config = OcrConfig::default();
    cacheability::scope(async {
        let (selected, backend) = retry_with(
            paddle.map(|paddle| paddle as Arc<dyn OcrBackend>),
            Arc::new(Vec::new()),
            (&config, &config),
            None,
            primary,
            async |_: &OcrConfig| models,
        )
        .await
        .unwrap();
        (selected, backend.is_some(), cacheability::is_cacheable())
    })
    .await
}

#[tokio::test]
async fn without_a_paddle_backend_the_primary_read_is_returned_unchanged() {
    let primary = tesseract_read(13, 3);
    let (selected, adopted, cacheable) = run(None, primary.clone(), true).await;
    assert_eq!(selected.content, primary.content);
    assert!(!adopted);
    assert!(cacheable);
}

#[tokio::test]
async fn a_robust_or_dense_or_empty_read_is_never_read_again_and_stays_cacheable() {
    for primary in [tesseract_read(3, 3), tesseract_read(150, 100), tesseract_read(0, 0)] {
        let paddle = MockPaddle::new("paddle-ocr", None);
        let (selected, adopted, cacheable) = run(Some(Arc::clone(&paddle)), primary.clone(), true).await;
        assert_eq!(selected.content, primary.content);
        assert!(!adopted);
        assert!(cacheable);
        assert_eq!(paddle.calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn missing_models_keep_the_tesseract_read_and_bypass_the_extraction_cache() {
    let primary = tesseract_read(13, 3);
    let paddle = MockPaddle::new("paddle-ocr", None);
    let (selected, adopted, cacheable) = run(Some(Arc::clone(&paddle)), primary.clone(), false).await;
    assert_eq!(selected.content, primary.content);
    assert!(!adopted);
    assert!(
        !cacheable,
        "a cold model cache must not publish a result a warm one would not"
    );
    assert_eq!(paddle.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn an_adopted_paddle_read_is_marked_and_bypasses_the_extraction_cache() {
    let paddle = MockPaddle::new("paddle-ocr", Some(paddle_read(11, 5)));
    let (selected, adopted, cacheable) = run(Some(Arc::clone(&paddle)), tesseract_read(13, 3), true).await;
    assert!(adopted);
    assert!(!cacheable);
    assert_eq!(selected.content, paddle_read(11, 5).content);
    assert_eq!(
        selected.metadata.additional.get(FALLBACK_BACKEND_METADATA_KEY),
        Some(&serde_json::Value::String(PADDLE_BACKEND.to_string()))
    );
    assert_eq!(paddle.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_rejected_paddle_read_keeps_the_tesseract_read_unmarked() {
    let primary = tesseract_read(13, 3);
    let paddle = MockPaddle::new("paddle-ocr", Some(paddle_read(2, 0)));
    let (selected, adopted, _) = run(Some(paddle), primary.clone(), true).await;
    assert!(!adopted);
    assert_eq!(selected.content, primary.content);
    assert!(
        selected
            .metadata
            .additional
            .get(FALLBACK_BACKEND_METADATA_KEY)
            .is_none()
    );
}

#[tokio::test]
async fn a_failed_paddle_read_keeps_the_tesseract_read() {
    let primary = tesseract_read(13, 3);
    let (selected, adopted) = read_if_better(
        None,
        async || {
            Err(crate::XbergError::Ocr {
                message: "model missing".to_string(),
                source: None,
            })
        },
        primary.clone(),
        &OcrQualityThresholds::default(),
    )
    .await
    .unwrap();
    assert!(!adopted);
    assert_eq!(selected.content, primary.content);
}

#[tokio::test]
async fn cancellation_before_the_read_prevents_it() {
    let token = CancellationToken::new();
    token.cancel();
    let result = read_if_better(
        Some(&token),
        async || panic!("cancelled before the PaddleOCR read"),
        tesseract_read(13, 3),
        &OcrQualityThresholds::default(),
    )
    .await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
}

#[tokio::test]
async fn cancellation_during_the_read_discards_the_candidate() {
    let token = CancellationToken::new();
    let result = read_if_better(
        Some(&token),
        async || {
            token.cancel();
            Ok(paddle_read(11, 5))
        },
        tesseract_read(13, 3),
        &OcrQualityThresholds::default(),
    )
    .await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
}

fn stamps(size: u64) -> Option<FileStamps> {
    Some(vec![(PathBuf::from("rec.onnx"), size, None)])
}

#[test]
fn the_model_memo_rehashes_only_when_the_files_change_and_fails_closed_when_they_go() {
    let memo = ModelMemo::default();
    let hashes = AtomicUsize::new(0);
    let counter = &hashes;
    let verify = |answer: bool| {
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            answer
        }
    };

    assert!(!memo.ready("set".into(), None, verify(true)), "a missing file");
    assert_eq!(hashes.load(Ordering::SeqCst), 0, "a partial cache is never hashed");

    assert!(memo.ready("set".into(), stamps(10), verify(true)));
    assert!(memo.ready("set".into(), stamps(10), verify(false)));
    assert_eq!(
        hashes.load(Ordering::SeqCst),
        1,
        "an unchanged verified set is hashed once"
    );
    assert!(
        !memo.ready("set".into(), None, verify(true)),
        "a verified set that disappeared fails closed instead of downloading"
    );

    assert!(
        !memo.ready("set".into(), stamps(11), verify(false)),
        "changed and corrupt"
    );
    assert!(!memo.ready("set".into(), stamps(11), verify(true)));
    assert_eq!(
        hashes.load(Ordering::SeqCst),
        2,
        "an unchanged failed set is not rehashed"
    );
    assert!(
        memo.ready("set".into(), stamps(12), verify(true)),
        "repaired files are rechecked"
    );
    assert_eq!(hashes.load(Ordering::SeqCst), 3);
}

#[test]
fn the_model_memo_stays_bounded() {
    let memo = ModelMemo::default();
    for index in 0..=MAX_MEMOIZED_MODEL_SETS {
        assert!(memo.ready(format!("set {index}"), stamps(1), || true));
    }
    assert!(memo.lock().len() <= MAX_MEMOIZED_MODEL_SETS);
}

#[test]
fn file_stamps_need_every_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let present = dir.path().join("det.onnx");
    std::fs::write(&present, b"model").expect("write model");
    let stamped = stamp_files(vec![present.clone()]).expect("an existing file is stamped");
    assert_eq!(stamped[0].1, 5);
    assert!(stamp_files(vec![present, dir.path().join("rec.onnx")]).is_none());
}

#[test]
fn a_document_from_the_paddle_read_is_recognised() {
    let doc = |backend: &str| InternalDocument {
        prebuilt_pages: Some(vec![page_read_by(backend)]),
        ..Default::default()
    };
    assert!(read_by_paddle(&doc(PADDLE_BACKEND)));
    assert!(!read_by_paddle(&doc("tesseract")));
    assert!(!read_by_paddle(&InternalDocument::default()));
}

#[test]
fn only_the_two_builtin_backends_pair_up_on_a_photo() {
    let config = ExtractionConfig::default();
    let ocr = OcrConfig::default();
    let photo = jpeg(960, 1280);
    let mut registry = OcrBackendRegistry::new();
    let tesseract = registry.get("tesseract").expect("built-in Tesseract");
    let paddle = backend(&registry, &tesseract, (&config, &ocr), &photo, "image/jpeg").expect("built-in PaddleOCR");
    assert_eq!(paddle.name(), PADDLE_BACKEND, "resolved through the `paddleocr` alias");

    assert!(backend(&registry, &tesseract, (&config, &ocr), &photo, "image/png").is_none());
    let opted_out = OcrConfig {
        image_paddle_fallback: false,
        ..Default::default()
    };
    assert!(backend(&registry, &tesseract, (&config, &opted_out), &photo, "image/jpeg").is_none());
    assert!(
        backend(&registry, &paddle, (&config, &ocr), &photo, "image/jpeg").is_none(),
        "primary is not Tesseract"
    );

    registry
        .register(MockPaddle::new(PADDLE_BACKEND, None))
        .expect("register a replacement PaddleOCR");
    assert!(
        backend(&registry, &tesseract, (&config, &ocr), &photo, "image/jpeg").is_none(),
        "a replaced PaddleOCR is not the built-in one"
    );

    let mut registry = OcrBackendRegistry::new();
    registry
        .register(Arc::new(crate::ocr::tesseract_backend::TesseractBackend::new()))
        .expect("register a replacement Tesseract");
    let replaced = registry.get("tesseract").expect("replacement Tesseract");
    assert!(
        backend(&registry, &replaced, (&config, &ocr), &photo, "image/jpeg").is_none(),
        "a replaced Tesseract is not the built-in one"
    );
}

/// End to end on a real receipt photo with the real backends. Needs the PaddleOCR models in the
/// local cache, so it runs only with `XBERG_TEST_PADDLE_MODELS=1` and `--ignored`.
#[tokio::test]
#[ignore = "needs cached PaddleOCR models and tessdata: XBERG_TEST_PADDLE_MODELS=1 cargo test -- --ignored"]
async fn a_real_receipt_photo_is_read_by_paddle() {
    if std::env::var_os("XBERG_TEST_PADDLE_MODELS").is_none() {
        eprintln!("SKIP: set XBERG_TEST_PADDLE_MODELS=1 with the PaddleOCR models cached");
        return;
    }
    let Some(bytes) = crate::utils::read_test_fixture("images/cord_receipt_02.jpg") else {
        return;
    };
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };
    let result = crate::core::extractor::extract_bytes(&bytes, "image/jpeg", &config)
        .await
        .expect("extraction");
    let page = result.pages.as_ref().and_then(|pages| pages.first()).expect("one page");
    assert_eq!(
        page.ocr_confidence
            .as_ref()
            .map(|confidence| confidence.backend.as_str()),
        Some(PADDLE_BACKEND),
        "{}",
        result.content
    );
    assert_eq!(
        result.metadata.additional.get(FALLBACK_BACKEND_METADATA_KEY),
        Some(&serde_json::Value::String(PADDLE_BACKEND.to_string()))
    );
    assert!(result.content.contains("CASH"), "{}", result.content);
}
