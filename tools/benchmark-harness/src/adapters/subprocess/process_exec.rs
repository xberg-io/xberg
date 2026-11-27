//! Low-level subprocess execution primitives: spawning with a start barrier, waiting with a
//! timeout, measuring peak memory (`wait4(2)` rusage on unix, the RSS sampler elsewhere), and
//! mapping the raw outcome into a `SubprocessExecution`.

use crate::monitoring::{ResourceMonitor, ResourceStats};
#[cfg(unix)]
use crate::types::MemoryMethod;
use crate::{Error, Result};
use std::process::Stdio;
use std::time::{Duration, Instant};
#[cfg(unix)]
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use super::SubprocessAdapter;
use super::support::{MeasuredCommandOutcome, SubprocessExecution, extract_json_from_stdout};

#[cfg(unix)]
type ReapResult = std::io::Result<(std::process::ExitStatus, u64)>;

/// A spawned unix child whose exit is collected by our own `wait4` call rather than by tokio,
/// so its rusage survives reaping.
#[cfg(unix)]
struct ReapedChild {
    pid: u32,
    kill_guard: ProcessGroupKillGuard,
    start_barrier: Option<std::process::ChildStdin>,
    stdout: Option<tokio::process::ChildStdout>,
    stderr: Option<tokio::process::ChildStderr>,
    reap: tokio::task::JoinHandle<ReapResult>,
}

/// Kills the child's process group when dropped before the child was reaped, replacing
/// tokio's `kill_on_drop`, which does not apply to a child spawned through `as_std_mut`.
#[cfg(unix)]
struct ProcessGroupKillGuard(Option<u32>);

#[cfg(unix)]
impl ProcessGroupKillGuard {
    fn disarm(&mut self) {
        self.0 = None;
    }
}

#[cfg(unix)]
impl Drop for ProcessGroupKillGuard {
    fn drop(&mut self) {
        SubprocessAdapter::kill_process_group(self.0.take());
    }
}

#[cfg(unix)]
async fn read_pipe<R: tokio::io::AsyncRead + Unpin>(pipe: Option<R>) -> std::io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    if let Some(mut pipe) = pipe {
        pipe.read_to_end(&mut buffer).await?;
    }
    Ok(buffer)
}

