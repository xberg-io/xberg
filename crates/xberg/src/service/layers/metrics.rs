//! Metrics layer for the extraction service.
//!
//! Records service-level counters, histograms, and gauges on every
//! extraction request using the xberg OTel metric instruments.

use crate::types::ExtractionResult;
use crate::{Result, XbergError};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use crate::service::request::{ExtractionRequest, ExtractionSource};
use crate::telemetry::conventions;

// ---------------------------------------------------------------------------
// Layer
// ---------------------------------------------------------------------------

/// A [`tower::Layer`] that records service-level extraction metrics.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct MetricsLayer;

impl MetricsLayer {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService { inner }
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Middleware service that records extraction metrics.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct MetricsService<S> {
    inner: S,
}

impl<S> Service<ExtractionRequest> for MetricsService<S>
where
    S: Service<ExtractionRequest, Response = ExtractionResult, Error = XbergError> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = ExtractionResult;
    type Error = XbergError;
    type Future = Pin<Box<dyn Future<Output = Result<ExtractionResult>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: ExtractionRequest) -> Self::Future {
        let metrics = crate::telemetry::metrics::get_metrics();
        let mime_type = match &req.source {
            ExtractionSource::File { .. } => "unknown".to_owned(),
            ExtractionSource::Bytes { mime_type, .. } => mime_type.clone(),
        };

        metrics.concurrent_extractions.add(1, &[]);

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let result = inner.call(req).await;

            let status = if result.is_ok() { "ok" } else { "error" };
            let attrs = [
                opentelemetry::KeyValue::new(conventions::DOCUMENT_MIME_TYPE, mime_type),
                opentelemetry::KeyValue::new("status", status),
            ];

            metrics.extraction_total.add(1, &attrs);
            metrics.concurrent_extractions.add(-1, &[]);

            result
        })
    }
}
