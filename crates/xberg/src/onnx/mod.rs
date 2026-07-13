//! Shared ONNX Runtime model-loading helpers.
//!
//! Consolidates the HuggingFace download + cross-process lock + tokenizer load +
//! ORT session-build machinery that would otherwise be copy-pasted across every
//! ONNX-backed capability (embeddings, reranking, sparse embeddings, late
//! interaction). New ONNX modules build on these helpers instead of vendoring
//! their own copies.
//!
//! Each fallible helper takes an [`ErrCtor`] — a module-specific error
//! constructor (e.g. [`crate::XbergError::embedding`] or
//! [`crate::XbergError::reranking`]) — so callers keep their module-tagged error
//! variant without this module needing to know which capability it serves.
//! ONNX-Runtime-missing failures are reported as [`crate::XbergError::MissingDependency`]
//! regardless of the caller.
//!
//! Since v5.0.0.

use std::path::{Path, PathBuf};

/// A module-specific error constructor, e.g. `crate::XbergError::embedding::<String>`.
///
/// Threaded through the fallible helpers so each caller keeps its own
/// module-tagged [`crate::XbergError`] variant.
pub(crate) type ErrCtor = fn(String) -> crate::XbergError;

/// Returns installation instructions for ONNX Runtime.
pub(crate) fn onnx_runtime_install_message() -> String {
    #[cfg(all(windows, target_env = "gnu"))]
    {
        return "ONNX Runtime is not supported on Windows MinGW builds. \
        ONNX Runtime requires MSVC toolchain. \
        Please use Windows MSVC builds or disable ONNX-backed features."
            .to_string();
    }

    #[cfg(not(all(windows, target_env = "gnu")))]
    {
        "ONNX Runtime is required for this functionality. \
        Install: \
        macOS: 'brew install onnxruntime', \
        Linux (Ubuntu/Debian): 'apt install libonnxruntime libonnxruntime-dev', \
        Linux (Fedora): 'dnf install onnxruntime onnxruntime-devel', \
        Linux (Arch): 'pacman -S onnxruntime', \
        Windows (MSVC): Download from https://github.com/microsoft/onnxruntime/releases and add to PATH. \
        \
        Alternatively, set ORT_DYLIB_PATH environment variable to the ONNX Runtime library path."
            .to_string()
    }
}

/// Check if an error message looks like an ONNX Runtime missing dependency.
pub(crate) fn looks_like_ort_error(msg: &str) -> bool {
    msg.contains("onnxruntime")
        || msg.contains("ORT")
        || msg.contains("libonnxruntime")
        || msg.contains("onnxruntime.dll")
        || msg.contains("Unable to load")
        || msg.contains("library load failed")
        || msg.contains("attempting to load")
        || msg.contains("An error occurred while")
}

/// Convert a panic payload to a string message.
pub(crate) fn panic_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

/// Map a failure message to either `MissingDependency` (when it looks like an ORT
/// load failure) or the caller's module-specific error.
fn ort_missing_or(err: ErrCtor, msg: String) -> crate::XbergError {
    if looks_like_ort_error(&msg) {
        crate::XbergError::MissingDependency(format!("ONNX Runtime - {}", onnx_runtime_install_message()))
    } else {
        err(msg)
    }
}

