//! File-based extraction operations.
//!
//! This module handles extraction from filesystem paths, including:
//! - MIME type detection and validation
//! - Legacy format conversion (DOC, PPT)
//! - File validation and reading
//! - Extraction pipeline orchestration

use crate::Result;
#[cfg(not(feature = "office"))]
use crate::XbergError;
use crate::core::config::ExtractionConfig;
use crate::core::mime::{LEGACY_POWERPOINT_MIME_TYPE, LEGACY_WORD_MIME_TYPE};
use crate::types::ExtractedDocument;
use std::path::Path;

use super::helpers::get_extractor;

/// Extract content from a file.
///
/// This is the main entry point for file-based extraction. It performs the following steps:
/// 1. Check cache for existing result (if caching enabled)
/// 2. Detect or validate MIME type
/// 3. Select appropriate extractor from registry
/// 4. Extract content
/// 5. Run post-processing pipeline
/// 6. Store result in cache (if caching enabled)
///
/// # Arguments
///
/// * `path` - Path to the file to extract
/// * `mime_type` - Optional MIME type override. If None, will be auto-detected
/// * `config` - Extraction configuration
///
/// # Returns
///
/// An `ExtractedDocument` containing the extracted content and metadata.
///
/// # Errors
///
/// Returns `XbergError::Io` if the file doesn't exist (NotFound) or for other file I/O errors.
/// Returns `XbergError::UnsupportedFormat` if MIME type is not supported.
///
/// # Example
///
/// ```rust,no_run
/// use xberg::core::extractor::extract_file;
/// use xberg::core::config::ExtractionConfig;
///
/// # async fn example() -> xberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let result = extract_file("document.pdf", None, &config).await?;
/// println!("Content: {}", result.content);
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "otel", tracing::instrument(
    skip(config, path),
    fields(
        { crate::telemetry::conventions::OPERATION } = crate::telemetry::conventions::operations::EXTRACT_FILE,
        { crate::telemetry::conventions::DOCUMENT_FILENAME } = tracing::field::Empty,
        { crate::telemetry::conventions::OTEL_STATUS_CODE } = tracing::field::Empty,
        { crate::telemetry::conventions::ERROR_TYPE } = tracing::field::Empty,
        { crate::telemetry::conventions::ERROR_MESSAGE } = tracing::field::Empty,
    )
))]
pub(crate) async fn extract_file(
    path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    use crate::core::{io, mime};

    let path = path.as_ref();

    #[cfg(feature = "otel")]
    {
        let span = tracing::Span::current();
        span.record(
            crate::telemetry::conventions::DOCUMENT_FILENAME,
            crate::telemetry::spans::sanitize_path(path),
        );
    }

    let extraction_future = Box::pin(async {
        io::validate_file_exists(path)?;

        if config.force_ocr && config.effective_disable_ocr() {
            return Err(crate::XbergError::Validation {
                message: "force_ocr and disable_ocr cannot both be true".to_string(),
                source: None,
            });
        }

        if matches!(
            config.ocr_strategy,
            crate::core::config::OcrStrategy::ScannedPages { .. }
        ) && config.effective_disable_ocr()
        {
            return Err(crate::XbergError::Validation {
                message: "ocr_strategy selects scanned pages for OCR, but disable_ocr is true".to_string(),
                source: None,
            });
        }

        let detected_mime = mime::detect_or_validate(path.to_str(), mime_type)?;

        #[cfg(not(feature = "office"))]
        match detected_mime.as_str() {
            LEGACY_WORD_MIME_TYPE => {
                return Err(XbergError::UnsupportedFormat(
                    "Legacy Word extraction requires the `office` feature".to_string(),
                ));
            }
            LEGACY_POWERPOINT_MIME_TYPE => {
                return Err(XbergError::UnsupportedFormat(
                    "Legacy PowerPoint extraction requires the `office` feature".to_string(),
                ));
            }
            _ => {}
        }

        #[cfg(feature = "office")]
        {
            let _ = LEGACY_WORD_MIME_TYPE;
            let _ = LEGACY_POWERPOINT_MIME_TYPE;
        }

        Box::pin(extract_file_with_extractor(path, &detected_mime, config)).await
    });

    #[cfg(feature = "tokio-runtime")]
    let result = if let Some(secs) = config.extraction_timeout_secs {
        let start = std::time::Instant::now();
        match tokio::time::timeout(std::time::Duration::from_secs(secs), extraction_future).await {
            Ok(inner) => inner,
            Err(_elapsed) => {
                if let Some(ref token) = config.cancel_token {
                    token.cancel();
                }
                Err(crate::XbergError::Timeout {
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    limit_ms: secs * 1000,
                })
            }
        }
    } else {
        extraction_future.await
    };

    #[cfg(not(feature = "tokio-runtime"))]
    let result = {
        // Without a tokio runtime (e.g. the WASM build) there is no timer to
        // enforce a timeout, but the default ExtractionConfig sets
        // extraction_timeout_secs, so erroring here would reject every default
        // call. Ignore the unenforceable limit and run the extraction instead. ~keep
        if config.extraction_timeout_secs.is_some() {
            tracing::debug!(
                "extraction_timeout_secs is ignored without the 'tokio-runtime' feature; running without a timeout"
            );
        }
        extraction_future.await
    };

    #[cfg(feature = "otel")]
    if let Err(ref e) = result {
        crate::telemetry::spans::record_error_on_current_span(e);
    }

    result
}

