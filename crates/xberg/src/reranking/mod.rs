//! Cross-encoder reranking support.
//!
//! This module provides `(query, document)` pair scoring using ONNX cross-encoder
//! models. Reranking is the standard "second pass" in retrieval pipelines:
//! a first-pass embedding search retrieves a candidate set cheaply; reranking
//! rescores and reorders those candidates by true relevance to the query.
//!
//! Three backend variants are supported:
//! - **Local ONNX** — cross-encoder models (ms-marco, bge-reranker) via ONNX Runtime.
//! - **liter-llm** — provider-hosted rerankers (Cohere, Jina, Voyage) via an API.
//! - **In-process plugin** — caller-supplied backends registered via
//!   [`crate::plugins::register_reranker_backend`].
//!
//! # Download/cache machinery
//!
//! The ONNX path vendors both the download/lock machinery and the three ORT
//! utility helpers (`onnx_runtime_install_message`, `panic_to_string`,
//! `looks_like_ort_error`) from `crates/xberg/src/embeddings/mod.rs`.
//! They are duplicated here because `crate::embeddings` requires the
//! `embedding-presets` feature, which may be absent in a `reranker`-only build.
//! Keep vendored copies in sync with `embeddings/mod.rs`.
//!
//! Since v5.0.0.

#[cfg(feature = "reranker")]
pub mod engine;

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[cfg(feature = "reranker")]
use ahash::AHashMap;
#[cfg(feature = "reranker")]
use engine::RerankerEngine;
#[cfg(feature = "reranker")]
use std::sync::{Arc, RwLock};

#[cfg(feature = "reranker")]
type CachedEngine = Arc<RerankerEngine>;

#[cfg(feature = "reranker")]
static ENGINE_CACHE: LazyLock<RwLock<AHashMap<String, CachedEngine>>> = LazyLock::new(|| RwLock::new(AHashMap::new()));

/// Global semaphore that limits concurrent ONNX reranker inference calls.
///
/// Prevents resource exhaustion when many async callers invoke `rerank_async`
/// against the ONNX path (Preset/Custom variants) simultaneously. The Llm and
/// Plugin variants short-circuit out of `rerank_async` before reaching the
/// semaphore. The permit count matches the thread budget used by the embedding
/// semaphore.
///
/// Since v5.0.0.
#[cfg(all(feature = "reranker", feature = "tokio-runtime"))]
static RERANK_SEMAPHORE: LazyLock<Arc<tokio::sync::Semaphore>> = LazyLock::new(|| {
    let budget = crate::core::config::concurrency::resolve_thread_budget(None);
    Arc::new(tokio::sync::Semaphore::new(budget))
});

/// A single document returned by the reranker, with its position in the input and score.
///
/// `index` maps back to the caller's original document list, so metadata arrays
/// (e.g. IDs, paths) can be reordered without passing them through the reranker.
///
/// Since v5.0.0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RerankedDocument {
    /// Position of this document in the original input `documents` slice.
    pub index: usize,
    /// Relevance score in `[0, 1]`. Higher means more relevant to the query.
    pub score: f32,
    /// The document text.
    pub document: String,
}

/// Metadata for a bundled reranker preset.
///
/// All string fields are owned `String` for FFI compatibility — instances are
/// safe to clone and pass across language boundaries.
///
/// Since v5.0.0.
#[cfg(feature = "reranker-presets")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerPreset {
    /// Short identifier (catalog name, e.g. `"bge-reranker-base"`).
    pub name: String,
    /// HuggingFace repository name for the model.
    pub model_repo: String,
    /// Path to the ONNX model file within the repo.
    pub model_file: String,
    /// Sibling files that must be downloaded alongside `model_file`.
    ///
    /// Empty for most presets. Used by repos that split the weight blob —
    /// e.g. `rozgo/bge-reranker-v2-m3` ships the model in `model.onnx` plus a
    /// co-located `model.onnx.data` payload.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_files: Vec<String>,
    /// Maximum token sequence length the model supports.
    pub max_length: usize,
    /// Human-readable description of the preset's intended use case.
    pub description: String,
}

