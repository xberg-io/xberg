//! A REST caller's extraction config may not name server paths or replace the operator config.

#![cfg(feature = "api")]

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use xberg::{ExtractionConfig, api::create_router};

const CSS_FILE_REJECTION: &str = "Validation error: Caller extraction config may not set html_output.css_file";

async fn send(operator: ExtractionConfig, request: Request<Body>) -> (StatusCode, Value) {
    let response = create_router(operator).oneshot(request).await.expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body reads");
    (status, serde_json::from_slice(&body).expect("response is JSON"))
}

fn json_request(uri: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body).expect("request body serializes")))
        .expect("valid request")
}

#[tokio::test]
async fn extract_rejects_server_css_file_in_request_config() {
    let body = json!({
        "inputs": [{"text": "<p>safe</p>", "mime_type": "text/html"}],
        "config": {"output_format": "html", "html_output": {"css_file": "/proc/self/environ"}}
    });

    let (status, error) = send(ExtractionConfig::default(), json_request("/extract", &body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["message"], CSS_FILE_REJECTION);
}

#[tokio::test]
async fn extract_rejects_server_css_file_in_per_input_config() {
    let body = json!({
        "inputs": [{
            "text": "<p>safe</p>",
            "mime_type": "text/html",
            "config": {"output_format": "html", "html_output": {"css_file": "/proc/self/environ"}}
        }]
    });

    let (status, error) = send(ExtractionConfig::default(), json_request("/extract", &body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["message"], CSS_FILE_REJECTION);
}

#[tokio::test]
async fn extract_rejects_server_css_file_in_multipart_config() {
    let boundary = "cssfileboundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"a.html\"\r\n\
         Content-Type: text/html\r\n\r\n<p>safe</p>\r\n\
         --{boundary}\r\nContent-Disposition: form-data; name=\"config\"\r\n\r\n\
         {{\"output_format\":\"html\",\"html_output\":{{\"css_file\":\"/proc/self/environ\"}}}}\r\n\
         --{boundary}--\r\n"
    );
    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={boundary}"))
        .body(Body::from(body))
        .expect("valid multipart request");

    let (status, error) = send(ExtractionConfig::default(), request).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["message"], CSS_FILE_REJECTION);
}

#[tokio::test]
async fn extract_async_rejects_server_css_file_in_request_config() {
    let body = json!({
        "inputs": [{"text": "<p>safe</p>", "mime_type": "text/html"}],
        "config": {"output_format": "html", "html_output": {"css_file": "/proc/self/environ"}}
    });

    let (status, error) = send(ExtractionConfig::default(), json_request("/extract-async", &body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["message"], CSS_FILE_REJECTION);
}

#[tokio::test]
async fn process_rejects_server_css_file_in_x_config() {
    let request = Request::builder()
        .method("PUT")
        .uri("/process")
        .header("content-type", "text/html")
        .header(
            "X-Config",
            r#"{"output_format":"html","html_output":{"css_file":"/proc/self/environ"}}"#,
        )
        .body(Body::from("<p>safe</p>"))
        .expect("valid request");

    let (status, error) = send(ExtractionConfig::default(), request).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["message"], CSS_FILE_REJECTION);
}

#[tokio::test]
#[cfg(feature = "html")]
async fn extract_with_empty_config_keeps_operator_settings() {
    let operator = ExtractionConfig {
        output_format: xberg::OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    };
    let body = json!({
        "inputs": [{"text": "<h1>Title</h1>", "mime_type": "text/html"}],
        "config": {}
    });

    let (status, output) = send(operator, json_request("/extract", &body)).await;

    assert_eq!(status, StatusCode::OK);
    let content = output["results"][0]["content"].as_str().expect("content is a string");
    assert!(
        content.contains("# Title"),
        "an empty caller config must keep the operator's Markdown output, got {content:?}"
    );
}