pub(in crate::core::extractor) async fn extract_file_with_extractor(
    path: &Path,
    mime_type: &str,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    let config = config.normalized();
    let config = config.as_ref();

    if !config.use_cache || config.cache_ttl_secs == Some(0) {
        return extract_file_uncached(path, mime_type, config).await;
    }

    let content_hash = crate::cache::blake3_hash_file(path)?;
    let config_hash = hash_extraction_config(config, mime_type);
    let cache_key = format!("{content_hash}_{config_hash}");

    let namespace = config.cache_namespace.as_deref();

    if let Some(cache) = get_extraction_cache()
        && let Ok(Some(data)) = cache.get(&cache_key, path.to_str(), namespace, config.cache_ttl_secs)
        && let Ok(result) = rmp_serde::from_slice::<ExtractedDocument>(&data)
    {
        tracing::debug!(cache_key = %cache_key, "Extraction cache hit");
        return Ok(result);
    }

    let result = Box::pin(extract_file_uncached(path, mime_type, config)).await?;

    if let Some(cache) = get_extraction_cache()
        && let Ok(data) = rmp_serde::to_vec(&result)
    {
        let _ = cache.set(&cache_key, data, path.to_str(), namespace, config.cache_ttl_secs);
    }

    Ok(result)
}

/// Extract without caching logic.
async fn extract_file_uncached(path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractedDocument> {
    let budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
    crate::core::config::concurrency::init_thread_pools(budget);

    crate::extractors::ensure_initialized()?;

    let extractor = get_extractor(mime_type)?;
    let doc = Box::pin(extractor.extract_path(path, mime_type, config)).await?;
    let result = Box::pin(crate::core::pipeline::run_pipeline(doc, config)).await?;
    Ok(result)
}

/// Hash ExtractionConfig fields that affect extraction output.
///
/// Excludes cache-control fields (use_cache, cache_namespace, cache_ttl_secs)
/// since they don't affect the extraction result. Uses a clone-and-normalize
/// approach to ensure determinism: cache fields are zeroed, then the struct
/// is serialized to canonical JSON via serde_json's sorted-keys representation.
fn hash_extraction_config(config: &ExtractionConfig, mime_type: &str) -> String {
    let mut normalized = config.clone();
    normalized.use_cache = true;
    normalized.cache_namespace = None;
    normalized.cache_ttl_secs = None;

    let mut hasher = blake3::Hasher::new();
    hasher.update(mime_type.as_bytes());
    if let Ok(bytes) = rmp_serde::to_vec(&normalized) {
        hasher.update(&bytes);
    }

    // `#[serde(skip)]` fields are absent from the MessagePack bytes above but DO
    hasher.update(b"\x00source_name\x00");
    if let Some(name) = normalized.source_name.as_deref() {
        hasher.update(name.as_bytes());
    }
    hasher.update(b"\x00tessdata\x00");
    if let Some(ocr) = normalized.ocr.as_ref()
        && let Some(tessdata) = ocr.tessdata_bytes.as_ref()
    {
        let mut keys: Vec<&String> = tessdata.keys().collect();
        keys.sort();
        for key in keys {
            hasher.update(key.as_bytes());
            hasher.update(&(tessdata[key].len() as u64).to_le_bytes());
            hasher.update(&tessdata[key]);
        }
    }

    let hash = hasher.finalize();
    hex::encode(&hash.as_bytes()[..16])
}

/// Get or initialize the global extraction cache.
fn get_extraction_cache() -> Option<&'static crate::cache::GenericCache> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Option<crate::cache::GenericCache>> = OnceLock::new();

    CACHE
        .get_or_init(|| crate::cache::GenericCache::new("extraction".to_string(), None, 30.0, 2000.0, 500.0).ok())
        .as_ref()
}

