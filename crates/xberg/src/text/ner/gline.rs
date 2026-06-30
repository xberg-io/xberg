//! `xberg-gliner` ONNX backend for named-entity recognition.
//!
//! Runs span-mode [GLiNER](https://huggingface.co/gliner-community) models
//! exported to ONNX and published under `xberg-io/gliner-models`. The source
//! model lineage is `gliner-community`; xberg consumes only the checked ONNX
//! runtime artifacts and tokenizer files.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

use ahash::AHashMap;
use async_trait::async_trait;
use parking_lot::RwLock;
use xberg_gliner::{Gliner, Gliner2, Parameters, RuntimeConfig, TextInput};

use crate::Result;
use crate::types::entity::{Entity, EntityCategory};

use super::backend::NerBackend;

/// Hugging Face repository that stores xberg-managed GLiNER ONNX exports.
pub const GLINER_MODELS_REPO: &str = "xberg-io/gliner-models";

const CHECKSUMS_FILE: &str = "checksums.sha256";

/// Default xberg GLiNER model alias.
pub const DEFAULT_MODEL_NAME: &str = "balanced";

/// Backwards-compatible constant for code that described the old default as a repository.
pub const DEFAULT_MODEL_REPO: &str = GLINER_MODELS_REPO;

/// Canonical GLiNER model ids xberg knows how to download.
///
/// Used by `xberg cache warm --all-ner-models` to pre-fetch the supported fleet.
pub const KNOWN_MODELS: &[&str] = &["gliner_small-v2.5", "gliner_medium-v2.5", "gliner_large-v2.5"];

/// Default entity labels used when the caller supplies an empty `categories` slice.
const DEFAULT_LABELS: &[&str] = &["person", "organization", "location", "date", "email"];

static PUBLISH_COUNTER: AtomicU64 = AtomicU64::new(0);

type BackendCache = AHashMap<GlineBackendCacheKey, Arc<GlineBackend>>;

static BACKEND_CACHE: LazyLock<RwLock<BackendCache>> = LazyLock::new(|| RwLock::new(AHashMap::default()));

#[derive(Debug, Clone, Copy)]
struct GlinerModelDefinition {
    id: &'static str,
    aliases: &'static [&'static str],
    upstream_repo: &'static str,
    mode: &'static str,
    variant: &'static str,
    model_file: &'static str,
    tokenizer_file: &'static str,
}

const GLINER_MODELS: &[GlinerModelDefinition] = &[
    GlinerModelDefinition {
        id: "gliner_small-v2.5",
        aliases: &["fast"],
        upstream_repo: "gliner-community/gliner_small-v2.5",
        mode: "span",
        variant: "fp32",
        model_file: "models/gliner_small-v2.5/span/fp32/model.onnx",
        tokenizer_file: "models/gliner_small-v2.5/span/fp32/tokenizer.json",
    },
    GlinerModelDefinition {
        id: "gliner_medium-v2.5",
        aliases: &["balanced", "multilingual"],
        upstream_repo: "gliner-community/gliner_medium-v2.5",
        mode: "span",
        variant: "fp32",
        model_file: "models/gliner_medium-v2.5/span/fp32/model.onnx",
        tokenizer_file: "models/gliner_medium-v2.5/span/fp32/tokenizer.json",
    },
    GlinerModelDefinition {
        id: "gliner_large-v2.5",
        aliases: &["quality"],
        upstream_repo: "gliner-community/gliner_large-v2.5",
        mode: "span",
        variant: "fp32",
        model_file: "models/gliner_large-v2.5/span/fp32/model.onnx",
        tokenizer_file: "models/gliner_large-v2.5/span/fp32/tokenizer.json",
    },
];

#[derive(Debug, Clone)]
struct GlinerModelFiles {
    id: String,
    model_path: PathBuf,
    tokenizer_path: PathBuf,
}

/// Caller-supplied override pointing GLiNER at an arbitrary Hugging Face repo
/// instead of the pinned `xberg-io/gliner-models` catalog.
///
/// Files downloaded from a custom repo are **not** checksum-verified — the
/// catalog's `checksums.sha256` only covers the pinned models xberg publishes.
/// Callers choosing a custom repo are trusting that source directly.
#[derive(Debug, Clone)]
pub struct CustomGlinerSource {
    /// Hugging Face repo id, e.g. `"gliner-community/gliner_small-v2.5"`.
    pub repo: String,
    /// Path to the ONNX model file within `repo`.
    pub model_file: String,
    /// Path to the tokenizer file within `repo`.
    pub tokenizer_file: String,
    /// Which GLiNER tensor I/O contract `model_file` uses.
    pub architecture: crate::core::config::ner::GlinerArchitecture,
}

/// Build a [`CustomGlinerSource`] from optional config fields.
///
/// Returns `Ok(None)` when `repo`/`model_file`/`tokenizer_file` are all unset
/// (use the pinned catalog), `Ok(Some(_))` when all three are set, and `Err`
/// when only some are set. `architecture` is independent of that all-or-nothing
/// rule — `None` defaults to [`crate::core::config::ner::GlinerArchitecture::Gliner1`].
pub fn custom_source_from_parts(
    repo: Option<&str>,
    model_file: Option<&str>,
    tokenizer_file: Option<&str>,
    architecture: Option<crate::core::config::ner::GlinerArchitecture>,
) -> Result<Option<CustomGlinerSource>> {
    match (repo, model_file, tokenizer_file) {
        (None, None, None) => Ok(None),
        (Some(repo), Some(model_file), Some(tokenizer_file)) => Ok(Some(CustomGlinerSource {
            repo: repo.to_string(),
            model_file: model_file.to_string(),
            tokenizer_file: tokenizer_file.to_string(),
            architecture: architecture.unwrap_or_default(),
        })),
        _ => Err(crate::XbergError::validation(
            "NerConfig.hf_repo, hf_model_file, and hf_tokenizer_file must all be set together, or all left unset",
        )),
    }
}

