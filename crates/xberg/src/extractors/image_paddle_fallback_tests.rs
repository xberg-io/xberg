use super::*;
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

fn tesseract_words(words: &[(&str, f64)]) -> ExtractedDocument {
    ExtractedDocument {
        content: words.iter().map(|(text, _)| *text).collect::<Vec<_>>().join(" "),
        ocr_elements: Some(words.iter().map(|(text, conf)| word(text, *conf)).collect()),
        ..Default::default()
    }
}

fn paddle_text(text: &str) -> ExtractedDocument {
    ExtractedDocument {
        content: text.into(),
        ..Default::default()
    }
}

#[path = "image_paddle_fallback_photos.rs"]
mod photos;
use photos::MEASURED_PHOTOS;

/// The six photos PaddleOCR reads better (word F1 against ground truth, Tesseract -> PaddleOCR:
/// .51->.95, .00->1.00, .43->.83, .49->1.00, .24->.80, .00->.67), judged on the real text both
/// backends produced. Expected numbers: (Tesseract words, trusted, Tesseract plausible/all,
/// PaddleOCR plausible/all).
#[test]
fn the_measured_photos_switch_to_the_paddle_read() {
    let expected = [
        (34, 13, (29, 38), (28, 34)),
        (13, 3, (7, 9), (11, 16)),
        (27, 10, (23, 30), (25, 29)),
        (35, 12, (22, 36), (14, 17)),
        (38, 7, (27, 34), (50, 61)),
        (40, 1, (22, 33), (34, 37)),
    ];
    for (photo, (words, trusted, tesseract, paddle)) in MEASURED_PHOTOS.iter().zip(expected) {
        let primary = tesseract_words(photo.tesseract);
        let candidate = paddle_text(photo.paddle);
        let name = photo.name;
        assert_eq!(usable_word_confidences(&primary).len(), words, "{name}");
        assert!(sparse_unreliable_read(&primary), "{name}: the photo must be eligible");
        assert_eq!(trusted_tesseract_tokens(&primary), trusted, "{name}");
        let all_tesseract: usize = photo.tesseract.iter().map(|(text, _)| tokens(text).count()).sum();
        let plausible_tesseract: usize = photo.tesseract.iter().map(|(text, _)| plausible_tokens(text)).sum();
        assert_eq!((plausible_tesseract, all_tesseract), tesseract, "{name}");
        assert_eq!(
            (plausible_tokens(photo.paddle), tokens(photo.paddle).count()),
            paddle,
            "{name}"
        );
        assert!(
            paddle_read_wins(&primary, &candidate, &OcrQualityThresholds::default()),
            "{name}"
        );
    }
}

#[test]
fn a_photo_without_text_keeps_the_tesseract_read() {
    // flower_no_text.jpg as measured: four Tesseract fragments, no PaddleOCR text. ~keep
    let primary = tesseract_words(&[(" &", 0.0), ("a,", 0.4723), ("Pay", 0.1524), ("<=", 0.7808)]);
    assert!(sparse_unreliable_read(&primary));
    assert!(!paddle_read_wins(
        &primary,
        &paddle_text(""),
        &OcrQualityThresholds::default()
    ));
}

#[test]
fn plausible_tokens_are_words_and_numbers_on_both_sides() {
    for token in ["total", "CASH", "91000", "Widget", "Ärger", "12", "٣٤", "１２"] {
        assert!(is_plausible_token(token), "{token}");
    }
    for token in [
        "x",
        "1",
        "rn",
        "CoE7",
        "x7Q",
        "TTL",
        "00xITEMs",
        "aaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        assert!(!is_plausible_token(token), "{token}");
    }
}

/// A Tesseract read of `words` words: `confident` numbers read at 0.9, the rest real-looking
/// words misread at 0.2, so its plausible share is 1.0.
fn tesseract_read(words: usize, confident: usize) -> ExtractedDocument {
    let words: Vec<(String, f64)> = (0..words)
        .map(|index| {
            if index < confident {
                (format!("{}", 1000 + index), 0.9)
            } else {
                ("word".to_string(), 0.2)
            }
        })
        .collect();
    let words: Vec<(&str, f64)> = words.iter().map(|(text, conf)| (text.as_str(), *conf)).collect();
    tesseract_words(&words)
}

