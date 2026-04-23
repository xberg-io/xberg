//! Embedding generation support for RAG (Retrieval-Augmented Generation) systems.
//!
//! This module provides text embedding generation using ONNX models via a vendored
//! text embedding inference engine. Embeddings can be generated for text chunks to
//! enable semantic search and RAG pipelines.
//!
//! # Features
//!
//! - Multiple pre-configured models optimized for different use cases
//! - Preset configurations for common RAG scenarios
//! - Full customization of model location and parameters
//! - Batch processing for efficient embedding generation
//! - Thread-safe inference without mutex contention
//! - Optional GPU acceleration via ONNX Runtime execution providers
//!
//! # ONNX Runtime Requirement
//!
//! **CRITICAL**: This module requires ONNX Runtime to be installed on the system.
//! The `embeddings` feature uses dynamic loading (`ort-load-dynamic`), which detects
//! the ONNX Runtime library at runtime.
//!
//! ## Installation Instructions
//!
//! - **macOS**: `brew install onnxruntime`
//! - **Linux (Ubuntu/Debian)**: `apt install libonnxruntime libonnxruntime-dev`
//! - **Linux (Fedora)**: `dnf install onnxruntime onnxruntime-devel`
//! - **Linux (Arch)**: `pacman -S onnxruntime`
//! - **Windows (MSVC)**: Download from https://github.com/microsoft/onnxruntime/releases and add to PATH
//!
//! Alternatively, set the `ORT_DYLIB_PATH` environment variable to the ONNX Runtime library path.
//!
//! For Docker/containers, install via package manager in your base image.
//! Verified packages: Ubuntu 22.04+, Fedora 38+, Arch Linux.
//!
//! ## Platform Limitations
//!
//! **Windows MinGW builds are not supported**. ONNX Runtime requires the MSVC toolchain on Windows.
//! Please use Windows MSVC builds or disable the embeddings feature.
//!
//! # Example
//!
//! ```rust,ignore
//! use kreuzberg::{extract_file, ExtractionConfig, ChunkingConfig, EmbeddingConfig};
//!
//! let config = ExtractionConfig {
//!     chunking: Some(ChunkingConfig {
//!         preset: Some("balanced".to_string()),
//!         embedding: Some(EmbeddingConfig::default()),
//!         ..Default::default()
//!     }),
//!     ..Default::default()
//! };
//!
//! let result = extract_file("document.pdf", None, &config).await?;
//! for chunk in result.chunks.unwrap() {
//!     if let Some(embedding) = chunk.embedding {
//!         println!("Chunk has {} dimension embedding", embedding.len());
//!     }
//! }
//! ```

pub mod engine;

use ahash::AHashMap;
use std::sync::{Arc, RwLock};

use std::sync::LazyLock;

use engine::EmbeddingEngine;

type CachedEngine = Arc<EmbeddingEngine>;

static ENGINE_CACHE: LazyLock<RwLock<AHashMap<String, CachedEngine>>> = LazyLock::new(|| RwLock::new(AHashMap::new()));

/// Global semaphore that limits concurrent ONNX embedding inference calls.
///
/// Prevents resource exhaustion when many async callers invoke `embed_texts_async`
/// against the ONNX path (Preset/Custom variants) simultaneously. The Llm and
/// Plugin variants short-circuit out of `embed_texts_async` before reaching the
/// semaphore — they don't share the local-inference resource pool. The permit
/// count is set once on first access using the thread budget, matching the pattern
/// used elsewhere (e.g., image OCR, batch extraction).
#[cfg(feature = "tokio-runtime")]
static EMBED_SEMAPHORE: LazyLock<Arc<tokio::sync::Semaphore>> = LazyLock::new(|| {
    let budget = crate::core::config::concurrency::resolve_thread_budget(None);
    Arc::new(tokio::sync::Semaphore::new(budget))
});

/// Preset configurations for common RAG use cases.
///
/// Each preset combines chunk size, overlap, and embedding model
/// to provide an optimized configuration for specific scenarios.
#[derive(Debug, Clone)]
pub struct EmbeddingPreset {
    pub name: &'static str,
    pub chunk_size: usize,
    pub overlap: usize,
    /// HuggingFace repository name for the model.
    pub model_repo: &'static str,
    /// Pooling strategy: "cls" or "mean".
    pub pooling: &'static str,
    /// Path to the ONNX model file within the repo.
    pub model_file: &'static str,
    pub dimensions: usize,
    pub description: &'static str,
}

/// All available embedding presets.
pub const EMBEDDING_PRESETS: &[EmbeddingPreset] = &[
    EmbeddingPreset {
        name: "fast",
        chunk_size: 512,
        overlap: 50,
        model_repo: "Xenova/all-MiniLM-L6-v2",
        pooling: "mean",
        model_file: "onnx/model_quantized.onnx",
        dimensions: 384,
        description: "Fast embedding with quantized model (384 dims, ~22M params). Best for: Quick prototyping, development, resource-constrained environments.",
    },
    EmbeddingPreset {
        name: "balanced",
        chunk_size: 1024,
        overlap: 100,
        model_repo: "Xenova/bge-base-en-v1.5",
        pooling: "cls",
        model_file: "onnx/model.onnx",
        dimensions: 768,
        description: "Balanced quality and speed (768 dims, ~109M params). Best for: General-purpose RAG, production deployments, English documents.",
    },
    EmbeddingPreset {
        name: "quality",
        chunk_size: 2000,
        overlap: 200,
        model_repo: "Xenova/bge-large-en-v1.5",
        pooling: "cls",
        model_file: "onnx/model.onnx",
        dimensions: 1024,
        description: "High quality with larger context (1024 dims, ~335M params). Best for: Complex documents, maximum accuracy, sufficient compute resources.",
    },
    EmbeddingPreset {
        name: "multilingual",
        chunk_size: 1024,
        overlap: 100,
        model_repo: "intfloat/multilingual-e5-base",
        pooling: "mean",
        model_file: "onnx/model.onnx",
        dimensions: 768,
        description: "Multilingual support (768 dims, 100+ languages). Best for: International documents, mixed-language content, global applications.",
    },
];

/// Get a preset by name.
pub fn get_preset(name: &str) -> Option<&'static EmbeddingPreset> {
    EMBEDDING_PRESETS.iter().find(|p| p.name == name)
}

/// Get the chunk_size for a preset by name.
pub(crate) fn preset_chunk_size(name: &str) -> Option<usize> {
    get_preset(name).map(|p| p.chunk_size)
}

/// List all available preset names.
pub fn list_presets() -> Vec<&'static str> {
    EMBEDDING_PRESETS.iter().map(|p| p.name).collect()
}

