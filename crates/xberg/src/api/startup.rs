//! API server startup functions.

use std::net::{IpAddr, SocketAddr};

use crate::{
    ExtractionConfig, Result, core::ServerConfig, extractors, plugins::startup_validation::validate_plugins_at_startup,
};

use super::{config::load_server_config, router::create_router_with_limits_and_server_config, types::ApiSizeLimits};

/// Wait for a shutdown signal: SIGTERM on Unix platforms or Ctrl-C on all platforms.
///
/// The future resolves as soon as the first signal arrives, allowing axum's
/// `with_graceful_shutdown` to drain in-flight connections before the process exits.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to install SIGTERM handler: {}", e);
                tokio::signal::ctrl_c()
                    .await
                    .unwrap_or_else(|e| tracing::warn!("Failed to listen for Ctrl-C: {}", e));
                tracing::info!("Shutting down gracefully on signal...");
                return;
            }
        };

        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("Shutting down gracefully on signal...");
            }
            result = tokio::signal::ctrl_c() => {
                if let Err(e) = result {
                    tracing::warn!("Failed to listen for Ctrl-C: {}", e);
                }
                tracing::info!("Shutting down gracefully on signal...");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .unwrap_or_else(|e| tracing::warn!("Failed to listen for Ctrl-C: {}", e));
        tracing::info!("Shutting down gracefully on signal...");
    }
}

/// Start the API server with config file discovery.
///
/// Searches for xberg.toml/yaml/yml/json in current and parent directories.
/// If no config file is found, uses default configuration.
///
/// # Arguments
///
/// * `host` - IP address to bind to (e.g., "127.0.0.1" or "0.0.0.0")
/// * `port` - Port number to bind to (e.g., 8000)
///
/// # Examples
///
/// ```no_run
/// use xberg::api::serve;
///
/// #[tokio::main]
/// async fn main() -> xberg::Result<()> {
///     // Local development
///     serve("127.0.0.1", 8000).await?;
///     Ok(())
/// }
/// ```
///
/// ```no_run
/// use xberg::api::serve;
///
/// #[tokio::main]
/// async fn main() -> xberg::Result<()> {
///     // Docker/production (listen on all interfaces)
///     serve("0.0.0.0", 8000).await?;
///     Ok(())
/// }
/// ```
///
/// # Environment Variables
///
/// ```bash
/// # Python/Docker usage
/// export XBERG_HOST=0.0.0.0
/// export XBERG_PORT=8000
///
/// # CORS configuration (IMPORTANT for production security)
/// # Default: allows all origins (permits CSRF attacks)
/// # Production: set to comma-separated list of allowed origins
/// export XBERG_CORS_ORIGINS="https://app.example.com,https://api.example.com"
///
/// # Upload size limits (default: 100 MB)
/// # Modern approach (in bytes):
/// export XBERG_MAX_REQUEST_BODY_BYTES=104857600       # 100 MB
/// export XBERG_MAX_MULTIPART_FIELD_BYTES=104857600    # 100 MB per file
///
/// python -m xberg.api
/// ```
#[cfg_attr(alef, alef(skip))]
pub async fn serve(host: impl AsRef<str>, port: u16) -> Result<()> {
    let extraction_config = match ExtractionConfig::discover()? {
        Some(config) => {
            tracing::info!("Loaded extraction config from discovered file");
            config
        }
        None => {
            tracing::info!("No config file found, using default configuration");
            ExtractionConfig::default()
        }
    };

    let server_config = load_server_config(None)?;
    let limits = ApiSizeLimits::new(
        server_config.max_request_body_bytes,
        server_config.max_multipart_field_bytes,
    );

    extractors::ensure_initialized()?;
    validate_plugins_at_startup()?;

    serve_with_config_and_limits(host, port, extraction_config, limits).await
}

/// Start the API server with explicit config.
///
/// Uses default size limits (100 MB). For custom limits, use `serve_with_config_and_limits`.
///
/// # Arguments
///
/// * `host` - IP address to bind to (e.g., "127.0.0.1" or "0.0.0.0")
/// * `port` - Port number to bind to (e.g., 8000)
/// * `config` - Default extraction configuration for all requests
///
/// # Examples
///
/// ```no_run
/// use xberg::{ExtractionConfig, api::serve_with_config};
///
/// #[tokio::main]
/// async fn main() -> xberg::Result<()> {
///     let config = ExtractionConfig::from_toml_file("config/xberg.toml")?;
///     serve_with_config("127.0.0.1", 8000, config).await?;
///     Ok(())
/// }
/// ```
#[cfg_attr(alef, alef(skip))]
pub async fn serve_with_config(host: impl AsRef<str>, port: u16, config: ExtractionConfig) -> Result<()> {
    let limits = ApiSizeLimits::default();
    tracing::info!(
        "Upload size limit: 100 MB (default, {} bytes)",
        limits.max_request_body_bytes
    );

    extractors::ensure_initialized()?;
    validate_plugins_at_startup()?;

    serve_with_config_and_limits(host, port, config, limits).await
}