#[cfg_attr(alef, alef(skip))]
/// A single GLiNER model artifact entry in the cache manifest.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GlinerManifestEntry {
    /// Relative path within the xberg cache directory.
    pub relative_path: String,
    /// SHA256 checksum of the model file when pinned locally.
    pub sha256: String,
    /// Expected file size in bytes. Zero means unknown.
    pub size_bytes: u64,
    /// Hugging Face source URL for downloading.
    pub source_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GlineBackendCacheKey {
    model_id: String,
    thread_budget: usize,
}

/// Eagerly fetch a GLiNER model (ONNX + tokenizer) into the xberg cache.
///
/// `name` must be a supported xberg GLiNER model alias or catalog id. Runtime
/// artifacts are downloaded from `xberg-io/gliner-models`.
pub fn download_model(name: &str, cache_dir: Option<PathBuf>) -> Result<PathBuf> {
    Ok(ensure_model(name, cache_dir)?.model_path)
}

fn ensure_model(name: &str, cache_dir: Option<PathBuf>) -> Result<GlinerModelFiles> {
    let definition = resolve_model(name)?;
    let base_dir = cache_dir.unwrap_or_else(|| crate::cache_dir::resolve_cache_dir("ner"));
    let checksums = load_gliner_checksums(&base_dir)?;
    let model_sha256 = required_checksum(&checksums, definition.model_file)?;
    let tokenizer_sha256 = required_checksum(&checksums, definition.tokenizer_file)?;
    let model_dir = base_dir
        .join("gliner")
        .join(definition.id)
        .join(definition.mode)
        .join(definition.variant);
    let model_path = model_dir.join("model.onnx");
    let tokenizer_path = model_dir.join("tokenizer.json");

    if model_path.exists() && tokenizer_path.exists() {
        let model_verified =
            crate::model_download::verify_sha256(&model_path, model_sha256, "ner-gliner-model").is_ok();
        let tokenizer_verified =
            crate::model_download::verify_sha256(&tokenizer_path, tokenizer_sha256, "ner-gliner-tokenizer").is_ok();
        if model_verified && tokenizer_verified {
            tracing::debug!(model = definition.id, "GLiNER model found in cache");
            return Ok(GlinerModelFiles {
                id: definition.id.to_string(),
                model_path,
                tokenizer_path,
            });
        }
        tracing::warn!(
            model = definition.id,
            "cached GLiNER files failed checksum verification; refreshing"
        );
    }

    std::fs::create_dir_all(&model_dir).map_err(|error| crate::XbergError::Plugin {
        message: format!("Failed to create GLiNER cache dir '{}': {error}", model_dir.display()),
        plugin_name: "ner-gliner".to_string(),
    })?;

    let cached_model =
        crate::model_download::hf_download(GLINER_MODELS_REPO, definition.model_file).map_err(|error| {
            crate::XbergError::Plugin {
                message: format!(
                    "Failed to download GLiNER model '{}' from {}: {error}",
                    definition.id, GLINER_MODELS_REPO
                ),
                plugin_name: "ner-gliner".to_string(),
            }
        })?;
    crate::model_download::verify_sha256(&cached_model, model_sha256, "ner-gliner-model")
        .map_err(|error| crate::XbergError::validation(format!("GLiNER model SHA256 verification failed: {error}")))?;
    atomic_publish(&cached_model, &model_path, &model_dir, model_sha256, "ner-gliner-model").map_err(|error| {
        crate::XbergError::Plugin {
            message: error,
            plugin_name: "ner-gliner".to_string(),
        }
    })?;

    let cached_tokenizer =
        crate::model_download::hf_download(GLINER_MODELS_REPO, definition.tokenizer_file).map_err(|error| {
            crate::XbergError::Plugin {
                message: format!(
                    "Failed to download GLiNER tokenizer '{}' from {}: {error}",
                    definition.id, GLINER_MODELS_REPO
                ),
                plugin_name: "ner-gliner".to_string(),
            }
        })?;
    crate::model_download::verify_sha256(&cached_tokenizer, tokenizer_sha256, "ner-gliner-tokenizer").map_err(
        |error| crate::XbergError::validation(format!("GLiNER tokenizer SHA256 verification failed: {error}")),
    )?;
    atomic_publish(
        &cached_tokenizer,
        &tokenizer_path,
        &model_dir,
        tokenizer_sha256,
        "ner-gliner-tokenizer",
    )
    .map_err(|error| crate::XbergError::Plugin {
        message: error,
        plugin_name: "ner-gliner".to_string(),
    })?;

    tracing::info!(
        model = definition.id,
        upstream = definition.upstream_repo,
        model_path = %model_path.display(),
        tokenizer_path = %tokenizer_path.display(),
        "xberg-gliner model downloaded"
    );

    Ok(GlinerModelFiles {
        id: definition.id.to_string(),
        model_path,
        tokenizer_path,
    })
}

