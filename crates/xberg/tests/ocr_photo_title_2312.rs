#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig, PageConfig, extract};

const TITLE: &str = "SECTION TITLE";
const TITLE_SUFFIX: &str = "\n\nSECTION TITLE";
const PHOTO: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/title.png");
const MIXED: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/mixed-sign.png");
const TABLE: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/table.png");
const PARTIAL: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/partial.png");
const NUMERIC: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/numeric.png");

fn png_config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..PageConfig::default()
        }),
        ..ExtractionConfig::default()
    }
}

async fn extract_png(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let input = ExtractInput::from_bytes(bytes.to_vec(), "image/png", None);
    let mut batch = extract(input, config).await.expect("extract the synthetic PNG");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    assert_eq!(batch.results.len(), 1);
    let document = batch.results.remove(0);
    assert_eq!(document.mime_type, "image/png");
    assert!(document.metadata.ocr_used);
    assert_eq!(document.metadata.additional["psm"], "11");
    assert_eq!(document.metadata.additional["ocr_processed_image_width"], 1700);
    assert_eq!(document.metadata.additional["ocr_processed_image_height"], 1100);
    document
}

async fn extract_primary(bytes: &[u8], word_count: u32, table_count: usize) -> ExtractedDocument {
    let document = extract_png(bytes, &png_config()).await;
    assert_eq!(document.tables.len(), table_count);
    let pages = document.pages.as_ref().expect("requested page output");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    let confidence = pages[0].ocr_confidence.as_ref().expect("original OCR confidence");
    assert_eq!(confidence.word_count, word_count);
    assert_eq!(confidence.backend, "tesseract");
    document
}

fn assert_body(text: &str, expected_sha: &str) {
    assert_eq!(hex::encode(Sha256::digest(text.as_bytes())), expected_sha, "{text:?}");
}

fn assert_addition(document: &ExtractedDocument, body_sha: &str, page_sha: &str) {
    let body = document
        .content
        .strip_suffix(TITLE_SUFFIX)
        .expect("append the independently recovered title to the unchanged OCR body");
    assert!(!body.contains(TITLE));
    assert_body(body, body_sha);
    let page = &document.pages.as_ref().unwrap()[0].content;
    let page_body = page
        .trim()
        .strip_suffix(TITLE_SUFFIX)
        .expect("append the same title on page one");
    assert_body(page_body, page_sha);
}

#[tokio::test]
async fn recovers_the_photo_title_and_keeps_the_entire_primary_body() {
    let document = extract_primary(PHOTO, 215, 0).await;
    assert_addition(
        &document,
        "56e3599e161041150f8a5363b1b8f8b6acf5ceac11adcae6528b5c47528d403e",
        "2ab44fd7d097fd8353d7c6f346ce3a4ba370e64fec2871d7131de9382012507c",
    );
}

#[tokio::test]
async fn recovers_the_photo_title_without_losing_the_independent_sign_or_amount() {
    let document = extract_primary(MIXED, 6, 0).await;
    assert!(document.content.contains("NORTH GATE 42"), "{:?}", document.content);
    assert!(document.content.contains("AUTHORIZED ONLY"), "{:?}", document.content);
    assert_addition(
        &document,
        "5b429f4f81a26747baf73316a7400b5edf39313eff0ce8dc93db6539cbbe61ff",
        "2b2f97eaeab18ccfa4ff22841fd76d969d241a34c948b0faab1844c32c20f6ac",
    );
}

#[tokio::test]
async fn leaves_accepted_tables_and_their_body_untouched() {
    let document = extract_primary(TABLE, 318, 2).await;
    assert_body(
        &document.content,
        "e19005d542795b6b3dcb3ed3a4701452fe5fb31068ce7e3ef62decbaba03fdbc",
    );
    assert_body(
        document.pages.unwrap()[0].content.trim(),
        "9fb98e2329424ee916c81a92569aab56565a3c9021ef80774ed061139c8c826a",
    );
}

#[tokio::test]
async fn leaves_a_partially_recognized_sign_at_its_existing_location() {
    let document = extract_primary(PARTIAL, 21, 0).await;
    assert!(document.content.lines().any(|line| line == "NORTH"));
    assert!(!document.content.contains("NORTH GATE"));
    assert_body(
        &document.content,
        "e0c04b85d5e3263df7e7302708b39bad37c54d6e322e74c1f3afa7f2ad202ff6",
    );
}

#[tokio::test]
async fn leaves_a_recognized_numeric_sign_untouched() {
    let document = extract_primary(NUMERIC, 3, 0).await;
    assert_eq!(document.content, "NORTH GATE 42");
}

#[tokio::test]
async fn leaves_requested_document_structure_untouched() {
    let config = ExtractionConfig {
        include_document_structure: true,
        ..png_config()
    };
    let document = extract_png(PHOTO, &config).await;
    assert!(document.document.is_some(), "structured extraction was requested");
    assert!(!document.content.contains(TITLE));
    assert!(!document.pages.unwrap()[0].content.contains(TITLE));
}

const CACHE_CASE: &str = "recovers_the_photo_title_with_extraction_cache_enabled";
const CACHE_CHILD: &str = "XBERG_PHOTO_TITLE_CACHE_TEST_CHILD";

#[tokio::test]
async fn recovers_the_photo_title_with_extraction_cache_enabled() {
    if std::env::var(CACHE_CHILD).as_deref() == Ok(CACHE_CASE) {
        check_photo_title_cache_paths().await;
    } else {
        run_isolated_cache_test();
    }
}