/// Block until `pid` exits, reap it, and return its exit status and `ru_maxrss` in bytes.
#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "wait4 is the only call that reaps a child together with its rusage"
)]
fn reap_with_rusage(pid: u32) -> ReapResult {
    use std::os::unix::process::ExitStatusExt;
    let pid = libc::pid_t::try_from(pid).map_err(std::io::Error::other)?;
    loop {
        let mut status: libc::c_int = 0;
        // SAFETY: `rusage` is a plain C struct of integers, for which all-zero is valid. ~keep
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        // SAFETY: both out-pointers reference live locals and `pid` is our own child, ~keep
        // which no other code waits on.
        let reaped = unsafe { libc::wait4(pid, &mut status, 0, &mut usage) };
        if reaped != -1 {
            return Ok((
                std::process::ExitStatus::from_raw(status),
                SubprocessAdapter::ru_maxrss_to_bytes(usage.ru_maxrss),
            ));
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

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

    /// Normalise `rusage.ru_maxrss` to bytes: Apple platforms report bytes, Linux and the BSDs
    /// report KiB. A negative value (never produced by a real kernel) maps to 0.
    #[cfg(unix)]
    pub(super) fn ru_maxrss_to_bytes(ru_maxrss: libc::c_long) -> u64 {
        let raw = u64::try_from(ru_maxrss).unwrap_or(0);
        if cfg!(target_vendor = "apple") {
            raw
        } else {
            raw.saturating_mul(1024)
        }
    }

    /// Spawn the command through std so tokio never reaps it, start the blocking `wait4` reaper
    /// immediately (before any fallible step, so every exit path still reaps the child), and
    /// split off stdin as the start barrier. ~keep
    #[cfg(unix)]
    #[allow(
        clippy::zombie_processes,
        reason = "the child is reaped by reap_with_rusage, not Child::wait, so its rusage is kept"
    )]
    fn spawn_reaped_child(cmd: &mut Command, operation: &str) -> Result<ReapedChild> {
        let mut child = cmd
            .as_std_mut()
            .spawn()
            .map_err(|error| Error::Benchmark(format!("Failed to spawn {operation}: {error}")))?;
        let pid = child.id();
        let kill_guard = ProcessGroupKillGuard(Some(pid));
        let reap = tokio::task::spawn_blocking(move || reap_with_rusage(pid));
        let start_barrier = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        drop(child);
        let pipe_error =
            |error: std::io::Error| Error::Benchmark(format!("Failed to attach {operation} pipes: {error}"));
        let stdout = stdout
            .map(tokio::process::ChildStdout::from_std)
            .transpose()
            .map_err(pipe_error)?;
        let stderr = stderr
            .map(tokio::process::ChildStderr::from_std)
            .transpose()
            .map_err(pipe_error)?;
        Ok(ReapedChild {
            pid,
            kill_guard,
            start_barrier,
            stdout,
            stderr,
            reap,
        })
    }

    /// Release the unix start barrier (a piped stdin line that lets us begin RSS sampling
    /// before the child does real work), or report why it couldn't be released. The write is
    /// six bytes into an empty pipe, so it cannot block. ~keep
    #[cfg(unix)]
    fn release_start_barrier(start_barrier: Option<std::process::ChildStdin>, operation: &str) -> Option<Error> {
        use std::io::Write;
        let Some(mut barrier) = start_barrier else {
            return Some(Error::Benchmark(format!("Failed to open {operation} start barrier")));
        };
        barrier
            .write_all(b"start\n")
            .err()
            .map(|error| Error::Benchmark(format!("Failed to release {operation} start barrier: {error}")))
    }

    /// Compute the final resource-usage stats for a measured command. A `wait4` peak, when
    /// available, replaces the sampled peak and makes even a sub-interval process measurable;
    /// without one, a successful target process that RSS monitoring never sampled yields a
    /// "not measurable on this platform" error rather than silently reporting zero usage. ~keep
    pub(super) async fn finalize_measured_resources(
        monitor: Option<ResourceMonitor>,
        child_pid: Option<u32>,
        error: Option<Error>,
        command_succeeded: bool,
        operation: &str,
        rusage_peak_bytes: Option<u64>,
    ) -> (ResourceStats, Option<Error>) {
        let mut resource_stats = if let Some(monitor) = monitor {
            let samples = monitor.stop().await;
            let snapshots = monitor.get_snapshots().await;
            let baseline = monitor.baseline_memory().await;
            ResourceMonitor::calculate_stats(&samples, &snapshots, baseline)
        } else {
            ResourceStats::default()
        };
        #[cfg(unix)]
        if let Some(peak) = rusage_peak_bytes {
            resource_stats.peak_memory_bytes = peak;
            resource_stats.peak_memory_delta_bytes = peak.saturating_sub(resource_stats.baseline_memory_bytes);
            resource_stats.memory_method = MemoryMethod::RusageMaxRss;
            return (resource_stats, error);
        }
        #[cfg(not(unix))]
        let _ = (rusage_peak_bytes, &mut resource_stats);
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
        #[cfg(unix)]
        {
            Self::execute_rusage_measured_command(cmd, timeout, operation, sample_interval).await
        }
        #[cfg(not(unix))]
        {
            Self::execute_sampled_command(cmd, timeout, operation, sample_interval).await
        }
    }

    /// Unix path: the sampler supplies CPU and the RSS timeline, `wait4` supplies the peak.
    /// stdout and stderr are drained concurrently with the reap, so a child that fills a pipe
    /// buffer never deadlocks against us. On timeout the whole process group is killed and the
    /// child is still reaped, so its last output reaches the timeout error. ~keep
    #[cfg(unix)]
    async fn execute_rusage_measured_command(
        cmd: &mut Command,
        timeout: Duration,
        operation: &str,
        sample_interval: Duration,
    ) -> Result<MeasuredCommandOutcome> {
        let ReapedChild {
            pid,
            mut kill_guard,
            start_barrier,
            stdout,
            stderr,
            reap,
        } = Self::spawn_reaped_child(cmd, operation)?;
        let child_pid = Some(pid);
        let monitor = ResourceMonitor::new_for_pid(pid);
        monitor.prepare().await;
        let start = Instant::now();
        let barrier_error = Self::release_start_barrier(start_barrier, operation);
        if barrier_error.is_none() {
            monitor.activate(sample_interval).await;
        }
        let mut collect = Box::pin(async move {
            let (reaped, stdout, stderr) = tokio::join!(reap, read_pipe(stdout), read_pipe(stderr));
            let (status, max_rss_bytes) = reaped.map_err(std::io::Error::other)??;
            let output = std::process::Output {
                status,
                stdout: stdout?,
                stderr: stderr?,
            };
            Ok::<_, std::io::Error>((output, max_rss_bytes))
        });
        let (output, rusage_peak_bytes, error, duration) = if let Some(error) = barrier_error {
            Self::kill_process_group(child_pid);
            let _ = collect.await;
            (None, None, Some(error), start.elapsed())
        } else {
            match tokio::time::timeout(timeout, &mut collect).await {
                Ok(Ok((output, max_rss_bytes))) => (Some(output), Some(max_rss_bytes), None, start.elapsed()),
                Ok(Err(error)) => (
                    None,
                    None,
                    Some(Error::Benchmark(format!("Failed to wait for {operation}: {error}"))),
                    start.elapsed(),
                ),
                Err(_) => {
                    let duration = start.elapsed();
                    Self::kill_process_group(child_pid);
                    let reaped = collect.await.ok();
                    let max_rss_bytes = reaped.as_ref().map(|(_, max_rss_bytes)| *max_rss_bytes);
                    let reaped_output = reaped.map(|(output, _)| output);
                    (
                        None,
                        max_rss_bytes,
                        Some(Self::timeout_error(operation, timeout, reaped_output.as_ref())),
                        duration,
                    )
                }
            }
        };
        kill_guard.disarm();
        let command_succeeded = output.as_ref().is_some_and(|output| output.status.success());
        let (resource_stats, error) = Self::finalize_measured_resources(
            Some(monitor),
            child_pid,
            error,
            command_succeeded,
            operation,
            rusage_peak_bytes,
        )
        .await;
        Ok(MeasuredCommandOutcome {
            output,
            duration,
            resource_stats,
            error,
        })
    }

    /// Non-unix path: no `wait4`, so peak memory comes from the RSS sampler. On timeout the
    /// wait future is dropped and tokio's `kill_on_drop` terminates the direct child.
    #[cfg(not(unix))]
    async fn execute_sampled_command(
        cmd: &mut Command,
        timeout: Duration,
        operation: &str,
        sample_interval: Duration,
    ) -> Result<MeasuredCommandOutcome> {
        let start = Instant::now();
        let deadline = start + timeout;
        let child = cmd
            .spawn()
            .map_err(|error| Error::Benchmark(format!("Failed to spawn {operation}: {error}")))?;
        let child_pid = child.id();
        let monitor = child_pid.map(ResourceMonitor::new_for_pid);
        if let Some(monitor) = &monitor {
            monitor.start(sample_interval).await;
        }
        let wait_timeout = deadline.saturating_duration_since(Instant::now());
        let (output, error, duration) = match tokio::time::timeout(wait_timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => (Some(output), None, start.elapsed()),
            Ok(Err(error)) => (
                None,
                Some(Error::Benchmark(format!("Failed to wait for {operation}: {error}"))),
                start.elapsed(),
            ),
            Err(_) => (
                None,
                Some(Self::timeout_error(operation, timeout, None)),
                start.elapsed(),
            ),
        };
        let command_succeeded = output.as_ref().is_some_and(|output| output.status.success());
        let (resource_stats, error) =
            Self::finalize_measured_resources(monitor, child_pid, error, command_succeeded, operation, None).await;
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
    #[allow(unsafe_code, reason = "libc::kill is the only way to signal a process group")]
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