/// All available reranker presets.
///
/// **Source of truth**: mirrors the `RerankerModel` catalog of
/// [fastembed-rs](https://github.com/Anush008/fastembed-rs) verbatim. Every
/// `model_repo` + `model_file` + `additional_files` triple here is the path
/// fastembed-rs uses, which they keep verified against the live HuggingFace
/// hub. When fastembed-rs publishes a new catalog entry, mirror it here in
/// one PR; do not invent paths.
///
/// Refresh procedure: read fastembed-rs `src/models/reranking.rs` on the
/// `main` branch and update this list to match. The `live-hf` CI job will
/// fail loudly if any preset path 404s.
///
/// Since v5.0.0.
#[cfg(feature = "reranker-presets")]
pub static RERANKER_PRESETS: LazyLock<Vec<RerankerPreset>> = LazyLock::new(|| {
    vec![
        RerankerPreset {
            name: "bge-reranker-base".to_string(),
            model_repo: "BAAI/bge-reranker-base".to_string(),
            model_file: "onnx/model.onnx".to_string(),
            additional_files: Vec::new(),
            max_length: 512,
            description: "BGE cross-encoder base (~278M params, EN + ZH). Best for: \
                general-purpose RAG, production deployments, English or Chinese documents."
                .to_string(),
        },
        RerankerPreset {
            name: "bge-reranker-v2-m3".to_string(),
            model_repo: "rozgo/bge-reranker-v2-m3".to_string(),
            model_file: "model.onnx".to_string(),
            additional_files: vec!["model.onnx.data".to_string()],
            max_length: 8192,
            description: "BGE cross-encoder v2 M3 (568M params, 100+ languages, 8192 max-len). \
                Best for: international documents, mixed-language retrieval. \
                Mirror of the official BAAI model; the weight is split into model.onnx + model.onnx.data."
                .to_string(),
        },
        RerankerPreset {
            name: "jina-reranker-v1-turbo-en".to_string(),
            model_repo: "jinaai/jina-reranker-v1-turbo-en".to_string(),
            model_file: "onnx/model.onnx".to_string(),
            additional_files: Vec::new(),
            max_length: 8192,
            description: "Jina reranker v1 turbo English (~37M params, 8192 max-len). \
                Best for: low-latency reranking, English documents, long-context retrieval."
                .to_string(),
        },
        RerankerPreset {
            name: "jina-reranker-v2-base-multilingual".to_string(),
            model_repo: "jinaai/jina-reranker-v2-base-multilingual".to_string(),
            model_file: "onnx/model.onnx".to_string(),
            additional_files: Vec::new(),
            max_length: 1024,
            description: "Jina reranker v2 base multilingual (~278M params, 1024 max-len, 100+ languages). \
                Best for: multilingual retrieval, balanced latency/quality."
                .to_string(),
        },
    ]
});

/// Friendly aliases mapped to catalog short-names.
///
/// `(alias, catalog_name)` pairs let callers say `"balanced"` or
/// `"multilingual"` and get a stable, opinionated default that may evolve
/// across releases. The catalog name is the underlying primitive — alias
/// resolution is single-hop (aliases cannot point at other aliases).
///
/// Since v5.0.0.
#[cfg(feature = "reranker-presets")]
const PRESET_ALIASES: &[(&str, &str)] = &[
    ("fast", "jina-reranker-v1-turbo-en"),
    ("balanced", "bge-reranker-base"),
    ("quality", "bge-reranker-v2-m3"),
    ("multilingual", "jina-reranker-v2-base-multilingual"),
];

/// Get a preset by name (returns an owned clone for FFI compatibility).
///
/// Lookup is case-sensitive. Checks the catalog literals first; falls back
/// to the alias table for the documented friendly names
/// (`fast` / `balanced` / `quality` / `multilingual`).
///
/// Since v5.0.0.
#[cfg(feature = "reranker-presets")]
pub(crate) fn get_preset(name: &str) -> Option<RerankerPreset> {
    if let Some(preset) = RERANKER_PRESETS.iter().find(|p| p.name == name) {
        return Some(preset.clone());
    }
    let resolved = PRESET_ALIASES.iter().find(|(alias, _)| *alias == name)?.1;
    RERANKER_PRESETS.iter().find(|p| p.name == resolved).cloned()
}

/// List all available reranker preset names (owned clones for FFI compatibility).
///
/// Returns the catalog short-names followed by the friendly aliases, so
/// `list_presets()[..4]` is the catalog and `list_presets()[4..]` is aliases.
///
/// Since v5.0.0.
#[cfg(feature = "reranker-presets")]
pub(crate) fn list_presets() -> Vec<String> {
    let mut out: Vec<String> = RERANKER_PRESETS.iter().map(|p| p.name.clone()).collect();
    out.extend(PRESET_ALIASES.iter().map(|(alias, _)| alias.to_string()));
    out
}

// ── ONNX Runtime helpers — vendored from embeddings/mod.rs ────────────────────
// These three tiny helpers are inlined here rather than imported from
// `crate::embeddings` because that module requires the `embedding-presets`
// feature; a build with `reranker` but without `embedding-presets` would fail
// to resolve `crate::embeddings::*`. Vendored copies kept in sync with
// `embeddings/mod.rs`.