/// A PaddleOCR read of `plausible` receipt words and amounts plus `implausible` fragments, two
/// tokens a line.
fn paddle_read(plausible: usize, implausible: usize) -> ExtractedDocument {
    let tokens: Vec<String> = (0..plausible)
        .map(|index| {
            if index % 2 == 0 {
                "total".to_string()
            } else {
                format!("{index}500")
            }
        })
        .chain((0..implausible).map(|index| format!("qx{index}")))
        .collect();
    paddle_text(
        &tokens
            .chunks(2)
            .map(|line| line.join(" "))
            .collect::<Vec<_>>()
            .join("\n"),
    )
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
fn the_share_rule_asks_paddle_only_for_what_tesseract_reached_capped_at_the_floor() {
    let thresholds = OcrQualityThresholds::default();
    // cord_receipt_02: Tesseract's 9 tokens are 78% plausible, PaddleOCR's 16 are 69%. ~keep
    let receipt = &MEASURED_PHOTOS[1];
    let primary = tesseract_words(receipt.tesseract);
    assert!(tesseract_plausible_share(&primary) > plausible_share(11, 16));
    assert!(paddle_read_wins(&primary, &paddle_text(receipt.paddle), &thresholds));

    let fragments = tesseract_words(&[("x1", 0.2), ("q7", 0.2), ("zz", 0.2), ("total", 0.2)]);
    assert!((tesseract_plausible_share(&fragments) - 0.25).abs() < f64::EPSILON);
    assert!(
        paddle_read_wins(&fragments, &paddle_read(4, 6), &thresholds),
        "40% plausible beats a 25% read"
    );
    assert!(
        !paddle_read_wins(&fragments, &paddle_read(4, 13), &thresholds),
        "24% does not"
    );
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
        "mostly fragments (6 of 11 plausible) against a word-like read"
    );
    assert!(
        paddle_read_wins(&primary, &paddle_read(6, 3), &thresholds),
        "6 of 9 plausible"
    );

    // Sixty repeats of one short word: plausible tokens, but fragmented and repetitive. ~keep
    let noise = paddle_text(&["on on on on on on on on on on"; 6].join("\n"));
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
    assert!(
        !paddle_read_wins(&primary, &paddle_text("  \n "), &thresholds),
        "empty text"
    );
}

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 80)
        .encode_image(&image::DynamicImage::ImageRgb8(image::RgbImage::new(width, height)))
        .expect("test JPEG encode");
    bytes
}

fn webp(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::new(width, height))
        .write_with_encoder(image::codecs::webp::WebPEncoder::new_lossless(&mut bytes))
        .expect("test WebP encode");
    bytes
}

#[test]
fn only_camera_photos_qualify() {
    let photo = jpeg(960, 1280);
    let limits = crate::extractors::security::SecurityLimits::default();
    assert!(is_camera_photo(&photo, "image/jpeg", &limits));
    assert!(is_camera_photo(&photo, "IMAGE/JPEG", &limits));
    assert!(is_camera_photo(&webp(1024, 768), "image/webp", &limits));
    for mime in ["image/png", "image/bmp", "image/gif", "image/tiff"] {
        assert!(!is_camera_photo(&photo, mime, &limits), "{mime}");
    }
    // US Letter at 150 dpi: a page scan saved as JPEG or WebP. ~keep
    assert!(!is_camera_photo(&jpeg(1275, 1650), "image/jpeg", &limits));
    assert!(!is_camera_photo(&webp(1275, 1650), "image/webp", &limits));
    assert!(!is_camera_photo(b"not an image", "image/jpeg", &limits));
}

fn default_extraction() -> ExtractionConfig {
    ExtractionConfig::default()
}

#[test]
fn the_default_tesseract_request_is_eligible_in_latin_languages() {
    for language in ["eng", "fra", "deu", "spa", "ita", "por", "nld", "pol"] {
        let ocr = OcrConfig {
            language: vec![language.to_string()],
            ..Default::default()
        };
        assert!(eligible(&default_extraction(), &ocr, "image/jpeg"), "{language}");
    }
    let cache_only = OcrConfig {
        backend_options: Some(serde_json::json!({"use_cache": false})),
        ..Default::default()
    };
    assert!(eligible(&default_extraction(), &cache_only, "image/jpeg"));
}

#[test]
fn unmeasured_scripts_keep_the_tesseract_read() {
    for language in [
        "chi_sim", "chi_tra", "rus", "ara", "jpn", "kor", "hin", "tha", "ell", "xyz",
    ] {
        let ocr = OcrConfig {
            language: vec![language.to_string()],
            ..Default::default()
        };
        assert!(!eligible(&default_extraction(), &ocr, "image/jpeg"), "{language}");
    }
}

