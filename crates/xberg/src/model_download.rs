//! Shared utilities for downloading and verifying ONNX models from HuggingFace Hub.
//!
//! Used by both layout detection and PaddleOCR model managers.

use std::io::{BufReader, Read};
use std::path::Path;
// `PathBuf` is only referenced by the HF-download and cache-dir helpers, which are
// gated to the features that use them; a candle-only build stages via ModelScope
// and doesn't compile them.
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "ner-onnx"
))]
use std::path::PathBuf;

use sha2::{Digest, Sha256};

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
    feature = "ner-onnx"
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
    feature = "ner-onnx"
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

    let repo = api.model(repo_id.to_string());
    let cached_path = repo
        .get(remote_filename)
        .map_err(|e| format!("Failed to download '{remote_filename}' from {repo_id}: {e}"))?;

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
#[cfg(any(feature = "ner-onnx", feature = "candle-ocr"))]
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

    #[cfg(any(feature = "ner-onnx", feature = "candle-ocr"))]
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

    #[cfg(any(feature = "ner-onnx", feature = "candle-ocr"))]
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