/// Returns installation instructions for ONNX Runtime.
///
/// Vendored from `embeddings/mod.rs` — keep in sync.
#[cfg(feature = "reranker")]
fn onnx_runtime_install_message() -> String {
    #[cfg(all(windows, target_env = "gnu"))]
    {
        return "ONNX Runtime reranking is not supported on Windows MinGW builds. \
        ONNX Runtime requires MSVC toolchain. \
        Please use Windows MSVC builds or disable reranker feature."
            .to_string();
    }

    #[cfg(not(all(windows, target_env = "gnu")))]
    {
        "ONNX Runtime is required for reranking functionality. \
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
///
/// Vendored from `embeddings/mod.rs` — keep in sync.
#[cfg(feature = "reranker")]
fn looks_like_ort_error(msg: &str) -> bool {
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
///
/// Vendored from `embeddings/mod.rs` — keep in sync.
#[cfg(feature = "reranker")]
fn panic_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

// ── Download / lock machinery — vendored from embeddings/mod.rs ───────────────

/// How long a partial download must be idle before it is considered stale.
#[cfg(feature = "reranker")]
const STALE_DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Remove stale `.lock` and `.part` files left behind by interrupted downloads.
///
/// Vendored from `embeddings/mod.rs` — keep in sync with that implementation.
#[cfg(feature = "reranker")]
fn cleanup_stale_locks(cache_dir: &std::path::Path, repo_name: &str) {
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
#[cfg(feature = "reranker")]
fn lock_acquisition_hint(cache_dir: &std::path::Path, repo_name: &str) -> String {
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
///
/// Vendored from `embeddings/mod.rs` — keep in sync with that implementation.
#[cfg(feature = "reranker")]
struct ProcessDownloadLock {
    file: std::fs::File,
}

#[cfg(feature = "reranker")]
impl ProcessDownloadLock {
    fn acquire(cache_dir: &std::path::Path, repo_name: &str) -> Option<Self> {
        let folder = format!("models--{}", repo_name.replace('/', "--"));
        let model_dir = cache_dir.join(folder);
        if let Err(error) = std::fs::create_dir_all(&model_dir) {
            tracing::debug!(?error, "Could not create model dir for download lock");
            return None;
        }
        let lock_path = model_dir.join(".kbz-reranker-download.lock");
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

#[cfg(feature = "reranker")]
impl Drop for ProcessDownloadLock {
    fn drop(&mut self) {
        unlock_file(&self.file);
    }
}

#[cfg(all(feature = "reranker", target_family = "unix"))]
fn blocking_lock_exclusive(file: &std::fs::File) -> bool {
    use std::os::fd::AsRawFd;
    // SAFETY: `file` is a live, open file owned by the caller for the duration
    // of the call; `as_raw_fd()` yields a valid descriptor. `flock` with
    // `LOCK_EX` (no `LOCK_NB`) blocks until the advisory lock is granted and
    // mutates no Rust-visible state. The lock is released by `unlock_file` on
    // drop or by the kernel when the process exits.
    #[allow(unsafe_code)]
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    result == 0
}

#[cfg(all(feature = "reranker", target_family = "unix"))]
fn unlock_file(file: &std::fs::File) {
    use std::os::fd::AsRawFd;
    // SAFETY: `file` is a live, open file owned by the caller; `flock` with
    // `LOCK_UN` releases any advisory lock and mutates no Rust-visible state.
    #[allow(unsafe_code)]
    unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_UN);
    }
}

#[cfg(all(feature = "reranker", not(target_family = "unix")))]
fn blocking_lock_exclusive(_file: &std::fs::File) -> bool {
    false
}

#[cfg(all(feature = "reranker", not(target_family = "unix")))]
fn unlock_file(_file: &std::fs::File) {}

/// Download model files from HuggingFace and return their local paths.
///
/// Returns `(model_path, tokenizer_path, config_path, special_tokens_path, tokenizer_config_path)`.
///
/// `additional_files` are sibling files that must accompany `model_file` (e.g.
/// the `model.onnx.data` weight blob for `rozgo/bge-reranker-v2-m3`). They are
/// downloaded into the same cache directory; their returned `PathBuf`s are
/// discarded because ONNX Runtime locates them by sibling-name relative to
/// `model_file` at load time.
///
/// Vendored from `embeddings/mod.rs` — keep in sync with that implementation.
#[cfg(feature = "reranker")]
fn download_model_files(
    repo_name: &str,
    model_file: &str,
    additional_files: &[String],
    cache_directory: &std::path::Path,
) -> crate::Result<(
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
)> {
    let _download_lock = ProcessDownloadLock::acquire(cache_directory, repo_name);
    cleanup_stale_locks(cache_directory, repo_name);

    let api = hf_hub::api::sync::ApiBuilder::from_env()
        .with_cache_dir(cache_directory.to_path_buf())
        .with_progress(true)
        .build()
        .map_err(|e| crate::XbergError::reranking(format!("Failed to create HF API client: {e}")))?;

    let repo = api.model(repo_name.to_string());

    let model_path = repo.get(model_file).map_err(|e| {
        let hint = if matches!(e, hf_hub::api::sync::ApiError::LockAcquisition(_)) {
            lock_acquisition_hint(cache_directory, repo_name)
        } else {
            String::new()
        };
        crate::XbergError::reranking(format!("Failed to download {model_file} from {repo_name}: {e}{hint}"))
    })?;

    // Sibling files (e.g. `model.onnx.data`) must be present in the same cache
    // dir before ORT opens the model. hf-hub places them next to model_file
    // because they share the repo's blobs/snapshots layout.
    for sibling in additional_files {
        repo.get(sibling).map_err(|e| {
            crate::XbergError::reranking(format!(
                "Failed to download sibling file {sibling} from {repo_name}: {e}"
            ))
        })?;
    }

    let tokenizer_path = repo
        .get("tokenizer.json")
        .map_err(|e| crate::XbergError::reranking(format!("Failed to download tokenizer.json: {e}")))?;

    let config_path = repo
        .get("config.json")
        .map_err(|e| crate::XbergError::reranking(format!("Failed to download config.json: {e}")))?;

    let special_tokens_path = repo
        .get("special_tokens_map.json")
        .unwrap_or_else(|_| std::path::PathBuf::new());

    let tokenizer_config_path = repo
        .get("tokenizer_config.json")
        .unwrap_or_else(|_| std::path::PathBuf::new());

    Ok((
        model_path,
        tokenizer_path,
        config_path,
        special_tokens_path,
        tokenizer_config_path,
    ))
}

/// Load and configure a tokenizer for cross-encoder pair encoding.
///
/// Adapted from `embeddings/mod.rs::load_tokenizer`. Cross-encoders need
/// pair encoding, so the tokenizer is configured identically — the pair input
/// is handled at call time via `EncodeInput::Dual`.
#[cfg(feature = "reranker")]
fn load_tokenizer(
    tokenizer_path: &std::path::Path,
    config_path: &std::path::Path,
    special_tokens_path: &std::path::Path,
    tokenizer_config_path: &std::path::Path,
    max_length: usize,
) -> crate::Result<tokenizers::Tokenizer> {
    use tokenizers::{AddedToken, PaddingParams, PaddingStrategy, TruncationParams};

    let config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(config_path)
            .map_err(|e| crate::XbergError::reranking(format!("Failed to read config.json: {e}")))?,
    )
    .map_err(|e| crate::XbergError::reranking(format!("Failed to parse config.json: {e}")))?;

    let tokenizer_config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(tokenizer_config_path)
            .map_err(|e| crate::XbergError::reranking(format!("Failed to read tokenizer_config.json: {e}")))?,
    )
    .map_err(|e| crate::XbergError::reranking(format!("Failed to parse tokenizer_config.json: {e}")))?;

    let mut tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
        .map_err(|e| crate::XbergError::reranking(format!("Failed to load tokenizer: {e}")))?;

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
        .map_err(|e| crate::XbergError::reranking(format!("Failed to configure tokenizer: {e}")))?;

    if let Ok(special_tokens_data) = std::fs::read(special_tokens_path)
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

/// Resolve the cache directory for reranker models.
#[cfg(feature = "reranker")]
fn resolve_cache_dir(cache_dir: Option<std::path::PathBuf>) -> std::path::PathBuf {
    cache_dir.unwrap_or_else(|| crate::cache_dir::resolve_cache_dir("rerankers"))
}

/// Get or initialize a reranker engine from cache.
///
/// Downloads model files from HuggingFace if needed, loads the tokenizer,
/// creates an ORT session, and caches the engine for reuse.
#[cfg(feature = "reranker")]
fn get_or_init_engine(
    repo_name: &str,
    model_file: &str,
    additional_files: &[String],
    max_length: usize,
    cache_dir: Option<std::path::PathBuf>,
    accel: Option<crate::core::config::acceleration::AccelerationConfig>,
) -> crate::Result<Arc<RerankerEngine>> {
    let cache_directory = resolve_cache_dir(cache_dir);
    let engine_key = format!(
        "{repo_name}_{model_file}_{cache_directory}",
        cache_directory = cache_directory.display()
    );

    // Fast path: read lock
    {
        match ENGINE_CACHE.read() {
            Ok(cache) => {
                if let Some(cached) = cache.get(&engine_key) {
                    return Ok(Arc::clone(cached));
                }
            }
            Err(poison_error) => {
                let cache = poison_error.get_ref();
                if let Some(cached) = cache.get(&engine_key) {
                    return Ok(Arc::clone(cached));
                }
            }
        }
    }

    // Slow path: write lock + initialization
    {
        let mut cache = match ENGINE_CACHE.write() {
            Ok(guard) => guard,
            Err(poison_error) => poison_error.into_inner(),
        };

        // Double-check after acquiring write lock
        if let Some(cached) = cache.get(&engine_key) {
            return Ok(Arc::clone(cached));
        }

        crate::ort_discovery::ensure_ort_available();

        let (model_path, tokenizer_path, config_path, special_tokens_path, tokenizer_config_path) =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                download_model_files(repo_name, model_file, additional_files, &cache_directory)
            }))
            .map_err(|panic_payload| {
                let panic_msg = panic_to_string(panic_payload);
                if looks_like_ort_error(&panic_msg) {
                    crate::XbergError::MissingDependency(format!("ONNX Runtime - {}", onnx_runtime_install_message()))
                } else {
                    crate::XbergError::reranking(format!("Model download panicked: {panic_msg}"))
                }
            })??;

        let tokenizer = load_tokenizer(
            &tokenizer_path,
            &config_path,
            &special_tokens_path,
            &tokenizer_config_path,
            max_length,
        )?;

        let thread_budget = crate::core::config::concurrency::resolve_thread_budget(None);
        let session = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
            builder = crate::ort_discovery::apply_execution_providers(builder, accel.as_ref())?;
            builder.commit_from_file(&model_path)
        }))
        .map_err(|panic_payload| {
            let panic_msg = panic_to_string(panic_payload);
            if looks_like_ort_error(&panic_msg) {
                crate::XbergError::MissingDependency(format!("ONNX Runtime - {}", onnx_runtime_install_message()))
            } else {
                crate::XbergError::reranking(format!("ONNX Runtime initialization panicked: {panic_msg}"))
            }
        })?
        .map_err(|e| {
            let error_msg = e.to_string();
            if looks_like_ort_error(&error_msg) {
                crate::XbergError::MissingDependency(format!("ONNX Runtime - {}", onnx_runtime_install_message()))
            } else {
                crate::XbergError::reranking(format!("Failed to create ONNX session: {e}"))
            }
        })?;

        let new_engine = Arc::new(RerankerEngine::new(tokenizer, session));
        cache.insert(engine_key, Arc::clone(&new_engine));

        Ok(new_engine)
    }
}

