//! API router setup and configuration.

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
};
use tower::limit::GlobalConcurrencyLimitLayer;
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveHeadersLayer,
    trace::{DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};

use crate::{ExtractionConfig, core::ServerConfig, service::ExtractionServiceBuilder};

#[cfg(feature = "prometheus")]
use super::handlers::metrics_handler;
use super::{
    auth::ApiAuth,
    handlers::{
        cache_clear_handler, cache_manifest_handler, cache_stats_handler, cache_warm_handler, cancel_job_handler,
        detect_handler, extract_async_handler, extract_handler, formats_handler, health_handler, info_handler,
        job_status_handler, not_found_handler, version_handler,
    },
    openweb::{openweb_docling_handler, openweb_external_handler},
    types::{ApiSizeLimits, ApiState},
};

/// Environment variable controlling the server's global concurrency limit.
/// Set to `0` to disable the limit (unbounded concurrent requests).
const MAX_CONCURRENT_REQUESTS_ENV: &str = "XBERG_MAX_CONCURRENT_REQUESTS";

/// Resolve the maximum number of requests processed concurrently by the server.
///
/// Without a bound, every in-flight request buffers its body and extraction
/// working set in memory, so a burst of large requests can exhaust RAM and OOM
/// the process — especially in memory-limited containers. The default caps
/// concurrency at twice the CPU count (clamped to `[4, 32]`), applying
/// backpressure instead of unbounded growth. `XBERG_MAX_CONCURRENT_REQUESTS`
/// overrides it; `0` disables the limit for deployments that bound concurrency
/// upstream (e.g. a reverse proxy or queue).
fn resolve_max_concurrent_requests() -> usize {
    const MIN_DEFAULT: usize = 4;
    const MAX_DEFAULT: usize = 32;

    match std::env::var(MAX_CONCURRENT_REQUESTS_ENV) {
        Ok(value) => value.trim().parse::<usize>().unwrap_or_else(|_| {
            tracing::warn!("{MAX_CONCURRENT_REQUESTS_ENV}={value:?} is not a valid usize; using the computed default");
            (num_cpus::get() * 2).clamp(MIN_DEFAULT, MAX_DEFAULT)
        }),
        Err(_) => (num_cpus::get() * 2).clamp(MIN_DEFAULT, MAX_DEFAULT),
    }
}

/// Create the API router with all routes configured.
///
/// This is public to allow users to embed the router in their own applications.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
///
/// # Examples
///
/// ```no_run
/// use xberg::{ExtractionConfig, api::create_router};
///
/// # #[tokio::main]
/// # async fn main() {
/// // Create router with default config and size limits
/// let config = ExtractionConfig::default();
/// let router = create_router(config);
/// # }
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn create_router(config: ExtractionConfig) -> Router {
    create_router_with_limits(config, ApiSizeLimits::default())
}

/// Create the API router with custom size limits.
///
/// This allows fine-grained control over request body and multipart field size limits.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
/// * `limits` - Size limits for request bodies and multipart uploads.
///
/// # Examples
///
/// ```ignore
/// use xberg::{ExtractionConfig, api::{create_router_with_limits, ApiSizeLimits}};
///
/// # #[tokio::main]
/// # async fn main() {
/// // Create router with 50 MB limits
/// let config = ExtractionConfig::default();
/// let limits = ApiSizeLimits::from_mb(50, 50);
/// let router = create_router_with_limits(config, limits);
/// # }
/// ```
///
/// ```ignore
/// use xberg::{ExtractionConfig, api::{create_router_with_limits, ApiSizeLimits}};
/// use tower_http::limit::RequestBodyLimitLayer;
///
/// # #[tokio::main]
/// # async fn main() {
/// // Custom limits for very large documents (500 MB)
/// let config = ExtractionConfig::default();
/// let limits = ApiSizeLimits::from_mb(500, 500);
/// let router = create_router_with_limits(config, limits);
/// # }
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn create_router_with_limits(config: ExtractionConfig, limits: ApiSizeLimits) -> Router {
    create_router_with_limits_and_server_config(config, limits, ServerConfig::default())
}

