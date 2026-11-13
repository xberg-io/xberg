use crate::core::config::{ExtractionConfig, PostProcessorConfig};
use crate::plugins::{Plugin, PostProcessor, ProcessingStage, register_post_processor, unregister_post_processor};
use crate::types::ExtractedDocument;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const LIMIT_SECS: u64 = 1;
const FAILSAFE: Duration = Duration::from_secs(4);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const WAIT: &str = "entry-deadline-wait";
const SKIPPED: &str = "entry-deadline-skipped";

struct DeadlineProcessor {
    name: &'static str,
    stage: ProcessingStage,
    observed_deadline: Arc<AtomicBool>,
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
        assert!(
            !token.is_cancelled(),
            "the outer timeout must not have cancelled this processor"
        );
        self.observed_deadline.store(true, Ordering::Relaxed);
        Ok(())
    }
}

struct Registered(&'static str);

impl Registered {
    fn new(name: &'static str, stage: ProcessingStage, observed_deadline: &Arc<AtomicBool>) -> Self {
        register_post_processor(Arc::new(DeadlineProcessor {
            name,
            stage,
            observed_deadline: Arc::clone(observed_deadline),
        }))
        .unwrap();
        Self(name)
    }
}

impl Drop for Registered {
    fn drop(&mut self) {
        unregister_post_processor(self.0).expect("remove test processor");
    }
}

async fn assert_entry_point_deadline(file: bool) {
    let observed_deadline = Arc::new(AtomicBool::new(false));
    let _wait = Registered::new(WAIT, ProcessingStage::Early, &observed_deadline);
    let _skipped = Registered::new(SKIPPED, ProcessingStage::Late, &observed_deadline);
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
    };
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_secs(LIMIT_SECS) && elapsed < FAILSAFE,
        "{elapsed:?}"
    );
    assert!(
        observed_deadline.load(Ordering::Relaxed),
        "processor must observe the armed deadline before the outer timeout returns: {result:?}"
    );
    // File extraction can yield after the processor observes the deadline. With no active
    // page loop, the outer timeout can then return an error instead of the document. ~keep
    let result = match result {
        Err(crate::XbergError::Timeout { limit_ms, .. }) if file => {
            assert_eq!(limit_ms, LIMIT_SECS * 1000);
            return;
        }
        other => other.expect("deadline returns the available document"),
    };
    assert_eq!(result.content.trim(), input);
    assert!(
        result.processing_warnings.iter().any(|warning| {
            warning.source == "time_limit" && warning.message.contains(&format!("post-processors {SKIPPED}"))
        }),
        "{:?}",
        result.processing_warnings
    );
}

fn run_isolated_deadline_case(name: &str, file: bool) {
    const CHILD: &str = "XBERG_DEADLINE_TEST_CHILD";
    let module = module_path!().split_once("::").expect("crate module prefix").1;
    let qualified = format!("{module}::{name}");
    if std::env::var(CHILD).as_deref() == Ok(qualified.as_str()) {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(Box::pin(assert_entry_point_deadline(file)));
        return;
    }

    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", &qualified, "--nocapture"])
        .env(CHILD, &qualified)
        .env("RUST_MIN_STACK", "16777216")
        .output()
        .expect("run isolated deadline test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected = format!("test {qualified} ... ok");
    assert!(
        output.status.success()
            && stdout.lines().any(|line| line == "running 1 test")
            && stdout.lines().any(|line| line == expected),
        "expected exactly one successful child test ({qualified}): {stdout}\n{stderr}"
    );
}

#[test]
fn bytes_entry_point_arms_the_processor_deadline() {
    run_isolated_deadline_case("bytes_entry_point_arms_the_processor_deadline", false);
}

#[test]
fn file_entry_point_arms_the_processor_deadline() {
    run_isolated_deadline_case("file_entry_point_arms_the_processor_deadline", true);
}
