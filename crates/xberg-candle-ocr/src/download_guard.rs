//! Wall-clock watchdog for blocking HuggingFace model downloads.
//!
//! This is a self-contained copy of the guard in the `xberg` crate's `model_download` module.
//! `xberg-candle-ocr` is a leaf crate that `xberg` depends on, so it cannot reach back into that
//! module; the helper is small and dependency-free, so duplicating it here is cheaper than
//! inverting the dependency. Keep the two copies in sync (same env var, same tracing target).

use std::time::Duration;

/// Default wall-clock ceiling for a single model-file download. hf-hub builds its ureq agent with
/// no read/connect timeout, so a stalled or firewalled connection to HuggingFace makes the blocking
/// `ApiRepo::get()` hang forever — silently wedging the whole extraction pipeline (observed: OCR /
/// embedding model pulls parked at 0% CPU behind a host firewall). We cap each fetch so a dead
/// network fails fast and the caller can degrade. Generous by default because a cold GB-scale model
/// legitimately takes minutes; override with `XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS`.
const DEFAULT_MODEL_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// Resolve the model-download deadline, honoring `XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS` (seconds; a
/// value of 0 or unparseable falls back to the default).
pub(crate) fn model_download_timeout() -> Duration {
    std::env::var("XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&s| s > 0)
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_MODEL_DOWNLOAD_TIMEOUT)
}

/// Run a blocking model-download closure under a hard wall-clock deadline so a hung network cannot
/// block the pipeline indefinitely. The closure runs on a detached worker thread; if it does not
/// finish within `model_download_timeout()` we log a warning and return `Err`, letting the caller
/// degrade (skip the model-backed backend) rather than hang. The worker thread cannot be
/// force-killed — it stays parked on the socket until the OS tears the connection down — but it
/// holds no lock the pipeline needs, so progress resumes. `label` names the fetch in the log/error.
pub(crate) fn with_download_deadline<T, F>(label: &str, f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    let deadline = model_download_timeout();
    let (tx, rx) = std::sync::mpsc::sync_channel::<Result<T, String>>(1);
    std::thread::Builder::new()
        .name("xberg-model-download".into())
        .spawn(move || {
            let _ = tx.send(f());
        })
        .map_err(|e| format!("failed to spawn model-download thread: {e}"))?;
    match rx.recv_timeout(deadline) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            tracing::warn!(
                target: "xberg::model_download",
                label = %label,
                timeout_secs = deadline.as_secs(),
                "model download exceeded deadline (network unreachable / firewalled?); aborting so \
                 the extraction pipeline does not hang. Set XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS to adjust."
            );
            Err(format!(
                "model download '{label}' timed out after {}s (HuggingFace unreachable?)",
                deadline.as_secs()
            ))
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err(format!("model-download thread for '{label}' died unexpectedly"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn with_download_deadline_returns_ok_for_fast_closure() {
        let result = with_download_deadline("fast", || Ok::<i32, String>(7));
        assert_eq!(result, Ok(7), "fast closure must return its Ok value verbatim");
    }

    #[test]
    fn deadline_reads_env_override_and_aborts_a_hung_closure() {
        // SAFETY: env mutation in the 2024 edition is unsafe; this var is exclusive to these tests
        // and only read at call time, so the set/read/remove sequence here has no observer to race.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS", "1");
        }
        assert_eq!(
            model_download_timeout(),
            Duration::from_secs(1),
            "explicit override must win"
        );

        let started = Instant::now();
        let result = with_download_deadline("hung", || {
            std::thread::sleep(Duration::from_secs(10));
            Ok::<(), String>(())
        });
        let elapsed = started.elapsed();
        // SAFETY: restore process state so sibling tests see the default (same reasoning as above).
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS");
        }

        let err = result.expect_err("a closure that outlives the deadline must return Err");
        assert!(err.contains("timed out"), "error must mention the timeout, got: {err}");
        assert!(
            elapsed < Duration::from_secs(3),
            "guard must fire near the 1s deadline, not wait out the 10s sleep (took {elapsed:?})"
        );
    }
}
