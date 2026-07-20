//! Integration tests for the OpenWebUI compatibility endpoints.

#![cfg(feature = "api")]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use xberg::{
    ExtractionConfig,
    api::{DoclingCompatResponse, OpenWebDocumentResponse, create_router},
};

/// Test successful extraction via the external engine endpoint.
#[tokio::test]
async fn test_openweb_process_text_file() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/process")
                .header("content-type", "text/plain")
                .header("X-Filename", "hello.txt")
                .body(Body::from("Hello, world!"))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read HTTP response body");
    let doc: OpenWebDocumentResponse = serde_json::from_slice(&body).expect("Failed to deserialize JSON response");

    assert!(
        doc.page_content.contains("Hello, world"),
        "Expected extracted text to contain 'Hello, world', got: {}",
        doc.page_content
    );
    assert_eq!(doc.metadata.source, "hello.txt");
}

/// Test that a URL-encoded filename in X-Filename is decoded correctly.
#[tokio::test]
async fn test_openweb_process_url_encoded_filename() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/process")
                .header("content-type", "text/plain")
                .header("X-Filename", "my%20document%20%281%29.txt")
                .body(Body::from("content"))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read HTTP response body");
    let doc: OpenWebDocumentResponse = serde_json::from_slice(&body).expect("Failed to deserialize JSON response");

    assert_eq!(doc.metadata.source, "my document (1).txt");
}

/// Test that the external endpoint returns 400 on empty body.
#[tokio::test]
async fn test_openweb_process_empty_body_returns_400() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/process")
                .header("content-type", "text/plain")
                .body(Body::empty())
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Test fallback when no X-Filename header is provided.
#[tokio::test]
async fn test_openweb_process_missing_filename_defaults_to_unknown() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/process")
                .header("content-type", "text/plain")
                .body(Body::from("some text"))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read HTTP response body");
    let doc: OpenWebDocumentResponse = serde_json::from_slice(&body).expect("Failed to deserialize JSON response");

    assert_eq!(doc.metadata.source, "unknown");
}

/// Test MIME type detection from filename when Content-Type is octet-stream.
#[tokio::test]
async fn test_openweb_process_octet_stream_detects_mime_from_filename() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/process")
                .header("content-type", "application/octet-stream")
                .header("X-Filename", "readme.txt")
                .body(Body::from("Plain text content"))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read HTTP response body");
    let doc: OpenWebDocumentResponse = serde_json::from_slice(&body).expect("Failed to deserialize JSON response");

    assert!(doc.page_content.contains("Plain text content"));
}

/// Test successful extraction via the docling-compatible endpoint.
#[tokio::test]
async fn test_openweb_docling_text_file() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let body_content = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         Hello from docling!\r\n\
         --{boundary}--\r\n"
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/convert/file")
                .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                .body(Body::from(body_content))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read HTTP response body");
    let resp: DoclingCompatResponse = serde_json::from_slice(&body).expect("Failed to deserialize JSON response");

    assert_eq!(resp.status, "success");
    assert!(
        resp.document.md_content.contains("Hello from docling"),
        "Expected md_content to contain 'Hello from docling', got: {}",
        resp.document.md_content
    );
}

/// Test that the docling endpoint returns 400 when no files field is provided.
#[tokio::test]
async fn test_openweb_docling_no_file_returns_400() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/convert/file")
                .header("content-type", "multipart/form-data; boundary=testboundary")
                .body(Body::from("--testboundary--\r\n"))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Test that the docling endpoint detects MIME from filename when Content-Type is octet-stream.
#[tokio::test]
async fn test_openweb_docling_octet_stream_detects_mime() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let body_content = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"data.txt\"\r\n\
         Content-Type: application/octet-stream\r\n\
         \r\n\
         Some plain text\r\n\
         --{boundary}--\r\n"
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/convert/file")
                .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                .body(Body::from(body_content))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read HTTP response body");
    let resp: DoclingCompatResponse = serde_json::from_slice(&body).expect("Failed to deserialize JSON response");

    assert_eq!(resp.status, "success");
    assert!(resp.document.md_content.contains("Some plain text"));
}

/// Test that the response JSON structure matches what OpenWebUI expects.
#[tokio::test]
async fn test_openweb_docling_response_structure() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let body_content = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         content\r\n\
         --{boundary}--\r\n"
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/convert/file")
                .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                .body(Body::from(body_content))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read HTTP response body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to deserialize JSON response");

    assert!(json["document"].is_object(), "Expected 'document' object");
    assert!(
        json["document"]["md_content"].is_string(),
        "Expected 'document.md_content' string"
    );
    assert!(json["status"].is_string(), "Expected 'status' string");
}

