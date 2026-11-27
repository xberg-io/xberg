//! Cache command - Manage cache operations
//!
//! This module provides commands for cache management including statistics,
//! clearing, manifest generation, and model warming.

use anyhow::{Context, Result};
use serde_json::json;
#[cfg(any(
    feature = "embeddings",
    feature = "layout-detection",
    feature = "paddle-ocr",
    feature = "tree-sitter",
    feature = "ner-onnx",
    feature = "classification-onnx",
    feature = "captioning-onnx"
))]
use std::path::Path;
use std::path::PathBuf;
use xberg::cache;

use crate::{WireFormat, style};

#[cfg(any(
    feature = "embeddings",
    feature = "layout-detection",
    feature = "paddle-ocr",
    feature = "tree-sitter",
    feature = "ner-onnx",
    feature = "classification-onnx",
    feature = "captioning-onnx"
))]
use super::warm_report::print_warm_summary;

#[cfg(all(feature = "paddle-ocr", feature = "sceptre-ocr"))]
pub fn parse_nonempty_cache_dir(value: &str) -> std::result::Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty() {
        return Err("classic OCR cache directory must not be empty".to_string());
    }
    Ok(path)
}

/// Seed the complete bounded PaddleOCR and Sceptre model catalog.
#[cfg(all(feature = "paddle-ocr", feature = "sceptre-ocr"))]
#[expect(
    clippy::print_stdout,
    reason = "classic OCR seed manifest is the command's stdout result output"
)]
pub fn seed_classic_ocr_command(cache_dir: PathBuf, format: WireFormat) -> Result<()> {
    let manifest = xberg::classic_ocr_cache::seed_classic_ocr_cache(cache_dir)
        .context("Failed to seed the classic OCR model cache")?;
    match format {
        WireFormat::Text => {
            println!("{}", style::header("Classic OCR model cache seeded"));
            println!("{} {}", style::label("Directory:"), manifest.cache_dir.display());
            println!("{} {}", style::label("Catalog IDs:"), manifest.catalog_count);
            println!("{} {}", style::label("Artifacts:"), manifest.artifact_count);
            println!("{} {}", style::label("Bytes:"), manifest.total_size_bytes);
            for model in &manifest.models {
                println!("{} {}@{}:{}", model.backend, model.repo, model.revision, model.file);
            }
        }
        WireFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&manifest).context("Failed to serialize classic OCR seed manifest to JSON")?
        ),
        WireFormat::Toon => println!(
            "{}",
            serde_toon::to_string(&manifest).context("Failed to serialize classic OCR seed manifest to TOON")?
        ),
    }
    Ok(())
}

#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "ner-onnx",
    feature = "formula-recognition"
))]
#[derive(Debug, Clone, serde::Serialize)]
struct CacheManifestEntry {
    relative_path: String,
    sha256: String,
    size_bytes: u64,
    source_url: String,
}

#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "ner-onnx",
    feature = "formula-recognition"
))]
impl CacheManifestEntry {
    fn new(relative_path: String, sha256: String, size_bytes: u64, source_url: String) -> Self {
        Self {
            relative_path,
            sha256,
            size_bytes,
            source_url,
        }
    }
}

