//! Integration tests for the /rerank API endpoint.

#![cfg(all(feature = "api", feature = "reranker"))]

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use async_trait::async_trait;
use xberg::{
    ExtractionConfig, Result, XbergError,
    api::{RerankResponse, create_router},
    plugins::{Plugin, RerankerBackend, register_reranker_backend, unregister_reranker_backend},
};

struct MockReranker {
    name: String,
    scores: Vec<f32>,
}

impl Plugin for MockReranker {
    fn name(&self) -> &str {
        &self.name
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
impl RerankerBackend for MockReranker {
    async fn rerank(&self, _query: String, documents: Vec<String>) -> Result<Vec<f32>> {
        if documents.len() != self.scores.len() {
            return Err(XbergError::Plugin {
                message: format!(
                    "MockReranker '{}' expected {} documents, got {}",
                    self.name,
                    self.scores.len(),
                    documents.len()
                ),
                plugin_name: self.name.clone(),
            });
        }
        Ok(self.scores.clone())
    }
}

/// Happy path: scores sorted descending, indices map back to inputs,
/// sigmoid is applied (mixed-sign logits land on either side of 0.5).
#[tokio::test(flavor = "multi_thread")]
async fn test_rerank_happy_path_sorted_descending() {
    const BACKEND_NAME: &str = "rerank-happy";
    // Targeted cleanup: only this test's backend, never the whole registry — a global
    // clear races with peer tests that share the process-wide reranker registry.
    let _ = unregister_reranker_backend(BACKEND_NAME);
    // Backend returns raw logits. Mixed-sign so the test catches a regression
    // where sigmoid is silently removed: negative logit must produce score
    // < 0.5, large positive logit must produce score close to 1.0.
    //
    // Logits: [-2.0, 3.0, 0.5]
    // Sigmoid: [~0.119, ~0.953, ~0.622]
    // Sorted desc by score: idx=1 (0.953), idx=2 (0.622), idx=0 (0.119)
    register_reranker_backend(Arc::new(MockReranker {
        name: BACKEND_NAME.to_string(),
        scores: vec![-2.0, 3.0, 0.5],
    }))
    .unwrap();

    let app = create_router(ExtractionConfig::default());
    let body = json!({
        "query": "a query",
        "documents": ["alpha", "bravo", "charlie"],
        "config": { "model": { "type": "plugin", "name": "rerank-happy" } }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rerank")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let rr: RerankResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(rr.results.len(), 3);

    // Index order: highest-logit (3.0 → idx=1) first, then 0.5 → idx=2, then -2.0 → idx=0.
    assert_eq!(rr.results[0].index, 1, "highest logit (3.0) must rank first");
    assert_eq!(rr.results[1].index, 2, "middle logit (0.5) must rank second");
    assert_eq!(rr.results[2].index, 0, "negative logit (-2.0) must rank last");

    // Strict descending order.
    assert!(rr.results[0].score > rr.results[1].score);
    assert!(rr.results[1].score > rr.results[2].score);

    // All scores in [0, 1] — sigmoid output range.
    for r in &rr.results {
        assert!(
            r.score >= 0.0 && r.score <= 1.0,
            "score {} out of sigmoid range",
            r.score
        );
    }

    // Sigmoid contract: negative logit → score < 0.5, large positive → score > 0.9.
    let score_for = |idx: usize| rr.results.iter().find(|r| r.index == idx).unwrap().score;
    assert!(
        score_for(0) < 0.5,
        "sigmoid(-2.0) should be < 0.5, got {}",
        score_for(0)
    );
    assert!(score_for(1) > 0.9, "sigmoid(3.0) should be > 0.9, got {}", score_for(1));

    let _ = unregister_reranker_backend(BACKEND_NAME);
}

/// top_k truncates the response to the highest-scoring documents.
#[tokio::test(flavor = "multi_thread")]
async fn test_rerank_top_k_truncation() {
    const BACKEND_NAME: &str = "rerank-topk";
    let _ = unregister_reranker_backend(BACKEND_NAME);
    register_reranker_backend(Arc::new(MockReranker {
        name: BACKEND_NAME.to_string(),
        scores: vec![0.9, 0.1, 0.5],
    }))
    .unwrap();

    let app = create_router(ExtractionConfig::default());
    let body = json!({
        "query": "q",
        "documents": ["a", "b", "c"],
        "config": {
            "model": { "type": "plugin", "name": "rerank-topk" },
            "top_k": 2
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rerank")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let rr: RerankResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(rr.results.len(), 2);

    let _ = unregister_reranker_backend(BACKEND_NAME);
}

/// Empty documents → empty results, not an error.
#[tokio::test(flavor = "multi_thread")]
async fn test_rerank_empty_documents_returns_empty_results() {
    let app = create_router(ExtractionConfig::default());
    let body = json!({
        "query": "q",
        "documents": [],
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rerank")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let rr: RerankResponse = serde_json::from_slice(&bytes).unwrap();
    assert!(rr.results.is_empty());
}

/// Empty query → 400.
#[tokio::test(flavor = "multi_thread")]
async fn test_rerank_empty_query_returns_400() {
    let app = create_router(ExtractionConfig::default());
    let body = json!({ "query": "", "documents": ["a"] });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rerank")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Whitespace-only query → 400.
#[tokio::test(flavor = "multi_thread")]
async fn test_rerank_whitespace_query_returns_400() {
    let app = create_router(ExtractionConfig::default());
    let body = json!({ "query": "   \t  ", "documents": ["a"] });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rerank")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Document containing only whitespace → 400.
#[tokio::test(flavor = "multi_thread")]
async fn test_rerank_empty_document_string_returns_400() {
    let app = create_router(ExtractionConfig::default());
    let body = json!({ "query": "q", "documents": ["valid", "   "] });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rerank")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
