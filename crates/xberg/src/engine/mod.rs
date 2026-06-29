//! Rust-only extraction engine.
//!
//! [`Engine`] owns the extraction internals that previously lived as free
//! functions in [`crate::core::extract`]. The crate-level [`crate::extract`]
//! and [`crate::extract_batch`] functions delegate to a process-global default
//! [`Engine`]. This is a pure refactor: behavior is identical to the previous
//! free-function implementation.
//!
//! This module is intentionally **not** part of the language-binding surface.
//! It is declared with a bare `pub mod engine;` in `lib.rs` and its files are
//! not listed in `alef.toml` `sources`, so the binding generator emits nothing
//! for it. The public types here are also listed in `alef.toml`
//! `[crates.exclude] types` as belt-and-suspenders.

use std::sync::Arc;

use crate::Result;
use crate::core::config::{ExtractInput, ExtractionConfig, ExtractionResult};

mod crawl_handle;
mod extract_impl;
pub mod seams;

use seams::{CacheBackend, NoopCache, NoopProgressSink, ProgressSink};
#[cfg(feature = "presets")]
use seams::{CorePresetResolver, PresetResolver};
#[cfg(feature = "layout-detection")]
use seams::{DefaultModelProvider, ModelProvider};
#[cfg(feature = "heuristics")]
use seams::{DefaultStructuredPolicy, StructuredPolicy};
#[cfg(feature = "liter-llm")]
use seams::{LiterLlmClient, LlmClient};

/// Internal engine state.
///
/// Holds the process-shared, fingerprinted crawl-engine memo so that multi-URL
/// batch extraction can reuse a single [`crawlberg::CrawlEngine`] (and its
/// shared middleware/cache/rate-limiter) across all URLs in a batch, plus the
/// six injected extension seams (each filled with its in-core default by
/// [`EngineBuilder::build`]). The single-URL `extract` path does not touch this
/// state.
struct EngineInner {
    /// Single-slot, fingerprinted memo of the last-built crawl engine. The slot
    /// is reused when the incoming [`crawlberg::CrawlConfig`] fingerprint
    /// matches, otherwise a fresh engine is built and stored.
    #[cfg(feature = "url-ingestion")]
    crawl: parking_lot::Mutex<Option<crawl_handle::CrawlHandleMemo>>,

    /// Content-addressed byte cache. Default: [`NoopCache`].
    cache: Arc<dyn CacheBackend>,
    /// Progress event sink. Default: [`NoopProgressSink`].
    progress: Arc<dyn ProgressSink>,

    /// Structured-extraction call-mode policy. Default: [`DefaultStructuredPolicy`].
    #[cfg(feature = "heuristics")]
    structured_policy: Arc<dyn StructuredPolicy>,
    /// Built-in preset resolver. Default: [`CorePresetResolver`].
    #[cfg(feature = "presets")]
    preset_resolver: Arc<dyn PresetResolver>,
    /// JSON-schema LLM client. Default: [`LiterLlmClient`].
    #[cfg(feature = "liter-llm")]
    llm_client: Arc<dyn LlmClient>,
    /// On-demand model-weight provider. Default: [`DefaultModelProvider`].
    #[cfg(feature = "layout-detection")]
    model_provider: Arc<dyn ModelProvider>,
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
    pub async fn extract(&self, input: ExtractInput, config: &ExtractionConfig) -> Result<ExtractionResult> {
        extract_impl::extract(input, config).await
    }

    /// Extract content from multiple bytes or URI inputs.
    pub async fn extract_batch(
        &self,
        inputs: Vec<ExtractInput>,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        extract_impl::extract_batch(&self.inner, inputs, config).await
    }

    /// The injected [`CacheBackend`] seam (default: [`NoopCache`]).
    pub fn cache_backend(&self) -> &Arc<dyn CacheBackend> {
        &self.inner.cache
    }

    /// The injected [`ProgressSink`] seam (default: [`NoopProgressSink`]).
    pub fn progress_sink(&self) -> &Arc<dyn ProgressSink> {
        &self.inner.progress
    }