/// Execute cache stats command
#[expect(
    clippy::print_stdout,
    reason = "cache statistics are the command's stdout result output"
)]
pub fn stats_command(cache_dir: Option<PathBuf>, format: WireFormat) -> Result<()> {
    let default_cache_dir = std::env::current_dir()
        .context("Failed to get current directory")?
        .join(".xberg");

    let cache_path = cache_dir.unwrap_or(default_cache_dir);
    let cache_dir_str = cache_path.to_string_lossy();

    let stats = cache::get_cache_metadata(&cache_dir_str).with_context(|| {
        format!(
            "Failed to get cache statistics from directory '{}'. Ensure the directory exists and is readable.",
            cache_dir_str
        )
    })?;

    match format {
        WireFormat::Text => {
            println!("{}", style::header("Cache Statistics"));
            println!("{}", style::dim("================"));
            println!("{} {}", style::label("Directory:"), style::success(&cache_dir_str));
            println!("{} {}", style::label("Total files:"), stats.total_files);
            println!("{} {:.2} MB", style::label("Total size:"), stats.total_size_mb);
            println!(
                "{} {:.2} MB",
                style::label("Available space:"),
                stats.available_space_mb
            );
            println!(
                "{} {:.2} days",
                style::label("Oldest file age:"),
                stats.oldest_file_age_days
            );
            println!(
                "{} {:.2} days",
                style::label("Newest file age:"),
                stats.newest_file_age_days
            );
        }
        WireFormat::Json => {
            let output = json!({
                "directory": cache_dir_str,
                "total_files": stats.total_files,
                "total_size_mb": stats.total_size_mb,
                "available_space_mb": stats.available_space_mb,
                "oldest_file_age_days": stats.oldest_file_age_days,
                "newest_file_age_days": stats.newest_file_age_days,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).context("Failed to serialize cache statistics to JSON")?
            );
        }
        WireFormat::Toon => {
            let output = json!({
                "directory": cache_dir_str,
                "total_files": stats.total_files,
                "total_size_mb": stats.total_size_mb,
                "available_space_mb": stats.available_space_mb,
                "oldest_file_age_days": stats.oldest_file_age_days,
                "newest_file_age_days": stats.newest_file_age_days,
            });
            println!(
                "{}",
                serde_toon::to_string(&output).context("Failed to serialize cache statistics to TOON")?
            );
        }
    }

    Ok(())
}

/// Clear the Xberg-managed cache. Shared Hugging Face cache files are excluded.
#[expect(
    clippy::print_stdout,
    reason = "cache clear summary is the command's stdout result output"
)]
pub fn clear_command(cache_dir: Option<PathBuf>, format: WireFormat) -> Result<()> {
    let default_cache_dir = std::env::current_dir()
        .context("Failed to get current directory")?
        .join(".xberg");

    let cache_path = cache_dir.unwrap_or(default_cache_dir);
    let cache_dir_str = cache_path.to_string_lossy();

    let (removed_files, freed_mb) = cache::clear_cache_directory(&cache_dir_str).with_context(|| {
        format!(
            "Failed to clear cache directory '{}'. Ensure you have write permissions.",
            cache_dir_str
        )
    })?;

    match format {
        WireFormat::Text => {
            println!("{}", style::success("Xberg-managed cache cleared successfully"));
            println!("Shared Hugging Face Hub cache files were not removed.");
            println!("{} {}", style::label("Directory:"), style::success(&cache_dir_str));
            println!("{} {}", style::label("Removed files:"), removed_files);
            println!("{} {:.2} MB", style::label("Freed space:"), freed_mb);
        }
        WireFormat::Json => {
            let output = json!({
                "directory": cache_dir_str,
                "removed_files": removed_files,
                "freed_mb": freed_mb,
                "hugging_face_cache_cleared": false,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).context("Failed to serialize cache clear results to JSON")?
            );
        }
        WireFormat::Toon => {
            let output = json!({
                "directory": cache_dir_str,
                "removed_files": removed_files,
                "freed_mb": freed_mb,
                "hugging_face_cache_cleared": false,
            });
            println!(
                "{}",
                serde_toon::to_string(&output).context("Failed to serialize cache clear results to TOON")?
            );
        }
    }

    Ok(())
}

/// Execute cache manifest command - outputs expected model files with checksums.
pub fn manifest_command(format: WireFormat) -> Result<()> {
    // below is `#[cfg]`-stripped and `entries: Vec<_>` has no anchor for
    #[cfg(not(any(
        feature = "paddle-ocr",
        feature = "layout-detection",
        feature = "ner-onnx",
        feature = "formula-recognition"
    )))]
    {
        let _ = format;
        anyhow::bail!(
            "manifest command unavailable: build xberg-cli with at least one of \
             --features \"paddle-ocr\", \"layout-detection\", or \"ner-onnx\""
        );
    }

    #[cfg(any(
        feature = "paddle-ocr",
        feature = "layout-detection",
        feature = "ner-onnx",
        feature = "formula-recognition"
    ))]
    {
        manifest_command_inner(format)
    }
}

