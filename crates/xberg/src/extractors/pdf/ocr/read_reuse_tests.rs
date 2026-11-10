use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};

use super::*;
use crate::cancellation::CancellationToken;
use crate::plugins::{OcrBackendType, Plugin};

struct Fake {
    kind: OcrBackendType,
    calls: AtomicUsize,
    fail: AtomicBool,
    change_model_on_read: AtomicBool,
    policy_sensitive: AtomicBool,
    cancel_on_version: parking_lot::Mutex<Option<CancellationToken>>,
}

impl Fake {
    fn new(kind: OcrBackendType) -> Arc<Self> {
        Arc::new(Self {
            kind,
            calls: AtomicUsize::new(0),
            fail: AtomicBool::new(false),
            change_model_on_read: AtomicBool::new(false),
            policy_sensitive: AtomicBool::new(false),
            cancel_on_version: parking_lot::Mutex::new(None),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(SeqCst)
    }
}

impl Plugin for Fake {
    fn name(&self) -> &str {
        "read-reuse-fake"
    }
    fn version(&self) -> String {
        if let Some(token) = self.cancel_on_version.lock().as_ref() {
            token.cancel();
        }
        "v0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for Fake {
    async fn process_image(&self, image: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        let call = self.calls.fetch_add(1, SeqCst) + 1;
        if self.change_model_on_read.load(SeqCst) {
            let model = config.tessdata_path.as_ref().unwrap().join("eng.traineddata");
            std::fs::write(model, b"replacement model written during recognition").unwrap();
        }
        if self.fail.load(SeqCst) {
            return Err(crate::XbergError::ocr("fake backend failure".to_string()));
        }
        let mut document = ExtractedDocument {
            content: format!("read {call} of {} bytes", image.len()),
            ..Default::default()
        };
        if self.policy_sensitive.load(SeqCst) {
            let day = if config.tesseract_config.as_ref().and_then(|c| c.psm) == Some(3) {
                "Tuesday"
            } else {
                "Thursday"
            };
            document.content = format!("The committee meets on {day}.");
            document.tables = vec![crate::types::Table {
                cells: vec![vec!["Day".into()], vec![day.into()]],
                ..Default::default()
            }];
        }
        Ok(document)
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        self.kind
    }
}

fn png(top_left: u8) -> Arc<Vec<u8>> {
    let mut image = image::RgbImage::from_pixel(2, 2, image::Rgb([255, 255, 255]));
    image.put_pixel(0, 0, image::Rgb([top_left; 3]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    Arc::new(bytes.into_inner())
}

fn effective_config() -> OcrConfig {
    OcrConfig {
        backend_options: Some(serde_json::json!({ "source_dpi": 300.0 })),
        ..Default::default()
    }
}

struct Fixture {
    _models: tempfile::TempDir,
    fake: Arc<Fake>,
    reader: Arc<dyn OcrBackend>,
    backend: Arc<dyn OcrBackend>,
    original: OcrConfig,
    effective: OcrConfig,
    pixels: Arc<Vec<u8>>,
}

impl Fixture {
    fn new() -> Self {
        let fake = Fake::new(OcrBackendType::Tesseract);
        let reader: Arc<dyn OcrBackend> = fake.clone();
        crate::plugins::ensure_ocr_backends_initialized();
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let backend = registry.read().get("tesseract").unwrap();
        assert!(registry.read().is_builtin_tesseract(&backend));
        let models = tempfile::tempdir().unwrap();
        std::fs::write(models.path().join("eng.traineddata"), b"fake model a").unwrap();
        let mut effective = effective_config();
        effective.tessdata_path = Some(models.path().into());
        Self {
            _models: models,
            fake,
            reader,
            backend,
            original: OcrConfig::default(),
            effective,
            pixels: png(0),
        }
    }

    fn call(&self, page: u32) -> Call<'_> {
        Call {
            page,
            backend: &self.backend,
            reader: &self.reader,
            original: &self.original,
            effective: &self.effective,
            bytes: &self.pixels,
            whole_page_raster: false,
            cancel: None,
        }
    }

    async fn read(&self, reuse: &Arc<ProbeReads>, page: u32) -> crate::Result<ExtractedDocument> {
        run(reuse, self.call(page)).await
    }
}

struct Call<'a> {
    page: u32,
    backend: &'a Arc<dyn OcrBackend>,
    reader: &'a Arc<dyn OcrBackend>,
    original: &'a OcrConfig,
    effective: &'a OcrConfig,
    bytes: &'a Arc<Vec<u8>>,
    whole_page_raster: bool,
    cancel: Option<&'a CancellationToken>,
}

async fn run(reuse: &Arc<ProbeReads>, call: Call<'_>) -> crate::Result<ExtractedDocument> {
    let identity = ReadIdentity {
        page: call.page,
        backend: call.backend,
        configs: (call.original, call.effective),
        whole_page_raster: call.whole_page_raster,
        correction_degrees: 0,
    };
    let engine = Arc::clone(call.reader);
    let config = call.effective.clone();
    let recognize = move |bytes: Arc<Vec<u8>>| async move { engine.process_image(bytes.as_slice(), &config).await };
    read(Some(reuse), identity, Arc::clone(call.bytes), call.cancel, recognize).await
}

fn reuse() -> Arc<ProbeReads> {
    Arc::new(ProbeReads::new(1 << 20))
}

#[tokio::test]
async fn final_read_takes_the_probe_read_once_for_a_contradicted_page() {
    let fixture = Fixture::new();
    let reuse = reuse();
    let probe = fixture.read(&reuse, 1).await.unwrap();
    reuse.finish(&[1]);

    let reused = fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 1, "the final read must not OCR again");
    assert_eq!(reused.content, probe.content);

    let again = fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 2, "a retained read is handed out once");
    assert_eq!(again.content, format!("read 2 of {} bytes", fixture.pixels.len()));
}

#[tokio::test]
async fn finish_retains_only_contradicted_pages() {
    let fixture = Fixture::new();
    let reuse = reuse();
    fixture.read(&reuse, 1).await.unwrap();
    fixture.read(&reuse, 2).await.unwrap();
    reuse.finish(&[2]);

    fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 3, "an uncontradicted page is dropped at finish");
    fixture.read(&reuse, 2).await.unwrap();
    assert_eq!(fixture.fake.calls(), 3, "the contradicted page is reused");
}

