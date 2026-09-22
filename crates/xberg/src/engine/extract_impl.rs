//! Engine-internal extraction implementation.
//!
//! This module holds the extraction internals moved verbatim from
//! `core/extract/mod.rs`. The public free functions `crate::extract` /
//! `crate::extract_batch` delegate here via a process-global default
//! [`crate::engine::Engine`].

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
#[cfg(feature = "otel")]
use tracing::Instrument;

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
use std::future::Future;
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
use std::sync::Arc;
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
use std::time::Instant;

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
type PendingBatchItem = (usize, ExtractInput, String);

#[cfg(feature = "url-ingestion")]
use crawlberg::{CrawlConfig, CrawlEngine, CrawlPageResult, DownloadedDocument, ScrapeResult};

#[cfg(feature = "url-ingestion")]
use crate::core::config::UrlExtractionMode;
use crate::core::config::{
    ExtractInput, ExtractInputKind, ExtractionConfig, ExtractionErrorItem, ExtractionResult, ExtractionSummary,
};
#[cfg(feature = "url-ingestion")]
use crate::types::ExtractedUri;
use crate::types::{ExtractedDocument, UriKind};
use crate::{Result, XbergError};

use crate::core::extractor::{extract_bytes, extract_file};
use crate::engine::seams::ProgressEvent;

const HTTP_SCHEME: &str = "http://";
const HTTPS_SCHEME: &str = "https://";
const FILE_SCHEME: &str = "file://";

/// Stable progress-sink stage labels for the single-input [`extract`] entry point.
///
/// Chosen at coarse, stable lifecycle points only (session start, cache hit,
/// completion, error) — never per-page or per-chunk — per the [`ProgressSink`]
/// "coarse progress" contract (`crate::engine::seams::ProgressSink`).
const PROGRESS_STAGE_START: &str = "extract_start";
const PROGRESS_STAGE_CACHE_HIT: &str = "extract_cache_hit";
const PROGRESS_STAGE_COMPLETE: &str = "extract_complete";
const PROGRESS_STAGE_ERROR: &str = "extract_error";
const BATCH_PROGRESS_STAGE_START: &str = "extract_batch_start";
const BATCH_PROGRESS_STAGE_CACHE_HIT: &str = "extract_batch_cache_hit";
const BATCH_PROGRESS_STAGE_COMPLETE: &str = "extract_batch_complete";
const BATCH_PROGRESS_STAGE_ERROR: &str = "extract_batch_error";

/// Namespace prefix mixed into the content-hash cache key so a future,
/// incompatible key derivation can never collide with entries this version wrote.
const CACHE_KEY_NAMESPACE: &[u8] = b"xberg-engine-extract-v2";
const BATCH_CACHE_KEY_NAMESPACE: &[u8] = b"xberg-engine-extract-batch-v2";

/// Extract content from a single bytes or URI input.
///
/// Honours the injected [`CacheBackend`](crate::engine::seams::CacheBackend) and
/// [`ProgressSink`](crate::engine::seams::ProgressSink) seams: a content-hash cache
/// hit (bytes inputs only, keyed on the raw bytes plus the resolved
/// [`ExtractionConfig`]) returns the cached [`ExtractionResult`] and skips
/// extraction entirely; a miss runs extraction as before and, on success,
/// populates the cache for next time. Both seams default to no-ops
/// ([`NoopCache`](crate::engine::seams::NoopCache),
/// [`NoopProgressSink`](crate::engine::seams::NoopProgressSink)), so callers who
/// inject nothing see byte-identical behavior.
pub(crate) async fn extract(
    inner: &super::EngineInner,
    input: ExtractInput,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    inner.progress.emit(ProgressEvent {
        stage: PROGRESS_STAGE_START.to_string(),
        message: None,
        fraction: Some(0.0),
    });

    if let Err(error) = config.validate().and_then(|()| ensure_not_cancelled(config)) {
        inner.progress.emit(ProgressEvent {
            stage: PROGRESS_STAGE_ERROR.to_string(),
            message: Some(error.to_string()),
            fraction: None,
        });
        return Err(error);
    }

    let cache_key = content_cache_key(&input, config);
    if let Some(key) = cache_key.as_deref()
        && let Some(cached_bytes) = inner.cache.get(key).await
        && let Ok(cached_result) = serde_json::from_slice::<ExtractionResult>(&cached_bytes)
    {
        inner.progress.emit(ProgressEvent {
            stage: PROGRESS_STAGE_CACHE_HIT.to_string(),
            message: None,
            fraction: Some(1.0),
        });
        return Ok(cached_result);
    }

    // Type-erased so `extract`'s future does not nest `extract_uncached`'s whole
    // (already very deep) future type inside its own. Without this the compiler
    // exceeds the recursion limit proving `Send` for the combined future, and any
    // caller doing `tokio::spawn(xberg::extract(..))` — the canonical usage — fails
    // to compile with E0275. ~keep
    //
    // No `+ Send` on wasm32: extractor futures are `!Send` there (`async_trait(?Send)`,
    // see `plugins/extractor/trait.rs`), and the JS-backed futures pulled in through
    // crawlberg/reqwest's wasm backend (`JsFuture`, wasm-bindgen closures) are never
    // `Send` either. wasm32 has no OS threads and no `tokio::spawn`, so nothing needs
    // the bound there — mirrors the `extract_batch` split below. ~keep
    #[cfg(not(target_arch = "wasm32"))]
    let uncached: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>> + Send>> =
        Box::pin(extract_uncached(input, config));
    #[cfg(target_arch = "wasm32")]
    let uncached: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>>>> =
        Box::pin(extract_uncached(input, config));
    let result = uncached.await;

    match &result {
        Ok(output) => {
            if output.errors.is_empty()
                && let Some(key) = cache_key
                && let Ok(serialized) = serde_json::to_vec(output)
            {
                inner.cache.put(&key, serialized, None).await;
            }
            inner.progress.emit(ProgressEvent {
                stage: PROGRESS_STAGE_COMPLETE.to_string(),
                message: None,
                fraction: Some(1.0),
            });
        }
        Err(error) => {
            inner.progress.emit(ProgressEvent {
                stage: PROGRESS_STAGE_ERROR.to_string(),
                message: Some(error.to_string()),
                fraction: None,
            });
        }
    }

    result
}

fn ensure_not_cancelled(config: &ExtractionConfig) -> Result<()> {
    if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return Err(XbergError::Cancelled);
    }
    Ok(())
}

/// The extraction path proper, unwrapped from cache/progress bookkeeping so
/// [`extract`] can wrap it uniformly for both the cache-hit and cache-miss cases.
async fn extract_uncached(input: ExtractInput, config: &ExtractionConfig) -> Result<ExtractionResult> {
    let mut seen = initial_seen_urls(std::slice::from_ref(&input));
    let seed_hosts = initial_seed_hosts(std::slice::from_ref(&input));
    let mut output = Box::pin(extract_one(input, config, 0)).await?;

    follow_recursive_document_urls(&mut output, config, &mut seen, &seed_hosts).await?;

    Ok(output)
}

/// Derive a content-hash cache key for `input`, or `None` when the input is not
/// eligible for caching.
///
/// Only `bytes` inputs are cached, per the cache-key contract (content-hash of
/// file bytes + config, never path-based): a `uri` input's content is not yet
/// known at this point without fetching it, which would defeat the purpose of a
/// pre-extraction cache check. The key mixes the byte length and content, the
/// MIME/filename hints, and the fully-resolved [`ExtractionConfig`] (base config
/// merged with any per-input override), so a config or override change is a
/// guaranteed cache miss.
fn content_cache_key(input: &ExtractInput, base_config: &ExtractionConfig) -> Option<String> {
    if input.kind != ExtractInputKind::Bytes {
        return None;
    }
    let bytes = input.bytes.as_deref()?;
    let resolved_config = resolve_input_config(input, base_config);
    let config_json = serde_json::to_vec(&resolved_config).ok()?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(CACHE_KEY_NAMESPACE);
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    hasher.update(input.mime_type.as_deref().unwrap_or_default().as_bytes());
    hasher.update(b"\0");
    hasher.update(input.filename.as_deref().unwrap_or_default().as_bytes());
    hasher.update(b"\0");
    hasher.update(&config_json);
    Some(hasher.finalize().to_hex().to_string())
}

