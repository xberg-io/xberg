//! R list -> ExtractionConfig conversion

use crate::error::{kreuzberg_error, to_r_error};
use extendr_api::prelude::*;

/// Parse a JSON config string into an ExtractionConfig
pub fn parse_config(config_json: Nullable<&str>) -> extendr_api::Result<kreuzberg::ExtractionConfig> {
    match config_json {
        Nullable::NotNull(json_str) => {
            let config: kreuzberg::ExtractionConfig =
                serde_json::from_str(json_str).map_err(to_r_error)?;
            Ok(config)
        }
        Nullable::Null => Ok(kreuzberg::ExtractionConfig::default()),
    }
}

/// Parse a JSON config string into an EmbeddingConfig
pub fn parse_config_embedding(config_json: Nullable<&str>) -> extendr_api::Result<kreuzberg::EmbeddingConfig> {
    match config_json {
        Nullable::NotNull(json_str) => {
            let config: kreuzberg::EmbeddingConfig =
                serde_json::from_str(json_str).map_err(to_r_error)?;
            Ok(config)
        }
        Nullable::Null => Ok(kreuzberg::EmbeddingConfig::default()),
    }
}

/// Load an ExtractionConfig from a file (TOML, YAML, or JSON)
pub fn from_file_impl(path: &str) -> extendr_api::Result<Nullable<String>> {
    let config = kreuzberg::ExtractionConfig::from_file(path).map_err(kreuzberg_error)?;
    let json = serde_json::to_string(&config).map_err(to_r_error)?;
    Ok(Nullable::NotNull(json))
}

/// Discover an ExtractionConfig from kreuzberg.toml in current or parent directories
pub fn discover_impl() -> extendr_api::Result<Nullable<String>> {
    match kreuzberg::ExtractionConfig::discover().map_err(kreuzberg_error)? {
        Some(config) => {
            let json = serde_json::to_string(&config).map_err(to_r_error)?;
            Ok(Nullable::NotNull(json))
        }
        None => Ok(Nullable::Null),
    }
}