#[tokio::test]
async fn same_pixels_on_another_page_are_not_reused() {
    let fixture = Fixture::new();
    let reuse = reuse();
    fixture.read(&reuse, 1).await.unwrap();
    reuse.finish(&[1, 2]);

    fixture.read(&reuse, 2).await.unwrap();
    assert_eq!(fixture.fake.calls(), 2, "page 2 must not receive page 1's read");
    fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 2, "page 1's read survives the page 2 miss");
}

#[tokio::test]
async fn zero_retention_limit_reads_again_and_returns_each_document_intact() {
    let fixture = Fixture::new();
    let reuse = Arc::new(ProbeReads::new(0));
    let size = fixture.pixels.len();
    let probe = fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(probe.content, format!("read 1 of {size} bytes"));
    reuse.finish(&[1]);

    let final_read = fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(
        fixture.fake.calls(),
        2,
        "nothing fits a zero budget, so the page is read again"
    );
    assert_eq!(final_read.content, format!("read 2 of {size} bytes"));
}

async fn assert_miss(fixture: &Fixture, reuse: &Arc<ProbeReads>, call: Call<'_>, what: &str) {
    let before = fixture.fake.calls();
    run(reuse, call).await.unwrap();
    assert_eq!(fixture.fake.calls(), before + 1, "{what} must miss");
}

#[tokio::test]
async fn identity_changes_miss_without_consuming_the_matching_read() {
    let fixture = Fixture::new();
    let reuse = reuse();
    fixture.read(&reuse, 1).await.unwrap();
    reuse.finish(&[1]);

    let changed = png(1);
    assert_miss(
        &fixture,
        &reuse,
        Call {
            bytes: &changed,
            ..fixture.call(1)
        },
        "a changed pixel",
    )
    .await;
    let original = OcrConfig {
        language: vec!["deu".to_string()],
        ..Default::default()
    };
    let call = Call {
        original: &original,
        ..fixture.call(1)
    };
    assert_miss(&fixture, &reuse, call, "a changed original config").await;
    let mut limited = fixture.effective.clone();
    limited.security_limits = Some(crate::extractors::security::SecurityLimits {
        max_iterations: 7,
        ..Default::default()
    });
    let call = Call {
        effective: &limited,
        ..fixture.call(1)
    };
    assert_miss(&fixture, &reuse, call, "a changed serde-skipped security limit").await;
    let other = Fake::new(OcrBackendType::Tesseract);
    let other_backend: Arc<dyn OcrBackend> = other.clone();
    run(
        &reuse,
        Call {
            backend: &other_backend,
            reader: &other_backend,
            ..fixture.call(1)
        },
    )
    .await
    .unwrap();
    assert_eq!(other.calls(), 1, "another backend instance must read for itself");

    let before = fixture.fake.calls();
    fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(
        fixture.fake.calls(),
        before,
        "every miss left the matching read in place"
    );
}

