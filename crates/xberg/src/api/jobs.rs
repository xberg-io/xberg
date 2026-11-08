//! In-memory job store for async extraction polling.

use std::collections::HashMap;
use std::time::Duration;

use moka::sync::Cache;
use parking_lot::Mutex;

use crate::api::types::{JobState, JobStatus};
use crate::cancellation::CancellationToken;

/// Time-to-live for completed/failed/cancelled jobs (5 minutes).
const JOB_TTL: Duration = Duration::from_secs(300);

/// Maximum number of finished jobs held in the cache.
const MAX_FINISHED_JOBS: u64 = 10_000;

/// Maximum number of jobs in Pending or Running state at any one time.
pub const MAX_ACTIVE_JOBS: usize = 100;

/// A pending or running job and the token that cancels its extraction.
struct ActiveJob {
    status: JobStatus,
    token: CancellationToken,
}

/// Thread-safe in-memory store for async extraction jobs.
///
/// Pending and running jobs live in `active`, which never expires or evicts them.
/// A job moves to `finished` once, when it completes, fails or is cancelled; that
/// [`moka::sync::Cache`] drops it after 5 minutes or under capacity pressure.
/// Server restarts clear all jobs.
pub struct JobStore {
    active: Mutex<HashMap<String, ActiveJob>>,
    finished: Cache<String, JobStatus>,
}

impl Default for JobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl JobStore {
    /// Create a new empty job store with default TTL and capacity.
    pub fn new() -> Self {
        Self::with_limits(JOB_TTL, MAX_FINISHED_JOBS)
    }

    /// Create a store that keeps at most `finished_capacity` finished jobs for `ttl`.
    fn with_limits(ttl: Duration, finished_capacity: u64) -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
            finished: Cache::builder()
                .max_capacity(finished_capacity)
                .time_to_live(ttl)
                .build(),
        }
    }

    /// Return the number of jobs currently in Pending or Running state.
    pub fn active_count(&self) -> usize {
        self.active.lock().len()
    }

    /// Create a new job in the store and return its ID, or `None` when
    /// [`MAX_ACTIVE_JOBS`] jobs are already pending or running.
    pub fn create_job(&self) -> Option<String> {
        let job_id = generate_job_id();
        self.create(job_id.clone(), now_rfc3339())?;
        Some(job_id)
    }

    /// Return the cancellation token of a pending or running job.
    ///
    /// Pass the returned token's clone to the extraction call (via
    /// `ExtractionConfig::cancel_token`) so it observes cancellation at its
    /// next checkpoint.
    pub fn cancellation_token(&self, job_id: &str) -> Option<CancellationToken> {
        self.active.lock().get(job_id).map(|job| job.token.clone())
    }

    /// Register a new job in `Pending` state and return its initial `JobStatus`.
    ///
    /// The capacity check and the insert happen under one lock, so concurrent
    /// submissions cannot take the store above [`MAX_ACTIVE_JOBS`]. Returns `None`
    /// when the store is full.
    pub fn create(&self, job_id: String, timestamp: String) -> Option<JobStatus> {
        let status = JobStatus {
            job_id: job_id.clone(),
            state: JobState::Pending,
            created_at: timestamp.clone(),
            updated_at: timestamp,
            result: None,
            error: None,
        };
        let job = ActiveJob {
            status: status.clone(),
            token: CancellationToken::default(),
        };
        let mut active = self.active.lock();
        if active.len() >= MAX_ACTIVE_JOBS {
            return None;
        }
        active.insert(job_id, job);
        Some(status)
    }

    /// Retrieve the current status of a job by ID.
    pub fn get(&self, job_id: &str) -> Option<JobStatus> {
        // `active` first: a job leaves `active` only after it is in `finished`, so this
        // order never misses a job that finishes between the two lookups. ~keep
        if let Some(job) = self.active.lock().get(job_id) {
            return Some(job.status.clone());
        }
        self.finished.get(job_id)
    }

    /// Transition a live job to the `Running` state.
    ///
    /// A finished job is no longer in `active`, so this is a no-op for it and a job
    /// cancelled before its task started stays cancelled.
    pub fn set_running(&self, job_id: &str, timestamp: String) {
        if let Some(job) = self.active.lock().get_mut(job_id) {
            job.status.state = JobState::Running;
            job.status.updated_at = timestamp;
        }
    }

    /// Mark a pending or running job as `Completed` and store its result.
    ///
    /// A no-op for a job that already finished, so a late-arriving result from a
    /// cancelled extraction cannot clobber the `Cancelled` state.
    pub fn complete(&self, job_id: &str, result: serde_json::Value, timestamp: String) {
        self.finish(job_id, |status| {
            status.state = JobState::Completed;
            status.result = Some(result);
            status.updated_at = timestamp;
        });
    }

    /// Mark a pending or running job as `Failed` and store the error message.
    ///
    /// A no-op for a job that already finished, like [`JobStore::complete`].
    pub fn fail(&self, job_id: &str, error: String, timestamp: String) {
        self.finish(job_id, |status| {
            status.state = JobState::Failed;
            status.error = Some(error);
            status.updated_at = timestamp;
        });
    }

    /// Move a live job to `finished`. A no-op once the job is no longer active.
    fn finish(&self, job_id: &str, apply: impl FnOnce(&mut JobStatus)) {
        let mut active = self.active.lock();
        let Some(ActiveJob { mut status, .. }) = active.remove(job_id) else {
            return;
        };
        apply(&mut status);
        // Inserted while `active` is still locked; see `get`. ~keep
        self.finished.insert(job_id.to_string(), status);
    }

    /// Cancel a pending or running job.
    ///
    /// Fires the job's [`CancellationToken`] so a running extraction observes
    /// it at its next checkpoint. Jobs that already reached a terminal state
    /// (`Completed`, `Failed`, or `Cancelled`) cannot be cancelled again.
    pub fn cancel(&self, job_id: &str, timestamp: String) -> CancelOutcome {
        let mut active = self.active.lock();
        if let Some(ActiveJob { mut status, token }) = active.remove(job_id) {
            token.cancel();
            status.state = JobState::Cancelled;
            status.updated_at = timestamp;
            self.finished.insert(job_id.to_string(), status.clone());
            return CancelOutcome::Cancelled(status);
        }
        drop(active);
        match self.finished.get(job_id) {
            Some(status) => CancelOutcome::Conflict(status),
            None => CancelOutcome::NotFound,
        }
    }
}

