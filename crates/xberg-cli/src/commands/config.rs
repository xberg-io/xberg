//! Config command - Configuration loading and discovery
//!
//! This module provides utilities for loading extraction configuration from files
//! or discovering them automatically in the project directory.

use anyhow::{Context, Result};
use std::path::PathBuf;
use xberg::ExtractionConfig;

/// Loads extraction configuration from a file or discovers it automatically.
///
/// This function implements the CLI's configuration hierarchy:
/// 1. Explicit config file (if `--config` flag provided)
/// 2. Auto-discovered config, unless `discover` is `false`
/// 3. Default configuration (if no config file found)
///
/// # Configuration File Formats
///
/// Supports three formats, determined by file extension:
/// - `.toml`: TOML format (recommended for humans)
/// - `.yaml` / `.yml`: YAML format
/// - `.json`: JSON format
///
/// # Errors
///
/// Returns an error if:
/// - Explicit config file has unsupported extension (must be .toml, .yaml, .yml, or .json)
/// - Config file cannot be read or parsed
/// - Config file contains invalid extraction settings
pub fn load_config(config_path: Option<PathBuf>, discover: bool) -> Result<ExtractionConfig> {
    if let Some(path) = config_path {
        let path_str = path.to_string_lossy();
        let path_lower = path_str.to_lowercase();
        let config = if path_lower.ends_with(".toml") {
            ExtractionConfig::from_toml_file(&path)
        } else if path_lower.ends_with(".yaml") || path_lower.ends_with(".yml") {
            ExtractionConfig::from_yaml_file(&path)
        } else if path_lower.ends_with(".json") {
            ExtractionConfig::from_json_file(&path)
        } else {
            anyhow::bail!("Config file must have .toml, .yaml, .yml, or .json extension (case-insensitive)");
        };
        config.with_context(|| format!("Failed to load configuration from '{}'. Ensure the file exists, is readable, and contains valid configuration.", path.display()))
    } else if discover {
        match ExtractionConfig::discover() {
            Ok(Some(config)) => Ok(config),
            Ok(None) => Ok(ExtractionConfig::default()),
            Err(e) => Err(e).context("Failed to auto-discover configuration file. Searched for xberg.{toml,yaml,json} in current and parent directories. Use --config to specify an explicit path."),
        }
    } else {
        Ok(ExtractionConfig::default())
    }
}
