//! Rust-only extraction engine.
//!
//! [`Engine`] owns the extraction internals that previously lived as free
//! functions in [`mod@crate::core::extract`]. The crate-level [`crate::extract`]
//! and [`crate::extract_batch`] functions delegate to a process-global default
//! [`Engine`].
//!
//! This is a Rust-only API. Language bindings expose the crate-level extraction
//! functions instead.

use std::sync::Arc;

use crate::Result;
#[cfg(all(feature = "redaction", feature = "tokio-runtime"))]
use crate::core::config::ExtractInputKind;
#[cfg(feature = "redaction")]
use crate::core::config::{ExternalRedactionFinding, RedactionOffsetEncoding};
use crate::core::config::{ExtractInput, ExtractInputWithOptions, ExtractionConfig, ExtractionResult};

#[cfg(all(feature = "url-ingestion", feature = "tokio-runtime", not(target_arch = "wasm32")))]
mod crawl_handle;
mod extract_impl;

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(crate) mod layout_region_text;
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub use layout_region_text::{LayoutRegionText, LayoutRegionTextExtraction};
pub mod seams;

/// Generic, MIT-clean structured-extraction mechanism (rasterize, chunk, schema,
/// citations, prompts). Gated on `heuristics` because the mechanism speaks in
/// heuristics call-mode / confidence types; individual submodules carry further
/// gates for their own hard dependencies (`pdf`, `presets`). Not part of the
/// binding surface — see `alef.toml` `[crates.exclude]`.
#[cfg(feature = "heuristics")]
pub mod structured;

/// Single-operation parse memo (MIME, page count, shared bytes, lazily-rendered
/// pages). Depends on the structured rasterizer, so it carries that module's
/// `pdf` gate in addition to `heuristics`. Not part of the binding surface —
/// see `alef.toml` `[crates.exclude]`.
#[cfg(all(feature = "heuristics", feature = "pdf"))]
pub mod parsed;

use seams::{CacheBackend, NoopCache, NoopProgressSink, ProgressSink};

/// Internal engine state.
///
/// Holds the process-shared, fingerprinted crawl-engine memo so that multi-URL
/// batch extraction can reuse a single [`crawlberg::CrawlEngine`] (and its
/// shared middleware/cache/rate-limiter) across all URLs in a batch, plus the
/// injected cache and progress seams. The single-URL `extract` path does not
/// touch the crawl state.
struct EngineInner {
    /// Single-slot, fingerprinted memo of the last-built crawl engine. The slot
    /// is reused when the incoming [`crawlberg::CrawlConfig`] fingerprint
    /// matches, otherwise a fresh engine is built and stored.
    #[cfg(all(feature = "url-ingestion", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    crawl: parking_lot::Mutex<Option<crawl_handle::CrawlHandleMemo>>,

    /// Content-addressed byte cache. Default: [`NoopCache`].
    cache: Arc<dyn CacheBackend>,
    cache_enabled: bool,
    /// Progress event sink. Default: [`NoopProgressSink`].
    progress: Arc<dyn ProgressSink>,
}

/// A reusable, cheaply-cloneable extraction engine.
///
/// Cloning an [`Engine`] shares the same underlying state via [`Arc`].
#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

impl Engine {
    /// Start building an [`Engine`].
    pub fn builder() -> EngineBuilder {
        EngineBuilder::default()
    }

    /// Construct an [`Engine`] with default configuration.
    pub fn new_default() -> Self {
        EngineBuilder::default().build()
    }

    /// Extract content from a single bytes or URI input.
    ///
    /// Honours the injected [`CacheBackend`] and [`ProgressSink`] seams: a bytes
    /// input whose content-hash cache key already has an entry short-circuits
    /// straight to the cached [`ExtractionResult`], and every call emits coarse
    /// `ProgressEvent`s (start, then either completion or error, plus a
    /// cache-hit event when one occurs). Both seams default to no-ops
    /// ([`NoopCache`], [`NoopProgressSink`]), so callers who inject nothing see
    /// byte-identical behavior to before this wiring existed.
    ///
    /// The work runs as a task on the shared 16 MiB-stack runtime, so a caller on a small host thread (an FFI
    /// caller of 1 to 2 MB) cannot overflow it. The task keeps the caller's tracing span and subscriber, but
    /// the caller's tokio task-locals do not cross into the extraction. It costs one task spawn and one clone
    /// of `config` per call.
    pub async fn extract(&self, input: ExtractInput, config: &ExtractionConfig) -> Result<ExtractionResult> {
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let result = {
            let (engine, config) = (self.clone(), config.clone());
            let work = async move { Box::pin(engine.extract_on_current_stack(input, &config)).await };
            #[cfg(test)]
            let work = inherit_test_task_locals(work);
            crate::core::runtime::run_on_large_stack(work).await?
        };

        #[cfg(not(all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
        let result = self.extract_on_current_stack(input, config).await;

        result
    }

