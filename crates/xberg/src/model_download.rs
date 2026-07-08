//! Shared utilities for downloading and verifying ONNX models from HuggingFace Hub.
//!
//! Used by both layout detection and PaddleOCR model managers.

use std::time::Duration;

// `BufReader`/`Read`/`Path`/`sha2` back `verify_sha256`, which is only compiled for the
// model-manager features that checksum their downloads. `PathBuf` is likewise only
// referenced by the HF-download and cache-dir helpers. Gate the imports to those features
// so an `embeddings`/`reranker`/`transcription`-only build (which reaches this module solely
// for the always-compiled download watchdog below) stays warning-clean.
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "ner-onnx",
    feature = "candle-hunyuan-ocr"
))]
use sha2::{Digest, Sha256};
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "ner-onnx",
    feature = "candle-hunyuan-ocr"
))]
use std::io::{BufReader, Read};
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "ner-onnx",
    feature = "candle-hunyuan-ocr"
))]
use std::path::Path;
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "ner-onnx",
    feature = "candle-hunyuan-ocr"
))]
use std::path::PathBuf;

/// Default wall-clock ceiling for a single model-file download. hf-hub builds its ureq agent with
/// no read/connect timeout, so a stalled or firewalled connection to HuggingFace makes the blocking
/// `ApiRepo::get()` hang forever — silently wedging the whole extraction pipeline (observed: OCR /
/// embedding model pulls parked at 0% CPU behind a host firewall). We cap each fetch so a dead
/// network fails fast and the caller can degrade. Generous by default because a cold GB-scale model
/// legitimately takes minutes; override with `XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS`.
// dead_code: the watchdog is always compiled but only *called* from features that download models
// (embeddings / reranker / transcription / the OCR + NER managers). A no-download-feature build
// legitimately never reaches it; suppress the lint rather than fragment the guard behind a cfg.
#[allow(dead_code)]
const DEFAULT_MODEL_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// Resolve the model-download deadline, honoring `XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS` (seconds; a
/// value of 0 or unparseable falls back to the default).
#[allow(dead_code)] // see DEFAULT_MODEL_DOWNLOAD_TIMEOUT: called only from download-feature builds.
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
#[allow(dead_code)] // see DEFAULT_MODEL_DOWNLOAD_TIMEOUT: called only from download-feature builds.
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

/// Return the process-wide lock guarding downloads of a single `(repo, file)`.
///
/// hf-hub takes a file lock on the blob it is fetching and *errors* ("Lock
/// acquisition failed") rather than waiting when a second thread races the same
/// uncached file — so two tests (or two parallel-page OCR workers) that both need
/// the same cold model can knock each other out. Serializing above hf-hub, keyed on
/// the exact file, lets the first thread populate the cache while the rest wait and
/// then get the warm copy; downloads of *different* files still run in parallel.
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "ner-onnx",
    feature = "candle-hunyuan-ocr"
))]
fn download_lock(key: &str) -> std::sync::Arc<std::sync::Mutex<()>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};

    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut map = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Arc::clone(map.entry(key.to_string()).or_default())
}

/// Download a file from a HuggingFace Hub repository.
///
/// Uses `hf-hub`'s built-in caching so repeated calls for the same file are fast.
/// Concurrent calls for the same file serialize (see [`download_lock`]) so a cold
/// cache is populated once instead of racing hf-hub's blob lock.
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "ner-onnx",
    feature = "candle-hunyuan-ocr"
))]
pub(crate) fn hf_download(repo_id: &str, remote_filename: &str) -> Result<PathBuf, String> {
    tracing::info!(repo = repo_id, filename = remote_filename, "Downloading via hf-hub");

    // Serialize concurrent fetches of this exact file; the guard is released once the
    // blob is in the hf-hub cache, so waiters return the warm copy immediately.
    let file_lock = download_lock(&format!("{repo_id}/{remote_filename}"));
    let _guard = file_lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

    let api = hf_hub::api::sync::ApiBuilder::from_env()
        .with_progress(true)
        .build()
        .map_err(|e| format!("Failed to initialize HuggingFace Hub API: {e}"))?;

    // Wrap the blocking fetch in the wall-clock watchdog so a firewalled HuggingFace host can no
    // longer hang the extraction pipeline forever (hf-hub sets no read/connect timeout). `ApiRepo`
    // is not `Clone` in hf-hub 0.4, but `Api` is, so the closure captures a cheap `Api` clone and
    // rebuilds its `ApiRepo` via `api.model(..)` inside.
    let cached_path = {
        let api = api.clone();
        let filename = remote_filename.to_string();
        let repo_id = repo_id.to_string();
        with_download_deadline(&format!("{repo_id}/{remote_filename}"), move || {
            api.model(repo_id.clone())
                .get(&filename)
                .map_err(|e| format!("Failed to download '{filename}' from {repo_id}: {e}"))
        })?
    };

    Ok(cached_path)
}