/// Gather the manifest entries for every model source compiled into this binary.
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "ner-onnx",
    feature = "formula-recognition"
))]
fn collect_manifest_entries() -> Vec<CacheManifestEntry> {
    let mut entries: Vec<CacheManifestEntry> = Vec::new();

    #[cfg(feature = "paddle-ocr")]
    {
        entries.extend(xberg::paddle_ocr::ModelManager::manifest().into_iter().map(|entry| {
            CacheManifestEntry::new(entry.relative_path, entry.sha256, entry.size_bytes, entry.source_url)
        }));
    }

    #[cfg(feature = "layout-detection")]
    {
        entries.extend(xberg::layout::LayoutModelManager::manifest().into_iter().map(|entry| {
            CacheManifestEntry::new(entry.relative_path, entry.sha256, entry.size_bytes, entry.source_url)
        }));
    }

    #[cfg(feature = "formula-recognition")]
    {
        entries.extend(xberg::formula_recognition::manifest().into_iter().map(|entry| {
            CacheManifestEntry::new(entry.relative_path, entry.sha256, entry.size_bytes, entry.source_url)
        }));
    }

    #[cfg(feature = "paddle-ocr")]
    {
        entries.extend(xberg::ocr::TessdataManager::manifest().into_iter().map(|entry| {
            CacheManifestEntry::new(entry.relative_path, entry.sha256, entry.size_bytes, entry.source_url)
        }));
    }

    #[cfg(feature = "ner-onnx")]
    {
        entries.extend(xberg::text::ner::manifest().into_iter().map(|entry| {
            CacheManifestEntry::new(entry.relative_path, entry.sha256, entry.size_bytes, entry.source_url)
        }));
    }

    entries
}

/// Print the model manifest as a human-readable table.
#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "ner-onnx",
    feature = "formula-recognition"
))]
#[expect(
    clippy::print_stdout,
    reason = "model manifest is the command's stdout result output"
)]
fn print_manifest_text(entries: &[CacheManifestEntry], version: &str, total_size_bytes: u64) {
    println!(
        "{} {}",
        style::header("Model Manifest"),
        style::dim(&format!("(xberg {version})"))
    );
    println!("{}", style::dim("===================================="));
    println!(
        "{:<50} {:>12} {}",
        style::label("PATH"),
        style::label("SIZE"),
        style::label("SHA256")
    );
    println!("{}", style::dim(&format!("{:<50} {:>12} ------", "----", "----")));
    for entry in entries {
        let size_str = if entry.size_bytes > 0 {
            format!("{:.1} MB", entry.size_bytes as f64 / 1_048_576.0)
        } else {
            "unknown".to_string()
        };
        let sha_display = if entry.sha256.len() >= 12 {
            &entry.sha256[..12]
        } else if entry.sha256.is_empty() {
            "-"
        } else {
            &entry.sha256
        };
        println!(
            "{:<50} {:>12} {}",
            entry.relative_path,
            size_str,
            style::dim(sha_display)
        );
    }
    println!();
    println!(
        "{} {} files, {:.1} MB",
        style::label("Total:"),
        entries.len(),
        total_size_bytes as f64 / 1_048_576.0
    );
}

#[cfg(any(
    feature = "paddle-ocr",
    feature = "layout-detection",
    feature = "ner-onnx",
    feature = "formula-recognition"
))]
#[expect(
    clippy::print_stdout,
    reason = "model manifest is the command's stdout result output"
)]
fn manifest_command_inner(format: WireFormat) -> Result<()> {
    let entries = collect_manifest_entries();
    let total_size_bytes: u64 = entries.iter().map(|e| e.size_bytes).sum();
    let version = env!("CARGO_PKG_VERSION");

    match format {
        WireFormat::Text => print_manifest_text(&entries, version, total_size_bytes),
        WireFormat::Json => {
            let output = json!({
                "xberg_version": version,
                "total_size_bytes": total_size_bytes,
                "model_count": entries.len(),
                "models": entries,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).context("Failed to serialize manifest to JSON")?
            );
        }
        WireFormat::Toon => {
            let output = json!({
                "xberg_version": version,
                "total_size_bytes": total_size_bytes,
                "model_count": entries.len(),
                "models": entries,
            });
            println!(
                "{}",
                serde_toon::to_string(&output).context("Failed to serialize manifest to TOON")?
            );
        }
    }

    Ok(())
}

