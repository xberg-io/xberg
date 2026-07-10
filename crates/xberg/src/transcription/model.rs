//! Whisper model resolution: map a [`WhisperModel`] variant to on-disk ONNX paths,
//! downloading from Hugging Face Hub on first use.
//!
//! # Cache layout
//!
//! ```text
//! {cache_root}/whisper/{model_size}/
//!   encoder.onnx
//!   decoder.onnx
//!   decoder_with_past.onnx
//!   decoder_model_merged.onnx_data   (LargeV3 only)
//!   tokenizer.json
//!   config.json
//! ```
//!
//! The `{cache_root}` defaults to the centralized xberg cache directory
//! (`~/.cache/xberg/whisper` on Linux/macOS, `%LOCALAPPDATA%/xberg/whisper`
//! on Windows), resolved via `crate::cache_dir::resolve_cache_dir`.
//! Pass `cache_dir` to override.
//!
//! # HF repos
//!
//! Tiny, Base, and Small are fetched from `onnx-community/whisper-{size}`;
//! Medium and LargeV3 from `Xenova/whisper-{size}` (onnx-community does not
//! publish ONNX exports for those two sizes). All use the same file layout.
//!
//! # Decoder layout
//!
//! Small, Medium, and LargeV3 export a single merged decoder file
//! (`onnx/decoder_model_merged.onnx`) used for both the initial and KV-cache
//! passes. LargeV3 additionally carries its weights in an external
//! `onnx/decoder_model_merged.onnx_data` shard (its decoder exceeds the 2 GiB
//! protobuf limit); Small and Medium are self-contained. Tiny and Base ship
//! separate `decoder_model.onnx` / `decoder_with_past_model.onnx` files.

#[cfg(feature = "transcription")]
use std::path::{Path, PathBuf};

#[cfg(feature = "transcription")]
use crate::core::config::transcription::WhisperModel;

/// On-disk paths for all files needed to load a Whisper model in an ORT session.
#[cfg(feature = "transcription")]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct WhisperModelPaths {
    /// Encoder ONNX model: `onnx/encoder_model.onnx`.
    pub encoder: PathBuf,
    /// Decoder ONNX model (without KV-cache past): `onnx/decoder_model.onnx`.
    /// For sharded variants (Small+) this is the merged decoder.
    pub decoder: PathBuf,
    /// Decoder ONNX model with KV-cache past: `onnx/decoder_with_past_model.onnx`.
    /// For sharded variants (Small+) this points to the same merged decoder as `decoder`.
    pub decoder_with_past: PathBuf,
    /// `tokenizer.json` — vocabulary + BPE rules.
    pub tokenizer: PathBuf,
    /// `config.json` — model hyper-parameters.
    pub config: PathBuf,
    /// Number of mel filter banks expected by this model's audio pre-processor.
    /// 80 for Tiny / Base / Small / Medium; 128 for LargeV3.
    pub n_mels: u32,
}

/// Errors that can occur while resolving Whisper model paths.
#[cfg(feature = "transcription")]
#[derive(Debug, thiserror::Error)]
#[cfg_attr(alef, alef(skip))]
pub enum WhisperModelError {
    /// The model is not cached locally and `allow_network` is `false`.
    #[error("network access disabled and model not cached: {0}")]
    ModelMissing(String),

    /// A file download from Hugging Face Hub failed.
    #[error("hf-hub download failed: {0}")]
    Download(String),

    /// An I/O error occurred (directory creation, file copy, etc.).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The resolved cache directory could not be determined or created.
    #[error("cache directory unavailable: {0}")]
    Cache(String),

    /// Hash verification was requested, but the resolver has no pinned checksum
    /// metadata for the mutable Hugging Face model refs it uses.
    #[error("hash verification is unavailable for unpinned Whisper model refs")]
    HashVerificationUnavailable,
}