fn batch_content_cache_key(inputs: &[ExtractInput], base_config: &ExtractionConfig) -> Option<String> {
    if inputs.is_empty() {
        return None;
    }

    let mut hasher = blake3::Hasher::new();
    hasher.update(BATCH_CACHE_KEY_NAMESPACE);
    hasher.update(&(inputs.len() as u64).to_le_bytes());
    for input in inputs {
        let item_key = content_cache_key(input, base_config)?;
        hasher.update(&(item_key.len() as u64).to_le_bytes());
        hasher.update(item_key.as_bytes());
    }
    Some(hasher.finalize().to_hex().to_string())
}

/// Extract content from multiple bytes or URI inputs.
pub(crate) async fn extract_batch(
    inner: &super::EngineInner,
    inputs: Vec<ExtractInput>,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    inner.progress.emit(ProgressEvent {
        stage: BATCH_PROGRESS_STAGE_START.to_string(),
        message: None,
        fraction: Some(0.0),
    });

    if let Err(error) = config.validate().and_then(|()| ensure_not_cancelled(config)) {
        inner.progress.emit(ProgressEvent {
            stage: BATCH_PROGRESS_STAGE_ERROR.to_string(),
            message: Some(error.to_string()),
            fraction: None,
        });
        return Err(error);
    }

    let cache_key = batch_content_cache_key(&inputs, config);
    if let Some(key) = cache_key.as_deref()
        && let Some(cached_bytes) = inner.cache.get(key).await
        && let Ok(cached_result) = serde_json::from_slice::<ExtractionResult>(&cached_bytes)
    {
        inner.progress.emit(ProgressEvent {
            stage: BATCH_PROGRESS_STAGE_CACHE_HIT.to_string(),
            message: None,
            fraction: Some(1.0),
        });
        return Ok(cached_result);
    }

    let result = extract_batch_uncached(inner, inputs, config).await;
    match &result {
        Ok(output) => {
            if output.errors.is_empty()
                && let Some(key) = cache_key
                && let Ok(serialized) = serde_json::to_vec(output)
            {
                inner.cache.put(&key, serialized, None).await;
            }
            inner.progress.emit(ProgressEvent {
                stage: BATCH_PROGRESS_STAGE_COMPLETE.to_string(),
                message: None,
                fraction: Some(1.0),
            });
        }
        Err(error) => {
            inner.progress.emit(ProgressEvent {
                stage: BATCH_PROGRESS_STAGE_ERROR.to_string(),
                message: Some(error.to_string()),
                fraction: None,
            });
        }
    }
    result
}

async fn extract_batch_uncached(
    inner: &super::EngineInner,
    inputs: Vec<ExtractInput>,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    #[cfg(feature = "otel")]
    let batch_span = crate::telemetry::spans::batch_span(inputs.len());
    // `Instant::now()` panics on wasm32 (no usable timer there, see the wasm32 note on
    // `extract_file_uncached`'s timeout handling), so the batch counter/histogram pair is
    // skipped on that target rather than risking a panic for a metrics-only side effect. ~keep
    #[cfg(all(feature = "otel", not(target_arch = "wasm32")))]
    let batch_started = std::time::Instant::now();

    // `extract_batch_concurrent` spawns tasks on `tokio::task::JoinSet`, which requires `Send`
    // futures; extractor futures are `!Send` on wasm32 (async_trait(?Send), see
    // plugins/extractor/trait.rs) and wasm32 has no OS threads to run them on regardless. Use
    // the sequential path there even though `tokio-runtime` is active (it's pulled in by
    // `chunking-tokenizers`/`static-embeddings`, not concurrency support). ~keep
    // Type-erased for the same reason as the single-extract path above: leaving the
    // inner future's concrete type visible here makes callers that `tokio::spawn` a
    // batch exceed the recursion limit proving `Send`. That bit the generated
    // xberg-node binding (`run_bounded_batch_tasks`), which cannot carry a
    // `#![recursion_limit]` of its own because it is regenerated from scratch. ~keep
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let batch: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>> + Send>> =
        Box::pin(extract_batch_concurrent(inner, inputs, config));

    // No `+ Send` on wasm32: extractor futures are `!Send` there (async_trait(?Send)). ~keep
    #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
    let batch: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>>>> = {
        let _ = inner;
        Box::pin(extract_batch_sequential(inputs, config))
    };

    #[cfg(feature = "otel")]
    let result = batch.instrument(batch_span).await;
    #[cfg(not(feature = "otel"))]
    let result = batch.await;

    #[cfg(all(feature = "otel", not(target_arch = "wasm32")))]
    record_batch_metrics(batch_started.elapsed(), &result);

    result
}

/// Emit the batch-level counter and duration histogram (#332).
///
/// Unlike the per-extraction metrics recorded in `plugins::extractor::instrumented`, there is
/// exactly one batch span per call, so this carries no per-item attributes — only the
/// overall `status` of the batch as a whole (an individual item's failure is captured in
/// `ExtractionResult::errors`, not here).
#[cfg(all(feature = "otel", not(target_arch = "wasm32")))]
fn record_batch_metrics(elapsed: std::time::Duration, result: &Result<ExtractionResult>) {
    let metrics = crate::telemetry::metrics::get_metrics();
    let status = if result.is_ok() { "ok" } else { "error" };
    let attrs = [opentelemetry::KeyValue::new("status", status)];

    metrics.batch_total.add(1, &attrs);
    metrics.batch_duration_ms.record(elapsed.as_secs_f64() * 1000.0, &[]);
}

