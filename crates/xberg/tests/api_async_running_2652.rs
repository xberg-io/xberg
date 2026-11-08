#![cfg(all(feature = "api", feature = "tokio-runtime"))]

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tokio::sync::Notify;
use tower::ServiceExt;

use xberg::{
    ExtractInput, ExtractedDocument, ExtractionConfig, ExtractionResult,
    api::create_router,
    plugins::{DocumentExtractor, Plugin, register_document_extractor, unregister_document_extractor},
};

const MIME: &str = "text/x-djot";
const PLUGIN_NAME: &str = "running-job-test";
const CONTENT: &str = "held extraction completed";
const DEADLINE: Duration = Duration::from_secs(10);

#[derive(Default)]
struct HeldExtractor {
    started: Notify,
    release: Notify,
}

impl Plugin for HeldExtractor {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl DocumentExtractor for HeldExtractor {
    async fn extract(&self, input: ExtractInput, _config: &ExtractionConfig) -> xberg::Result<ExtractedDocument> {
        assert_eq!(input.bytes.as_deref(), Some(CONTENT.as_bytes()));
        self.started.notify_one();
        self.release.notified().await;
        let mut document = ExtractedDocument::default();
        document.content = CONTENT.to_owned();
        document.mime_type = MIME.into();
        Ok(document)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[MIME]
    }

    fn priority(&self) -> i32 {
        100
    }
}

struct RegisteredExtractor(Arc<HeldExtractor>);

impl Drop for RegisteredExtractor {
    fn drop(&mut self) {
        self.0.release.notify_one();
        unregister_document_extractor(PLUGIN_NAME).expect("test extractor unregistered");
    }
}

async fn submit_job(app: &Router) -> String {
    let request = Request::builder()
        .method("POST")
        .uri("/extract-async")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"inputs": [{"text": CONTENT, "mime_type": MIME}]}).to_string(),
        ))
        .expect("valid submission");
    let response = app.clone().oneshot(request).await.expect("submission responded");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("submission body");
    let submission: serde_json::Value = serde_json::from_slice(&bytes).expect("job submission JSON");
    submission["job_id"].as_str().expect("submitted job ID").to_owned()
}

async fn poll_job(app: &Router, job_id: &str) -> serde_json::Value {
    let request = Request::builder()
        .uri(format!("/jobs/{job_id}"))
        .body(Body::empty())
        .expect("valid poll");
    let response = app.clone().oneshot(request).await.expect("poll responded");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("poll body");
    let status: serde_json::Value = serde_json::from_slice(&bytes).expect("job status JSON");
    assert_eq!(status["job_id"], job_id);
    status
}

#[tokio::test]
async fn should_report_running_while_extractor_is_held_then_complete_after_release() {
    let extractor = Arc::new(HeldExtractor::default());
    register_document_extractor(extractor.clone()).expect("test extractor registered");
    let _registration = RegisteredExtractor(extractor.clone());
    let app = create_router(ExtractionConfig {
        use_cache: false,
        ..Default::default()
    });

    tokio::time::timeout(DEADLINE, async {
        let job_id = submit_job(&app).await;
        extractor.started.notified().await;
        let running = poll_job(&app, &job_id).await;
        assert_eq!(running["state"], "running");
        assert_eq!(running.get("result"), None);
        assert_eq!(running.get("error"), None);

        extractor.release.notify_one();
        let completed = loop {
            let status = poll_job(&app, &job_id).await;
            if status["state"] != "running" {
                break status;
            }
            tokio::task::yield_now().await;
        };
        assert_eq!(completed["state"], "completed");
        assert_eq!(completed.get("error"), None);
        let result: ExtractionResult =
            serde_json::from_value(completed["result"].clone()).expect("extraction envelope");
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].content, CONTENT);
        assert_eq!(result.results[0].mime_type, MIME);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(
            (result.summary.inputs, result.summary.results, result.summary.errors),
            (1, 1, 0)
        );
    })
    .await
    .expect("job must start and complete within deadline");
}