/// Map a [`WhisperModel`] to its directory name inside the whisper cache root.
#[cfg(feature = "transcription")]
pub(crate) fn model_dirname(model: WhisperModel) -> &'static str {
    match model {
        WhisperModel::Tiny => "tiny",
        WhisperModel::Base => "base",
        WhisperModel::Small => "small",
        WhisperModel::Medium => "medium",
        WhisperModel::LargeV3 => "large-v3",
    }
}

/// Map a [`WhisperModel`] to its Hugging Face Hub repository identifier.
#[cfg(feature = "transcription")]
pub(crate) fn hf_repo(model: WhisperModel) -> &'static str {
    match model {
        WhisperModel::Tiny => "onnx-community/whisper-tiny",
        WhisperModel::Base => "onnx-community/whisper-base",
        WhisperModel::Small => "onnx-community/whisper-small",
        WhisperModel::Medium => "Xenova/whisper-medium",
        WhisperModel::LargeV3 => "Xenova/whisper-large-v3",
    }
}

/// Number of mel filter banks the audio pre-processor produces for `model`.
///
/// 80 for every model except LargeV3, which uses 128.
#[cfg(feature = "transcription")]
pub(crate) fn n_mels(model: WhisperModel) -> u32 {
    match model {
        WhisperModel::LargeV3 => 128,
        _ => 80,
    }
}

/// Returns `true` when the model ships its decoder as a single merged file
/// (`decoder_model_merged.onnx`), used for both the initial and KV-cache
/// passes (Small, Medium, LargeV3).
///
/// Tiny and Base ship separate `decoder_model.onnx` /
/// `decoder_with_past_model.onnx` files instead.
#[cfg(feature = "transcription")]
fn is_sharded(model: WhisperModel) -> bool {
    matches!(
        model,
        WhisperModel::Small | WhisperModel::Medium | WhisperModel::LargeV3
    )
}

/// Returns `true` when the merged decoder carries its weights in a separate
/// external `.onnx_data` shard. Only LargeV3's decoder exceeds the 2 GB
/// protobuf limit; Small and Medium ship a self-contained merged decoder.
#[cfg(feature = "transcription")]
fn has_external_data_shard(model: WhisperModel) -> bool {
    matches!(model, WhisperModel::LargeV3)
}

/// Remote paths (relative to the repo root on HF Hub) for the files that
/// must be downloaded for `model`.
///
/// Returns a `Vec` of `(remote_path, local_filename)` pairs where
/// `remote_path` is the HF Hub path and `local_filename` is the canonical
/// name stored in the local cache directory.
#[cfg(feature = "transcription")]
fn model_files(model: WhisperModel) -> Vec<(&'static str, &'static str)> {
    if is_sharded(model) {
        let mut files = vec![
            ("onnx/encoder_model.onnx", "encoder.onnx"),
            ("onnx/decoder_model_merged.onnx", "decoder.onnx"),
        ];
        if has_external_data_shard(model) {
            files.push(("onnx/decoder_model_merged.onnx_data", "decoder.onnx_data"));
        }
        files.push(("tokenizer.json", "tokenizer.json"));
        files.push(("config.json", "config.json"));
        files
    } else {
        vec![
            ("onnx/encoder_model.onnx", "encoder.onnx"),
            ("onnx/decoder_model.onnx", "decoder.onnx"),
            ("onnx/decoder_with_past_model.onnx", "decoder_with_past.onnx"),
            ("tokenizer.json", "tokenizer.json"),
            ("config.json", "config.json"),
        ]
    }
}

/// Returns `true` when all required local files already exist in `target_dir`.
#[cfg(feature = "transcription")]
fn all_files_cached(target_dir: &Path, model: WhisperModel) -> bool {
    model_files(model)
        .into_iter()
        .all(|(_, local_name)| target_dir.join(local_name).exists())
}

#[cfg(feature = "transcription")]
struct ProcessDownloadLock {
    file: std::fs::File,
}

