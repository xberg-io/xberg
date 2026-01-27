//! API server startup functions.

use std::net::{IpAddr, SocketAddr};

use crate::{ExtractionConfig, Result, core::ServerConfig, plugins::startup_validation::validate_plugins_at_startup};

use super::{config::load_server_config, router::create_router_with_limits_and_server_config, types::ApiSizeLimits};

/// Start the API server with config file discovery.
///
/// Searches for kreuzberg.toml/yaml/json in current and parent directories.
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
/// use kreuzberg::api::serve;
///
/// #[tokio::main]
/// async fn main() -> kreuzberg::Result<()> {
///     // Local development
///     serve("127.0.0.1", 8000).await?;
///     Ok(())
/// }
/// ```
///
/// ```no_run
/// use kreuzberg::api::serve;
///
/// #[tokio::main]
/// async fn main() -> kreuzberg::Result<()> {
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
/// export KREUZBERG_HOST=0.0.0.0
/// export KREUZBERG_PORT=8000
///
/// # CORS configuration (IMPORTANT for production security)
/// # Default: allows all origins (permits CSRF attacks)
/// # Production: set to comma-separated list of allowed origins
/// export KREUZBERG_CORS_ORIGINS="https://app.example.com,https://api.example.com"
///
/// # Upload size limits (default: 100 MB)
/// # Modern approach (in bytes):
/// export KREUZBERG_MAX_REQUEST_BODY_BYTES=104857600       # 100 MB
/// export KREUZBERG_MAX_MULTIPART_FIELD_BYTES=104857600    # 100 MB per file
///
/// # Legacy approach (in MB, applies to both limits):
/// export KREUZBERG_MAX_UPLOAD_SIZE_MB=100  # 100 MB
///
/// python -m kreuzberg.api
/// ```
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

    // Validate plugins at startup
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
/// use kreuzberg::{ExtractionConfig, api::serve_with_config};
///
/// #[tokio::main]
/// async fn main() -> kreuzberg::Result<()> {
///     let config = ExtractionConfig::from_toml_file("config/kreuzberg.toml")?;
///     serve_with_config("127.0.0.1", 8000, config).await?;
///     Ok(())
/// }
/// ```
pub async fn serve_with_config(host: impl AsRef<str>, port: u16, config: ExtractionConfig) -> Result<()> {
    let limits = ApiSizeLimits::default();
    tracing::info!(
        "Upload size limit: 100 MB (default, {} bytes)",
        limits.max_request_body_bytes
    );

    // Validate plugins at startup
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
/// ```no_run
/// use kreuzberg::{ExtractionConfig, api::{serve_with_config_and_limits, ApiSizeLimits}};
///
/// #[tokio::main]
/// async fn main() -> kreuzberg::Result<()> {
///     let config = ExtractionConfig::from_toml_file("config/kreuzberg.toml")?;
///     let limits = ApiSizeLimits::from_mb(200, 200);
///     serve_with_config_and_limits("127.0.0.1", 8000, config, limits).await?;
///     Ok(())
/// }
/// ```
pub async fn serve_with_config_and_limits(
    host: impl AsRef<str>,
    port: u16,
    config: ExtractionConfig,
    limits: ApiSizeLimits,
) -> Result<()> {
    let ip: IpAddr = host
        .as_ref()
        .parse()
        .map_err(|e| crate::error::KreuzbergError::validation(format!("Invalid host address: {}", e)))?;

    let server_config = ServerConfig {
        host: host.as_ref().to_string(),
        port,
        max_request_body_bytes: limits.max_request_body_bytes,
        max_multipart_field_bytes: limits.max_multipart_field_bytes,
        ..Default::default()
    };

    let addr = SocketAddr::new(ip, port);
    let app = create_router_with_limits_and_server_config(config, limits, server_config);

    // Validate plugins at startup
    validate_plugins_at_startup()?;

    tracing::info!("Starting Kreuzberg API server on http://{}:{}", ip, port);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(crate::error::KreuzbergError::Io)?;

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::KreuzbergError::Other(e.to_string()))?;

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
/// use kreuzberg::{ExtractionConfig, api::serve_with_server_config, core::ServerConfig};
///
/// #[tokio::main]
/// async fn main() -> kreuzberg::Result<()> {
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
pub async fn serve_with_server_config(extraction_config: ExtractionConfig, server_config: ServerConfig) -> Result<()> {
    let ip: IpAddr = server_config
        .host
        .parse()
        .map_err(|e| crate::error::KreuzbergError::validation(format!("Invalid host address: {}", e)))?;

    let limits = ApiSizeLimits::new(
        server_config.max_request_body_bytes,
        server_config.max_multipart_field_bytes,
    );

    let addr = SocketAddr::new(ip, server_config.port);
    let app = create_router_with_limits_and_server_config(extraction_config, limits, server_config.clone());

    // Validate plugins at startup
    validate_plugins_at_startup()?;

    tracing::info!(
        "Starting Kreuzberg API server on http://{}:{} (request_body_limit={} MB, multipart_field_limit={} MB)",
        ip,
        server_config.port,
        server_config.max_request_body_mb(),
        server_config.max_multipart_field_mb()
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(crate::error::KreuzbergError::Io)?;

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::KreuzbergError::Other(e.to_string()))?;

    Ok(())
}

/// Start the API server with default host and port.
///
/// Defaults: host = "127.0.0.1", port = 8000
///
/// Uses config file discovery (searches current/parent directories for kreuzberg.toml/yaml/json).
/// Validates plugins at startup to help diagnose configuration issues.
pub async fn serve_default() -> Result<()> {
    serve("127.0.0.1", 8000).await
}