/// How long a partial download must be idle before it is considered stale.
const STALE_DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Remove stale `.lock` and `.part` files left behind by interrupted downloads.
fn cleanup_stale_locks(cache_dir: &Path, repo_name: &str) {
    let folder = format!("models--{}", repo_name.replace('/', "--"));
    let blobs_dir = cache_dir.join(folder).join("blobs");

    let entries = match std::fs::read_dir(&blobs_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let now = std::time::SystemTime::now();

    for entry in entries.flatten() {
        let lock_path = entry.path();
        if lock_path.extension().is_some_and(|ext| ext == "lock") {
            let part_path = lock_path.with_extension("part");
            let probe_path = if part_path.exists() { &part_path } else { &lock_path };

            let age = probe_path
                .metadata()
                .and_then(|m| m.modified())
                .and_then(|modified| now.duration_since(modified).map_err(std::io::Error::other))
                .unwrap_or(std::time::Duration::ZERO);

            if age >= STALE_DOWNLOAD_TIMEOUT {
                if std::fs::remove_file(&lock_path).is_ok() {
                    tracing::info!(
                        path = ?lock_path,
                        idle_minutes = age.as_secs() / 60,
                        "Removed stale download lock file",
                    );
                }
                if part_path.exists() && std::fs::remove_file(&part_path).is_ok() {
                    tracing::info!(path = ?part_path, "Removed stale partial download");
                }
            }
        }
    }
}

/// Build a human-readable hint to attach to a LockAcquisition error.
fn lock_acquisition_hint(cache_dir: &Path, repo_name: &str) -> String {
    let folder = format!("models--{}", repo_name.replace('/', "--"));
    format!(
        "\n\nAnother process may be downloading this model. \
        If no download is in progress, remove the stale files and retry:\n  \
        rm -f {cache}/{folder}/blobs/*.lock\n  \
        rm -f {cache}/{folder}/blobs/*.part",
        cache = cache_dir.display(),
        folder = folder,
    )
}

/// A held cross-process advisory lock that serializes model downloads.
struct ProcessDownloadLock {
    file: std::fs::File,
}

impl ProcessDownloadLock {
    fn acquire(cache_dir: &Path, repo_name: &str) -> Option<Self> {
        let folder = format!("models--{}", repo_name.replace('/', "--"));
        let model_dir = cache_dir.join(folder);
        if let Err(error) = std::fs::create_dir_all(&model_dir) {
            tracing::debug!(?error, "Could not create model dir for download lock");
            return None;
        }
        let lock_path = model_dir.join(".xberg-download.lock");
        let file = match std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(file) => file,
            Err(error) => {
                tracing::debug!(?error, path = ?lock_path, "Could not open download lock file");
                return None;
            }
        };

        if !blocking_lock_exclusive(&file) {
            tracing::debug!(path = ?lock_path, "Could not acquire cross-process download lock");
            return None;
        }

        tracing::debug!(path = ?lock_path, "Acquired cross-process download lock");
        Some(Self { file })
    }
}

impl Drop for ProcessDownloadLock {
    fn drop(&mut self) {
        unlock_file(&self.file);
    }
}

#[cfg(target_family = "unix")]
fn blocking_lock_exclusive(file: &std::fs::File) -> bool {
    use std::os::fd::AsRawFd;
    #[allow(unsafe_code)]
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    result == 0
}

#[cfg(target_family = "unix")]
fn unlock_file(file: &std::fs::File) {
    use std::os::fd::AsRawFd;
    #[allow(unsafe_code)]
    unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_UN);
    }
}

#[cfg(not(target_family = "unix"))]
fn blocking_lock_exclusive(_file: &std::fs::File) -> bool {
    false
}

#[cfg(not(target_family = "unix"))]
fn unlock_file(_file: &std::fs::File) {}

/// Local paths of a downloaded model's files.
///
/// `special_tokens` and `tokenizer_config` may be empty paths when the repo does
/// not ship those optional files; [`load_tokenizer`] handles the empty case.
pub(crate) struct DownloadedModel {
    pub model: PathBuf,
    pub tokenizer: PathBuf,
    pub config: PathBuf,
    pub special_tokens: PathBuf,
    pub tokenizer_config: PathBuf,
}

/// Download a model's files from HuggingFace and return their local paths.
///
/// `additional_files` are sibling files that must accompany `model_file` (e.g. a
/// `model.onnx.data` weight blob). They are downloaded into the same cache
/// directory; their paths are not returned because ONNX Runtime locates them by
/// sibling-name relative to `model_file` at load time.
///
/// Serializes concurrent first-time downloads across processes via a blocking
/// cross-process advisory lock, and self-heals stale `.lock`/`.part` files.
///
/// `manifest` is the module's checked-in `presets.sha256sum` (compiled in via
/// `include_str!`). Every downloaded file whose repo-relative path appears in the
/// manifest is verified against its pinned SHA-256 and the download fails on a
/// mismatch (fail-closed against a tampered/rolled-back mirror). Files absent from
/// the manifest — `Custom` repos, which ship no manifest — are downloaded without
/// verification, preserving the existing behaviour for user-supplied models. Pass
/// `None` to skip verification entirely.
pub(crate) fn download_model_files(
    repo_name: &str,
    model_file: &str,
    additional_files: &[String],
    cache_directory: &Path,
    manifest: Option<&str>,
    err: ErrCtor,
) -> crate::Result<DownloadedModel> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        download_model_files_inner(repo_name, model_file, additional_files, cache_directory, manifest, err)
    })) {
        Ok(result) => result,
        Err(payload) => {
            let panic_msg = panic_to_string(payload);
            Err(ort_missing_or(err, format!("Model download panicked: {panic_msg}")))
        }
    }
}