/// Returns installation instructions for ONNX Runtime.
fn onnx_runtime_install_message() -> String {
    #[cfg(all(windows, target_env = "gnu"))]
    {
        return "ONNX Runtime embeddings are not supported on Windows MinGW builds. \
        ONNX Runtime requires MSVC toolchain. \
        Please use Windows MSVC builds or disable embeddings feature."
            .to_string();
    }

    #[cfg(not(all(windows, target_env = "gnu")))]
    {
        "ONNX Runtime is required for embeddings functionality. \
        Install: \
        macOS: 'brew install onnxruntime', \
        Linux (Ubuntu/Debian): 'apt install libonnxruntime libonnxruntime-dev', \
        Linux (Fedora): 'dnf install onnxruntime onnxruntime-devel', \
        Linux (Arch): 'pacman -S onnxruntime', \
        Windows (MSVC): Download from https://github.com/microsoft/onnxruntime/releases and add to PATH. \
        \
        Alternatively, set ORT_DYLIB_PATH environment variable to the ONNX Runtime library path. \
        \
        For Docker/containers: Install via package manager in your base image. \
        Verified packages: Ubuntu 22.04+, Fedora 38+, Arch Linux."
            .to_string()
    }
}

/// Resolve the cache directory for embedding models.
fn resolve_cache_dir(cache_dir: Option<std::path::PathBuf>) -> std::path::PathBuf {
    cache_dir.unwrap_or_else(|| crate::cache_dir::resolve_cache_dir("embeddings"))
}

/// Resolve model info (repo, model file, pooling) from an EmbeddingModelType config.
fn resolve_model_info(
    model_type: &crate::core::config::EmbeddingModelType,
) -> crate::Result<(&str, &str, engine::Pooling)> {
    match model_type {
        crate::core::config::EmbeddingModelType::Preset { name } => {
            let preset = get_preset(name)
                .ok_or_else(|| crate::KreuzbergError::embedding(format!("Unknown embedding preset: {name}")))?;
            let pooling = match preset.pooling {
                "cls" => engine::Pooling::Cls,
                _ => engine::Pooling::Mean,
            };
            Ok((preset.model_repo, preset.model_file, pooling))
        }
        crate::core::config::EmbeddingModelType::Custom { model_id, .. } => {
            // For custom models, default to mean pooling and standard model path.
            // Users providing custom HF models should ensure the repo has the expected layout.
            Ok((model_id.as_str(), "onnx/model.onnx", engine::Pooling::Mean))
        }
        crate::core::config::EmbeddingModelType::Llm { .. } => Err(crate::KreuzbergError::embedding(
            "LLM embeddings have no local model to warm or download — the provider serves them over HTTP at embed time.",
        )),
        crate::core::config::EmbeddingModelType::Plugin { .. } => Err(crate::KreuzbergError::embedding(
            "Plugin embeddings have no local model to warm or download — the registered backend owns the model lifecycle.",
        )),
    }
}

