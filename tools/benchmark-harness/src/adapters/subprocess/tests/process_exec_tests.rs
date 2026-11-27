//! Tests for the low-level measured-command primitives: timeout handling, resource-stats
//! bookkeeping, RSS measurability, and stderr-tail surfacing.

use crate::Error;
use crate::types::{MemoryMethod, OutputFormat};
use std::process::Stdio;
use std::time::{Duration, Instant};

use super::super::SubprocessAdapter;

#[cfg(unix)]
#[tokio::test]
async fn subprocess_timeout_kills_and_reaps_process_group() {
    let adapter = SubprocessAdapter::new(
        "timeout-test",
        "sh",
        vec!["-c".to_string(), "sleep 30 & wait".to_string()],
        vec![],
        vec!["pdf".to_string()],
    );
    let input = tempfile::NamedTempFile::new().unwrap();
    let start = Instant::now();

    let execution = adapter
        .execute_subprocess(
            input.path(),
            Duration::from_millis(50),
            false,
            None,
            OutputFormat::Markdown,
        )
        .await
        .unwrap();

    assert!(matches!(execution.error, Some(Error::Timeout(_))));
    assert!(execution.resource_stats.baseline_memory_bytes > 0);
    assert!(execution.resource_stats.peak_memory_bytes > 0);
    assert!(execution.resource_stats.sample_count > 0);
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[cfg(unix)]
#[tokio::test]
async fn measured_command_timer_excludes_pre_spawn_staging() {
    let wall_start = Instant::now();
    tokio::time::sleep(Duration::from_millis(60)).await;
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "sleep 0.02; printf ok"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);

    let outcome = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_secs(1),
        "timer test",
        Duration::from_millis(1),
    )
    .await
    .unwrap();

    assert!(outcome.error.is_none());
    assert!(outcome.output.unwrap().status.success());
    assert!(outcome.resource_stats.baseline_memory_bytes > 0);
    assert!(outcome.resource_stats.peak_memory_bytes > 0);
    assert!(outcome.resource_stats.sample_count > 0);
    assert!(wall_start.elapsed().saturating_sub(outcome.duration) >= Duration::from_millis(40));
}

#[cfg(unix)]
#[tokio::test]
async fn measured_ultrashort_command_is_measured_by_rusage_without_samples() {
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "printf ok"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);

    let outcome = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_secs(1),
        "ultrashort command",
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    assert!(
        outcome.error.is_none(),
        "rusage makes a sub-interval command measurable: {:?}",
        outcome.error
    );
    let output = outcome.output.unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"ok");
    assert!(outcome.resource_stats.baseline_memory_bytes > 0);
    assert_eq!(outcome.resource_stats.sample_count, 0);
    assert!(outcome.resource_stats.peak_memory_bytes > 0);
    assert_eq!(outcome.resource_stats.memory_method, MemoryMethod::RusageMaxRss);
}

/// Shaped like the liteparse wrapper: the measured shell forks a short-lived child that
/// allocates 64 MiB and exits long before the sampler's first sample. The sampler records
/// nothing (the old method would have reported no or a tiny peak); rusage still sees the
/// child's peak because the shell reaped it.
#[cfg(unix)]
#[tokio::test]
async fn rusage_captures_short_lived_grandchild_peak_the_sampler_misses() {
    const SIXTY_MIB: u64 = 60 * 1024 * 1024;
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "python3 -c 'b = b\"x\" * (64 << 20)'; printf done"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);
    let sample_interval = Duration::from_secs(5);

    let outcome = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_secs(30),
        "short-lived allocator",
        sample_interval,
    )
    .await
    .unwrap();

    assert!(outcome.error.is_none(), "unexpected error: {:?}", outcome.error);
    let output = outcome.output.unwrap();
    assert!(
        output.status.success(),
        "python3 child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"done");
    assert!(outcome.duration < sample_interval);
    assert_eq!(
        outcome.resource_stats.sample_count, 0,
        "the sampler must not have observed the child"
    );
    assert_eq!(outcome.resource_stats.memory_method, MemoryMethod::RusageMaxRss);
    assert!(
        outcome.resource_stats.peak_memory_bytes >= SIXTY_MIB,
        "peak {} bytes is below the child's 64 MiB allocation",
        outcome.resource_stats.peak_memory_bytes
    );
    assert_eq!(
        outcome.resource_stats.peak_memory_delta_bytes,
        outcome
            .resource_stats
            .peak_memory_bytes
            .saturating_sub(outcome.resource_stats.baseline_memory_bytes)
    );
}