/// Resolve model info (repo, model file, additional_files, max_length) from a RerankerModelType config.
#[cfg(feature = "reranker")]
fn resolve_model_info(
    model_type: &crate::core::config::RerankerModelType,
) -> crate::Result<(String, String, Vec<String>, usize)> {
    match model_type {
        crate::core::config::RerankerModelType::Preset { name } => {
            let preset = get_preset(name)
                .ok_or_else(|| crate::XbergError::reranking(format!("Unknown reranker preset: {name}")))?;
            Ok((
                preset.model_repo,
                preset.model_file,
                preset.additional_files,
                preset.max_length,
            ))
        }
        crate::core::config::RerankerModelType::Custom {
            model_id,
            model_file,
            additional_files,
            max_length,
        } => {
            let len = match max_length.unwrap_or(512) {
                n if n <= 0 => {
                    return Err(crate::XbergError::Validation {
                        message: format!("max_length must be positive, got {n}"),
                        source: None,
                    });
                }
                n => n as usize,
            };
            let file = model_file.clone().unwrap_or_else(|| "onnx/model.onnx".to_string());
            Ok((model_id.clone(), file, additional_files.clone(), len))
        }
        crate::core::config::RerankerModelType::Llm { .. } => Err(crate::XbergError::reranking(
            "LLM rerankers have no local model to warm or download — the provider serves them over HTTP.",
        )),
        crate::core::config::RerankerModelType::Plugin { .. } => Err(crate::XbergError::reranking(
            "Plugin rerankers have no local model to warm or download — the registered backend owns the model lifecycle.",
        )),
    }
}

