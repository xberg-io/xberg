//! API server setup and configuration.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

use crate::{ExtractionConfig, Result, core::ServerConfig};

use super::{
    handlers::{
        cache_clear_handler, cache_stats_handler, chunk_handler, embed_handler, extract_handler, health_handler,
        info_handler,
    },
    types::{ApiSizeLimits, ApiState},
};

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
/// ```no_run
/// use kreuzberg::api::load_server_config;
/// use std::path::Path;
///
/// # fn example() -> kreuzberg::Result<()> {
/// // Load from file with env overrides
/// let config = load_server_config(Some(Path::new("server.toml")))?;
///
/// // Or use defaults with env overrides
/// let config = load_server_config(None)?;
/// # Ok(())
/// # }
/// ```
pub fn load_server_config(config_path: Option<&std::path::Path>) -> Result<ServerConfig> {
    let mut config = if let Some(path) = config_path {
        ServerConfig::from_file(path)?
    } else {
        ServerConfig::default()
    };

    // Apply environment variable overrides with proper logging
    config.apply_env_overrides()?;

    tracing::info!(
        "Server configuration loaded: host={}, port={}, request_body_limit={} MB, multipart_field_limit={} MB, CORS={}",
        config.host,
        config.port,
        config.max_request_body_mb(),
        config.max_multipart_field_mb(),
        if config.cors_allows_all() {
            "allow all origins".to_string()
        } else {
            format!("{} specific origins", config.cors_origins.len())
        }
    );

    Ok(config)
}

/// Create the API router with all routes configured.
///
/// This is public to allow users to embed the router in their own applications.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
///
/// # Examples
///
/// ```no_run
/// use kreuzberg::{ExtractionConfig, api::create_router};
///
/// # #[tokio::main]
/// # async fn main() {
/// // Create router with default config and size limits
/// let config = ExtractionConfig::default();
/// let router = create_router(config);
/// # }
/// ```
pub fn create_router(config: ExtractionConfig) -> Router {
    create_router_with_limits(config, ApiSizeLimits::default())
}

/// Create the API router with custom size limits.
///
/// This allows fine-grained control over request body and multipart field size limits.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
/// * `limits` - Size limits for request bodies and multipart uploads.
///
/// # Examples
///
/// ```no_run
/// use kreuzberg::{ExtractionConfig, api::{create_router_with_limits, ApiSizeLimits}};
///
/// # #[tokio::main]
/// # async fn main() {
/// // Create router with 50 MB limits
/// let config = ExtractionConfig::default();
/// let limits = ApiSizeLimits::from_mb(50, 50);
/// let router = create_router_with_limits(config, limits);
/// # }
/// ```
///
/// ```no_run
/// use kreuzberg::{ExtractionConfig, api::{create_router_with_limits, ApiSizeLimits}};
/// use tower_http::limit::RequestBodyLimitLayer;
///
/// # #[tokio::main]
/// # async fn main() {
/// // Custom limits for very large documents (500 MB)
/// let config = ExtractionConfig::default();
/// let limits = ApiSizeLimits::from_mb(500, 500);
/// let router = create_router_with_limits(config, limits);
/// # }
/// ```
pub fn create_router_with_limits(config: ExtractionConfig, limits: ApiSizeLimits) -> Router {
    create_router_with_limits_and_server_config(config, limits, ServerConfig::default())
}

/// Create the API router with custom size limits and server configuration.
///
/// This function provides full control over request limits, CORS, and server settings via ServerConfig.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
/// * `limits` - Size limits for request bodies and multipart uploads.
/// * `server_config` - Server configuration including host, port, and CORS settings.
///
/// # Examples
///
/// ```no_run
/// use kreuzberg::{ExtractionConfig, api::create_router_with_limits, core::ServerConfig};
///
/// # #[tokio::main]
/// # async fn main() -> kreuzberg::Result<()> {
/// let extraction_config = ExtractionConfig::default();
/// let mut server_config = ServerConfig::default();
/// server_config.cors_origins = vec!["https://example.com".to_string()];
/// let router = create_router_with_limits_and_server_config(
///     extraction_config,
///     Default::default(),
///     server_config
/// );
/// # Ok(())
/// # }
/// ```
pub fn create_router_with_limits_and_server_config(
    config: ExtractionConfig,
    limits: ApiSizeLimits,
    server_config: ServerConfig,
) -> Router {
    let state = ApiState {
        default_config: Arc::new(config),
    };

    // CORS configuration based on ServerConfig
    let cors_layer = if server_config.cors_allows_all() {
        tracing::warn!(
            "CORS configured to allow all origins (default). This permits CSRF attacks. \
             For production, set KREUZBERG_CORS_ORIGINS environment variable to comma-separated \
             list of allowed origins (e.g., 'https://app.example.com,https://api.example.com')"
        );
        CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
    } else {
        let origins: Vec<_> = server_config
            .cors_origins
            .iter()
            .filter_map(|s| s.trim().parse::<axum::http::HeaderValue>().ok())
            .collect();

        if !origins.is_empty() {
            tracing::info!("CORS configured with {} explicit allowed origin(s)", origins.len());
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods(Any)
                .allow_headers(Any)
        } else {
            tracing::warn!(
                "CORS origins configured but empty/invalid - falling back to permissive CORS. \
                 This allows CSRF attacks. Set explicit origins for production."
            );
            CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
        }
    };

    Router::new()
        .route("/extract", post(extract_handler))
        .route("/embed", post(embed_handler))
        .route("/chunk", post(chunk_handler))
        .route("/health", get(health_handler))
        .route("/info", get(info_handler))
        .route("/cache/stats", get(cache_stats_handler))
        .route("/cache/clear", delete(cache_clear_handler))
        .layer(DefaultBodyLimit::max(limits.max_request_body_bytes))
        .layer(RequestBodyLimitLayer::new(limits.max_request_body_bytes))
        .layer(cors_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

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
    use std::net::IpAddr;

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
    use std::net::IpAddr;

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
pub async fn serve_default() -> Result<()> {
    serve("127.0.0.1", 8000).await
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn test_create_router() {
        let config = ExtractionConfig::default();
        let _router = create_router(config);
    }

    #[test]
    fn test_router_has_routes() {
        let config = ExtractionConfig::default();
        let router = create_router(config);
        assert!(size_of_val(&router) > 0);
    }

    #[test]
    fn test_create_router_with_limits() {
        let config = ExtractionConfig::default();
        let limits = ApiSizeLimits::from_mb(50, 50);
        let _router = create_router_with_limits(config, limits);
    }

    #[test]
    fn test_create_router_with_server_config() {
        let extraction_config = ExtractionConfig::default();
        let limits = ApiSizeLimits::from_mb(100, 100);
        let server_config = ServerConfig::default();
        let _router = create_router_with_limits_and_server_config(extraction_config, limits, server_config);
    }

    #[test]
    fn test_server_config_cors_handling() {
        let extraction_config = ExtractionConfig::default();
        let limits = ApiSizeLimits::default();
        let server_config = ServerConfig {
            cors_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };
        let _router = create_router_with_limits_and_server_config(extraction_config, limits, server_config);
    }
}