/// Create the API router with explicit size limits and authentication settings.
///
/// Unlike [`create_router_with_limits`], this never reads `XBERG_API_TOKEN` from the process
/// environment, so embedders and tests get the same behaviour regardless of it.
#[cfg_attr(alef, alef(skip))]
pub fn create_router_with_auth(config: ExtractionConfig, limits: ApiSizeLimits, auth: ApiAuth) -> Router {
    build_router(
        config,
        limits,
        ServerConfig::default(),
        resolve_max_concurrent_requests(),
        auth,
    )
}

/// Build the [`ApiState`] shared by every route, including a fresh extraction service and
/// (behind `feature = "api"`) a fresh job store.
///
/// Extracted from [`create_router_with_limits_and_server_config`] so the wiring from
/// `server_config` into `ApiState` — in particular `job_timeout_secs` — is unit-testable
/// without building a full router.
///
/// Async jobs extract under their own limit of `max_concurrent_requests` slots (`0` leaves it
/// unbounded, like the request limit). It is separate from the request limiter so a queued job
/// never blocks a submission or a synchronous request.
fn build_api_state(config: ExtractionConfig, server_config: &ServerConfig, max_concurrent_requests: usize) -> ApiState {
    // Fallback for embedders that only ever use this built-in router: install the
    // Prometheus-backed meter provider before the extraction service (and therefore the
    // `telemetry::metrics::get_metrics()` `OnceLock`) is built below. Callers who construct
    // their own extraction pipeline outside this router must call
    // `xberg::telemetry::init_prometheus()` themselves, before their first extraction —
    // see that function's doc comment. This call is idempotent, so calling it again here
    // after an embedder's own explicit call is harmless. ~keep
    #[cfg(feature = "prometheus")]
    let prometheus_registry = crate::telemetry::init_prometheus();

    let extraction_service_builder = ExtractionServiceBuilder::new().with_tracing();
    #[cfg(feature = "otel")]
    let extraction_service_builder = extraction_service_builder.with_metrics();
    let extraction_service = extraction_service_builder
        .build()
        .expect("the built-in extraction service uses a valid concurrency limit");

    ApiState {
        default_config: Arc::new(config),
        extraction_service: Arc::new(std::sync::Mutex::new(extraction_service)),
        #[cfg(feature = "api")]
        job_store: Arc::new(super::jobs::JobStore::with_execution_slots(
            (max_concurrent_requests > 0).then(|| Arc::new(tokio::sync::Semaphore::new(max_concurrent_requests))),
        )),
        #[cfg(feature = "api")]
        job_timeout_secs: server_config.job_timeout_secs,
        #[cfg(feature = "prometheus")]
        prometheus_registry,
    }
}

/// Build the CORS layer for `server_config`. Cross-origin access is denied unless
/// `cors_origins` lists origins explicitly; the entry `*` opts in to every origin. ~keep
fn build_cors_layer(server_config: &ServerConfig) -> CorsLayer {
    if server_config.cors_allows_all() {
        tracing::warn!(
            "CORS configured to allow all origins ('*'). Any website can call this API from a \
             browser; without XBERG_API_TOKEN that is a CSRF risk. Prefer listing origins in \
             XBERG_CORS_ORIGINS (e.g., 'https://app.example.com,https://api.example.com')"
        );
        return CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    }

    let origins: Vec<_> = server_config
        .cors_origins
        .iter()
        .filter_map(|s| s.trim().parse::<axum::http::HeaderValue>().ok())
        .collect();

    if origins.is_empty() {
        tracing::info!("CORS denied for all cross-origin requests (set XBERG_CORS_ORIGINS to allow origins)");
        return CorsLayer::new();
    }

    tracing::info!("CORS configured with {} explicit allowed origin(s)", origins.len());
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers(Any)
}