/// Download a GLiNER model (ONNX + tokenizer) from an arbitrary Hugging Face
/// repo, bypassing the pinned `xberg-io/gliner-models` catalog.
///
/// Unlike [`ensure_model`], downloaded files are **not** checksum-verified —
/// there is no pinned manifest for caller-chosen repos. Cached under a
/// content-derived directory so distinct `(repo, model_file, tokenizer_file)`
/// triples never collide.
fn ensure_custom_model(source: &CustomGlinerSource, cache_dir: Option<PathBuf>) -> Result<GlinerModelFiles> {
    let repo = source.repo.trim();
    let model_file = source.model_file.trim();
    let tokenizer_file = source.tokenizer_file.trim();
    if repo.is_empty() || model_file.is_empty() || tokenizer_file.is_empty() {
        return Err(crate::XbergError::validation(
            "Custom GLiNER source requires hf_repo, hf_model_file, and hf_tokenizer_file to all be non-empty",
        ));
    }

    let base_dir = cache_dir.unwrap_or_else(|| crate::cache_dir::resolve_cache_dir("ner"));
    let cache_key = custom_cache_key(repo, model_file, tokenizer_file, source.architecture);
    let model_dir = base_dir.join("gliner").join("custom").join(&cache_key);
    let model_path = model_dir.join("model.onnx");
    let tokenizer_path = model_dir.join("tokenizer.json");
    let id = format!("custom:{repo}/{model_file}");

    if model_path.exists() && tokenizer_path.exists() {
        tracing::debug!(repo, model_file, "custom GLiNER model found in cache");
        return Ok(GlinerModelFiles {
            id,
            model_path,
            tokenizer_path,
        });
    }

    std::fs::create_dir_all(&model_dir).map_err(|error| crate::XbergError::Plugin {
        message: format!("Failed to create custom GLiNER cache dir '{}': {error}", model_dir.display()),
        plugin_name: "ner-gliner".to_string(),
    })?;

    let cached_model = crate::model_download::hf_download(repo, model_file).map_err(|error| crate::XbergError::Plugin {
        message: format!("Failed to download custom GLiNER model '{model_file}' from {repo}: {error}"),
        plugin_name: "ner-gliner".to_string(),
    })?;
    atomic_publish(&cached_model, &model_path, &model_dir, "", "ner-gliner-custom-model")
        .map_err(|error| crate::XbergError::Plugin { message: error, plugin_name: "ner-gliner".to_string() })?;

    let cached_tokenizer =
        crate::model_download::hf_download(repo, tokenizer_file).map_err(|error| crate::XbergError::Plugin {
            message: format!("Failed to download custom GLiNER tokenizer '{tokenizer_file}' from {repo}: {error}"),
            plugin_name: "ner-gliner".to_string(),
        })?;
    atomic_publish(&cached_tokenizer, &tokenizer_path, &model_dir, "", "ner-gliner-custom-tokenizer")
        .map_err(|error| crate::XbergError::Plugin { message: error, plugin_name: "ner-gliner".to_string() })?;

    tracing::info!(
        repo,
        model_file,
        tokenizer_file,
        model_path = %model_path.display(),
        tokenizer_path = %tokenizer_path.display(),
        "xberg-gliner custom model downloaded (unverified — no checksum pinning for caller-supplied repos)"
    );

    Ok(GlinerModelFiles {
        id,
        model_path,
        tokenizer_path,
    })
}

/// Content-derived cache directory name for a custom GLiNER source, so
/// distinct `(repo, model_file, tokenizer_file, architecture)` tuples never
/// collide and arbitrary caller-supplied strings never escape the cache directory.
fn custom_cache_key(
    repo: &str,
    model_file: &str,
    tokenizer_file: &str,
    architecture: crate::core::config::ner::GlinerArchitecture,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(repo.as_bytes());
    hasher.update(b"\0");
    hasher.update(model_file.as_bytes());
    hasher.update(b"\0");
    hasher.update(tokenizer_file.as_bytes());
    hasher.update(b"\0");
    hasher.update(architecture_tag(architecture).as_bytes());
    hex::encode(hasher.finalize())
}

fn architecture_tag(architecture: crate::core::config::ner::GlinerArchitecture) -> &'static str {
    use crate::core::config::ner::GlinerArchitecture;
    match architecture {
        GlinerArchitecture::Gliner1 => "gliner1",
        GlinerArchitecture::Gliner2 => "gliner2",
    }
}

/// Returns the GLiNER files expected by `xberg cache manifest`.
#[cfg_attr(alef, alef(skip))]
pub fn manifest() -> Vec<GlinerManifestEntry> {
    let mut entries = vec![GlinerManifestEntry {
        relative_path: format!("ner/gliner/{CHECKSUMS_FILE}"),
        sha256: String::new(),
        size_bytes: 0,
        source_url: format!("https://huggingface.co/{GLINER_MODELS_REPO}/resolve/main/{CHECKSUMS_FILE}"),
    }];

    for definition in GLINER_MODELS {
        let cache_prefix = format!(
            "ner/gliner/{}/{}/{}",
            definition.id, definition.mode, definition.variant
        );
        entries.push(GlinerManifestEntry {
            relative_path: format!("{cache_prefix}/model.onnx"),
            sha256: String::new(),
            size_bytes: 0,
            source_url: format!(
                "https://huggingface.co/{GLINER_MODELS_REPO}/resolve/main/{}",
                definition.model_file
            ),
        });
        entries.push(GlinerManifestEntry {
            relative_path: format!("{cache_prefix}/tokenizer.json"),
            sha256: String::new(),
            size_bytes: 0,
            source_url: format!(
                "https://huggingface.co/{GLINER_MODELS_REPO}/resolve/main/{}",
                definition.tokenizer_file
            ),
        });
    }
    entries
}

