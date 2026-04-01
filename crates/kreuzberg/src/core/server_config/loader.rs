//! File loading logic for server configuration.
//!
//! This module provides functionality to load server configuration from various
//! file formats (TOML, YAML, JSON) with support for both flat and nested formats.

use crate::{KreuzbergError, Result};
use serde::Deserialize;
use std::path::Path;

use super::ServerConfig;

/// Load server configuration from a file.
///
/// Automatically detects the file format based on extension:
/// - `.toml` - TOML format
/// - `.yaml` or `.yml` - YAML format
/// - `.json` - JSON format
///
/// This function handles two config file formats:
/// 1. Flat format: Server config at root level
/// 2. Nested format: Server config under `[server]` section (combined with ExtractionConfig)
///
/// # Arguments
///
/// * `path` - Path to the configuration file
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if:
/// - File doesn't exist or cannot be read
/// - File extension is not recognized
/// - File content is invalid for the detected format
pub fn from_file(path: impl AsRef<Path>) -> Result<ServerConfig> {
    let path = path.as_ref();

    let content = std::fs::read_to_string(path)
        .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;

    let extension = path.extension().and_then(|ext| ext.to_str()).ok_or_else(|| {
        KreuzbergError::validation(format!(
            "Cannot determine file format: no extension found in {}",
            path.display()
        ))
    })?;

    let config = match extension.to_lowercase().as_str() {
        "toml" => from_toml_str(&content, path)?,
        "yaml" | "yml" => from_yaml_str(&content, path)?,
        "json" => from_json_str(&content, path)?,
        _ => {
            return Err(KreuzbergError::validation(format!(
                "Unsupported config file format: .{}. Supported formats: .toml, .yaml, .yml, .json",
                extension
            )));
        }
    };

    Ok(config)
}

/// Load server configuration from a TOML file.
///
/// # Arguments
///
/// * `path` - Path to the TOML file
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if the file doesn't exist or is invalid TOML.
pub fn from_toml_file(path: impl AsRef<Path>) -> Result<ServerConfig> {
    let path = path.as_ref();

    let content = std::fs::read_to_string(path)
        .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;

    let config: ServerConfig = toml::from_str(&content)
        .map_err(|e| KreuzbergError::validation(format!("Invalid TOML in {}: {}", path.display(), e)))?;

    Ok(config)
}

/// Load server configuration from a YAML file.
///
/// # Arguments
///
/// * `path` - Path to the YAML file
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if the file doesn't exist or is invalid YAML.
pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<ServerConfig> {
    let path = path.as_ref();

    let content = std::fs::read_to_string(path)
        .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;

    let config: ServerConfig = serde_yaml_ng::from_str(&content)
        .map_err(|e| KreuzbergError::validation(format!("Invalid YAML in {}: {}", path.display(), e)))?;

    Ok(config)
}

/// Load server configuration from a JSON file.
///
/// # Arguments
///
/// * `path` - Path to the JSON file
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if the file doesn't exist or is invalid JSON.
pub fn from_json_file(path: impl AsRef<Path>) -> Result<ServerConfig> {
    let path = path.as_ref();

    let content = std::fs::read_to_string(path)
        .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;

    let config: ServerConfig = serde_json::from_str(&content)
        .map_err(|e| KreuzbergError::validation(format!("Invalid JSON in {}: {}", path.display(), e)))?;

    Ok(config)
}

// Helper functions for parsing different formats

fn from_toml_str(content: &str, path: &Path) -> Result<ServerConfig> {
    // Try nested format first (with [server] section)
    #[derive(Deserialize)]
    struct RootConfig {
        #[serde(default)]
        server: Option<ServerConfig>,
    }

    if let Ok(root) = toml::from_str::<RootConfig>(content) {
        if let Some(server) = root.server {
            return Ok(server);
        } else {
            // No [server] section, try flat format
            return toml::from_str::<ServerConfig>(content)
                .map_err(|e| KreuzbergError::validation(format!("Invalid TOML in {}: {}", path.display(), e)));
        }
    }

    // Fall back to flat format
    toml::from_str::<ServerConfig>(content)
        .map_err(|e| KreuzbergError::validation(format!("Invalid TOML in {}: {}", path.display(), e)))
}

fn from_yaml_str(content: &str, path: &Path) -> Result<ServerConfig> {
    // Try nested format first (with server: section)
    #[derive(Deserialize)]
    struct RootConfig {
        #[serde(default)]
        server: Option<ServerConfig>,
    }

    if let Ok(root) = serde_yaml_ng::from_str::<RootConfig>(content) {
        if let Some(server) = root.server {
            return Ok(server);
        } else {
            // No server section, try flat format
            return serde_yaml_ng::from_str::<ServerConfig>(content)
                .map_err(|e| KreuzbergError::validation(format!("Invalid YAML in {}: {}", path.display(), e)));
        }
    }

    // Fall back to flat format
    serde_yaml_ng::from_str::<ServerConfig>(content)
        .map_err(|e| KreuzbergError::validation(format!("Invalid YAML in {}: {}", path.display(), e)))
}

fn from_json_str(content: &str, path: &Path) -> Result<ServerConfig> {
    // Try nested format first (with "server" key)
    #[derive(Deserialize)]
    struct RootConfig {
        #[serde(default)]
        server: Option<ServerConfig>,
    }

    if let Ok(root) = serde_json::from_str::<RootConfig>(content) {
        if let Some(server) = root.server {
            return Ok(server);
        } else {
            // No server key, try flat format
            return serde_json::from_str::<ServerConfig>(content)
                .map_err(|e| KreuzbergError::validation(format!("Invalid JSON in {}: {}", path.display(), e)));
        }
    }

    // Fall back to flat format
    serde_json::from_str::<ServerConfig>(content)
        .map_err(|e| KreuzbergError::validation(format!("Invalid JSON in {}: {}", path.display(), e)))
}