/// Parse a `sha256sum`-format manifest into ordered `(path, sha256)` pairs.
///
/// Skips blank lines and `#` comments; each remaining line must be
/// `<64-hex-sha256>  <path>`. Leading `./` is stripped from paths and checksums are
/// lowercased. Returns the pairs in file order (may be empty if the content is all
/// comments — callers that require at least one entry check that themselves).
///
/// Shared by every checksum-manifest consumer (GLiNER model checksums, Candle VLM-OCR
/// weight staging) so the format and validation live in one place.
#[cfg(any(feature = "ner-onnx", feature = "candle-hunyuan-ocr"))]
pub(crate) fn parse_sha256_manifest(content: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let checksum = parts
            .next()
            .ok_or_else(|| format!("Invalid checksum line {}: missing checksum", index + 1))?;
        let path = parts
            .next()
            .ok_or_else(|| format!("Invalid checksum line {}: missing path", index + 1))?;
        if checksum.len() != 64 || !checksum.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!(
                "Invalid checksum line {}: checksum must be SHA256 hex",
                index + 1
            ));
        }
        entries.push((path.trim_start_matches("./").to_string(), checksum.to_ascii_lowercase()));
    }
    Ok(entries)
}

/// Verify the SHA256 checksum of a file using streaming reads.
///
/// Streams the file in 64 KiB chunks to avoid loading large model files (100MB+) entirely
/// into memory. Returns `Ok(())` if the checksum matches or is empty (skip verification).
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "ner-onnx",
    feature = "candle-hunyuan-ocr"
))]
pub(crate) fn verify_sha256(path: &Path, expected: &str, label: &str) -> Result<(), String> {
    if expected.is_empty() {
        return Ok(());
    }

    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open file for checksum: {e}"))?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();

    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Failed to read file for checksum: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let hash_hex = hex::encode(hasher.finalize());

    if hash_hex != expected {
        return Err(format!(
            "Checksum mismatch for {label}: expected {expected}, got {hash_hex}"
        ));
    }

    tracing::debug!(label, "Checksum verified");
    Ok(())
}

/// Resolve the xberg cache directory for a given module.
///
/// Delegates to [`crate::cache_dir::resolve_cache_dir`] for centralized,
/// platform-aware cache directory resolution.
#[cfg(feature = "layout-detection")]
pub(crate) fn resolve_cache_dir(module: &str) -> PathBuf {
    crate::cache_dir::resolve_cache_dir(module)
}

/// Tests for the always-compiled download watchdog. Deliberately network-free: they exercise the
/// deadline machinery with plain closures so the guard's behavior is provable in CI without any
/// HuggingFace connectivity.
#[cfg(test)]
mod download_deadline_tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn with_download_deadline_returns_ok_for_fast_closure() {
        // A closure that finishes well within the (default, generous) deadline must pass its
        // value straight through untouched.
        let result = with_download_deadline("fast", || Ok::<i32, String>(42));
        assert_eq!(result, Ok(42), "fast closure must return its Ok value verbatim");
    }

    // The env var `XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS` is process-global, so the two tests that
    // mutate it are folded into one serial test rather than racing under the parallel runner.
    #[test]
    fn deadline_reads_env_override_and_aborts_a_hung_closure() {
        // First: the override resolves as the exact configured ceiling.
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

        // Second: simulate a wedged (firewalled) download — a closure that sleeps far past the
        // 1s ceiling must be abandoned with Err("...timed out...") in ~1s, never the full sleep.
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

#[cfg(all(
    test,
    any(
        feature = "paddle-ocr",
        feature = "layout-detection",
        feature = "auto-rotate",
        feature = "ner-onnx"
    )
))]
mod tests {
    use super::*;

    #[cfg(any(feature = "ner-onnx", feature = "candle-hunyuan-ocr"))]
    #[test]
    fn parse_sha256_manifest_reads_entries_and_normalizes() {
        let entries = parse_sha256_manifest(
            "# comment\n\
             AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA  ./config.json\n\
             bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  tokenizer.json\n",
        )
        .expect("valid manifest");
        // `./` stripped, order preserved, checksum lowercased.
        assert_eq!(entries[0], ("config.json".to_string(), "a".repeat(64)));
        assert_eq!(entries[1].0, "tokenizer.json");
        // All comments → empty (callers decide whether that is an error).
        assert!(parse_sha256_manifest("# only comments\n").unwrap().is_empty());
    }

    #[cfg(any(feature = "ner-onnx", feature = "candle-hunyuan-ocr"))]
    #[test]
    fn parse_sha256_manifest_rejects_malformed_lines() {
        assert!(
            parse_sha256_manifest("not-a-sha256  config.json").is_err(),
            "invalid hash"
        );
        assert!(parse_sha256_manifest(&"a".repeat(64)).is_err(), "missing path");
    }

    #[test]
    fn download_lock_is_stable_per_key_and_distinct_across_keys() {
        let a1 = download_lock("xberg-io/layout-models/rtdetr/model.onnx");
        let a2 = download_lock("xberg-io/layout-models/rtdetr/model.onnx");
        let b = download_lock("xberg-io/layout-models/tatr/model.onnx");

        // Same file → same lock, so concurrent fetches of it serialize.
        assert!(std::sync::Arc::ptr_eq(&a1, &a2), "same key must share one lock");
        // Different file → different lock, so unrelated downloads stay parallel.
        assert!(!std::sync::Arc::ptr_eq(&a1, &b), "distinct keys must not share a lock");
    }

    #[test]
    fn download_lock_serializes_same_key_across_threads() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        // If the same-key lock were not exclusive, the two threads' critical sections
        // would overlap and `in_flight` would exceed 1.
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let (in_flight, max_seen) = (in_flight.clone(), max_seen.clone());
                std::thread::spawn(move || {
                    let lock = download_lock("same/key/file.bin");
                    let _g = lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    max_seen.fetch_max(now, Ordering::SeqCst);
                    std::thread::yield_now();
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(
            max_seen.load(Ordering::SeqCst),
            1,
            "same-key critical sections must not overlap"
        );
    }
}
