//! Tracing layer for the extraction service.
//!
//! Adds a semantic span to every extraction request using xberg conventions.

use crate::telemetry::conventions;
use crate::types::ExtractionResult;
use crate::{Result, XbergError};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::Instrument;

use crate::service::request::{ExtractionRequest, ExtractionSource};

// ---------------------------------------------------------------------------
// Layer
// ---------------------------------------------------------------------------

/// A [`tower::Layer`] that wraps each extraction in a semantic tracing span.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct TracingLayer;

impl TracingLayer {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for TracingLayer {
    type Service = TracingService<S>;

    #[cfg_attr(alef, alef(skip))]
    fn layer(&self, inner: S) -> Self::Service {
        TracingService { inner }
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Middleware service that creates a span per extraction request.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct TracingService<S> {
    inner: S,
}

impl<S> Service<ExtractionRequest> for TracingService<S>
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
        let span = make_span(&req);
        let mut inner = self.inner.clone();

        Box::pin(
            async move {
                let result = inner.call(req).await;

                #[cfg(feature = "otel")]
                match &result {
                    Ok(_) => crate::telemetry::spans::record_success_on_current_span(),
                    Err(e) => crate::telemetry::spans::record_error_on_current_span(e),
                }

                result
            }
            .instrument(span),
        )
    }
}

fn make_span(req: &ExtractionRequest) -> tracing::Span {
    match &req.source {
        ExtractionSource::File { path, .. } => {
            let filename = conventions::sanitize_filename(path);
            tracing::info_span!(
                "xberg.service",
                { conventions::OPERATION } = conventions::operations::EXTRACT_FILE,
                { conventions::DOCUMENT_FILENAME } = filename,
                { conventions::OTEL_STATUS_CODE } = tracing::field::Empty,
                { conventions::ERROR_TYPE } = tracing::field::Empty,
                { conventions::ERROR_MESSAGE } = tracing::field::Empty,
            )
        }
        ExtractionSource::Bytes { mime_type, data } => tracing::info_span!(
            "xberg.service",
            { conventions::OPERATION } = conventions::operations::EXTRACT_BYTES,
            { conventions::DOCUMENT_MIME_TYPE } = %mime_type,
            { conventions::DOCUMENT_SIZE_BYTES } = data.len(),
            { conventions::OTEL_STATUS_CODE } = tracing::field::Empty,
            { conventions::ERROR_TYPE } = tracing::field::Empty,
            { conventions::ERROR_MESSAGE } = tracing::field::Empty,
        ),
    }
}
