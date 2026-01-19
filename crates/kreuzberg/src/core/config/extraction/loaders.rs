//! Configuration file loading with caching support.
//!
//! This module provides methods for loading extraction configuration from various
//! file formats (TOML, YAML, JSON) with automatic caching based on file modification times.

use crate::{KreuzbergError, Result};
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::SystemTime;

use super::core::ExtractionConfig;

static CONFIG_CACHE: LazyLock<DashMap<PathBuf, (SystemTime, Arc<ExtractionConfig>)>> = LazyLock::new(DashMap::new);

impl ExtractionConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML file
    ///
    /// # Errors
    ///
    /// Returns `KreuzbergError::Validation` if file doesn't exist or is invalid TOML.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let metadata = std::fs::metadata(path)
            .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;
        let mtime = metadata.modified().map_err(|e| {
            KreuzbergError::validation(format!("Failed to get modification time for {}: {}", path.display(), e))
        })?;

        if let Some(entry) = CONFIG_CACHE.get(path)
            && entry.0 == mtime
        {
            return Ok((*entry.1).clone());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;

        let config: Self = toml::from_str(&content)
            .map_err(|e| KreuzbergError::validation(format!("Invalid TOML in {}: {}", path.display(), e)))?;

        let config_arc = Arc::new(config.clone());
        CONFIG_CACHE.insert(path.to_path_buf(), (mtime, config_arc));

        Ok(config)
    }

    /// Load configuration from a YAML file.
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let metadata = std::fs::metadata(path)
            .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;
        let mtime = metadata.modified().map_err(|e| {
            KreuzbergError::validation(format!("Failed to get modification time for {}: {}", path.display(), e))
        })?;

        if let Some(entry) = CONFIG_CACHE.get(path)
            && entry.0 == mtime
        {
            return Ok((*entry.1).clone());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;

        let config: Self = serde_yaml_ng::from_str(&content)
            .map_err(|e| KreuzbergError::validation(format!("Invalid YAML in {}: {}", path.display(), e)))?;

        let config_arc = Arc::new(config.clone());
        CONFIG_CACHE.insert(path.to_path_buf(), (mtime, config_arc));

        Ok(config)
    }

    /// Load configuration from a JSON file.
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let metadata = std::fs::metadata(path)
            .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;
        let mtime = metadata.modified().map_err(|e| {
            KreuzbergError::validation(format!("Failed to get modification time for {}: {}", path.display(), e))
        })?;

        if let Some(entry) = CONFIG_CACHE.get(path)
            && entry.0 == mtime
        {
            return Ok((*entry.1).clone());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;

        let config: Self = serde_json::from_str(&content)
            .map_err(|e| KreuzbergError::validation(format!("Invalid JSON in {}: {}", path.display(), e)))?;

        let config_arc = Arc::new(config.clone());
        CONFIG_CACHE.insert(path.to_path_buf(), (mtime, config_arc));

        Ok(config)
    }

    /// Load configuration from a file, auto-detecting format by extension.
    ///
    /// Supported formats:
    /// - `.toml` - TOML format
    /// - `.yaml` - YAML format
    /// - `.json` - JSON format
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Errors
    ///
    /// Returns `KreuzbergError::Validation` if:
    /// - File doesn't exist
    /// - File extension is not supported
    /// - File content is invalid for the detected format
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::core::config::ExtractionConfig;
    ///
    /// // Auto-detects TOML format
    /// // let config = ExtractionConfig::from_file("kreuzberg.toml")?;
    ///
    /// // Auto-detects YAML format
    /// // let config = ExtractionConfig::from_file("kreuzberg.yaml")?;
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let metadata = std::fs::metadata(path)
            .map_err(|e| KreuzbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;
        let mtime = metadata.modified().map_err(|e| {
            KreuzbergError::validation(format!("Failed to get modification time for {}: {}", path.display(), e))
        })?;

        if let Some(entry) = CONFIG_CACHE.get(path)
            && entry.0 == mtime
        {
            return Ok((*entry.1).clone());
        }

        let extension = path.extension().and_then(|ext| ext.to_str()).ok_or_else(|| {
            KreuzbergError::validation(format!(
                "Cannot determine file format: no extension found in {}",
                path.display()
            ))
        })?;

        let config = match extension.to_lowercase().as_str() {
            "toml" => Self::from_toml_file(path)?,
            "yaml" | "yml" => Self::from_yaml_file(path)?,
            "json" => Self::from_json_file(path)?,
            _ => {
                return Err(KreuzbergError::validation(format!(
                    "Unsupported config file format: .{}. Supported formats: .toml, .yaml, .json",
                    extension
                )));
            }
        };

        let config_arc = Arc::new(config.clone());
        CONFIG_CACHE.insert(path.to_path_buf(), (mtime, config_arc));

        Ok(config)
    }

    /// Discover configuration file in parent directories.
    ///
    /// Searches for `kreuzberg.toml` in current directory and parent directories.
    ///
    /// # Returns
    ///
    /// - `Some(config)` if found
    /// - `None` if no config file found
    pub fn discover() -> Result<Option<Self>> {
        let mut current = std::env::current_dir().map_err(KreuzbergError::Io)?;

        loop {
            let kreuzberg_toml = current.join("kreuzberg.toml");
            if kreuzberg_toml.exists() {
                return Ok(Some(Self::from_toml_file(kreuzberg_toml)?));
            }

            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }

        Ok(None)
    }
}
