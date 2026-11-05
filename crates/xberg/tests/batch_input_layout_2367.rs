//! Real batch transport and isolation controls for per-input layout options.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use xberg::engine::Engine;
use xberg::engine::seams::CacheBackend;
use xberg::plugins::{InternalDocumentExtractor, Plugin, register_document_extractor, unregister_document_extractor};
use xberg::types::internal::InternalDocument;
use xberg::types::internal_builder::InternalDocumentBuilder;
use xberg::{
    ConcurrencyConfig, ExtractInput, ExtractInputWithOptions, ExtractionConfig, FileExtractionConfig, OutputFormat,
};

const PLUGIN: &str = "per-input-layout-2367";

#[derive(Clone, Debug)]
struct Receipt {
    identity: String,
    before: bool,
    after: bool,
    quality: bool,
    strict: bool,
    batch_limit: Option<usize>,
}

#[derive(Default)]
struct Observed {
    receipts: Mutex<Vec<Receipt>>,
    active: AtomicUsize,
    peak: AtomicUsize,
}

struct Probe {
    observed: Arc<Observed>,
    barrier: Option<Arc<tokio::sync::Barrier>>,
}

impl Plugin for Probe {
    fn name(&self) -> &str {
        PLUGIN
    }
}

#[async_trait::async_trait]
impl InternalDocumentExtractor for Probe {
    async fn extract_content(
        &self,
        bytes: &[u8],
        mime: &str,
        config: &ExtractionConfig,
    ) -> xberg::Result<InternalDocument> {
        assert_eq!(mime, "text/plain");
        let identity = String::from_utf8(bytes.to_vec()).unwrap();
        let before = config.use_layout_for_markdown;
        let active = self.observed.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.observed.peak.fetch_max(active, Ordering::SeqCst);
        if let Some(barrier) = &self.barrier {
            barrier.wait().await;
        }
        tokio::task::yield_now().await;
        self.observed.receipts.lock().unwrap().push(Receipt {
            identity: identity.clone(),
            before,
            after: config.use_layout_for_markdown,
            quality: config.enable_quality_processing,
            strict: config.strict_parse,
            batch_limit: config.max_concurrent_extractions,
        });
        self.observed.active.fetch_sub(1, Ordering::SeqCst);
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph(&identity, vec![], None, None);
        let mut document = builder.build();
        document.mime_type = mime.to_string();
        Ok(document)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/plain"]
    }

    fn priority(&self) -> i32 {
        100
    }
}

struct Registered {
    observed: Arc<Observed>,
}

impl Registered {
    fn new(concurrent: bool) -> Self {
        let observed = Arc::new(Observed::default());
        register_document_extractor(Arc::new(Probe {
            observed: Arc::clone(&observed),
            barrier: concurrent.then(|| Arc::new(tokio::sync::Barrier::new(2))),
        }))
        .unwrap();
        Self { observed }
    }

    fn receipts(&self) -> Vec<Receipt> {
        let mut receipts = self.observed.receipts.lock().unwrap().clone();
        receipts.sort_by(|left, right| left.identity.cmp(&right.identity));
        receipts
    }
}

impl Drop for Registered {
    fn drop(&mut self) {
        unregister_document_extractor(PLUGIN).unwrap();
    }
}