/// Fetch a companion file (tokenizer/config/…) trying the model's own directory
/// first, then the repo root.
///
/// Consolidated repos (e.g. `xberg-io/reranker-models`) co-locate every file for
/// a model under a `<name>/` subdir, so `<model_dir>/tokenizer.json` is correct.
/// Standard HF repos keep the model in `onnx/` but the tokenizer at the root, so
/// the root fallback covers those (and arbitrary `Custom` repos). Runs each
/// candidate under the download watchdog.
///
/// Returns the local cache path plus the repo-relative path that actually
/// resolved, so the caller can look that path up in the sha256 manifest.
fn fetch_companion(
    api: &hf_hub::HFClientSync,
    repo_name: &str,
    model_dir: Option<&str>,
    file_name: &str,
) -> Result<(PathBuf, String), String> {
    let candidates: Vec<String> = match model_dir {
        Some(dir) if !dir.is_empty() => vec![format!("{dir}/{file_name}"), file_name.to_string()],
        _ => vec![file_name.to_string()],
    };
    let mut last_err = String::new();
    for candidate in candidates {
        let api = api.clone();
        let repo = repo_name.to_string();
        let path = candidate.clone();
        match crate::model_download::with_download_deadline(&format!("{repo}/{candidate}"), move || {
            let (owner, name) = hf_hub::split_id(&repo);
            api.model(owner, name)
                .download_file()
                .filename(path)
                .send()
                .map_err(|e| e.to_string())
        }) {
            Ok(path) => return Ok((path, candidate)),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// Verify a downloaded file against the module's sha256 manifest.
///
/// When `manifest` lists `repo_path`, the file at `local` must hash to the pinned
/// value or an error is returned (fail-closed against tamper/rollback). Paths not
/// in the manifest are left unverified — this covers `Custom` repos (no manifest)
/// and any companion a preset deliberately does not pin. An empty `repo_path`
/// (an optional companion that was not downloaded) is a no-op.
fn verify_downloaded(manifest: &[(String, String)], repo_path: &str, local: &Path, err: ErrCtor) -> crate::Result<()> {
    if repo_path.is_empty() {
        return Ok(());
    }
    if let Some((_, sha256)) = manifest.iter().find(|(path, _)| path == repo_path) {
        crate::model_download::verify_sha256(local, sha256, repo_path).map_err(err)?;
    }
    Ok(())
}

fn download_model_files_inner(
    repo_name: &str,
    model_file: &str,
    additional_files: &[String],
    cache_directory: &Path,
    manifest: Option<&str>,
    err: ErrCtor,
) -> crate::Result<DownloadedModel> {
    let manifest: Vec<(String, String)> = match manifest {
        Some(content) => crate::model_download::parse_sha256_manifest(content)
            .map_err(|e| err(format!("Invalid sha256 manifest for {repo_name}: {e}")))?,
        None => Vec::new(),
    };

    let _download_lock = ProcessDownloadLock::acquire(cache_directory, repo_name);
    cleanup_stale_locks(cache_directory, repo_name);

    let api = crate::model_download::hf_client_builder()
        .cache_dir(cache_directory.to_path_buf())
        .build_sync()
        .map_err(|e| err(format!("Failed to create HF API client: {e}")))?;

    let model = {
        let api = api.clone();
        let file = model_file.to_string();
        let cache_dir = cache_directory.to_path_buf();
        let repo = repo_name.to_string();
        crate::model_download::with_download_deadline(&format!("{repo}/{model_file}"), move || {
            let (owner, name) = hf_hub::split_id(&repo);
            api.model(owner, name)
                .download_file()
                .filename(file)
                .send()
                .map_err(|e| {
                    let hint = if matches!(e, hf_hub::HFError::CacheLockTimeout { .. }) {
                        lock_acquisition_hint(&cache_dir, &repo)
                    } else {
                        String::new()
                    };
                    format!("{e}{hint}")
                })
        })
    }
    .map_err(|e| err(format!("Failed to download {model_file} from {repo_name}: {e}")))?;
    verify_downloaded(&manifest, model_file, &model, err)?;

    for sibling in additional_files {
        let sib_path = {
            let api = api.clone();
            let repo = repo_name.to_string();
            let sib = sibling.clone();
            crate::model_download::with_download_deadline(&format!("{repo}/{sibling}"), move || {
                let (owner, name) = hf_hub::split_id(&repo);
                api.model(owner, name)
                    .download_file()
                    .filename(sib)
                    .send()
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| {
                err(format!(
                    "Failed to download sibling file {sibling} from {repo_name}: {e}"
                ))
            })?
        };
        verify_downloaded(&manifest, sibling, &sib_path, err)?;
    }

    let model_dir = Path::new(model_file)
        .parent()
        .and_then(|p| p.to_str())
        .filter(|s| !s.is_empty());

    let (tokenizer, tokenizer_rel) = fetch_companion(&api, repo_name, model_dir, "tokenizer.json")
        .map_err(|e| err(format!("Failed to download tokenizer.json: {e}")))?;
    verify_downloaded(&manifest, &tokenizer_rel, &tokenizer, err)?;

    let (config, config_rel) = fetch_companion(&api, repo_name, model_dir, "config.json")
        .map_err(|e| err(format!("Failed to download config.json: {e}")))?;
    verify_downloaded(&manifest, &config_rel, &config, err)?;

    let (special_tokens, special_tokens_rel) =
        fetch_companion(&api, repo_name, model_dir, "special_tokens_map.json").unwrap_or_default();
    verify_downloaded(&manifest, &special_tokens_rel, &special_tokens, err)?;

    let (tokenizer_config, tokenizer_config_rel) =
        fetch_companion(&api, repo_name, model_dir, "tokenizer_config.json").unwrap_or_default();
    verify_downloaded(&manifest, &tokenizer_config_rel, &tokenizer_config, err)?;

    Ok(DownloadedModel {
        model,
        tokenizer,
        config,
        special_tokens,
        tokenizer_config,
    })
}

/// Load and configure a tokenizer with `BatchLongest` padding and truncation.
///
/// Reads `pad_token_id` from `config.json` and `model_max_length`/`pad_token`
/// from `tokenizer_config.json` (both optional, sensible defaults applied), then
/// merges any special tokens declared in `special_tokens_map.json`. `max_length`
/// is capped at the model's declared maximum.
pub(crate) fn load_tokenizer(
    files: &DownloadedModel,
    max_length: usize,
    err: ErrCtor,
) -> crate::Result<tokenizers::Tokenizer> {
    use tokenizers::{AddedToken, PaddingParams, PaddingStrategy, TruncationParams};

    let config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&files.config).map_err(|e| err(format!("Failed to read config.json: {e}")))?,
    )
    .map_err(|e| err(format!("Failed to parse config.json: {e}")))?;

    let tokenizer_config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&files.tokenizer_config)
            .map_err(|e| err(format!("Failed to read tokenizer_config.json: {e}")))?,
    )
    .map_err(|e| err(format!("Failed to parse tokenizer_config.json: {e}")))?;

    let mut tokenizer = tokenizers::Tokenizer::from_file(&files.tokenizer)
        .map_err(|e| err(format!("Failed to load tokenizer: {e}")))?;

    let model_max_length = tokenizer_config["model_max_length"].as_f64().unwrap_or(512.0) as usize;
    let max_length = max_length.min(model_max_length);
    let pad_id = config["pad_token_id"].as_u64().unwrap_or(0) as u32;
    let pad_token = tokenizer_config["pad_token"].as_str().unwrap_or("[PAD]").to_string();

    tokenizer
        .with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_token,
            pad_id,
            ..Default::default()
        }))
        .with_truncation(Some(TruncationParams {
            max_length,
            ..Default::default()
        }))
        .map_err(|e| err(format!("Failed to configure tokenizer: {e}")))?;

    if let Ok(special_tokens_data) = std::fs::read(&files.special_tokens)
        && let Ok(serde_json::Value::Object(map)) = serde_json::from_slice(&special_tokens_data)
    {
        for (_, value) in &map {
            if let Some(content) = value.as_str() {
                let _ = tokenizer.add_special_tokens([AddedToken {
                    content: content.to_string(),
                    special: true,
                    ..Default::default()
                }]);
            } else if value.is_object()
                && let (Some(content), Some(single_word), Some(lstrip), Some(rstrip), Some(normalized)) = (
                    value["content"].as_str(),
                    value["single_word"].as_bool(),
                    value["lstrip"].as_bool(),
                    value["rstrip"].as_bool(),
                    value["normalized"].as_bool(),
                )
            {
                let _ = tokenizer.add_special_tokens([AddedToken {
                    content: content.to_string(),
                    special: true,
                    single_word,
                    lstrip,
                    rstrip,
                    normalized,
                }]);
            }
        }
    }

    Ok(tokenizer)
}