/// Feature-gated options for [`warm_command`], bundled so the command function itself
/// stays within the CLI's parameter-count limit. Each field only exists when its owning
/// feature is enabled, mirroring the `warm` subcommand's own `#[cfg]`-gated CLI flags. ~keep
#[cfg(any(
    feature = "embeddings",
    feature = "layout-detection",
    feature = "paddle-ocr",
    feature = "tree-sitter",
    feature = "ner-onnx",
    feature = "classification-onnx",
    feature = "captioning-onnx"
))]
#[derive(Debug, Default)]
pub struct WarmOptions {
    #[cfg(feature = "embeddings")]
    pub all_embeddings: bool,
    #[cfg(feature = "embeddings")]
    pub embedding_model: Option<String>,
    #[cfg(feature = "layout-detection")]
    pub all_table_models: bool,
    #[cfg(feature = "layout-detection")]
    pub layout_model: Option<String>,
    #[cfg(feature = "tree-sitter")]
    pub all_grammars: bool,
    #[cfg(feature = "tree-sitter")]
    pub grammar_groups: Option<Vec<String>>,
    #[cfg(feature = "tree-sitter")]
    pub grammars: Option<Vec<String>>,
    #[cfg(feature = "ner-onnx")]
    pub ner: bool,
    #[cfg(feature = "ner-onnx")]
    pub ner_model: Option<String>,
    #[cfg(feature = "ner-onnx")]
    pub all_ner_models: bool,
    #[cfg(any(feature = "captioning-onnx", feature = "classification-onnx"))]
    pub onnx: super::onnx_candidates::StageSelection,
    /// Tesseract model sets to warm; empty means the default set.
    #[cfg(any(feature = "ocr", feature = "paddle-ocr"))]
    pub tessdata_models: Vec<xberg::TessdataModel>,
    /// Tesseract languages to warm; empty means every pinned language.
    #[cfg(any(feature = "ocr", feature = "paddle-ocr"))]
    pub tessdata_languages: Vec<String>,
}

/// Resolve the Hugging Face cache label shown in the warm summary: `Some` only when at
/// least one requested model source actually reads the HF cache (paddle-ocr always does;
/// embeddings/ner-onnx depend on what was requested).
#[cfg(any(
    feature = "embeddings",
    feature = "ner-onnx",
    feature = "paddle-ocr",
    feature = "classification-onnx",
    feature = "captioning-onnx"
))]
fn resolve_hf_cache_label(
    hf_cache_dir: &Option<PathBuf>,
    #[cfg_attr(not(any(feature = "embeddings", feature = "ner-onnx")), allow(unused_variables))] options: &WarmOptions,
) -> Option<String> {
    let uses_hf_cache = cfg!(feature = "paddle-ocr")
        || {
            #[cfg(feature = "embeddings")]
            {
                options.all_embeddings || options.embedding_model.is_some()
            }
            #[cfg(not(feature = "embeddings"))]
            {
                false
            }
        }
        || {
            #[cfg(feature = "ner-onnx")]
            {
                options.ner || options.ner_model.is_some() || options.all_ner_models
            }
            #[cfg(not(feature = "ner-onnx"))]
            {
                false
            }
        }
        || {
            #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
            {
                !options.onnx.is_empty()
            }
            #[cfg(not(any(feature = "classification-onnx", feature = "captioning-onnx")))]
            {
                false
            }
        };
    uses_hf_cache.then(|| {
        hf_cache_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "HF_HUB_CACHE/HF_HOME/platform default".to_string())
    })
}

/// Download (or confirm already cached) the PaddleOCR v2 model set.
#[cfg(feature = "paddle-ocr")]
fn warm_paddle_ocr_models(hf_cache_dir: Option<PathBuf>, downloaded: &mut Vec<String>) -> Result<()> {
    let manager = hf_cache_dir
        .map(xberg::paddle_ocr::ModelManager::new)
        .unwrap_or_default();

    manager
        .ensure_all_models()
        .context("Failed to download PaddleOCR v2 models")?;
    downloaded.push("paddle-ocr v2 (server+mobile det, cls, doc_ori, unified+per-script rec)".to_string());
    Ok(())
}