#[test]
fn any_explicit_ocr_setting_keeps_the_tesseract_read() {
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
    let cases = [
        (
            "element_config",
            OcrConfig {
                element_config: Some(Default::default()),
                ..Default::default()
            },
        ),
        (
            "paddle_ocr_settings",
            OcrConfig {
                paddle_ocr_settings: Some(Default::default()),
                ..Default::default()
            },
        ),
        (
            "paddle_ocr_config",
            OcrConfig {
                paddle_ocr_config: Some(serde_json::json!({})),
                ..Default::default()
            },
        ),
        (
            "tessdata_path",
            OcrConfig {
                tessdata_path: Some("/opt/tessdata".into()),
                ..Default::default()
            },
        ),
        (
            "tessdata_bytes",
            OcrConfig {
                tessdata_bytes: Some(Default::default()),
                ..Default::default()
            },
        ),
        (
            "numeric_repair",
            OcrConfig {
                numeric_repair: true,
                ..Default::default()
            },
        ),
    ];
    for (name, ocr) in cases {
        assert!(!eligible(&default_extraction(), &ocr, "image/jpeg"), "{name}");
    }
    let structure = ExtractionConfig {
        include_document_structure: true,
        ..Default::default()
    };
    assert!(!eligible(&structure, &OcrConfig::default(), "image/jpeg"));
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
fn the_cache_key_depends_on_the_models_only_for_requests_the_fallback_may_serve() {
    let config = ExtractionConfig::default();
    for mime in [None, Some(""), Some("image/jpeg"), Some("image/webp; q=1")] {
        assert!(model_request(&config, mime).is_some(), "{mime:?}");
    }
    for mime in ["image/png", "application/pdf", "text/plain", "image/tiff"] {
        assert!(model_request(&config, Some(mime)).is_none(), "{mime}");
    }
    let no_cache = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };
    assert!(model_request(&no_cache, Some("image/jpeg")).is_none());
    let opted_out = ExtractionConfig {
        ocr: Some(OcrConfig {
            image_paddle_fallback: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(model_request(&opted_out, Some("image/jpeg")).is_none());
}

fn model_set(cache: &std::path::Path) -> crate::paddle_ocr::RequiredModelSet {
    let base = crate::paddle_ocr::PaddleOcrConfig {
        cache_dir: Some(cache.to_path_buf()),
        ..Default::default()
    };
    crate::paddle_ocr::RequiredModelSet::resolve(&base, &paddle_read_config(&OcrConfig::default()))
        .expect("the default model set resolves")
}

#[test]
fn model_files_are_found_in_a_hugging_face_cache_layout() {
    let cache = tempfile::tempdir().expect("temp dir");
    let set = model_set(cache.path());
    let paths = set
        .manager
        .model_file_paths(&set.version, set.family, &set.tier, set.include_doc_ori)
        .expect("model paths");
    assert!(paths.len() >= 3, "det, cls and rec at least: {paths:?}");
    for path in &paths {
        assert!(
            path.starts_with(
                cache
                    .path()
                    .join("models--xberg-io--paddleocr-onnx-models")
                    .join("snapshots")
            ),
            "{}",
            path.display()
        );
    }
    assert!(set.files().is_none(), "an empty cache");
    let missing = model_state_of(Ok(model_set(cache.path())));

    for path in &paths {
        std::fs::create_dir_all(path.parent().expect("snapshot dir")).expect("create snapshot dir");
        std::fs::write(path, b"model").expect("write model");
    }
    assert_eq!(set.files().as_deref(), Some(paths.as_slice()));
    let present = model_state_of(Ok(model_set(cache.path())));
    assert_ne!(missing, present, "a warm cache keys apart from a cold one");
    assert_eq!(present, model_state_of(Ok(model_set(cache.path()))));

    std::fs::write(&paths[0], b"other model").expect("rewrite model");
    assert_ne!(present, model_state_of(Ok(model_set(cache.path()))), "a changed file");

    std::fs::remove_file(&paths[0]).expect("remove model");
    assert!(set.files().is_none(), "a partial cache");
    assert_eq!(missing, model_state_of(Ok(model_set(cache.path()))));
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
    use crate::core::extractor::cacheability;
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

fn receipt() -> (ExtractedDocument, ExtractedDocument) {
    let photo = &MEASURED_PHOTOS[1];
    (tesseract_words(photo.tesseract), paddle_text(photo.paddle))
}

#[tokio::test]
async fn without_a_paddle_backend_the_primary_read_is_returned_unchanged() {
    let (primary, _) = receipt();
    let (selected, adopted, cacheable) = run(None, primary.clone(), true).await;
    assert_eq!(selected.content, primary.content);
    assert!(!adopted);
    assert!(cacheable);
}

#[tokio::test]
async fn a_robust_or_dense_or_empty_read_is_never_read_again() {
    for primary in [tesseract_read(3, 3), tesseract_read(150, 100), tesseract_read(0, 0)] {
        let paddle = MockPaddle::new("paddle-ocr", None);
        let (selected, adopted, _) = run(Some(Arc::clone(&paddle)), primary.clone(), true).await;
        assert_eq!(selected.content, primary.content);
        assert!(!adopted);
        assert_eq!(paddle.calls.load(Ordering::SeqCst), 0);
    }
}

/// The model state is in the extraction cache key, so neither a skipped nor an adopted fallback
/// takes the operation, or the other items of a batch sharing it, out of the cache.
#[tokio::test]
async fn a_sparse_photo_leaves_the_operation_cacheable_for_the_other_items() {
    let (primary, candidate) = receipt();
    let skipped = MockPaddle::new("paddle-ocr", None);
    let (selected, adopted, cacheable) = run(Some(Arc::clone(&skipped)), primary.clone(), false).await;
    assert_eq!(selected.content, primary.content);
    assert!(!adopted);
    assert!(cacheable, "models missing");
    assert_eq!(skipped.calls.load(Ordering::SeqCst), 0);

    let read = MockPaddle::new("paddle-ocr", Some(candidate.clone()));
    let (selected, adopted, cacheable) = run(Some(Arc::clone(&read)), primary, true).await;
    assert!(adopted);
    assert!(cacheable, "adopted");
    assert_eq!(selected.content, candidate.content);
    assert_eq!(
        selected.metadata.additional.get(FALLBACK_BACKEND_METADATA_KEY),
        Some(&serde_json::Value::String(PADDLE_BACKEND.to_string()))
    );
    assert_eq!(read.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_rejected_paddle_read_keeps_the_tesseract_read_unmarked() {
    let (primary, _) = receipt();
    let paddle = MockPaddle::new("paddle-ocr", Some(paddle_text("TOTAL 91000")));
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
    let (primary, _) = receipt();
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
        receipt().0,
        &OcrQualityThresholds::default(),
    )
    .await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
}

#[tokio::test]
async fn cancellation_during_the_read_discards_the_candidate() {
    let token = CancellationToken::new();
    let (primary, candidate) = receipt();
    let result = read_if_better(
        Some(&token),
        async || {
            token.cancel();
            Ok(candidate)
        },
        primary,
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
fn concurrent_checks_of_one_model_set_hash_once() {
    let memo = ModelMemo::default();
    let hashes = AtomicUsize::new(0);
    let (memo, counter) = (&memo, &hashes);
    let (release, released) = std::sync::mpsc::channel::<()>();
    std::thread::scope(|scope| {
        let first = scope.spawn(move || {
            memo.ready("set".into(), stamps(1), move || {
                counter.fetch_add(1, Ordering::SeqCst);
                released.recv().expect("release the first check");
                true
            })
        });
        while !matches!(memo.lock().get("set"), Some(ModelCheck::InFlight)) {
            std::thread::yield_now();
        }
        let second = scope.spawn(move || {
            memo.ready("set".into(), stamps(1), move || {
                counter.fetch_add(1, Ordering::SeqCst);
                true
            })
        });
        // Let the second caller reach the in-flight check before the first one finishes. ~keep
        std::thread::sleep(std::time::Duration::from_millis(100));
        release.send(()).expect("first check waiting");
        assert!(first.join().expect("first check"));
        assert!(second.join().expect("second check"));
    });
    assert_eq!(hashes.load(Ordering::SeqCst), 1);
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
    let mut doc = InternalDocument::default();
    assert!(!read_by_paddle(&doc));
    mark_read_by_paddle(&mut doc);
    assert!(read_by_paddle(&doc));
    doc.metadata.additional.insert(
        std::borrow::Cow::Borrowed(FALLBACK_BACKEND_METADATA_KEY),
        serde_json::Value::String("tesseract".to_string()),
    );
    assert!(!read_by_paddle(&doc));
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
/// local cache and the fetched corpus: `XBERG_TEST_PADDLE_MODELS=1 cargo test -- --ignored`.
#[tokio::test]
#[ignore = "needs cached PaddleOCR models and the corpus: XBERG_TEST_PADDLE_MODELS=1 cargo test -- --ignored"]
async fn a_real_receipt_photo_is_read_by_paddle() {
    assert!(
        std::env::var_os("XBERG_TEST_PADDLE_MODELS").is_some(),
        "set XBERG_TEST_PADDLE_MODELS=1 with the PaddleOCR models cached"
    );
    let bytes = crate::utils::read_test_fixture("images/cord_receipt_02.jpg")
        .expect("images/cord_receipt_02.jpg must be fetched for this test");
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
