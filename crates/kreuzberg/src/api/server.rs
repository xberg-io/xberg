//! API server setup and configuration.

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

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

use crate::{ExtractionConfig, Result};

use super::{
    handlers::{cache_clear_handler, cache_stats_handler, extract_handler, health_handler, info_handler},
    types::{ApiSizeLimits, ApiState},
};

/// Parse size limits from environment variables.
///
/// Reads environment variables in the following order of preference:
/// 1. `KREUZBERG_MAX_REQUEST_BODY_BYTES` - Maximum total request body size (in bytes)
/// 2. `KREUZBERG_MAX_MULTIPART_FIELD_BYTES` - Maximum individual multipart field size (in bytes)
/// 3. `KREUZBERG_MAX_UPLOAD_SIZE_MB` - (Legacy) Maximum upload size in MB (applies to both limits)
///
/// Falls back to default (100 MB) if not set or invalid.
///
/// # Examples
///
/// ```bash
/// # Method 1: Set limits in bytes (e.g., 500 MB)
/// export KREUZBERG_MAX_REQUEST_BODY_BYTES=524288000  # 500 MB
/// export KREUZBERG_MAX_MULTIPART_FIELD_BYTES=524288000  # 500 MB
///
/// # Method 2: Set limits in MB (legacy, backward compatible)
/// export KREUZBERG_MAX_UPLOAD_SIZE_MB=500
/// ```
fn parse_size_limits_from_env() -> ApiSizeLimits {
    const DEFAULT_100MB_MB: usize = 100;

    if let Ok(value) = std::env::var("KREUZBERG_MAX_REQUEST_BODY_BYTES") {
        if let Ok(bytes) = value.parse::<usize>() {
            if bytes > 0 {
                let multipart_bytes = std::env::var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(bytes);

                tracing::info!(
                    "Upload size limits configured from environment: request_body={} bytes ({:.1} GB), multipart_field={} bytes ({:.1} GB)",
                    bytes,
                    bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                    multipart_bytes,
                    multipart_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
                );

                return ApiSizeLimits::new(bytes, multipart_bytes);
            }
        } else {
            tracing::warn!(
                "Failed to parse KREUZBERG_MAX_REQUEST_BODY_BYTES='{}', must be a valid usize",
                value
            );
        }
    }

    if let Ok(value) = std::env::var("KREUZBERG_MAX_UPLOAD_SIZE_MB") {
        if let Ok(mb) = value.parse::<usize>() {
            if mb > 0 {
                tracing::info!(
                    "Upload size limit configured from environment (legacy): {} MB ({} bytes)",
                    mb,
                    mb * 1024 * 1024
                );
                return ApiSizeLimits::from_mb(mb, mb);
            } else {
                tracing::warn!("Invalid KREUZBERG_MAX_UPLOAD_SIZE_MB value (must be > 0)");
            }
        } else {
            tracing::warn!(
                "Failed to parse KREUZBERG_MAX_UPLOAD_SIZE_MB='{}', must be a valid usize",
                value
            );
        }
    }

    let limits = ApiSizeLimits::from_mb(DEFAULT_100MB_MB, DEFAULT_100MB_MB);
    tracing::info!(
        "Upload size limit: 100 MB (default, {} bytes) - Configure with KREUZBERG_MAX_REQUEST_BODY_BYTES or KREUZBERG_MAX_UPLOAD_SIZE_MB",
        limits.max_request_body_bytes
    );
    limits
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
    let state = ApiState {
        default_config: Arc::new(config),
    };

    // SECURITY WARNING: The default allows all origins for development convenience,
    let cors_layer = if let Ok(origins_str) = std::env::var("KREUZBERG_CORS_ORIGINS") {
        let origins: Vec<_> = origins_str
            .split(',')
            .filter(|s| !s.trim().is_empty())
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
                "KREUZBERG_CORS_ORIGINS set but empty/invalid - falling back to permissive CORS. \
                 This allows CSRF attacks. Set explicit origins for production."
            );
            CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
        }
    } else {
        tracing::warn!(
            "CORS configured to allow all origins (default). This permits CSRF attacks. \
             For production, set KREUZBERG_CORS_ORIGINS environment variable to comma-separated \
             list of allowed origins (e.g., 'https://app.example.com,https://api.example.com')"
        );
        CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
    };

    Router::new()
        .route("/extract", post(extract_handler))
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
    let config = match ExtractionConfig::discover()? {
        Some(config) => {
            tracing::info!("Loaded extraction config from discovered file");
            config
        }
        None => {
            tracing::info!("No config file found, using default configuration");
            ExtractionConfig::default()
        }
    };

    let limits = parse_size_limits_from_env();

    serve_with_config_and_limits(host, port, config, limits).await
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
    let ip: IpAddr = host
        .as_ref()
        .parse()
        .map_err(|e| crate::error::KreuzbergError::validation(format!("Invalid host address: {}", e)))?;

    let addr = SocketAddr::new(ip, port);
    let app = create_router_with_limits(config, limits);

    tracing::info!("Starting Kreuzberg API server on http://{}:{}", ip, port);

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
    #[serial_test::serial]
    fn test_parse_size_limits_default_100mb() {
        unsafe {
            std::env::remove_var("KREUZBERG_MAX_REQUEST_BODY_BYTES");
            std::env::remove_var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES");
            std::env::remove_var("KREUZBERG_MAX_UPLOAD_SIZE_MB");
        }

        let limits = parse_size_limits_from_env();
        assert_eq!(limits.max_request_body_bytes, 100 * 1024 * 1024);
        assert_eq!(limits.max_multipart_field_bytes, 100 * 1024 * 1024);
    }

    #[test]
    #[serial_test::serial]
    fn test_parse_size_limits_from_bytes_env_vars() {
        unsafe {
            std::env::set_var("KREUZBERG_MAX_REQUEST_BODY_BYTES", "5368709120");
            std::env::set_var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES", "2684354560");
            std::env::remove_var("KREUZBERG_MAX_UPLOAD_SIZE_MB");
        }

        let limits = parse_size_limits_from_env();
        assert_eq!(limits.max_request_body_bytes, 5 * 1024 * 1024 * 1024);
        assert_eq!(limits.max_multipart_field_bytes, 2_684_354_560);

        unsafe {
            std::env::remove_var("KREUZBERG_MAX_REQUEST_BODY_BYTES");
            std::env::remove_var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_parse_size_limits_bytes_env_var_only() {
        unsafe {
            std::env::set_var("KREUZBERG_MAX_REQUEST_BODY_BYTES", "1073741824");
            std::env::remove_var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES");
            std::env::remove_var("KREUZBERG_MAX_UPLOAD_SIZE_MB");
        }

        let limits = parse_size_limits_from_env();
        assert_eq!(limits.max_request_body_bytes, 1 * 1024 * 1024 * 1024);
        assert_eq!(limits.max_multipart_field_bytes, 1 * 1024 * 1024 * 1024);

        unsafe {
            std::env::remove_var("KREUZBERG_MAX_REQUEST_BODY_BYTES");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_parse_size_limits_from_legacy_mb_env_var() {
        unsafe {
            std::env::remove_var("KREUZBERG_MAX_REQUEST_BODY_BYTES");
            std::env::remove_var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES");
            std::env::set_var("KREUZBERG_MAX_UPLOAD_SIZE_MB", "5000");
        }

        let limits = parse_size_limits_from_env();
        assert_eq!(limits.max_request_body_bytes, 5000 * 1024 * 1024);
        assert_eq!(limits.max_multipart_field_bytes, 5000 * 1024 * 1024);

        unsafe {
            std::env::remove_var("KREUZBERG_MAX_UPLOAD_SIZE_MB");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_parse_size_limits_invalid_bytes_env_var() {
        unsafe {
            std::env::set_var("KREUZBERG_MAX_REQUEST_BODY_BYTES", "not a number");
            std::env::remove_var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES");
            std::env::remove_var("KREUZBERG_MAX_UPLOAD_SIZE_MB");
        }

        let limits = parse_size_limits_from_env();
        assert_eq!(limits.max_request_body_bytes, 100 * 1024 * 1024);

        unsafe {
            std::env::remove_var("KREUZBERG_MAX_REQUEST_BODY_BYTES");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_parse_size_limits_zero_bytes_env_var() {
        unsafe {
            std::env::set_var("KREUZBERG_MAX_REQUEST_BODY_BYTES", "0");
            std::env::remove_var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES");
            std::env::remove_var("KREUZBERG_MAX_UPLOAD_SIZE_MB");
        }

        let limits = parse_size_limits_from_env();
        assert_eq!(limits.max_request_body_bytes, 100 * 1024 * 1024);

        unsafe {
            std::env::remove_var("KREUZBERG_MAX_REQUEST_BODY_BYTES");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_parse_size_limits_bytes_env_var_precedence() {
        unsafe {
            std::env::set_var("KREUZBERG_MAX_REQUEST_BODY_BYTES", "1073741824");
            std::env::remove_var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES");
            std::env::set_var("KREUZBERG_MAX_UPLOAD_SIZE_MB", "5000");
        }

        let limits = parse_size_limits_from_env();
        assert_eq!(limits.max_request_body_bytes, 1 * 1024 * 1024 * 1024);
        assert_ne!(limits.max_request_body_bytes, 5000 * 1024 * 1024);

        unsafe {
            std::env::remove_var("KREUZBERG_MAX_REQUEST_BODY_BYTES");
            std::env::remove_var("KREUZBERG_MAX_UPLOAD_SIZE_MB");
        }
    }
}