/// Download (or confirm already cached) the formula-recognition model set.
#[cfg(feature = "formula-recognition")]
fn warm_formula_recognition(
    cache_base: &Path,
    downloaded: &mut Vec<String>,
    already_cached: &mut Vec<String>,
) -> Result<()> {
    let formula_dir = cache_base.join("formula-recognition");
    if xberg::formula_recognition::models_cached_in(Some(&formula_dir)) {
        already_cached.push("formula-recognition (latex_ocr)".to_string());
    } else {
        xberg::formula_recognition::ensure_models_in(Some(&formula_dir))
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to download formula recognition models")?;
        downloaded.push("formula-recognition (latex_ocr)".to_string());
    }
    Ok(())
}

#[cfg(feature = "layout-detection")]
fn parse_layout_model(layout_model: Option<&str>) -> Result<xberg::LayoutDetectionModel> {
    match layout_model {
        Some(name) => name
            .parse()
            .map_err(|error: String| anyhow::anyhow!("Invalid --layout-model: {error}")),
        None => Ok(xberg::LayoutDetectionModel::default()),
    }
}

/// Whether the selected layout model has weights beyond the always-warmed RT-DETR and TATR.
///
/// This is independent of `--all-table-models`: that flag only fetches the full set when RT-DETR
/// and TATR are not yet cached, so it cannot be relied on to include the selected model.
#[cfg(feature = "layout-detection")]
fn selected_model_needs_download(selected: xberg::LayoutDetectionModel) -> bool {
    selected == xberg::LayoutDetectionModel::PpDocLayoutV3
}

/// Download (or confirm already cached) the layout-detection model set.
#[cfg(feature = "layout-detection")]
fn warm_layout_detection(
    cache_base: &Path,
    all_table_models: bool,
    layout_model: Option<&str>,
    downloaded: &mut Vec<String>,
    already_cached: &mut Vec<String>,
) -> Result<()> {
    let layout_dir = cache_base.join("layout");
    let manager = xberg::layout::LayoutModelManager::new(Some(layout_dir));
    let selected = parse_layout_model(layout_model)?;

    if all_table_models {
        let was_cached = manager.is_rtdetr_cached() && manager.is_tatr_cached();
        if was_cached {
            already_cached.push("layout (rtdetr, tatr, slanet variants)".to_string());
        } else {
            manager
                .ensure_all_models()
                .context("Failed to download layout models")?;
            downloaded.push("layout (rtdetr, tatr, slanet variants)".to_string());
        }
    } else {
        let was_cached = manager.is_rtdetr_cached() && manager.is_tatr_cached();
        if was_cached {
            already_cached.push("layout (rtdetr, tatr)".to_string());
        } else {
            manager
                .ensure_default_models()
                .context("Failed to download layout models")?;
            downloaded.push("layout (rtdetr, tatr)".to_string());
        }
    }
    if selected_model_needs_download(selected) {
        if manager.is_pp_doclayout_v3_cached() {
            already_cached.push("layout (pp_doc_layout_v3)".to_string());
        } else {
            manager
                .ensure_pp_doclayout_v3_model()
                .context("Failed to download the PP-DocLayout-V3 model")?;
            downloaded.push("layout (pp_doc_layout_v3)".to_string());
        }
    }
    Ok(())
}