#[tokio::test]
async fn cancelled_read_returns_cancelled_without_ocr_or_consuming_the_read() {
    let fixture = Fixture::new();
    let reuse = reuse();
    fixture.read(&reuse, 1).await.unwrap();
    reuse.finish(&[1]);

    let cancelled = CancellationToken::new();
    cancelled.cancel();
    let result = run(
        &reuse,
        Call {
            cancel: Some(&cancelled),
            ..fixture.call(1)
        },
    )
    .await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    assert_eq!(fixture.fake.calls(), 1);
    fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 1, "a pre-cancelled read leaves the retained read");
}

#[tokio::test]
async fn plugin_reuse_bypass_does_not_consult_version() {
    let mut fixture = Fixture::new();
    fixture.backend = Arc::clone(&fixture.reader);
    let reuse = reuse();
    let token = CancellationToken::new();
    *fixture.fake.cancel_on_version.lock() = Some(token.clone());
    run(
        &reuse,
        Call {
            cancel: Some(&token),
            ..fixture.call(1)
        },
    )
    .await
    .unwrap();
    assert!(!token.is_cancelled(), "reuse must not call the plugin's version method");
    assert_eq!(fixture.fake.calls(), 1);
    assert!(!reuse.finish(&[1]));
}

#[tokio::test]
async fn geometry_observer_bypasses_retained_reads() {
    use crate::ocr::preprocessing::geometry_evidence::observe;
    let fixture = Fixture::new();
    let reuse_a = reuse();
    observe(fixture.read(&reuse_a, 1)).await.0.unwrap();
    reuse_a.finish(&[1]);
    fixture.read(&reuse_a, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 2, "an observed probe read is not retained");

    let reuse_b = reuse();
    fixture.read(&reuse_b, 1).await.unwrap();
    reuse_b.finish(&[1]);
    observe(fixture.read(&reuse_b, 1)).await.0.unwrap();
    assert_eq!(fixture.fake.calls(), 4, "an observed final read runs OCR");
    fixture.read(&reuse_b, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 4, "the observed miss left the retained read");
}

#[tokio::test]
async fn failed_backend_read_is_not_retained() {
    let fixture = Fixture::new();
    let reuse = reuse();
    fixture.fake.fail.store(true, SeqCst);
    assert!(fixture.read(&reuse, 1).await.is_err());
    fixture.fake.fail.store(false, SeqCst);
    reuse.finish(&[1]);

    fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 2, "the failed probe left nothing to reuse");
}

#[tokio::test]
async fn model_change_during_a_read_is_not_retained() {
    let fixture = Fixture::new();
    let reuse = reuse();
    fixture.fake.change_model_on_read.store(true, SeqCst);
    fixture.read(&reuse, 1).await.unwrap();
    fixture.fake.change_model_on_read.store(false, SeqCst);
    assert_eq!(
        std::fs::read(fixture._models.path().join("eng.traineddata")).unwrap(),
        b"replacement model written during recognition"
    );
    assert!(!reuse.finish(&[1]), "the post-read check refuses a changed model");

    fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 2);
}