fn load_gliner_checksums(base_dir: &Path) -> Result<HashMap<String, String>> {
    let checksums_dir = base_dir.join("gliner");
    let checksums_path = checksums_dir.join(CHECKSUMS_FILE);
    if checksums_path.exists() {
        return read_gliner_checksums(&checksums_path);
    }

    let path = crate::model_download::hf_download(GLINER_MODELS_REPO, CHECKSUMS_FILE).map_err(|error| {
        crate::XbergError::Plugin {
            message: format!("Failed to download GLiNER checksums from {GLINER_MODELS_REPO}: {error}"),
            plugin_name: "ner-gliner".to_string(),
        }
    })?;
    let checksums = read_gliner_checksums(&path)?;
    std::fs::create_dir_all(&checksums_dir).map_err(|error| crate::XbergError::Plugin {
        message: format!(
            "Failed to create GLiNER checksums cache dir '{}': {error}",
            checksums_dir.display()
        ),
        plugin_name: "ner-gliner".to_string(),
    })?;
    atomic_publish(&path, &checksums_path, &checksums_dir, "", "ner-gliner-checksums").map_err(|error| {
        crate::XbergError::Plugin {
            message: error,
            plugin_name: "ner-gliner".to_string(),
        }
    })?;
    Ok(checksums)
}

fn read_gliner_checksums(path: &Path) -> Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path).map_err(|error| crate::XbergError::Plugin {
        message: format!("Failed to read GLiNER checksums '{}': {error}", path.display()),
        plugin_name: "ner-gliner".to_string(),
    })?;
    parse_checksums(&content)
}

fn parse_checksums(content: &str) -> Result<HashMap<String, String>> {
    let mut checksums = HashMap::new();
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let checksum = parts.next().ok_or_else(|| {
            crate::XbergError::validation(format!("Invalid GLiNER checksum line {}: missing checksum", index + 1))
        })?;
        let path = parts.next().ok_or_else(|| {
            crate::XbergError::validation(format!("Invalid GLiNER checksum line {}: missing path", index + 1))
        })?;
        if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(crate::XbergError::validation(format!(
                "Invalid GLiNER checksum line {}: checksum must be SHA256 hex",
                index + 1
            )));
        }
        checksums.insert(path.trim_start_matches("./").to_string(), checksum.to_ascii_lowercase());
    }
    Ok(checksums)
}

fn required_checksum<'a>(checksums: &'a HashMap<String, String>, path: &str) -> Result<&'a str> {
    checksums.get(path).map(String::as_str).ok_or_else(|| {
        crate::XbergError::validation(format!(
            "GLiNER checksums file does not include required artifact '{path}'"
        ))
    })
}

fn resolve_model(name: &str) -> Result<GlinerModelDefinition> {
    let requested = name.trim();
    if requested.is_empty() {
        return Err(crate::XbergError::validation("GLiNER model name must not be empty"));
    }

    GLINER_MODELS
        .iter()
        .copied()
        .find(|definition| definition.id == requested || definition.aliases.contains(&requested))
        .ok_or_else(|| {
            crate::XbergError::validation(format!(
                "Unknown GLiNER model '{requested}'. Available models: {}",
                available_model_names().join(", ")
            ))
        })
}

fn requested_model_name(model_name: Option<&str>) -> Result<String> {
    let requested = model_name.unwrap_or(DEFAULT_MODEL_NAME).trim();
    if requested.is_empty() {
        return Err(crate::XbergError::validation("GLiNER model name must not be empty"));
    }
    Ok(requested.to_string())
}

fn backend_cache_key(
    model_name: Option<&str>,
    custom_source: Option<&CustomGlinerSource>,
    thread_budget: usize,
) -> Result<GlineBackendCacheKey> {
    let model_id = match custom_source {
        Some(source) => format!(
            "custom:{}",
            custom_cache_key(
                source.repo.trim(),
                source.model_file.trim(),
                source.tokenizer_file.trim(),
                source.architecture,
            )
        ),
        None => {
            let requested = requested_model_name(model_name)?;
            resolve_model(&requested)?.id.to_string()
        }
    };
    Ok(GlineBackendCacheKey { model_id, thread_budget })
}

fn available_model_names() -> Vec<&'static str> {
    let mut names = Vec::new();
    for definition in GLINER_MODELS {
        names.push(definition.id);
        names.extend(definition.aliases);
    }
    names.sort_unstable();
    names
}

