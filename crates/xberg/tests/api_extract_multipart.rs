//! Integration test for the `/extract` API handler using multipart uploads.

#![cfg(feature = "api")]

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use xberg::{ExtractionConfig, ExtractionResult, api::ApiSizeLimits};

#[tokio::test]
async fn test_extract_accepts_single_file_multipart() {
    let router = create_router_with_limits(
        ExtractionConfig::default(),
        ApiSizeLimits {
            max_request_body_bytes: 5 * 1024 * 1024,
            max_multipart_field_bytes: 5 * 1024 * 1024,
        },
    );

    let boundary = "X-BOUNDARY";
    let body = format!(
        "--{boundary}\r\n\
Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
Hello world\r\n\
--{boundary}--\r\n"
    );
    let body_bytes = body.into_bytes();

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={boundary}"))
        .header("content-length", body_bytes.len())
        .body(Body::from(body_bytes))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), 1_000_000)
        .await
        .expect("Failed to read body");
    let value: Value = serde_json::from_slice(&bytes).expect("Response JSON parse failed");
    let envelope: ExtractionResult = serde_json::from_slice(&bytes).expect("Response should match ExtractionResult");

    let content = value
        .get("results")
        .and_then(Value::as_array)
        .and_then(|results| results.first())
        .and_then(|v| v.get("content"))
        .and_then(Value::as_str)
        .expect("Response should include extracted content");

    assert_eq!(content.trim_end_matches('\n'), "Hello world");
    if let Some(errors) = value.get("errors") {
        assert!(
            errors.as_array().is_some_and(Vec::is_empty),
            "Expected empty errors array, got {errors}"
        );
    }
    assert_eq!(value.pointer("/summary/inputs").and_then(Value::as_u64), Some(1));
    assert_eq!(value.pointer("/summary/results").and_then(Value::as_u64), Some(1));
    assert_eq!(value.pointer("/summary/errors").and_then(Value::as_u64), Some(0));
    assert!(
        envelope.errors.is_empty(),
        "Multipart extraction should not report errors"
    );
    assert_eq!(envelope.summary.inputs, 1);
    assert_eq!(envelope.summary.results, 1);
    assert_eq!(envelope.summary.errors, 0);
}

/// Router with authentication pinned off, so the test does not depend on `XBERG_API_TOKEN`.
fn create_router_with_limits(config: ExtractionConfig, limits: ApiSizeLimits) -> axum::Router {
    xberg::api::create_router_with_auth(config, limits, xberg::api::ApiAuth::new(None, false))
}