#[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
async fn extract_batch_sequential(inputs: Vec<ExtractInput>, config: &ExtractionConfig) -> Result<ExtractionResult> {
    let mut seen = initial_seen_urls(&inputs);
    let seed_hosts = initial_seed_hosts(&inputs);
    let mut output = ExtractionResult {
        summary: ExtractionSummary {
            inputs: inputs.len(),
            ..Default::default()
        },
        ..Default::default()
    };

    for (index, input) in inputs.into_iter().enumerate() {
        let source = input_source(&input);
        let item = Box::pin(extract_one(input, config, index));

        #[cfg(feature = "otel")]
        let item_result = item.instrument(crate::telemetry::spans::batch_item_span(index)).await;
        #[cfg(not(feature = "otel"))]
        let item_result = item.await;

        match item_result {
            Ok(item_output) => append_extraction_output(&mut output, item_output),
            Err(error) => output.errors.push(error_item(index, source, &error)),
        }
    }

    output.refresh_counts();
    follow_recursive_document_urls(&mut output, config, &mut seen, &seed_hosts).await?;
    Ok(output)
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
async fn extract_batch_concurrent(
    inner: &super::EngineInner,
    inputs: Vec<ExtractInput>,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    let input_count = inputs.len();
    let mut seen = initial_seen_urls(&inputs);
    let seed_hosts = initial_seed_hosts(&inputs);
    let mut output = ExtractionResult {
        summary: ExtractionSummary {
            inputs: input_count,
            ..Default::default()
        },
        ..Default::default()
    };

    if input_count == 0 {
        return Ok(output);
    }

    // Batch workers each receive a divided per-document budget, but Rayon is global and
    // immutable after first initialization, so the pools take the whole batch budget here
    // rather than the smaller share the first worker to start would otherwise install.
    crate::core::config::concurrency::init_thread_pools(config.concurrency.as_ref());
    let base_config = Arc::new(config.clone());
    let mut pending: VecDeque<PendingBatchItem> = VecDeque::with_capacity(input_count);

    let mut items: Vec<Option<BatchItemResult>> = Vec::with_capacity(input_count);
    items.resize_with(input_count, || None);

    #[cfg(feature = "url-ingestion")]
    let mut shared_items: Vec<SharedUrlItem> = Vec::new();
    #[cfg(feature = "url-ingestion")]
    let base_crawl_fingerprint = super::crawl_handle::crawl_fingerprint(&base_config.url.crawl);

    for (index, input) in inputs.into_iter().enumerate() {
        let source = input_source(&input);

        #[cfg(feature = "url-ingestion")]
        {
            if let Some(uri) = shared_group_uri(&input) {
                let resolved_config = resolve_input_config(&input, &base_config);
                if resolved_config.url.mode == base_config.url.mode
                    && super::crawl_handle::crawl_fingerprint(&resolved_config.url.crawl) == base_crawl_fingerprint
                {
                    shared_items.push(SharedUrlItem {
                        index,
                        source,
                        uri,
                        config: resolved_config,
                    });
                    continue;
                }
            }
        }

        pending.push_back((index, input, source));
    }

    let execution_plan = resolve_pending_batch_execution_plan(config, &pending);
    let pending = if should_prioritize_pending_batch_items(pending.len(), execution_plan.workers) {
        prioritize_pending_batch_items(pending, config).await
    } else {
        pending
    };
    let task_config = resolve_batch_base_config(&base_config, execution_plan.thread_budget);
    let completed = run_bounded_batch_tasks(pending, execution_plan.workers, move |(index, input, source)| {
        let base_config = Arc::clone(&task_config);
        async move {
            let resolved_config = resolve_batch_input_config(&input, &base_config, execution_plan.thread_budget);
            let timeout_secs = resolved_config.extraction_timeout_secs;
            let cancel_token = resolved_config.cancel_token.clone();
            let item = run_batch_item(index, source, timeout_secs, cancel_token, || async move {
                // Type-erased, not merely boxed. `run_bounded_batch_tasks` requires the
                // enclosing async block to be `Send`, and proving that walks the whole
                // nested future type -- which reaches h2/hyper/slab through reqwest and
                // overflows rustc's 128 auto-trait recursion limit (E0275) in the
                // alef-generated binding crates, which cannot carry `#![recursion_limit]`.
                // A bare `Box::pin` does not help: it yields `Pin<Box<ConcreteFuture>>`,
                // so the concrete type stays in the proof. Coercing to `dyn Future` cuts
                // the chain here. ~keep
                let extraction: std::pin::Pin<Box<dyn Future<Output = Result<ExtractionResult>> + Send + '_>> =
                    Box::pin(extract_one_resolved(input, &resolved_config, index));
                extraction.await
            });

            #[cfg(feature = "otel")]
            {
                item.instrument(crate::telemetry::spans::batch_item_span(index)).await
            }
            #[cfg(not(feature = "otel"))]
            {
                item.await
            }
        }
    })
    .await?;

    for item in completed {
        let index = item.index;
        if index < items.len() {
            items[index] = Some(item);
        } else {
            return Err(XbergError::Other(format!("batch task returned invalid index: {index}")));
        }
    }

    #[cfg(feature = "url-ingestion")]
    if !shared_items.is_empty() {
        run_shared_url_group(inner, &base_config, shared_items, &mut items).await;
    }
    #[cfg(not(feature = "url-ingestion"))]
    let _ = inner;

    for item in items.into_iter().flatten() {
        match item.result {
            Ok(item_output) => append_extraction_output(&mut output, item_output),
            Err(error) => output.errors.push(error_item(item.index, item.source, &error)),
        }
    }

    output.refresh_counts();
    follow_recursive_document_urls(&mut output, config, &mut seen, &seed_hosts).await?;
    Ok(output)
}

#[cfg(all(test, feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn resolve_engine_batch_execution_plan(
    config: &ExtractionConfig,
    inputs: &[ExtractInput],
) -> crate::core::config::concurrency::BatchExecutionPlan {
    resolve_engine_batch_execution_plan_for(config, classify_layout_batch(config, inputs), inputs.len())
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn resolve_pending_batch_execution_plan(
    config: &ExtractionConfig,
    pending: &VecDeque<PendingBatchItem>,
) -> crate::core::config::concurrency::BatchExecutionPlan {
    resolve_engine_batch_execution_plan_for(
        config,
        classify_layout_batch(config, pending.iter().map(|(_, input, _)| input)),
        pending.len(),
    )
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn should_prioritize_pending_batch_items(pending_count: usize, workers: usize) -> bool {
    workers > 1 && pending_count > workers
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
async fn prioritize_pending_batch_items(
    pending: VecDeque<PendingBatchItem>,
    base_config: &ExtractionConfig,
) -> VecDeque<PendingBatchItem> {
    const MAX_CONCURRENT_SIZE_HINTS: usize = 16;
    const SIZE_HINT_BUDGET: std::time::Duration = std::time::Duration::from_millis(25);

    let mut costs = vec![None; pending.len()];
    let mut local_paths = Vec::new();

    for (position, (_, input, _)) in pending.iter().enumerate() {
        match input.kind {
            ExtractInputKind::Bytes => {
                costs[position] = input.bytes.as_ref().map(|bytes| bytes.len() as u64);
            }
            ExtractInputKind::Uri => {
                let effective_config = resolve_input_config(input, base_config);
                let cancelled = effective_config
                    .cancel_token
                    .as_ref()
                    .is_some_and(crate::cancellation::CancellationToken::is_cancelled);
                if !cancelled && let Some(path) = local_batch_path(input, &effective_config) {
                    local_paths.push((position, path));
                }
            }
        }
    }

    let collect_costs = async {
        let mut pending_paths: VecDeque<_> = local_paths.into();
        let mut probes = tokio::task::JoinSet::new();
        while probes.len() < MAX_CONCURRENT_SIZE_HINTS {
            let Some((position, path)) = pending_paths.pop_front() else {
                break;
            };
            probes.spawn(metadata_size_hint(position, path));
        }

        while let Some(result) = probes.join_next().await {
            match result {
                Ok((position, Some(cost))) => costs[position] = Some(cost),
                Ok((position, None)) => {
                    tracing::debug!(position, "batch input size hint unavailable; preserving FIFO slot");
                }
                Err(error) => {
                    tracing::debug!(%error, "batch input size hint task failed; preserving FIFO slot");
                }
            }

            if let Some((position, path)) = pending_paths.pop_front() {
                probes.spawn(metadata_size_hint(position, path));
            }
        }
    };
    if tokio::time::timeout(SIZE_HINT_BUDGET, collect_costs).await.is_err() {
        tracing::debug!("batch input size hint budget exhausted; preserving unfinished FIFO slots");
    }

    reorder_pending_batch_items(pending, &costs)
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
async fn metadata_size_hint(position: usize, path: PathBuf) -> (usize, Option<u64>) {
    let cost = tokio::fs::metadata(path).await.ok().map(|metadata| metadata.len());
    (position, cost)
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn local_batch_path(input: &ExtractInput, config: &ExtractionConfig) -> Option<PathBuf> {
    let uri = input.uri.as_deref()?;
    if uri.starts_with(HTTP_SCHEME) || uri.starts_with(HTTPS_SCHEME) {
        return None;
    }
    if uri.starts_with(FILE_SCHEME) {
        return config.url.allow_file_uris.then(|| file_uri_to_path(uri).ok()).flatten();
    }
    if uri.contains("://") {
        return None;
    }
    config.url.allow_local_file_inputs.then(|| PathBuf::from(uri))
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn reorder_pending_batch_items(
    pending: VecDeque<PendingBatchItem>,
    costs: &[Option<u64>],
) -> VecDeque<PendingBatchItem> {
    let mut prioritized = Vec::new();
    let mut prioritized_positions = Vec::new();
    let mut scheduled = Vec::with_capacity(pending.len());

    for (position, item) in pending.into_iter().enumerate() {
        if let Some(cost) = costs.get(position).copied().flatten() {
            prioritized_positions.push(position);
            prioritized.push((cost, position, item));
            scheduled.push(None);
        } else {
            scheduled.push(Some(item));
        }
    }

    prioritized.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    for (position, (_, _, item)) in prioritized_positions.into_iter().zip(prioritized) {
        scheduled[position] = Some(item);
    }

    scheduled.into_iter().flatten().collect()
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn classify_layout_batch<'a>(
    config: &ExtractionConfig,
    inputs: impl IntoIterator<Item = &'a ExtractInput>,
) -> crate::core::config::concurrency::LayoutBatchWorkload {
    #[cfg(layout_detection)]
    {
        use crate::core::config::concurrency::LayoutBatchWorkload;

        let mut any_layout_work = false;
        let mut all_markdown_pdf = true;
        let mut input_count = 0;
        for input in inputs {
            input_count += 1;
            let effective = resolve_input_config(input, config);
            let pdf_evidence = pdf_batch_evidence(input);
            let markdown_pdf = effective.layout.is_some()
                && effective.use_layout_for_markdown
                && pdf_evidence == PdfBatchEvidence::Likely;
            all_markdown_pdf &= markdown_pdf;

            let ocr_layout_may_run = effective.layout.is_some() && !effective.disable_ocr;
            let markdown_layout_may_run = effective.layout.is_some()
                && effective.use_layout_for_markdown
                && pdf_evidence != PdfBatchEvidence::Unlikely;
            any_layout_work |= ocr_layout_may_run || markdown_layout_may_run;
        }

        if input_count > 0 && all_markdown_pdf {
            LayoutBatchWorkload::All
        } else if any_layout_work {
            LayoutBatchWorkload::Mixed
        } else {
            LayoutBatchWorkload::None
        }
    }
    #[cfg(not(layout_detection))]
    {
        let _ = (config, inputs);
        crate::core::config::concurrency::LayoutBatchWorkload::None
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32"), layout_detection))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum PdfBatchEvidence {
    Likely,
    Unlikely,
    Unknown,
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32"), layout_detection))]
fn pdf_batch_evidence(input: &ExtractInput) -> PdfBatchEvidence {
    const PDF_HEADER_MAGIC: &[u8] = b"%PDF-";

    if input.kind == ExtractInputKind::Bytes
        && input
            .bytes
            .as_deref()
            .is_some_and(|bytes| bytes.starts_with(PDF_HEADER_MAGIC))
    {
        return PdfBatchEvidence::Likely;
    }
    if input.mime_type.as_deref().is_some_and(is_pdf_mime) {
        return PdfBatchEvidence::Likely;
    }
    if input.mime_type.is_some() {
        return PdfBatchEvidence::Unlikely;
    }

    match input.kind {
        ExtractInputKind::Bytes => match input.bytes.as_deref() {
            Some(_) => PdfBatchEvidence::Unlikely,
            None => PdfBatchEvidence::Unknown,
        },
        ExtractInputKind::Uri => input
            .uri
            .as_deref()
            .map(uri_pdf_evidence)
            .unwrap_or(PdfBatchEvidence::Unknown),
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32"), layout_detection))]
fn is_pdf_mime(mime_type: &str) -> bool {
    mime_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/pdf"))
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32"), layout_detection))]
fn uri_pdf_evidence(uri: &str) -> PdfBatchEvidence {
    let path = uri.split(['?', '#']).next().unwrap_or(uri);
    match std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some(extension) if extension.eq_ignore_ascii_case("pdf") => PdfBatchEvidence::Likely,
        Some(_) => PdfBatchEvidence::Unlikely,
        None => PdfBatchEvidence::Unknown,
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn resolve_engine_batch_execution_plan_for(
    config: &ExtractionConfig,
    layout_workload: crate::core::config::concurrency::LayoutBatchWorkload,
    input_count: usize,
) -> crate::core::config::concurrency::BatchExecutionPlan {
    crate::core::config::concurrency::resolve_batch_execution_plan(
        config.concurrency.as_ref(),
        layout_workload,
        input_count,
        config.max_concurrent_extractions,
    )
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
async fn run_bounded_batch_tasks<T, F, Fut>(
    mut pending: VecDeque<T>,
    max_concurrent: usize,
    mut task: F,
) -> Result<Vec<BatchItemResult>>
where
    T: Send + 'static,
    F: FnMut(T) -> Fut,
    Fut: Future<Output = BatchItemResult> + Send + 'static,
{
    use tokio::task::JoinSet;

    let mut tasks = JoinSet::new();
    let max_concurrent = max_concurrent.max(1);
    while tasks.len() < max_concurrent {
        let Some(item) = pending.pop_front() else {
            break;
        };
        tasks.spawn(task(item));
    }

    let mut completed = Vec::with_capacity(tasks.len() + pending.len());
    while let Some(task_result) = tasks.join_next().await {
        let item = task_result.map_err(|error| XbergError::Other(format!("batch task failed to join: {error}")))?;
        completed.push(item);
        if let Some(next) = pending.pop_front() {
            tasks.spawn(task(next));
        }
    }
    Ok(completed)
}

/// Owned http(s) URI of an input eligible for the shared-URL batch group.
///
/// Returns `None` for non-URI inputs and for URIs that are not http(s) (bytes,
/// file paths, `file://`, and other schemes stay on the per-item path).
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion", not(target_arch = "wasm32")))]
fn shared_group_uri(input: &ExtractInput) -> Option<String> {
    if !matches!(input.kind, ExtractInputKind::Uri) {
        return None;
    }
    let uri = input.uri.as_deref()?;
    if uri.starts_with(HTTP_SCHEME) || uri.starts_with(HTTPS_SCHEME) {
        Some(uri.to_string())
    } else {
        None
    }
}

/// One http(s) URL routed through the shared crawl engine, carrying everything
/// needed to map the (completion-order) batch result back to its input slot.
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion", not(target_arch = "wasm32")))]
struct SharedUrlItem {
    index: usize,
    source: String,
    uri: String,
    config: ExtractionConfig,
}

/// Run the shared-URL group through ONE crawlberg engine and write each result
/// back into its input slot.
///
/// Timeout semantics in batch mode: the actual network fetch happens inside
/// crawlberg's `batch_scrape` / `batch_crawl`, which manages concurrency and
/// per-request timeouts internally via the shared [`crawlberg::CrawlConfig`]
/// (e.g. `request_timeout_ms`, `rate_limit_ms`). The per-item
/// `extraction_timeout_secs` therefore governs only the *conversion* stage
/// (the [`extract_bytes`] pipeline run by `output_from_scrape` /
/// `output_from_crawl`), which is what [`finalize_shared_item`] wraps. This is
/// the precise nuance that differs from the per-item path, where the same
/// timeout also bounds the fetch.
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion", not(target_arch = "wasm32")))]
async fn run_shared_url_group(
    inner: &super::EngineInner,
    base_config: &ExtractionConfig,
    shared_items: Vec<SharedUrlItem>,
    items: &mut [Option<BatchItemResult>],
) {
    use std::collections::HashMap;

    let engine = match inner.crawl_engine_for(&base_config.url.crawl) {
        Ok(engine) => engine,
        Err(error) => {
            for shared in &shared_items {
                items[shared.index] = Some(BatchItemResult {
                    index: shared.index,
                    source: shared.source.clone(),
                    result: Err(duplicate_construction_error(&error)),
                });
            }
            return;
        }
    };

    let mut positions_for_url: HashMap<&str, VecDeque<usize>> = HashMap::new();
    for (position, shared) in shared_items.iter().enumerate() {
        positions_for_url
            .entry(shared.uri.as_str())
            .or_default()
            .push_back(position);
    }
    let urls: Vec<&str> = shared_items.iter().map(|shared| shared.uri.as_str()).collect();

    let mut unmatched_errors: VecDeque<XbergError> = VecDeque::new();
    let batch_started = Instant::now();

    match base_config.url.mode {
        UrlExtractionMode::Auto | UrlExtractionMode::Document => {
            for (url, result) in engine.batch_scrape(&urls).await {
                let Some(position) = positions_for_url.get_mut(url.as_str()).and_then(VecDeque::pop_front) else {
                    if let Err(error) = result {
                        unmatched_errors.push_back(map_crawl_error(error));
                    }
                    continue;
                };
                let shared = &shared_items[position];
                let conversion = async {
                    match result {
                        Ok(scrape) => output_from_scrape(scrape, &shared.config, shared.index).await,
                        Err(error) => Err(map_crawl_error(error)),
                    }
                };
                items[shared.index] = Some(finalize_shared_item(shared, batch_started, conversion).await);
            }
        }
        UrlExtractionMode::Crawl => {
            for (url, result) in engine.batch_crawl(&urls).await {
                let Some(position) = positions_for_url.get_mut(url.as_str()).and_then(VecDeque::pop_front) else {
                    if let Err(error) = result {
                        unmatched_errors.push_back(map_crawl_error(error));
                    }
                    continue;
                };
                let shared = &shared_items[position];
                let conversion = async {
                    match result {
                        Ok(crawl) => output_from_crawl(crawl, &shared.config, shared.index).await,
                        Err(error) => Err(map_crawl_error(error)),
                    }
                };
                items[shared.index] = Some(finalize_shared_item(shared, batch_started, conversion).await);
            }
        }
    }

    fill_dropped_shared_slots(&shared_items, items, unmatched_errors);
}

/// Guarantee every shared input yields exactly one result-or-error.
///
/// Any slot still `None` after the batch results were drained means a returned
/// pair did not map back to it (e.g. a panicked task's empty-URL pair). Without
/// this the final reduce in [`extract_batch`] skips the `None` slot, so the
/// input would vanish from BOTH `results` and `errors` while `summary.inputs`
/// still counts it. Re-attach a captured unmatched error when one is available
/// (FIFO), otherwise synthesize one carrying the input's URL.
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion", not(target_arch = "wasm32")))]
fn fill_dropped_shared_slots(
    shared_items: &[SharedUrlItem],
    items: &mut [Option<BatchItemResult>],
    mut unmatched_errors: VecDeque<XbergError>,
) {
    for shared in shared_items {
        if items[shared.index].is_none() {
            let error = unmatched_errors
                .pop_front()
                .unwrap_or_else(|| XbergError::Other(format!("no batch result returned for URL: {}", shared.uri)));
            items[shared.index] = Some(BatchItemResult {
                index: shared.index,
                source: shared.source.clone(),
                result: Err(error),
            });
        }
    }
}

/// Apply batch-mode context, the per-item conversion timeout, and duration
/// metadata to a shared-URL conversion future, mirroring `run_batch_item`.
///
/// `batch_started` precedes the shared fetch, so reported duration covers both
/// fetch and conversion. The timeout still starts immediately before conversion.
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion", not(target_arch = "wasm32")))]
async fn finalize_shared_item<Fut>(shared: &SharedUrlItem, batch_started: Instant, conversion: Fut) -> BatchItemResult
where
    Fut: Future<Output = Result<ExtractionResult>>,
{
    let conversion_started = Instant::now();
    let future = Box::pin(crate::core::batch_mode::with_batch_mode(conversion));

    let mut result = match shared.config.extraction_timeout_secs {
        Some(secs) => match tokio::time::timeout(std::time::Duration::from_secs(secs), future).await {
            Ok(inner) => inner,
            Err(_elapsed) => {
                if let Some(ref token) = shared.config.cancel_token {
                    token.cancel();
                }
                Err(XbergError::Timeout {
                    elapsed_ms: conversion_started.elapsed().as_millis() as u64,
                    limit_ms: secs * 1000,
                })
            }
        },
        None => future.await,
    };

    if let Ok(ref mut item_output) = result {
        let elapsed_ms = batch_started.elapsed().as_millis() as u64;
        for extraction_result in &mut item_output.results {
            extraction_result.metadata.extraction_duration_ms = Some(elapsed_ms);
        }
    }

    BatchItemResult {
        index: shared.index,
        source: shared.source.clone(),
        result,
    }
}

/// Rebuild a non-cloneable crawl-engine construction error so the identical
/// failure can be isolated into every shared-URL error slot.
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion", not(target_arch = "wasm32")))]
fn duplicate_construction_error(error: &XbergError) -> XbergError {
    match error {
        XbergError::Validation { message, .. } => XbergError::validation(message.clone()),
        XbergError::UnsupportedFormat(message) => XbergError::UnsupportedFormat(message.clone()),
        other => XbergError::Other(other.to_string()),
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
struct BatchItemResult {
    index: usize,
    source: String,
    result: Result<ExtractionResult>,
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
async fn run_batch_item<F, Fut>(
    index: usize,
    source: String,
    timeout_secs: Option<u64>,
    cancel_token: Option<crate::cancellation::CancellationToken>,
    extract_fn: F,
) -> BatchItemResult
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<ExtractionResult>>,
{
    let start = Instant::now();
    let extraction_future = Box::pin(crate::core::batch_mode::with_batch_mode(Box::pin(extract_fn())));

    let mut result = match timeout_secs {
        Some(secs) => match tokio::time::timeout(std::time::Duration::from_secs(secs), extraction_future).await {
            Ok(inner) => inner,
            Err(_elapsed) => {
                if let Some(ref token) = cancel_token {
                    token.cancel();
                }
                Err(XbergError::Timeout {
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    limit_ms: secs * 1000,
                })
            }
        },
        None => extraction_future.await,
    };

    if let Ok(ref mut item_output) = result {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        for extraction_result in &mut item_output.results {
            extraction_result.metadata.extraction_duration_ms = Some(elapsed_ms);
        }
    }

    BatchItemResult { index, source, result }
}

fn append_extraction_output(output: &mut ExtractionResult, mut item_output: ExtractionResult) {
    output.summary.remote_urls += item_output.summary.remote_urls;
    output.summary.pages_crawled += item_output.summary.pages_crawled;
    output.summary.documents_downloaded += item_output.summary.documents_downloaded;
    output.results.append(&mut item_output.results);
    output.errors.append(&mut item_output.errors);
    merge_crawl_summary(
        output,
        item_output.crawl_final_urls,
        item_output.crawl_redirect_count,
        item_output.crawl_unique_normalized_urls,
    );
}

async fn extract_one(input: ExtractInput, base_config: &ExtractionConfig, index: usize) -> Result<ExtractionResult> {
    let config = resolve_input_config(&input, base_config);
    extract_one_resolved(input, &config, index).await
}

fn resolve_input_config(input: &ExtractInput, base_config: &ExtractionConfig) -> ExtractionConfig {
    let mut resolved = input
        .config
        .as_ref()
        .map(|overrides| base_config.with_file_overrides(overrides))
        .unwrap_or_else(|| base_config.clone());
    // Install a token here, before this item's own timeout wrapper (`run_batch_item`,
    // `finalize_shared_item`) races against `extract_file`/`extract_bytes`'s inner
    // timeout — the outer wrapper starts first and has the same duration, so it always
    // wins that race, and its `token.cancel()` needs the SAME token this config's
    // extractor checkpoints observe. See `ExtractionConfig::ensure_cancel_token`.
    resolved.ensure_cancel_token();
    resolved
}

/// Resolve config for batch items, taking Arc<ExtractionConfig> to avoid unnecessary clones.
/// When there are no per-item overrides, this returns Arc::clone (cheap reference increment)
/// rather than cloning the inner ExtractionConfig.
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn resolve_input_config_arc(input: &ExtractInput, base_config: &Arc<ExtractionConfig>) -> Arc<ExtractionConfig> {
    match input.config.as_ref() {
        Some(overrides) => Arc::new(base_config.with_file_overrides(overrides)),
        None => Arc::clone(base_config),
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn resolve_batch_input_config(
    input: &ExtractInput,
    base_config: &Arc<ExtractionConfig>,
    thread_budget: usize,
) -> Arc<ExtractionConfig> {
    let resolved = resolve_input_config_arc(input, base_config);
    let needs_thread_budget =
        crate::core::config::concurrency::resolve_thread_budget(resolved.concurrency.as_ref()) != thread_budget;
    // See `ExtractionConfig::ensure_cancel_token` and `resolve_input_config`: this
    // item's `run_batch_item` timeout wrapper needs the same token its extractor
    // checkpoints observe, installed before that wrapper races `extract_one_resolved`.
    let needs_cancel_token = resolved.extraction_timeout_secs.is_some() && resolved.cancel_token.is_none();
    if !needs_thread_budget && !needs_cancel_token {
        return resolved;
    }

    let mut resolved = Arc::unwrap_or_clone(resolved);
    if needs_thread_budget {
        // Only the thread budget is divided across batch workers. A caller's
        // `max_concurrent_ocr` is a memory bound on the host, not a per-document
        // share, so it survives the rewrite; rebuilding the struct from scratch
        // silently dropped it. ~keep
        resolved.concurrency = Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(thread_budget),
            ..resolved.concurrency.unwrap_or_default()
        });
    }
    resolved.ensure_cancel_token();
    Arc::new(resolved)
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn resolve_batch_base_config(base_config: &Arc<ExtractionConfig>, thread_budget: usize) -> Arc<ExtractionConfig> {
    if crate::core::config::concurrency::resolve_thread_budget(base_config.concurrency.as_ref()) == thread_budget {
        return Arc::clone(base_config);
    }

    let mut resolved = (**base_config).clone();
    // As in `resolve_batch_input_config`: divide the thread budget, carry the rest. ~keep
    resolved.concurrency = Some(crate::core::config::ConcurrencyConfig {
        max_threads: Some(thread_budget),
        ..resolved.concurrency.unwrap_or_default()
    });
    Arc::new(resolved)
}

async fn extract_one_resolved(
    input: ExtractInput,
    config: &ExtractionConfig,
    index: usize,
) -> Result<ExtractionResult> {
    // Type-erased per arm. An inline `.await` embeds BOTH arms' coroutines in this
    // function's state, and that state is inlined into `extract_one`, which is
    // alloca'd at four `Box::pin` sites and embedded again in `extract_uncached` --
    // so every byte here is paid many times over. Boxing leaves an 8-byte pointer and
    // makes the two arms mutually exclusive in memory as well as in control flow.
    //
    // No `+ Send` on wasm32; see the `extract` entry point above. ~keep
    #[cfg(not(target_arch = "wasm32"))]
    let resolved: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>> + Send + '_>> =
        match input.kind {
            ExtractInputKind::Bytes => Box::pin(extract_bytes_input(input, config, index)),
            ExtractInputKind::Uri => Box::pin(extract_uri_input(input, config, index)),
        };
    #[cfg(target_arch = "wasm32")]
    let resolved: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>> + '_>> = match input.kind
    {
        ExtractInputKind::Bytes => Box::pin(extract_bytes_input(input, config, index)),
        ExtractInputKind::Uri => Box::pin(extract_uri_input(input, config, index)),
    };
    resolved.await
}

async fn extract_bytes_input(input: ExtractInput, config: &ExtractionConfig, index: usize) -> Result<ExtractionResult> {
    let filename = input.filename;
    let mime_type = input.mime_type;
    let bytes = input
        .bytes
        .ok_or_else(|| XbergError::validation("extract input kind 'bytes' requires the 'bytes' field".to_string()))?;
    let (bytes, mime_type) = resolve_owned_bytes_mime(bytes, mime_type, filename.clone(), config).await?;
    let mut cfg = config.clone();
    cfg.source_name.clone_from(&filename);
    let mut result = Box::pin(extract_bytes(&bytes, &mime_type, &cfg)).await?;
    annotate_source(
        &mut result,
        "bytes",
        filename.as_deref().unwrap_or("<bytes>"),
        filename.as_deref().unwrap_or("<bytes>"),
        index,
    );
    Ok(ExtractionResult::single(result))
}

async fn extract_uri_input(input: ExtractInput, config: &ExtractionConfig, index: usize) -> Result<ExtractionResult> {
    let uri = input
        .uri
        .ok_or_else(|| XbergError::validation("extract input kind 'uri' requires the 'uri' field".to_string()))?;

    if uri.starts_with(HTTP_SCHEME) || uri.starts_with(HTTPS_SCHEME) {
        // Type-erased, not merely boxed, and for a stack reason rather than a `Send`-proof one.
        // This branch is not taken for the overwhelmingly common local-path/`file://` input, yet
        // an inline `.await` folds the entire URL-ingestion subtree — `extract_remote_uri` (which
        // holds a `CrawlConfig` by value plus a crawlberg engine future), `output_from_scrape`,
        // `output_from_crawl`, `extract_downloaded_document` and `run_url_page_pipeline`, the last
        // two each embedding a full unboxed `extract_bytes` pipeline — into this coroutine's TYPE.
        // That type is paid unconditionally: it inflates `extract_one_resolved` -> `extract_one`,
        // and `extract_one` is materialised in an `alloca` at every `Box::pin(extract_one(..))`
        // call site (rust-lang/rust#54628), so the cost lands on the stack of every extraction,
        // remote or not. A bare `Box::pin` does NOT fix this: `Pin<Box<Concrete>>` keeps the
        // concrete coroutine as a type parameter. Coercing to `dyn Future` turns the whole subtree
        // into a 16-byte pointer here and moves its materialisation into a frame cost paid only
        // when an HTTP(S) URI is actually seen. ~keep
        //
        // No `+ Send` on wasm32, for the same reason as `extract`/`extract_batch` above: extractor
        // futures are `!Send` there (`async_trait(?Send)`), as are crawlberg/reqwest's JS-backed
        // wasm futures, and wasm32 has no `tokio::spawn` that would need the bound. ~keep
        #[cfg(not(target_arch = "wasm32"))]
        let remote: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>> + Send + '_>> =
            Box::pin(extract_remote_uri(&uri, config, index));
        #[cfg(target_arch = "wasm32")]
        let remote: std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExtractionResult>> + '_>> =
            Box::pin(extract_remote_uri(&uri, config, index));
        return remote.await;
    }

    if uri.contains("://") && !uri.starts_with(FILE_SCHEME) {
        return Err(XbergError::UnsupportedFormat(format!(
            "unsupported URI scheme for extraction input: {uri}"
        )));
    }

    let path = if uri.starts_with(FILE_SCHEME) {
        if !config.url.allow_local_file_inputs || !config.url.allow_file_uris {
            return Err(XbergError::validation(
                "file:// URI inputs are disabled by configuration".to_string(),
            ));
        }
        file_uri_to_path(&uri)?
    } else {
        if !config.url.allow_local_file_inputs {
            return Err(XbergError::validation(
                "local filesystem path inputs are disabled by configuration".to_string(),
            ));
        }
        PathBuf::from(&uri)
    };

    let mut result = Box::pin(extract_file(&path, input.mime_type.as_deref(), config)).await?;
    annotate_source(&mut result, "uri", &uri, path.to_string_lossy().as_ref(), index);
    Ok(ExtractionResult::single(result))
}

async fn resolve_owned_bytes_mime(
    bytes: Vec<u8>,
    mime_type: Option<String>,
    filename: Option<String>,
    config: &ExtractionConfig,
) -> Result<(Vec<u8>, String)> {
    crate::core::mime::resolve_owned_bytes_mime(bytes, filename, mime_type, config.mime_detection_policy).await
}

#[cfg(all(test, feature = "tokio-runtime"))]
fn resolve_bytes_mime_type(
    mime_type: Option<&str>,
    filename: Option<&str>,
    bytes: &[u8],
    config: &ExtractionConfig,
) -> Result<String> {
    let explicit = mime_type.filter(|mime| *mime != "application/octet-stream");
    crate::core::mime::detect_or_validate_bytes(bytes, filename, explicit, config.mime_detection_policy)
}

fn file_uri_to_path(uri: &str) -> Result<PathBuf> {
    let parsed = url::Url::parse(uri)
        .map_err(|error| XbergError::validation(format!("invalid file URI for extraction input: {error}")))?;
    if parsed.scheme() != "file" {
        return Err(XbergError::UnsupportedFormat(format!(
            "unsupported URI scheme for local file extraction: {}",
            parsed.scheme()
        )));
    }

    if let Some(host) = parsed.host_str().filter(|host| !host.is_empty())
        && !host.eq_ignore_ascii_case("localhost")
    {
        return Err(XbergError::UnsupportedFormat(format!(
            "unsupported non-local file URI host: {host}"
        )));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        parsed
            .to_file_path()
            .map_err(|()| XbergError::UnsupportedFormat(format!("unsupported file URI path: {uri}")))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = parsed;
        Err(XbergError::UnsupportedFormat(format!(
            "file URIs are not supported on this platform: {uri}"
        )))
    }
}

fn annotate_source(result: &mut ExtractedDocument, source_kind: &str, source_uri: &str, final_uri: &str, index: usize) {
    result
        .metadata
        .additional
        .insert("source_kind".into(), serde_json::json!(source_kind));
    result
        .metadata
        .additional
        .insert("source_uri".into(), serde_json::json!(source_uri));
    result
        .metadata
        .additional
        .insert("final_uri".into(), serde_json::json!(final_uri));
    result
        .metadata
        .additional
        .insert("source_index".into(), serde_json::json!(index));
}

fn input_source(input: &ExtractInput) -> String {
    match input.kind {
        ExtractInputKind::Bytes => input.filename.clone().unwrap_or_else(|| "<bytes>".to_string()),
        ExtractInputKind::Uri => input.uri.clone().unwrap_or_else(|| "<uri>".to_string()),
    }
}

#[cfg(feature = "url-ingestion")]
async fn extract_remote_uri(uri: &str, config: &ExtractionConfig, index: usize) -> Result<ExtractionResult> {
    let crawl_config = crawlberg_config(config)?;
    let engine = CrawlEngine::builder()
        .config(crawl_config)
        .build()
        .map_err(map_crawl_error)?;

    match config.url.mode {
        UrlExtractionMode::Auto | UrlExtractionMode::Document => {
            let scrape = engine.scrape(uri).await.map_err(map_crawl_error)?;
            output_from_scrape(scrape, config, index).await
        }
        UrlExtractionMode::Crawl => {
            let crawl = engine.crawl(uri).await.map_err(map_crawl_error)?;
            output_from_crawl(crawl, config, index).await
        }
    }
}

#[cfg(not(feature = "url-ingestion"))]
async fn extract_remote_uri(uri: &str, _config: &ExtractionConfig, _index: usize) -> Result<ExtractionResult> {
    Err(XbergError::UnsupportedFormat(format!(
        "HTTP(S) URI extraction requires the 'url-ingestion' feature: {uri}"
    )))
}

#[cfg(feature = "url-ingestion")]
fn crawlberg_config(config: &ExtractionConfig) -> Result<CrawlConfig> {
    let crawl_config = config.url.crawl.clone();
    crawl_config.validate().map_err(map_crawl_error)?;
    Ok(crawl_config)
}

#[cfg(feature = "url-ingestion")]
async fn output_from_scrape(scrape: ScrapeResult, config: &ExtractionConfig, index: usize) -> Result<ExtractionResult> {
    let final_url = scrape.final_url.clone();
    let mut output = ExtractionResult {
        summary: ExtractionSummary {
            inputs: 1,
            remote_urls: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    if let Some(document) = scrape.downloaded_document {
        let result = extract_downloaded_document(document, config, index).await?;
        output.results.push(result);
        output.summary.documents_downloaded = 1;
    } else {
        output
            .results
            .push(result_from_scrape_page(scrape, config, index).await?);
        output.summary.pages_crawled = 1;
    }

    merge_crawl_summary(&mut output, vec![final_url], 0, Vec::new());
    output.refresh_counts();
    Ok(output)
}

#[cfg(feature = "url-ingestion")]
async fn output_from_crawl(
    crawl: crawlberg::CrawlResult,
    config: &ExtractionConfig,
    index: usize,
) -> Result<ExtractionResult> {
    let final_url = crawl.final_url.clone();
    let redirect_count = crawl.redirect_count;
    // Derived from `pages`, not from the deprecated `CrawlResult::normalized_urls`.
    // That field is no longer populated upstream — crawlberg 1.2.x keeps it only to
    // avoid a breaking field removal and `CrawlResult::new` drops the value on the
    // floor — so reading it yielded an always-empty `Vec` and silently lost every
    // crawled URL from the summary. `CrawlResult::unique_normalized_urls()` is the
    // documented replacement but returns a *count*; `merge_crawl_summary` needs the
    // strings themselves, so mirror that method's own derivation instead. Dedup here
    // matches its `AHashSet` semantics while preserving first-seen order. ~keep
    let unique_normalized_urls = {
        let mut seen = std::collections::HashSet::new();
        crawl
            .pages
            .iter()
            .filter(|page| seen.insert(page.normalized_url.as_str()))
            .map(|page| page.normalized_url.clone())
            .collect::<Vec<String>>()
    };
    let crawl_error = crawl.error.clone();
    let mut output = ExtractionResult {
        summary: ExtractionSummary {
            inputs: 1,
            remote_urls: 1,
            pages_crawled: crawl.pages.len(),
            ..Default::default()
        },
        ..Default::default()
    };

    for page in crawl.pages {
        if let Some(document) = page.downloaded_document.clone() {
            match extract_downloaded_document(document, config, index).await {
                Ok(result) => {
                    output.results.push(result);
                    output.summary.documents_downloaded += 1;
                }
                Err(error) => output.errors.push(error_item(index, page.url.clone(), &error)),
            }
        } else {
            let page_source = page.url.clone();
            match result_from_crawl_page(page, config, index).await {
                Ok(result) => output.results.push(result),
                Err(error) => output.errors.push(error_item(index, page_source, &error)),
            }
        }
    }

    if let Some(error) = crawl_error {
        output.errors.push(ExtractionErrorItem {
            index,
            code: 1099,
            error_type: "crawl".into(),
            source: final_url.clone(),
            message: error,
        });
    }

    merge_crawl_summary(&mut output, vec![final_url], redirect_count, unique_normalized_urls);
    output.refresh_counts();
    Ok(output)
}

fn merge_crawl_summary(
    output: &mut ExtractionResult,
    final_urls: Vec<String>,
    redirect_count: usize,
    unique_normalized_urls: Vec<String>,
) {
    if final_urls.is_empty() && redirect_count == 0 && unique_normalized_urls.is_empty() {
        return;
    }

    output.crawl_redirect_count += redirect_count;
    for url in final_urls {
        if !output.crawl_final_urls.contains(&url) {
            output.crawl_final_urls.push(url);
        }
    }
    for url in unique_normalized_urls {
        if !output.crawl_unique_normalized_urls.contains(&url) {
            output.crawl_unique_normalized_urls.push(url);
        }
    }
}

#[cfg(feature = "url-ingestion")]
async fn extract_downloaded_document(
    document: DownloadedDocument,
    config: &ExtractionConfig,
    index: usize,
) -> Result<ExtractedDocument> {
    let mime_hint = Some(document.mime_type.clone().into_owned());
    let filename_hint = document.filename.as_deref().map(str::to_owned);
    let (content, mime_type) = resolve_owned_bytes_mime(document.content, mime_hint, filename_hint, config).await?;
    let mut cfg = config.clone();
    cfg.source_name = document.filename.as_deref().map(str::to_string);
    let mut result = extract_bytes(&content, &mime_type, &cfg).await?;
    annotate_source(&mut result, "url_document", &document.url, &document.url, index);
    result
        .metadata
        .additional
        .insert("downloaded_size".into(), serde_json::json!(document.size));
    result
        .metadata
        .additional
        .insert("content_hash".into(), serde_json::json!(document.content_hash));
    if let Some(filename) = document.filename {
        result
            .metadata
            .additional
            .insert("filename".into(), serde_json::json!(filename));
    }
    Ok(result)
}

#[cfg(feature = "url-ingestion")]
async fn result_from_scrape_page(
    scrape: ScrapeResult,
    config: &ExtractionConfig,
    index: usize,
) -> Result<ExtractedDocument> {
    let final_url = scrape.final_url.clone();
    let status_code = scrape.status_code;
    let browser_used = scrape.browser_used;
    let content_type = scrape.content_type.clone();
    let content = scrape
        .markdown
        .as_ref()
        .map(|markdown| markdown.content.clone())
        .filter(|content| !content.is_empty())
        .unwrap_or_else(|| scrape.html.clone());
    let mut result = run_url_page_pipeline(
        content,
        scrape.markdown.is_some(),
        &content_type,
        &scrape.html,
        links_to_uris(scrape.links.iter().map(|link| (&link.url, &link.text))),
        config,
    )
    .await?;
    annotate_source(&mut result, "url_page", &final_url, &final_url, index);
    result
        .metadata
        .additional
        .insert("status_code".into(), serde_json::json!(status_code));
    result
        .metadata
        .additional
        .insert("browser_used".into(), serde_json::json!(browser_used));
    Ok(result)
}

#[cfg(feature = "url-ingestion")]
async fn result_from_crawl_page(
    page: CrawlPageResult,
    config: &ExtractionConfig,
    index: usize,
) -> Result<ExtractedDocument> {
    let url = page.url.clone();
    let normalized_url = page.normalized_url.clone();
    let status_code = page.status_code;
    let depth = page.depth;
    let browser_used = page.browser_used;
    let content_type = page.content_type.clone();
    let content = page
        .markdown
        .as_ref()
        .map(|markdown| markdown.content.clone())
        .filter(|content| !content.is_empty())
        .unwrap_or_else(|| page.html.clone());
    let mut result = run_url_page_pipeline(
        content,
        page.markdown.is_some(),
        &content_type,
        &page.html,
        links_to_uris(page.links.iter().map(|link| (&link.url, &link.text))),
        config,
    )
    .await?;
    annotate_source(&mut result, "url_page", &url, &normalized_url, index);
    result
        .metadata
        .additional
        .insert("status_code".into(), serde_json::json!(status_code));
    result
        .metadata
        .additional
        .insert("crawl_depth".into(), serde_json::json!(depth));
    result
        .metadata
        .additional
        .insert("browser_used".into(), serde_json::json!(browser_used));
    Ok(result)
}

#[cfg(feature = "url-ingestion")]
async fn run_url_page_pipeline(
    content: String,
    is_markdown: bool,
    content_type: &str,
    source_html: &str,
    uris: Vec<ExtractedUri>,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    #[cfg(not(feature = "html"))]
    let _ = source_html;
    let source_mime_type = normalized_content_type(content_type);
    let extraction_mime_type = if is_markdown {
        "text/markdown".to_string()
    } else {
        source_mime_type.clone()
    };
    #[cfg(feature = "html")]
    let is_html_source = source_mime_type.starts_with("text/html");
    let mut result = extract_bytes(content.as_bytes(), &extraction_mime_type, config).await?;
    result.mime_type = source_mime_type.into();

    // ~keep The line above restamps the result `text/html`, so a consumer reading
    // `metadata.format.html` is entitled to find it. But when crawlberg supplied pre-rendered
    // markdown the extraction ran as `text/markdown`, so the HTML extractor never ran and no
    // HtmlMetadata was ever produced -- leaving `format: None` contradicting the MIME type
    // (CI E2E `test_metadata_access`). Recover it from the page HTML instead.
    #[cfg(feature = "html")]
    if is_html_source
        && result.metadata.format.is_none()
        && !source_html.is_empty()
        && let Some(html_metadata) = crate::extraction::html::extract_html_metadata_only(source_html)
    {
        result.metadata.format = Some(crate::types::FormatMetadata::Html(Box::new(html_metadata)));
    }
    match result.uris.as_mut() {
        Some(existing) => existing.extend(uris),
        None if !uris.is_empty() => result.uris = Some(uris),
        None => {}
    }
    Ok(result)
}

#[cfg(feature = "url-ingestion")]
fn normalized_content_type(content_type: &str) -> String {
    crate::core::mime::validate_mime_type(content_type).unwrap_or_else(|_| "text/html".to_string())
}

#[cfg(feature = "url-ingestion")]
fn links_to_uris<'a>(links: impl Iterator<Item = (&'a String, &'a String)>) -> Vec<ExtractedUri> {
    links
        .map(|(url, text)| ExtractedUri {
            url: url.clone(),
            label: if text.is_empty() { None } else { Some(text.clone()) },
            page: None,
            kind: UriKind::Hyperlink,
        })
        .collect()
}

#[cfg(feature = "url-ingestion")]
pub(crate) fn map_crawl_error(error: crawlberg::CrawlError) -> XbergError {
    XbergError::validation(format!("crawlberg URL extraction failed: {error}"))
}

async fn follow_recursive_document_urls(
    output: &mut ExtractionResult,
    config: &ExtractionConfig,
    seen: &mut HashSet<String>,
    seed_hosts: &HashSet<String>,
) -> Result<()> {
    if !follow_document_urls(config) {
        return Ok(());
    }

    let max_depth = document_url_depth(config);
    if max_depth == 0 {
        return Ok(());
    }

    let pattern = config
        .url
        .document_url_pattern
        .as_deref()
        .map(regex::Regex::new)
        .transpose()
        .map_err(|error| XbergError::validation(format!("invalid document_url_pattern regex: {error}")))?;

    let mut queue = VecDeque::new();
    enqueue_discovered_urls(output, config, &pattern, seed_hosts, seen, &mut queue, 1);

    while let Some((uri, depth)) = queue.pop_front() {
        let index = output.summary.inputs;
        output.summary.inputs += 1;

        match Box::pin(extract_one(ExtractInput::from_uri(uri.clone()), config, index)).await {
            Ok(mut item_output) => {
                if depth < max_depth {
                    enqueue_discovered_urls(&item_output, config, &pattern, seed_hosts, seen, &mut queue, depth + 1);
                }
                output.summary.remote_urls += item_output.summary.remote_urls;
                output.summary.pages_crawled += item_output.summary.pages_crawled;
                output.summary.documents_downloaded += item_output.summary.documents_downloaded;
                output.results.append(&mut item_output.results);
                output.errors.append(&mut item_output.errors);
            }
            Err(error) => output.errors.push(error_item(index, uri, &error)),
        }
    }

    output.refresh_counts();
    Ok(())
}

fn enqueue_discovered_urls(
    output: &ExtractionResult,
    config: &ExtractionConfig,
    pattern: &Option<regex::Regex>,
    seed_hosts: &HashSet<String>,
    seen: &mut HashSet<String>,
    queue: &mut VecDeque<(String, u32)>,
    depth: u32,
) {
    let max_total = config.url.max_total_urls.unwrap_or(1_000) as usize;
    if seen.len() >= max_total {
        return;
    }

    for result in &output.results {
        let max_per_result = config.url.max_document_urls_per_result.unwrap_or(100) as usize;
        for url in urls_from_result(result).into_iter().take(max_per_result) {
            if seen.len() >= max_total {
                return;
            }
            if should_follow_discovered_url(&url, config, pattern, seed_hosts) && seen.insert(url.clone()) {
                queue.push_back((url, depth));
            }
        }
    }
}

fn urls_from_result(result: &ExtractedDocument) -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(uris) = &result.uris {
        urls.extend(
            uris.iter()
                .filter(|uri| matches!(uri.kind, UriKind::Hyperlink | UriKind::Reference | UriKind::Citation))
                .map(|uri| uri.url.clone()),
        );
    }

    let text_regex = regex::Regex::new(r#"https?://[^\s<>"')\]]+"#).expect("static URL regex is valid");
    urls.extend(text_regex.find_iter(&result.content).map(|matched| {
        matched
            .as_str()
            .trim_end_matches(['.', ',', ';', ':', '!', '?'])
            .to_string()
    }));
    urls
}

fn should_follow_discovered_url(
    url: &str,
    config: &ExtractionConfig,
    pattern: &Option<regex::Regex>,
    seed_hosts: &HashSet<String>,
) -> bool {
    if !(url.starts_with(HTTP_SCHEME) || url.starts_with(HTTPS_SCHEME)) {
        return false;
    }
    if let Some(pattern) = pattern
        && !pattern.is_match(url)
    {
        return false;
    }
    if stay_on_domain(config)
        && !seed_hosts.is_empty()
        && let Some(host) = http_host(url)
    {
        return seed_hosts.contains(&host)
            || (allow_subdomains(config) && seed_hosts.iter().any(|seed| host.ends_with(&format!(".{seed}"))));
    }
    true
}

fn follow_document_urls(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "url-ingestion")]
    {
        config.url.crawl.follow_document_urls
    }
    #[cfg(not(feature = "url-ingestion"))]
    {
        let _ = config;
        false
    }
}

fn document_url_depth(config: &ExtractionConfig) -> u32 {
    #[cfg(feature = "url-ingestion")]
    {
        config
            .url
            .crawl
            .document_url_depth
            .or_else(|| config.url.crawl.max_depth.map(|depth| depth as u32))
            .unwrap_or(1)
    }
    #[cfg(not(feature = "url-ingestion"))]
    {
        let _ = config;
        0
    }
}

fn stay_on_domain(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "url-ingestion")]
    {
        config.url.crawl.stay_on_domain
    }
    #[cfg(not(feature = "url-ingestion"))]
    {
        let _ = config;
        false
    }
}

fn allow_subdomains(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "url-ingestion")]
    {
        config.url.crawl.allow_subdomains
    }
    #[cfg(not(feature = "url-ingestion"))]
    {
        let _ = config;
        false
    }
}

fn initial_seen_urls(inputs: &[ExtractInput]) -> HashSet<String> {
    inputs
        .iter()
        .filter_map(|input| input.uri.clone())
        .filter(|uri| uri.starts_with(HTTP_SCHEME) || uri.starts_with(HTTPS_SCHEME))
        .collect()
}

fn initial_seed_hosts(inputs: &[ExtractInput]) -> HashSet<String> {
    inputs
        .iter()
        .filter_map(|input| input.uri.as_deref().and_then(http_host))
        .collect()
}

fn http_host(uri: &str) -> Option<String> {
    let parsed = url::Url::parse(uri).ok()?;
    match parsed.scheme() {
        "http" | "https" => parsed.host_str().map(|host| host.to_ascii_lowercase()),
        _ => None,
    }
}

fn error_item(index: usize, source: String, error: &XbergError) -> ExtractionErrorItem {
    ExtractionErrorItem {
        index,
        code: error_code(error),
        error_type: error_type(error).to_string(),
        source,
        message: error.to_string(),
    }
}

fn error_code(error: &XbergError) -> u32 {
    error.extraction_error_code()
}

fn error_type(error: &XbergError) -> &'static str {
    error.extraction_error_type()
}

#[cfg(all(test, feature = "tokio-runtime"))]
mod tests;
