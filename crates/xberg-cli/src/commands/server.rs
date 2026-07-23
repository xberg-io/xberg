//! Server command - Start API and MCP servers
//!
//! This module provides commands for starting the Xberg API server
//! and the MCP (Model Context Protocol) server.

use anyhow::Result;

/// Execute API server command
#[cfg(feature = "api")]
pub fn serve_command(
    cli_host: Option<String>,
    cli_port: Option<u16>,
    extraction_config: xberg::ExtractionConfig,
    config_path: Option<std::path::PathBuf>,
) -> Result<()> {
    use anyhow::Context;
    use xberg::ServerConfig;

    let mut server_config = if let Some(path) = &config_path {
        ServerConfig::from_file(path).with_context(|| {
            format!(
                "Failed to load server configuration from '{}'. \
                 Ensure the file contains valid server settings under [server] section or at root level.",
                path.display()
            )
        })?
    } else {
        ServerConfig::default()
    };

    server_config.apply_env_overrides()?;

    if let Some(host) = cli_host {
        server_config.host = host;
    }
    if let Some(port) = cli_port {
        server_config.port = port;
    }

    tracing::info!("Starting Xberg API server on http://{}", server_config.listen_addr());

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(xberg::api::serve_with_server_config(
        extraction_config,
        server_config.clone(),
    ))
    .with_context(|| {
        format!(
            "Failed to start API server on {}. Ensure the port is not already in use and you have permission to bind to this address.",
            server_config.listen_addr()
        )
    })?;

    Ok(())
}

/// Execute MCP server command
#[cfg(feature = "mcp")]
pub fn mcp_command(
    config: xberg::ExtractionConfig,
    transport: String,
    #[cfg(feature = "mcp-http")] host: String,
    #[cfg(feature = "mcp-http")] port: u16,
    #[cfg(feature = "mcp-http")] allowed_hosts: Vec<String>,
    #[cfg(not(feature = "mcp-http"))] _host: String,
    #[cfg(not(feature = "mcp-http"))] _port: u16,
    #[cfg(not(feature = "mcp-http"))] _allowed_hosts: Vec<String>,
) -> Result<()> {
    tracing::debug!("Starting Xberg MCP server with transport: {}", transport);
    let rt = tokio::runtime::Runtime::new()?;

    match transport.to_lowercase().as_str() {
        "stdio" => {
            rt.block_on(xberg::mcp::start_mcp_server_with_config(config))
                .map_err(|e| anyhow::anyhow!("Failed to start MCP server: {}", e))?;
        }
        "http" => {
            #[cfg(not(feature = "mcp-http"))]
            {
                anyhow::bail!(
                    "HTTP transport requires 'mcp-http' feature. \
                     Rebuild with: cargo build --features mcp-http"
                );
            }

            #[cfg(feature = "mcp-http")]
            {
                tracing::debug!("Starting MCP server on http://{}:{}", host, port);
                rt.block_on(xberg::mcp::start_mcp_server_http_with_config(
                    &host,
                    port,
                    config,
                    &allowed_hosts,
                ))
                .map_err(|e| anyhow::anyhow!("Failed to start MCP server on {}:{}: {}", host, port, e))?;
            }
        }
        other => {
            anyhow::bail!("Unknown transport '{}'. Use 'stdio' or 'http'", other);
        }
    }

    Ok(())
}

/// Resolve the MCP HTTP transport's extra `allowed_hosts` using the precedence cascade:
/// CLI flag > `XBERG_MCP_ALLOWED_HOSTS` env var > `[mcp] allowed_hosts` config-file key >
/// default (rmcp's loopback-only allowlist, unchanged).
///
/// The config-file tier is only consulted when `config_path` is `Some` (an explicit
/// `--config` flag); auto-discovered config files are not searched for this key, since
/// the CLI's discovery helper does not currently surface the discovered path.
///
/// # Errors
///
/// Returns an error if `config_path` is set but the file cannot be read or parsed.
#[cfg(feature = "mcp")]
pub fn resolve_mcp_allowed_hosts(cli_hosts: &[String], config_path: Option<&std::path::Path>) -> Result<Vec<String>> {
    use anyhow::Context;

    let env_value = std::env::var(xberg::mcp::MCP_ALLOWED_HOSTS_ENV).ok();
    let config_hosts = match config_path {
        Some(path) => xberg::mcp::read_mcp_allowed_hosts_from_file(path)
            .with_context(|| format!("Failed to read MCP allowed_hosts from config file '{}'", path.display()))?,
        None => Vec::new(),
    };

    Ok(xberg::mcp::resolve_extra_allowed_hosts(
        cli_hosts,
        env_value.as_deref(),
        &config_hosts,
    ))
}