/// Load a tokenizer from HuggingFace model files.
///
/// Adapted from the vendored embedding engine's tokenizer initialization.
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
            .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to read config.json: {e}")))?,
    )
    .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to parse config.json: {e}")))?;

    let tokenizer_config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(tokenizer_config_path)
            .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to read tokenizer_config.json: {e}")))?,
    )
    .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to parse tokenizer_config.json: {e}")))?;

    let mut tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
        .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to load tokenizer: {e}")))?;

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
        .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to configure tokenizer: {e}")))?;

    // Add special tokens from special_tokens_map.json
    if let Ok(special_tokens_data) = std::fs::read(special_tokens_path)
        && let Ok(serde_json::Value::Object(map)) = serde_json::from_slice(&special_tokens_data)
    {
        for (_, value) in &map {
            if let Some(content) = value.as_str() {
                tokenizer.add_special_tokens(&[AddedToken {
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
                tokenizer.add_special_tokens(&[AddedToken {
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

/// How long a partial download must be idle before it is considered stale.
///
/// hf-hub writes to the `.part` file continuously during an active download.
/// If the file has not been modified in this window, no live process is writing
/// to it and the corresponding lock is safe to remove.
const STALE_DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Remove stale `.lock` and `.part` files left behind by interrupted downloads.
///
/// hf-hub coordinates concurrent downloads with `flock(LOCK_EX)`. The OS
/// releases the flock when the owning process exits, but the `.lock` and
/// `.part` files remain on disk. In practice this causes permanent
/// `LockAcquisition` failures in two scenarios:
///
/// - A CI job or Docker container is killed mid-download; the next invocation
///   cannot acquire the lock because the file still exists (even though no
///   process holds it — the flock was released).
/// - Two concurrent first-time invocations race; the loser exits with an
///   error and the `.lock` / `.part` files are never cleaned up if the winner
///   also fails later.
///
/// Staleness is detected via the modification time of the `.part` file (or
/// the `.lock` file when no `.part` exists): if neither has been written in
/// [`STALE_DOWNLOAD_TIMEOUT`], no live process is actively downloading and
/// it is safe to remove both files so that the next `repo.get()` can proceed.
fn cleanup_stale_locks(cache_dir: &std::path::Path, repo_name: &str) {
    // hf-hub folder_name(): "models--" + repo_id.replace('/', "--")
    let folder = format!("models--{}", repo_name.replace('/', "--"));
    let blobs_dir = cache_dir.join(folder).join("blobs");

    let entries = match std::fs::read_dir(&blobs_dir) {
        Ok(e) => e,
        Err(_) => return, // blobs dir doesn't exist yet — nothing to clean
    };

    let now = std::time::SystemTime::now();

    for entry in entries.flatten() {
        let lock_path = entry.path();
        if lock_path.extension().is_some_and(|ext| ext == "lock") {
            let part_path = lock_path.with_extension("part");

            // Prefer the .part file's mtime: an active download writes bytes
            // continuously, so a stale mtime there is the strongest signal.
            // Fall back to the .lock file's mtime when no .part exists.
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

/// Download model files from HuggingFace and return their local paths.
///
/// Returns `(model_path, tokenizer_path, config_path, special_tokens_path, tokenizer_config_path)`.
///
/// Before downloading, stale lock/part files left by interrupted or concurrent
/// invocations are removed automatically so that the download can proceed
/// without requiring manual intervention.
fn download_model_files(
    repo_name: &str,
    model_file: &str,
    cache_directory: &std::path::Path,
) -> crate::Result<(
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
)> {
    // Self-heal any stale .lock/.part files from a previous interrupted download
    // before hf-hub's own lock_file() runs and fails on them.
    cleanup_stale_locks(cache_directory, repo_name);

    let api = hf_hub::api::sync::ApiBuilder::from_env()
        .with_cache_dir(cache_directory.to_path_buf())
        .with_progress(true)
        .build()
        .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to create HF API client: {e}")))?;

    let repo = api.model(repo_name.to_string());

    let model_path = repo.get(model_file).map_err(|e| {
        let hint = if matches!(e, hf_hub::api::sync::ApiError::LockAcquisition(_)) {
            lock_acquisition_hint(cache_directory, repo_name)
        } else {
            String::new()
        };
        crate::KreuzbergError::embedding(format!("Failed to download {model_file}: {e}{hint}"))
    })?;

    let tokenizer_path = repo
        .get("tokenizer.json")
        .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to download tokenizer.json: {e}")))?;

    let config_path = repo
        .get("config.json")
        .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to download config.json: {e}")))?;

    // These are optional — fall back to empty paths that load_tokenizer handles gracefully
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

/// Get or initialize an embedding engine from cache.
///
/// Downloads model files from HuggingFace if needed, loads the tokenizer,
/// creates an ORT session, and caches the engine for reuse.
fn get_or_init_engine(
    repo_name: &str,
    model_file: &str,
    pooling: engine::Pooling,
    cache_dir: Option<std::path::PathBuf>,
    accel: Option<crate::core::config::acceleration::AccelerationConfig>,
) -> crate::Result<Arc<EmbeddingEngine>> {
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

        // Download model files
        let (model_path, tokenizer_path, config_path, special_tokens_path, tokenizer_config_path) =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                download_model_files(repo_name, model_file, &cache_directory)
            }))
            .map_err(|panic_payload| {
                let panic_msg = panic_to_string(panic_payload);
                if looks_like_ort_error(&panic_msg) {
                    crate::KreuzbergError::MissingDependency(format!(
                        "ONNX Runtime - {}",
                        onnx_runtime_install_message()
                    ))
                } else {
                    crate::KreuzbergError::embedding(format!("Model download panicked: {panic_msg}"))
                }
            })??;

        // Load tokenizer
        let tokenizer = load_tokenizer(
            &tokenizer_path,
            &config_path,
            &special_tokens_path,
            &tokenizer_config_path,
            512, // default max_length
        )?;

        // Create ORT session
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
                crate::KreuzbergError::MissingDependency(format!("ONNX Runtime - {}", onnx_runtime_install_message()))
            } else {
                crate::KreuzbergError::embedding(format!("ONNX Runtime initialization panicked: {panic_msg}"))
            }
        })?
        .map_err(|e| {
            let error_msg = e.to_string();
            if looks_like_ort_error(&error_msg) {
                crate::KreuzbergError::MissingDependency(format!("ONNX Runtime - {}", onnx_runtime_install_message()))
            } else {
                crate::KreuzbergError::embedding(format!("Failed to create ONNX session: {e}"))
            }
        })?;

        let new_engine = Arc::new(EmbeddingEngine::new(tokenizer, session, pooling));
        cache.insert(engine_key, Arc::clone(&new_engine));

        Ok(new_engine)
    }
}

/// Check if an error message looks like an ONNX Runtime missing dependency.
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
fn panic_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

/// Eagerly download and cache an embedding model without returning the handle.
///
/// This triggers the same download and initialization as `get_or_init_engine`
/// but discards the result, making it suitable for cache-warming scenarios
/// where the caller doesn't need to use the model immediately.
///
/// **Note**: This function downloads AND initializes the ONNX model, which
/// requires ONNX Runtime and uses significant memory. For download-only
/// scenarios (e.g., init containers), use [`download_model`] instead.
pub fn warm_model(
    model_type: &crate::core::config::EmbeddingModelType,
    cache_dir: Option<std::path::PathBuf>,
) -> crate::Result<()> {
    let (repo, model_file, pooling) = resolve_model_info(model_type)?;
    get_or_init_engine(repo, model_file, pooling, cache_dir, None).map(|_| ())
}

/// Download an embedding model's files without initializing ONNX Runtime.
///
/// Downloads the model files (ONNX model, tokenizer, config) from HuggingFace
/// to the cache directory. Subsequent calls to `warm_model` or
/// `get_or_init_engine` will find the files cached and skip the download step.
///
/// This is ideal for init containers or CI environments where you want to
/// pre-populate the cache without loading models into memory.
pub fn download_model(
    model_type: &crate::core::config::EmbeddingModelType,
    cache_dir: Option<std::path::PathBuf>,
) -> crate::Result<()> {
    let (repo_name, model_file, _pooling) = resolve_model_info(model_type)?;
    let cache_directory = resolve_cache_dir(cache_dir);

    let files = &[
        model_file,
        "tokenizer.json",
        "config.json",
        "special_tokens_map.json",
        "tokenizer_config.json",
    ];

    tracing::info!(repo = %repo_name, "Downloading embedding model files (no ONNX init)");

    let api = hf_hub::api::sync::ApiBuilder::from_env()
        .with_cache_dir(cache_directory)
        .with_progress(true)
        .build()
        .map_err(|e| crate::KreuzbergError::embedding(format!("Failed to create HF API client: {e}")))?;

    let repo = api.model(repo_name.to_string());

    for file in files {
        match repo.get(file) {
            Ok(path) => tracing::debug!(file = %file, path = ?path, "Downloaded"),
            Err(e) => {
                // Model and tokenizer are required; others are optional
                if *file == model_file || *file == "tokenizer.json" {
                    return Err(crate::KreuzbergError::embedding(format!(
                        "Failed to download {file}: {e}"
                    )));
                }
                tracing::debug!(file = %file, error = %e, "Optional file not found, skipping");
            }
        }
    }

    tracing::info!(repo = %repo_name, "Embedding model files downloaded successfully");
    Ok(())
}

/// Normalize an embedding vector in-place (L2 normalization).
fn normalize_in_place(embedding: &mut [f32]) {
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > f32::EPSILON {
        let inv_mag = 1.0 / magnitude;
        embedding.iter_mut().for_each(|x| *x *= inv_mag);
    }
}

/// Validate that a backend-produced batch of embeddings matches the expected
/// shape (batch size and per-vector dimension).
///
/// The dispatcher calls this on every `Plugin`-variant response before returning
/// to downstream consumers. A non-conforming backend surfaces as a
/// [`crate::KreuzbergError::Validation`] here rather than a panic in semantic
/// chunking, `chunk.embedding` assignment, or user code.
///
/// # Errors
///
/// - [`crate::KreuzbergError::Validation`] if `embeddings.len() != expected_count`.
/// - [`crate::KreuzbergError::Validation`] if any `embeddings[i].len() != expected_dim`.
fn validate_embedding_shape(
    embeddings: &[Vec<f32>],
    expected_count: usize,
    expected_dim: usize,
    backend_name: &str,
) -> crate::Result<()> {
    if embeddings.len() != expected_count {
        return Err(crate::KreuzbergError::Validation {
            message: format!(
                "Embedding backend '{backend_name}' returned {got} vectors for {expected} inputs",
                got = embeddings.len(),
                expected = expected_count,
            ),
            source: None,
        });
    }

    for (i, vec) in embeddings.iter().enumerate() {
        if vec.len() != expected_dim {
            return Err(crate::KreuzbergError::Validation {
                message: format!(
                    "Embedding backend '{backend_name}' returned vector at index {i} with length {got}, expected {expected_dim}",
                    got = vec.len(),
                ),
                source: None,
            });
        }
    }

    Ok(())
}

/// Apply normalization to a batch of embeddings (parallel for large batches).
fn normalize_embeddings(embeddings: &mut [Vec<f32>]) {
    const PARALLEL_THRESHOLD: usize = 64;
    if embeddings.len() >= PARALLEL_THRESHOLD {
        use rayon::prelude::*;
        embeddings.par_iter_mut().for_each(|v| normalize_in_place(v));
    } else {
        embeddings.iter_mut().for_each(|v| normalize_in_place(v));
    }
}

/// Generate embeddings for text chunks using the specified configuration.
///
/// This function modifies chunks in-place, populating their `embedding` field
/// with generated embedding vectors. It uses batch processing for efficiency.
///
/// # Arguments
///
/// * `chunks` - Mutable reference to vector of chunks to generate embeddings for
/// * `config` - Embedding configuration specifying model and parameters
///
/// # Returns
///
/// Returns `Ok(())` if embeddings were generated successfully, or an error if
/// model initialization or embedding generation fails.
pub(crate) fn generate_embeddings_for_chunks(
    chunks: &mut [crate::types::Chunk],
    config: &crate::core::config::EmbeddingConfig,
) -> crate::Result<()> {
    if chunks.is_empty() {
        return Ok(());
    }

    let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
    let embeddings_result = embed_texts(&texts, config)?;

    // Assign embeddings to chunks.
    for (chunk, embedding) in chunks.iter_mut().zip(embeddings_result) {
        chunk.embedding = Some(embedding);
    }

    Ok(())
}

/// Generate embeddings for a list of raw text strings (standalone, no chunking pipeline).
///
/// Returns one embedding vector per input text, in the same order as the input.
/// Uses the same model resolution, engine caching, and batch processing as the
/// chunking pipeline. Normalization is applied if `config.normalize` is true.
///
/// # Arguments
///
/// * `texts` - Slice of strings to embed
/// * `config` - Embedding configuration specifying model, batch size, and normalization
///
/// # Returns
///
/// Returns `Vec<Vec<f32>>` — one `Vec<f32>` per input text. Returns an empty
/// `Vec` if `texts` is empty (no error).
///
/// # Errors
///
/// - `KreuzbergError::MissingDependency` if ONNX Runtime is not installed
/// - `KreuzbergError::Embedding` if the preset name is unknown or model download fails
///
/// # Example
///
/// ```rust,ignore
/// use kreuzberg::{embed_texts, EmbeddingConfig, EmbeddingModelType};
///
/// let config = EmbeddingConfig {
///     model: EmbeddingModelType::Preset { name: "balanced".to_string() },
///     normalize: true,
///     ..Default::default()
/// };
/// let embeddings = embed_texts(&["Hello, world!", "Second text"], &config)?;
/// assert_eq!(embeddings.len(), 2);
/// assert_eq!(embeddings[0].len(), 768); // balanced preset = 768 dims
/// ```
pub fn embed_texts<T: AsRef<str>>(
    texts: &[T],
    config: &crate::core::config::EmbeddingConfig,
) -> crate::Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    // Validate that no individual text is empty — empty strings produce
    // meaningless embeddings and can cause tokenizer edge-cases.
    for (i, t) in texts.iter().enumerate() {
        if t.as_ref().is_empty() {
            return Err(crate::KreuzbergError::embedding(format!(
                "Text at position {pos} is empty. All texts must be non-empty.",
                pos = i + 1
            )));
        }
    }

    // Dispatch: LLM-hosted embeddings bypass the local ONNX engine entirely.
    match &config.model {
        #[cfg(feature = "liter-llm")]
        crate::core::config::EmbeddingModelType::Llm { llm } => {
            let normalize = config.normalize;
            // If we're already inside an async runtime (e.g. server mode),
            // use block_in_place to avoid the "cannot block inside runtime" panic.
            // Otherwise, create a dedicated single-threaded runtime for the sync path.
            let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
                tokio::task::block_in_place(|| {
                    handle.block_on(crate::llm::vlm_embeddings::embed_via_llm(texts, llm, normalize))
                })
            } else {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        crate::KreuzbergError::embedding(format!("Failed to create runtime for LLM embeddings: {e}"))
                    })?;
                rt.block_on(crate::llm::vlm_embeddings::embed_via_llm(texts, llm, normalize))
            };
            result.map(|(embeddings, _usage)| embeddings)
        }
        #[cfg(not(feature = "liter-llm"))]
        crate::core::config::EmbeddingModelType::Llm { .. } => Err(crate::KreuzbergError::MissingDependency(
            "LLM embeddings require the 'liter-llm' feature. Rebuild with --features liter-llm".into(),
        )),
        crate::core::config::EmbeddingModelType::Plugin { name } => {
            let registry = crate::plugins::get_embedding_backend_registry();
            // Clone the Arc out of the lock along with the dimensions captured
            // at registration (the trait contract requires stability, but we
            // don't re-ask the backend on every dispatch — that would let a
            // buggy backend drift past shape validation silently).
            let (backend, expected_dim) = {
                let guard = registry.read();
                guard.get_with_dimensions(name)?
            };
            let expected_count = texts.len();
            let owned_texts: Vec<String> = texts.iter().map(|t| t.as_ref().to_string()).collect();

            // Dispatch the async `embed` call. Same pattern as the `Llm` arm:
            // if we're already in a tokio runtime, `block_in_place` to avoid
            // starving workers; otherwise spin up a single-threaded runtime.
            // `Some(0)` is treated as "no timeout" rather than "timeout immediately" — a
            // zero-duration `tokio::time::timeout` fires before the backend gets a chance
            // to run, which is a surprising config interpretation users rarely intend.
            let timeout = config
                .max_embed_duration_secs
                .filter(|&s| s > 0)
                .map(std::time::Duration::from_secs);
            let embed_future = async {
                match timeout {
                    Some(dur) => tokio::time::timeout(dur, backend.embed(owned_texts))
                        .await
                        .map_err(|_| crate::KreuzbergError::Plugin {
                            message: format!("Embedding backend '{name}' did not complete within {dur:?}"),
                            plugin_name: name.clone(),
                        })?,
                    None => backend.embed(owned_texts).await,
                }
            };
            let embed_result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
                tokio::task::block_in_place(|| handle.block_on(embed_future))
            } else {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        crate::KreuzbergError::embedding(format!("Failed to create runtime for plugin embeddings: {e}"))
                    })?;
                rt.block_on(embed_future)
            };
            let mut embeddings = embed_result?;

            validate_embedding_shape(&embeddings, expected_count, expected_dim, name)?;

            if config.normalize {
                normalize_embeddings(&mut embeddings);
            }

            Ok(embeddings)
        }
        crate::core::config::EmbeddingModelType::Preset { .. }
        | crate::core::config::EmbeddingModelType::Custom { .. } => {
            // Local ONNX path for Preset and Custom model types.
            let chunk_count = texts.len();
            let (repo, model_file, pooling) = resolve_model_info(&config.model)?;
            let engine = get_or_init_engine(
                repo,
                model_file,
                pooling,
                config.cache_dir.clone(),
                config.acceleration.clone(),
            )?;

            let text_refs: Vec<&str> = texts.iter().map(|t| t.as_ref()).collect();
            let mut embeddings = engine.embed(&text_refs, config.batch_size).map_err(|e| {
                crate::KreuzbergError::embedding(format!(
                    "Failed to generate embeddings for {chunk_count} texts (model={:?}, batch_size={}): {e}",
                    config.model, config.batch_size
                ))
            })?;

            if config.normalize {
                normalize_embeddings(&mut embeddings);
            }

            Ok(embeddings)
        }
    }
}