pub(in crate::core::extractor) async fn extract_bytes_with_extractor(
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    let config = config.normalized();
    let config = config.as_ref();

    let budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
    crate::core::config::concurrency::init_thread_pools(budget);

    crate::extractors::ensure_initialized()?;

    let extractor = get_extractor(mime_type)?;
    let doc = Box::pin(extractor.extract_content(content, mime_type, config)).await?;
    let result = Box::pin(crate::core::pipeline::run_pipeline(doc, config)).await?;
    Ok(result)
}

#[cfg(test)]
mod cache_key_tests {
    use super::hash_extraction_config;
    use crate::core::config::ExtractionConfig;

    #[test]
    fn source_name_changes_the_cache_key() {
        let a = ExtractionConfig {
            source_name: Some("snippet.py".to_string()),
            ..Default::default()
        };
        let b = ExtractionConfig {
            source_name: Some("snippet.rb".to_string()),
            ..Default::default()
        };
        assert_ne!(
            hash_extraction_config(&a, "text/x-source-code"),
            hash_extraction_config(&b, "text/x-source-code"),
            "source_name (serde-skipped) must be part of the cache key"
        );
    }

    #[test]
    #[cfg(feature = "ocr")]
    fn tessdata_bytes_changes_the_cache_key() {
        use crate::core::config::OcrConfig;
        use std::collections::HashMap;

        let mut eng = HashMap::new();
        eng.insert("eng".to_string(), vec![1u8, 2, 3]);
        let mut deu = HashMap::new();
        deu.insert("eng".to_string(), vec![9u8, 9, 9]);

        let a = ExtractionConfig {
            ocr: Some(OcrConfig {
                tessdata_bytes: Some(eng),
                ..OcrConfig::default()
            }),
            ..Default::default()
        };
        let b = ExtractionConfig {
            ocr: Some(OcrConfig {
                tessdata_bytes: Some(deu),
                ..OcrConfig::default()
            }),
            ..Default::default()
        };
        assert_ne!(
            hash_extraction_config(&a, "image/png"),
            hash_extraction_config(&b, "image/png"),
            "tessdata_bytes (serde-skipped) must be part of the cache key"
        );
    }

    #[test]
    fn ocr_strategy_changes_the_cache_key() {
        use crate::core::config::OcrStrategy;

        let auto = ExtractionConfig::default();
        let scanned = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            ..Default::default()
        };
        assert_ne!(
            hash_extraction_config(&auto, "application/pdf"),
            hash_extraction_config(&scanned, "application/pdf"),
            "ocr_strategy selects different pages for OCR and must be part of the cache key"
        );
    }

    #[test]
    fn scanned_pages_min_confidence_changes_the_cache_key() {
        use crate::core::config::OcrStrategy;

        let lenient = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.6 },
            ..Default::default()
        };
        let strict = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.9 },
            ..Default::default()
        };
        assert_ne!(
            hash_extraction_config(&lenient, "application/pdf"),
            hash_extraction_config(&strict, "application/pdf"),
            "min_confidence selects different pages for OCR and must be part of the cache key"
        );
    }
}