/// Start the API server with explicit config and size limits.
///
/// # Arguments
///
/// * `host` - IP address to bind to (e.g., "127.0.0.1" or "0.0.0.0")
/// * `port` - Port number to bind to (e.g., 8000)
/// * `config` - Default extraction configuration for all requests
/// * `limits` - Size limits for request bodies and multipart uploads
///
/// # Examples
///
/// ```ignore
/// use xberg::{ExtractionConfig, api::{serve_with_config_and_limits, ApiSizeLimits}};
///
/// #[tokio::main]
/// async fn main() -> xberg::Result<()> {
///     let config = ExtractionConfig::from_toml_file("config/xberg.toml")?;
///     let limits = ApiSizeLimits::from_mb(200, 200);
///     serve_with_config_and_limits("127.0.0.1", 8000, config, limits).await?;
///     Ok(())
/// }
/// ```
#[cfg_attr(alef, alef(skip))]
pub async fn serve_with_config_and_limits(
    host: impl AsRef<str>,
    port: u16,
    config: ExtractionConfig,
    limits: ApiSizeLimits,
) -> Result<()> {
    let ip: IpAddr = host
        .as_ref()
        .parse()
        .map_err(|e| crate::error::XbergError::validation(format!("Invalid host address: {}", e)))?;

    let server_config = ServerConfig {
        host: host.as_ref().to_string(),
        port,
        max_request_body_bytes: limits.max_request_body_bytes,
        max_multipart_field_bytes: limits.max_multipart_field_bytes,
        ..Default::default()
    };

    let addr = SocketAddr::new(ip, port);
    let app = create_router_with_limits_and_server_config(config, limits, server_config);

    extractors::ensure_initialized()?;
    validate_plugins_at_startup()?;

    tracing::info!("Starting Xberg API server on http://{}:{}", ip, port);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(crate::XbergError::from)?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| crate::error::XbergError::Other(e.to_string()))?;

    Ok(())
}

/// Start the API server with explicit extraction config and server config.
///
/// This function accepts a fully-configured ServerConfig, including CORS origins,
/// size limits, host, and port. It respects all ServerConfig fields without
/// re-parsing environment variables, making it ideal for CLI usage where
/// configuration precedence has already been applied.
///
/// # Arguments
///
/// * `extraction_config` - Default extraction configuration for all requests
/// * `server_config` - Server configuration including host, port, CORS, and size limits
///
/// # Examples
///
/// ```no_run
/// use xberg::{ExtractionConfig, api::serve_with_server_config, core::ServerConfig};
///
/// #[tokio::main]
/// async fn main() -> xberg::Result<()> {
///     let extraction_config = ExtractionConfig::default();
///     let mut server_config = ServerConfig::default();
///     server_config.host = "0.0.0.0".to_string();
///     server_config.port = 3000;
///     server_config.cors_origins = vec!["https://example.com".to_string()];
///
///     serve_with_server_config(extraction_config, server_config).await?;
///     Ok(())
/// }
/// ```
#[cfg_attr(alef, alef(skip))]
pub async fn serve_with_server_config(extraction_config: ExtractionConfig, server_config: ServerConfig) -> Result<()> {
    let ip: IpAddr = server_config
        .host
        .parse()
        .map_err(|e| crate::error::XbergError::validation(format!("Invalid host address: {}", e)))?;

    let limits = ApiSizeLimits::new(
        server_config.max_request_body_bytes,
        server_config.max_multipart_field_bytes,
    );

    let addr = SocketAddr::new(ip, server_config.port);
    let app = create_router_with_limits_and_server_config(extraction_config, limits, server_config.clone());

    extractors::ensure_initialized()?;
    validate_plugins_at_startup()?;

    tracing::info!(
        "Starting Xberg API server on http://{}:{} (request_body_limit={} MB, multipart_field_limit={} MB)",
        ip,
        server_config.port,
        server_config.max_request_body_mb(),
        server_config.max_multipart_field_mb()
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(crate::XbergError::from)?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| crate::error::XbergError::Other(e.to_string()))?;

    Ok(())
}

/// Start the API server with default host and port.
///
/// Defaults: host = "127.0.0.1", port = 8000
///
/// Uses config file discovery (searches current/parent directories for xberg.toml/yaml/yml/json).
/// Validates plugins at startup to help diagnose configuration issues.
#[cfg_attr(alef, alef(skip))]
pub async fn serve_default() -> Result<()> {
    serve("127.0.0.1", 8000).await
}
