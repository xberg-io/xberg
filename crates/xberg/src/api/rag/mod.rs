//! Fork-only API surface (absent from xberg-io/xberg): the /v1/process
//! pipeline (extract → NER → redact) and encrypted rehydration.
//! Fenced here so upstream `api/handlers.rs` and `api/types.rs` stay
//! byte-close to upstream. Gated on the `process-api` feature.

pub mod handlers;
pub mod types;

use axum::Router;
use axum::routing::post;

use super::types::ApiState;

/// Routes owned by the fork. Merged into the main router by `router.rs`.
pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/process", post(handlers::process_handler))
        .route(
            "/v1/documents/{rehydration_key}/rehydrate",
            post(handlers::rehydrate_handler),
        )
}

// Verifies the fork API extraction preserves the /v1/process route in the
// fenced module. `ApiState` cannot be built here without the
// `redaction-rehydrate` feature (see `handlers::tests::make_api_state`), so
// this test is additionally gated on it.
#[cfg(all(test, feature = "process-api", feature = "redaction-rehydrate"))]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt; // oneshot

    use super::handlers::tests::make_api_state;
    use super::routes;

    #[tokio::test]
    async fn process_route_is_matched_by_rag_module() {
        let app = routes().with_state(make_api_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/process")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Route exists → not a 404. (Empty body may yield 400/415/422; all prove match.)
        assert_ne!(resp.status(), StatusCode::NOT_FOUND);
    }
}
