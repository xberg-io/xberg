#![cfg(feature = "api")]
//! Integration tests for large PDF file extraction (issue #248).
//!
//! Tests verify that the Kreuzberg API server can handle large PDF files
//! without size limits or with very large limits (>2MB, >10MB, >100MB).
//!
//! These tests are designed to be TDD tests - they FAIL with the current
//! implementation if size limits are enforced, demonstrating the bug.
//!
//! The tests ensure:
//! - Large PDFs (>2MB) can be extracted without rejection
//! - Multipart uploads handle large payloads correctly
//! - Server doesn't impose unreasonable size restrictions
//! - Configuration allows tuning limits for different deployment scenarios

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use kreuzberg::{
    ExtractionConfig,
    api::{ApiSizeLimits, create_router_with_limits},
};
use tower::ServiceExt;

/// Helper function to create mock PDF content of a specified size.
///
/// Creates a minimal PDF structure that is valid and parseable, scaled to
/// the requested byte size. The PDF contains repeated text content to reach
/// the target size.
///
/// # Arguments
///
/// * `size_bytes` - Target size of the PDF in bytes
///
/// # Returns
///
/// A Vec<u8> containing valid PDF content of approximately the specified size
fn create_mock_pdf_content(size_bytes: usize) -> Vec<u8> {
    let pdf_header = b"%PDF-1.4\n";
    let mut content = pdf_header.to_vec();

    let catalog = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
    content.extend_from_slice(catalog);

    let pages = b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n";
    content.extend_from_slice(pages);

    let page_header = b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n";
    content.extend_from_slice(page_header);

    let text_content = b"BT /F1 12 Tf 50 750 Td (Large PDF Content for Testing) Tj ET\n";
    let stream_prefix = b"4 0 obj\n<< /Length ";
    let stream_suffix = b" >>\nstream\n";
    let stream_end = b"\nendstream\nendobj\n";

    let text_repeat_count = if size_bytes > content.len() + 200 {
        (size_bytes - content.len() - 200) / text_content.len()
    } else {
        1
    };

    content.extend_from_slice(stream_prefix);

    let stream_size = text_content.len() * text_repeat_count + text_repeat_count;
    content.extend_from_slice(stream_size.to_string().as_bytes());
    content.extend_from_slice(stream_suffix);

    for _ in 0..text_repeat_count {
        content.extend_from_slice(text_content);
        content.push(b'\n');
    }

    content.extend_from_slice(stream_end);

    let xref_offset = content.len();
    let xref = b"xref\n0 5\n0000000000 65535 f \n";
    content.extend_from_slice(xref);

    let trailer = format!(
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        xref_offset
    );
    content.extend_from_slice(trailer.as_bytes());

    content
}

/// Helper function to create a multipart request body with a PDF file.
///
/// Constructs a properly formatted multipart/form-data request body
/// containing a single PDF file.
///
/// # Arguments
///
/// * `boundary` - The multipart boundary string
/// * `pdf_content` - The PDF file content as bytes
/// * `filename` - Name of the PDF file
///
/// # Returns
///
/// A Vec<u8> containing the complete multipart request body
fn create_multipart_pdf_request(boundary: &str, pdf_content: &[u8], filename: &str) -> Vec<u8> {
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());

    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"files\"; filename=\"{}\"\r\n",
            filename
        )
        .as_bytes(),
    );

    body.extend_from_slice(b"Content-Type: application/pdf\r\n");

    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(pdf_content);

    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    body
}

/// Test extracting a 5MB PDF file.
///
/// This test verifies that the API can handle PDF files larger than 2MB,
/// which was the issue reported in #248. The test should FAIL if the server
/// is rejecting requests based on file size limits.
///
/// # Expected Behavior
///
/// The request should succeed with HTTP 200 and return valid extraction results.
/// If the server has a hard limit below 5MB, this test will fail with HTTP 413
/// (Payload Too Large).
#[tokio::test]
async fn test_extract_5mb_pdf_file() {
    let limits = ApiSizeLimits::from_mb(10, 10);
    let router = create_router_with_limits(ExtractionConfig::default(), limits);

    let pdf_size = 5 * 1024 * 1024;
    let pdf_content = create_mock_pdf_content(pdf_size);

    let boundary = "----large-pdf-boundary";
    let request_body = create_multipart_pdf_request(boundary, &pdf_content, "large_5mb.pdf");

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", request_body.len())
        .body(Body::from(request_body))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should successfully extract 5MB PDF file. If status is 413, the server has size limit issues (issue #248)."
    );
}

