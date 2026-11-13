use crate::core::config::{ExtractionConfig, PostProcessorConfig};
use crate::plugins::{Plugin, PostProcessor, ProcessingStage, register_post_processor, unregister_post_processor};
use crate::types::ExtractedDocument;
use std::sync::Arc;
use std::time::{Duration, Instant};

const LIMIT_SECS: u64 = 1;
const FAILSAFE: Duration = Duration::from_secs(4);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const WAIT: &str = "entry-deadline-wait";
const SKIPPED: &str = "entry-deadline-skipped";

struct DeadlineProcessor {
    name: &'static str,
    stage: ProcessingStage,
}

impl Plugin for DeadlineProcessor {
    fn name(&self) -> &str {
        self.name
    }
}

#[async_trait::async_trait]
impl PostProcessor for DeadlineProcessor {
    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }

    async fn process(&self, result: &mut ExtractedDocument, config: &ExtractionConfig) -> crate::Result<()> {
        if self.name == SKIPPED {
            result.content.push_str(SKIPPED);
            return Ok(());
        }

        let token = config.cancel_token.as_ref().expect("timed extraction provides a token");
        let started = Instant::now();
        // The outer Tokio timeout cannot poll while this processor is in flight. ~keep
        while !token.time_limit_reached() && started.elapsed() < FAILSAFE {
            std::thread::sleep(POLL_INTERVAL);
        }
        assert!(token.time_limit_reached(), "entry point never armed the deadline");
        assert!(!token.stop_requested(), "the deadline must fire before timeout grace");
        Ok(())
    }
}

struct Registered(&'static str);

impl Registered {
    fn new(name: &'static str, stage: ProcessingStage) -> Self {
        register_post_processor(Arc::new(DeadlineProcessor { name, stage })).unwrap();
        Self(name)
    }
}

impl Drop for Registered {
    fn drop(&mut self) {
        unregister_post_processor(self.0).expect("remove test processor");
    }
}

async fn assert_entry_point_deadline(file: bool) {
    let _wait = Registered::new(WAIT, ProcessingStage::Early);
    let _skipped = Registered::new(SKIPPED, ProcessingStage::Late);
    let config = ExtractionConfig {
        extraction_timeout_secs: Some(LIMIT_SECS),
        use_cache: false,
        postprocessor: Some(PostProcessorConfig {
            enabled_processors: Some(vec![WAIT.into(), SKIPPED.into()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = "Deadline test content.";
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("deadline.txt");
    std::fs::write(&path, input).unwrap();

    let started = Instant::now();
    let result = if file {
        Box::pin(super::super::extract_file(&path, Some("text/plain"), &config)).await
    } else {
        Box::pin(super::super::extract_bytes(input.as_bytes(), "text/plain", &config)).await
    }
    .expect("deadline returns the available document");
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_secs(LIMIT_SECS) && elapsed < FAILSAFE,
        "{elapsed:?}"
    );
    assert_eq!(result.content.trim(), input);
    assert!(
        result.processing_warnings.iter().any(|warning| {
            warning.source == "time_limit" && warning.message.contains(&format!("post-processors {SKIPPED}"))
        }),
        "{:?}",
        result.processing_warnings
    );
}

#[tokio::test]
#[serial_test::serial]
async fn bytes_entry_point_arms_the_processor_deadline() {
    Box::pin(assert_entry_point_deadline(false)).await;
}

#[tokio::test]
#[serial_test::serial]
async fn file_entry_point_arms_the_processor_deadline() {
    Box::pin(assert_entry_point_deadline(true)).await;
}
