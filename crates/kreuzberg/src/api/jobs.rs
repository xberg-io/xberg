//! In-memory job store for async extraction polling.

use std::time::Duration;

use moka::sync::Cache;

use crate::types::events::{JobState, JobStatus};

/// Default time-to-live for completed/failed jobs (5 minutes).
const JOB_TTL: Duration = Duration::from_secs(300);

/// Maximum number of concurrent jobs held in the cache.
const MAX_CAPACITY: u64 = 10_000;

/// Thread-safe in-memory store for async extraction jobs.
///
/// Uses [`moka::sync::Cache`] with built-in TTL eviction — no background
/// eviction task required. Entries are evicted automatically after 5 minutes.
/// Server restarts clear all active jobs.
#[derive(Clone)]
pub struct JobStore {
    jobs: Cache<String, JobStatus>,
}

impl Default for JobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl JobStore {
    /// Create a new empty job store with default TTL and capacity.
    pub fn new() -> Self {
        let jobs = Cache::builder()
            .max_capacity(MAX_CAPACITY)
            .time_to_live(JOB_TTL)
            .build();
        Self { jobs }
    }

    /// Create a new job in the store and return its ID.
    ///
    /// This handles ID generation and initial state registration in one atomic step.
    pub fn create_job(&self) -> String {
        let job_id = generate_job_id();
        let now = now_rfc3339();
        self.create(job_id.clone(), now);
        job_id
    }

    /// Register a new job in `Pending` state. Returns its initial `JobStatus`.
    pub fn create(&self, job_id: String, timestamp: String) -> JobStatus {
        let status = JobStatus {
            job_id: job_id.clone(),
            state: JobState::Pending,
            created_at: timestamp.clone(),
            updated_at: timestamp,
            result: None,
            error: None,
        };
        self.jobs.insert(job_id, status.clone());
        status
    }

    /// Retrieve the current status of a job by ID.
    pub fn get(&self, job_id: &str) -> Option<JobStatus> {
        self.jobs.get(job_id)
    }

    /// Transition a job to the `Running` state.
    pub fn set_running(&self, job_id: &str, timestamp: String) {
        if let Some(mut status) = self.jobs.get(job_id) {
            status.state = JobState::Running;
            status.updated_at = timestamp;
            self.jobs.insert(job_id.to_string(), status);
        }
    }

    /// Mark a job as `Completed` and store its result.
    pub fn complete(&self, job_id: &str, result: serde_json::Value, timestamp: String) {
        if let Some(mut status) = self.jobs.get(job_id) {
            status.state = JobState::Completed;
            status.result = Some(result);
            status.updated_at = timestamp;
            self.jobs.insert(job_id.to_string(), status);
        }
    }

    /// Mark a job as `Failed` and store the error message.
    pub fn fail(&self, job_id: &str, error: String, timestamp: String) {
        if let Some(mut status) = self.jobs.get(job_id) {
            status.state = JobState::Failed;
            status.error = Some(error);
            status.updated_at = timestamp;
            self.jobs.insert(job_id.to_string(), status);
        }
    }
}

/// Generate a new unique job ID (UUID v4).
pub fn generate_job_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Return the current time formatted as RFC 3339 (ISO 8601).
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_job_is_pending() {
        let store = JobStore::new();
        let ts = "2026-05-01T12:00:00Z".to_string();
        let status = store.create("job-1".to_string(), ts.clone());
        assert_eq!(status.state, JobState::Pending);
        assert_eq!(status.job_id, "job-1");
        assert_eq!(status.created_at, ts);
    }

    #[test]
    fn test_get_existing_job() {
        let store = JobStore::new();
        store.create("job-2".to_string(), "2026-05-01T12:00:00Z".to_string());
        let got = store.get("job-2");
        assert!(got.is_some());
        assert_eq!(got.unwrap().job_id, "job-2");
    }

    #[test]
    fn test_get_missing_job_returns_none() {
        let store = JobStore::new();
        assert!(store.get("nope").is_none());
    }

    #[test]
    fn test_set_running_transitions_state() {
        let store = JobStore::new();
        store.create("job-3".to_string(), "2026-05-01T12:00:00Z".to_string());
        store.set_running("job-3", "2026-05-01T12:00:01Z".to_string());
        let status = store.get("job-3").unwrap();
        assert_eq!(status.state, JobState::Running);
    }

    #[test]
    fn test_complete_stores_result() {
        let store = JobStore::new();
        store.create("job-4".to_string(), "2026-05-01T12:00:00Z".to_string());
        store.complete(
            "job-4",
            serde_json::json!({"content": "hello"}),
            "2026-05-01T12:00:02Z".to_string(),
        );
        let status = store.get("job-4").unwrap();
        assert_eq!(status.state, JobState::Completed);
        assert!(status.result.is_some());
    }

    #[test]
    fn test_fail_stores_error() {
        let store = JobStore::new();
        store.create("job-5".to_string(), "2026-05-01T12:00:00Z".to_string());
        store.fail(
            "job-5",
            "OCR unavailable".to_string(),
            "2026-05-01T12:00:03Z".to_string(),
        );
        let status = store.get("job-5").unwrap();
        assert_eq!(status.state, JobState::Failed);
        assert_eq!(status.error.as_deref(), Some("OCR unavailable"));
    }

    #[test]
    fn test_generate_job_id_is_valid_uuid() {
        let id = generate_job_id();
        assert!(!id.is_empty());
        assert!(
            uuid::Uuid::parse_str(&id).is_ok(),
            "generated job ID must be a valid UUID: {id}"
        );
    }

    #[test]
    fn test_create_job_helper() {
        let store = JobStore::new();
        let id = store.create_job();
        assert!(!id.is_empty());
        let status = store.get(&id).expect("job must be created");
        assert_eq!(status.state, JobState::Pending);
    }
}
