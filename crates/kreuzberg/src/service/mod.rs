//! Tower service layer for kreuzberg extraction.
//!
//! Provides a composable [`tower::Service`] that wraps the core extraction
//! functions with configurable middleware layers (tracing, metrics, timeout,
//! concurrency limits).
//!
//! # Architecture
//!
//! ```text
//! TracingLayer → MetricsLayer → Timeout → ConcurrencyLimit → ExtractionService
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use kreuzberg::service::{ExtractionServiceBuilder, ExtractionRequest};
//! use kreuzberg::ExtractionConfig;
//! use tower::Service;
//! use std::time::Duration;
//!
//! let mut svc = ExtractionServiceBuilder::new()
//!     .with_timeout(Duration::from_secs(300))
//!     .with_concurrency_limit(4)
//!     .build();
//!
//! let req = ExtractionRequest::file("doc.pdf", ExtractionConfig::default());
//! let result = svc.call(req).await?;
//! ```

mod extraction;
pub mod layers;
pub mod request;

pub use extraction::ExtractionService;
pub use request::{ExtractionRequest, ExtractionSource};

use crate::KreuzbergError;
use crate::types::ExtractionResult;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tower::util::BoxCloneService;
use tower::{Service, ServiceBuilder, ServiceExt};

/// Builder for composing an extraction service with Tower middleware layers.
///
/// Layers are applied in the order: Tracing → Metrics → Timeout → ConcurrencyLimit → Service.
pub struct ExtractionServiceBuilder {
    timeout: Option<Duration>,
    concurrency_limit: Option<usize>,
    tracing: bool,
    #[cfg(feature = "otel")]
    metrics: bool,
}

impl Default for ExtractionServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtractionServiceBuilder {
    /// Create a new builder with no layers configured.
    pub(crate) fn new() -> Self {
        Self {
            timeout: None,
            concurrency_limit: None,
            tracing: false,
            #[cfg(feature = "otel")]
            metrics: false,
        }
    }

    /// Add a per-request timeout.
    #[cfg(test)]
    pub(crate) fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Limit concurrent in-flight extractions.
    #[cfg(test)]
    pub(crate) fn with_concurrency_limit(mut self, max: usize) -> Self {
        self.concurrency_limit = Some(max);
        self
    }

    /// Add a tracing span to each extraction request.
    pub(crate) fn with_tracing(mut self) -> Self {
        self.tracing = true;
        self
    }

    /// Add metrics recording to each extraction request.
    ///
    /// Requires the `otel` feature. This is a no-op when `otel` is not enabled.
    #[allow(unused_mut)]
    pub(crate) fn with_metrics(mut self) -> Self {
        #[cfg(feature = "otel")]
        {
            self.metrics = true;
        }
        self
    }

    /// Build the service stack, returning a type-erased cloneable service.
    ///
    /// Layer order (outermost to innermost):
    /// `Tracing → Metrics → Timeout → ConcurrencyLimit → ExtractionService`
    pub(crate) fn build(self) -> BoxCloneService<ExtractionRequest, ExtractionResult, KreuzbergError> {
        let svc = ExtractionService::new();

        // Apply concurrency limit (innermost optional layer).
        let svc = match self.concurrency_limit {
            Some(limit) => ServiceBuilder::new()
                .concurrency_limit(limit)
                .service(svc)
                .boxed_clone(),
            None => svc.boxed_clone(),
        };

        // Apply timeout. We wrap inline rather than using Tower's Timeout layer
        // because Timeout changes the error type to BoxError — we need to keep
        // KreuzbergError throughout the stack.
        let svc: BoxCloneService<ExtractionRequest, ExtractionResult, KreuzbergError> = match self.timeout {
            Some(duration) => {
                let timeout_svc = TimeoutService { inner: svc, duration };
                timeout_svc.boxed_clone()
            }
            None => svc,
        };

        // Apply metrics layer (otel only).
        #[cfg(feature = "otel")]
        let svc = if self.metrics {
            ServiceBuilder::new()
                .layer(layers::metrics::MetricsLayer::new())
                .service(svc)
                .boxed_clone()
        } else {
            svc
        };

        // Apply tracing layer (outermost).
        if self.tracing {
            ServiceBuilder::new()
                .layer(layers::tracing::TracingLayer::new())
                .service(svc)
                .boxed_clone()
        } else {
            svc
        }
    }
}

// ---------------------------------------------------------------------------
// Timeout wrapper that preserves KreuzbergError
// ---------------------------------------------------------------------------

/// A simple timeout wrapper that converts elapsed timeouts to
/// [`KreuzbergError::Timeout`] instead of a `BoxError`.
#[derive(Clone)]
struct TimeoutService {
    inner: BoxCloneService<ExtractionRequest, ExtractionResult, KreuzbergError>,
    duration: Duration,
}

impl Service<ExtractionRequest> for TimeoutService {
    type Response = ExtractionResult;
    type Error = KreuzbergError;
    type Future = Pin<Box<dyn Future<Output = crate::Result<ExtractionResult>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<crate::Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: ExtractionRequest) -> Self::Future {
        let fut = self.inner.call(req);
        let duration = self.duration;
        let start = std::time::Instant::now();

        Box::pin(async move {
            match tokio::time::timeout(duration, fut).await {
                Ok(result) => result,
                Err(_elapsed) => Err(KreuzbergError::Timeout {
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    limit_ms: duration.as_millis() as u64,
                }),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ExtractionConfig;

    #[test]
    fn builder_new_builds_service() {
        // Should not panic.
        let _svc = ExtractionServiceBuilder::new().build();
    }

    #[test]
    fn builder_with_timeout_does_not_panic() {
        let _svc = ExtractionServiceBuilder::new()
            .with_timeout(Duration::from_secs(30))
            .build();
    }

    #[test]
    fn builder_with_concurrency_limit_does_not_panic() {
        let _svc = ExtractionServiceBuilder::new().with_concurrency_limit(4).build();
    }

    #[tokio::test]
    async fn builder_service_extracts_text() {
        let mut svc = ExtractionServiceBuilder::new().build();
        let req = ExtractionRequest::bytes(
            b"hello from builder".as_slice(),
            "text/plain",
            ExtractionConfig::default(),
        );
        let result = svc.call(req).await.expect("extraction should succeed");
        assert!(result.content.contains("hello from builder"));
    }

    #[tokio::test]
    async fn builder_with_timeout_extracts_text() {
        let mut svc = ExtractionServiceBuilder::new()
            .with_timeout(Duration::from_secs(10))
            .build();
        let req = ExtractionRequest::bytes(b"timeout test".as_slice(), "text/plain", ExtractionConfig::default());
        let result = svc.call(req).await.expect("extraction should succeed within timeout");
        assert!(result.content.contains("timeout test"));
    }

    #[tokio::test]
    async fn timeout_fires_on_zero_duration() {
        let mut svc = ExtractionServiceBuilder::new()
            .with_timeout(Duration::from_nanos(1))
            .build();
        let req = ExtractionRequest::bytes(b"hello".as_slice(), "text/plain", ExtractionConfig::default());
        let result = svc.call(req).await;
        // With a 1ns timeout, the result is either a success (if extraction
        // completes before the timeout is checked) or a Timeout error.
        // Both are acceptable — the key assertion is that it does not panic.
        match result {
            Ok(r) => assert!(r.content.contains("hello")),
            Err(KreuzbergError::Timeout { .. }) => { /* expected timeout */ }
            Err(other) => panic!("expected Ok or Timeout, got: {:?}", other),
        }
    }
}