fn atomic_publish(
    src: &Path,
    dst: &Path,
    dst_dir: &Path,
    sha256: &str,
    label: &str,
) -> std::result::Result<(), String> {
    let stem = dst.file_name().and_then(|name| name.to_str()).unwrap_or("model");
    let tmp = dst_dir.join(format!(
        ".{stem}.{}.{}.tmp",
        std::process::id(),
        PUBLISH_COUNTER.fetch_add(1, Ordering::Relaxed),
    ));

    std::fs::copy(src, &tmp).map_err(|error| {
        let _ = std::fs::remove_file(&tmp);
        format!("Failed to stage GLiNER model at {}: {error}", tmp.display())
    })?;

    if let Err(error) = crate::model_download::verify_sha256(&tmp, sha256, label) {
        let _ = std::fs::remove_file(&tmp);
        return Err(error);
    }

    match std::fs::rename(&tmp, dst) {
        Ok(()) => Ok(()),
        Err(_) if dst.exists() && crate::model_download::verify_sha256(dst, sha256, label).is_ok() => {
            let _ = std::fs::remove_file(&tmp);
            tracing::debug!(path = %dst.display(), "GLiNER cache file already present");
            Ok(())
        }
        Err(error) if dst.exists() => {
            let _ = std::fs::remove_file(&tmp);
            Err(format!(
                "Failed to publish GLiNER model to {} because the destination already exists: {error}. \
                 Remove the cached file and retry.",
                dst.display()
            ))
        }
        Err(error) => {
            let _ = std::fs::remove_file(&tmp);
            Err(format!("Failed to publish GLiNER model to {}: {error}", dst.display()))
        }
    }
}

/// Map an [`EntityCategory`] to the label string GLiNER expects.
fn category_to_label(category: &EntityCategory) -> String {
    match category {
        EntityCategory::Person => "person".to_string(),
        EntityCategory::Organization => "organization".to_string(),
        EntityCategory::Location => "location".to_string(),
        EntityCategory::Date => "date".to_string(),
        EntityCategory::Time => "time".to_string(),
        EntityCategory::Money => "money".to_string(),
        EntityCategory::Percent => "percent".to_string(),
        EntityCategory::Email => "email".to_string(),
        EntityCategory::Phone => "phone".to_string(),
        EntityCategory::Url => "url".to_string(),
        EntityCategory::Custom(label) => label.clone(),
    }
}

/// Reverse-map a GLiNER class string to an [`EntityCategory`].
fn label_to_category(class: &str) -> EntityCategory {
    match class {
        "person" => EntityCategory::Person,
        "organization" => EntityCategory::Organization,
        "location" => EntityCategory::Location,
        "date" => EntityCategory::Date,
        "time" => EntityCategory::Time,
        "money" => EntityCategory::Money,
        "percent" => EntityCategory::Percent,
        "email" => EntityCategory::Email,
        "phone" => EntityCategory::Phone,
        "url" => EntityCategory::Url,
        other => EntityCategory::Custom(other.to_string()),
    }
}

fn default_labels() -> Vec<String> {
    DEFAULT_LABELS.iter().map(|label| (*label).to_string()).collect()
}

fn checked_offset_to_u32(offset: usize, field: &str, text: &str, class: &str) -> Result<u32> {
    u32::try_from(offset).map_err(|error| {
        crate::XbergError::validation_with_source(
            format!(
                "GLiNER returned {field} offset {offset} for class '{class}' in span '{text}', \
                 which exceeds the u32 entity offset limit"
            ),
            error,
        )
    })
}

fn prepare_labels(categories: &[EntityCategory], custom_labels: &[String]) -> Vec<String> {
    if categories.is_empty() && custom_labels.is_empty() {
        return default_labels();
    }

    let mut seen = HashSet::new();
    let mut labels = Vec::new();
    for label in categories
        .iter()
        .map(category_to_label)
        .chain(custom_labels.iter().cloned())
    {
        let label = label.trim();
        if !label.is_empty() && seen.insert(label.to_string()) {
            labels.push(label.to_string());
        }
    }

    if labels.is_empty() { default_labels() } else { labels }
}

fn get_or_insert_arc<K, V, F>(cache: &RwLock<AHashMap<K, Arc<V>>>, key: K, build: F) -> Result<Arc<V>>
where
    K: Clone + Eq + Hash,
    F: FnOnce() -> Result<V>,
{
    {
        let cache = cache.read();
        if let Some(value) = cache.get(&key) {
            return Ok(Arc::clone(value));
        }
    }

    let mut cache = cache.write();
    if let Some(value) = cache.get(&key) {
        return Ok(Arc::clone(value));
    }

    let value = Arc::new(build()?);
    cache.insert(key, Arc::clone(&value));
    Ok(value)
}

enum GlinerEngine {
    V1(Gliner),
    V2(Gliner2),
}

impl GlinerEngine {
    fn inference(&self, input: TextInput) -> xberg_gliner::Result<xberg_gliner::SpanOutput> {
        match self {
            Self::V1(engine) => engine.inference(input),
            Self::V2(engine) => engine.inference(input),
        }
    }
}

/// `xberg-gliner` ONNX backend wrapper.
///
/// Holds an initialised GLiNER (v1 span-mode or v2 schema-prompt) model.
/// Inference is synchronous and internally serialized around the underlying
/// ONNX Runtime session.
pub struct GlineBackend {
    /// xberg GLiNER model alias or catalog id used to load this model.
    pub repo_id: String,
    /// Local path to the cached ONNX model file.
    pub model_path: PathBuf,
    /// Local path to the cached tokenizer file.
    pub tokenizer_path: PathBuf,
    model: Arc<GlinerEngine>,
}

impl GlineBackend {
    /// Build a backend for `model_name`, or the default model when `None`.
    ///
    /// Downloads the ONNX weights and tokenizer from `xberg-io/gliner-models`
    /// on first use. After this returns, inference is available without
    /// further network I/O.
    pub fn new(model_name: Option<&str>) -> Result<Self> {
        let thread_budget = crate::core::config::concurrency::resolve_thread_budget(None);
        Self::new_with_thread_budget(model_name, None, thread_budget)
    }

