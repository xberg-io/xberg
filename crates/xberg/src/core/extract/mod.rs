//! Unified public extraction API.
//!
//! These functions are the stable, binding-generated public surface. Their
//! signatures must remain byte-identical (they are scanned by the alef binding
//! generator). The implementation delegates to a process-global default
//! [`crate::engine::Engine`]; the extraction internals live in
//! [`crate::engine`] and are a pure refactor of what previously lived here.

use std::sync::LazyLock;

use crate::Result;
#[cfg(feature = "url-ingestion")]
use crate::core::config::UrlExtractionConfig;
use crate::core::config::{ExtractInput, ExtractInputWithOptions, ExtractionConfig, ExtractionResult};
#[cfg(feature = "url-ingestion")]
use crawlberg::{CrawlEngine, MapResult};

/// Process-global default engine backing the free `extract` / `extract_batch`
/// functions. Construction is cheap and side-effect free.
static DEFAULT_ENGINE: LazyLock<crate::engine::Engine> = LazyLock::new(crate::engine::Engine::new_default);

/// Extract content from a single bytes or URI input.
pub async fn extract(input: ExtractInput, config: &ExtractionConfig) -> Result<ExtractionResult> {
    DEFAULT_ENGINE.extract(input, config).await
}

/// Extract one PDF bytes input with layout inference restricted to explicit 1-based pages. ~keep
///
/// This call enables layout on the native PDF route even when ordinary extraction leaves it disabled.
/// Selected pages override the automatic layout gate; an empty list skips layout inference.
/// Invalid pages are ignored with processing warnings. Both extraction caches are bypassed.
/// This additive entry point leaves existing extraction configuration structs unchanged.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub async fn extract_with_layout_pages(
    input: ExtractInput,
    config: &ExtractionConfig,
    pages: Vec<u32>,
) -> Result<ExtractionResult> {
    DEFAULT_ENGINE.extract_with_layout_pages(input, config, pages).await
}

/// Extract one bytes input and redact a JSON array or JSON Lines payload from an external inspection engine.
///
/// The payload is parsed in Rust so vendor aliases and nested fields remain intact across language bindings.
/// `offset_encoding` defaults to `unicode_code_points` and `max_findings` defaults to 10,000 when omitted.
/// Unknown encodings return a validation error.
#[cfg(feature = "redaction")]
#[cfg_attr(feature = "alef-meta", alef(since = "1.3.1"))]
pub async fn extract_with_external_redaction(
    input: ExtractInput,
    config: &ExtractionConfig,
    findings_json: &str,
    offset_encoding: Option<&str>,
    max_findings: Option<u32>,
) -> Result<ExtractionResult> {
    let offset_encoding = offset_encoding.unwrap_or("unicode_code_points").parse()?;
    let requested_limit = max_findings.unwrap_or(crate::text::redaction::external::DEFAULT_MAX_FINDINGS);
    let default_limits = crate::extractors::security::SecurityLimits::default();
    let security_limit = crate::text::redaction::external::security_finding_limit(
        config.security_limits.as_ref().unwrap_or(&default_limits),
    );
    let effective_limit = u32::try_from(security_limit.min(requested_limit as usize)).map_err(|_| {
        crate::XbergError::validation("effective redaction finding limit exceeds the supported u32 range".to_string())
    })?;
    let findings = crate::text::redaction::parse_external_findings_bounded(findings_json, effective_limit)?;
    DEFAULT_ENGINE
        .extract_with_external_redaction(input, config, findings, offset_encoding, max_findings)
        .await
}

/// Extract content from multiple bytes or URI inputs.
pub async fn extract_batch(inputs: Vec<ExtractInput>, config: &ExtractionConfig) -> Result<ExtractionResult> {
    DEFAULT_ENGINE.extract_batch(inputs, config).await
}

/// Extract a batch whose inputs carry additional per-input options.
pub async fn extract_batch_with_input_options(
    inputs: Vec<ExtractInputWithOptions>,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    DEFAULT_ENGINE.extract_batch_with_input_options(inputs, config).await
}

/// Extract a batch from a JSON array of per-input objects.
///
/// Uses the same fields and semantics as [`ExtractInputWithOptions`]. This is
/// useful for callers that pass input values through a text-based boundary.
pub async fn extract_batch_with_input_options_json(
    inputs_json: &str,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    let inputs = serde_json::from_str(inputs_json)
        .map_err(|error| crate::XbergError::validation(format!("Invalid batch inputs: {error}")))?;
    extract_batch_with_input_options(inputs, config).await
}

/// Discover all pages and sitemaps reachable from `uri` without extracting document content.
///
/// Builds a [`crawlberg::CrawlEngine`] from `config.crawl`, calls
/// [`CrawlEngine::map`], and returns the set of discovered URLs as a
/// [`crawlberg::MapResult`] (re-exported as [`crate::MapResult`]).
///
/// Use this when you need the URL inventory of a site before committing to
/// full document extraction — e.g. to build a crawl queue or validate scope.
///
/// # Errors
///
/// Returns [`crate::XbergError::Validation`] if the crawl configuration fails
/// validation or if the map operation itself fails.
#[cfg(feature = "url-ingestion")]
pub async fn map_url(uri: &str, config: &UrlExtractionConfig) -> Result<MapResult> {
    config.crawl.validate().map_err(map_crawl_err)?;
    let engine = CrawlEngine::builder()
        .config(config.crawl.clone())
        .build()
        .map_err(map_crawl_err)?;
    engine.map(uri).await.map_err(map_crawl_err)
}

/// Convert a [`crawlberg::CrawlError`] into an [`crate::XbergError`].
///
/// Mirrors the conversion used by the URL-ingestion extraction paths.
#[cfg(feature = "url-ingestion")]
fn map_crawl_err(error: crawlberg::CrawlError) -> crate::XbergError {
    crate::XbergError::validation(format!("crawlberg URL extraction failed: {error}"))
}