/// Download (or confirm already cached) the Tesseract language data files of each requested
/// model set into `<cache>/tessdata/<model>`, the directory OCR resolves that set from.
///
/// Needs only native Tesseract OCR (`ocr`, which `paddle-ocr` implies), in a build that has the
/// `warm` command at all.
#[cfg(all(
    any(feature = "ocr", feature = "paddle-ocr"),
    any(
        feature = "embeddings",
        feature = "layout-detection",
        feature = "paddle-ocr",
        feature = "tree-sitter",
        feature = "ner-onnx",
        feature = "classification-onnx",
        feature = "captioning-onnx"
    )
))]
fn warm_tessdata(
    cache_base: &Path,
    options: &WarmOptions,
    downloaded: &mut Vec<String>,
    already_cached: &mut Vec<String>,
) -> Result<()> {
    let (models, languages) = (&options.tessdata_models, &options.tessdata_languages);
    let default_models = [xberg::TessdataModel::default()];
    let models = if models.is_empty() {
        &default_models[..]
    } else {
        models.as_slice()
    };
    for &model in models {
        let manager =
            xberg::ocr::TessdataManager::for_model(Some(cache_base.join("tessdata").join(model.as_str())), model);
        let newly_downloaded = if languages.is_empty() {
            manager.ensure_all_languages()
        } else {
            manager.ensure_languages(&languages.iter().map(String::as_str).collect::<Vec<_>>())
        }
        .with_context(|| format!("Failed to download tessdata_{model} files"))?;

        let scope = if languages.is_empty() {
            "all languages".to_string()
        } else {
            languages.join(", ")
        };
        if newly_downloaded > 0 {
            downloaded.push(format!("tessdata_{model} ({newly_downloaded} languages)"));
        } else {
            already_cached.push(format!("tessdata_{model} ({scope})"));
        }
    }
    Ok(())
}

