//! Server command - Start API and MCP servers
//!
//! This module provides commands for starting the Kreuzberg API server
//! and the MCP (Model Context Protocol) server.

use anyhow::Result;

/// Execute API server command
#[cfg(feature = "api")]
pub fn serve_command(
    cli_host: Option<String>,
    cli_port: Option<u16>,
    extraction_config: kreuzberg::ExtractionConfig,
    config_path: Option<std::path::PathBuf>,
) -> Result<()> {
    use anyhow::Context;
    use kreuzberg::ServerConfig;

    // Load server config from same file or defaults
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

    // Apply environment variable overrides (precedence: env vars > config file)
    server_config.apply_env_overrides()?;

    // CLI args override everything (highest precedence)
    if let Some(host) = cli_host {
        server_config.host = host;
    }
    if let Some(port) = cli_port {
        server_config.port = port;
    }

    // Log the final configuration for debugging
    tracing::info!(
        "Starting Kreuzberg API server on http://{}",
        server_config.listen_addr()
    );

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(kreuzberg::api::serve_with_server_config(
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
    config: kreuzberg::ExtractionConfig,
    transport: String,
    #[cfg(feature = "mcp-http")] host: String,
    #[cfg(feature = "mcp-http")] port: u16,
    #[cfg(not(feature = "mcp-http"))] _host: String,
    #[cfg(not(feature = "mcp-http"))] _port: u16,
) -> Result<()> {
    tracing::debug!("Starting Kreuzberg MCP server with transport: {}", transport);
    let rt = tokio::runtime::Runtime::new()?;

    match transport.to_lowercase().as_str() {
        "stdio" => {
            rt.block_on(kreuzberg::mcp::start_mcp_server_with_config(config))
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
                rt.block_on(kreuzberg::mcp::start_mcp_server_http_with_config(&host, port, config))
                    .map_err(|e| anyhow::anyhow!("Failed to start MCP server on {}:{}: {}", host, port, e))?;
            }
        }
        other => {
            anyhow::bail!("Unknown transport '{}'. Use 'stdio' or 'http'", other);
        }
    }

    Ok(())
}