    /// The body of [`Engine::extract`], polled on the caller's stack.
    ///
    /// Callers that already run on the 16 MiB runtime, or that scope a task-local around the call, use this
    /// directly: a hop to a new task would not inherit the task-local.
    async fn extract_on_current_stack(
        &self,
        input: ExtractInput,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        #[cfg(feature = "tokio-runtime")]
        return seams::scope_progress(
            Arc::clone(&self.inner.progress),
            None,
            extract_impl::extract(&self.inner, input, config),
        )
        .await;

        #[cfg(not(feature = "tokio-runtime"))]
        extract_impl::extract(&self.inner, input, config).await
    }

    /// Extract one PDF bytes input using request-owned explicit layout-page selection. ~keep
    /// Enables the native layout path and bypasses both caches after resolving per-input overrides.
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    pub async fn extract_with_layout_pages(
        &self,
        input: ExtractInput,
        config: &ExtractionConfig,
        pages: Vec<u32>,
    ) -> Result<ExtractionResult> {
        let selection = crate::core::layout_pages::LayoutPageSelection::new(pages);
        self.extract_with_layout_selection(input, config, &selection).await
    }

    /// Extract one PDF bytes input and collect source-proven text for explicitly selected layout pages.
    ///
    /// This enables page extraction and native layout reading order, and bypasses both caches.
    /// Unsupported pages, unproved regions and redaction requests yield no region text.
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    #[cfg_attr(alef, alef(skip))]
    pub async fn extract_with_layout_region_text(
        &self,
        input: ExtractInput,
        config: &ExtractionConfig,
        pages: Vec<u32>,
    ) -> Result<LayoutRegionTextExtraction> {
        let mut selection = crate::core::layout_pages::LayoutPageSelection::new(pages);
        let pending: layout_region_text::RegionTextCollector = Default::default();
        selection.region_text = Some(Arc::clone(&pending));
        let extraction = self.extract_with_layout_selection(input, config, &selection).await?;
        let pending = std::mem::take(&mut *pending.lock());
        Ok(layout_region_text::finish(extraction, pending))
    }

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    async fn extract_with_layout_selection(
        &self,
        input: ExtractInput,
        config: &ExtractionConfig,
        selection: &crate::core::layout_pages::LayoutPageSelection,
    ) -> Result<ExtractionResult> {
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let result = {
            let (engine, config, selection) = (self.clone(), config.clone(), selection.clone());
            let work = async move {
                Box::pin(engine.extract_with_layout_selection_on_current_stack(input, &config, &selection)).await
            };
            #[cfg(test)]
            let work = inherit_test_task_locals(work);
            crate::core::runtime::run_on_large_stack(work).await?
        };

        #[cfg(not(all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
        let result = self
            .extract_with_layout_selection_on_current_stack(input, config, selection)
            .await;

        result
    }

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    async fn extract_with_layout_selection_on_current_stack(
        &self,
        input: ExtractInput,
        config: &ExtractionConfig,
        selection: &crate::core::layout_pages::LayoutPageSelection,
    ) -> Result<ExtractionResult> {
        // ~keep The native PDF future is large; erase it before progress/public wrappers embed its state.
        #[cfg(not(target_arch = "wasm32"))]
        let operation: std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ExtractionResult>> + Send + '_>,
        > = Box::pin(extract_impl::extract_with_layout_pages(
            &self.inner,
            input,
            config,
            selection,
        ));
        #[cfg(target_arch = "wasm32")]
        let operation: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>> + '_>> = Box::pin(
            extract_impl::extract_with_layout_pages(&self.inner, input, config, selection),
        );
        #[cfg(feature = "tokio-runtime")]
        return seams::scope_progress(Arc::clone(&self.inner.progress), None, operation).await;
        #[cfg(not(feature = "tokio-runtime"))]
        operation.await
    }

    /// Extract one bytes input and apply request-scoped external redaction findings.
    ///
    /// The findings run in the existing Late redaction stage, before an output
    /// renderer packages DOCX or PDF bytes. Scoped extraction bypasses both
    /// extraction caches because the findings intentionally do not become part
    /// of [`ExtractionConfig`]. `max_findings` defaults to 10,000 when omitted.
    #[cfg(feature = "redaction")]
    pub async fn extract_with_external_redaction(
        &self,
        input: ExtractInput,
        config: &ExtractionConfig,
        findings: Vec<ExternalRedactionFinding>,
        offset_encoding: RedactionOffsetEncoding,
        max_findings: Option<u32>,
    ) -> Result<ExtractionResult> {
        #[cfg(not(feature = "tokio-runtime"))]
        {
            let _ = (input, config, findings, offset_encoding, max_findings);
            return Err(crate::XbergError::validation(
                "external redaction extraction requires the tokio-runtime feature".to_string(),
            ));
        }

        #[cfg(feature = "tokio-runtime")]
        {
            let max_findings = max_findings.unwrap_or(crate::text::redaction::external::DEFAULT_MAX_FINDINGS);
            let default_limits = crate::extractors::security::SecurityLimits::default();
            let security_max = crate::text::redaction::external::security_finding_limit(
                config.security_limits.as_ref().unwrap_or(&default_limits),
            );
            if input.kind != ExtractInputKind::Bytes {
                return Err(crate::XbergError::validation(
                    "external redaction extraction accepts one bytes input; read URI input first".to_string(),
                ));
            }
            if findings.len() > max_findings as usize {
                return Err(crate::XbergError::validation(format!(
                    "redaction findings exceed maximum of {max_findings}"
                )));
            }
            if findings.len() > security_max {
                return Err(crate::XbergError::validation(format!(
                    "redaction findings exceed the effective redaction finding limit ({security_max})"
                )));
            }

            let include_configured_sources = config.redaction.is_some()
                || input
                    .config
                    .as_ref()
                    .is_some_and(|input_config| input_config.redaction.is_some());
            let request = crate::text::redaction::external::ExternalRedactionRequest::new(
                findings,
                offset_encoding,
                max_findings,
                include_configured_sources,
            );
            let mut uncached_config = config.clone();
            uncached_config.use_cache = false;
            let engine = self.clone();
            let work = async move {
                let result = crate::text::redaction::external::scope_external_redaction(
                    request.clone(),
                    engine.extract_on_current_stack(input, &uncached_config),
                )
                .await;
                if result.is_ok() && !request.was_consumed() {
                    return Err(crate::XbergError::validation(
                        "external redaction request did not reach the Late processor".to_string(),
                    ));
                }
                result
            };
            // The poll chain of this future exceeds the ~2 MB stack of an FFI host thread or a default tokio
            // worker, so it runs as a task on the shared 16 MiB runtime. wasm32 has no threads to spawn onto.
            #[cfg(not(target_arch = "wasm32"))]
            {
                let work: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>> + Send>> =
                    Box::pin(work);
                crate::core::runtime::run_on_large_stack(work).await?
            }
            #[cfg(target_arch = "wasm32")]
            work.await
        }
    }

    /// Extract content from multiple bytes or URI inputs.
    ///
    /// Honours the injected [`CacheBackend`] and [`ProgressSink`] seams. Complete,
    /// all-bytes batches are cached by content and configuration; batches containing
    /// URI inputs or per-input errors are not cached.
    pub async fn extract_batch(
        &self,
        inputs: Vec<ExtractInput>,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        self.extract_batch_inputs(inputs, config).await
    }

    /// Extract a batch whose inputs carry additional per-input options.
    pub async fn extract_batch_with_input_options(
        &self,
        inputs: Vec<ExtractInputWithOptions>,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        self.extract_batch_inputs(inputs, config).await
    }

    async fn extract_batch_inputs<I: extract_impl::BatchInputRef + Send + 'static>(
        &self,
        inputs: Vec<I>,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let result = {
            let (engine, config) = (self.clone(), config.clone());
            let work = async move {
                Box::pin(seams::scope_progress(
                    Arc::clone(&engine.inner.progress),
                    None,
                    extract_impl::extract_batch(&engine.inner, inputs, &config),
                ))
                .await
            };
            #[cfg(test)]
            let work = inherit_test_task_locals(work);
            crate::core::runtime::run_on_large_stack(work).await?
        };

        #[cfg(all(feature = "tokio-runtime", target_arch = "wasm32"))]
        let result = seams::scope_progress(
            Arc::clone(&self.inner.progress),
            None,
            extract_impl::extract_batch(&self.inner, inputs, config),
        )
        .await;

        #[cfg(not(feature = "tokio-runtime"))]
        let result = extract_impl::extract_batch(&self.inner, inputs, config).await;

        result
    }

    /// The injected [`CacheBackend`] seam (default: [`NoopCache`]).
    pub fn cache_backend(&self) -> &Arc<dyn CacheBackend> {
        &self.inner.cache
    }

    /// The injected [`ProgressSink`] seam (default: [`NoopProgressSink`]).
    pub fn progress_sink(&self) -> &Arc<dyn ProgressSink> {
        &self.inner.progress
    }
}