/// Outcome of a [`JobStore::cancel`] call.
#[derive(Debug, Clone)]
pub enum CancelOutcome {
    /// The job was pending or running and is now cancelled.
    Cancelled(JobStatus),
    /// The job already reached a terminal state and cannot be cancelled.
    Conflict(JobStatus),
    /// No job exists with this ID (unknown or expired).
    NotFound,
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

    use crate::api::types::JobState;

    #[test]
    fn test_create_job_is_pending() {
        let store = JobStore::new();
        let ts = "2026-05-01T12:00:00Z".to_string();
        let status = store.create("job-1".to_string(), ts.clone()).expect("slot available");
        assert_eq!(status.state, JobState::Pending);
        assert_eq!(status.job_id, "job-1");
        assert_eq!(status.created_at, ts);
    }

    #[test]
    fn test_get_existing_job() {
        let store = JobStore::new();
        store
            .create("job-2".to_string(), "2026-05-01T12:00:00Z".to_string())
            .expect("slot available");
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
        store
            .create("job-3".to_string(), "2026-05-01T12:00:00Z".to_string())
            .expect("slot available");
        store.set_running("job-3", "2026-05-01T12:00:01Z".to_string());
        let status = store.get("job-3").unwrap();
        assert_eq!(status.state, JobState::Running);
    }