    /// Build a backend from a caller-supplied Hugging Face repo, bypassing the
    /// pinned catalog. See [`CustomGlinerSource`] for the checksum caveat.
    pub fn new_with_custom_source(source: &CustomGlinerSource) -> Result<Self> {
        let thread_budget = crate::core::config::concurrency::resolve_thread_budget(None);
        Self::new_with_thread_budget(None, Some(source), thread_budget)
    }

    fn new_with_thread_budget(
        model_name: Option<&str>,
        custom_source: Option<&CustomGlinerSource>,
        thread_budget: usize,
    ) -> Result<Self> {
        let files = match custom_source {
            Some(source) => ensure_custom_model(source, None)?,
            None => {
                let requested = requested_model_name(model_name)?;
                ensure_model(&requested, None)?
            }
        };
        let architecture = custom_source
            .map(|source| source.architecture)
            .unwrap_or_default();
        let engine = match architecture {
            crate::core::config::ner::GlinerArchitecture::Gliner1 => GlinerEngine::V1(
                Gliner::with_runtime(
                    Parameters::default(),
                    RuntimeConfig::default().with_intra_threads(thread_budget),
                    &files.tokenizer_path,
                    &files.model_path,
                )
                .map_err(|error| crate::XbergError::Plugin {
                    message: format!("Failed to initialise GLiNER model '{}': {error}", files.id),
                    plugin_name: "ner-gliner".to_string(),
                })?,
            ),
            crate::core::config::ner::GlinerArchitecture::Gliner2 => GlinerEngine::V2(
                Gliner2::with_runtime(
                    Parameters::default(),
                    RuntimeConfig::default().with_intra_threads(thread_budget),
                    &files.tokenizer_path,
                    &files.model_path,
                )
                .map_err(|error| crate::XbergError::Plugin {
                    message: format!("Failed to initialise GLiNER2 model '{}': {error}", files.id),
                    plugin_name: "ner-gliner".to_string(),
                })?,
            ),
        };
        Ok(Self {
            repo_id: files.id,
            model_path: files.model_path,
            tokenizer_path: files.tokenizer_path,
            model: Arc::new(engine),
        })
    }
}

pub(crate) fn get_or_init_backend(
    model_name: Option<&str>,
    custom_source: Option<&CustomGlinerSource>,
) -> Result<Arc<GlineBackend>> {
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(None);
    let key = backend_cache_key(model_name, custom_source, thread_budget)?;

    get_or_insert_arc(&BACKEND_CACHE, key, || {
        GlineBackend::new_with_thread_budget(model_name, custom_source, thread_budget)
    })
}

#[async_trait]
impl NerBackend for GlineBackend {
    async fn detect(&self, text: &str, categories: &[EntityCategory]) -> Result<Vec<Entity>> {
        let labels = prepare_labels(categories, &[]);
        self.detect_labels(text, labels).await
    }

    /// Native zero-shot multi-label inference: passes the union of `categories`
    /// and `custom_labels` to a single GLiNER inference call.
    async fn detect_with_custom(
        &self,
        text: &str,
        categories: &[EntityCategory],
        custom_labels: &[String],
    ) -> Result<Vec<Entity>> {
        let labels = prepare_labels(categories, custom_labels);
        self.detect_labels(text, labels).await
    }
}

impl GlineBackend {
    async fn detect_labels(&self, text: &str, labels: Vec<String>) -> Result<Vec<Entity>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let text = text.to_string();
        let backend = Arc::clone(&self.model);
        let model_path = self.model_path.clone();
        let tokenizer_path = self.tokenizer_path.clone();