/// Build an ORT session for `model_path` with the standard xberg configuration:
/// `GraphOptimizationLevel::All`, an intra-op thread budget resolved from the
/// concurrency config, a single inter-op thread, and the execution provider
/// selected by [`crate::ort_discovery::apply_execution_providers`].
///
/// The build runs inside `catch_unwind` because ORT can panic on a missing or
/// incompatible native library; such failures map to
/// [`crate::XbergError::MissingDependency`].
pub(crate) fn build_session(
    model_path: &Path,
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    err: ErrCtor,
) -> crate::Result<ort::session::Session> {
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(None);

    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut builder = ort::session::Session::builder()?;
        builder = builder
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::All)
            .map_err(|e| ort::Error::new(e.message()))?;
        builder = builder
            .with_intra_threads(thread_budget)
            .map_err(|e| ort::Error::new(e.message()))?;
        builder = builder
            .with_inter_threads(1)
            .map_err(|e| ort::Error::new(e.message()))?;
        builder = crate::ort_discovery::apply_execution_providers(builder, accel)?;
        builder.commit_from_file(model_path)
    }))
    .map_err(|payload| {
        ort_missing_or(
            err,
            format!("ONNX Runtime initialization panicked: {}", panic_to_string(payload)),
        )
    })?
    .map_err(|e| ort_missing_or(err, format!("Failed to create ONNX session: {e}")))
}

