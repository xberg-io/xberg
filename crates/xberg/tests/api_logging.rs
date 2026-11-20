//! Integration tests verifying that the API router's TraceLayer emits
//! per-request events at DEBUG (not INFO), so they are suppressed under the
//! default `tower_http=info` filter while still surfacing under
//! `RUST_LOG=tower_http=debug`.

#![cfg(feature = "api")]

use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt as _};

use xberg::ExtractionConfig;

/// A tracing `Layer` that records the target + level of every emitted event.
#[derive(Clone, Default)]
struct EventCapture {
    events: Arc<Mutex<Vec<CapturedEvent>>>,
}

#[derive(Debug, Clone)]
struct CapturedEvent {
    target: String,
}

impl<S> tracing_subscriber::Layer<S> for EventCapture
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let meta = event.metadata();
        self.events.lock().unwrap().push(CapturedEvent {
            target: meta.target().to_owned(),
        });
    }
}

fn captured_events(capture: &EventCapture) -> Vec<CapturedEvent> {
    capture.events.lock().unwrap().clone()
}

/// Drive a GET /health request through the router.
async fn get_health(router: axum::Router) -> StatusCode {
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .expect("Failed to build /health request"),
        )
        .await
        .expect("Failed to send /health request");
    response.status()
}

/// At default subscriber level (INFO), the TraceLayer must not emit any
/// per-request or per-response events from `tower_http::trace`.
///
/// The configured TraceLayer sets per-request events to DEBUG; the subscriber
/// here uses `tower_http=info` so those DEBUG events are filtered out.
///
/// Uses a plain `#[test]` (not `#[tokio::test]`) so we can wrap the runtime
/// construction inside `tracing::subscriber::with_default` without nesting
/// a second runtime on top of the tokio test runtime.
#[test]
fn tower_http_trace_events_suppressed_at_info() {
    let capture = EventCapture::default();
    let capture_clone = capture.clone();

    let filter = EnvFilter::new("info,tower_http=info");
    let subscriber = tracing_subscriber::registry().with(filter).with(capture_clone);

    let status = tracing::subscriber::with_default(subscriber, || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let router = create_router(ExtractionConfig::default());
                get_health(router).await
            })
    });

    assert_eq!(status, StatusCode::OK, "/health must return 200");

    let events = captured_events(&capture);
    let tower_trace_events: Vec<_> = events
        .iter()
        .filter(|e| e.target.starts_with("tower_http::trace"))
        .collect();
    assert!(
        tower_trace_events.is_empty(),
        "expected zero tower_http::trace events at info filter; got: {tower_trace_events:?}"
    );
}

/// At DEBUG level for tower_http, per-request events from `tower_http::trace`
/// DO appear — confirming we suppressed by filter config, not by removing the layer.
///
/// Uses a plain `#[test]` for the same reason as `tower_http_trace_events_suppressed_at_info`.
#[test]
fn tower_http_trace_events_visible_at_debug() {
    let capture = EventCapture::default();
    let capture_clone = capture.clone();

    // Explicitly enable tower_http at DEBUG.
    let filter = EnvFilter::new("info,tower_http=debug");
    let subscriber = tracing_subscriber::registry().with(filter).with(capture_clone);

    let status = tracing::subscriber::with_default(subscriber, || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let router = create_router(ExtractionConfig::default());
                get_health(router).await
            })
    });

    assert_eq!(status, StatusCode::OK, "/health must return 200");

    let events = captured_events(&capture);
    let tower_trace_events: Vec<_> = events.iter().filter(|e| e.target.starts_with("tower_http")).collect();
    assert!(
        !tower_trace_events.is_empty(),
        "expected at least one tower_http event at debug filter; got none (total events: {:?})",
        events
    );
}

/// The /health route must respond correctly regardless of logging configuration.
///
/// Uses `#[tokio::test]` since it does not install a custom subscriber.
#[tokio::test]
async fn health_route_works_without_subscriber() {
    let router = create_router(ExtractionConfig::default());
    let status = get_health(router).await;
    assert_eq!(status, StatusCode::OK);
}

/// Router with authentication pinned off, so the test does not depend on `XBERG_API_TOKEN`.
fn create_router(config: ExtractionConfig) -> axum::Router {
    xberg::api::create_router_with_auth(
        config,
        xberg::api::ApiSizeLimits::default(),
        xberg::api::ApiAuth::new(None, false),
    )
}
