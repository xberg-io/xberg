//! API server configuration loading.

use crate::{Result, core::ServerConfig};

/// Load ServerConfig with proper precedence order.
///
/// This function implements the configuration hierarchy:
/// 1. File (if provided)
/// 2. Environment variables (via apply_env_overrides)
/// 3. Defaults
///
/// The config file can be in flat format (server settings at root) or nested format
/// (server settings under [server] section alongside other configs like [ocr]).
///
/// # Arguments
///
/// * `config_path` - Optional path to a ServerConfig file (TOML, YAML, or JSON)
///
/// # Returns
///
/// A configured ServerConfig with proper precedence applied.
///
/// # Errors
///
/// Returns an error if:
/// - The config file path is provided but cannot be read
/// - The config file contains invalid server configuration
/// - Environment variable overrides contain invalid values
///
/// # Examples
///
/// Not run as a doctest: `pub(crate)`, so it is unreachable from a downstream crate.
///
/// ```ignore
/// use xberg::api::load_server_config;
///
/// # fn example() -> xberg::Result<()> {
/// // Load from file with env overrides
/// let config = load_server_config(Some("server.toml"))?;
///
/// // Or use defaults with env overrides
/// let config = load_server_config(None)?;
/// # Ok(())
/// # }
/// ```
pub(crate) fn load_server_config(config_path: Option<&str>) -> Result<ServerConfig> {
    let mut config = if let Some(path) = config_path.map(std::path::Path::new) {
        ServerConfig::from_file(path)?
    } else {
        ServerConfig::default()
    };

    config.apply_env_overrides()?;

    tracing::info!(
        "Server configuration loaded: host={}, port={}, request_body_limit={} MB, multipart_field_limit={} MB, \
         job_timeout={}s, CORS={}",
        config.host,
        config.port,
        config.max_request_body_mb(),
        config.max_multipart_field_mb(),
        config.job_timeout_secs,
        if config.cors_allows_all() {
            "allow all origins".to_string()
        } else {
            format!("{} specific origins", config.cors_origins.len())
        }
    );

    Ok(config)
}