/// Download the requested embedding model preset(s).
#[cfg(feature = "embeddings")]
fn warm_embeddings(
    all_embeddings: bool,
    embedding_model: Option<String>,
    hf_cache_dir: Option<PathBuf>,
    downloaded: &mut Vec<String>,
) -> Result<()> {
    let presets_to_warm: Vec<xberg::EmbeddingPreset> = if all_embeddings {
        xberg::list_embedding_presets()
            .into_iter()
            .filter_map(|name| xberg::get_embedding_preset(&name))
            .collect()
    } else if let Some(ref name) = embedding_model {
        match xberg::get_embedding_preset(name) {
            Some(preset) => vec![preset],
            None => {
                let available = xberg::list_embedding_presets();
                anyhow::bail!(
                    "Unknown embedding preset '{}'. Available: {}",
                    name,
                    available.join(", ")
                );
            }
        }
    } else {
        vec![]
    };

    for preset in &presets_to_warm {
        let label = format!("embedding ({})", preset.name);
        xberg::embeddings::warm_model(
            &xberg::core::config::EmbeddingModelType::Preset {
                name: preset.name.clone(),
            },
            hf_cache_dir.clone(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to download embedding model '{}': {}", preset.name, e))?;
        downloaded.push(label);
    }
    Ok(())
}

/// Download the requested tree-sitter grammar(s).
#[cfg(feature = "tree-sitter")]
fn warm_tree_sitter_grammars(
    all_grammars: bool,
    grammar_groups: Option<Vec<String>>,
    grammars: Option<Vec<String>>,
    downloaded: &mut Vec<String>,
    already_cached: &mut Vec<String>,
) -> Result<()> {
    if all_grammars {
        let count = tree_sitter_language_pack::download_all().context("Failed to download all tree-sitter grammars")?;
        if count > 0 {
            downloaded.push(format!("tree-sitter grammars ({count} languages)"));
        } else {
            already_cached.push("tree-sitter grammars (all)".to_string());
        }
    } else if let Some(ref groups) = grammar_groups {
        let config = tree_sitter_language_pack::PackConfig {
            cache_dir: None,
            languages: None,
            groups: Some(groups.clone()),
        };
        tree_sitter_language_pack::init(&config).context("Failed to download tree-sitter grammar groups")?;
        downloaded.push(format!("tree-sitter grammars (groups: {})", groups.join(", ")));
    } else if let Some(ref langs) = grammars {
        let refs: Vec<&str> = langs.iter().map(String::as_str).collect();
        let count = tree_sitter_language_pack::download(&refs).context("Failed to download tree-sitter grammars")?;
        if count > 0 {
            downloaded.push(format!("tree-sitter grammars ({count} languages)"));
        } else {
            already_cached.push(format!("tree-sitter grammars ({})", langs.join(", ")));
        }
    }
    Ok(())
}

/// Download the requested GLiNER NER model(s).
#[cfg(feature = "ner-onnx")]
fn warm_ner_models(
    ner: bool,
    ner_model: Option<String>,
    all_ner_models: bool,
    hf_cache_dir: Option<PathBuf>,
    downloaded: &mut Vec<String>,
) -> Result<()> {
    let ner_models: Vec<String> = ner_model.into_iter().collect();
    if ner || !ner_models.is_empty() || all_ner_models {
        let to_download = crate::commands::ner::select_models(ner, ner_models, all_ner_models)?;
        downloaded.extend(
            crate::commands::ner::download_models(&to_download, hf_cache_dir)
                .context("Failed to download GLiNER NER models")?
                .into_iter()
                .map(|entry| format!("ner gliner ({entry})")),
        );
    }
    Ok(())
}

/// Warm every model family this binary was built with. Kept separate from [`warm_command`] so the
/// command reads as setup, one call and a summary while each family keeps its own feature gate.
#[cfg(any(
    feature = "embeddings",
    feature = "layout-detection",
    feature = "paddle-ocr",
    feature = "tree-sitter",
    feature = "ner-onnx",
    feature = "classification-onnx",
    feature = "captioning-onnx"
))]
fn warm_model_families(
    options: &WarmOptions,
    cache_base: &Path,
    #[cfg(any(
        feature = "embeddings",
        feature = "ner-onnx",
        feature = "paddle-ocr",
        feature = "classification-onnx",
        feature = "captioning-onnx"
    ))]
    hf_cache_dir: &Option<PathBuf>,
    downloaded: &mut Vec<String>,
    #[cfg(any(
        feature = "ocr",
        feature = "paddle-ocr",
        feature = "layout-detection",
        feature = "tree-sitter",
        feature = "formula-recognition"
    ))]
    already_cached: &mut Vec<String>,
    #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))] candidates: &mut Vec<
        super::onnx_candidates::CandidateSummary,
    >,
) -> Result<()> {
    #[cfg(feature = "paddle-ocr")]
    warm_paddle_ocr_models(hf_cache_dir.clone(), downloaded)?;

    #[cfg(feature = "formula-recognition")]
    warm_formula_recognition(cache_base, downloaded, already_cached)?;

    #[cfg(feature = "layout-detection")]
    warm_layout_detection(
        cache_base,
        options.all_table_models,
        options.layout_model.as_deref(),
        downloaded,
        already_cached,
    )?;

    #[cfg(any(feature = "ocr", feature = "paddle-ocr"))]
    warm_tessdata(cache_base, options, downloaded, already_cached)?;

    #[cfg(feature = "embeddings")]
    warm_embeddings(
        options.all_embeddings,
        options.embedding_model.clone(),
        hf_cache_dir.clone(),
        downloaded,
    )?;

    #[cfg(feature = "tree-sitter")]
    warm_tree_sitter_grammars(
        options.all_grammars,
        options.grammar_groups.clone(),
        options.grammars.clone(),
        downloaded,
        already_cached,
    )?;

    #[cfg(feature = "ner-onnx")]
    warm_ner_models(
        options.ner,
        options.ner_model.clone(),
        options.all_ner_models,
        hf_cache_dir.clone(),
        downloaded,
    )?;

    #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
    if !options.onnx.is_empty() {
        *candidates = super::onnx_candidates::stage_and_prepare(&options.onnx, hf_cache_dir.as_deref())?;
        downloaded.extend(
            candidates
                .iter()
                .map(|candidate| format!("{}/{} (staged and prepared)", candidate.feature, candidate.candidate)),
        );
    }

    Ok(())
}