/// Validate that a plugin backend returned the correct number of scores.
#[cfg(feature = "reranker")]
fn validate_reranker_output(scores: &[f32], expected_count: usize, backend_name: &str) -> crate::Result<()> {
    if scores.len() != expected_count {
        return Err(crate::XbergError::Validation {
            message: format!(
                "Reranker backend '{backend_name}' returned {got} scores for {expected} documents",
                got = scores.len(),
                expected = expected_count,
            ),
            source: None,
        });
    }
    Ok(())
}

/// Apply sigmoid to convert a raw logit to a `[0, 1]` score.
#[cfg(any(feature = "reranker", test))]
pub(crate) fn sigmoid_f32(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Build the sorted, optionally truncated result vector from raw logits.
#[cfg(any(feature = "reranker", test))]
fn build_results(documents: &[String], logits: Vec<f32>, top_k: Option<usize>) -> Vec<RerankedDocument> {
    let mut results: Vec<RerankedDocument> = documents
        .iter()
        .enumerate()
        .zip(logits.iter())
        .map(|((index, document), &logit)| RerankedDocument {
            index,
            score: sigmoid_f32(logit),
            document: document.clone(),
        })
        .collect();

    // Sort descending by score.
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Truncate to top_k if requested.
    if let Some(k) = top_k {
        results.truncate(k);
    }

    results
}

/// Rerank a list of documents by relevance to a query.
///
/// Returns `RerankedDocument`s sorted descending by score. If `top_k` is set in
/// the config, only the top-k results are returned.
///
/// Returns an empty `Vec` for empty `documents` input (no error).
///
/// # Errors
///
/// - `XbergError::Validation` if `query` is blank or empty after trimming.
/// - `XbergError::MissingDependency` if ONNX Runtime is not installed (ONNX path).
/// - `XbergError::Reranking` if the preset name is unknown or model download fails.
///
/// Since v5.0.0.
#[cfg(feature = "reranker")]
pub fn rerank(
    query: String,
    documents: Vec<String>,
    config: &crate::core::config::RerankerConfig,
) -> crate::Result<Vec<RerankedDocument>> {
    if documents.is_empty() {
        return Ok(Vec::new());
    }

    if query.trim().is_empty() {
        return Err(crate::XbergError::Validation {
            message: "Reranker query must not be empty or blank".to_string(),
            source: None,
        });
    }

    // Dispatch by model type.
    match &config.model {
        #[cfg(all(feature = "liter-llm", not(target_arch = "wasm32")))]
        crate::core::config::RerankerModelType::Llm { llm } => {
            let top_k = config.top_k;
            let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
                // `block_in_place` requires a multi-thread runtime; calling it
                // on a current-thread runtime panics. Detect via the runtime
                // flavor and fall back to a nested current-thread runtime when
                // we're embedded in one.
                if matches!(handle.runtime_flavor(), tokio::runtime::RuntimeFlavor::CurrentThread) {
                    return Err(crate::XbergError::reranking(
                        "Synchronous rerank() with an LLM backend cannot be called from a current-thread Tokio runtime. \
                         Use rerank_async() or build a multi-thread runtime.",
                    ));
                }
                tokio::task::block_in_place(|| {
                    handle.block_on(crate::llm::rerank::rerank_via_llm(&query, &documents, llm, top_k))
                })
            } else {
                // No ambient runtime: drive the future on the shared, never-dropped
                // global runtime. Building a per-call runtime here would panic on
                // drop when this sync path runs inside a caller's blocking context.
                crate::core::runtime::global_runtime()?
                    .block_on(crate::llm::rerank::rerank_via_llm(&query, &documents, llm, top_k))
            };
            result.map(|(results, _usage)| results)
        }
        #[cfg(any(not(feature = "liter-llm"), target_arch = "wasm32"))]
        crate::core::config::RerankerModelType::Llm { .. } => Err(crate::XbergError::MissingDependency(
            "LLM reranking requires the 'liter-llm' feature. Rebuild with --features liter-llm".into(),
        )),
        crate::core::config::RerankerModelType::Plugin { name } => {
            let registry = crate::plugins::registry::get_reranker_backend_registry();
            let backend = {
                let guard = registry.read();
                guard.get(name)?
            };
            let expected_count = documents.len();
            let timeout = config
                .max_rerank_duration_secs
                .filter(|&s| s > 0)
                .map(std::time::Duration::from_secs);

            let rerank_future = async {
                match timeout {
                    Some(dur) => tokio::time::timeout(dur, backend.rerank(query.clone(), documents.clone()))
                        .await
                        .map_err(|_| crate::XbergError::Plugin {
                            message: format!("Reranker backend '{name}' did not complete within {dur:?}"),
                            plugin_name: name.clone(),
                        })?,
                    None => backend.rerank(query.clone(), documents.clone()).await,
                }
            };

            let logits = if let Ok(handle) = tokio::runtime::Handle::try_current() {
                if matches!(handle.runtime_flavor(), tokio::runtime::RuntimeFlavor::CurrentThread) {
                    return Err(crate::XbergError::reranking(
                        "Synchronous rerank() with a Plugin backend cannot be called from a current-thread Tokio runtime. \
                         Use rerank_async() or build a multi-thread runtime.",
                    ));
                }
                tokio::task::block_in_place(|| handle.block_on(rerank_future))
            } else {
                // No ambient runtime: drive the future on the shared, never-dropped
                // global runtime. Building a per-call runtime here would panic on
                // drop when this sync path runs inside a caller's blocking context.
                crate::core::runtime::global_runtime()?.block_on(rerank_future)
            }?;

            validate_reranker_output(&logits, expected_count, name)?;
            Ok(build_results(&documents, logits, config.top_k))
        }
        crate::core::config::RerankerModelType::Preset { .. }
        | crate::core::config::RerankerModelType::Custom { .. } => {
            let (repo, model_file, additional_files, max_length) = resolve_model_info(&config.model)?;
            let engine = get_or_init_engine(
                &repo,
                &model_file,
                &additional_files,
                max_length,
                config.cache_dir.clone(),
                config.acceleration.clone(),
            )?;

            let doc_refs: Vec<&str> = documents.iter().map(|d| d.as_str()).collect();
            let logits = engine
                .rerank(&query, &doc_refs, config.batch_size)
                .map_err(|e| crate::XbergError::reranking(format!("ONNX inference failed: {e}")))?;

            Ok(build_results(&documents, logits, config.top_k))
        }
    }
}