/// Output larger than a pipe buffer must be drained while the child is reaped, or the child
/// blocks on write and the reap never returns.
#[cfg(unix)]
#[tokio::test]
async fn rusage_path_drains_output_larger_than_a_pipe_buffer() {
    const OUTPUT_BYTES: usize = 1_000_000;
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "head -c 1000000 /dev/zero; head -c 1000000 /dev/zero 1>&2"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);

    let outcome = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_secs(10),
        "large output",
        Duration::from_millis(10),
    )
    .await
    .unwrap();

    assert!(outcome.error.is_none(), "unexpected error: {:?}", outcome.error);
    let output = outcome.output.unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout.len(), OUTPUT_BYTES);
    assert_eq!(output.stderr.len(), OUTPUT_BYTES);
}

#[cfg(unix)]
#[test]
fn ru_maxrss_is_normalised_to_bytes_per_platform() {
    let expected = if cfg!(target_vendor = "apple") {
        2048
    } else {
        2048 * 1024
    };
    assert_eq!(SubprocessAdapter::ru_maxrss_to_bytes(2048), expected);
    assert_eq!(SubprocessAdapter::ru_maxrss_to_bytes(0), 0);
    assert_eq!(SubprocessAdapter::ru_maxrss_to_bytes(-1), 0);
}

#[test]
fn self_reported_peak_is_recorded_but_never_replaces_the_measured_peak() {
    let resource_stats = crate::monitoring::ResourceStats {
        baseline_memory_bytes: 1_000,
        peak_memory_bytes: 50_000,
        peak_memory_delta_bytes: 49_000,
        memory_method: MemoryMethod::RusageMaxRss,
        ..Default::default()
    };

    let metrics = SubprocessAdapter::measured_metrics(&resource_stats, Some(90_000), 12.5);

    assert_eq!(metrics.peak_memory_bytes, 50_000);
    assert_eq!(metrics.peak_memory_delta_bytes, 49_000);
    assert_eq!(metrics.memory_method, MemoryMethod::RusageMaxRss);
    assert_eq!(metrics.self_reported_peak_memory_bytes, Some(90_000));
    assert_eq!(metrics.throughput_bytes_per_sec, 12.5);
}

#[cfg(unix)]
#[tokio::test]
async fn timeout_error_surfaces_child_stderr_tail() {
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "echo XBERG_HANG_SENTINEL 1>&2; sleep 5"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);

    let outcome = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_millis(300),
        "hang probe",
        Duration::from_millis(20),
    )
    .await
    .unwrap();

    let error = outcome.error.expect("a timed-out subprocess must produce an error");
    let Error::Timeout(message) = &error else {
        panic!("expected Error::Timeout, got: {error:?}");
    };
    assert!(
        message.contains("XBERG_HANG_SENTINEL"),
        "timeout error must surface the hung child's stderr tail: {message}"
    );
}

#[cfg(unix)]
#[test]
fn measured_nonzero_exit_preserves_resource_stats() {
    use super::super::support::MeasuredCommandOutcome;
    use crate::monitoring::{ResourceMonitor, ResourceSample};
    use std::os::unix::process::ExitStatusExt;

    let samples = [ResourceSample {
        memory_bytes: 2048,
        vm_size_bytes: 8192,
        page_faults: 0,
        cpu_percent: 0.0,
        timestamp_ms: 1,
    }];
    // The blocked shell is baseline metadata; target RSS may be lower after exec. ~keep
    for target_samples in [samples.as_slice(), &[]] {
        let expected = ResourceMonitor::calculate_stats(target_samples, &[], 4096);
        let measured = MeasuredCommandOutcome {
            output: Some(std::process::Output {
                status: std::process::ExitStatus::from_raw(7 << 8),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }),
            duration: Duration::from_millis(20),
            resource_stats: expected.clone(),
            error: None,
        };
        let execution = SubprocessAdapter::finish_measured_command(measured, "Failing command");

        assert!(matches!(execution.error, Some(Error::Benchmark(_))));
        assert_eq!(execution.duration, Duration::from_millis(20));
        assert_eq!(
            execution.resource_stats.baseline_memory_bytes,
            expected.baseline_memory_bytes
        );
        assert_eq!(execution.resource_stats.peak_memory_bytes, expected.peak_memory_bytes);
        assert_eq!(
            execution.resource_stats.peak_memory_delta_bytes,
            expected.peak_memory_delta_bytes
        );
        assert_eq!(execution.resource_stats.sample_count, expected.sample_count);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn measured_nonzero_exit_preserves_existing_timeout_error() {
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "sleep 0.02; exit 7"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);

    let mut measured = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_secs(1),
        "failing command",
        Duration::from_millis(1),
    )
    .await
    .unwrap();
    measured.error = Some(Error::Timeout("original timeout".to_string()));

    let execution = SubprocessAdapter::finish_measured_command(measured, "Failing command");

    assert!(matches!(
        execution.error,
        Some(Error::Timeout(message)) if message == "original timeout"
    ));
}