fn run_isolated_cache_test() {
    let root = tempfile::tempdir().expect("isolated OCR data and cache");
    let tessdata = root.path().join("tessdata");
    std::fs::create_dir(&tessdata).expect("create isolated tessdata");
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English fixture");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.join("eng.traineddata"), english).expect("write bundled English fixture");
    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", CACHE_CASE, "--nocapture"])
        .env(CACHE_CHILD, CACHE_CASE)
        .env("TESSDATA_PREFIX", tessdata)
        .env("XBERG_CACHE_DIR", root.path().join("cache"))
        .output()
        .expect("run isolated OCR test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success()
            && stdout
                .lines()
                .any(|line| { line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;") }),
        "isolated OCR test failed: {stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn check_photo_title_cache_paths() {
    use std::sync::{Arc, atomic::Ordering};
    use tracing_subscriber::layer::SubscriberExt as _;

    let lookups = OcrCacheLookups::default();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(lookups.clone()))
        .expect("capture OCR worker cache events in the isolated process");

    let cache = Arc::new(CountedCache::default());
    let engine = xberg::engine::Engine::builder()
        .with_cache_backend(cache.clone())
        .build();
    let config = ExtractionConfig {
        use_cache: ExtractionConfig::default().use_cache,
        ..png_config()
    };
    assert!(config.use_cache, "exercise the default cache policy");
    let cold = read_engine(&engine, PHOTO, &config).await;
    assert_cached_title(&cold);
    assert_eq!(cache.writes.load(Ordering::SeqCst), 1);
    assert_eq!(cache.hits.load(Ordering::SeqCst), 0);
    let first_lookup = lookups.0.lock().unwrap().clone();
    assert_eq!(first_lookup.len(), 1, "the cold primary should do one lookup");
    let (key, hit) = &first_lookup[0];
    assert!(!hit, "the child starts with a cold disk cache");

    let outer_warm = read_engine(&engine, PHOTO, &config).await;
    assert_same_document(&cold, &outer_warm);
    assert_eq!(cache.hits.load(Ordering::SeqCst), 1);
    assert_eq!(cache.writes.load(Ordering::SeqCst), 1);
    assert_eq!(*lookups.0.lock().unwrap(), first_lookup);

    let fresh_cache = Arc::new(CountedCache::default());
    let fresh_engine = xberg::engine::Engine::builder()
        .with_cache_backend(fresh_cache.clone())
        .build();
    let inner_warm = read_engine(&fresh_engine, PHOTO, &config).await;
    assert_same_document(&cold, &inner_warm);
    assert_eq!(fresh_cache.hits.load(Ordering::SeqCst), 0);
    assert_eq!(fresh_cache.writes.load(Ordering::SeqCst), 1);
    assert_eq!(
        *lookups.0.lock().unwrap(),
        [(key.clone(), false), (key.clone(), true)],
        "a second Engine must miss its outer cache and hit the same built-in OCR key"
    );

    let uncached = png_config();
    assert_same_document(&cold, &read_engine(&engine, PHOTO, &uncached).await);
    for fixture in [NUMERIC, TABLE] {
        let baseline = read_engine(&engine, fixture, &uncached).await;
        let cache_enabled = read_engine(&engine, fixture, &config).await;
        assert_same_document(&baseline, &cache_enabled);
        assert!(!cache_enabled.content.contains(TITLE));
    }
}

fn assert_cached_title(document: &ExtractedDocument) {
    assert_addition(
        document,
        "56e3599e161041150f8a5363b1b8f8b6acf5ceac11adcae6528b5c47528d403e",
        "2ab44fd7d097fd8353d7c6f346ce3a4ba370e64fec2871d7131de9382012507c",
    );
    assert!(document.ocr_elements.is_none());
    assert!(document.tables.is_empty());
    let pages = document.pages.as_ref().expect("requested pages");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    let confidence = pages[0].ocr_confidence.as_ref().expect("original primary confidence");
    assert_eq!(confidence.word_count, 215);
    assert_eq!(confidence.backend, "tesseract");
}

fn assert_same_document(expected: &ExtractedDocument, actual: &ExtractedDocument) {
    assert_eq!(
        serde_json::to_value(expected).unwrap(),
        serde_json::to_value(actual).unwrap(),
        "the complete public document must be identical across cache modes"
    );
}

#[derive(Default)]
struct CountedCache {
    entries: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    hits: std::sync::atomic::AtomicUsize,
    writes: std::sync::atomic::AtomicUsize,
}

#[async_trait::async_trait]
impl xberg::engine::seams::CacheBackend for CountedCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let value = self.entries.lock().unwrap().get(key).cloned();
        if value.is_some() {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        value
    }

    async fn put(&self, key: &str, value: Vec<u8>, _ttl: Option<std::time::Duration>) {
        self.entries.lock().unwrap().insert(key.into(), value);
        self.writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Default)]
struct CacheLookup {
    key: Option<String>,
    hit: Option<bool>,
}

impl tracing::field::Visit for CacheLookup {
    fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if field.name() == "xberg.cache.hit" {
            self.hit = Some(value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "xberg.cache.key" {
            self.key = Some(value.to_owned());
        }
    }
}

#[derive(Clone, Default)]
struct OcrCacheLookups(std::sync::Arc<std::sync::Mutex<Vec<(String, bool)>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for OcrCacheLookups {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        if event.metadata().target() != "xberg::ocr::cache" {
            return;
        }
        let mut lookup = CacheLookup::default();
        event.record(&mut lookup);
        if let (Some(key), Some(hit)) = (lookup.key, lookup.hit) {
            self.0.lock().unwrap().push((key, hit));
        }
    }
}

async fn read_engine(engine: &xberg::engine::Engine, bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let input = ExtractInput::from_bytes(bytes.to_vec(), "image/png", None);
    let mut batch = engine.extract(input, config).await.expect("extract the synthetic PNG");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    assert_eq!(batch.results.len(), 1);
    batch.results.remove(0)
}
