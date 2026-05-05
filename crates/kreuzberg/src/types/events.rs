use serde::{Deserialize, Serialize};

/// The state of an async extraction job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// The job has been accepted but not yet started.
    Pending,
    /// The job is currently being processed.
    Running,
    /// The job completed successfully.
    Completed,
    /// The job terminated with an error.
    Failed,
}

/// The status of an async extraction job returned by `GET /jobs/{id}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct JobStatus {
    /// Unique identifier of the job.
    pub job_id: String,
    /// Current lifecycle state of the job.
    pub state: JobState,
    /// ISO 8601 timestamp when the job was created.
    pub created_at: String,
    /// ISO 8601 timestamp of the last state change.
    pub updated_at: String,
    /// The extraction result, present only when `state == completed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message, present only when `state == failed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_state_serializes_to_snake_case() {
        assert_eq!(serde_json::to_string(&JobState::Pending).unwrap(), "\"pending\"");
        assert_eq!(serde_json::to_string(&JobState::Running).unwrap(), "\"running\"");
        assert_eq!(serde_json::to_string(&JobState::Completed).unwrap(), "\"completed\"");
        assert_eq!(serde_json::to_string(&JobState::Failed).unwrap(), "\"failed\"");
    }

    #[test]
    fn test_job_status_completed_serialization() {
        let status = JobStatus {
            job_id: "job-abc".to_string(),
            state: JobState::Completed,
            created_at: "2026-05-01T09:00:00Z".to_string(),
            updated_at: "2026-05-01T09:01:30Z".to_string(),
            result: Some(serde_json::json!({"text": "Hello, world!", "pages": 1})),
            error: None,
        };

        let json = serde_json::to_string(&status).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["job_id"], "job-abc");
        assert_eq!(parsed["state"], "completed");
        assert_eq!(parsed["created_at"], "2026-05-01T09:00:00Z");
        assert_eq!(parsed["updated_at"], "2026-05-01T09:01:30Z");
        assert_eq!(parsed["result"]["text"], "Hello, world!");
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn test_job_status_failed_serialization() {
        let status = JobStatus {
            job_id: "job-def".to_string(),
            state: JobState::Failed,
            created_at: "2026-05-01T10:00:00Z".to_string(),
            updated_at: "2026-05-01T10:00:05Z".to_string(),
            result: None,
            error: Some("Unsupported file format: application/x-unknown".to_string()),
        };

        let json = serde_json::to_string(&status).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["job_id"], "job-def");
        assert_eq!(parsed["state"], "failed");
        assert_eq!(parsed["error"], "Unsupported file format: application/x-unknown");
        assert!(parsed.get("result").is_none());
    }

    #[test]
    fn test_job_status_optional_fields_omitted_when_none() {
        let status = JobStatus {
            job_id: "job-ghi".to_string(),
            state: JobState::Pending,
            created_at: "2026-05-01T11:00:00Z".to_string(),
            updated_at: "2026-05-01T11:00:00Z".to_string(),
            result: None,
            error: None,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.contains("result"));
        assert!(!json.contains("error"));

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["state"], "pending");
        assert!(parsed.get("result").is_none());
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn test_job_status_result_and_error_omitted_when_none() {
        let status = JobStatus {
            job_id: "job-jkl".to_string(),
            state: JobState::Running,
            created_at: "2026-05-01T12:00:00Z".to_string(),
            updated_at: "2026-05-01T12:00:10Z".to_string(),
            result: None,
            error: None,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.contains("\"result\""));
        assert!(!json.contains("\"error\""));

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["state"], "running");
        assert!(parsed.get("result").is_none());
        assert!(parsed.get("error").is_none());
    }
}