#[cfg(feature = "transcription")]
impl ProcessDownloadLock {
    /// Acquire the cross-process download lock for `repo_name`.
    ///
    /// Lock file location: `<cache_dir>/models--<repo>/.kbz-download.lock`.
    /// Returns `None` when the lock file cannot be created or the advisory lock
    /// cannot be acquired — the caller proceeds without the lock (no worse than
    /// the previous behavior before this guard existed).
    fn acquire(cache_dir: &Path, repo_name: &str) -> Option<Self> {
        let folder = format!("models--{}", repo_name.replace('/', "--"));
        let model_dir = cache_dir.join(folder);
        if let Err(error) = std::fs::create_dir_all(&model_dir) {
            tracing::debug!(?error, "Could not create model dir for download lock");
            return None;
        }
        let lock_path = model_dir.join(".kbz-download.lock");
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

#[cfg(feature = "transcription")]
impl Drop for ProcessDownloadLock {
    fn drop(&mut self) {
        unlock_file(&self.file);
    }
}

/// Acquire a blocking exclusive advisory lock on `file`. Returns `true` on success.
#[cfg(all(feature = "transcription", target_family = "unix"))]
fn blocking_lock_exclusive(file: &std::fs::File) -> bool {
    use std::os::fd::AsRawFd;
    #[allow(unsafe_code)]
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    result == 0
}

/// Release an advisory lock held on `file`.
#[cfg(all(feature = "transcription", target_family = "unix"))]
fn unlock_file(file: &std::fs::File) {
    use std::os::fd::AsRawFd;
    #[allow(unsafe_code)]
    unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_UN);
    }
}

/// Fallback for non-unix targets (Windows): no cross-process lock is taken.
///
/// `hf-hub` retains its own (best-effort, non-blocking) lock on these targets.
/// Returning `false` causes [`ProcessDownloadLock::acquire`] to yield `None`
/// and the caller proceeds directly to the hf-hub download path.
#[cfg(all(feature = "transcription", not(target_family = "unix")))]
fn blocking_lock_exclusive(_file: &std::fs::File) -> bool {
    false
}

/// No-op unlock for non-unix targets — no lock was taken.
#[cfg(all(feature = "transcription", not(target_family = "unix")))]
fn unlock_file(_file: &std::fs::File) {}