    #[test]
    fn test_complete_stores_result() {
        let store = JobStore::new();
        store
            .create("job-4".to_string(), "2026-05-01T12:00:00Z".to_string())
            .expect("slot available");
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
        store
            .create("job-5".to_string(), "2026-05-01T12:00:00Z".to_string())
            .expect("slot available");
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
    fn test_cancel_pending_job() {
        let store = JobStore::new();
        let job_id = store.create_job().expect("slot available");
        assert_eq!(store.active_count(), 1);

        match store.cancel(&job_id, "2026-05-01T12:00:01Z".to_string()) {
            CancelOutcome::Cancelled(status) => assert_eq!(status.state, JobState::Cancelled),
            other => panic!("expected Cancelled, got {other:?}"),
        }
        assert_eq!(store.get(&job_id).unwrap().state, JobState::Cancelled);
        assert_eq!(store.active_count(), 0);
    }

    #[test]
    fn test_cancel_running_job_fires_token() {
        let store = JobStore::new();
        let job_id = store.create_job().expect("slot available");
        store.set_running(&job_id, "2026-05-01T12:00:01Z".to_string());
        let token = store.cancellation_token(&job_id).expect("token registered on create");
        assert!(!token.is_cancelled());

        match store.cancel(&job_id, "2026-05-01T12:00:02Z".to_string()) {
            CancelOutcome::Cancelled(status) => assert_eq!(status.state, JobState::Cancelled),
            other => panic!("expected Cancelled, got {other:?}"),
        }
        assert!(token.is_cancelled(), "cancelling a running job must fire its token");
    }

    #[test]
    fn test_cancel_completed_job_is_conflict() {
        let store = JobStore::new();
        let job_id = store.create_job().expect("slot available");
        store.complete(
            &job_id,
            serde_json::json!({"content": "done"}),
            "2026-05-01T12:00:01Z".to_string(),
        );

        match store.cancel(&job_id, "2026-05-01T12:00:02Z".to_string()) {
            CancelOutcome::Conflict(status) => assert_eq!(status.state, JobState::Completed),
            other => panic!("expected Conflict, got {other:?}"),
        }
        assert_eq!(
            store.get(&job_id).unwrap().state,
            JobState::Completed,
            "a conflicting cancel must not alter the job's state"
        );
    }

    #[test]
    fn test_cancel_failed_job_is_conflict() {
        let store = JobStore::new();
        let job_id = store.create_job().expect("slot available");
        store.fail(&job_id, "boom".to_string(), "2026-05-01T12:00:01Z".to_string());

        match store.cancel(&job_id, "2026-05-01T12:00:02Z".to_string()) {
            CancelOutcome::Conflict(status) => assert_eq!(status.state, JobState::Failed),
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[test]
    fn test_cancel_already_cancelled_job_is_conflict() {
        let store = JobStore::new();
        let job_id = store.create_job().expect("slot available");
        store.cancel(&job_id, "2026-05-01T12:00:01Z".to_string());

        match store.cancel(&job_id, "2026-05-01T12:00:02Z".to_string()) {
            CancelOutcome::Conflict(status) => assert_eq!(status.state, JobState::Cancelled),
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[test]
    fn test_cancel_missing_job_returns_not_found() {
        let store = JobStore::new();
        assert!(matches!(
            store.cancel("nope", "2026-05-01T12:00:00Z".to_string()),
            CancelOutcome::NotFound
        ));
    }

    #[test]
    fn test_complete_after_cancel_is_noop() {
        let store = JobStore::new();
        let job_id = store.create_job().expect("slot available");
        store.cancel(&job_id, "2026-05-01T12:00:01Z".to_string());

        store.complete(
            &job_id,
            serde_json::json!({"content": "late"}),
            "2026-05-01T12:00:02Z".to_string(),
        );

        let status = store.get(&job_id).unwrap();
        assert_eq!(
            status.state,
            JobState::Cancelled,
            "a late completion must not override a cancelled job"
        );
        assert!(status.result.is_none());
    }

    #[test]
    fn test_fail_after_cancel_is_noop() {
        let store = JobStore::new();
        let job_id = store.create_job().expect("slot available");
        store.cancel(&job_id, "2026-05-01T12:00:01Z".to_string());

        store.fail(&job_id, "late error".to_string(), "2026-05-01T12:00:02Z".to_string());

        let status = store.get(&job_id).unwrap();
        assert_eq!(
            status.state,
            JobState::Cancelled,
            "a late failure must not override a cancelled job"
        );
        assert!(status.error.is_none());
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
        let id = store.create_job().expect("slot available");
        assert!(!id.is_empty());
        let status = store.get(&id).expect("job must be created");
        assert_eq!(status.state, JobState::Pending);
    }

    #[test]
    fn test_create_job_refuses_when_store_is_full() {
        let store = JobStore::new();
        let ids: Vec<String> = (0..MAX_ACTIVE_JOBS)
            .map(|_| store.create_job().expect("slot available"))
            .collect();
        assert!(store.create_job().is_none());
        assert_eq!(store.active_count(), MAX_ACTIVE_JOBS);

        store.complete(&ids[0], serde_json::json!({}), "2026-05-01T12:00:01Z".to_string());
        assert!(store.create_job().is_some());
        assert_eq!(store.active_count(), MAX_ACTIVE_JOBS);
    }

    #[test]
    fn test_job_ttl_counts_from_finish() {
        let store = JobStore::with_limits(Duration::from_millis(50), MAX_FINISHED_JOBS);
        let job_id = store.create_job().expect("slot available");
        store.set_running(&job_id, "2026-05-01T12:00:01Z".to_string());
        std::thread::sleep(Duration::from_millis(150));

        assert_eq!(store.get(&job_id).unwrap().state, JobState::Running);
        assert_eq!(store.active_count(), 1);
        assert!(store.cancellation_token(&job_id).is_some());

        store.complete(
            &job_id,
            serde_json::json!({"content": "done"}),
            "2026-05-01T12:00:02Z".to_string(),
        );
        assert_eq!(store.get(&job_id).unwrap().state, JobState::Completed);
        assert_eq!(store.active_count(), 0);

        std::thread::sleep(Duration::from_millis(150));
        assert!(store.get(&job_id).is_none());
    }

    #[test]
    fn test_set_running_after_cancel_does_not_resurrect() {
        let store = JobStore::new();
        let job_id = store.create_job().expect("slot available");
        store.cancel(&job_id, "2026-05-01T12:00:01Z".to_string());

        store.set_running(&job_id, "2026-05-01T12:00:02Z".to_string());
        store.complete(&job_id, serde_json::json!({}), "2026-05-01T12:00:03Z".to_string());

        let status = store.get(&job_id).unwrap();
        assert_eq!(status.state, JobState::Cancelled);
        assert!(status.result.is_none());
        assert_eq!(store.active_count(), 0);
    }

    #[test]
    fn test_finished_job_does_not_transition_again() {
        let store = JobStore::new();
        let job_id = store.create_job().expect("slot available");
        store.complete(&job_id, serde_json::json!({}), "2026-05-01T12:00:01Z".to_string());

        store.fail(&job_id, "late error".to_string(), "2026-05-01T12:00:02Z".to_string());
        store.set_running(&job_id, "2026-05-01T12:00:03Z".to_string());

        let status = store.get(&job_id).unwrap();
        assert_eq!(status.state, JobState::Completed);
        assert!(status.error.is_none());
    }

    #[test]
    fn test_finished_capacity_never_evicts_running_job() {
        let store = JobStore::with_limits(JOB_TTL, 2);
        let running = store.create_job().expect("slot available");
        store.set_running(&running, "2026-05-01T12:00:01Z".to_string());
        let token = store.cancellation_token(&running).unwrap();

        for _ in 0..100 {
            let job_id = store.create_job().expect("slot available");
            store.complete(&job_id, serde_json::json!({}), "2026-05-01T12:00:02Z".to_string());
        }
        store.finished.run_pending_tasks();

        assert!(store.finished.entry_count() <= 2);
        assert_eq!(store.get(&running).unwrap().state, JobState::Running);
        assert_eq!(store.active_count(), 1);
        assert!(matches!(
            store.cancel(&running, "2026-05-01T12:00:03Z".to_string()),
            CancelOutcome::Cancelled(_)
        ));
        assert!(token.is_cancelled());
    }
}

#[test]
fn test_create_job_concurrent_uniqueness() {
    use std::sync::Arc;
    use std::thread;

    let store = Arc::new(JobStore::new());
    let mut handles = vec![];

    for _ in 0..100 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || store_clone.create_job().expect("slot available")));
    }

    let mut job_ids = std::collections::HashSet::new();
    for handle in handles {
        let id = handle.join().unwrap();
        assert!(job_ids.insert(id.clone()), "Duplicate job ID generated: {}", id);
    }

    assert_eq!(job_ids.len(), 100);
}

/// With one slot left, concurrent submissions admit exactly one job: the capacity
/// check and the insert happen under one lock, so the store never exceeds
/// `MAX_ACTIVE_JOBS`.
#[test]
fn test_concurrent_admission_fills_last_slot_once() {
    use std::sync::{Arc, Barrier};
    use std::thread;

    for _ in 0..50 {
        let store = Arc::new(JobStore::new());
        for _ in 1..MAX_ACTIVE_JOBS {
            store.create_job().expect("slot available");
        }

        let contenders = 16;
        let barrier = Arc::new(Barrier::new(contenders));
        let handles: Vec<_> = (0..contenders)
            .map(|_| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    store.create_job()
                })
            })
            .collect();