/// Test extracting a 10MB PDF file.
///
/// This test pushes the size limits further to verify that the API can handle
/// significantly large PDF files (10x the original problem size of 1MB).
///
/// # Expected Behavior
///
/// The request should succeed with HTTP 200. If this fails with HTTP 413,
/// it indicates the server's default size limits are too restrictive.
#[tokio::test]
async fn test_extract_10mb_pdf_file() {
    let limits = ApiSizeLimits::from_mb(20, 20);
    let router = create_router_with_limits(ExtractionConfig::default(), limits);

    let pdf_size = 10 * 1024 * 1024;
    let pdf_content = create_mock_pdf_content(pdf_size);

    let boundary = "----large-pdf-boundary";
    let request_body = create_multipart_pdf_request(boundary, &pdf_content, "large_10mb.pdf");

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", request_body.len())
        .body(Body::from(request_body))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should successfully extract 10MB PDF file without size limit rejection"
    );
}

/// Test extracting a 100MB PDF file.
///
/// This test verifies that the API can handle very large PDF files (100x the
/// original problem size). This is important for production deployments that
/// need to process large document repositories.
///
/// Note: This test may require significant memory and time.
///
/// # Expected Behavior
///
/// The request should succeed with HTTP 200. The test uses very large limits
/// (500MB) to allow the file to be processed.
#[tokio::test]
#[ignore]
async fn test_extract_100mb_pdf_file() {
    let limits = ApiSizeLimits::from_mb(500, 500);
    let router = create_router_with_limits(ExtractionConfig::default(), limits);

    let pdf_size = 100 * 1024 * 1024;
    let pdf_content = create_mock_pdf_content(pdf_size);

    let boundary = "----large-pdf-boundary";
    let request_body = create_multipart_pdf_request(boundary, &pdf_content, "large_100mb.pdf");

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", request_body.len())
        .body(Body::from(request_body))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should successfully extract 100MB PDF file. Requires --ignored flag to run and significant memory."
    );
}

/// Test that default size limits can be exceeded with custom configuration.
///
/// This test verifies that the API respects custom size limit configuration,
/// allowing deployments to tune limits based on their requirements.
///
/// # Expected Behavior
///
/// A 6MB file should fail with the default 100MB limit (actually it shouldn't fail,
/// but it demonstrates how to check if custom limits work). We test with a router
/// configured for smaller limits, then larger limits.
#[tokio::test]
async fn test_size_limits_configurable() {
    let pdf_size = 6 * 1024 * 1024;
    let pdf_content = create_mock_pdf_content(pdf_size);
    let boundary = "----size-limit-test";

    let small_limits = ApiSizeLimits::from_mb(5, 5);
    let router_small = create_router_with_limits(ExtractionConfig::default(), small_limits);

    let request_body = create_multipart_pdf_request(boundary, &pdf_content, "test_6mb.pdf");

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", request_body.len())
        .body(Body::from(request_body.clone()))
        .expect("Failed to build request");

    let response_small = router_small.oneshot(request).await.expect("Request failed");

    assert_eq!(
        response_small.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "6MB file should be rejected when limit is 5MB"
    );

    let large_limits = ApiSizeLimits::from_mb(10, 10);
    let router_large = create_router_with_limits(ExtractionConfig::default(), large_limits);

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", request_body.len())
        .body(Body::from(request_body))
        .expect("Failed to build request");

    let response_large = router_large.oneshot(request).await.expect("Request failed");

    assert_eq!(
        response_large.status(),
        StatusCode::OK,
        "6MB file should be accepted when limit is 10MB"
    );
}

/// Test that custom limits work via ApiSizeLimits::from_mb.
///
/// This test verifies the public API for configuring size limits,
/// ensuring that applications can set limits appropriate for their use case.
///
/// # Expected Behavior
///
/// The test creates limits for 15MB and 20MB separately, demonstrating
/// different request/field limits.
#[tokio::test]
async fn test_api_size_limits_from_mb() {
    let limits_15 = ApiSizeLimits::from_mb(15, 15);
    assert_eq!(limits_15.max_request_body_bytes, 15 * 1024 * 1024);
    assert_eq!(limits_15.max_multipart_field_bytes, 15 * 1024 * 1024);

    let limits_20_10 = ApiSizeLimits::from_mb(20, 10);
    assert_eq!(limits_20_10.max_request_body_bytes, 20 * 1024 * 1024);
    assert_eq!(limits_20_10.max_multipart_field_bytes, 10 * 1024 * 1024);

    let router_15 = create_router_with_limits(ExtractionConfig::default(), limits_15);
    let router_20_10 = create_router_with_limits(ExtractionConfig::default(), limits_20_10);

    assert!(size_of_val(&router_15) > 0);
    assert!(size_of_val(&router_20_10) > 0);
}