fn config(base: bool) -> ExtractionConfig {
    ExtractionConfig {
        use_layout_for_markdown: base,
        use_cache: false,
        disable_ocr: true,
        enable_quality_processing: false,
        output_format: OutputFormat::Plain,
        max_concurrent_extractions: Some(2),
        concurrency: Some(ConcurrencyConfig {
            max_threads: Some(4),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    }
}

fn item(identity: &str, setting: Option<bool>) -> ExtractInputWithOptions {
    let mut item = ExtractInputWithOptions::new(ExtractInput::from_bytes(
        identity.as_bytes().to_vec(),
        "text/plain",
        None,
    ));
    item.use_layout_for_markdown = setting;
    item
}

fn assert_output(output: &xberg::ExtractionResult, identities: &[&str]) {
    assert_eq!(output.summary.inputs, identities.len());
    assert_eq!(output.summary.results, identities.len());
    assert_eq!(output.summary.errors, 0);
    assert!(output.errors.is_empty());
    assert_eq!(
        output
            .results
            .iter()
            .map(|result| result.content.as_str())
            .collect::<Vec<_>>(),
        identities
    );
}

async fn base_matrix(base: bool) {
    let registered = Registered::new(false);
    let config = config(base);
    let before = serde_json::to_value(&config).unwrap();
    let output = xberg::extract_batch_with_input_options(
        vec![item("a", Some(true)), item("b", None), item("c", Some(false))],
        &config,
    )
    .await
    .unwrap();
    assert_output(&output, &["a", "b", "c"]);
    let receipts = registered.receipts();
    assert_eq!(receipts.len(), 3);
    assert!(receipts.iter().all(|row| row.before == row.after));
    assert!(
        receipts
            .iter()
            .all(|row| row.batch_limit == Some(2) && !row.quality && !row.strict)
    );
    assert_eq!(serde_json::to_value(&config).unwrap(), before);
    assert_eq!(
        receipts.iter().map(|row| row.before).collect::<Vec<_>>(),
        [true, base, false]
    );
}

#[tokio::test]
#[serial_test::serial]
async fn real_batch_resolves_layout_options_from_false_base() {
    base_matrix(false).await;
}

#[tokio::test]
#[serial_test::serial]
async fn real_batch_resolves_layout_options_from_true_base() {
    base_matrix(true).await;
}

#[tokio::test]
#[serial_test::serial]
async fn omitted_options_inherit_default_false_without_affecting_legacy_inputs() {
    let registered = Registered::new(false);
    let config = config(false);
    assert!(!ExtractionConfig::default().use_layout_for_markdown);
    let output = Engine::new_default()
        .extract_batch_with_input_options(vec![item("inherit", None)], &config)
        .await
        .unwrap();
    assert_output(&output, &["inherit"]);
    let receipts = registered.receipts();
    assert_eq!(receipts.len(), 1);
    assert_eq!((receipts[0].before, receipts[0].after), (false, false));
}

#[tokio::test]
#[serial_test::serial]
async fn layout_override_preserves_existing_file_override_precedence() {
    let registered = Registered::new(false);
    let mut config = config(true);
    config.enable_quality_processing = true;
    let before = serde_json::to_value(&config).unwrap();
    let mut input = item("precedence", Some(false));
    input.input.config = Some(FileExtractionConfig {
        enable_quality_processing: Some(false),
        strict_parse: Some(true),
        ..Default::default()
    });
    let output = Engine::new_default()
        .extract_batch_with_input_options(vec![input], &config)
        .await
        .unwrap();
    assert_output(&output, &["precedence"]);
    let receipts = registered.receipts();
    assert_eq!(receipts.len(), 1);
    assert!(!receipts[0].quality && receipts[0].strict);
    assert_eq!(receipts[0].batch_limit, Some(2));
    assert_eq!(serde_json::to_value(&config).unwrap(), before);
    assert_eq!((receipts[0].before, receipts[0].after), (false, false));
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn concurrent_mixed_options_remain_paired_with_the_original_input() {
    let registered = Registered::new(true);
    let config = config(false);
    let before = serde_json::to_value(&config).unwrap();
    let operation = Engine::new_default();
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        operation.extract_batch_with_input_options(
            vec![
                item("d", None),
                item("a", Some(true)),
                item("c", Some(false)),
                item("b", Some(true)),
            ],
            &config,
        ),
    )
    .await
    .expect("the actual batch must schedule two extractor calls concurrently")
    .unwrap();
    assert_output(&output, &["d", "a", "c", "b"]);
    let receipts = registered.receipts();
    assert_eq!(receipts.len(), 4);
    assert_eq!(registered.observed.active.load(Ordering::SeqCst), 0);
    assert_eq!(registered.observed.peak.load(Ordering::SeqCst), 2);
    assert!(receipts.iter().all(|row| row.before == row.after));
    assert_eq!(serde_json::to_value(&config).unwrap(), before);
    assert_eq!(
        receipts
            .iter()
            .map(|row| (row.identity.as_str(), row.before))
            .collect::<Vec<_>>(),
        [("a", true), ("b", true), ("c", false), ("d", false)]
    );
}

#[derive(Default)]
struct MemoryCache {
    values: Mutex<HashMap<String, Vec<u8>>>,
    gets: AtomicUsize,
    puts: AtomicUsize,
}

#[async_trait::async_trait]
impl CacheBackend for MemoryCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.gets.fetch_add(1, Ordering::SeqCst);
        self.values.lock().unwrap().get(key).cloned()
    }

    async fn put(&self, key: &str, value: Vec<u8>, _ttl: Option<Duration>) {
        self.puts.fetch_add(1, Ordering::SeqCst);
        self.values.lock().unwrap().insert(key.to_string(), value);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn batch_cache_distinguishes_effective_layout_options_before_lookup() {
    let registered = Registered::new(false);
    let cache = Arc::new(MemoryCache::default());
    let engine = Engine::builder().with_cache_backend(cache.clone()).build();
    let namespace = tempfile::tempdir().unwrap();
    let mut config = config(false);
    config.use_cache = true;
    config.cache_namespace = Some(format!(
        "cfg2367-{}",
        namespace.path().file_name().unwrap().to_str().unwrap()
    ));
    let output = engine
        .extract_batch_with_input_options(vec![item("cache", Some(false))], &config)
        .await
        .unwrap();
    assert_output(&output, &["cache"]);
    assert_eq!(registered.receipts().len(), 1);
    assert_eq!(cache.gets.load(Ordering::SeqCst), 1);
    assert_eq!(cache.puts.load(Ordering::SeqCst), 1);
    for _ in 0..2 {
        let output = engine
            .extract_batch_with_input_options(vec![item("cache", Some(true))], &config)
            .await
            .unwrap();
        assert_output(&output, &["cache"]);
    }
    assert_eq!(cache.gets.load(Ordering::SeqCst), 3);
    assert_eq!(
        registered.receipts().len(),
        2,
        "a changed effective flag must miss; its repeat must hit"
    );
    assert_eq!(cache.puts.load(Ordering::SeqCst), 2);
    assert_eq!(
        registered.receipts().iter().map(|row| row.before).collect::<Vec<_>>(),
        [false, true]
    );
}