        tokio::task::spawn_blocking(move || {
            let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            let input =
                TextInput::from_str(&[text.as_str()], &label_refs).map_err(|error| crate::XbergError::Plugin {
                    message: format!("Failed to build GLiNER input: {error}"),
                    plugin_name: "ner-gliner".to_string(),
                })?;
            let output = backend.inference(input).map_err(|error| crate::XbergError::Plugin {
                message: format!(
                    "GLiNER inference failed for model '{}' (tokenizer '{}'): {error}",
                    model_path.display(),
                    tokenizer_path.display()
                ),
                plugin_name: "ner-gliner".to_string(),
            })?;

            let entities = output
                .spans
                .into_iter()
                .next()
                .unwrap_or_default()
                .into_iter()
                .map(|span| {
                    let (start, end) = span.offsets();
                    let start = checked_offset_to_u32(start, "start", span.text(), span.class())?;
                    let end = checked_offset_to_u32(end, "end", span.text(), span.class())?;
                    Ok(Entity {
                        category: label_to_category(span.class()),
                        text: span.text().to_string(),
                        start,
                        end,
                        confidence: Some(span.probability()),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(entities)
        })
        .await
        .map_err(|error| crate::XbergError::Plugin {
            message: format!("GLiNER spawn_blocking task panicked: {error}"),
            plugin_name: "ner-gliner".to_string(),
        })?
    }
}

#[cfg(all(test, feature = "ner-onnx"))]
mod tests {
    use super::*;

    #[test]
    fn category_to_label_maps_all_canonical_variants() {
        assert_eq!(category_to_label(&EntityCategory::Person), "person");
        assert_eq!(category_to_label(&EntityCategory::Organization), "organization");
        assert_eq!(category_to_label(&EntityCategory::Location), "location");
        assert_eq!(category_to_label(&EntityCategory::Date), "date");
        assert_eq!(category_to_label(&EntityCategory::Time), "time");
        assert_eq!(category_to_label(&EntityCategory::Money), "money");
        assert_eq!(category_to_label(&EntityCategory::Percent), "percent");
        assert_eq!(category_to_label(&EntityCategory::Email), "email");
        assert_eq!(category_to_label(&EntityCategory::Phone), "phone");
        assert_eq!(category_to_label(&EntityCategory::Url), "url");
        assert_eq!(
            category_to_label(&EntityCategory::Custom("vessel".to_string())),
            "vessel"
        );
    }

    #[test]
    fn label_to_category_maps_all_canonical_labels() {
        assert_eq!(label_to_category("person"), EntityCategory::Person);
        assert_eq!(label_to_category("organization"), EntityCategory::Organization);
        assert_eq!(label_to_category("location"), EntityCategory::Location);
        assert_eq!(label_to_category("date"), EntityCategory::Date);
        assert_eq!(label_to_category("time"), EntityCategory::Time);
        assert_eq!(label_to_category("money"), EntityCategory::Money);
        assert_eq!(label_to_category("percent"), EntityCategory::Percent);
        assert_eq!(label_to_category("email"), EntityCategory::Email);
        assert_eq!(label_to_category("phone"), EntityCategory::Phone);
        assert_eq!(label_to_category("url"), EntityCategory::Url);
        assert_eq!(
            label_to_category("unknown_label"),
            EntityCategory::Custom("unknown_label".to_string())
        );
    }

    #[test]
    fn category_to_label_roundtrips() {
        let categories = [
            EntityCategory::Person,
            EntityCategory::Organization,
            EntityCategory::Location,
            EntityCategory::Date,
            EntityCategory::Time,
            EntityCategory::Money,
            EntityCategory::Percent,
            EntityCategory::Email,
            EntityCategory::Phone,
            EntityCategory::Url,
        ];
        for category in &categories {
            let label = category_to_label(category);
            assert_eq!(
                label_to_category(&label),
                *category,
                "roundtrip failed for {category:?}"
            );
        }
    }

    #[test]
    fn prepare_labels_uses_defaults_when_inputs_are_empty() {
        assert_eq!(
            prepare_labels(&[], &[]),
            vec![
                "person".to_string(),
                "organization".to_string(),
                "location".to_string(),
                "date".to_string(),
                "email".to_string(),
            ]
        );
    }

    #[test]
    fn prepare_labels_deduplicates_categories_and_custom_labels() {
        let custom_labels = vec![
            "person".to_string(),
            "treatment".to_string(),
            "organization".to_string(),
            "treatment".to_string(),
        ];
        let labels = prepare_labels(
            &[
                EntityCategory::Person,
                EntityCategory::Organization,
                EntityCategory::Person,
                EntityCategory::Custom("treatment".to_string()),
            ],
            &custom_labels,
        );

        assert_eq!(
            labels,
            vec![
                "person".to_string(),
                "organization".to_string(),
                "treatment".to_string(),
            ]
        );
    }

    #[test]
    fn prepare_labels_ignores_blank_custom_labels() {
        let labels = prepare_labels(
            &[EntityCategory::Custom("   ".to_string())],
            &["".to_string(), "  ".to_string()],
        );

        assert_eq!(labels, default_labels());
    }

    #[test]
    fn manifest_includes_gliner_models_and_checksums() {
        let entries = manifest();
        assert!(
            entries
                .iter()
                .any(|entry| entry.relative_path == "ner/gliner/checksums.sha256")
        );
        assert!(entries.iter().any(|entry| {
            entry.relative_path == "ner/gliner/gliner_medium-v2.5/span/fp32/model.onnx"
                && entry.source_url.contains(GLINER_MODELS_REPO)
        }));
        assert!(entries.iter().any(|entry| {
            entry.relative_path == "ner/gliner/gliner_medium-v2.5/span/fp32/tokenizer.json"
                && entry.source_url.contains(GLINER_MODELS_REPO)
        }));
    }

    #[test]
    fn load_gliner_checksums_reads_warmed_cache_without_network() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let checksums_dir = temp_dir.path().join("gliner");
        std::fs::create_dir_all(&checksums_dir).expect("checksums dir");
        std::fs::write(
            checksums_dir.join(CHECKSUMS_FILE),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  models/gliner_medium-v2.5/span/fp32/model.onnx\n",
        )
        .expect("write checksums");

        let checksums = load_gliner_checksums(temp_dir.path()).expect("checksums");
        assert_eq!(
            required_checksum(&checksums, "models/gliner_medium-v2.5/span/fp32/model.onnx").expect("checksum"),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
    }

    #[test]
    fn backend_cache_key_uses_canonical_model_and_runtime_config() {
        let default_key = backend_cache_key(None, None, 4).expect("default key");
        let alias_key = backend_cache_key(Some("balanced"), None, 4).expect("alias key");
        let id_key = backend_cache_key(Some("gliner_medium-v2.5"), None, 4).expect("id key");
        let different_runtime_key = backend_cache_key(Some("balanced"), None, 2).expect("runtime key");

        assert_eq!(default_key, alias_key);
        assert_eq!(alias_key, id_key);
        assert_ne!(alias_key, different_runtime_key);
    }

    #[test]
    fn backend_cache_key_rejects_empty_model_name_without_downloading() {
        assert!(backend_cache_key(Some("   "), None, 4).is_err());
    }

    #[test]
    fn known_models_are_unique_canonical_download_targets() {
        let mut names = std::collections::HashSet::new();
        let mut model_ids = std::collections::HashSet::new();

        for name in KNOWN_MODELS {
            assert!(names.insert(*name), "duplicate known model name: {name}");

            let definition = resolve_model(name).expect("known model resolves");
            assert_eq!(
                definition.id, *name,
                "known model '{name}' should be a canonical model id, not an alias"
            );
            assert!(
                model_ids.insert(definition.id),
                "duplicate canonical GLiNER model in warm list: {}",
                definition.id
            );
        }

        assert_eq!(model_ids.len(), GLINER_MODELS.len());
    }

    #[test]
    fn backend_cache_helper_reuses_arc_without_rebuilding() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cache = RwLock::new(AHashMap::default());
        let key = backend_cache_key(Some("balanced"), None, 4).expect("key");
        let build_count = AtomicUsize::new(0);

        let first = get_or_insert_arc(&cache, key.clone(), || {
            build_count.fetch_add(1, Ordering::Relaxed);
            Ok(7usize)
        })
        .expect("first build");
        let second = get_or_insert_arc(&cache, key, || {
            build_count.fetch_add(1, Ordering::Relaxed);
            Ok(9usize)
        })
        .expect("cache hit");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(*second, 7);
        assert_eq!(build_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn resolves_supported_model_aliases() {
        assert_eq!(resolve_model("fast").expect("fast").id, "gliner_small-v2.5");
        assert_eq!(resolve_model("balanced").expect("balanced").id, "gliner_medium-v2.5");
        assert_eq!(
            resolve_model("gliner_large-v2.5").expect("id").upstream_repo,
            "gliner-community/gliner_large-v2.5"
        );
        assert!(resolve_model("unknown/gliner-model").is_err());
    }

    #[test]
    fn checked_offset_conversion_rejects_u32_overflow() {
        let offset = u32::MAX as usize + 1;
        let err = checked_offset_to_u32(offset, "start", "Alice", "person").expect_err("overflow must fail");

        match err {
            crate::XbergError::Validation { message, source } => {
                assert!(message.contains("GLiNER returned start offset"));
                assert!(message.contains("exceeds the u32 entity offset limit"));
                assert!(source.is_some());
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn parses_gliner_checksum_file() {
        let checksums = parse_checksums(
            r#"
            # generated checksums
            AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA  ./models/gliner_small-v2.5/span/fp32/model.onnx
            bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  models/gliner_small-v2.5/span/fp32/tokenizer.json
            "#,
        )
        .expect("checksums");

        assert_eq!(
            required_checksum(&checksums, "models/gliner_small-v2.5/span/fp32/model.onnx").expect("model"),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(
            required_checksum(&checksums, "models/gliner_small-v2.5/span/fp32/tokenizer.json").expect("tokenizer"),
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        );
        assert!(required_checksum(&checksums, "missing/model.onnx").is_err());
    }

    #[test]
    fn rejects_invalid_gliner_checksum_file() {
        assert!(parse_checksums("not-a-sha256 models/model.onnx").is_err());
        assert!(parse_checksums("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_err());
    }

    /// Smoke test — downloads the real model and runs one inference.
    /// Excluded from normal CI; run with:
    ///   cargo test -p xberg --features ner-onnx,ner --lib ner::gline -- --ignored
    #[ignore]
    #[tokio::test]
    async fn smoke_test_real_inference() {
        let backend = GlineBackend::new(None).expect("GlineBackend::new failed");
        let entities = backend
            .detect("Elon Musk founded SpaceX in Hawthorne, California.", &[])
            .await
            .expect("detect failed");
        assert!(!entities.is_empty(), "expected at least one entity");
        let texts: Vec<&str> = entities.iter().map(|entity| entity.text.as_str()).collect();
        assert!(
            texts.contains(&"Elon Musk") || texts.contains(&"SpaceX"),
            "expected at least one known entity, got: {texts:?}"
        );
    }

    /// Smoke test — downloads a real GLiNER2 ONNX export and runs one inference.
    /// `lion-ai/gliner2-base-v1-onnx` is the only publicly available monolithic
    /// single-file GLiNER2 ONNX export found (most GLiNER2 model cards ship
    /// safetensors only). Excluded from normal CI; run with:
    ///   cargo test -p xberg --features ner-onnx,ner --lib ner::gline -- --ignored gliner2
    #[ignore]
    #[tokio::test]
    async fn smoke_test_gliner2_real_inference() {
        let source = CustomGlinerSource {
            repo: "lion-ai/gliner2-base-v1-onnx".to_string(),
            model_file: "model.onnx".to_string(),
            tokenizer_file: "tokenizer.json".to_string(),
            architecture: crate::core::config::ner::GlinerArchitecture::Gliner2,
        };
        let backend = GlineBackend::new_with_custom_source(&source).expect("GlineBackend::new_with_custom_source failed");
        let entities = backend
            .detect(
                "Steve Jobs founded Apple Inc. in Cupertino, California on April 1, 1976.",
                &[],
            )
            .await
            .expect("detect failed");
        assert!(!entities.is_empty(), "expected at least one entity");
        let texts: Vec<&str> = entities.iter().map(|entity| entity.text.as_str()).collect();
        assert!(
            texts.iter().any(|text| text.eq_ignore_ascii_case("steve jobs") || text.eq_ignore_ascii_case("apple inc")),
            "expected at least one known entity, got: {texts:?}"
        );
    }
}