/// Builder for [`Engine`].
///
/// Cache and progress seams left unset are filled with no-op defaults by
/// [`build`](EngineBuilder::build).
#[derive(Default)]
pub struct EngineBuilder {
    cache: Option<Arc<dyn CacheBackend>>,
    progress: Option<Arc<dyn ProgressSink>>,
}

impl EngineBuilder {
    /// Inject a [`CacheBackend`], overriding the [`NoopCache`] default.
    pub fn with_cache_backend(mut self, cache: Arc<dyn CacheBackend>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Inject a [`ProgressSink`], overriding the [`NoopProgressSink`] default.
    pub fn with_progress_sink(mut self, progress: Arc<dyn ProgressSink>) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Finalize the builder into an [`Engine`].
    pub fn build(self) -> Engine {
        let cache_enabled = self.cache.is_some();
        let inner = EngineInner {
            #[cfg(all(feature = "url-ingestion", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            crawl: parking_lot::Mutex::new(None),
            cache: self.cache.unwrap_or_else(|| Arc::new(NoopCache)),
            cache_enabled,
            progress: self.progress.unwrap_or_else(|| Arc::new(NoopProgressSink)),
        };
        Engine { inner: Arc::new(inner) }
    }
}

/// Carries test-only task-locals scoped around an engine call across the hop to the large-stack runtime.
#[cfg(all(test, feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn inherit_test_task_locals<F>(
    future: F,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = F::Output> + Send + 'static>>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    // Boxed at each step: unoptimized builds otherwise stack the wrappers' states by value on the test thread.
    let future: std::pin::Pin<Box<dyn std::future::Future<Output = F::Output> + Send>> = Box::pin(future);
    #[cfg(all(feature = "pdf", feature = "ocr-pipeline"))]
    let future = Box::pin(crate::plugins::registry::inherit_test_ocr_backend(future));
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let future = Box::pin(crate::extractors::pdf::inherit_test_selected_page_detections(future));
    // The external-redaction scope never crosses a spawn in production; tests that scope it around a layout call do.
    #[cfg(feature = "redaction")]
    let future = {
        let request = crate::text::redaction::external::current_external_redaction();
        Box::pin(async move {
            match request {
                Some(request) => crate::text::redaction::external::scope_external_redaction(request, future).await,
                None => future.await,
            }
        })
    };
    future
}