    /// The injected [`StructuredPolicy`] seam (default: [`DefaultStructuredPolicy`]).
    #[cfg(feature = "heuristics")]
    pub fn structured_policy(&self) -> &Arc<dyn StructuredPolicy> {
        &self.inner.structured_policy
    }

    /// The injected [`PresetResolver`] seam (default: [`CorePresetResolver`]).
    #[cfg(feature = "presets")]
    pub fn preset_resolver(&self) -> &Arc<dyn PresetResolver> {
        &self.inner.preset_resolver
    }

    /// The injected [`LlmClient`] seam (default: [`LiterLlmClient`]).
    #[cfg(feature = "liter-llm")]
    pub fn llm_client(&self) -> &Arc<dyn LlmClient> {
        &self.inner.llm_client
    }

    /// The injected [`ModelProvider`] seam (default: [`DefaultModelProvider`]).
    #[cfg(feature = "layout-detection")]
    pub fn model_provider(&self) -> &Arc<dyn ModelProvider> {
        &self.inner.model_provider
    }
}

/// Builder for [`Engine`].
///
/// Each extension seam left unset is filled with its in-core default by
/// [`build`](EngineBuilder::build), so [`Engine::new_default`] produces an
/// engine whose seams are exactly those defaults.
#[derive(Default)]
pub struct EngineBuilder {
    cache: Option<Arc<dyn CacheBackend>>,
    progress: Option<Arc<dyn ProgressSink>>,
    #[cfg(feature = "heuristics")]
    structured_policy: Option<Arc<dyn StructuredPolicy>>,
    #[cfg(feature = "presets")]
    preset_resolver: Option<Arc<dyn PresetResolver>>,
    #[cfg(feature = "liter-llm")]
    llm_client: Option<Arc<dyn LlmClient>>,
    #[cfg(feature = "layout-detection")]
    model_provider: Option<Arc<dyn ModelProvider>>,
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

    /// Inject a [`StructuredPolicy`], overriding the [`DefaultStructuredPolicy`] default.
    #[cfg(feature = "heuristics")]
    pub fn with_structured_policy(mut self, policy: Arc<dyn StructuredPolicy>) -> Self {
        self.structured_policy = Some(policy);
        self
    }

    /// Inject a [`PresetResolver`], overriding the [`CorePresetResolver`] default.
    #[cfg(feature = "presets")]
    pub fn with_preset_resolver(mut self, resolver: Arc<dyn PresetResolver>) -> Self {
        self.preset_resolver = Some(resolver);
        self
    }

    /// Inject an [`LlmClient`], overriding the [`LiterLlmClient`] default.
    #[cfg(feature = "liter-llm")]
    pub fn with_llm_client(mut self, client: Arc<dyn LlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    /// Inject a [`ModelProvider`], overriding the [`DefaultModelProvider`] default.
    #[cfg(feature = "layout-detection")]
    pub fn with_model_provider(mut self, provider: Arc<dyn ModelProvider>) -> Self {
        self.model_provider = Some(provider);
        self
    }

    /// Finalize the builder into an [`Engine`], filling every unset seam with
    /// its in-core default.
    pub fn build(self) -> Engine {
        let inner = EngineInner {
            #[cfg(feature = "url-ingestion")]
            crawl: parking_lot::Mutex::new(None),
            cache: self.cache.unwrap_or_else(|| Arc::new(NoopCache)),
            progress: self.progress.unwrap_or_else(|| Arc::new(NoopProgressSink)),
            #[cfg(feature = "heuristics")]
            structured_policy: self
                .structured_policy
                .unwrap_or_else(|| Arc::new(DefaultStructuredPolicy::default())),
            #[cfg(feature = "presets")]
            preset_resolver: self.preset_resolver.unwrap_or_else(|| Arc::new(CorePresetResolver)),
            #[cfg(feature = "liter-llm")]
            llm_client: self.llm_client.unwrap_or_else(|| Arc::new(LiterLlmClient)),
            #[cfg(feature = "layout-detection")]
            model_provider: self
                .model_provider
                .unwrap_or_else(|| Arc::new(DefaultModelProvider::default())),
        };
        Engine { inner: Arc::new(inner) }
    }
}
