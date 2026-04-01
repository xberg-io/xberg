//! Server configuration for the Kreuzberg API.
//!
//! This module provides the `ServerConfig` struct for managing API server settings
//! including host, port, CORS, and upload size limits. Configuration can be loaded
//! from TOML, YAML, or JSON files and can be overridden by environment variables.
//!
//! # Features
//!
//! - **Multi-format support**: Load configuration from TOML, YAML, or JSON files
//! - **Environment overrides**: All settings can be overridden via environment variables
//! - **Sensible defaults**: All fields have reasonable defaults matching current behavior
//! - **Flexible CORS**: Support for all origins (default) or specific origin lists
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::core::ServerConfig;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! // Create with defaults
//! let mut config = ServerConfig::default();
//!
//! // Or load from file
//! let mut config = ServerConfig::from_file("kreuzberg.toml")?;
//!
//! // Apply environment variable overrides
//! config.apply_env_overrides()?;
//!
//! # Ok(())
//! # }
//! ```

use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

mod env;
mod loader;
mod validation;

#[cfg(test)]
mod tests;

/// Default host address for API server
const DEFAULT_HOST: &str = "127.0.0.1";

/// Default port for API server
const DEFAULT_PORT: u16 = 8000;

/// Default maximum request body size: 100 MB
const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 104_857_600;

/// Default maximum multipart field size: 100 MB
const DEFAULT_MAX_MULTIPART_FIELD_BYTES: usize = 104_857_600;

/// API server configuration.
///
/// This struct holds all configuration options for the Kreuzberg API server,
/// including host/port settings, CORS configuration, and upload limits.
///
/// # Defaults
///
/// - `host`: "127.0.0.1" (localhost only)
/// - `port`: 8000
/// - `cors_origins`: empty vector (allows all origins)
/// - `max_request_body_bytes`: 104_857_600 (100 MB)
/// - `max_multipart_field_bytes`: 104_857_600 (100 MB)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Server host address (e.g., "127.0.0.1", "0.0.0.0")
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port number
    #[serde(default = "default_port")]
    pub port: u16,

    /// CORS allowed origins. Empty vector means allow all origins.
    ///
    /// If this is an empty vector, the server will accept requests from any origin.
    /// If populated with specific origins (e.g., ["https://example.com"]), only
    /// those origins will be allowed.
    #[serde(default)]
    pub cors_origins: Vec<String>,

    /// Maximum size of request body in bytes (default: 100 MB)
    #[serde(default = "default_max_request_body_bytes")]
    pub max_request_body_bytes: usize,

    /// Maximum size of multipart fields in bytes (default: 100 MB)
    #[serde(default = "default_max_multipart_field_bytes")]
    pub max_multipart_field_bytes: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            cors_origins: Vec::new(),
            max_request_body_bytes: default_max_request_body_bytes(),
            max_multipart_field_bytes: default_max_multipart_field_bytes(),
        }
    }
}