/// Execute cache warm command - eagerly downloads all models.
#[cfg(any(
    feature = "embeddings",
    feature = "layout-detection",
    feature = "paddle-ocr",
    feature = "tree-sitter",
    feature = "ner-onnx",
    feature = "classification-onnx",
    feature = "captioning-onnx"
))]
pub fn warm_command(cache_dir: Option<PathBuf>, format: WireFormat, options: WarmOptions) -> Result<()> {
    #[cfg(any(
        feature = "embeddings",
        feature = "ner-onnx",
        feature = "paddle-ocr",
        feature = "classification-onnx",
        feature = "captioning-onnx"
    ))]
    let hf_cache_dir = cache_dir.clone();
    #[cfg(any(
        feature = "embeddings",
        feature = "ner-onnx",
        feature = "paddle-ocr",
        feature = "classification-onnx",
        feature = "captioning-onnx"
    ))]
    let hf_cache_label = resolve_hf_cache_label(&hf_cache_dir, &options);
    #[cfg(not(any(
        feature = "embeddings",
        feature = "ner-onnx",
        feature = "paddle-ocr",
        feature = "classification-onnx",
        feature = "captioning-onnx"
    )))]
    let hf_cache_label: Option<String> = None;
    let cache_base = resolve_cache_base(cache_dir);

    let mut downloaded: Vec<String> = Vec::new();
    #[cfg(any(
        feature = "ocr",
        feature = "paddle-ocr",
        feature = "layout-detection",
        feature = "tree-sitter",
        feature = "formula-recognition"
    ))]
    let mut already_cached: Vec<String> = Vec::new();
    #[cfg(not(any(
        feature = "ocr",
        feature = "paddle-ocr",
        feature = "layout-detection",
        feature = "tree-sitter",
        feature = "formula-recognition"
    )))]
    let already_cached: Vec<String> = Vec::new();

    #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
    let mut candidates: Vec<super::onnx_candidates::CandidateSummary> = Vec::new();

    warm_model_families(
        &options,
        &cache_base,
        #[cfg(any(
            feature = "embeddings",
            feature = "ner-onnx",
            feature = "paddle-ocr",
            feature = "classification-onnx",
            feature = "captioning-onnx"
        ))]
        &hf_cache_dir,
        &mut downloaded,
        #[cfg(any(
            feature = "ocr",
            feature = "paddle-ocr",
            feature = "layout-detection",
            feature = "tree-sitter",
            feature = "formula-recognition"
        ))]
        &mut already_cached,
        #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
        &mut candidates,
    )?;

    print_warm_summary(
        format,
        &cache_base,
        &hf_cache_label,
        &downloaded,
        &already_cached,
        #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
        &candidates,
    )
}

/// Resolve the cache base directory.
///
/// An explicit `--cache-dir` wins; otherwise this is the same root the library reads from
/// (`XBERG_CACHE_DIR`, then the platform cache directory), so `cache warm` stages models
/// where `serve` and `extract` look for them.
#[cfg(any(
    feature = "embeddings",
    feature = "layout-detection",
    feature = "paddle-ocr",
    feature = "tree-sitter",
    feature = "ner-onnx",
    feature = "classification-onnx",
    feature = "captioning-onnx"
))]
fn resolve_cache_base(cache_dir: Option<PathBuf>) -> PathBuf {
    cache_dir.unwrap_or_else(xberg::cache::resolve_cache_base_dir)
}

#[cfg(all(test, feature = "paddle-ocr", feature = "sceptre-ocr"))]
mod classic_ocr_seed_tests {
    use super::*;

    #[test]
    fn should_reject_an_empty_classic_ocr_cache_path() {
        let error = parse_nonempty_cache_dir("").expect_err("empty path must fail");
        assert_eq!(error, "classic OCR cache directory must not be empty");
    }

    #[test]
    fn should_preserve_a_nonempty_classic_ocr_cache_path() {
        assert_eq!(
            parse_nonempty_cache_dir("/opt/xberg/ocr").unwrap(),
            PathBuf::from("/opt/xberg/ocr")
        );
    }
}

#[cfg(all(test, feature = "layout-detection"))]
mod layout_warm_tests {
    use super::*;

    #[test]
    fn only_pp_doclayout_v3_needs_an_extra_download() {
        assert!(selected_model_needs_download(
            xberg::LayoutDetectionModel::PpDocLayoutV3
        ));
        assert!(!selected_model_needs_download(xberg::LayoutDetectionModel::RtDetr));
    }

    #[test]
    fn layout_model_flag_defaults_to_rtdetr_and_rejects_unknown_names() {
        assert_eq!(parse_layout_model(None).unwrap(), xberg::LayoutDetectionModel::RtDetr);
        assert_eq!(
            parse_layout_model(Some("pp_doc_layout_v3")).unwrap(),
            xberg::LayoutDetectionModel::PpDocLayoutV3
        );
        let error = parse_layout_model(Some("yolo")).unwrap_err();
        assert!(error.to_string().contains("Invalid --layout-model"));
    }
}
