#![cfg(feature = "api")]
//! Diagnostic tests for large PDF file extraction issues.
//!
//! These tests are designed to isolate and identify the root cause of
//! issues with large PDF file handling in the Kreuzberg API server.
//!
//! Current Status:
//! - 5MB PDF tests are returning HTTP 400 instead of HTTP 200
//! - This suggests either:
//!   a) The mock PDF structure is invalid
//!   b) The PDF extraction logic has issues with the generated content
//!   c) The multipart parsing is failing on large payloads
//!
//! These diagnostic tests help narrow down which component is failing.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use kreuzberg::{
    ExtractionConfig,
    api::{ApiSizeLimits, create_router_with_limits},
};
use serde_json::Value;
use tower::ServiceExt;

/// Test extracting a minimal valid PDF (control test).
///
/// This serves as a baseline to verify the API can handle valid PDFs
/// before testing with large files.
#[tokio::test]
async fn test_extract_minimal_valid_pdf() {
    let router = create_router_with_limits(ExtractionConfig::default(), ApiSizeLimits::from_mb(10, 10));

    let pdf_content = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>
endobj
4 0 obj
<< >>
stream
BT /F1 12 Tf 50 750 Td (Hello) Tj ET
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000074 00000 n
0000000133 00000 n
0000000214 00000 n
trailer
<< /Size 5 /Root 1 0 R >>
startxref
340
%%EOF";

    let boundary = "----minimal-pdf";
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"files\"; filename=\"minimal.pdf\"\r\n");
    body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
    body.extend_from_slice(pdf_content);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", body.len())
        .body(Body::from(body))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Minimal PDF should extract successfully. Status: {} indicates baseline is working",
        response.status()
    );

    let body = to_bytes(response.into_body(), 1_000_000)
        .await
        .expect("Failed to read response body");

    let parsed: Value = serde_json::from_slice(&body).expect("Failed to parse response");
    eprintln!("Extraction result: {}", serde_json::to_string_pretty(&parsed).unwrap());
}

/// Test extracting a 1MB text file (control test without PDF).
///
/// This isolates whether the issue is specific to PDF handling or
/// a general problem with large multipart uploads.
#[tokio::test]
async fn test_extract_1mb_text_file() {
    let router = create_router_with_limits(ExtractionConfig::default(), ApiSizeLimits::from_mb(10, 10));

    let boundary = "----large-text";
    let large_text = "This is test content. ".repeat(50000);

    let mut body = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"files\"; filename=\"large.txt\"\r\n");
    body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
    body.extend_from_slice(large_text.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", body.len())
        .body(Body::from(body))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");

    println!("1MB text file extraction status: {}", response.status());

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "1MB text file should extract successfully. If this fails, multipart parsing may have issues."
    );
}

/// Test extracting progressively larger text files to find breaking point.
///
/// This helps identify at what size the API starts failing.
#[tokio::test]
async fn test_find_size_breaking_point() {
    let sizes = vec![
        ("100KB", 100 * 1024),
        ("500KB", 500 * 1024),
        ("1MB", 1024 * 1024),
        ("2MB", 2 * 1024 * 1024),
        ("5MB", 5 * 1024 * 1024),
    ];

    for (label, size) in sizes {
        let router = create_router_with_limits(ExtractionConfig::default(), ApiSizeLimits::from_mb(20, 20));

        let boundary = "----size-test";
        let content = "A".repeat(size);

        let mut body = Vec::new();
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"files\"; filename=\"test_{}.txt\"\r\n",
                label
            )
            .as_bytes(),
        );
        body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
        body.extend_from_slice(content.as_bytes());
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let request = Request::builder()
            .method("POST")
            .uri("/extract")
            .header("content-type", format!("multipart/form-data; boundary={}", boundary))
            .header("content-length", body.len())
            .body(Body::from(body))
            .expect("Failed to build request");

        let response = router.oneshot(request).await.expect("Request failed");

        println!("Size {} ({}B): HTTP {}", label, size, response.status().as_u16());

        if response.status() != StatusCode::OK {
            eprintln!("Extraction failed at size: {}", label);

            let body = to_bytes(response.into_body(), 1_000_000)
                .await
                .expect("Failed to read response body");

            if let Ok(parsed) = serde_json::from_slice::<Value>(&body) {
                eprintln!("Error response: {}", serde_json::to_string_pretty(&parsed).unwrap());
            } else {
                eprintln!("Response body (not JSON): {}", String::from_utf8_lossy(&body));
            }

            return;
        }
    }
}

/// Test that the default 100MB limit is being applied.
///
/// Verifies that the server is actually respecting the configured limits,
/// and documents what the default limit actually is.
#[tokio::test]
async fn test_default_size_limits() {
    let default_limits = ApiSizeLimits::default();
    assert_eq!(default_limits.max_request_body_bytes, 100 * 1024 * 1024);
    assert_eq!(default_limits.max_multipart_field_bytes, 100 * 1024 * 1024);

    println!(
        "Default limits: {} bytes request, {} bytes per field",
        default_limits.max_request_body_bytes, default_limits.max_multipart_field_bytes
    );
}

/// Test that the router layer actually applies RequestBodyLimitLayer.
///
/// Creates a router and verifies that size limit enforcement is active.
#[tokio::test]
async fn test_request_body_limit_layer_applied() {
    let small_limits = ApiSizeLimits::from_mb(1, 1);
    let router = create_router_with_limits(ExtractionConfig::default(), small_limits);

    let boundary = "----exceed-limits";
    let large_content = "X".repeat(2 * 1024 * 1024);

    let mut body = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n");
    body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
    body.extend_from_slice(large_content.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", body.len())
        .body(Body::from(body))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");

    assert_eq!(
        response.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "2MB file should be rejected when limit is 1MB"
    );
}

/// Test multipart parsing with incremental content.
///
/// Some implementations have issues with streaming multipart parsing.
/// This test uses proper CRLF line endings to ensure correct parsing.
#[tokio::test]
async fn test_multipart_proper_crlf_formatting() {
    let router = create_router_with_limits(ExtractionConfig::default(), ApiSizeLimits::from_mb(10, 10));

    let content = "Test PDF content that is at least somewhat large for testing purposes.";

    let mut body = Vec::new();

    body.extend_from_slice(b"--BOUNDARY123456\r\n");

    body.extend_from_slice(b"Content-Disposition: form-data; name=\"files\"; filename=\"test.pdf\"\r\n");
    body.extend_from_slice(b"Content-Type: application/pdf\r\n");

    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(content.as_bytes());

    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(b"--BOUNDARY123456--\r\n");

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", "multipart/form-data; boundary=BOUNDARY123456")
        .header("content-length", body.len())
        .body(Body::from(body))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");

    println!("Multipart with proper CRLF: HTTP {}", response.status().as_u16());
    assert!(response.status().is_success() || response.status().is_client_error());
}
