//! Cross-section dependency validation.
//!
//! This module contains validation functions that check dependencies and relationships
//! between different configuration sections. These validators ensure that related
//! configuration values are consistent and compatible with each other.

#[cfg(test)]
use crate::{Result, XbergError};

#[cfg(test)]
pub(crate) fn validate_port(port: u32) -> Result<()> {
    if port == 0 || port > 65535 {
        Err(XbergError::Validation {
            message: format!("Port must be 1-65535, got {}", port),
            source: None,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn validate_host(host: &str) -> Result<()> {
    let host = host.trim();

    if host.is_empty() {
        return Err(XbergError::Validation {
            message: "Invalid host '': must be a valid IP address or hostname".to_string(),
            source: None,
        });
    }

    // Check if it's a valid IPv4 address
    if host.parse::<std::net::Ipv4Addr>().is_ok() {
        return Ok(());
    }

    // Check if it's a valid IPv6 address
    if host.parse::<std::net::Ipv6Addr>().is_ok() {
        return Ok(());
    }

    // Check if it's a valid hostname (basic validation)
    // Hostnames must contain only alphanumeric characters, dots, and hyphens
    // Must not look like an invalid IPv4 address (all numeric with dots)
    let looks_like_ipv4 = host
        .split('.')
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_numeric()));
    if !looks_like_ipv4
        && host.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-')
        && !host.starts_with('-')
        && !host.ends_with('-')
    {
        return Ok(());
    }

    Err(XbergError::Validation {
        message: format!("Invalid host '{}': must be a valid IP address or hostname", host),
        source: None,
    })
}

#[cfg(test)]
pub(crate) fn validate_cors_origin(origin: &str) -> Result<()> {
    let origin = origin.trim();

    if origin == "*" {
        return Ok(());
    }

    if origin.starts_with("http://") || origin.starts_with("https://") {
        // Basic validation: ensure there's something after the protocol
        if origin.len() > 8 && (origin.starts_with("http://") && origin.len() > 7 || origin.starts_with("https://")) {
            return Ok(());
        }
    }

    Err(XbergError::Validation {
        message: format!(
            "Invalid CORS origin '{}': must be a valid HTTP/HTTPS URL or '*'",
            origin
        ),
        source: None,
    })
}

#[cfg(test)]
pub(crate) fn validate_upload_size(size: usize) -> Result<()> {
    if size > 0 {
        Ok(())
    } else {
        Err(XbergError::Validation {
            message: format!("Upload size must be greater than 0, got {}", size),
            source: None,
        })
    }
}
