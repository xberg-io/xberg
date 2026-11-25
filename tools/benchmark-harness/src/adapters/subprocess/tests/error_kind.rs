//! Tests for `error_to_error_kind`'s classification heuristics and `finish_measured_command`'s
//! signal-termination error reporting.

use crate::Error;
use crate::monitoring::ResourceStats;
use crate::types::ErrorKind;
use std::time::Duration;

use super::super::SubprocessAdapter;
use super::super::support::{MeasuredCommandOutcome, error_to_error_kind};

#[test]
fn test_error_to_error_kind_mapping() {
    assert_eq!(error_to_error_kind(&Error::Timeout("test".into())), ErrorKind::Timeout);
    assert_eq!(
        error_to_error_kind(&Error::FrameworkError("test".into())),
        ErrorKind::FrameworkError
    );
    assert_eq!(
        error_to_error_kind(&Error::EmptyContent("test".into())),
        ErrorKind::EmptyContent
    );
    assert_eq!(
        error_to_error_kind(&Error::LayoutFailed("test".into())),
        ErrorKind::LayoutFailed
    );
    assert_eq!(
        error_to_error_kind(&Error::Benchmark("test".into())),
        ErrorKind::HarnessError
    );

    assert_eq!(
        error_to_error_kind(&Error::Benchmark("torch.PP-OCRv6 not found".into())),
        ErrorKind::ConfigSetupError
    );
    assert_eq!(
        error_to_error_kind(&Error::Benchmark("partition_X not available".into())),
        ErrorKind::ConfigSetupError
    );
    assert_eq!(
        error_to_error_kind(&Error::Benchmark("tessdata not found".into())),
        ErrorKind::ConfigSetupError
    );
    assert_eq!(
        error_to_error_kind(&Error::Config("Module not installed".into())),
        ErrorKind::ConfigSetupError
    );
}

/// Regression test for Bug C: a framework-emitted crash (captured in the subprocess's
/// stderr and embedded into the `Error::Benchmark` message by `execute_subprocess`) must be
/// classified as `FrameworkError`, not `HarnessError` — the framework failed, not the
/// harness. Uses the exact stderr shape `docling_extract.py` produces on an uncaught
/// exception: `Error extracting with Docling: Unsupported configuration: ...`.
#[test]
fn test_error_to_error_kind_framework_crash_stderr_is_framework_error() {
    let msg = "Subprocess failed with exit status: 1\nstderr: Error extracting with Docling: \
                    Unsupported configuration: torch.PP-OCRv6.det.small"
        .to_string();
    assert_eq!(error_to_error_kind(&Error::Benchmark(msg)), ErrorKind::FrameworkError);
}

#[cfg(unix)]
#[test]
fn signaled_subprocess_error_reports_signal() {
    let output = std::process::Command::new("sh")
        .args(["-c", "kill -TERM $$"])
        .output()
        .expect("signal test subprocess should start");
    let measured = MeasuredCommandOutcome {
        output: Some(output),
        duration: Duration::ZERO,
        resource_stats: ResourceStats::default(),
        error: None,
    };

    let execution = SubprocessAdapter::finish_measured_command(measured, "Batch subprocess");
    let message = execution
        .error
        .expect("signaled subprocess should report an error")
        .to_string();
    let expected_signal = format!("signal: {}", libc::SIGTERM);
    assert!(
        message.contains(&expected_signal),
        "unexpected subprocess error: {message}"
    );
}

/// A genuine harness-side failure (e.g. we failed to spawn the subprocess at all) must stay
/// `HarnessError` — the framework-crash heuristic must not over-reach.
#[test]
fn test_error_to_error_kind_harness_spawn_failure_stays_harness_error() {
    let msg = "Failed to spawn subprocess 'docling-cli' with args []: No such file or directory".to_string();
    assert_eq!(error_to_error_kind(&Error::Benchmark(msg)), ErrorKind::HarnessError);
}
