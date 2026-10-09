//! Low-level subprocess execution primitives: spawning with a start barrier, RSS-monitored
//! waiting with a timeout, and mapping the raw outcome into a `SubprocessExecution`.

use crate::monitoring::{ResourceMonitor, ResourceStats};
use crate::{Error, Result};
use std::process::Stdio;
use std::time::{Duration, Instant};
#[cfg(unix)]
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::SubprocessAdapter;
use super::support::{MeasuredCommandOutcome, SubprocessExecution, extract_json_from_stdout};

impl SubprocessAdapter {
    pub(super) fn timeout_error(operation: &str, timeout: Duration, reaped: Option<&std::process::Output>) -> Error {
        #[cfg(windows)]
        let cleanup = "; Windows timeout cleanup terminates the direct child only; descendant cleanup is unsupported";
        #[cfg(not(windows))]
        let cleanup = "";
        let mut message = format!("{operation} exceeded {timeout:?}{cleanup}");
        if let Some(output) = reaped {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr_tail = Self::tail_chars(stderr.trim_end(), 2000);
            if !stderr_tail.is_empty() {
                message.push_str(&format!("\nlast subprocess stderr (tail):\n{stderr_tail}"));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stdout_tail = Self::tail_chars(stdout.trim_end(), 500);
            if !stdout_tail.is_empty() {
                message.push_str(&format!("\nlast subprocess stdout (tail):\n{stdout_tail}"));
            }
        }
        Error::Timeout(message)
    }
    /// Return the last `max` bytes of `s`, prefixed with `…` when truncated, snapped to a char boundary.
    pub(super) fn tail_chars(s: &str, max: usize) -> String {
        if s.len() <= max {
            return s.to_string();
        }
        let mut cut = s.len() - max;
        while cut < s.len() && !s.is_char_boundary(cut) {
            cut += 1;
        }
        format!("…{}", &s[cut..])
    }
    pub(super) fn measured_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
        #[cfg(unix)]
        {
            let mut command = Command::new("sh");
            command
                .arg("-c")
                .arg("IFS= read -r _ || exit 125; exec \"$@\"")
                .arg("xberg-benchmark-start-barrier")
                .arg(program);
            command
        }
        #[cfg(not(unix))]
        {
            Command::new(program)
        }
    }
    pub(super) fn configure_measured_stdin(cmd: &mut Command) {
        #[cfg(unix)]
        cmd.stdin(Stdio::piped());
        #[cfg(not(unix))]
        cmd.stdin(Stdio::null());
    }
    /// Spawn the command, capture its OS pid, and (on unix) split off its stdin as the
    /// start barrier used to delay RSS sampling until the child has actually begun. ~keep
    pub(super) fn spawn_measured_child(
        cmd: &mut Command,
        operation: &str,
    ) -> Result<(tokio::process::Child, Option<u32>, Option<tokio::process::ChildStdin>)> {
        let child = cmd
            .spawn()
            .map_err(|error| Error::Benchmark(format!("Failed to spawn {operation}: {error}")))?;
        let child_pid = child.id();
        #[cfg(unix)]
        let (child, start_barrier) = {
            let mut child = child;
            let start_barrier = child.stdin.take();
            (child, start_barrier)
        };
        #[cfg(not(unix))]
        let start_barrier: Option<tokio::process::ChildStdin> = None;
        Ok((child, child_pid, start_barrier))
    }

