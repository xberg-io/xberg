//! Validation for server configuration.
//!
//! [`validate`] is the single entry point that checks every network-facing field of a
//! [`ServerConfig`] against the shared validators in
//! [`crate::core::config_validation`]. It runs at the configuration boundary — after a
//! config file is parsed and after environment overrides are applied — so an invalid
//! host, port, CORS origin or upload limit is rejected before the server binds.

use super::ServerConfig;
use crate::Result;
use crate::XbergError;
use crate::core::config_validation::{validate_cors_origin, validate_host, validate_port, validate_upload_size};

/// Validate every network-facing field of a [`ServerConfig`].
///
/// # Errors
///
/// Returns [`crate::XbergError::Validation`] when `host` is neither an IP address nor a
/// hostname, when `port` is outside 1-65535, when any entry of `cors_origins` is neither
/// `*` nor an HTTP(S) URL, when either upload limit is zero, or when `job_timeout_secs` is
/// zero.
pub(super) fn validate(config: &ServerConfig) -> Result<()> {
    validate_host(&config.host)?;
    validate_port(u32::from(config.port))?;

    for origin in &config.cors_origins {
        validate_cors_origin(origin)?;
    }

    validate_upload_size(config.max_request_body_bytes)?;
    validate_upload_size(config.max_multipart_field_bytes)?;
    validate_job_timeout_secs(config.job_timeout_secs)?;

    Ok(())
}

/// Reject a zero job timeout: `Duration::from_secs(0)` expires immediately, so every async
/// job would fail before it could run.
fn validate_job_timeout_secs(job_timeout_secs: u64) -> Result<()> {
    if job_timeout_secs > 0 {
        Ok(())
    } else {
        Err(XbergError::Validation {
            message: format!(
                "Job timeout must be greater than 0, got {job_timeout_secs}. \
                 Set 'server.job_timeout_secs' (or XBERG_JOB_TIMEOUT_SECS) \
                 to a positive number of seconds such as 600 (10 minutes)."
            ),
            source: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::ServerConfig;

    /// Render a validation outcome as an exactly comparable string: the empty string for
    /// an accepted config, or the full `Display` text of the rejection.
    fn outcome(config: &ServerConfig) -> String {
        match config.validate() {
            Ok(()) => String::new(),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn should_accept_default_server_config() {
        assert_eq!(outcome(&ServerConfig::default()), "");
    }

    #[test]
    fn should_accept_config_with_wildcard_and_url_cors_origins() {
        let config = ServerConfig {
            cors_origins: vec!["*".to_string(), "https://example.com".to_string()],
            ..ServerConfig::default()
        };
        assert_eq!(outcome(&config), "");
    }

    #[test]
    fn should_reject_config_when_host_is_not_an_address_or_hostname() {
        let config = ServerConfig {
            host: "not a host".to_string(),
            ..ServerConfig::default()
        };
        assert_eq!(
            outcome(&config),
            "Validation error: Invalid host 'not a host': must be a valid IP address or hostname. \
             Set 'server.host' (or XBERG_HOST) to e.g. '127.0.0.1', '0.0.0.0' or 'localhost'."
        );
    }

    #[test]
    fn should_reject_config_when_port_is_zero() {
        let config = ServerConfig {
            port: 0,
            ..ServerConfig::default()
        };
        assert_eq!(
            outcome(&config),
            "Validation error: Port must be 1-65535, got 0. \
             Set 'server.port' (or XBERG_PORT) to a free port such as 8000."
        );
    }

    #[test]
    fn should_reject_config_when_a_cors_origin_has_no_scheme() {
        let config = ServerConfig {
            cors_origins: vec!["https://example.com".to_string(), "example.com".to_string()],
            ..ServerConfig::default()
        };
        assert_eq!(
            outcome(&config),
            "Validation error: Invalid CORS origin 'example.com': must be a valid HTTP/HTTPS URL or '*'. \
             Set 'server.cors_origins' (or XBERG_CORS_ORIGINS) to e.g. 'https://example.com'."
        );
    }

    #[test]
    fn should_reject_config_when_max_request_body_bytes_is_zero() {
        let config = ServerConfig {
            max_request_body_bytes: 0,
            ..ServerConfig::default()
        };
        assert_eq!(
            outcome(&config),
            "Validation error: Upload size must be greater than 0, got 0. \
             Set 'server.max_request_body_bytes' / 'server.max_multipart_field_bytes' \
             to a positive byte count such as 104857600 (100 MB)."
        );
    }

    #[test]
    fn should_reject_config_when_max_multipart_field_bytes_is_zero() {
        let config = ServerConfig {
            max_multipart_field_bytes: 0,
            ..ServerConfig::default()
        };
        assert_eq!(
            outcome(&config),
            "Validation error: Upload size must be greater than 0, got 0. \
             Set 'server.max_request_body_bytes' / 'server.max_multipart_field_bytes' \
             to a positive byte count such as 104857600 (100 MB)."
        );
    }

    #[test]
    fn should_reject_config_when_job_timeout_secs_is_zero() {
        let config = ServerConfig {
            job_timeout_secs: 0,
            ..ServerConfig::default()
        };
        assert_eq!(
            outcome(&config),
            "Validation error: Job timeout must be greater than 0, got 0. \
             Set 'server.job_timeout_secs' (or XBERG_JOB_TIMEOUT_SECS) \
             to a positive number of seconds such as 600 (10 minutes)."
        );
    }
}
