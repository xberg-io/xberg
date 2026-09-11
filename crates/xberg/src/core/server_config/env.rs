//! Environment variable overrides for server configuration.
//!
//! This module provides functionality to override server configuration values
//! using environment variables. All settings can be overridden at runtime.

use crate::{Result, XbergError};

/// Apply environment variable overrides to a ServerConfig.
///
/// Reads the following environment variables and overrides config values if set:
///
/// - `XBERG_HOST` - Server host address
/// - `XBERG_PORT` - Server port number (parsed as u16)
/// - `XBERG_CORS_ORIGINS` - Comma-separated list of allowed origins
/// - `XBERG_MAX_REQUEST_BODY_BYTES` - Max request body size in bytes
/// - `XBERG_MAX_MULTIPART_FIELD_BYTES` - Max multipart field size in bytes
/// - `XBERG_JOB_TIMEOUT_SECS` - Async job fallback timeout in seconds
///
/// # Errors
///
/// Returns `XbergError::Validation` if:
/// - `XBERG_PORT` cannot be parsed as u16
/// - `XBERG_MAX_REQUEST_BODY_BYTES` cannot be parsed as usize
/// - `XBERG_MAX_MULTIPART_FIELD_BYTES` cannot be parsed as usize
/// - `XBERG_JOB_TIMEOUT_SECS` cannot be parsed as u64
pub(crate) fn apply_env_overrides(
    host: &mut String,
    port: &mut u16,
    cors_origins: &mut Vec<String>,
    max_request_body_bytes: &mut usize,
    max_multipart_field_bytes: &mut usize,
    job_timeout_secs: &mut u64,
) -> Result<()> {
    if let Ok(env_host) = std::env::var("XBERG_HOST") {
        *host = env_host;
    }

    if let Ok(port_str) = std::env::var("XBERG_PORT") {
        *port = port_str.parse::<u16>().map_err(|e| {
            XbergError::validation(format!(
                "XBERG_PORT must be a valid u16 number, got '{}': {}",
                port_str, e
            ))
        })?;
    }

    if let Ok(origins_str) = std::env::var("XBERG_CORS_ORIGINS") {
        *cors_origins = origins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    if let Ok(bytes_str) = std::env::var("XBERG_MAX_REQUEST_BODY_BYTES") {
        *max_request_body_bytes = bytes_str.parse::<usize>().map_err(|e| {
            XbergError::validation(format!(
                "XBERG_MAX_REQUEST_BODY_BYTES must be a valid usize, got '{}': {}",
                bytes_str, e
            ))
        })?;
    }

    if let Ok(bytes_str) = std::env::var("XBERG_MAX_MULTIPART_FIELD_BYTES") {
        *max_multipart_field_bytes = bytes_str.parse::<usize>().map_err(|e| {
            XbergError::validation(format!(
                "XBERG_MAX_MULTIPART_FIELD_BYTES must be a valid usize, got '{}': {}",
                bytes_str, e
            ))
        })?;
    }

    if let Ok(secs_str) = std::env::var("XBERG_JOB_TIMEOUT_SECS") {
        *job_timeout_secs = secs_str.parse::<u64>().map_err(|e| {
            XbergError::validation(format!(
                "XBERG_JOB_TIMEOUT_SECS must be a valid u64, got '{}': {}",
                secs_str, e
            ))
        })?;
    }

    Ok(())
}