        let admitted = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(Option::is_some)
            .count();

        assert_eq!(admitted, 1, "exactly one submission may take the last slot");
        assert_eq!(store.active_count(), MAX_ACTIVE_JOBS);
    }
}

/// Regression test for a race between `cancel()` and `complete()` on the same job:
/// exactly one of them finishes the job, the active slot is released once, and when
/// `cancel()` wins it has fired the job's token.
#[test]
fn test_concurrent_cancel_and_complete_finish_once() {
    use std::sync::Arc;
    use std::thread;

    for _ in 0..200 {
        let store = Arc::new(JobStore::new());
        let job_id = store.create_job().expect("slot available");
        let token = store.cancellation_token(&job_id).unwrap();

        let store_a = Arc::clone(&store);
        let job_id_a = job_id.clone();
        let cancel_thread = thread::spawn(move || store_a.cancel(&job_id_a, "2026-05-01T12:00:01Z".to_string()));

        let store_b = Arc::clone(&store);
        let job_id_b = job_id.clone();
        let complete_thread = thread::spawn(move || {
            store_b.complete(
                &job_id_b,
                serde_json::json!({"content": "done"}),
                "2026-05-01T12:00:01Z".to_string(),
            );
        });

        let cancel_outcome = cancel_thread.join().unwrap();
        complete_thread.join().unwrap();

        assert_eq!(store.active_count(), 0);
        let final_state = store.get(&job_id).unwrap().state;
        match cancel_outcome {
            CancelOutcome::Cancelled(_) => {
                assert_eq!(final_state, JobState::Cancelled);
                assert!(token.is_cancelled(), "a successful cancel must signal the worker");
            }
            CancelOutcome::Conflict(_) => assert_eq!(final_state, JobState::Completed),
            CancelOutcome::NotFound => panic!("the job must be found in one of the stores"),
        }
    }
}

/// A poll that runs while the job finishes must never see it missing: `get()` reads
/// `active` first, and a job leaves `active` only after it is in `finished`.
#[test]
fn test_concurrent_poll_never_misses_finishing_job() {
    use std::sync::{Arc, Barrier};
    use std::thread;

    for _ in 0..200 {
        let store = Arc::new(JobStore::new());
        let job_id = store.create_job().expect("slot available");
        let barrier = Arc::new(Barrier::new(2));

        let store_a = Arc::clone(&store);
        let job_id_a = job_id.clone();
        let barrier_a = Arc::clone(&barrier);
        let finish_thread = thread::spawn(move || {
            barrier_a.wait();
            store_a.complete(&job_id_a, serde_json::json!({}), "2026-05-01T12:00:01Z".to_string());
        });

        let store_b = Arc::clone(&store);
        let job_id_b = job_id.clone();
        let poll_thread = thread::spawn(move || {
            barrier.wait();
            loop {
                let status = store_b
                    .get(&job_id_b)
                    .expect("a live or finished job is always visible");
                if status.state == JobState::Completed {
                    break;
                }
            }
        });

        finish_thread.join().unwrap();
        poll_thread.join().unwrap();
    }
}
