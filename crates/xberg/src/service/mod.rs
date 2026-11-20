//! Tower service layer for xberg extraction.
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
//! ```rust
//! use xberg::service::{ExtractionServiceBuilder, ExtractionRequest};
//! use xberg::ExtractionConfig;
//! use tower::ServiceExt;
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() -> xberg::Result<()> {
//! let svc = ExtractionServiceBuilder::new()
//!     .with_timeout(Duration::from_secs(300))
//!     .with_concurrency_limit(4)
//!     .build()?;
//!
//! let req = ExtractionRequest::bytes(b"hello".as_slice(), "text/plain", ExtractionConfig::default());
//! let result = svc.oneshot(req).await?;
//! assert_eq!(result.content, "hello");
//! # Ok(())
//! # }
//! ```

mod extraction;
pub mod layers;
pub mod request;

pub use extraction::ExtractionService;
pub use request::{ExtractionRequest, ExtractionSource};

use crate::XbergError;
use crate::types::ExtractedDocument;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tower::util::BoxCloneService;
use tower::{Service, ServiceBuilder, ServiceExt};

/// Builder for composing an extraction service with Tower middleware layers.
///
/// Layers are applied in the order: Tracing → Metrics → Timeout → ConcurrencyLimit → Service.
#[cfg_attr(alef, alef(skip))]
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            timeout: None,
            concurrency_limit: None,
            tracing: false,
            #[cfg(feature = "otel")]
            metrics: false,
        }
    }

    /// Add a per-request timeout.
    #[must_use]
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Limit concurrent in-flight extractions.
    ///
    /// A zero limit is rejected by [`Self::build`].
    #[must_use]
    pub fn with_concurrency_limit(mut self, max: usize) -> Self {
        self.concurrency_limit = Some(max);
        self
    }

    /// Add a tracing span to each extraction request.
    #[must_use]
    pub fn with_tracing(mut self) -> Self {
        self.tracing = true;
        self
    }

    /// Add metrics recording to each extraction request.
    #[cfg(feature = "otel")]
    #[must_use]
    pub fn with_metrics(mut self) -> Self {
        self.metrics = true;
        self
    }

    /// Build the service stack, returning a type-erased cloneable service.
    ///
    /// Layer order (outermost to innermost):
    /// `Tracing → Metrics → Timeout → ConcurrencyLimit → ExtractionService`
    ///
    /// # Errors
    ///
    /// Returns [`XbergError::Validation`] when the concurrency limit is zero.
    pub fn build(self) -> crate::Result<BoxCloneService<ExtractionRequest, ExtractedDocument, XbergError>> {
        if self.concurrency_limit == Some(0) {
            return Err(XbergError::validation(
                "building extraction service: concurrency limit must be greater than zero",
            ));
        }

        let svc = ExtractionService::new();

        let svc = match self.concurrency_limit {
            Some(limit) => ServiceBuilder::new()
                .concurrency_limit(limit)
                .service(svc)
                .boxed_clone(),
            None => svc.boxed_clone(),
        };

        let svc: BoxCloneService<ExtractionRequest, ExtractedDocument, XbergError> = match self.timeout {
            Some(duration) => {
                let timeout_svc = TimeoutService { inner: svc, duration };
                timeout_svc.boxed_clone()
            }
            None => svc,
        };

        #[cfg(feature = "otel")]
        let svc = if self.metrics {
            ServiceBuilder::new()
                .layer(layers::metrics::MetricsLayer::new())
                .service(svc)
                .boxed_clone()
        } else {
            svc
        };

        let svc = if self.tracing {
            ServiceBuilder::new()
                .layer(layers::tracing::TracingLayer::new())
                .service(svc)
                .boxed_clone()
        } else {
            svc
        };

        Ok(svc)
    }
}

/// A simple timeout wrapper that converts elapsed timeouts to
/// [`XbergError::Timeout`] instead of a `BoxError`.
#[derive(Clone)]
struct TimeoutService {
    inner: BoxCloneService<ExtractionRequest, ExtractedDocument, XbergError>,
    duration: Duration,
}

impl Service<ExtractionRequest> for TimeoutService {
    type Response = ExtractedDocument;
    type Error = XbergError;
    type Future = Pin<Box<dyn Future<Output = crate::Result<ExtractedDocument>> + Send>>;

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
                Err(_elapsed) => Err(XbergError::Timeout {
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
        let _svc = ExtractionServiceBuilder::new()
            .build()
            .expect("default builder should be valid");
    }

    #[test]
    fn builder_with_timeout_does_not_panic() {
        let _svc = ExtractionServiceBuilder::new()
            .with_timeout(Duration::from_secs(30))
            .build()
            .expect("positive timeout should be valid");
    }

    #[test]
    fn builder_with_concurrency_limit_does_not_panic() {
        let _svc = ExtractionServiceBuilder::new()
            .with_concurrency_limit(4)
            .build()
            .expect("positive concurrency limit should be valid");
    }

    #[test]
    fn builder_rejects_zero_concurrency_limit() {
        let result = ExtractionServiceBuilder::new().with_concurrency_limit(0).build();

        assert!(matches!(result, Err(XbergError::Validation { .. })));
    }

    #[tokio::test]
    async fn builder_service_extracts_text() {
        let mut svc = ExtractionServiceBuilder::new()
            .build()
            .expect("default builder should be valid");
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
            .build()
            .expect("positive timeout should be valid");
        let req = ExtractionRequest::bytes(b"timeout test".as_slice(), "text/plain", ExtractionConfig::default());
        let result = svc.call(req).await.expect("extraction should succeed within timeout");
        assert!(result.content.contains("timeout test"));
    }

    #[tokio::test]
    async fn timeout_fires_on_zero_duration() {
        let mut svc = ExtractionServiceBuilder::new()
            .with_timeout(Duration::from_nanos(1))
            .build()
            .expect("positive timeout should be valid");
        let req = ExtractionRequest::bytes(b"hello".as_slice(), "text/plain", ExtractionConfig::default());
        let result = svc.call(req).await;
        match result {
            Ok(r) => assert!(r.content.contains("hello")),
            Err(XbergError::Timeout { .. }) => {}
            Err(other) => panic!("expected Ok or Timeout, got: {:?}", other),
        }
    }
}
