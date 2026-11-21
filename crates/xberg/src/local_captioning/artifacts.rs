use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::control::WorkerControl;
use super::policy::invalid;
use super::types::{LocalCaptioningFile, LocalCaptioningFormat};
use super::{LocalCaptioningConfig, LocalCaptioningModel};

/// Per-graph ceiling. The largest pinned fp32 decoder is under 1.5 GiB; int8 graphs are far
/// smaller. This bounds a corrupted or substituted artifact, not the runtime's memory.
pub(super) const MAX_GRAPH_BYTES: usize = 2 * 1024 * 1024 * 1024;
/// Tokenizer, processor and generation metadata are small; the limit rejects a swapped graph.
pub(super) const MAX_COMPANION_BYTES: usize = 64 * 1024 * 1024;
/// Ceiling over every resolved artifact in one preparation.
pub(super) const MAX_TOTAL_BYTES: usize = 4 * 1024 * 1024 * 1024;

const REQUIRED_COMPANIONS: [&str; 6] = [
    "tokenizer.json",
    "tokenizer_config.json",
    "preprocessor_config.json",
    "processor_config.json",
    "config.json",
    "generation_config.json",
];
const FLORENCE_ONLY: [&str; 1] = ["added_tokens.json"];
const OPTIONAL_COMPANIONS: [&str; 1] = ["special_tokens_map.json"];

pub(super) struct VerifiedArtifacts {
    pub(super) vision: Vec<u8>,
    pub(super) embed: Vec<u8>,
    pub(super) decoder: Vec<u8>,
    /// Companion bytes keyed by file name (`config.json`, `tokenizer.json`, ...).
    pub(super) companions: BTreeMap<String, Vec<u8>>,
}

struct Budget(usize);

impl Budget {
    fn charge(&mut self, length: usize) -> crate::Result<()> {
        self.0 = self
            .0
            .checked_sub(length)
            .ok_or_else(|| invalid("Local captioning artifacts exceed the total byte limit"))?;
        Ok(())
    }
}

fn basename(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

fn companion_required(format: LocalCaptioningFormat, name: &str) -> bool {
    REQUIRED_COMPANIONS.contains(&name) || (format == LocalCaptioningFormat::Florence2 && FLORENCE_ONLY.contains(&name))
}

fn is_companion(format: LocalCaptioningFormat, name: &str) -> bool {
    companion_required(format, name) || OPTIONAL_COMPANIONS.contains(&name)
}

fn cache_root(config: &LocalCaptioningConfig) -> PathBuf {
    config
        .cache_directory
        .as_deref()
        .map(Path::new)
        .map_or_else(hf_hub::resolve_cache_dir, Path::to_path_buf)
}

fn verify(file: &LocalCaptioningFile, bytes: &[u8]) -> crate::Result<()> {
    if bytes.len() as u64 != file.size_bytes {
        return Err(invalid(format!(
            "Local captioning size mismatch for {}: expected {} bytes, got {}",
            file.path,
            file.size_bytes,
            bytes.len()
        )));
    }
    let actual = hex::encode(Sha256::digest(bytes));
    if actual != file.sha256 {
        return Err(invalid(format!(
            "Local captioning checksum mismatch for {}: expected {}, got {actual}",
            file.path, file.sha256
        )));
    }
    Ok(())
}

fn cached_path(config: &LocalCaptioningConfig, file: &LocalCaptioningFile) -> crate::Result<PathBuf> {
    let client = crate::model_download::hf_client(config.cache_directory.as_deref().map(Path::new)).map_err(invalid)?;
    let model = &config.model;
    let path = crate::model_download::hf_cached_revision_with_client(
        &client,
        &model.repository,
        &file.path,
        Some(&model.revision),
    )
    .map_err(invalid)?
    .ok_or_else(|| {
        invalid(format!(
            "Local captioning artifact is missing from the pre-staged cache: {}@{} {}",
            model.repository, model.revision, file.path
        ))
    })?;
    let root = cache_root(config);
    let root = root
        .canonicalize()
        .map_err(|error| invalid(format!("Local captioning cache root is unavailable: {error}")))?;
    let path = path
        .canonicalize()
        .map_err(|error| invalid(format!("Local captioning cached artifact path: {error}")))?;
    if !path.starts_with(&root) {
        return Err(invalid("Local captioning artifact escapes the configured cache root"));
    }
    Ok(path)
}

fn resolve(config: &LocalCaptioningConfig, file: &LocalCaptioningFile, budget: &mut Budget) -> crate::Result<Vec<u8>> {
    let limit = if basename(&file.path).ends_with(".onnx") {
        MAX_GRAPH_BYTES
    } else {
        MAX_COMPANION_BYTES
    };
    let path = cached_path(config, file)?;
    let metadata = std::fs::metadata(&path)
        .map_err(|error| invalid(format!("Local captioning artifact {}: {error}", file.path)))?;
    if !metadata.is_file() {
        return Err(invalid(format!(
            "Local captioning artifact is not a regular file: {}",
            file.path
        )));
    }
    let length = usize::try_from(metadata.len())
        .ok()
        .filter(|length| *length != 0 && *length <= limit)
        .ok_or_else(|| {
            invalid(format!(
                "Local captioning artifact is empty or too large: {}",
                file.path
            ))
        })?;
    // Charge the declared length before allocating or reading the bytes.
    budget.charge(length)?;
    let bytes =
        std::fs::read(&path).map_err(|error| invalid(format!("Local captioning artifact {}: {error}", file.path)))?;
    if bytes.len() != length {
        return Err(invalid(format!(
            "Local captioning artifact changed while being read: {}",
            file.path
        )));
    }
    verify(file, &bytes)?;
    Ok(bytes)
}

fn find<'a>(model: &'a LocalCaptioningModel, path: &str) -> crate::Result<&'a LocalCaptioningFile> {
    model
        .files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| invalid(format!("Local captioning artifact is not declared: {path}")))
}