#[tokio::test]
async fn unkeyable_reads_are_never_retained() {
    let fixture = Fixture::new();
    let paddle = Fake::new(OcrBackendType::PaddleOCR);
    let paddle_backend: Arc<dyn OcrBackend> = paddle.clone();
    let no_dpi = OcrConfig::default();
    let mut rotating = fixture.effective.clone();
    rotating.auto_rotate = Some(true);
    let mut nested_rotation = fixture.effective.clone();
    nested_rotation.tesseract_config = Some(crate::types::TesseractConfig {
        preprocessing: Some(crate::types::ImagePreprocessingConfig {
            auto_rotate: true,
            ..Default::default()
        }),
        ..Default::default()
    });
    let reuse = reuse();
    for _ in 0..2 {
        run(
            &reuse,
            Call {
                whole_page_raster: true,
                ..fixture.call(1)
            },
        )
        .await
        .unwrap();
        run(
            &reuse,
            Call {
                effective: &no_dpi,
                ..fixture.call(2)
            },
        )
        .await
        .unwrap();
        run(
            &reuse,
            Call {
                backend: &paddle_backend,
                reader: &paddle_backend,
                ..fixture.call(3)
            },
        )
        .await
        .unwrap();
        for (index, effective) in [&rotating, &nested_rotation].into_iter().enumerate() {
            run(
                &reuse,
                Call {
                    effective,
                    ..fixture.call(4 + index as u32)
                },
            )
            .await
            .unwrap();
        }
        reuse.finish(&[1, 2, 3, 4, 5]);
    }
    assert_eq!(
        fixture.fake.calls(),
        8,
        "scan, dpi-less and rotating reads are never reused"
    );
    assert_eq!(paddle.calls(), 2, "a non-Tesseract backend is read twice");
}

#[test]
fn model_identity_tracks_override_traineddata_and_osd() {
    let directory = tempfile::tempdir().unwrap();
    let identity = || model_identity(&ModelFiles::resolve(vec!["eng".into()], Some(directory.path()))?);
    std::fs::write(directory.path().join("eng.traineddata"), b"fake model a").unwrap();
    let first = identity().expect("an override directory holding eng resolves");
    assert_eq!(identity(), Some(first), "unchanged files hash identically");

    std::fs::write(directory.path().join("eng.traineddata"), b"fake model b").unwrap();
    let changed = identity().unwrap();
    assert_ne!(changed, first, "changed language data changes the identity");

    std::fs::write(directory.path().join("osd.traineddata"), b"fake osd").unwrap();
    assert_ne!(identity().unwrap(), changed, "adding osd changes the identity");
}

#[cfg(feature = "bundle-tessdata-eng")]
struct StockFixture {
    _models: tempfile::TempDir,
    backend: Arc<dyn OcrBackend>,
    config: OcrConfig,
    pixels: Arc<Vec<u8>>,
}