// Default value functions for serde
fn default_host() -> String {
    DEFAULT_HOST.to_string()
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

fn default_max_request_body_bytes() -> usize {
    DEFAULT_MAX_REQUEST_BODY_BYTES
}

fn default_max_multipart_field_bytes() -> usize {
    DEFAULT_MAX_MULTIPART_FIELD_BYTES
}

impl ServerConfig {
    /// Create a new `ServerConfig` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the server listen address (host:port).
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::core::ServerConfig;
    ///
    /// let config = ServerConfig::default();
    /// assert_eq!(config.listen_addr(), "127.0.0.1:8000");
    /// ```
    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Check if CORS allows all origins.
    ///
    /// Returns `true` if the `cors_origins` vector is empty, meaning all origins
    /// are allowed. Returns `false` if specific origins are configured.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::core::ServerConfig;
    ///
    /// let mut config = ServerConfig::default();
    /// assert!(config.cors_allows_all());
    ///
    /// config.cors_origins.push("https://example.com".to_string());
    /// assert!(!config.cors_allows_all());
    /// ```
    pub fn cors_allows_all(&self) -> bool {
        self.cors_origins.is_empty()
    }

    /// Check if a given origin is allowed by CORS configuration.
    ///
    /// Returns `true` if:
    /// - CORS allows all origins (empty origins list), or
    /// - The given origin is in the allowed origins list
    ///
    /// # Arguments
    ///
    /// * `origin` - The origin to check (e.g., "https://example.com")
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::core::ServerConfig;
    ///
    /// let mut config = ServerConfig::default();
    /// assert!(config.is_origin_allowed("https://example.com"));
    ///
    /// config.cors_origins.push("https://allowed.com".to_string());
    /// assert!(config.is_origin_allowed("https://allowed.com"));
    /// assert!(!config.is_origin_allowed("https://denied.com"));
    /// ```
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        self.cors_origins.is_empty() || self.cors_origins.contains(&origin.to_string())
    }

    /// Get maximum request body size in megabytes (rounded up).
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::core::ServerConfig;
    ///
    /// let mut config = ServerConfig::default();
    /// assert_eq!(config.max_request_body_mb(), 100);
    /// ```
    pub fn max_request_body_mb(&self) -> usize {
        self.max_request_body_bytes.div_ceil(1_048_576)
    }

    /// Get maximum multipart field size in megabytes (rounded up).
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::core::ServerConfig;
    ///
    /// let mut config = ServerConfig::default();
    /// assert_eq!(config.max_multipart_field_mb(), 100);
    /// ```
    pub fn max_multipart_field_mb(&self) -> usize {
        self.max_multipart_field_bytes.div_ceil(1_048_576)
    }

    /// Apply environment variable overrides to the configuration.
    ///
    /// Reads the following environment variables and overrides config values if set:
    ///
    /// - `KREUZBERG_HOST` - Server host address
    /// - `KREUZBERG_PORT` - Server port number (parsed as u16)
    /// - `KREUZBERG_CORS_ORIGINS` - Comma-separated list of allowed origins
    /// - `KREUZBERG_MAX_REQUEST_BODY_BYTES` - Max request body size in bytes
    /// - `KREUZBERG_MAX_MULTIPART_FIELD_BYTES` - Max multipart field size in bytes
    ///
    /// # Errors
    ///
    /// Returns `KreuzbergError::Validation` if:
    /// - `KREUZBERG_PORT` cannot be parsed as u16
    /// - `KREUZBERG_MAX_REQUEST_BODY_BYTES` cannot be parsed as usize
    /// - `KREUZBERG_MAX_MULTIPART_FIELD_BYTES` cannot be parsed as usize
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use kreuzberg::core::ServerConfig;
    ///
    /// # fn example() -> kreuzberg::Result<()> {
    /// unsafe {
    ///     std::env::set_var("KREUZBERG_HOST", "0.0.0.0");
    ///     std::env::set_var("KREUZBERG_PORT", "3000");
    /// }
    ///
    /// let mut config = ServerConfig::default();
    /// config.apply_env_overrides()?;
    ///
    /// assert_eq!(config.host, "0.0.0.0");
    /// assert_eq!(config.port, 3000);
    /// # Ok(())
    /// # }
    /// ```
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        env::apply_env_overrides(
            &mut self.host,
            &mut self.port,
            &mut self.cors_origins,
            &mut self.max_request_body_bytes,
            &mut self.max_multipart_field_bytes,
        )?;

        Ok(())
    }

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
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use kreuzberg::core::ServerConfig;
    ///
    /// # fn example() -> kreuzberg::Result<()> {
    /// let config = ServerConfig::from_file("kreuzberg.toml")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        loader::from_file(path)
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
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use kreuzberg::core::ServerConfig;
    ///
    /// # fn example() -> kreuzberg::Result<()> {
    /// let config = ServerConfig::from_toml_file("kreuzberg.toml")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        loader::from_toml_file(path)
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
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        loader::from_yaml_file(path)
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
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self> {
        loader::from_json_file(path)
    }
}