#[cfg(all(test, feature = "tokio-runtime"))]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;
    use crate::types::ExtractedDocument;
    use seams::ProgressEvent;

    #[test]
    fn default_engine_disables_the_engine_cache_seam() {
        let engine = Engine::new_default();

        assert!(!engine.inner.cache_enabled);
    }

    #[test]
    fn injected_backend_enables_the_engine_cache_seam() {
        let engine = Engine::builder().with_cache_backend(Arc::new(NoopCache)).build();

        assert!(engine.inner.cache_enabled);
    }

    /// A [`ProgressSink`] that records the stage of every emitted event, in order.
    #[derive(Default)]
    struct RecordingProgressSink {
        stages: Mutex<Vec<String>>,
    }

    impl ProgressSink for RecordingProgressSink {
        fn emit(&self, event: ProgressEvent) {
            self.stages
                .lock()
                .expect("recording sink mutex poisoned")
                .push(event.stage);
        }
    }

    /// A [`ProgressSink`] that records the thread each event is emitted on.
    #[derive(Default)]
    struct ThreadRecordingSink {
        threads: Mutex<Vec<std::thread::ThreadId>>,
    }

    impl ProgressSink for ThreadRecordingSink {
        fn emit(&self, _event: ProgressEvent) {
            self.threads
                .lock()
                .expect("thread sink mutex poisoned")
                .push(std::thread::current().id());
        }
    }

    // A current-thread runtime stands in for an FFI host thread: every await runs on the caller's own stack.
    // Reverting `Engine::extract` to call `extract_on_current_stack` directly makes the first assertion fail.
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    #[test]
    fn extract_and_batch_run_on_the_large_stack_runtime_not_the_callers_thread() {
        let caller = std::thread::current().id();
        let sink = Arc::new(ThreadRecordingSink::default());
        let engine = Engine::builder().with_progress_sink(sink.clone()).build();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let input = || ExtractInput::from_bytes(b"hello stack".to_vec(), "text/plain", None);

        runtime
            .block_on(engine.extract(input(), &ExtractionConfig::default()))
            .unwrap();
        let single = std::mem::take(&mut *sink.threads.lock().unwrap());
        assert!(!single.is_empty(), "extract emitted no progress events");
        assert!(
            single.iter().all(|thread| *thread != caller),
            "extract polled on the caller's thread"
        );

        runtime
            .block_on(engine.extract_batch(vec![input()], &ExtractionConfig::default()))
            .unwrap();
        let batch = std::mem::take(&mut *sink.threads.lock().unwrap());
        assert!(!batch.is_empty(), "extract_batch emitted no progress events");
        assert!(
            batch.iter().all(|thread| *thread != caller),
            "extract_batch polled on the caller's thread"
        );

        runtime
            .block_on(engine.extract_on_current_stack(input(), &ExtractionConfig::default()))
            .unwrap();
        let direct = sink.threads.lock().unwrap().clone();
        assert!(
            direct.iter().all(|thread| *thread == caller),
            "the on-current-stack variant must stay on the caller's thread"
        );
    }

    // Revert line: change `Engine::extract` back to
    // `extract_impl::extract(input, config).await` (dropping `&self.inner`) to make
    // this test fail — `RecordingProgressSink::emit` is then never called and
    // `stages` stays empty. ~keep
    #[tokio::test]
    async fn should_emit_start_then_complete_progress_events_for_a_successful_bytes_extraction() {
        let sink = Arc::new(RecordingProgressSink::default());
        let engine = Engine::builder().with_progress_sink(sink.clone()).build();

        let output = engine
            .extract(
                ExtractInput::from_bytes(b"hello progress".to_vec(), "text/plain", None),
                &ExtractionConfig::default(),
            )
            .await
            .unwrap();

        assert_eq!(output.results.len(), 1);
        assert_eq!(
            *sink.stages.lock().expect("recording sink mutex poisoned"),
            vec!["extract_start".to_string(), "extract_complete".to_string()],
            "expected exactly a start event followed by a complete event, in that order"
        );
    }

    #[tokio::test]
    async fn should_emit_start_then_error_progress_events_for_a_failed_extraction() {
        let sink = Arc::new(RecordingProgressSink::default());
        let engine = Engine::builder().with_progress_sink(sink.clone()).build();

        let error = engine
            .extract(
                ExtractInput::from_uri("s3://bucket/file.txt"),
                &ExtractionConfig::default(),
            )
            .await
            .unwrap_err();

        assert!(error.to_string().contains("unsupported URI scheme"));
        assert_eq!(
            *sink.stages.lock().expect("recording sink mutex poisoned"),
            vec!["extract_start".to_string(), "extract_error".to_string()],
            "expected exactly a start event followed by an error event, in that order"
        );
    }

    /// A [`CacheBackend`] that always serves one fixed payload and counts lookups,
    /// so a test can prove a hit was actually consulted (not just that the result
    /// happens to match).
    struct StubCacheBackend {
        cached_payload: Vec<u8>,
        gets: AtomicUsize,
        puts: AtomicUsize,
    }

    #[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
    impl CacheBackend for StubCacheBackend {
        async fn get(&self, _key: &str) -> Option<Vec<u8>> {
            self.gets.fetch_add(1, Ordering::SeqCst);
            Some(self.cached_payload.clone())
        }

        async fn put(&self, _key: &str, _value: Vec<u8>, _ttl: Option<Duration>) {
            self.puts.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Revert line: change `content_cache_key` in `extract_impl.rs` to always
    // `return None;` to make this test fail — with no cache key, `extract` never
    // calls `inner.cache.get`, `gets` stays 0, and the real (uncached) extraction
    // result ("this is not the cached content") is returned instead.
    #[tokio::test]
    async fn should_return_cached_result_and_skip_extraction_on_cache_hit() {
        let cached_result = ExtractionResult::single(ExtractedDocument {
            content: "CACHED-RESULT-NOT-REEXTRACTED".to_string(),
            ..Default::default()
        });
        let cache = Arc::new(StubCacheBackend {
            cached_payload: serde_json::to_vec(&cached_result).unwrap(),
            gets: AtomicUsize::new(0),
            puts: AtomicUsize::new(0),
        });
        let engine = Engine::builder().with_cache_backend(cache.clone()).build();

        let output = engine
            .extract(
                ExtractInput::from_bytes(b"this is not the cached content".to_vec(), "text/plain", None),
                &ExtractionConfig::default(),
            )
            .await
            .unwrap();

        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].content, "CACHED-RESULT-NOT-REEXTRACTED");
        assert_eq!(
            cache.gets.load(Ordering::SeqCst),
            1,
            "the cache backend must be consulted exactly once"
        );
        assert_eq!(
            cache.puts.load(Ordering::SeqCst),
            0,
            "a hit must not also write back to the cache"
        );
    }

    /// A [`CacheBackend`] backed by a real `HashMap`, so hit/miss behavior reflects
    /// actual key derivation (content hash + config) rather than a scripted response.
    #[derive(Default)]
    struct InMemoryCacheBackend {
        store: Mutex<std::collections::HashMap<String, Vec<u8>>>,
        gets: AtomicUsize,
        puts: AtomicUsize,
    }

    #[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
    impl CacheBackend for InMemoryCacheBackend {
        async fn get(&self, key: &str) -> Option<Vec<u8>> {
            self.gets.fetch_add(1, Ordering::SeqCst);
            self.store
                .lock()
                .expect("in-memory cache mutex poisoned")
                .get(key)
                .cloned()
        }

        async fn put(&self, key: &str, value: Vec<u8>, _ttl: Option<Duration>) {
            self.puts.fetch_add(1, Ordering::SeqCst);
            self.store
                .lock()
                .expect("in-memory cache mutex poisoned")
                .insert(key.to_string(), value);
        }
    }

    fn legacy_item_key(input: &ExtractInput, config: &ExtractionConfig) -> String {
        let bytes = input.bytes.as_deref().unwrap();
        let mut hash = blake3::Hasher::new();
        hash.update(b"xberg-engine-extract-v2");
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
        hash.update(input.mime_type.as_deref().unwrap().as_bytes());
        hash.update(b"\0\0");
        hash.update(&serde_json::to_vec(config).unwrap());
        hash.finalize().to_hex().to_string()
    }

    fn legacy_batch_key(item: &str) -> String {
        let mut hash = blake3::Hasher::new();
        hash.update(b"xberg-engine-extract-batch-v2");
        hash.update(&1_u64.to_le_bytes());
        hash.update(&(item.len() as u64).to_le_bytes());
        hash.update(item.as_bytes());
        hash.finalize().to_hex().to_string()
    }

    #[tokio::test]
    async fn single_and_batch_extraction_ignore_pre_version_cache_entries_and_reuse_current_ones() {
        const TEXT: &str = "Current text from the source bytes.";
        const STALE: &str = "Legacy cached output that must be ignored.";

        let config = ExtractionConfig::default();
        assert!(config.use_cache);
        let input = ExtractInput::from_bytes(TEXT.as_bytes().to_vec(), "text/plain", None);

        let legacy_single = legacy_item_key(&input, &config);
        let legacy_batch = legacy_batch_key(&legacy_single);
        let stale = serde_json::to_vec(&ExtractionResult::single(ExtractedDocument {
            content: STALE.to_owned(),
            ..Default::default()
        }))
        .unwrap();
        let cache = Arc::new(InMemoryCacheBackend::default());
        cache.store.lock().unwrap().extend([
            (legacy_single.clone(), stale.clone()),
            (legacy_batch.clone(), stale.clone()),
        ]);
        let engine = Engine::builder().with_cache_backend(cache.clone()).build();

        let single = engine.extract(input.clone(), &config).await.unwrap();
        let batch = engine.extract_batch(vec![input.clone()], &config).await.unwrap();
        assert_eq!(
            single.results[0].content.trim(),
            TEXT,
            "single misses pre-version entry"
        );
        assert_eq!(batch.results[0].content.trim(), TEXT, "batch misses pre-version entry");
        assert_eq!(cache.gets.load(Ordering::SeqCst), 2);
        assert_eq!(cache.puts.load(Ordering::SeqCst), 2);

        let warm_single = engine.extract(input.clone(), &config).await.unwrap();
        let warm_batch = engine.extract_batch(vec![input], &config).await.unwrap();
        assert_eq!(
            serde_json::to_value(warm_single).unwrap(),
            serde_json::to_value(single).unwrap()
        );
        assert_eq!(
            serde_json::to_value(warm_batch).unwrap(),
            serde_json::to_value(batch).unwrap()
        );
        assert_eq!(cache.gets.load(Ordering::SeqCst), 4);
        assert_eq!(cache.puts.load(Ordering::SeqCst), 2);
        let entries = cache.store.lock().unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries.get(&legacy_single), Some(&stale));
        assert_eq!(entries.get(&legacy_batch), Some(&stale));
    }

    // Revert line: remove the `inner.cache.put(...)` call in the `Ok(output)` arm of
    // `extract_impl::extract` to make this test fail -- `puts` stays 0 after the first
    // (miss) extraction, and the second identical extract also misses (still 0 entries
    // in the store), so `puts` never reaches 1 either. ~keep
    #[tokio::test]
    async fn should_populate_cache_on_miss_and_skip_reextraction_on_identical_second_call() {
        let cache = Arc::new(InMemoryCacheBackend::default());
        let engine = Engine::builder().with_cache_backend(cache.clone()).build();
        let config = ExtractionConfig::default();
        let bytes = b"identical bytes for cache".to_vec();

        let first = engine
            .extract(ExtractInput::from_bytes(bytes.clone(), "text/plain", None), &config)
            .await
            .unwrap();

        assert_eq!(
            cache.gets.load(Ordering::SeqCst),
            1,
            "the first extract must consult the cache exactly once (a miss)"
        );
        assert_eq!(
            cache.puts.load(Ordering::SeqCst),
            1,
            "a successful cache-miss extraction must populate the cache exactly once"
        );

        let second = engine
            .extract(ExtractInput::from_bytes(bytes, "text/plain", None), &config)
            .await
            .unwrap();

        assert_eq!(
            cache.gets.load(Ordering::SeqCst),
            2,
            "the second identical extract must also consult the cache"
        );
        assert_eq!(
            cache.puts.load(Ordering::SeqCst),
            1,
            "a cache hit must short-circuit extraction and must not write back to the cache again"
        );
        assert_eq!(
            first.results[0].content, second.results[0].content,
            "the cache-hit result must equal the originally-extracted content"
        );
    }

    // Revert line: change `content_cache_key` in `extract_impl.rs` to drop
    // `config_json` from the hash to make this test fail -- both configs would then
    // derive the same key, the second extract would hit the first extract's cache
    // entry, and `puts` would stay at 1 instead of reaching 2.
    #[tokio::test]
    async fn should_miss_cache_when_extraction_config_changes_for_identical_bytes() {
        let cache = Arc::new(InMemoryCacheBackend::default());
        let engine = Engine::builder().with_cache_backend(cache.clone()).build();
        let bytes = b"same bytes different config".to_vec();

        let config_a = ExtractionConfig::default();
        engine
            .extract(ExtractInput::from_bytes(bytes.clone(), "text/plain", None), &config_a)
            .await
            .unwrap();

        let config_b = ExtractionConfig {
            enable_quality_processing: !config_a.enable_quality_processing,
            ..ExtractionConfig::default()
        };
        engine
            .extract(ExtractInput::from_bytes(bytes, "text/plain", None), &config_b)
            .await
            .unwrap();

        assert_eq!(
            cache.gets.load(Ordering::SeqCst),
            2,
            "both extracts must consult the cache once each"
        );
        assert_eq!(
            cache.puts.load(Ordering::SeqCst),
            2,
            "a config change must derive a different cache key, forcing a second miss and a second write"
        );
    }

    // Revert line: move the `PROGRESS_STAGE_CACHE_HIT` emit in `extract_impl.rs` so it
    // no longer runs before the early `return Ok(cached_result)` to make this test fail
    // -- `stages` would then read only `["extract_start"]` instead of including the
    // cache-hit stage. ~keep
    #[tokio::test]
    async fn should_emit_start_then_cache_hit_progress_events_on_a_cache_hit() {
        let sink = Arc::new(RecordingProgressSink::default());
        let cached_result = ExtractionResult::single(ExtractedDocument {
            content: "CACHED".to_string(),
            ..Default::default()
        });
        let cache = Arc::new(StubCacheBackend {
            cached_payload: serde_json::to_vec(&cached_result).unwrap(),
            gets: AtomicUsize::new(0),
            puts: AtomicUsize::new(0),
        });
        let engine = Engine::builder()
            .with_progress_sink(sink.clone())
            .with_cache_backend(cache)
            .build();

        engine
            .extract(
                ExtractInput::from_bytes(b"progress on cache hit".to_vec(), "text/plain", None),
                &ExtractionConfig::default(),
            )
            .await
            .unwrap();

        assert_eq!(
            *sink.stages.lock().expect("recording sink mutex poisoned"),
            vec!["extract_start".to_string(), "extract_cache_hit".to_string()],
            "a cache hit must emit start then cache_hit, not complete"
        );
    }

    #[tokio::test]
    async fn should_cache_an_identical_bytes_batch_and_emit_batch_progress() {
        let sink = Arc::new(RecordingProgressSink::default());
        let cache = Arc::new(InMemoryCacheBackend::default());
        let engine = Engine::builder()
            .with_progress_sink(sink.clone())
            .with_cache_backend(cache.clone())
            .build();
        let config = ExtractionConfig::default();
        let inputs = vec![
            ExtractInput::from_bytes(b"first batch item".to_vec(), "text/plain", None),
            ExtractInput::from_bytes(b"second batch item".to_vec(), "text/plain", None),
        ];

        let first = engine.extract_batch(inputs.clone(), &config).await.unwrap();
        let second = engine.extract_batch(inputs, &config).await.unwrap();

        assert_eq!(first.results.len(), 2);
        assert_eq!(second.results.len(), 2);
        assert_eq!(cache.gets.load(Ordering::SeqCst), 2);
        assert_eq!(cache.puts.load(Ordering::SeqCst), 1);
        assert_eq!(
            *sink.stages.lock().expect("recording sink mutex poisoned"),
            vec![
                "extract_batch_start".to_string(),
                "extract_batch_complete".to_string(),
                "extract_batch_start".to_string(),
                "extract_batch_cache_hit".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn should_not_consult_injected_cache_when_config_disables_caching() {
        let cache = Arc::new(InMemoryCacheBackend::default());
        let engine = Engine::builder().with_cache_backend(cache.clone()).build();
        let config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };

        let single = engine
            .extract(
                ExtractInput::from_bytes(b"uncached single input".to_vec(), "text/plain", None),
                &config,
            )
            .await
            .unwrap();
        let batch = engine
            .extract_batch(
                vec![
                    ExtractInput::from_bytes(b"uncached batch item one".to_vec(), "text/plain", None),
                    ExtractInput::from_bytes(b"uncached batch item two".to_vec(), "text/plain", None),
                ],
                &config,
            )
            .await
            .unwrap();

        assert_eq!(single.results.len(), 1);
        assert_eq!(batch.results.len(), 2);
        assert_eq!(
            cache.gets.load(Ordering::SeqCst),
            0,
            "cache-disabled single and batch extraction must not consult an injected backend"
        );
        assert_eq!(
            cache.puts.load(Ordering::SeqCst),
            0,
            "cache-disabled single and batch extraction must not write to an injected backend"
        );
    }

    #[tokio::test]
    async fn should_not_cache_a_batch_with_per_input_errors() {
        let cache = Arc::new(InMemoryCacheBackend::default());
        let engine = Engine::builder().with_cache_backend(cache.clone()).build();
        let inputs = vec![
            ExtractInput::from_bytes(b"valid batch item".to_vec(), "text/plain", None),
            ExtractInput::from_bytes(b"invalid batch item".to_vec(), "", None),
        ];

        let output = engine
            .extract_batch(inputs, &ExtractionConfig::default())
            .await
            .unwrap();

        assert_eq!(output.results.len(), 1);
        assert_eq!(output.errors.len(), 1);
        assert_eq!(cache.gets.load(Ordering::SeqCst), 1);
        assert_eq!(cache.puts.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_validate_config_before_single_and_batch_extraction() {
        let sink = Arc::new(RecordingProgressSink::default());
        let cache = Arc::new(InMemoryCacheBackend::default());
        let engine = Engine::builder()
            .with_progress_sink(sink.clone())
            .with_cache_backend(cache.clone())
            .build();
        let config = ExtractionConfig {
            csv: Some(crate::core::config::CsvConfig {
                delimiter: Some(String::new()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let input = ExtractInput::from_bytes(b"invalid config".to_vec(), "text/plain", None);

        let single_error = engine.extract(input.clone(), &config).await.unwrap_err();
        let batch_error = engine.extract_batch(vec![input], &config).await.unwrap_err();

        assert!(matches!(single_error, crate::XbergError::Validation { .. }));
        assert!(matches!(batch_error, crate::XbergError::Validation { .. }));
        assert_eq!(cache.gets.load(Ordering::SeqCst), 0);
        assert_eq!(cache.puts.load(Ordering::SeqCst), 0);
        assert_eq!(
            *sink.stages.lock().expect("recording sink mutex poisoned"),
            vec![
                "extract_start".to_string(),
                "extract_error".to_string(),
                "extract_batch_start".to_string(),
                "extract_batch_error".to_string(),
            ]
        );
    }
}