/// Resolve the cache directory for a module's models, honoring an explicit
/// override and otherwise falling back to `~/.cache/xberg/<module>/`.
///
/// Only `sparse_embeddings` and `late_interaction` route through this shared
/// helper; `embeddings`/`reranking` keep their own local wrappers, so gate it to
/// its callers to avoid a dead-code warning in reranker/embeddings-only builds.
#[cfg(any(feature = "sparse-embeddings", feature = "late-interaction"))]
pub(crate) fn resolve_cache_dir(module: &str, cache_dir: Option<PathBuf>) -> PathBuf {
    cache_dir.unwrap_or_else(|| crate::cache_dir::resolve_cache_dir(module))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embed_err(msg: String) -> crate::XbergError {
        crate::XbergError::embedding(msg)
    }

    fn write_file_aged(path: &Path, age_secs: u64) {
        std::fs::write(path, b"x").unwrap();
        let mtime = std::time::SystemTime::now() - std::time::Duration::from_secs(age_secs);
        let ft = filetime::FileTime::from_system_time(mtime);
        filetime::set_file_mtime(path, ft).unwrap();
    }

    #[test]
    fn looks_like_ort_error_detects_keywords() {
        assert!(looks_like_ort_error("failed to load libonnxruntime.so"));
        assert!(looks_like_ort_error("An error occurred while loading the model"));
        assert!(!looks_like_ort_error("some unrelated parsing failure"));
    }

    #[test]
    fn panic_to_string_handles_str_and_string_and_other() {
        assert_eq!(panic_to_string(Box::new("boom")), "boom");
        assert_eq!(panic_to_string(Box::new(String::from("kaboom"))), "kaboom");
        assert_eq!(panic_to_string(Box::new(42_u8)), "Unknown panic");
    }

    #[test]
    fn ort_missing_or_maps_ort_errors_to_missing_dependency() {
        let e = ort_missing_or(embed_err, "libonnxruntime not found".to_string());
        assert!(matches!(e, crate::XbergError::MissingDependency(_)));
        let e = ort_missing_or(embed_err, "generic failure".to_string());
        assert!(matches!(e, crate::XbergError::Embedding { .. }));
    }

    #[test]
    fn lock_acquisition_hint_contains_recovery_commands() {
        let hint = lock_acquisition_hint(Path::new("/tmp/cache"), "org/model");
        assert!(hint.contains("models--org--model"));
        assert!(hint.contains("*.lock"));
        assert!(hint.contains("*.part"));
    }

    #[test]
    fn cleanup_stale_locks_nonexistent_dir_is_noop() {
        cleanup_stale_locks(Path::new("/nonexistent/xberg/cache"), "org/model");
    }

    #[test]
    fn cleanup_stale_locks_removes_old_lock_and_part() {
        let tmp = tempfile::tempdir().unwrap();
        let blobs = tmp.path().join("models--org--model").join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        let lock = blobs.join("abc.lock");
        let part = blobs.join("abc.part");
        write_file_aged(&lock, 60 * 60);
        write_file_aged(&part, 60 * 60);
        cleanup_stale_locks(tmp.path(), "org/model");
        assert!(!lock.exists(), "stale lock should be removed");
        assert!(!part.exists(), "stale part should be removed");
    }

    #[test]
    fn cleanup_stale_locks_leaves_recent_files_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let blobs = tmp.path().join("models--org--model").join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        let lock = blobs.join("abc.lock");
        write_file_aged(&lock, 5);
        cleanup_stale_locks(tmp.path(), "org/model");
        assert!(lock.exists(), "recent lock should be left alone");
    }

    #[test]
    fn process_download_lock_acquire_creates_and_releases() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let guard = ProcessDownloadLock::acquire(tmp.path(), "org/model");
            assert!(guard.is_some(), "should acquire lock on unix");
            let lock_file = tmp.path().join("models--org--model").join(".xberg-download.lock");
            assert!(lock_file.exists());
        }
        let guard = ProcessDownloadLock::acquire(tmp.path(), "org/model");
        assert!(guard.is_some(), "should re-acquire after previous guard dropped");
    }

    #[cfg(any(feature = "sparse-embeddings", feature = "late-interaction"))]
    #[test]
    fn resolve_cache_dir_honors_override() {
        let custom = PathBuf::from("/tmp/custom-xberg-cache");
        assert_eq!(resolve_cache_dir("embeddings", Some(custom.clone())), custom);
    }

    #[test]
    fn verify_downloaded_errors_on_checksum_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("model.onnx");
        std::fs::write(&file, b"tampered bytes").unwrap();
        let manifest = vec![("name/model.onnx".to_string(), "0".repeat(64))];
        let result = verify_downloaded(&manifest, "name/model.onnx", &file, embed_err);
        assert!(result.is_err(), "tampered file must fail checksum verification");
    }

    #[test]
    fn verify_downloaded_passes_on_checksum_match() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("model.onnx");
        std::fs::write(&file, b"pinned content").unwrap();
        let digest = "28f10de8a12ace2df7c733d697168479b5707cdb2a21df8561cabda49473e3c1";
        let manifest = vec![("name/model.onnx".to_string(), digest.to_string())];
        verify_downloaded(&manifest, "name/model.onnx", &file, embed_err)
            .expect("matching file must pass verification");
    }

    #[test]
    fn verify_downloaded_skips_unlisted_and_empty_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("model.onnx");
        std::fs::write(&file, b"anything").unwrap();
        let manifest = vec![("other/model.onnx".to_string(), "0".repeat(64))];
        verify_downloaded(&manifest, "name/model.onnx", &file, embed_err).expect("unlisted path is skipped");
        verify_downloaded(&manifest, "", &file, embed_err).expect("empty path is a no-op");
    }
}