/// Generate embeddings asynchronously for a list of text strings.
///
/// This is the async counterpart to [`embed_texts`]. It offloads the blocking
/// ONNX inference work to a dedicated blocking thread pool via Tokio's
/// `spawn_blocking`, keeping the async executor free.
///
/// Returns one embedding vector per input text in the same order.
///
/// # Arguments
///
/// * `texts` - Vec of strings to embed (owned, sent to blocking thread)
/// * `config` - Embedding configuration specifying model, batch size, and normalization
///
/// # Errors
///
/// - `KreuzbergError::MissingDependency` if ONNX Runtime is not installed
/// - `KreuzbergError::Embedding` if the preset name is unknown, model download fails,
///   or the blocking inference task panics
///
/// # Example
///
/// ```rust,ignore
/// use kreuzberg::{embed_texts_async, EmbeddingConfig};
///
/// let embeddings = embed_texts_async(
///     vec!["Hello!".to_string()],
///     &EmbeddingConfig::default(),
/// ).await?;
/// ```
#[cfg(feature = "tokio-runtime")]
pub async fn embed_texts_async<T: AsRef<str> + Send + 'static>(
    texts: Vec<T>,
    config: &crate::core::config::EmbeddingConfig,
) -> crate::Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    // Reject empty-string inputs here so every dispatch path (Llm fast path,
    // Plugin fast path, ONNX via spawn_blocking) rejects them the same way.
    // The sync `embed_texts` has this check; the async fast paths used to skip
    // it.
    for (i, t) in texts.iter().enumerate() {
        if t.as_ref().is_empty() {
            return Err(crate::KreuzbergError::embedding(format!(
                "Text at position {pos} is empty. All texts must be non-empty.",
                pos = i + 1
            )));
        }
    }

    // Dispatch is exhaustive over EmbeddingModelType so a future variant must add an arm here.
    // Llm is cfg-gated; Plugin awaits the host-language backend directly (no spawn_blocking
    // round-trip since the trait is async); Preset/Custom fall through to the local ONNX path.
    match &config.model {
        #[cfg(feature = "liter-llm")]
        crate::core::config::EmbeddingModelType::Llm { llm } => {
            return crate::llm::vlm_embeddings::embed_via_llm(&texts, llm, config.normalize)
                .await
                .map(|(embeddings, _usage)| embeddings);
        }
        #[cfg(not(feature = "liter-llm"))]
        crate::core::config::EmbeddingModelType::Llm { .. } => {
            return Err(crate::KreuzbergError::MissingDependency(
                "LLM embeddings require the 'liter-llm' feature. Rebuild with --features liter-llm".into(),
            ));
        }
        crate::core::config::EmbeddingModelType::Plugin { name } => {
            let registry = crate::plugins::get_embedding_backend_registry();
            let (backend, expected_dim) = {
                let guard = registry.read();
                guard.get_with_dimensions(name)?
            };
            let expected_count = texts.len();
            let owned_texts: Vec<String> = texts.iter().map(|t| t.as_ref().to_string()).collect();
            // `Some(0)` is treated as "no timeout" rather than "timeout immediately" — a
            // zero-duration `tokio::time::timeout` fires before the backend gets a chance
            // to run, which is a surprising config interpretation users rarely intend.
            let timeout = config
                .max_embed_duration_secs
                .filter(|&s| s > 0)
                .map(std::time::Duration::from_secs);
            let mut embeddings = match timeout {
                Some(dur) => tokio::time::timeout(dur, backend.embed(owned_texts))
                    .await
                    .map_err(|_| crate::KreuzbergError::Plugin {
                        message: format!("Embedding backend '{name}' did not complete within {dur:?}"),
                        plugin_name: name.clone(),
                    })??,
                None => backend.embed(owned_texts).await?,
            };
            validate_embedding_shape(&embeddings, expected_count, expected_dim, name)?;
            if config.normalize {
                normalize_embeddings(&mut embeddings);
            }
            return Ok(embeddings);
        }
        crate::core::config::EmbeddingModelType::Preset { .. }
        | crate::core::config::EmbeddingModelType::Custom { .. } => {
            // Fall through to ONNX path below.
        }
    }

    // Acquire a permit from the global semaphore to limit concurrent ONNX
    // inference calls, preventing resource exhaustion under high fan-out.
    let _permit = EMBED_SEMAPHORE
        .acquire()
        .await
        .map_err(|_| crate::KreuzbergError::embedding("Embedding semaphore closed".to_string()))?;

    // Wrap config in Arc to avoid cloning the entire struct (strings, PathBuf)
    // into the blocking closure.
    let config = Arc::new(config.clone());
    tokio::task::spawn_blocking(move || embed_texts(&texts, &config))
        .await
        .map_err(|e| crate::KreuzbergError::embedding(format!("Embedding task panicked: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_preset() {
        assert!(get_preset("balanced").is_some());
        assert!(get_preset("fast").is_some());
        assert!(get_preset("quality").is_some());
        assert!(get_preset("multilingual").is_some());
        assert!(get_preset("nonexistent").is_none());
    }

    #[test]
    fn test_list_presets() {
        let presets = list_presets();
        assert_eq!(presets.len(), 4);
        assert!(presets.contains(&"fast"));
        assert!(presets.contains(&"balanced"));
        assert!(presets.contains(&"quality"));
        assert!(presets.contains(&"multilingual"));
    }

    #[test]
    fn test_preset_dimensions() {
        let balanced = get_preset("balanced").unwrap();
        assert_eq!(balanced.dimensions, 768);

        let fast = get_preset("fast").unwrap();
        assert_eq!(fast.dimensions, 384);

        let quality = get_preset("quality").unwrap();
        assert_eq!(quality.dimensions, 1024);
    }

    #[test]
    fn test_preset_chunk_sizes() {
        let fast = get_preset("fast").unwrap();
        assert_eq!(fast.chunk_size, 512);
        assert_eq!(fast.overlap, 50);

        let quality = get_preset("quality").unwrap();
        assert_eq!(quality.chunk_size, 2000);
        assert_eq!(quality.overlap, 200);
    }

    #[test]
    fn test_preset_model_repos() {
        let fast = get_preset("fast").unwrap();
        assert_eq!(fast.model_repo, "Xenova/all-MiniLM-L6-v2");
        assert_eq!(fast.pooling, "mean");
        assert_eq!(fast.model_file, "onnx/model_quantized.onnx");

        let balanced = get_preset("balanced").unwrap();
        assert_eq!(balanced.model_repo, "Xenova/bge-base-en-v1.5");
        assert_eq!(balanced.pooling, "cls");
    }

    #[test]
    fn test_embed_texts_rejects_empty_string() {
        let config = crate::core::config::EmbeddingConfig::default();
        let texts = vec!["valid", ""];
        let err = embed_texts(&texts, &config).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("position 2"),
            "Error should identify the empty text position, got: {msg}"
        );
        assert!(msg.contains("empty"), "Error should mention empty text, got: {msg}");
    }

    #[test]
    fn test_embed_texts_empty_list_returns_empty() {
        let config = crate::core::config::EmbeddingConfig::default();
        let texts: Vec<&str> = vec![];
        let result = embed_texts(&texts, &config).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_embed_texts_rejects_first_empty_string() {
        let config = crate::core::config::EmbeddingConfig::default();
        let texts = vec![""];
        let err = embed_texts(&texts, &config).unwrap_err();
        assert!(err.to_string().contains("position 1"));
    }

    /// Regression test for #713: embed_texts called from inside a tokio runtime
    /// (e.g. server mode) must not panic with "cannot block inside runtime".
    /// The LLM path will fail with MissingDependency or a connection error,
    /// but it must NOT panic.
    #[cfg(feature = "liter-llm")]
    #[tokio::test]
    async fn test_embed_texts_llm_inside_runtime_does_not_panic() {
        let config = crate::core::config::EmbeddingConfig {
            model: crate::core::config::EmbeddingModelType::Llm {
                llm: crate::core::config::LlmConfig {
                    model: "openai/text-embedding-3-small".to_string(),
                    api_key: Some("invalid-key-for-test".to_string()),
                    base_url: None,
                    timeout_secs: None,
                    max_retries: None,
                    temperature: None,
                    max_tokens: None,
                },
            },
            ..Default::default()
        };
        // This should return an error (bad API key), NOT panic.
        let result = tokio::task::spawn_blocking(move || embed_texts(&["test text"], &config)).await;
        assert!(result.is_ok(), "spawn_blocking should not panic");
        // The inner result should be an error (auth failure), not a panic
        assert!(result.unwrap().is_err(), "Expected auth error, not success");
    }

    // ── Stale lock cleanup tests ──────────────────────────────────────────────

    /// Helper: write a file with a modified-time set to `age` seconds in the past.
    fn write_file_aged(path: &std::path::Path, age_secs: u64) {
        std::fs::write(path, b"").unwrap();
        let mtime = std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(age_secs))
            .unwrap();
        let ft = filetime::FileTime::from_system_time(mtime);
        filetime::set_file_mtime(path, ft).unwrap();
    }

    #[test]
    fn test_cleanup_stale_locks_nonexistent_dir_is_noop() {
        // Should not panic when the blobs dir does not exist.
        let tmp = tempfile::tempdir().unwrap();
        cleanup_stale_locks(tmp.path(), "org/model");
    }

    #[test]
    fn test_cleanup_stale_locks_removes_old_lock_and_part() {
        let tmp = tempfile::tempdir().unwrap();
        let blobs = tmp.path().join("models--org--model").join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();

        let lock = blobs.join("abc123.lock");
        let part = blobs.join("abc123.part");

        // Write files aged beyond the timeout.
        let old_secs = STALE_DOWNLOAD_TIMEOUT.as_secs() + 60;
        write_file_aged(&lock, old_secs);
        write_file_aged(&part, old_secs);

        cleanup_stale_locks(tmp.path(), "org/model");

        assert!(!lock.exists(), ".lock should have been removed");
        assert!(!part.exists(), ".part should have been removed");
    }

    #[test]
    fn test_cleanup_stale_locks_leaves_recent_files_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let blobs = tmp.path().join("models--org--model").join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();

        let lock = blobs.join("def456.lock");
        let part = blobs.join("def456.part");

        // Write files that are only 60 seconds old — well within the timeout.
        write_file_aged(&lock, 60);
        write_file_aged(&part, 60);

        cleanup_stale_locks(tmp.path(), "org/model");

        assert!(lock.exists(), ".lock for active download must not be removed");
        assert!(part.exists(), ".part for active download must not be removed");
    }

    #[test]
    fn test_cleanup_stale_locks_removes_lock_when_no_part() {
        // When only a .lock file exists (download killed before first byte),
        // staleness is assessed from the .lock file's own mtime.
        let tmp = tempfile::tempdir().unwrap();
        let blobs = tmp.path().join("models--org--model").join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();

        let lock = blobs.join("ghi789.lock");
        let old_secs = STALE_DOWNLOAD_TIMEOUT.as_secs() + 60;
        write_file_aged(&lock, old_secs);

        cleanup_stale_locks(tmp.path(), "org/model");

        assert!(!lock.exists(), "stale .lock with no .part should be removed");
    }

    #[test]
    fn test_lock_acquisition_hint_contains_recovery_commands() {
        let cache = std::path::Path::new("/tmp/kreuzberg/embeddings");
        let hint = lock_acquisition_hint(cache, "intfloat/multilingual-e5-base");

        assert!(hint.contains("rm -f"), "hint must include rm command");
        assert!(
            hint.contains("models--intfloat--multilingual-e5-base"),
            "hint must include the repo folder name"
        );
        assert!(hint.contains("*.lock"), "hint must mention .lock pattern");
        assert!(hint.contains("*.part"), "hint must mention .part pattern");
    }

    /// Regression test for #683: GraphOptimizationLevel::Level3 maps to
    /// ORT_ENABLE_LAYOUT (3), only valid in ORT >= 1.21. The correct variant
    /// for "all optimisations" is ::All (ORT_ENABLE_ALL = 99), valid across
    /// every ORT 1.x release.
    #[cfg(feature = "embeddings")]
    #[test]
    fn test_ort_optimization_level_all_not_level3() {
        use ort::session::builder::GraphOptimizationLevel;
        let all_repr = format!("{:?}", GraphOptimizationLevel::All);
        let level3_repr = format!("{:?}", GraphOptimizationLevel::Level3);
        assert_eq!(all_repr, "All");
        assert_ne!(level3_repr, "All", "Level3 must not be the same variant as All");
    }

    // --- Plugin variant dispatch tests -----------------------------------

    mod plugin_dispatch {
        use crate::plugins::embedding::{register_embedding_backend, unregister_embedding_backend};
        use crate::plugins::{EmbeddingBackend, Plugin};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU64, Ordering};

        fn unique_name(suffix: &str) -> String {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            format!("dispatch-{suffix}-{id}")
        }

        /// Backend whose `embed` response shape is fully parameterised so tests
        /// can exercise the validation paths (length mismatch, dim mismatch).
        struct ConfigurableBackend {
            name: String,
            reported_dimensions: usize,
            vector_dimensions: usize,
            response_count: Option<usize>, // None → one vector per input (correct); Some(n) → force n
            panic_on_embed: bool,
            fill_value: f32,
        }

        impl Plugin for ConfigurableBackend {
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

        #[async_trait::async_trait]
        impl EmbeddingBackend for ConfigurableBackend {
            fn dimensions(&self) -> usize {
                self.reported_dimensions
            }

            async fn embed(&self, texts: Vec<String>) -> crate::Result<Vec<Vec<f32>>> {
                if self.panic_on_embed {
                    return Err(crate::KreuzbergError::Plugin {
                        message: "simulated backend failure".to_string(),
                        plugin_name: self.name.clone(),
                    });
                }
                let count = self.response_count.unwrap_or(texts.len());
                Ok((0..count)
                    .map(|_| vec![self.fill_value; self.vector_dimensions])
                    .collect())
            }
        }

        fn config_for(name: &str, normalize: bool) -> crate::core::config::EmbeddingConfig {
            crate::core::config::EmbeddingConfig {
                model: crate::core::config::EmbeddingModelType::Plugin { name: name.to_string() },
                normalize,
                ..Default::default()
            }
        }

        #[test]
        fn dispatches_to_registered_backend() {
            let name = unique_name("happy");
            register_embedding_backend(Arc::new(ConfigurableBackend {
                name: name.clone(),
                reported_dimensions: 4,
                vector_dimensions: 4,
                response_count: None,
                panic_on_embed: false,
                fill_value: 0.25,
            }))
            .unwrap();

            let vectors = super::super::embed_texts(&["a", "b", "c"], &config_for(&name, false)).unwrap();
            assert_eq!(vectors.len(), 3);
            assert!(vectors.iter().all(|v| v.len() == 4 && v[0] == 0.25));

            unregister_embedding_backend(&name).unwrap();
        }

        #[test]
        fn unknown_plugin_name_errors() {
            let config = config_for("never-registered-x", false);
            let err = super::super::embed_texts(&["a"], &config).unwrap_err();
            assert!(matches!(err, crate::KreuzbergError::Plugin { .. }));
        }

        #[test]
        fn length_mismatch_surfaces_as_validation_error() {
            let name = unique_name("len-mismatch");
            register_embedding_backend(Arc::new(ConfigurableBackend {
                name: name.clone(),
                reported_dimensions: 3,
                vector_dimensions: 3,
                response_count: Some(2), // 2 vectors for 3 inputs
                panic_on_embed: false,
                fill_value: 0.0,
            }))
            .unwrap();

            let err = super::super::embed_texts(&["a", "b", "c"], &config_for(&name, false)).unwrap_err();
            let msg = err.to_string();
            assert!(
                matches!(err, crate::KreuzbergError::Validation { .. }),
                "expected Validation error, got {err:?}"
            );
            assert!(msg.contains('2') && msg.contains('3'), "message: {msg}");

            unregister_embedding_backend(&name).unwrap();
        }

        #[test]
        fn dimension_mismatch_surfaces_as_validation_error() {
            let name = unique_name("dim-mismatch");
            register_embedding_backend(Arc::new(ConfigurableBackend {
                name: name.clone(),
                reported_dimensions: 4,
                vector_dimensions: 5, // wrong
                response_count: None,
                panic_on_embed: false,
                fill_value: 0.0,
            }))
            .unwrap();

            let err = super::super::embed_texts(&["a", "b"], &config_for(&name, false)).unwrap_err();
            assert!(matches!(err, crate::KreuzbergError::Validation { .. }));
            let msg = err.to_string();
            assert!(msg.contains("index 0"), "message should cite bad index: {msg}");

            unregister_embedding_backend(&name).unwrap();
        }

        #[test]
        fn backend_error_surfaces_as_plugin_error() {
            let name = unique_name("err");
            register_embedding_backend(Arc::new(ConfigurableBackend {
                name: name.clone(),
                reported_dimensions: 3,
                vector_dimensions: 3,
                response_count: None,
                panic_on_embed: true, // flag now means "return Plugin error"
                fill_value: 0.0,
            }))
            .unwrap();

            let err = super::super::embed_texts(&["a"], &config_for(&name, false)).unwrap_err();
            assert!(matches!(err, crate::KreuzbergError::Plugin { .. }));
            assert!(err.to_string().contains("simulated backend failure"));

            unregister_embedding_backend(&name).unwrap();
        }

        #[test]
        fn empty_texts_short_circuits_before_backend_call() {
            // No backend registered under this name — if we don't short-circuit,
            // the dispatch would fail with a Plugin-not-registered error.
            let config = config_for("never-looked-up", false);
            let texts: Vec<&str> = vec![];
            let vectors = super::super::embed_texts(&texts, &config).unwrap();
            assert!(vectors.is_empty());
        }

        #[test]
        fn concurrent_registration_stress() {
            use std::thread;
            // Stress-test the registry under concurrent registrations.
            // 8 threads each register 10 uniquely-named backends. Assert final
            // list() contains all 80 + each dispatch still resolves cleanly.
            let mut handles = Vec::new();
            let prefix = unique_name("stress");
            for t in 0..8 {
                let prefix = prefix.clone();
                handles.push(thread::spawn(move || {
                    for i in 0..10 {
                        let name = format!("{prefix}-t{t}-i{i}");
                        register_embedding_backend(Arc::new(ConfigurableBackend {
                            name: name.clone(),
                            reported_dimensions: 2,
                            vector_dimensions: 2,
                            response_count: None,
                            panic_on_embed: false,
                            fill_value: 0.5,
                        }))
                        .unwrap();
                    }
                }));
            }
            for h in handles {
                h.join().unwrap();
            }

            let list = crate::plugins::embedding::list_embedding_backends().unwrap();
            let registered = list.iter().filter(|n| n.starts_with(&prefix)).count();
            assert_eq!(registered, 80, "expected 80 registrations, got {registered}");

            // Dispatch should still resolve any of them.
            let sample = format!("{prefix}-t0-i0");
            let vectors = super::super::embed_texts(&["probe"], &config_for(&sample, false)).unwrap();
            assert_eq!(vectors.len(), 1);

            // Clean up the 80 we created.
            for t in 0..8 {
                for i in 0..10 {
                    let name = format!("{prefix}-t{t}-i{i}");
                    let _ = crate::plugins::embedding::unregister_embedding_backend(&name);
                }
            }
        }

        /// Backend that sleeps longer than the configured timeout — exercises
        /// the tokio::time::timeout wrapper in the dispatch arm.
        struct SlowBackend {
            name: String,
            sleep_duration: std::time::Duration,
        }

        impl Plugin for SlowBackend {
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

        #[async_trait::async_trait]
        impl EmbeddingBackend for SlowBackend {
            fn dimensions(&self) -> usize {
                4
            }

            async fn embed(&self, texts: Vec<String>) -> crate::Result<Vec<Vec<f32>>> {
                tokio::time::sleep(self.sleep_duration).await;
                Ok(texts.iter().map(|_| vec![0.0; 4]).collect())
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn timeout_fires_when_backend_exceeds_duration() {
            let name = unique_name("timeout");
            // Backend sleeps 2 seconds; timeout is 1 second.
            register_embedding_backend(Arc::new(SlowBackend {
                name: name.clone(),
                sleep_duration: std::time::Duration::from_secs(2),
            }))
            .unwrap();

            let config = crate::core::config::EmbeddingConfig {
                model: crate::core::config::EmbeddingModelType::Plugin { name: name.clone() },
                max_embed_duration_secs: Some(1),
                ..Default::default()
            };

            let err = super::super::embed_texts(&["probe"], &config).expect_err("timeout should fire");
            assert!(
                matches!(err, crate::KreuzbergError::Plugin { .. }),
                "expected Plugin error, got {err:?}"
            );
            let msg = err.to_string();
            assert!(
                msg.contains("did not complete within"),
                "error message should mention timeout; got: {msg}"
            );

            unregister_embedding_backend(&name).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn async_dispatch_applies_normalization_when_enabled() {
            // Complements normalization_applied_when_enabled (sync path): makes
            // sure the async fast path calls normalize_embeddings too. Any
            // refactor that drops the normalize step only on one path gets
            // caught.
            let name = unique_name("async-normalize");
            register_embedding_backend(Arc::new(ConfigurableBackend {
                name: name.clone(),
                reported_dimensions: 2,
                vector_dimensions: 2,
                response_count: None,
                panic_on_embed: false,
                fill_value: 3.0, // non-unit-norm — must be normalized to unit length
            }))
            .unwrap();

            let texts: Vec<String> = vec!["probe".to_string()];
            let vectors = super::super::embed_texts_async(texts, &config_for(&name, true))
                .await
                .expect("async dispatch should succeed");
            let v = &vectors[0];
            let mag = (v[0] * v[0] + v[1] * v[1]).sqrt();
            assert!(
                (mag - 1.0).abs() < 1e-6,
                "expected unit-norm after normalize=true on async path; got mag={mag}"
            );

            unregister_embedding_backend(&name).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn async_dispatch_smoke_test() {
            // Smoke-test embed_texts_async for the Plugin variant — routes
            // through the async fast path, returns the expected shape. A
            // regression that silently re-routed Plugin through spawn_blocking
            // would still pass this test (it only checks output); keep that
            // stronger property enforced by code review + the normalize +
            // timeout tests above.
            let name = unique_name("async-path");
            register_embedding_backend(Arc::new(ConfigurableBackend {
                name: name.clone(),
                reported_dimensions: 3,
                vector_dimensions: 3,
                response_count: None,
                panic_on_embed: false,
                fill_value: 0.5,
            }))
            .unwrap();

            let config = config_for(&name, false);
            let texts: Vec<String> = vec!["x".to_string(), "y".to_string()];
            let vectors = super::super::embed_texts_async(texts, &config)
                .await
                .expect("async dispatch should succeed");
            assert_eq!(vectors.len(), 2);
            assert!(vectors.iter().all(|v| v.len() == 3 && v[0] == 0.5));

            unregister_embedding_backend(&name).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn disabled_timeout_allows_slow_backend_to_complete() {
            // Complement to the timeout test: with None, a slow backend must
            // still succeed (sleep under test threshold).
            let name = unique_name("no-timeout");
            register_embedding_backend(Arc::new(SlowBackend {
                name: name.clone(),
                sleep_duration: std::time::Duration::from_millis(100),
            }))
            .unwrap();

            let config = crate::core::config::EmbeddingConfig {
                model: crate::core::config::EmbeddingModelType::Plugin { name: name.clone() },
                max_embed_duration_secs: None,
                ..Default::default()
            };

            let result = super::super::embed_texts(&["probe"], &config);
            assert!(result.is_ok(), "expected Ok with timeout disabled; got {result:?}");

            unregister_embedding_backend(&name).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn zero_max_duration_treated_as_disabled() {
            // `Some(0)` is a config nonsense value — a zero-duration timeout would
            // fire before the backend ever runs. Dispatch should treat it as
            // equivalent to `None` (no timeout) instead of erroring on every call.
            let name = unique_name("zero-timeout");
            register_embedding_backend(Arc::new(SlowBackend {
                name: name.clone(),
                sleep_duration: std::time::Duration::from_millis(50),
            }))
            .unwrap();

            let config = crate::core::config::EmbeddingConfig {
                model: crate::core::config::EmbeddingModelType::Plugin { name: name.clone() },
                max_embed_duration_secs: Some(0),
                ..Default::default()
            };

            let result = super::super::embed_texts(&["probe"], &config);
            assert!(
                result.is_ok(),
                "expected Ok with Some(0) treated as disabled; got {result:?}"
            );

            unregister_embedding_backend(&name).unwrap();
        }

        #[test]
        fn normalization_applied_when_enabled() {
            let name = unique_name("normalize");
            register_embedding_backend(Arc::new(ConfigurableBackend {
                name: name.clone(),
                reported_dimensions: 2,
                vector_dimensions: 2,
                response_count: None,
                panic_on_embed: false,
                fill_value: 3.0, // non-unit-norm input
            }))
            .unwrap();

            let vectors = super::super::embed_texts(&["a"], &config_for(&name, true)).unwrap();
            let v = &vectors[0];
            let mag = (v[0] * v[0] + v[1] * v[1]).sqrt();
            assert!(
                (mag - 1.0).abs() < 1e-6,
                "expected unit-norm after normalize=true, got mag={mag}"
            );

            unregister_embedding_backend(&name).unwrap();
        }
    }

    // --- validate_embedding_shape direct tests ---------------------------

    #[test]
    fn validate_shape_accepts_correct_response() {
        let embeddings = vec![vec![0.0; 4]; 3];
        super::validate_embedding_shape(&embeddings, 3, 4, "ok").unwrap();
    }

    #[test]
    fn validate_shape_rejects_count_mismatch() {
        let embeddings = vec![vec![0.0; 4]; 2];
        let err = super::validate_embedding_shape(&embeddings, 3, 4, "bad-count").unwrap_err();
        assert!(matches!(err, crate::KreuzbergError::Validation { .. }));
    }

    #[test]
    fn validate_shape_rejects_dim_mismatch() {
        let embeddings = vec![vec![0.0; 4], vec![0.0; 3], vec![0.0; 4]];
        let err = super::validate_embedding_shape(&embeddings, 3, 4, "bad-dim").unwrap_err();
        assert!(matches!(err, crate::KreuzbergError::Validation { .. }));
        assert!(err.to_string().contains("index 1"));
    }

    #[test]
    fn validate_shape_empty_expected_count_ok() {
        super::validate_embedding_shape(&[], 0, 4, "empty").unwrap();
    }
}