/// Fixture extractor + guard used to prove that the OpenWebUI-compatible
/// routes honor a caller-configured `extraction_timeout_secs` rather than a
/// hardcoded default. See #1273.
mod slow_fixture_extractor {
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::time::Duration;
    use xberg::plugins::{DocumentExtractor, Plugin};
    use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, Result};

    /// A made-up `image/*` subtype: the `image/` prefix short-circuits
    /// `validate_mime_type` without needing to be in the static supported-MIME
    /// list, and the exact string is unique enough that registering an
    /// extractor for it cannot shadow any real, built-in extractor.
    pub(crate) const SLOW_FIXTURE_MIME: &str = "image/x-xberg-router-timeout-fixture-1273";
    const SLOW_FIXTURE_NAME: &str = "router-test-slow-fixture-1273";

    /// Sleeps for a fixed duration before returning, so a test can force a
    /// deterministic timeout instead of racing a near-zero duration.
    struct SlowFixtureExtractor {
        sleep: Duration,
    }

    impl Plugin for SlowFixtureExtractor {
        fn name(&self) -> &str {
            SLOW_FIXTURE_NAME
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl DocumentExtractor for SlowFixtureExtractor {
        async fn extract(&self, _input: ExtractInput, _config: &ExtractionConfig) -> Result<ExtractedDocument> {
            tokio::time::sleep(self.sleep).await;
            // `ExtractedDocument` has crate-private fields, so it cannot be built with
            // struct-literal `..Default::default()` from outside `xberg`. Start from
            // `Default::default()` and mutate the public fields instead. ~keep
            let mut document = ExtractedDocument::default();
            document.content = "slow fixture output".to_string();
            document.mime_type = SLOW_FIXTURE_MIME.into();
            Ok(document)
        }

        fn supported_mime_types(&self) -> &[&str] {
            &[SLOW_FIXTURE_MIME]
        }

        fn priority(&self) -> i32 {
            i32::MAX
        }
    }

    /// RAII guard that unregisters the fixture extractor even if an assertion
    /// panics, so a failing test run can't leak global registry state into
    /// tests that follow it.
    pub(crate) struct FixtureGuard;

    impl FixtureGuard {
        pub(crate) fn register(sleep: Duration) -> Self {
            xberg::plugins::register_document_extractor(Arc::new(SlowFixtureExtractor { sleep }))
                .expect("slow fixture extractor registration must succeed");
            Self
        }
    }

    impl Drop for FixtureGuard {
        fn drop(&mut self) {
            let _ = xberg::plugins::unregister_document_extractor(SLOW_FIXTURE_NAME);
        }
    }
}

/// Regression test for #1273: `xberg serve`'s OpenWebUI-compatible routes
/// returned HTTP 500 after ~60s on large/slow (VLM) documents because the
/// hard `extraction_timeout_secs` default fired regardless of what a
/// deployment configured. This proves `create_router` threads a
/// caller-supplied `extraction_timeout_secs` all the way through to the
/// timeout actually enforced around extraction on the `/process` route — not
/// a hardcoded default — by configuring a timeout (1s) far below both the
/// pre-fix (60s) and post-fix (600s) hard defaults and asserting the request
/// fails fast instead of running out the mock extractor's 3s delay.
#[serial_test::serial]
#[tokio::test]
async fn test_openweb_process_honors_configured_extraction_timeout_not_hard_default() {
    use slow_fixture_extractor::{FixtureGuard, SLOW_FIXTURE_MIME};

    let _guard = FixtureGuard::register(std::time::Duration::from_secs(3));

    let config = ExtractionConfig {
        extraction_timeout_secs: Some(1),
        ..ExtractionConfig::default()
    };
    let app = create_router(config);

    let started = std::time::Instant::now();
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/process")
                .header("content-type", SLOW_FIXTURE_MIME)
                .header("X-Filename", "slow.bin")
                .body(Body::from(vec![0u8; 16]))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");
    let elapsed = started.elapsed();

    assert_eq!(
        response.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "a 1s configured extraction_timeout_secs must trip before the 3s mock extractor finishes"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "timed-out response took {:?}; it must fire at the configured 1s bound instead of waiting \
         out the 3s mock extractor, which would indicate the router used a hardcoded default \
         instead of the configured value",
        elapsed
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read HTTP response body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to deserialize JSON response");
    assert_eq!(json["error_type"], "TimeoutError");
}

/// Test that the external engine response structure matches what OpenWebUI expects.
#[tokio::test]
async fn test_openweb_process_response_structure() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/process")
                .header("content-type", "text/plain")
                .header("X-Filename", "test.txt")
                .body(Body::from("content"))
                .expect("Failed to create HTTP request body"),
        )
        .await
        .expect("Failed to send HTTP request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read HTTP response body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to deserialize JSON response");

    assert!(json["page_content"].is_string(), "Expected 'page_content' string");
    assert!(json["metadata"].is_object(), "Expected 'metadata' object");
    assert!(
        json["metadata"]["source"].is_string(),
        "Expected 'metadata.source' string"
    );
}