    /// Release the unix start barrier (a piped stdin byte that lets us begin RSS sampling
    /// before the child does real work), or report why it couldn't be released. On
    /// non-unix platforms there is no barrier to release, so this always succeeds. ~keep
    pub(super) async fn release_start_barrier(
        mut start_barrier: Option<tokio::process::ChildStdin>,
        operation: &str,
    ) -> Option<Error> {
        #[cfg(unix)]
        {
            match start_barrier.take() {
                Some(mut barrier) => match barrier.write_all(b"start\n").await {
                    Ok(()) => barrier.shutdown().await.err(),
                    Err(error) => Some(error),
                }
                .map(|error| Error::Benchmark(format!("Failed to release {operation} start barrier: {error}"))),
                None => Some(Error::Benchmark(format!("Failed to open {operation} start barrier"))),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = start_barrier;
            let _ = operation;
            None
        }
    }

    /// Compute the final resource-usage stats for a measured command, and — when a successful
    /// target process existed but RSS monitoring never captured a sample — synthesize a "not
    /// measurable on this platform" error rather than silently reporting zero usage. ~keep
    pub(super) async fn finalize_measured_resources(
        monitor: Option<ResourceMonitor>,
        child_pid: Option<u32>,
        error: Option<Error>,
        command_succeeded: bool,
        operation: &str,
    ) -> (ResourceStats, Option<Error>) {
        let resource_stats = if let Some(monitor) = monitor {
            let samples = monitor.stop().await;
            let snapshots = monitor.get_snapshots().await;
            let baseline = monitor.baseline_memory().await;
            ResourceMonitor::calculate_stats(&samples, &snapshots, baseline)
        } else {
            ResourceStats::default()
        };
        let error = if command_succeeded && child_pid.is_some() && resource_stats.sample_count == 0 && error.is_none() {
            Some(Error::Benchmark(format!(
                "{operation} completed before RSS monitoring captured a target sample; result is not measurable on this platform"
            )))
        } else {
            error
        };
        (resource_stats, error)
    }

    pub(super) async fn execute_measured_command(
        cmd: &mut Command,
        timeout: Duration,
        operation: &str,
        sample_interval: Duration,
    ) -> Result<MeasuredCommandOutcome> {
        #[cfg(not(unix))]
        let start = Instant::now();
        #[cfg(not(unix))]
        let deadline = start + timeout;
        let (child, child_pid, mut start_barrier) = Self::spawn_measured_child(cmd, operation)?;
        let monitor = child_pid.map(ResourceMonitor::new_for_pid);
        if let Some(monitor) = &monitor {
            #[cfg(unix)]
            monitor.prepare().await;
            #[cfg(not(unix))]
            monitor.start(sample_interval).await;
        }
        #[cfg(unix)]
        let start = Instant::now();
        let barrier_error = Self::release_start_barrier(start_barrier.take(), operation).await;
        #[cfg(unix)]
        if barrier_error.is_none()
            && let Some(monitor) = &monitor
        {
            monitor.activate(sample_interval).await;
        }
        #[cfg(unix)]
        let wait_timeout = timeout;
        #[cfg(not(unix))]
        let wait_timeout = deadline.saturating_duration_since(Instant::now());
        let mut wait = Box::pin(child.wait_with_output());
        let (output, error, duration) = if let Some(error) = barrier_error {
            #[cfg(unix)]
            Self::kill_process_group(child_pid);
            let _ = wait.await;
            (None, Some(error), start.elapsed())
        } else {
            match tokio::time::timeout(wait_timeout, &mut wait).await {
                Ok(Ok(output)) => (Some(output), None, start.elapsed()),
                Ok(Err(error)) => (
                    None,
                    Some(Error::Benchmark(format!("Failed to wait for {operation}: {error}"))),
                    start.elapsed(),
                ),
                Err(_) => {
                    let duration = start.elapsed();
                    // Capture the reaped child output so the hung subprocess's last
                    // stderr/stdout is surfaced in the timeout error (CI self-diagnosis).
                    #[cfg(unix)]
                    let reaped = {
                        Self::kill_process_group(child_pid);
                        wait.await.ok()
                    };
                    #[cfg(not(unix))]
                    let reaped: Option<std::process::Output> = None;
                    (
                        None,
                        Some(Self::timeout_error(operation, timeout, reaped.as_ref())),
                        duration,
                    )
                }
            }
        };
        let command_succeeded = output.as_ref().is_some_and(|output| output.status.success());
        let (resource_stats, error) =
            Self::finalize_measured_resources(monitor, child_pid, error, command_succeeded, operation).await;
        Ok(MeasuredCommandOutcome {
            output,
            duration,
            resource_stats,
            error,
        })
    }
    pub(super) fn finish_measured_command(measured: MeasuredCommandOutcome, operation: &str) -> SubprocessExecution {
        let mut error = measured.error;
        let stdout = measured.output.map_or_else(String::new, |output| {
            let raw_stdout = String::from_utf8_lossy(&output.stdout);
            let stdout = extract_json_from_stdout(&raw_stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if !output.status.success() {
                let mut message = format!("{operation} failed with {}", output.status);
                if !stderr.is_empty() {
                    message.push_str(&format!("\nstderr: {stderr}"));
                }
                if !stdout.is_empty() && stdout.len() < 500 {
                    message.push_str(&format!("\nstdout: {stdout}"));
                }
                if error.is_none() {
                    error = Some(Error::Benchmark(message));
                }
            }
            stdout
        });

        SubprocessExecution {
            stdout,
            duration: measured.duration,
            resource_stats: measured.resource_stats,
            error,
        }
    }
    pub(super) fn configure_child_process(cmd: &mut Command) {
        cmd.kill_on_drop(true);
        #[cfg(unix)]
        cmd.process_group(0);
    }
    #[cfg(unix)]
    pub(super) fn kill_process_group(pid: Option<u32>) {
        if let Some(pid) = pid {
            // SAFETY: the child was placed in a process group whose id equals its ~keep
            // pid. A negative pid targets only that group, never the harness.
            unsafe {
                libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
            }
        }
    }
    /// Determine if a framework supports OCR based on its name
    ///
    /// Known frameworks with OCR support:
    /// - xberg-* (all Xberg bindings support OCR)
    /// - pymupdf (supports OCR via tesseract)
    ///
    /// Frameworks without OCR support include other basic PDF parsers.
    pub(super) fn framework_supports_ocr(framework_name: &str) -> bool {
        let name_lower = framework_name.to_lowercase();

        if name_lower.starts_with("xberg-") || name_lower == "xberg" {
            return true;
        }

        if name_lower.contains("pymupdf") {
            return true;
        }

        if name_lower.contains("docling") {
            return true;
        }

        if name_lower.contains("unstructured") {
            return true;
        }

        if name_lower.contains("tika") {
            return true;
        }

        if name_lower.contains("mineru") {
            return true;
        }

        if name_lower.contains("liteparse") {
            return true;
        }

        false
    }
}
