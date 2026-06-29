//! Core extraction service implementing [`tower::Service`].

use crate::core::config::ExtractionConfig;
use crate::core::extractor::{extract_bytes, extract_file};
use crate::types::ExtractedDocument;
use crate::{Result, XbergError};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

use super::request::{ExtractionRequest, ExtractionSource};

/// A [`tower::Service`] that dispatches extraction requests to the xberg
/// core library.
///
/// This service is cheap to clone and can be shared across handlers.
/// Concurrency and timeouts are managed by composing Tower layers on top
/// (see [`super::ExtractionServiceBuilder`]).
///
/// # Example
///
/// ```rust,ignore
/// use xberg::service::{ExtractionService, ExtractionRequest};
/// use xberg::ExtractionConfig;
/// use tower::Service;
///
/// let mut svc = ExtractionService::new();
/// let req = ExtractionRequest::bytes(b"hello".as_slice(), "text/plain", ExtractionConfig::default());
/// let result = svc.call(req).await?;
/// ```
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct ExtractionService {
    _private: (),
}

impl ExtractionService {
    /// Create a new extraction service.
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for ExtractionService {
    fn default() -> Self {
        Self::new()
    }
}

impl Service<ExtractionRequest> for ExtractionService {
    type Response = ExtractedDocument;
    type Error = XbergError;
    type Future = Pin<Box<dyn Future<Output = Result<ExtractedDocument>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ExtractionRequest) -> Self::Future {
        let config = resolve_config(req.config, req.file_overrides);

        match req.source {
            ExtractionSource::File { path, mime_hint } => {
                Box::pin(async move { extract_file(&path, mime_hint.as_deref(), &config).await })
            }
            ExtractionSource::Bytes { data, mime_type } => {
                Box::pin(async move { extract_bytes(&data, &mime_type, &config).await })
            }
        }
    }
}

/// Merge optional per-file overrides into the base config.
fn resolve_config(
    base: ExtractionConfig,
    overrides: Option<crate::core::config::FileExtractionConfig>,
) -> ExtractionConfig {
    match overrides {
        Some(file_overrides) => base.with_file_overrides(&file_overrides),
        None => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::task::Poll;
    use tower::Service;

    #[test]
    fn poll_ready_returns_ready() {
        let mut svc = ExtractionService::new();
        let waker = std::task::Waker::noop();
        let mut cx = Context::from_waker(waker);
        assert!(matches!(svc.poll_ready(&mut cx), Poll::Ready(Ok(()))));
    }

    #[tokio::test]
    async fn extract_plain_text_bytes() {
        let mut svc = ExtractionService::new();
        let req = ExtractionRequest::bytes(b"hello".as_slice(), "text/plain", ExtractionConfig::default());
        let result = svc.call(req).await.expect("extraction should succeed");
        assert!(result.content.contains("hello"));
    }

    #[tokio::test]
    async fn extract_from_tempfile() {
        let mut svc = ExtractionService::new();
        // Give the tempfile a `.txt` suffix: plain ASCII has no magic bytes, so
        // MIME detection relies on the extension for extension-less paths.
        let mut tmp = tempfile::Builder::new()
            .suffix(".txt")
            .tempfile()
            .expect("failed to create tempfile");
        tmp.write_all(b"tempfile content").expect("failed to write");
        tmp.flush().expect("failed to flush");

        let req = ExtractionRequest::file(tmp.path(), ExtractionConfig::default());
        let result = svc.call(req).await.expect("extraction should succeed");
        assert!(result.content.contains("tempfile content"));
    }
}
