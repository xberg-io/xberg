//! Cache command - Manage cache operations
//!
//! This module provides commands for cache management including statistics,
//! clearing, manifest generation, and model warming.

use anyhow::{Context, Result};
use kreuzberg::cache;
use serde_json::json;
use std::path::PathBuf;

use crate::{WireFormat, style};

/// Execute cache stats command
pub fn stats_command(cache_dir: Option<PathBuf>, format: WireFormat) -> Result<()> {
    let default_cache_dir = std::env::current_dir()
        .context("Failed to get current directory")?
        .join(".kreuzberg");

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

/// Execute cache clear command
pub fn clear_command(cache_dir: Option<PathBuf>, format: WireFormat) -> Result<()> {
    let default_cache_dir = std::env::current_dir()
        .context("Failed to get current directory")?
        .join(".kreuzberg");

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
            println!("{}", style::success("Cache cleared successfully"));
            println!("{} {}", style::label("Directory:"), style::success(&cache_dir_str));
            println!("{} {}", style::label("Removed files:"), removed_files);
            println!("{} {:.2} MB", style::label("Freed space:"), freed_mb);
        }
        WireFormat::Json => {
            let output = json!({
                "directory": cache_dir_str,
                "removed_files": removed_files,
                "freed_mb": freed_mb,
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
    // Without at least one model-providing feature, every `extend` call
    // below is `#[cfg]`-stripped and `entries: Vec<_>` has no anchor for
    // type inference — `e.size_bytes` on the closure further down then
    // fails compilation with E0282. Bail with a clear error instead so
    // the build with `--no-default-features --features "bundled-pdfium"`
    // (or similar minimal configurations) succeeds.
    #[cfg(not(any(feature = "paddle-ocr", feature = "layout-detection")))]
    {
        let _ = format;
        anyhow::bail!(
            "manifest command unavailable: build kreuzberg-cli with at least one of \
             --features \"paddle-ocr\" or --features \"layout-detection\""
        );
    }

    #[cfg(any(feature = "paddle-ocr", feature = "layout-detection"))]
    {
        manifest_command_inner(format)
    }
}

#[cfg(any(feature = "paddle-ocr", feature = "layout-detection"))]
fn manifest_command_inner(format: WireFormat) -> Result<()> {
    let mut entries = Vec::new();

    #[cfg(feature = "paddle-ocr")]
    {
        entries.extend(kreuzberg::paddle_ocr::ModelManager::manifest());
    }

    #[cfg(feature = "layout-detection")]
    {
        entries.extend(kreuzberg::layout::LayoutModelManager::manifest());
    }

    #[cfg(feature = "paddle-ocr")]
    {
        entries.extend(kreuzberg::ocr::TessdataManager::manifest());
    }

    let total_size_bytes: u64 = entries.iter().map(|e| e.size_bytes).sum();
    let version = env!("CARGO_PKG_VERSION");

    match format {
        WireFormat::Text => {
            println!(
                "{} {}",
                style::header("Model Manifest"),
                style::dim(&format!("(kreuzberg {})", version))
            );
            println!("{}", style::dim("===================================="));
            println!(
                "{:<50} {:>12} {}",
                style::label("PATH"),
                style::label("SIZE"),
                style::label("SHA256")
            );
            println!("{}", style::dim(&format!("{:<50} {:>12} ------", "----", "----")));
            for entry in &entries {
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
        WireFormat::Json => {
            let output = json!({
                "kreuzberg_version": version,
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
                "kreuzberg_version": version,
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

/// Execute cache warm command - eagerly downloads all models.
#[allow(clippy::too_many_arguments)]
pub fn warm_command(
    cache_dir: Option<PathBuf>,
    format: WireFormat,
    all_embeddings: bool,
    embedding_model: Option<String>,
    all_table_models: bool,
    all_grammars: bool,
    grammar_groups: Option<Vec<String>>,
    grammars: Option<Vec<String>>,
) -> Result<()> {
    let cache_base = resolve_cache_base(cache_dir);

    let mut downloaded: Vec<String> = Vec::new();
    let mut already_cached: Vec<String> = Vec::new();

    #[cfg(feature = "paddle-ocr")]
    {
        let paddle_dir = cache_base.join("paddle-ocr");
        let manager = kreuzberg::paddle_ocr::ModelManager::new(paddle_dir);

        // ensure_all_models downloads v2 det (server+mobile), cls (PP-LCNet),
        // doc_ori, v2 unified rec models, and all per-script rec families
        manager
            .ensure_all_models()
            .context("Failed to download PaddleOCR v2 models")?;
        downloaded.push("paddle-ocr v2 (server+mobile det, cls, doc_ori, unified+per-script rec)".to_string());
    }

    #[cfg(feature = "layout-detection")]
    {
        let layout_dir = cache_base.join("layout");
        let manager = kreuzberg::layout::LayoutModelManager::new(Some(layout_dir));

        if all_table_models {
            // Download rtdetr + tatr + all SLANeXT variants (~730MB)
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
            // Default: download only rtdetr + tatr
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
    }

    #[cfg(feature = "paddle-ocr")]
    {
        let tessdata_dir = cache_base.join("tessdata");
        let manager = kreuzberg::ocr::TessdataManager::new(Some(tessdata_dir));

        let newly_downloaded = manager
            .ensure_all_languages()
            .context("Failed to download tessdata files")?;

        if newly_downloaded > 0 {
            downloaded.push(format!("tessdata ({newly_downloaded} languages)"));
        } else {
            already_cached.push("tessdata (all languages)".to_string());
        }
    }

    #[cfg(feature = "embeddings")]
    {
        let embeddings_dir = cache_base.join("embeddings");
        let presets_to_warm: Vec<&kreuzberg::EmbeddingPreset> = if all_embeddings {
            kreuzberg::EMBEDDING_PRESETS.iter().collect()
        } else if let Some(ref name) = embedding_model {
            match kreuzberg::get_preset(name) {
                Some(preset) => vec![preset],
                None => {
                    let available: Vec<&str> = kreuzberg::list_presets();
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
            kreuzberg::warm_model(
                &kreuzberg::core::config::EmbeddingModelType::Preset {
                    name: preset.name.to_string(),
                },
                Some(embeddings_dir.clone()),
            )
            .map_err(|e| anyhow::anyhow!("Failed to download embedding model '{}': {}", preset.name, e))?;
            downloaded.push(label);
        }
    }

    #[cfg(not(feature = "embeddings"))]
    {
        if all_embeddings || embedding_model.is_some() {
            anyhow::bail!("Embedding model warming requires the 'embeddings' feature to be enabled");
        }
    }

    // Tree-sitter grammar downloads
    #[cfg(feature = "tree-sitter")]
    {
        if all_grammars {
            let count =
                tree_sitter_language_pack::download_all().context("Failed to download all tree-sitter grammars")?;
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
            let count =
                tree_sitter_language_pack::download(&refs).context("Failed to download tree-sitter grammars")?;
            if count > 0 {
                downloaded.push(format!("tree-sitter grammars ({count} languages)"));
            } else {
                already_cached.push(format!("tree-sitter grammars ({})", langs.join(", ")));
            }
        }
    }

    #[cfg(not(feature = "tree-sitter"))]
    {
        if all_grammars || grammar_groups.is_some() || grammars.is_some() {
            anyhow::bail!("Tree-sitter grammar warming requires the 'tree-sitter' feature to be enabled");
        }
    }

    match format {
        WireFormat::Text => {
            if !downloaded.is_empty() {
                println!("{}", style::label("Downloaded:"));
                for d in &downloaded {
                    println!("  {}", style::success(d));
                }
            }
            if !already_cached.is_empty() {
                println!("{}", style::label("Already cached:"));
                for c in &already_cached {
                    println!("  {}", style::dim(c));
                }
            }
            println!(
                "All models ready in {}",
                style::success(&cache_base.display().to_string())
            );
        }
        WireFormat::Json => {
            let output = json!({
                "cache_dir": cache_base.to_string_lossy(),
                "downloaded": downloaded,
                "already_cached": already_cached,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).context("Failed to serialize warm results to JSON")?
            );
        }
        WireFormat::Toon => {
            let output = json!({
                "cache_dir": cache_base.to_string_lossy(),
                "downloaded": downloaded,
                "already_cached": already_cached,
            });
            println!(
                "{}",
                serde_toon::to_string(&output).context("Failed to serialize warm results to TOON")?
            );
        }
    }

    Ok(())
}

/// Resolve the cache base directory.
fn resolve_cache_base(cache_dir: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = cache_dir {
        return dir;
    }
    if let Ok(env_path) = std::env::var("KREUZBERG_CACHE_DIR") {
        return PathBuf::from(env_path);
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".kreuzberg")
}