/// Rerank documents asynchronously.
///
/// Async counterpart to [`rerank`]. Offloads blocking ONNX inference to a
/// dedicated blocking thread pool via Tokio's `spawn_blocking`, keeping the
/// async executor free.
///
/// Since v5.0.0.
#[doc(alias = "rerank")]
#[cfg(all(feature = "reranker", feature = "tokio-runtime"))]
#[cfg_attr(alef, alef(skip))]
pub async fn rerank_async(
    query: String,
    documents: Vec<String>,
    config: &crate::core::config::RerankerConfig,
) -> crate::Result<Vec<RerankedDocument>> {
    if documents.is_empty() {
        return Ok(Vec::new());
    }

    if query.trim().is_empty() {
        return Err(crate::XbergError::Validation {
            message: "Reranker query must not be empty or blank".to_string(),
            source: None,
        });
    }

    match &config.model {
        #[cfg(all(feature = "liter-llm", not(target_arch = "wasm32")))]
        crate::core::config::RerankerModelType::Llm { llm } => {
            return crate::llm::rerank::rerank_via_llm(&query, &documents, llm, config.top_k)
                .await
                .map(|(results, _usage)| results);
        }
        #[cfg(any(not(feature = "liter-llm"), target_arch = "wasm32"))]
        crate::core::config::RerankerModelType::Llm { .. } => {
            return Err(crate::XbergError::MissingDependency(
                "LLM reranking requires the 'liter-llm' feature. Rebuild with --features liter-llm".into(),
            ));
        }
        crate::core::config::RerankerModelType::Plugin { name } => {
            let registry = crate::plugins::registry::get_reranker_backend_registry();
            let backend = {
                let guard = registry.read();
                guard.get(name)?
            };
            let expected_count = documents.len();
            let timeout = config
                .max_rerank_duration_secs
                .filter(|&s| s > 0)
                .map(std::time::Duration::from_secs);
            let logits = match timeout {
                Some(dur) => tokio::time::timeout(dur, backend.rerank(query, documents.clone()))
                    .await
                    .map_err(|_| crate::XbergError::Plugin {
                        message: format!("Reranker backend '{name}' did not complete within {dur:?}"),
                        plugin_name: name.clone(),
                    })??,
                None => backend.rerank(query, documents.clone()).await?,
            };
            validate_reranker_output(&logits, expected_count, name)?;
            return Ok(build_results(&documents, logits, config.top_k));
        }
        crate::core::config::RerankerModelType::Preset { .. }
        | crate::core::config::RerankerModelType::Custom { .. } => {
            // Fall through to ONNX path below.
        }
    }

    let _permit = RERANK_SEMAPHORE
        .acquire()
        .await
        .map_err(|_| crate::XbergError::reranking("Reranker semaphore closed".to_string()))?;

    let config = std::sync::Arc::new(config.clone());
    tokio::task::spawn_blocking(move || rerank(query, documents, &config))
        .await
        .map_err(|e| crate::XbergError::reranking(format!("Reranker task panicked: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_documents_returns_empty_vec() {
        let results = build_results(&[], vec![], None);
        assert!(results.is_empty());
    }

    #[test]
    fn build_results_sorts_descending_by_score() {
        let documents = vec!["doc0".to_string(), "doc1".to_string(), "doc2".to_string()];
        // Logits: -1.0, 2.0, 0.5 — sigmoid: ~0.27, ~0.88, ~0.62
        let logits = vec![-1.0_f32, 2.0_f32, 0.5_f32];
        let results = build_results(&documents, logits, None);

        assert_eq!(results.len(), 3);
        // First result should have highest score (doc at index 1 with logit=2.0)
        assert_eq!(results[0].index, 1);
        assert!(results[0].score > results[1].score, "Results must be sorted descending");
        assert!(results[1].score > results[2].score, "Results must be sorted descending");
    }

    #[test]
    fn top_k_truncation_applies() {
        let documents = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
        let logits = vec![1.0_f32, 2.0_f32, 0.5_f32, 1.5_f32];
        let results = build_results(&documents, logits, Some(2));

        assert_eq!(results.len(), 2, "top_k=2 should truncate to 2 results");
        // Scores should still be descending
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn top_k_zero_returns_empty() {
        let documents = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let logits = vec![1.0_f32, 2.0_f32, 0.5_f32];
        let results = build_results(&documents, logits, Some(0));
        assert!(results.is_empty(), "top_k=0 must return an empty vec");
    }

    #[test]
    fn top_k_larger_than_docs_returns_all() {
        let documents = vec!["a".to_string(), "b".to_string()];
        let logits = vec![1.0_f32, 0.5_f32];
        let results = build_results(&documents, logits, Some(100));
        assert_eq!(results.len(), 2, "top_k larger than docs.len() should return all docs");
    }

    #[test]
    fn build_results_preserves_document_text() {
        let documents = vec!["hello world".to_string(), "foo bar".to_string()];
        let logits = vec![0.0_f32, 1.0_f32];
        let results = build_results(&documents, logits, None);

        // Results sorted: index 1 first (higher logit), then index 0
        assert_eq!(results[0].document, "foo bar");
        assert_eq!(results[1].document, "hello world");
    }

    #[test]
    fn reranked_document_serde_roundtrip() {
        let doc = RerankedDocument {
            index: 3,
            score: 0.87,
            document: "test document".to_string(),
        };
        let json = serde_json::to_string(&doc).unwrap();
        let back: RerankedDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(back.index, doc.index);
        assert!((back.score - doc.score).abs() < 1e-6);
        assert_eq!(back.document, doc.document);
    }

    #[cfg(feature = "reranker-presets")]
    #[test]
    fn preset_list_exposes_catalog_plus_aliases() {
        let presets = list_presets();
        // 4 catalog + 4 friendly aliases.
        assert_eq!(presets.len(), RERANKER_PRESETS.len() + PRESET_ALIASES.len());
        // Catalog names present.
        assert!(presets.iter().any(|n| n == "bge-reranker-base"));
        assert!(presets.iter().any(|n| n == "bge-reranker-v2-m3"));
        assert!(presets.iter().any(|n| n == "jina-reranker-v1-turbo-en"));
        assert!(presets.iter().any(|n| n == "jina-reranker-v2-base-multilingual"));
        // Friendly aliases present.
        for (alias, _) in PRESET_ALIASES {
            assert!(presets.iter().any(|n| n == *alias), "missing alias: {alias}");
        }
    }

    #[cfg(feature = "reranker-presets")]
    #[test]
    fn get_preset_case_sensitive() {
        assert!(get_preset("bge-reranker-base").is_some());
        assert!(
            get_preset("BGE-Reranker-Base").is_none(),
            "Preset lookup must be case-sensitive"
        );
        assert!(get_preset("nonexistent").is_none());
    }

    #[cfg(feature = "reranker-presets")]
    #[test]
    fn aliases_resolve_to_catalog_entries() {
        for (alias, catalog_name) in PRESET_ALIASES {
            let preset = get_preset(alias).expect("alias must resolve");
            assert_eq!(
                preset.name, *catalog_name,
                "alias {alias} should resolve to catalog entry {catalog_name}"
            );
        }
    }

    #[cfg(feature = "reranker-presets")]
    #[test]
    fn catalog_paths_match_fastembed_rs() {
        // Source of truth: fastembed-rs `src/models/reranking.rs`. Lock these to
        // catch accidental drift; if fastembed-rs updates we mirror it here.
        let by_name = |n: &str| get_preset(n).expect(n);

        let base = by_name("bge-reranker-base");
        assert_eq!(base.model_repo, "BAAI/bge-reranker-base");
        assert_eq!(base.model_file, "onnx/model.onnx");
        assert!(base.additional_files.is_empty());

        let m3 = by_name("bge-reranker-v2-m3");
        assert_eq!(m3.model_repo, "rozgo/bge-reranker-v2-m3");
        assert_eq!(m3.model_file, "model.onnx");
        assert_eq!(m3.additional_files, vec!["model.onnx.data".to_string()]);

        let turbo = by_name("jina-reranker-v1-turbo-en");
        assert_eq!(turbo.model_repo, "jinaai/jina-reranker-v1-turbo-en");
        assert_eq!(turbo.model_file, "onnx/model.onnx");

        let multi = by_name("jina-reranker-v2-base-multilingual");
        assert_eq!(multi.model_repo, "jinaai/jina-reranker-v2-base-multilingual");
        assert_eq!(multi.model_file, "onnx/model.onnx");
    }

    #[cfg(all(feature = "reranker", feature = "tokio-runtime"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn plugin_backend_rerank_roundtrip() {
        use crate::core::config::RerankerConfig;
        use crate::plugins::{Plugin, RerankerBackend, register_reranker_backend, unregister_reranker_backend};
        use async_trait::async_trait;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU64, Ordering};

        struct MockPlugin {
            name: String,
        }

        impl Plugin for MockPlugin {
            fn name(&self) -> &str {
                &self.name
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl RerankerBackend for MockPlugin {
            async fn rerank(&self, _query: String, documents: Vec<String>) -> crate::Result<Vec<f32>> {
                // Return descending logits so doc 0 wins.
                Ok(documents
                    .iter()
                    .enumerate()
                    .map(|(i, _)| (documents.len() - i) as f32)
                    .collect())
            }
        }

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let name = format!("test-mock-reranker-{id}");

        register_reranker_backend(Arc::new(MockPlugin { name: name.clone() })).unwrap();

        let config = RerankerConfig {
            model: crate::core::config::RerankerModelType::Plugin { name: name.clone() },
            top_k: Some(2),
            ..Default::default()
        };

        let results = rerank_async(
            "test query".to_string(),
            vec!["doc0".to_string(), "doc1".to_string(), "doc2".to_string()],
            &config,
        )
        .await
        .unwrap();

        assert_eq!(results.len(), 2, "top_k=2 should limit to 2 results");
        // Scores must be descending.
        assert!(results[0].score >= results[1].score);

        unregister_reranker_backend(&name).unwrap();
    }
}