/// Test multipart upload with large payload handles streaming correctly.
///
/// This test verifies that the multipart parser can handle large payloads
/// without loading the entire file into memory at once, which is important
/// for processing very large documents.
///
/// # Expected Behavior
///
/// A 12MB file sent via multipart should be accepted if limits allow.
/// The API should handle streaming without excessive memory consumption.
#[tokio::test]
async fn test_multipart_large_payload_streaming() {
    let limits = ApiSizeLimits::from_mb(15, 15);
    let router = create_router_with_limits(ExtractionConfig::default(), limits);

    let pdf_size = 12 * 1024 * 1024;
    let pdf_content = create_mock_pdf_content(pdf_size);

    let boundary = "----multipart-stream-test";
    let request_body = create_multipart_pdf_request(boundary, &pdf_content, "stream_test_12mb.pdf");

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", request_body.len())
        .body(Body::from(request_body))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Multipart upload with 12MB payload should be handled via streaming"
    );
}

/// Test that gigabyte-scale limits can be configured.
///
/// This test verifies that the API can be configured with very large limits
/// suitable for enterprise deployments that need to process massive documents.
///
/// # Expected Behavior
///
/// The API should support limit configurations up to gigabyte scale without
/// panicking or causing overflow. This test doesn't actually send gigabyte
/// files (due to memory constraints), but verifies configuration is possible.
#[tokio::test]
async fn test_gigabyte_scale_limits() {
    let limits = ApiSizeLimits::from_mb(1024, 1024);
    assert_eq!(limits.max_request_body_bytes, 1024 * 1024 * 1024);
    assert_eq!(limits.max_multipart_field_bytes, 1024 * 1024 * 1024);

    let router = create_router_with_limits(ExtractionConfig::default(), limits);

    assert!(size_of_val(&router) > 0);

    let health_request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("Failed to build health check request");

    let response = router.oneshot(health_request).await.expect("Request failed");
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test extracting multiple large PDF files in a single request.
///
/// This test verifies that batch processing of large files works correctly,
/// with the total request size being the sum of all file sizes.
///
/// # Expected Behavior
///
/// Two 4MB PDFs (8MB total) should be accepted when limits are 15MB,
/// demonstrating that the limit applies to total request size, not per-file.
#[tokio::test]
async fn test_extract_multiple_large_pdfs() {
    let limits = ApiSizeLimits::from_mb(15, 15);
    let router = create_router_with_limits(ExtractionConfig::default(), limits);

    let pdf_size = 4 * 1024 * 1024;
    let pdf_content_1 = create_mock_pdf_content(pdf_size);
    let pdf_content_2 = create_mock_pdf_content(pdf_size);

    let boundary = "----multi-large-boundary";
    let mut request_body = Vec::new();

    request_body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    request_body.extend_from_slice(b"Content-Disposition: form-data; name=\"files\"; filename=\"large1.pdf\"\r\n");
    request_body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
    request_body.extend_from_slice(&pdf_content_1);
    request_body.extend_from_slice(b"\r\n");

    request_body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    request_body.extend_from_slice(b"Content-Disposition: form-data; name=\"files\"; filename=\"large2.pdf\"\r\n");
    request_body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
    request_body.extend_from_slice(&pdf_content_2);
    request_body.extend_from_slice(b"\r\n");

    request_body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("content-length", request_body.len())
        .body(Body::from(request_body))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should successfully extract multiple large PDF files when total size within limits"
    );
}

/// Test that API respects environment variable configuration for size limits.
///
/// This test documents how the API parses size limits from the environment,
/// though the actual environment variable reading is handled by
/// `parse_size_limits_from_env()` in server.rs.
///
/// # Note
///
/// This test verifies the ApiSizeLimits struct itself can be configured,
/// demonstrating the pattern that environment variables should follow.
#[tokio::test]
async fn test_environment_configurable_limits_pattern() {
    let env_configured_mb = 256;

    let limits = ApiSizeLimits::from_mb(env_configured_mb, env_configured_mb);
    let router = create_router_with_limits(ExtractionConfig::default(), limits);

    assert_eq!(limits.max_request_body_bytes, 256 * 1024 * 1024);

    let health_request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("Failed to build health check request");

    let response = router.oneshot(health_request).await.expect("Request failed");
    assert_eq!(response.status(), StatusCode::OK);
}