/// Resolve a Whisper model to on-disk ONNX paths, downloading from HF Hub if needed.
///
/// # Behaviour
///
/// 1. Compute `target_dir = {cache_root}/whisper/{model_size}/` (see module docs for
///    the full resolution order).
/// 2. If every required file already exists locally, return the paths immediately.
/// 3. If `allow_network` is `false` and the model is not cached, return
///    [`WhisperModelError::ModelMissing`].
/// 4. Acquire a cross-process advisory lock to serialise concurrent first-time
///    downloads (a killed process never permanently wedges the lock).
/// 5. Download each file via `hf-hub` into `target_dir` under canonical local names.
/// 6. Return populated [`WhisperModelPaths`].
///
/// # Errors
///
/// Returns [`WhisperModelError`] on I/O failures, download failures, or when the
/// model is unavailable and `allow_network` is `false`.
#[cfg(feature = "transcription")]
#[cfg_attr(alef, alef(skip))]
pub fn ensure_whisper_model(
    model: WhisperModel,
    cache_dir: Option<&Path>,
    allow_network: bool,
    verify_hash: bool,
) -> Result<WhisperModelPaths, WhisperModelError> {
    if verify_hash {
        return Err(WhisperModelError::HashVerificationUnavailable);
    }

    let whisper_cache = cache_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| crate::cache_dir::resolve_cache_dir("whisper"));
    let target_dir = whisper_cache.join(model_dirname(model));

    if all_files_cached(&target_dir, model) {
        tracing::debug!(
            dir = ?target_dir,
            model = model_dirname(model),
            "Whisper model already cached",
        );
        return Ok(build_paths(model, &target_dir));
    }

    if !allow_network {
        return Err(WhisperModelError::ModelMissing(hf_repo(model).to_string()));
    }

    std::fs::create_dir_all(&target_dir).map_err(|error| {
        WhisperModelError::Cache(format!(
            "Failed to create Whisper cache directory {}: {error}",
            target_dir.display()
        ))
    })?;

    let _download_lock = ProcessDownloadLock::acquire(&whisper_cache, hf_repo(model));

    if all_files_cached(&target_dir, model) {
        tracing::debug!(
            dir = ?target_dir,
            model = model_dirname(model),
            "Whisper model already cached (post-lock check)",
        );
        return Ok(build_paths(model, &target_dir));
    }

    let api = hf_hub::api::sync::ApiBuilder::from_env()
        .with_cache_dir(whisper_cache.clone())
        .with_progress(false)
        .build()
        .map_err(|error| WhisperModelError::Download(format!("Failed to initialise hf-hub API: {error}")))?;

    for (remote_path, local_name) in model_files(model) {
        let local_path = target_dir.join(local_name);

        if local_path.exists() {
            tracing::debug!(file = local_name, "Whisper file already present, skipping");
            continue;
        }

        tracing::info!(
            repo = hf_repo(model),
            remote = remote_path,
            local = %local_path.display(),
            "Downloading Whisper model file",
        );

        let downloaded = {
            let api = api.clone();
            let remote = remote_path.to_string();
            let repo_id = hf_repo(model).to_string();
            crate::model_download::with_download_deadline(&format!("{}/{remote_path}", hf_repo(model)), move || {
                api.model(repo_id).get(&remote).map_err(|e| e.to_string())
            })
        }
        .map_err(|error| {
            WhisperModelError::Download(format!(
                "Failed to download '{remote_path}' from {}: {error}",
                hf_repo(model)
            ))
        })?;

        std::fs::copy(&downloaded, &local_path).map_err(|error| {
            WhisperModelError::Io(std::io::Error::other(format!(
                "Failed to copy '{remote_path}' to {}: {error}",
                local_path.display()
            )))
        })?;

        tracing::debug!(
            local = %local_path.display(),
            "Whisper file ready",
        );
    }

    Ok(build_paths(model, &target_dir))
}

