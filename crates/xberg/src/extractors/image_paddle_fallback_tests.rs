use super::*;
use crate::core::extractor::cacheability;
use crate::plugins::{OcrBackendType, Plugin};
use crate::types::{OcrBoundingGeometry, OcrConfidence, OcrElement};
use std::sync::atomic::{AtomicUsize, Ordering};

fn element(text: &str, recognition: f64, level: OcrElementLevel) -> OcrElement {
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
        level,
        rotation: None,
        page_number: 1,
        parent_id: None,
        backend_metadata: Default::default(),
    }
}

/// A Tesseract read of `words` words, `confident` of them read at 0.9 and the rest at 0.2.
fn tesseract_read(words: usize, confident: usize) -> ExtractedDocument {
    let words: Vec<(String, f64)> = (0..words)
        .map(|index| {
            if index < confident {
                (format!("Word{index}"), 0.9)
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
        ocr_elements: Some(
            words
                .iter()
                .map(|(text, conf)| element(text, *conf, OcrElementLevel::Word))
                .collect(),
        ),
        ..Default::default()
    }
}

/// A PaddleOCR read whose lines hold `tokens` receipt-like tokens, read at 0.95.
fn paddle_read(tokens: usize) -> ExtractedDocument {
    let tokens: Vec<String> = (0..tokens)
        .map(|index| {
            if index % 2 == 0 {
                format!("Item{index}")
            } else {
                format!("{index}500")
            }
        })
        .collect();
    let lines: Vec<String> = tokens.chunks(2).map(|line| line.join(" ")).collect();
    paddle_lines(&lines.iter().map(String::as_str).collect::<Vec<_>>(), 0.95)
}

fn paddle_lines(lines: &[&str], recognition: f64) -> ExtractedDocument {
    ExtractedDocument {
        content: lines.join("\n"),
        ocr_elements: Some(
            lines
                .iter()
                .map(|line| element(line, recognition, OcrElementLevel::Line))
                .collect(),
        ),
        ..Default::default()
    }
}

/// Measured on the benchmark photos (2026-11-26): usable Tesseract words, Tesseract tokens read
/// at >= 0.70, PaddleOCR line tokens read at >= 0.70. PaddleOCR's read raised word F1 on each.
const MEASURED_PHOTOS: &[(&str, usize, usize, usize)] = &[
    ("cord_receipt_01", 34, 16, 34),
    ("cord_receipt_02", 13, 3, 16),
    ("cord_receipt_03", 27, 12, 29),
    ("cord_receipt_04", 35, 12, 17),
    ("textocr_scene_01", 38, 9, 61),
    ("textocr_scene_03", 40, 2, 36),
];

#[test]
fn the_measured_photos_switch_to_the_paddle_read() {
    for &(name, words, trusted, read) in MEASURED_PHOTOS {
        let primary = tesseract_read(words, trusted);
        assert!(sparse_unreliable_read(&primary), "{name}: the photo must be eligible");
        assert_eq!(confident_tokens(&primary, OcrElementLevel::Word), trusted, "{name}");
        let candidate = paddle_read(read);
        assert_eq!(confident_tokens(&candidate, OcrElementLevel::Line), read, "{name}");
        assert!(
            paddle_read_wins(&primary, &candidate, &OcrQualityThresholds::default()),
            "{name}"
        );
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
fn a_paddle_read_within_the_margin_of_tesseract_loses() {
    let primary = tesseract_read(70, 48);
    assert!(sparse_unreliable_read(&primary));
    let thresholds = OcrQualityThresholds::default();
    assert!(!paddle_read_wins(&primary, &paddle_read(59), &thresholds));
    assert!(paddle_read_wins(&primary, &paddle_read(60), &thresholds));
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
        !paddle_read_wins(&primary, &paddle_read(3), &thresholds),
        "below the token gain"
    );
    assert!(paddle_read_wins(&primary, &paddle_read(4), &thresholds));

    let noise = paddle_lines(&["a %%"; 20], 0.95);
    assert!(
        crate::extractors::pdf::ocr::compute_quality_score(&noise.content, &thresholds)
            < thresholds.pipeline_min_quality,
        "the fixture must fail the default quality gate"
    );
    assert!(confident_tokens(&noise, OcrElementLevel::Line) >= MIN_TOKEN_GAIN);
    assert!(
        !paddle_read_wins(&primary, &noise, &thresholds),
        "below the quality gate"
    );

    let mut blank = paddle_read(8);
    blank.content = "  \n ".to_string();
    assert!(!paddle_read_wins(&primary, &blank, &thresholds), "empty text");

    let unsure = paddle_lines(&["Item0 1500", "Item2 3500", "Item4 5500"], 0.5);
    assert!(
        !paddle_read_wins(&primary, &unsure, &thresholds),
        "lines read without confidence"
    );
}

fn default_extraction() -> ExtractionConfig {
    ExtractionConfig::default()
}

#[test]
fn the_default_tesseract_request_is_eligible() {
    assert!(eligible(&default_extraction(), &OcrConfig::default(), "image/jpeg"));
    let cache_only = OcrConfig {
        tesseract_config: Some(crate::types::TesseractConfig {
            use_cache: false,
            ..Default::default()
        }),
        backend_options: Some(serde_json::json!({"use_cache": false})),
        ..Default::default()
    };
    assert!(eligible(&default_extraction(), &cache_only, "image/png"));
    let materialized = OcrConfig {
        tesseract_config: Some(Default::default()),
        ..Default::default()
    };
    assert!(eligible(&default_extraction(), &materialized, "image/png"));
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
fn any_tesseract_setting_off_its_default_keeps_the_tesseract_read() {
    use crate::types::TesseractConfig;
    let tuned: Vec<(&str, TesseractConfig)> = vec![
        (
            "psm",
            TesseractConfig {
                psm: Some(6),
                ..Default::default()
            },
        ),
        (
            "preprocessing",
            TesseractConfig {
                preprocessing: Some(Default::default()),
                ..Default::default()
            },
        ),
        (
            "whitelist",
            TesseractConfig {
                tessedit_char_whitelist: "0123456789".into(),
                ..Default::default()
            },
        ),
        (
            "blacklist",
            TesseractConfig {
                tessedit_char_blacklist: "|".into(),
                ..Default::default()
            },
        ),
        (
            "oem",
            TesseractConfig {
                oem: 0,
                ..Default::default()
            },
        ),
        (
            "min_confidence",
            TesseractConfig {
                min_confidence: 42.0,
                ..Default::default()
            },
        ),
        (
            "ngram",
            TesseractConfig {
                language_model_ngram_on: !TesseractConfig::default().language_model_ngram_on,
                ..Default::default()
            },
        ),
        (
            "table detection",
            TesseractConfig {
                enable_table_detection: !TesseractConfig::default().enable_table_detection,
                ..Default::default()
            },
        ),
        (
            "thresholding",
            TesseractConfig {
                thresholding_method: 2,
                ..Default::default()
            },
        ),
        (
            "language",
            TesseractConfig {
                language: vec!["deu".into()],
                ..Default::default()
            },
        ),
    ];
    for (name, tesseract) in tuned {
        let ocr = OcrConfig {
            tesseract_config: Some(tesseract),
            ..Default::default()
        };
        assert!(!eligible(&default_extraction(), &ocr, "image/jpeg"), "{name}");
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
    let paddle = MockPaddle::new("paddle-ocr", Some(paddle_read(16)));
    let (selected, adopted, cacheable) = run(Some(Arc::clone(&paddle)), tesseract_read(13, 3), true).await;
    assert!(adopted);
    assert!(!cacheable);
    assert_eq!(selected.content, paddle_read(16).content);
    assert_eq!(
        selected.metadata.additional.get(FALLBACK_BACKEND_METADATA_KEY),
        Some(&serde_json::Value::String(PADDLE_BACKEND.to_string()))
    );
    assert_eq!(paddle.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_rejected_paddle_read_keeps_the_tesseract_read_unmarked() {
    let primary = tesseract_read(13, 3);
    let paddle = MockPaddle::new("paddle-ocr", Some(paddle_read(2)));
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
            Ok(paddle_read(16))
        },
        tesseract_read(13, 3),
        &OcrQualityThresholds::default(),
    )
    .await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
}

#[tokio::test]
async fn the_model_memo_hashes_once_per_key_and_fails_closed_when_files_disappear() {
    let memo = ModelMemo::default();
    let hashes = Arc::new(AtomicUsize::new(0));
    let verify = |answer: bool| {
        let hashes = Arc::clone(&hashes);
        move || {
            hashes.fetch_add(1, Ordering::SeqCst);
            answer
        }
    };

    assert!(
        !memo.ready("partial".into(), || false, verify(true)).await,
        "partial cache"
    );
    assert_eq!(hashes.load(Ordering::SeqCst), 0, "a partial cache is never hashed");

    assert!(memo.ready("warm".into(), || true, verify(true)).await);
    assert!(memo.ready("warm".into(), || true, verify(true)).await);
    assert_eq!(hashes.load(Ordering::SeqCst), 1, "a verified set is hashed once");
    assert!(
        !memo.ready("warm".into(), || false, verify(true)).await,
        "a verified set that disappeared fails closed instead of downloading"
    );

    assert!(!memo.ready("corrupt".into(), || true, verify(false)).await);
    assert!(!memo.ready("corrupt".into(), || true, verify(true)).await);
    assert_eq!(hashes.load(Ordering::SeqCst), 2, "a failed verification is kept too");
}

#[tokio::test]
async fn the_model_memo_stays_bounded() {
    let memo = ModelMemo::default();
    for index in 0..=MAX_MEMOIZED_MODEL_SETS {
        assert!(memo.ready(format!("set {index}"), || true, || true).await);
    }
    assert!(memo.lock().len() <= MAX_MEMOIZED_MODEL_SETS);
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
fn only_the_two_builtin_backends_pair_up() {
    let config = ExtractionConfig::default();
    let ocr = OcrConfig::default();
    let mut registry = OcrBackendRegistry::new();
    let tesseract = registry.get("tesseract").expect("built-in Tesseract");
    let paddle = backend(&registry, &tesseract, &config, &ocr, "image/jpeg").expect("built-in PaddleOCR");
    assert_eq!(paddle.name(), PADDLE_BACKEND, "resolved through the `paddleocr` alias");

    let opted_out = OcrConfig {
        image_paddle_fallback: false,
        ..Default::default()
    };
    assert!(backend(&registry, &tesseract, &config, &opted_out, "image/jpeg").is_none());
    assert!(
        backend(&registry, &paddle, &config, &ocr, "image/jpeg").is_none(),
        "primary is not Tesseract"
    );

    registry
        .register(MockPaddle::new(PADDLE_BACKEND, None))
        .expect("register a replacement PaddleOCR");
    assert!(
        backend(&registry, &tesseract, &config, &ocr, "image/jpeg").is_none(),
        "a replaced PaddleOCR is not the built-in one"
    );

    let mut registry = OcrBackendRegistry::new();
    registry
        .register(Arc::new(crate::ocr::tesseract_backend::TesseractBackend::new()))
        .expect("register a replacement Tesseract");
    let replaced = registry.get("tesseract").expect("replacement Tesseract");
    assert!(
        backend(&registry, &replaced, &config, &ocr, "image/jpeg").is_none(),
        "a replaced Tesseract is not the built-in one"
    );
}
