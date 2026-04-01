//! Environment variable overrides for server configuration.
//!
//! This module provides functionality to override server configuration values
//! using environment variables. All settings can be overridden at runtime.

use crate::{KreuzbergError, Result};

/// Apply environment variable overrides to a ServerConfig.
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
pub fn apply_env_overrides(
    host: &mut String,
    port: &mut u16,
    cors_origins: &mut Vec<String>,
    max_request_body_bytes: &mut usize,
    max_multipart_field_bytes: &mut usize,
) -> Result<()> {
    // Host override
    if let Ok(env_host) = std::env::var("KREUZBERG_HOST") {
        *host = env_host;
    }

    // Port override
    if let Ok(port_str) = std::env::var("KREUZBERG_PORT") {
        *port = port_str.parse::<u16>().map_err(|e| {
            KreuzbergError::validation(format!(
                "KREUZBERG_PORT must be a valid u16 number, got '{}': {}",
                port_str, e
            ))
        })?;
    }

    // CORS origins override (comma-separated)
    if let Ok(origins_str) = std::env::var("KREUZBERG_CORS_ORIGINS") {
        *cors_origins = origins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // Max request body bytes override
    if let Ok(bytes_str) = std::env::var("KREUZBERG_MAX_REQUEST_BODY_BYTES") {
        *max_request_body_bytes = bytes_str.parse::<usize>().map_err(|e| {
            KreuzbergError::validation(format!(
                "KREUZBERG_MAX_REQUEST_BODY_BYTES must be a valid usize, got '{}': {}",
                bytes_str, e
            ))
        })?;
    }

    // Max multipart field bytes override
    if let Ok(bytes_str) = std::env::var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES") {
        *max_multipart_field_bytes = bytes_str.parse::<usize>().map_err(|e| {
            KreuzbergError::validation(format!(
                "KREUZBERG_MAX_MULTIPART_FIELD_BYTES must be a valid usize, got '{}': {}",
                bytes_str, e
            ))
        })?;
    }

    Ok(())
}