#[cfg(feature = "bundle-tessdata-eng")]
fn stock_fixture() -> StockFixture {
    crate::plugins::ensure_ocr_backends_initialized();
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let backend = registry.read().get("tesseract").unwrap();
    assert!(registry.read().is_builtin_tesseract(&backend));
    let models = tempfile::tempdir().unwrap();
    std::fs::write(
        models.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().expect("verified bundled English model"),
    )
    .unwrap();
    let config = OcrConfig {
        tessdata_path: Some(models.path().into()),
        auto_rotate: Some(false),
        backend_options: Some(serde_json::json!({"source_dpi": 300.0, "use_cache": false})),
        ..Default::default()
    };
    let image = image::RgbImage::from_pixel(64, 64, image::Rgb([255, 255, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    StockFixture {
        _models: models,
        backend,
        config,
        pixels: Arc::new(bytes.into_inner()),
    }
}

#[cfg(feature = "bundle-tessdata-eng")]
async fn stock_read(
    fixture: &StockFixture,
    reuse: &Arc<ProbeReads>,
    config: &OcrConfig,
    calls: &AtomicUsize,
) -> crate::Result<ExtractedDocument> {
    let identity = ReadIdentity {
        page: 1,
        backend: &fixture.backend,
        configs: (config, config),
        whole_page_raster: false,
        correction_degrees: 0,
    };
    read(
        Some(reuse),
        identity,
        Arc::clone(&fixture.pixels),
        None,
        |bytes| async move {
            calls.fetch_add(1, SeqCst);
            fixture.backend.process_image_owned(bytes, config).await
        },
    )
    .await
}

#[cfg(feature = "bundle-tessdata-eng")]
#[tokio::test]
async fn stock_builtin_reuses_its_own_raw_read_once() {
    let fixture = stock_fixture();
    let reuse = reuse();
    let calls = AtomicUsize::new(0);
    let mut probe = fixture.config.clone();
    let mut thresholds = probe.effective_thresholds();
    thresholds.discard_suspected_ocr_noise = true;
    probe.quality_thresholds = Some(thresholds);
    let original = stock_read(&fixture, &reuse, &probe, &calls).await.unwrap();
    assert!(
        matches!(&original.metadata.format, Some(crate::types::FormatMetadata::Ocr(meta)) if meta.language == "eng")
    );
    assert!(reuse.finish(&[1]));
    let reused = stock_read(&fixture, &reuse, &fixture.config, &calls).await.unwrap();
    assert_eq!(
        calls.load(SeqCst),
        1,
        "count the actual builtin API call, with its disk cache disabled"
    );
    assert_eq!(
        serde_json::to_value(reused).unwrap(),
        serde_json::to_value(original).unwrap()
    );
}

#[cfg(feature = "bundle-tessdata-eng")]
#[tokio::test]
async fn stock_builtin_does_not_reuse_a_different_decode_budget() {
    let fixture = stock_fixture();
    let reuse = reuse();
    let calls = AtomicUsize::new(0);
    stock_read(&fixture, &reuse, &fixture.config, &calls).await.unwrap();
    assert!(reuse.finish(&[1]));
    let mut restricted = fixture.config.clone();
    let mut limits = crate::extractors::security::SecurityLimits::default();
    limits.max_content_size -= 1;
    restricted.security_limits = Some(limits);
    stock_read(&fixture, &reuse, &restricted, &calls).await.unwrap();
    assert_eq!(
        calls.load(SeqCst),
        2,
        "an effective reserved-budget change must invoke the builtin again"
    );
}

#[tokio::test]
async fn custom_backend_reads_changed_encoding_even_when_rgb_pixels_match() {
    use image::ImageEncoder;
    let mut fixture = Fixture::new();
    fixture.backend = Arc::clone(&fixture.reader);
    let reuse = reuse();
    fixture.read(&reuse, 1).await.unwrap();
    assert!(!reuse.finish(&[1]));
    let pixels = image::load_from_memory(fixture.pixels.as_slice()).unwrap().to_rgb8();
    let mut encoded = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut encoded,
        image::codecs::png::CompressionType::Best,
        image::codecs::png::FilterType::NoFilter,
    )
    .write_image(
        pixels.as_raw(),
        pixels.width(),
        pixels.height(),
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
    assert_ne!(encoded.as_slice(), fixture.pixels.as_slice());
    assert_eq!(image::load_from_memory(&encoded).unwrap().to_rgb8(), pixels);
    let encoded = Arc::new(encoded);
    let read = run(
        &reuse,
        Call {
            bytes: &encoded,
            ..fixture.call(1)
        },
    )
    .await
    .unwrap();
    assert_eq!(
        fixture.fake.calls(),
        2,
        "a non-built-in backend receives its exact encoded input"
    );
    assert_eq!(read.content, format!("read 2 of {} bytes", encoded.len()));
}

#[tokio::test]
async fn model_change_alone_requires_a_fresh_backend_read() {
    let fixture = Fixture::new();
    let reuse = reuse();
    fixture.read(&reuse, 1).await.unwrap();
    assert!(reuse.finish(&[1]));
    assert_eq!(fixture.fake.version(), "v0");
    std::fs::write(fixture._models.path().join("eng.traineddata"), b"fake model b").unwrap();
    let read = fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(fixture.fake.version(), "v0");
    assert_eq!(fixture.fake.calls(), 2, "only the model bytes changed");
    assert_eq!(
        reuse.model_hashes.load(SeqCst),
        1,
        "a changed file invalidates the snapshot"
    );
    assert_eq!(read.content, format!("read 2 of {} bytes", fixture.pixels.len()));
}

#[tokio::test]
async fn changed_policy_preserves_fresh_source_words_and_table_payload() {
    let mut fixture = Fixture::new();
    fixture.fake.policy_sensitive.store(true, SeqCst);
    fixture.effective.tesseract_config = Some(crate::types::TesseractConfig {
        psm: Some(3),
        ..Default::default()
    });
    let reuse = reuse();
    let probe = fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(probe.content, "The committee meets on Tuesday.");
    assert_eq!(probe.tables[0].cells, vec![vec!["Day"], vec!["Tuesday"]]);
    assert!(reuse.finish(&[1]));
    let mut changed = fixture.effective.clone();
    changed.tesseract_config.as_mut().unwrap().psm = Some(6);
    let read = run(
        &reuse,
        Call {
            effective: &changed,
            ..fixture.call(1)
        },
    )
    .await
    .unwrap();
    assert_eq!(
        fixture.fake.calls(),
        2,
        "only the effective page segmentation mode changed"
    );
    assert_eq!(read.content, "The committee meets on Thursday.");
    assert_eq!(read.tables[0].cells, vec![vec!["Day"], vec!["Thursday"]]);
    let reused = fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(fixture.fake.calls(), 2, "the original policy still reuses its own read");
    assert_eq!(reused.content, "The committee meets on Tuesday.");
    assert_eq!(reused.tables[0].cells, vec![vec!["Day"], vec!["Tuesday"]]);
}

#[cfg(feature = "bundle-tessdata-eng")]
#[tokio::test]
async fn stock_builtin_does_not_reuse_a_different_encoded_working_size() {
    use image::ImageEncoder;
    let mut fixture = stock_fixture();
    let reuse = reuse();
    let calls = AtomicUsize::new(0);
    stock_read(&fixture, &reuse, &fixture.config, &calls).await.unwrap();
    assert!(reuse.finish(&[1]));
    let pixels = image::load_from_memory(fixture.pixels.as_slice()).unwrap().to_rgb8();
    let mut encoded = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut encoded,
        image::codecs::png::CompressionType::Best,
        image::codecs::png::FilterType::NoFilter,
    )
    .write_image(
        pixels.as_raw(),
        pixels.width(),
        pixels.height(),
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
    assert_ne!(
        encoded.len(),
        fixture.pixels.len(),
        "the encoded workspace cost changed"
    );
    assert_eq!(image::load_from_memory(&encoded).unwrap().to_rgb8(), pixels);
    fixture.pixels = Arc::new(encoded);
    stock_read(&fixture, &reuse, &fixture.config, &calls).await.unwrap();
    assert_eq!(
        calls.load(SeqCst),
        2,
        "identical RGB can have a different encoded working cost"
    );
}

#[tokio::test]
async fn model_contents_are_hashed_once_for_two_probe_reads_and_two_hits() {
    let fixture = Fixture::new();
    let reuse = reuse();
    for page in [1, 2] {
        fixture.read(&reuse, page).await.unwrap();
    }
    assert!(reuse.finish(&[1, 2]));
    for page in [1, 2] {
        fixture.read(&reuse, page).await.unwrap();
    }
    assert_eq!(fixture.fake.calls(), 2, "both retained reads must actually be consumed");
    assert_eq!(
        reuse.model_hashes.load(SeqCst),
        1,
        "hash one model snapshot for the extraction"
    );
}

fn fabricated_key(tag: &[u8]) -> ReadKey {
    ReadKey {
        digest: blake3::hash(tag),
        revision: (String::new(), blake3::hash(tag)),
    }
}

fn tesseract_typed_backend() -> Arc<dyn OcrBackend> {
    Fake::new(OcrBackendType::Tesseract)
}

#[tokio::test]
async fn tesseract_typed_plugin_that_is_not_the_builtin_reads_every_time() {
    let mut fixture = Fixture::new();
    fixture.backend = Arc::clone(&fixture.reader);
    assert!(matches!(fixture.backend.backend_type(), OcrBackendType::Tesseract));
    assert!(
        !crate::plugins::registry::get_ocr_backend_registry()
            .read()
            .is_builtin_tesseract(&fixture.backend),
        "premise: the fake only claims to be Tesseract"
    );
    let reuse = reuse();
    let probe = fixture.read(&reuse, 1).await.unwrap();
    let retained = reuse.finish(&[1]);

    let final_read = fixture.read(&reuse, 1).await.unwrap();
    assert_eq!(
        fixture.fake.calls(),
        2,
        "a plugin outside the registry builtin must be invoked for the final read"
    );
    assert_eq!(final_read.content, format!("read 2 of {} bytes", fixture.pixels.len()));
    assert_ne!(final_read.content, probe.content);
    assert!(!retained);
    assert_eq!(reuse.model_hashes.load(SeqCst), 0);
}

fn hidden(formatted: String) -> ExtractedDocument {
    ExtractedDocument {
        content: "tiny".to_string(),
        formatted_content: Some(formatted),
        ..Default::default()
    }
}

fn oversized() -> String {
    "x".repeat(2 << 20)
}

fn child_entry() -> crate::types::ArchiveEntry {
    crate::types::ArchiveEntry {
        path: "inner.txt".to_string(),
        mime_type: "text/plain".to_string(),
        result: Box::new(hidden(oversized())),
    }
}

fn image_with_ocr() -> crate::types::ExtractedImage {
    crate::types::ExtractedImage {
        ocr_result: Some(Box::new(hidden(oversized()))),
        ..Default::default()
    }
}

fn nested_oversized_reads() -> Vec<(&'static str, ExtractedDocument)> {
    vec![
        (
            "child ArchiveEntry formatted_content",
            ExtractedDocument {
                content: "tiny".to_string(),
                children: Some(vec![child_entry()]),
                ..Default::default()
            },
        ),
        (
            "image ocr_result formatted_content",
            ExtractedDocument {
                content: "tiny".to_string(),
                images: Some(vec![image_with_ocr()]),
                ..Default::default()
            },
        ),
        (
            "ocr_internal_document child",
            ExtractedDocument {
                content: "tiny".to_string(),
                ocr_internal_document: Some(crate::types::internal::InternalDocument {
                    children: Some(vec![child_entry()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ),
        (
            "internal_document image OCR",
            ExtractedDocument {
                content: "tiny".to_string(),
                internal_document: Some(crate::types::internal::InternalDocument {
                    images: vec![image_with_ocr()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        ),
    ]
}

#[test]
fn nested_skipped_payload_over_the_limit_is_not_retained() {
    let limit = 1 << 20;
    assert!(
        retained_size(&hidden("tiny".to_string()), limit).is_some(),
        "control: a tiny top-level read fits, so the limit alone refuses nothing"
    );
    let backend = tesseract_typed_backend();
    for (index, (what, document)) in nested_oversized_reads().into_iter().enumerate() {
        assert!(
            serde_json::to_vec(&document).unwrap().len() < 1 << 16,
            "premise: {what} serializes small"
        );
        assert_eq!(retained_size(&document, limit), None, "{what} must not pass sizing");

        let reuse = Arc::new(ProbeReads::new(limit));
        let tag = format!("nested-{index}");
        reuse.insert(1, Arc::clone(&backend), fabricated_key(tag.as_bytes()), &document);
        assert!(!reuse.finish(&[1]), "{what} must not be retained");
        assert!(
            reuse.take(1, &fabricated_key(tag.as_bytes()), &backend).is_none(),
            "{what}: the final read has nothing to take and reads fresh"
        );
    }
}

#[test]
fn retention_cap_does_not_follow_a_huge_requested_content_limit() {
    let requested = 1 << 30;
    let reuse = Arc::new(ProbeReads::new(requested));
    let backend = tesseract_typed_backend();
    let document = ExtractedDocument {
        content: "x".repeat(9 << 20),
        ..Default::default()
    };
    let visible = serde_json::to_vec(&document).unwrap().len();
    assert!(visible > 8 << 20 && visible < requested, "premise: {visible} bytes");

    reuse.insert(1, Arc::clone(&backend), fabricated_key(b"cap"), &document);
    assert!(
        !reuse.finish(&[1]),
        "a 9 MiB read must exceed the dedicated retention cap whatever max_content_size asks for"
    );
}

#[test]
fn insert_clones_the_read_before_waiting_for_the_state_lock() {
    use std::time::{Duration, Instant};
    let reuse = reuse();
    let backend = tesseract_typed_backend();
    let table = Arc::new(crate::types::Table {
        cells: vec![vec!["Day".into()], vec!["Tuesday".into()]],
        ..Default::default()
    });
    let document = ExtractedDocument {
        content: "The committee meets on Tuesday.".to_string(),
        pages: Some(vec![crate::types::PageContent {
            page_number: 1,
            content: "The committee meets on Tuesday.".to_string(),
            tables: vec![Arc::clone(&table)],
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
        }]),
        ..Default::default()
    };
    assert!(serde_json::to_vec(&document).unwrap().len() < 1 << 16);
    let baseline = Arc::strong_count(&table);
    let started = std::sync::Barrier::new(2);

    let observed = std::thread::scope(|scope| {
        let guard = reuse.state.lock();
        let (reuse, backend, started) = (&reuse, &backend, &started);
        let worker = scope.spawn(move || {
            started.wait();
            reuse.insert(1, Arc::clone(backend), fabricated_key(b"lock"), &document);
        });
        started.wait();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut observed = false;
        while Instant::now() < deadline {
            if Arc::strong_count(&table) > baseline {
                observed = true;
                break;
            }
            std::thread::yield_now();
        }
        drop(guard);
        worker.join().unwrap();
        observed
    });

    assert!(reuse.finish(&[1]), "the read was retained once the lock was free");
    assert!(
        observed,
        "the retained copy must be made before the State lock is needed, not while another holder blocks it"
    );
}