pub(super) fn read_artifacts(
    config: &LocalCaptioningConfig,
    control: &WorkerControl,
) -> crate::Result<VerifiedArtifacts> {
    let model = &config.model;
    let mut budget = Budget(MAX_TOTAL_BYTES);
    let mut read = |path: &str| -> crate::Result<Vec<u8>> {
        control.check()?;
        let file = find(model, path)?;
        let bytes = resolve(config, file, &mut budget)?;
        control.check()?;
        Ok(bytes)
    };
    let vision = read(&model.vision_encoder_file)?;
    let embed = read(&model.embed_tokens_file)?;
    let decoder = read(&model.decoder_file)?;
    let mut companions = BTreeMap::new();
    for file in &model.files {
        let name = basename(&file.path);
        if !is_companion(model.format, name) {
            continue;
        }
        let bytes = resolve(config, file, &mut budget)?;
        companions.insert(name.to_owned(), bytes);
    }
    for name in REQUIRED_COMPANIONS {
        if !companions.contains_key(name) {
            return Err(invalid(format!("Local captioning artifact set resolved no {name}")));
        }
    }
    if model.format == LocalCaptioningFormat::Florence2 && !companions.contains_key("added_tokens.json") {
        return Err(invalid(
            "Florence-2 captioning artifact set resolved no added_tokens.json",
        ));
    }
    Ok(VerifiedArtifacts {
        vision,
        embed,
        decoder,
        companions,
    })
}

/// Resolves every selected artifact through the shared download/cache path, verifying the pinned
/// SHA-256 and size. A cache hit performs no network request; a miss downloads only when offline
/// mode is off. The preparation step that follows is cache-only.
pub(super) fn stage(config: &LocalCaptioningConfig, control: &WorkerControl) -> crate::Result<()> {
    let model = &config.model;
    for file in &model.files {
        control.check()?;
        let path = crate::model_download::hf_resolve_file_with_progress(
            &model.repository,
            &file.path,
            Some(&model.revision),
            config.cache_directory.as_deref().map(Path::new),
            Some(&file.sha256),
            crate::core::config::DownloadProgress::SILENT,
        )
        .map_err(invalid)?;
        let metadata = std::fs::metadata(&path).map_err(|error| invalid(format!("{}: {error}", file.path)))?;
        if metadata.len() != file.size_bytes {
            return Err(invalid(format!(
                "Local captioning staged size mismatch for {}: expected {} bytes, got {}",
                file.path,
                file.size_bytes,
                metadata.len()
            )));
        }
    }
    Ok(())
}