/// Cap how many extraction requests process concurrently so a burst of large
/// uploads applies backpressure instead of exhausting memory (#1368). Applied to the
/// extraction routes only: `/health`, `/info` and `/version` must keep answering while
/// every slot is busy, or orchestrator probes fail on a saturated but healthy pod. ~keep
fn apply_concurrency_limit(router: Router<ApiState>, max_concurrent_requests: usize) -> Router<ApiState> {
    if max_concurrent_requests > 0 {
        tracing::info!(
            max_concurrent_requests,
            "API global concurrency limit active (set {MAX_CONCURRENT_REQUESTS_ENV}=0 to disable)"
        );
        router.layer(GlobalConcurrencyLimitLayer::new(max_concurrent_requests))
    } else {
        tracing::warn!(
            "API global concurrency limit DISABLED ({MAX_CONCURRENT_REQUESTS_ENV}=0); \
             concurrent requests are unbounded and may exhaust memory under load"
        );
        router
    }
}

/// Create the API router with custom size limits and server configuration.
///
/// This function provides full control over request limits, CORS, and server settings via ServerConfig.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
/// * `limits` - Size limits for request bodies and multipart uploads.
/// * `server_config` - Server configuration including host, port, and CORS settings.
///
/// # Examples
///
/// Not run as a doctest: `pub(crate)`, so it is unreachable from a downstream crate.
/// [`create_router_with_limits`] is the public entry point with the same shape
/// minus the server config.
///
/// ```ignore
/// use xberg::{ExtractionConfig, api::{create_router_with_limits_and_server_config, ApiSizeLimits}, core::ServerConfig};
///
/// # #[tokio::main]
/// # async fn main() -> xberg::Result<()> {
/// let extraction_config = ExtractionConfig::default();
/// let mut server_config = ServerConfig::default();
/// server_config.cors_origins = vec!["https://example.com".to_string()];
/// let router = create_router_with_limits_and_server_config(
///     extraction_config,
///     ApiSizeLimits::default(),
///     server_config
/// );
/// # Ok(())
/// # }
/// ```
pub(crate) fn create_router_with_limits_and_server_config(
    config: ExtractionConfig,
    limits: ApiSizeLimits,
    server_config: ServerConfig,
) -> Router {
    build_router(
        config,
        limits,
        server_config,
        resolve_max_concurrent_requests(),
        ApiAuth::from_env(),
    )
}

fn build_router(
    config: ExtractionConfig,
    limits: ApiSizeLimits,
    server_config: ServerConfig,
    max_concurrent_requests: usize,
    auth: ApiAuth,
) -> Router {
    let state = build_api_state(config, &server_config, max_concurrent_requests);

    let cors_layer = build_cors_layer(&server_config);

    let limited_routes = apply_concurrency_limit(
        Router::new()
            .route("/extract", post(extract_handler))
            .route("/extract-async", post(extract_async_handler))
            .route("/detect", post(detect_handler))
            .route("/process", put(openweb_external_handler))
            .route("/v1/convert/file", post(openweb_docling_handler))
            .route("/cache/clear", delete(cache_clear_handler))
            .route("/cache/warm", post(cache_warm_handler)),
        max_concurrent_requests,
    );

    let mut protected = Router::new()
        .merge(limited_routes)
        .route("/jobs/{job_id}", get(job_status_handler).delete(cancel_job_handler))
        .route("/formats", get(formats_handler))
        .route("/info", get(info_handler))
        .route("/version", get(version_handler))
        .route("/cache/stats", get(cache_stats_handler))
        .route("/cache/manifest", get(cache_manifest_handler));

    #[cfg(feature = "api")]
    {
        protected = protected.route("/openapi.json", get(openapi_schema_handler));
    }

    #[cfg(feature = "prometheus")]
    {
        protected = protected.route("/metrics", get(metrics_handler));
    }

    // The auth layer is added after the concurrency limit so it runs first: a rejected
    // request never occupies an extraction slot. `/health` stays outside it for probes.
    protected = auth.protect(protected);

    let router = Router::new()
        .route("/health", get(health_handler))
        .merge(protected)
        .fallback(not_found_handler)
        .layer(DefaultBodyLimit::max(limits.max_request_body_bytes))
        .layer(RequestBodyLimitLayer::new(limits.max_request_body_bytes))
        .layer(cors_layer)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CompressionLayer::new())
        .layer(CatchPanicLayer::new())
        .layer(SetSensitiveHeadersLayer::new([axum::http::header::AUTHORIZATION]))
        // Per-request and per-response events are demoted to DEBUG so the default
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::DEBUG))
                .on_request(DefaultOnRequest::new().level(tracing::Level::DEBUG))
                .on_response(DefaultOnResponse::new().level(tracing::Level::DEBUG))
                .on_failure(DefaultOnFailure::new().level(tracing::Level::WARN)),
        );

    router.with_state(state)
}

