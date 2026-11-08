use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tokio::sync::Notify;
use tower::ServiceExt;

use crate::{
    ExtractionConfig, ExtractionResult,
    api::{
        create_router,
        types::{AsyncJobResponse, JobState, JobStatus},
    },
    plugins::{InternalDocumentExtractor, Plugin, register_document_extractor, unregister_document_extractor},
    types::{internal::InternalDocument, internal_builder::InternalDocumentBuilder},
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
impl InternalDocumentExtractor for HeldExtractor {
    async fn extract_content(
        &self,
        bytes: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> crate::Result<InternalDocument> {
        assert_eq!(bytes, CONTENT.as_bytes());
        self.started.notify_one();
        self.release.notified().await;
        let mut builder = InternalDocumentBuilder::new("running-job-test");
        builder.push_paragraph(CONTENT, vec![], None, None);
        let mut document = builder.build();
        document.mime_type = mime_type.to_owned();
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
    serde_json::from_slice::<AsyncJobResponse>(&bytes)
        .expect("job submission JSON")
        .job_id
}

async fn poll_job(app: &Router, job_id: &str) -> JobStatus {
    let request = Request::builder()
        .uri(format!("/jobs/{job_id}"))
        .body(Body::empty())
        .expect("valid poll");
    let response = app.clone().oneshot(request).await.expect("poll responded");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("poll body");
    let status: JobStatus = serde_json::from_slice(&bytes).expect("job status JSON");
    assert_eq!(status.job_id, job_id);
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
        assert_eq!(running.state, JobState::Running);
        assert_eq!(running.result, None);
        assert_eq!(running.error, None);

        extractor.release.notify_one();
        let completed = loop {
            let status = poll_job(&app, &job_id).await;
            if status.state != JobState::Running {
                break status;
            }
            tokio::task::yield_now().await;
        };
        assert_eq!(completed.state, JobState::Completed);
        assert_eq!(completed.error, None);
        let result: ExtractionResult =
            serde_json::from_value(completed.result.expect("completed result")).expect("extraction envelope");
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