/// Construct [`WhisperModelPaths`] from a resolved `target_dir`.
///
/// For sharded models (Small, Medium, LargeV3) both `decoder` and
/// `decoder_with_past` point at the merged decoder file.
#[cfg(feature = "transcription")]
fn build_paths(model: WhisperModel, target_dir: &Path) -> WhisperModelPaths {
    let encoder = target_dir.join("encoder.onnx");
    let tokenizer = target_dir.join("tokenizer.json");
    let config = target_dir.join("config.json");

    let (decoder, decoder_with_past) = if is_sharded(model) {
        let merged = target_dir.join("decoder.onnx");
        (merged.clone(), merged)
    } else {
        (
            target_dir.join("decoder.onnx"),
            target_dir.join("decoder_with_past.onnx"),
        )
    };

    WhisperModelPaths {
        encoder,
        decoder,
        decoder_with_past,
        tokenizer,
        config,
        n_mels: n_mels(model),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "transcription")]
    use crate::core::config::transcription::WhisperModel;

    #[cfg(feature = "transcription")]
    #[test]
    fn model_dirname_is_deterministic() {
        assert_eq!(model_dirname(WhisperModel::Tiny), "tiny");
        assert_eq!(model_dirname(WhisperModel::Base), "base");
        assert_eq!(model_dirname(WhisperModel::Small), "small");
        assert_eq!(model_dirname(WhisperModel::Medium), "medium");
        assert_eq!(model_dirname(WhisperModel::LargeV3), "large-v3");
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn hf_repo_points_at_published_onnx_exports() {
        assert_eq!(hf_repo(WhisperModel::Tiny), "onnx-community/whisper-tiny");
        assert_eq!(hf_repo(WhisperModel::Base), "onnx-community/whisper-base");
        assert_eq!(hf_repo(WhisperModel::Small), "onnx-community/whisper-small");
        assert_eq!(hf_repo(WhisperModel::Medium), "Xenova/whisper-medium");
        assert_eq!(hf_repo(WhisperModel::LargeV3), "Xenova/whisper-large-v3");
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn only_large_v3_uses_external_data_shard() {
        assert!(!has_external_data_shard(WhisperModel::Small));
        assert!(!has_external_data_shard(WhisperModel::Medium));
        assert!(has_external_data_shard(WhisperModel::LargeV3));
        assert!(
            !model_files(WhisperModel::Small)
                .iter()
                .any(|(remote, _)| remote.ends_with(".onnx_data"))
        );
        assert!(
            model_files(WhisperModel::LargeV3)
                .iter()
                .any(|(remote, _)| remote.ends_with(".onnx_data"))
        );
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn n_mels_is_128_only_for_large_v3() {
        for model in [
            WhisperModel::Tiny,
            WhisperModel::Base,
            WhisperModel::Small,
            WhisperModel::Medium,
        ] {
            assert_eq!(n_mels(model), 80, "{model:?} should have 80 mels");
        }
        assert_eq!(n_mels(WhisperModel::LargeV3), 128);
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn ensure_model_returns_missing_when_network_disabled_and_uncached() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = ensure_whisper_model(WhisperModel::Tiny, Some(tmp.path()), false, false);
        assert!(
            matches!(result, Err(WhisperModelError::ModelMissing(_))),
            "expected ModelMissing, got: {result:?}",
        );
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn verify_hash_requests_fail_fast() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = ensure_whisper_model(WhisperModel::Tiny, Some(tmp.path()), false, true);
        assert!(
            matches!(result, Err(WhisperModelError::HashVerificationUnavailable)),
            "expected HashVerificationUnavailable, got: {result:?}",
        );
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn sharded_models_use_merged_decoder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target_dir = tmp.path().join("small");
        std::fs::create_dir_all(&target_dir).unwrap();

        for (_, local_name) in model_files(WhisperModel::Small) {
            std::fs::write(target_dir.join(local_name), b"stub").unwrap();
        }

        let paths = ensure_whisper_model(WhisperModel::Small, Some(tmp.path()), false, false)
            .expect("should succeed when all files are cached");

        assert_eq!(
            paths.decoder, paths.decoder_with_past,
            "sharded model: decoder and decoder_with_past must point at the merged file",
        );
        assert_eq!(paths.n_mels, 80);
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn non_sharded_models_have_distinct_decoder_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target_dir = tmp.path().join("tiny");
        std::fs::create_dir_all(&target_dir).unwrap();

        for (_, local_name) in model_files(WhisperModel::Tiny) {
            std::fs::write(target_dir.join(local_name), b"stub").unwrap();
        }

        let paths = ensure_whisper_model(WhisperModel::Tiny, Some(tmp.path()), false, false)
            .expect("should succeed when all files are cached");

        assert_ne!(
            paths.decoder, paths.decoder_with_past,
            "non-sharded model: decoder and decoder_with_past must be distinct files",
        );
        assert_eq!(paths.n_mels, 80);
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn large_v3_uses_128_mels_from_cached_paths() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target_dir = tmp.path().join("large-v3");
        std::fs::create_dir_all(&target_dir).unwrap();

        for (_, local_name) in model_files(WhisperModel::LargeV3) {
            std::fs::write(target_dir.join(local_name), b"stub").unwrap();
        }

        let paths = ensure_whisper_model(WhisperModel::LargeV3, Some(tmp.path()), false, false)
            .expect("should succeed when all files are cached");

        assert_eq!(paths.n_mels, 128);
    }
}
