//! Proves the durable rehydration path end-to-end: a map written through
//! `POST /v1/process` is still rehydratable through `POST
//! /v1/documents/{id}/rehydrate` after the backing store is dropped and
//! reopened against the same file — the scenario the in-memory backend
//! (24h TTL, lost on restart) could never satisfy.

#![cfg(all(feature = "api", feature = "redaction-rehydrate", feature = "doc-store-sqlite"))]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[allow(unsafe_code)]
fn build_router_with_sqlite_store(db_path: &std::path::Path) -> axum::Router {
    // SAFETY: `env::set_var`/`remove_var` are `unsafe` as of the 2024 edition
    // because concurrent env mutation across threads is a data race. This is
    // sound here because each file under `crates/xberg/tests/` compiles to
    // its own test binary/process (cargo's integration-test model), and this
    // file contains exactly one `#[tokio::test]` function, so no other test
    // — in this process or any other — observes or mutates this env var
    // concurrently.
    unsafe {
        std::env::set_var("XBERG_REHYDRATION_DB_PATH", db_path);
    }
    let router = xberg::api::create_router(xberg::ExtractionConfig::default());
    unsafe {
        std::env::remove_var("XBERG_REHYDRATION_DB_PATH");
    }
    router
}

#[tokio::test]
async fn rehydration_map_survives_router_rebuild_against_same_db_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("rehydration.sqlite3");

    // First "process" — build a router backed by the durable store, redact
    // with rehydrate=true, and capture the returned rehydration key.
    let app = build_router_with_sqlite_store(&db_path);
    let process_body = serde_json::json!({
        "text": "Contact Alice at alice@example.com.",
        "operations": {
            "redact": {
                "strategy": "token_replace",
                "rehydrate": true,
                "passphrase": "durability-test-passphrase"
            }
        }
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/process")
                .header("content-type", "application/json")
                .body(Body::from(process_body.to_string()))
                .expect("valid request"),
        )
        .await
        .expect("handler responded");
    assert_eq!(response.status(), StatusCode::OK, "expected /v1/process to succeed");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
    let process_json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");
    let rehydration_key = process_json["rehydration_key"]
        .as_str()
        .expect("rehydration_key must be present when rehydrate=true")
        .to_string();

    // `app` (and the SqliteRehydrationStore it owns) is dropped here —
    // simulating a process restart. A brand-new router is built against the
    // same on-disk database file.
    let app_after_restart = build_router_with_sqlite_store(&db_path);

    let rehydrate_body = serde_json::json!({ "passphrase": "durability-test-passphrase" });
    let response = app_after_restart
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/documents/{rehydration_key}/rehydrate"))
                .header("content-type", "application/json")
                .body(Body::from(rehydrate_body.to_string()))
                .expect("valid request"),
        )
        .await
        .expect("handler responded");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "rehydration must succeed after a simulated restart against the same DB file"
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
    let rehydrate_json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");
    assert_eq!(
        rehydrate_json["restored"]["[EMAIL_1]"].as_str(),
        Some("alice@example.com"),
        "restored map must contain the original PII value after reopening the durable store"
    );
}