/// OpenAPI schema handler.
///
/// Returns the OpenAPI 3.1 JSON schema for all documented endpoints.
#[cfg(feature = "api")]
async fn openapi_schema_handler() -> axum::Json<serde_json::Value> {
    use crate::api::openapi::openapi_json;

    let schema_str = openapi_json();
    let schema: serde_json::Value = serde_json::from_str(&schema_str)
        .unwrap_or_else(|_| serde_json::json!({"error": "Failed to generate OpenAPI schema"}));

    axum::Json(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probes_answer_while_every_extraction_slot_is_busy() {
        use axum::{body::Body, http::Request};
        use std::{convert::Infallible, time::Duration};
        use tower::ServiceExt;

        let router = build_router(
            ExtractionConfig::default(),
            ApiSizeLimits::default(),
            ServerConfig::default(),
            1,
            ApiAuth::default(),
        );

        let stalled_body = Body::from_stream(futures::stream::pending::<Result<Vec<u8>, Infallible>>());
        let holder = router.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/extract")
                .header("content-type", "multipart/form-data; boundary=x")
                .body(stalled_body)
                .unwrap(),
        );
        let holder = tokio::spawn(holder);
        tokio::time::sleep(Duration::from_millis(100)).await;

        for uri in ["/health", "/info", "/version"] {
            let response = tokio::time::timeout(
                Duration::from_secs(2),
                router
                    .clone()
                    .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()),
            )
            .await
            .unwrap_or_else(|_| panic!("{uri} queued behind the concurrency limit"))
            .unwrap();
            assert!(response.status().is_success(), "{uri}: {}", response.status());
        }

        let queued = tokio::time::timeout(
            Duration::from_millis(300),
            router.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/detect")
                    .body(Body::empty())
                    .unwrap(),
            ),
        )
        .await;
        assert!(queued.is_err(), "extraction routes must still queue behind the limit");
        holder.abort();
    }

    mod access_control {
        use super::*;
        use axum::{
            body::Body,
            http::{Request, StatusCode, header},
        };
        use std::{convert::Infallible, time::Duration};
        use tower::ServiceExt;

        const TOKEN: &str = "s3cret-token";

        fn router(cors_origins: &[&str], token: Option<&str>) -> Router {
            build_router(
                ExtractionConfig::default(),
                ApiSizeLimits::default(),
                ServerConfig {
                    cors_origins: cors_origins.iter().map(ToString::to_string).collect(),
                    ..ServerConfig::default()
                },
                1,
                ApiAuth::new(token, false),
            )
        }

        async fn status(router: &Router, method: &str, uri: &str, authorization: Option<&str>) -> StatusCode {
            let mut request = Request::builder().method(method).uri(uri);
            if let Some(value) = authorization {
                request = request.header(header::AUTHORIZATION, value);
            }
            router
                .clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status()
        }

        #[tokio::test]
        async fn rejects_missing_and_wrong_tokens_with_a_challenge() {
            let router = router(&[], Some(TOKEN));
            for authorization in [
                None,
                Some("Bearer wrong"),
                Some("Bearer s3cret-toke"),
                Some("Bearer s3cret-token-extra"),
                Some("Basic s3cret-token"),
                Some(TOKEN),
            ] {
                assert_eq!(
                    status(&router, "GET", "/info", authorization).await,
                    StatusCode::UNAUTHORIZED,
                    "{authorization:?}"
                );
            }

            let response = router
                .clone()
                .oneshot(Request::builder().uri("/info").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
        }

        #[tokio::test]
        async fn accepts_the_right_token_case_insensitive_scheme() {
            let router = router(&[], Some(TOKEN));
            for authorization in ["Bearer s3cret-token", "bearer s3cret-token"] {
                assert_eq!(
                    status(&router, "GET", "/info", Some(authorization)).await,
                    StatusCode::OK
                );
            }
        }

        #[tokio::test]
        async fn health_stays_open_when_a_token_is_set() {
            let router = router(&[], Some(TOKEN));
            assert_eq!(status(&router, "GET", "/health", None).await, StatusCode::OK);
        }

        #[tokio::test]
        async fn without_a_token_every_route_is_open() {
            let router = router(&[], None);
            assert_eq!(status(&router, "GET", "/info", None).await, StatusCode::OK);
            assert_eq!(status(&router, "GET", "/health", None).await, StatusCode::OK);
        }

        #[tokio::test]
        async fn cache_routes_require_the_token() {
            let router = router(&[], Some(TOKEN));
            for (method, uri) in [
                ("DELETE", "/cache/clear"),
                ("POST", "/cache/warm"),
                ("GET", "/cache/stats"),
                ("GET", "/cache/manifest"),
            ] {
                assert_eq!(
                    status(&router, method, uri, None).await,
                    StatusCode::UNAUTHORIZED,
                    "{method} {uri}"
                );
                assert_eq!(
                    status(&router, method, uri, Some("Bearer wrong")).await,
                    StatusCode::UNAUTHORIZED,
                    "{method} {uri}"
                );
            }
            assert_eq!(
                status(&router, "GET", "/cache/stats", Some("Bearer s3cret-token")).await,
                StatusCode::OK
            );
        }

        #[tokio::test]
        async fn cache_mutations_queue_behind_the_concurrency_limit() {
            let router = router(&[], None);
            let stalled_body = Body::from_stream(futures::stream::pending::<Result<Vec<u8>, Infallible>>());
            let holder = tokio::spawn(
                router.clone().oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/extract")
                        .header("content-type", "multipart/form-data; boundary=x")
                        .body(stalled_body)
                        .unwrap(),
                ),
            );
            tokio::time::sleep(Duration::from_millis(100)).await;

            for (method, uri) in [("DELETE", "/cache/clear"), ("POST", "/cache/warm")] {
                let queued = tokio::time::timeout(
                    Duration::from_millis(300),
                    router
                        .clone()
                        .oneshot(Request::builder().method(method).uri(uri).body(Body::empty()).unwrap()),
                )
                .await;
                assert!(queued.is_err(), "{method} {uri} must wait for a free slot");
            }
            holder.abort();
        }

        #[tokio::test]
        async fn extract_openapi_and_metrics_require_the_token() {
            let router = router(&[], Some(TOKEN));
            let mut routes = vec![("POST", "/extract"), ("GET", "/openapi.json")];
            if cfg!(feature = "prometheus") {
                routes.push(("GET", "/metrics"));
            }
            for (method, uri) in routes {
                assert_eq!(
                    status(&router, method, uri, None).await,
                    StatusCode::UNAUTHORIZED,
                    "{method} {uri}"
                );
            }
        }

        #[tokio::test]
        async fn alternate_methods_and_paths_never_bypass_auth() {
            let router = router(&[], Some(TOKEN));
            for (method, uri) in [
                ("HEAD", "/info"),
                ("OPTIONS", "/info"),
                ("GET", "/info/"),
                ("GET", "//info"),
                ("GET", "/%69nfo"),
            ] {
                assert_ne!(
                    status(&router, method, uri, None).await,
                    StatusCode::OK,
                    "{method} {uri}"
                );
            }
        }

        #[tokio::test]
        async fn malformed_authorization_headers_are_rejected() {
            let router = router(&[], Some(TOKEN));
            for authorization in ["Bearer  s3cret-token", "Bearer ", "Bearer"] {
                assert_eq!(
                    status(&router, "GET", "/info", Some(authorization)).await,
                    StatusCode::UNAUTHORIZED,
                    "{authorization:?}"
                );
            }

            for headers in [
                ["Bearer s3cret-token", "Bearer wrong"],
                ["Bearer wrong", "Bearer s3cret-token"],
            ] {
                let response = router
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri("/info")
                            .header(header::AUTHORIZATION, headers[0])
                            .header(header::AUTHORIZATION, headers[1])
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{headers:?}");
            }
        }

        #[tokio::test]
        async fn unauthenticated_requests_do_not_wait_for_a_slot() {
            let router = router(&[], Some(TOKEN));
            let stalled_body = Body::from_stream(futures::stream::pending::<Result<Vec<u8>, Infallible>>());
            let holder = tokio::spawn(
                router.clone().oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/extract")
                        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                        .header("content-type", "multipart/form-data; boundary=x")
                        .body(stalled_body)
                        .unwrap(),
                ),
            );
            tokio::time::sleep(Duration::from_millis(100)).await;

            let rejected = tokio::time::timeout(Duration::from_secs(2), status(&router, "POST", "/extract", None))
                .await
                .expect("an unauthenticated request must not queue behind the held slot");
            assert_eq!(rejected, StatusCode::UNAUTHORIZED);
            holder.abort();
        }

        #[tokio::test]
        async fn preflight_to_a_protected_route_is_answered_before_auth() {
            let router = router(&["https://app.example.com"], Some(TOKEN));

            let response = preflight(&router, "https://app.example.com").await;
            assert!(response.status().is_success(), "{}", response.status());
            assert_eq!(
                response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
                "https://app.example.com"
            );

            let real = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/extract")
                        .header(header::ORIGIN, "https://app.example.com")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(real.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn simple_cross_origin_request_gets_no_cors_headers_by_default() {
            let router = router(&[], None);
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/info")
                        .header(header::ORIGIN, "https://evil.example")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert!(!response.headers().contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
        }

        async fn preflight(router: &Router, origin: &str) -> axum::response::Response {
            router
                .clone()
                .oneshot(
                    Request::builder()
                        .method("OPTIONS")
                        .uri("/extract")
                        .header(header::ORIGIN, origin)
                        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap()
        }

        #[tokio::test]
        async fn cors_is_denied_by_default() {
            let router = router(&[], Some(TOKEN));
            let response = preflight(&router, "https://evil.example").await;
            assert!(!response.headers().contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
        }

        #[tokio::test]
        async fn cors_allows_only_listed_origins() {
            let router = router(&["https://app.example.com"], Some(TOKEN));

            let allowed = preflight(&router, "https://app.example.com").await;
            assert_eq!(
                allowed.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
                "https://app.example.com"
            );

            let denied = preflight(&router, "https://evil.example").await;
            assert!(!denied.headers().contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
        }

        #[tokio::test]
        async fn cors_wildcard_is_an_explicit_opt_in() {
            let router = router(&["*"], Some(TOKEN));
            let response = preflight(&router, "https://anything.example").await;
            assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
        }
    }

    #[test]
    fn test_create_router() {
        let config = ExtractionConfig::default();
        let _router = create_router(config);
    }

    #[test]
    fn test_router_has_routes() {
        use std::mem::size_of_val;
        let config = ExtractionConfig::default();
        let router = create_router(config);
        assert!(size_of_val(&router) > 0);
    }

    #[test]
    fn test_create_router_with_limits() {
        let config = ExtractionConfig::default();
        let limits = ApiSizeLimits::from_mb(50, 50);
        let _router = create_router_with_limits(config, limits);
    }

    #[test]
    fn test_create_router_with_server_config() {
        let extraction_config = ExtractionConfig::default();
        let limits = ApiSizeLimits::from_mb(100, 100);
        let server_config = ServerConfig::default();
        let _router = create_router_with_limits_and_server_config(extraction_config, limits, server_config);
    }

    #[cfg(feature = "api")]
    #[test]
    fn build_api_state_copies_job_timeout_secs_from_server_config() {
        let server_config = ServerConfig {
            job_timeout_secs: 42,
            ..ServerConfig::default()
        };
        let state = build_api_state(ExtractionConfig::default(), &server_config, 0);
        assert_eq!(state.job_timeout_secs, 42);
    }

    #[cfg(feature = "api")]
    #[tokio::test]
    async fn build_api_state_sizes_job_execution_slots_from_the_request_limit() {
        use std::time::Duration;

        let state = build_api_state(ExtractionConfig::default(), &ServerConfig::default(), 2);
        let store = &state.job_store;
        let job_ids: Vec<String> = (0..3).map(|_| store.create_job().expect("slot available")).collect();

        let first = store.acquire_execution_slot(&job_ids[0]).await.expect("first slot");
        let _second = store.acquire_execution_slot(&job_ids[1]).await.expect("second slot");
        let third = tokio::time::timeout(Duration::from_millis(200), store.acquire_execution_slot(&job_ids[2])).await;
        assert!(third.is_err(), "a third job must wait while two jobs run");

        drop(first);
        let third = tokio::time::timeout(Duration::from_secs(2), store.acquire_execution_slot(&job_ids[2])).await;
        assert!(third.expect("the third job takes the freed slot").is_some());
    }

    #[test]
    fn test_server_config_cors_handling() {
        let extraction_config = ExtractionConfig::default();
        let limits = ApiSizeLimits::default();
        let server_config = ServerConfig {
            cors_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };
        let _router = create_router_with_limits_and_server_config(extraction_config, limits, server_config);
    }

    const MAX_CONCURRENCY_CASE: &str = "XBERG_API_MAX_CONCURRENCY_TEST_CASE";
    const MAX_CONCURRENCY_CHILD: &str = "api::router::tests::resolve_max_concurrent_requests_child";

    #[test]
    fn resolve_max_concurrent_requests_honors_env_and_defaults() {
        for (case, value) in [
            ("explicit", Some("7")),
            ("disabled", Some("0")),
            ("invalid", Some("not-a-number")),
            ("unset", None),
        ] {
            crate::utils::test_support::run_child_test(MAX_CONCURRENCY_CHILD, MAX_CONCURRENCY_CASE, case, |command| {
                match value {
                    Some(value) => command.env(MAX_CONCURRENT_REQUESTS_ENV, value),
                    None => command.env_remove(MAX_CONCURRENT_REQUESTS_ENV),
                };
            });
        }
    }

    #[test]
    #[ignore = "run in an isolated subprocess by the max-concurrency test"]
    fn resolve_max_concurrent_requests_child() {
        let Some(case) = crate::utils::test_support::child_test_root(MAX_CONCURRENCY_CASE) else {
            return;
        };
        let (value, expected) = match case.to_str() {
            Some("explicit") => (Some("7"), Some(7)),
            Some("disabled") => (Some("0"), Some(0)),
            Some("invalid") => (Some("not-a-number"), None),
            Some("unset") => (None, None),
            _ => panic!("unknown max-concurrency test case"),
        };
        let actual_env = std::env::var_os(MAX_CONCURRENT_REQUESTS_ENV);
        assert!(
            actual_env.as_deref() == value.map(std::ffi::OsStr::new),
            "wrong input for {case:?}"
        );

        let actual = resolve_max_concurrent_requests();
        if let Some(expected) = expected {
            assert_eq!(actual, expected, "wrong limit for {case:?}");
        } else {
            assert!(
                (4..=32).contains(&actual),
                "fallback for {case:?} must be bounded, got {actual}"
            );
        }
    }
}
