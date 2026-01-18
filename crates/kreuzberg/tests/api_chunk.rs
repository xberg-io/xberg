//! Integration tests for the /chunk API endpoint.

#![cfg(feature = "api")]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use kreuzberg::{ExtractionConfig, api::create_router};

#[tokio::test]
async fn test_chunk_basic() {
    let app = create_router(ExtractionConfig::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "text": "Short text. More text here. Even more content to chunk."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_chunk_empty_text_returns_400() {
    let app = create_router(ExtractionConfig::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(json!({"text": ""}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chunk_markdown_strategy() {
    let app = create_router(ExtractionConfig::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "text": "# Heading\n\nParagraph text here.",
                        "chunker_type": "markdown"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_chunk_response_structure() {
    use kreuzberg::api::ChunkResponse;

    let app = create_router(ExtractionConfig::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "text": "This is a test. Another sentence here. And one more sentence to ensure we get chunks.",
                        "config": {
                            "max_characters": 50,
                            "overlap": 10,
                            "trim": true
                        },
                        "chunker_type": "text"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let chunk_response: ChunkResponse = serde_json::from_slice(&body).unwrap();

    // Verify response structure
    assert!(chunk_response.chunk_count > 0);
    assert_eq!(chunk_response.chunks.len(), chunk_response.chunk_count);
    assert_eq!(chunk_response.chunker_type, "text");
    assert_eq!(chunk_response.config.max_characters, 50);
    assert_eq!(chunk_response.config.overlap, 10);
    assert!(chunk_response.config.trim);
    assert!(chunk_response.input_size_bytes > 0);

    // Verify chunk metadata
    for (idx, chunk) in chunk_response.chunks.iter().enumerate() {
        assert!(!chunk.content.is_empty());
        assert_eq!(chunk.chunk_index, idx);
        assert_eq!(chunk.total_chunks, chunk_response.chunk_count);
        assert!(chunk.byte_end > chunk.byte_start);
    }
}

#[tokio::test]
async fn test_chunk_invalid_strategy_returns_400() {
    let app = create_router(ExtractionConfig::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "text": "Test text",
                        "chunker_type": "invalid_type"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chunk_with_defaults() {
    use kreuzberg::api::ChunkResponse;

    let app = create_router(ExtractionConfig::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "text": "This is a test sentence. Another sentence here."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let chunk_response: ChunkResponse = serde_json::from_slice(&body).unwrap();

    // Verify defaults are applied
    assert_eq!(chunk_response.config.max_characters, 2000);
    assert_eq!(chunk_response.config.overlap, 100);
    assert!(chunk_response.config.trim);
    assert_eq!(chunk_response.chunker_type, "text");
}

#[tokio::test]
async fn test_chunk_malformed_json_returns_400() {
    let app = create_router(ExtractionConfig::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from("{invalid json}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chunk_case_insensitive_chunker_type() {
    use kreuzberg::api::ChunkResponse;

    let app = create_router(ExtractionConfig::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "text": "# Title\n\nContent here.",
                        "chunker_type": "MARKDOWN"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let chunk_response: ChunkResponse = serde_json::from_slice(&body).unwrap();

    // Verify it's normalized to lowercase
    assert_eq!(chunk_response.chunker_type, "markdown");
}

#[tokio::test]
async fn test_chunk_long_text() {
    use kreuzberg::api::ChunkResponse;

    let app = create_router(ExtractionConfig::default());
    let long_text = "Lorem ipsum dolor sit amet. ".repeat(200);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "text": long_text,
                        "config": {
                            "max_characters": 500,
                            "overlap": 50
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let chunk_response: ChunkResponse = serde_json::from_slice(&body).unwrap();

    // Should have multiple chunks
    assert!(chunk_response.chunk_count > 1);
    assert_eq!(chunk_response.chunks.len(), chunk_response.chunk_count);
}

#[tokio::test]
async fn test_chunk_custom_config() {
    use kreuzberg::api::ChunkResponse;

    let app = create_router(ExtractionConfig::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/chunk")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "text": "Test sentence one. Test sentence two. Test sentence three.",
                        "config": {
                            "max_characters": 30,
                            "overlap": 5,
                            "trim": false
                        },
                        "chunker_type": "text"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let chunk_response: ChunkResponse = serde_json::from_slice(&body).unwrap();

    // Verify custom config was applied
    assert_eq!(chunk_response.config.max_characters, 30);
    assert_eq!(chunk_response.config.overlap, 5);
    assert!(!chunk_response.config.trim);
}
