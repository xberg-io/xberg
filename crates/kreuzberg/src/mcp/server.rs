//! Kreuzberg MCP server implementation.
//!
//! This module provides the main MCP server struct and startup functions.

use crate::ExtractionConfig;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
};

#[cfg(feature = "mcp-http")]
use rmcp::transport::streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager};

/// Kreuzberg MCP server.
///
/// Provides document extraction capabilities via MCP tools.
///
/// The server loads a default extraction configuration from kreuzberg.toml/yaml/json
/// via discovery. Per-request OCR settings override the defaults.
#[derive(Clone)]
pub struct KreuzbergMcp {
    tool_router: ToolRouter<KreuzbergMcp>,
    /// Default extraction configuration loaded from config file via discovery
    default_config: std::sync::Arc<ExtractionConfig>,
}

#[tool_router]
impl KreuzbergMcp {
    /// Create a new Kreuzberg MCP server instance with default config.
    ///
    /// Uses `ExtractionConfig::discover()` to search for kreuzberg.toml/yaml/json
    /// in current and parent directories. Falls back to default configuration if
    /// no config file is found.
    #[allow(clippy::manual_unwrap_or_default)]
    pub fn new() -> crate::Result<Self> {
        let config = match ExtractionConfig::discover()? {
            Some(config) => {
                #[cfg(feature = "api")]
                tracing::info!("Loaded extraction config from discovered file");
                config
            }
            None => {
                #[cfg(feature = "api")]
                tracing::info!("No config file found, using default configuration");
                ExtractionConfig::default()
            }
        };

        Ok(Self::with_config(config))
    }

    /// Create a new Kreuzberg MCP server instance with explicit config.
    ///
    /// # Arguments
    ///
    /// * `config` - Default extraction configuration for all tool calls
    pub fn with_config(config: ExtractionConfig) -> Self {
        Self {
            tool_router: Self::tool_router(),
            default_config: std::sync::Arc::new(config),
        }
    }

    /// Extract content from a file.
    ///
    /// This tool extracts text, metadata, and tables from documents in various formats
    /// including PDFs, Word documents, Excel spreadsheets, images (with OCR), and more.
    ///
    /// Note: The `async` parameter is accepted for API compatibility but ignored.
    /// Extraction always runs asynchronously since the MCP server operates within
    /// a Tokio runtime. Using sync wrappers would cause a nested runtime panic.
    #[tool(
        description = "Extract content from a file by path. Supports PDFs, Word, Excel, images (with OCR), HTML, and more.",
        annotations(title = "Extract File", read_only_hint = true, idempotent_hint = true)
    )]
    async fn extract_file(
        &self,
        Parameters(params): Parameters<super::params::ExtractFileParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_kreuzberg_error_to_mcp;
        use super::format::{build_config, format_extraction_result};
        use crate::extract_file;

        let config =
            build_config(&self.default_config, params.config).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;

        // Always use async extraction - we're already in a Tokio runtime context.
        // Calling sync wrappers (which use GLOBAL_RUNTIME.block_on()) from within
        // an async context causes "Cannot start a runtime from within a runtime" panic.
        let result = extract_file(&params.path, params.mime_type.as_deref(), &config)
            .await
            .map_err(map_kreuzberg_error_to_mcp)?;

        let response = format_extraction_result(&result);
        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Extract content from base64-encoded bytes.
    ///
    /// This tool extracts text, metadata, and tables from base64-encoded document data.
    ///
    /// Note: The `async` parameter is accepted for API compatibility but ignored.
    /// Extraction always runs asynchronously since the MCP server operates within
    /// a Tokio runtime. Using sync wrappers would cause a nested runtime panic.
    #[tool(
        description = "Extract content from base64-encoded file data. Returns extracted text, metadata, and tables.",
        annotations(title = "Extract Bytes", read_only_hint = true, idempotent_hint = true)
    )]
    async fn extract_bytes(
        &self,
        Parameters(params): Parameters<super::params::ExtractBytesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_kreuzberg_error_to_mcp;
        use super::format::{build_config, format_extraction_result};
        use crate::extract_bytes;
        use base64::prelude::*;

        let bytes = BASE64_STANDARD
            .decode(&params.data)
            .map_err(|e| rmcp::ErrorData::invalid_params(format!("Invalid base64: {}", e), None))?;

        let config =
            build_config(&self.default_config, params.config).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;

        let mime_type = params.mime_type.as_deref().unwrap_or("");

        // Always use async extraction - we're already in a Tokio runtime context.
        let result = extract_bytes(&bytes, mime_type, &config)
            .await
            .map_err(map_kreuzberg_error_to_mcp)?;

        let response = format_extraction_result(&result);
        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Extract content from multiple files in parallel.
    ///
    /// This tool efficiently processes multiple documents simultaneously, useful for batch operations.
    ///
    /// Note: The `async` parameter is accepted for API compatibility but ignored.
    /// Extraction always runs asynchronously since the MCP server operates within
    /// a Tokio runtime. Using sync wrappers would cause a nested runtime panic.
    #[tool(
        description = "Extract content from multiple files in parallel. Returns results for all files.",
        annotations(title = "Batch Extract Files", read_only_hint = true, idempotent_hint = true)
    )]
    async fn batch_extract_files(
        &self,
        Parameters(params): Parameters<super::params::BatchExtractFilesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_kreuzberg_error_to_mcp;
        use super::format::build_config;
        use crate::batch_extract_file;

        let config =
            build_config(&self.default_config, params.config).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;

        // Always use async extraction - we're already in a Tokio runtime context.
        let results = batch_extract_file(params.paths.clone(), &config)
            .await
            .map_err(map_kreuzberg_error_to_mcp)?;

        let response = serde_json::to_string_pretty(&results).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Detect the MIME type of a file.
    ///
    /// This tool identifies the file format, useful for determining which extractor to use.
    #[tool(
        description = "Detect the MIME type of a file. Returns the detected MIME type string.",
        annotations(title = "Detect MIME Type", read_only_hint = true, idempotent_hint = true)
    )]
    fn detect_mime_type(
        &self,
        Parameters(params): Parameters<super::params::DetectMimeTypeParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_kreuzberg_error_to_mcp;
        use crate::detect_mime_type;

        let mime_type = detect_mime_type(&params.path, params.use_content).map_err(map_kreuzberg_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(mime_type)]))
    }

    /// Get cache statistics.
    ///
    /// This tool returns statistics about the cache including total files, size, and disk space.
    #[tool(
        description = "Get cache statistics including total files, size, and available disk space.",
        annotations(title = "Cache Stats", read_only_hint = true, idempotent_hint = true)
    )]
    fn cache_stats(
        &self,
        Parameters(_): Parameters<super::params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_kreuzberg_error_to_mcp;
        use crate::cache;

        let cache_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".kreuzberg");

        let stats = cache::get_cache_metadata(cache_dir.to_str().unwrap_or(".")).map_err(map_kreuzberg_error_to_mcp)?;

        let response = format!(
            "Cache Statistics\n\
             ================\n\
             Directory: {}\n\
             Total files: {}\n\
             Total size: {:.2} MB\n\
             Available space: {:.2} MB\n\
             Oldest file age: {:.2} days\n\
             Newest file age: {:.2} days",
            cache_dir.to_string_lossy(),
            stats.total_files,
            stats.total_size_mb,
            stats.available_space_mb,
            stats.oldest_file_age_days,
            stats.newest_file_age_days
        );

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// List all supported document formats.
    ///
    /// This tool returns all file extensions and MIME types that Kreuzberg can process.
    #[tool(
        description = "List all supported document formats with their file extensions and MIME types.",
        annotations(title = "List Formats", read_only_hint = true, idempotent_hint = true)
    )]
    fn list_formats(
        &self,
        Parameters(_): Parameters<super::params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let formats = crate::list_supported_formats();
        let response = serde_json::to_string_pretty(&formats).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Clear the cache.
    ///
    /// This tool removes all cached files and returns the number of files removed and space freed.
    #[tool(
        description = "Clear all cached files. Returns the number of files removed and space freed in MB.",
        annotations(title = "Clear Cache", destructive_hint = true)
    )]
    fn cache_clear(
        &self,
        Parameters(_): Parameters<super::params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_kreuzberg_error_to_mcp;
        use crate::cache;

        let cache_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".kreuzberg");

        let (removed_files, freed_mb) =
            cache::clear_cache_directory(cache_dir.to_str().unwrap_or(".")).map_err(map_kreuzberg_error_to_mcp)?;

        let response = format!(
            "Cache cleared successfully\n\
             Directory: {}\n\
             Removed files: {}\n\
             Freed space: {:.2} MB",
            cache_dir.to_string_lossy(),
            removed_files,
            freed_mb
        );

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }
}

#[tool_handler]
impl ServerHandler for KreuzbergMcp {
    fn get_info(&self) -> ServerInfo {
        let mut capabilities = ServerCapabilities::default();
        capabilities.tools = Some(ToolsCapability::default());

        let server_info = Implementation::new("kreuzberg-mcp", env!("CARGO_PKG_VERSION"))
            .with_title("Kreuzberg Document Intelligence MCP Server")
            .with_description(
                "Document intelligence library for extracting content from PDFs, images, office documents, and more.",
            )
            .with_website_url("https://kreuzberg-dev.github.io/kreuzberg/");

        InitializeResult::new(capabilities)
            .with_server_info(server_info)
            .with_instructions(
                "Extract content from documents in various formats. Supports PDFs, Word documents, \
                 Excel spreadsheets, images (with OCR), HTML, emails, and more. Use enable_ocr=true \
                 for scanned documents, force_ocr=true to always use OCR even if text extraction \
                 succeeds.",
            )
    }
}

impl Default for KreuzbergMcp {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            #[cfg(feature = "api")]
            tracing::warn!("Failed to discover config, using default: {}", e);
            #[cfg(not(feature = "api"))]
            tracing::debug!("Warning: Failed to discover config, using default: {}", e);
            Self::with_config(ExtractionConfig::default())
        })
    }
}

/// Start the Kreuzberg MCP server.
///
/// This function initializes and runs the MCP server using stdio transport.
/// It will block until the server is shut down.
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters a fatal error.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::mcp::start_mcp_server;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     start_mcp_server().await?;
///     Ok(())
/// }
/// ```
pub async fn start_mcp_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = KreuzbergMcp::new()?.serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

/// Start MCP server with custom extraction config.
///
/// This variant allows specifying a custom extraction configuration
/// (e.g., loaded from a file) instead of using defaults.
pub async fn start_mcp_server_with_config(
    config: ExtractionConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = KreuzbergMcp::with_config(config).serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

/// Start MCP server with HTTP Stream transport.
///
/// Uses rmcp's built-in StreamableHttpService for HTTP/SSE support per MCP spec.
///
/// # Arguments
///
/// * `host` - Host to bind to (e.g., "127.0.0.1" or "0.0.0.0")
/// * `port` - Port number (e.g., 8001)
///
/// # Example
///
/// ```no_run
/// use kreuzberg::mcp::start_mcp_server_http;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     start_mcp_server_http("127.0.0.1", 8001).await?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "mcp-http")]
pub async fn start_mcp_server_http(
    host: impl AsRef<str>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::Router;
    use std::net::SocketAddr;

    let http_service = StreamableHttpService::new(
        || KreuzbergMcp::new().map_err(|e| std::io::Error::other(e.to_string())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let router = Router::new().nest_service("/mcp", http_service);

    let addr: SocketAddr = format!("{}:{}", host.as_ref(), port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    #[cfg(feature = "api")]
    tracing::info!("Starting MCP HTTP server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

/// Start MCP HTTP server with custom extraction config.
///
/// This variant allows specifying a custom extraction configuration
/// while using HTTP Stream transport.
///
/// # Arguments
///
/// * `host` - Host to bind to (e.g., "127.0.0.1" or "0.0.0.0")
/// * `port` - Port number (e.g., 8001)
/// * `config` - Custom extraction configuration
///
/// # Example
///
/// ```no_run
/// use kreuzberg::mcp::start_mcp_server_http_with_config;
/// use kreuzberg::ExtractionConfig;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let config = ExtractionConfig::default();
///     start_mcp_server_http_with_config("127.0.0.1", 8001, config).await?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "mcp-http")]
pub async fn start_mcp_server_http_with_config(
    host: impl AsRef<str>,
    port: u16,
    config: ExtractionConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::Router;
    use std::net::SocketAddr;

    let http_service = StreamableHttpService::new(
        move || Ok(KreuzbergMcp::with_config(config.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let router = Router::new().nest_service("/mcp", http_service);

    let addr: SocketAddr = format!("{}:{}", host.as_ref(), port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    #[cfg(feature = "api")]
    tracing::info!("Starting MCP HTTP server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_router_has_routes() {
        let router = KreuzbergMcp::tool_router();
        assert!(router.has_route("extract_file"));
        assert!(router.has_route("extract_bytes"));
        assert!(router.has_route("batch_extract_files"));
        assert!(router.has_route("detect_mime_type"));
        assert!(router.has_route("list_formats"));
        assert!(router.has_route("cache_stats"));
        assert!(router.has_route("cache_clear"));

        let tools = router.list_all();
        assert_eq!(tools.len(), 7);
    }

    #[test]
    fn test_server_info() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.name, "kreuzberg-mcp");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_with_config_stores_provided_config() {
        let custom_config = ExtractionConfig {
            force_ocr: true,
            use_cache: false,
            ..Default::default()
        };

        let server = KreuzbergMcp::with_config(custom_config);

        assert!(server.default_config.force_ocr);
        assert!(!server.default_config.use_cache);
    }

    #[test]
    fn test_new_creates_server_with_default_config() {
        let server = KreuzbergMcp::new();
        assert!(server.is_ok());
    }

    #[test]
    fn test_default_creates_server_without_panic() {
        let server = KreuzbergMcp::default();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "kreuzberg-mcp");
    }

    #[test]
    fn test_server_info_has_correct_fields() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.name, "kreuzberg-mcp");
        assert_eq!(
            info.server_info.title,
            Some("Kreuzberg Document Intelligence MCP Server".to_string())
        );
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            info.server_info.website_url,
            Some("https://kreuzberg-dev.github.io/kreuzberg/".to_string())
        );
        assert!(info.instructions.is_some());
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_mcp_server_info_protocol_version() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.protocol_version, ProtocolVersion::default());
    }

    #[test]
    fn test_mcp_server_info_has_all_required_fields() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert!(!info.server_info.name.is_empty());
        assert!(!info.server_info.version.is_empty());

        assert!(info.server_info.title.is_some());
        assert!(info.server_info.website_url.is_some());
        assert!(info.instructions.is_some());
    }

    #[test]
    fn test_mcp_server_capabilities_declares_tools() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_mcp_server_name_follows_convention() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.name, "kreuzberg-mcp");
        assert!(!info.server_info.name.contains('_'));
        assert!(!info.server_info.name.contains(' '));
    }

    #[test]
    fn test_mcp_version_matches_cargo_version() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_mcp_instructions_are_helpful() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        let instructions = info.instructions.expect("Instructions should be present");

        assert!(instructions.contains("extract") || instructions.contains("Extract"));
        assert!(instructions.contains("OCR") || instructions.contains("ocr"));
        assert!(instructions.contains("document"));
    }

    #[tokio::test]
    async fn test_all_tools_are_registered() {
        let router = KreuzbergMcp::tool_router();

        let expected_tools = vec![
            "extract_file",
            "extract_bytes",
            "batch_extract_files",
            "detect_mime_type",
            "list_formats",
            "cache_stats",
            "cache_clear",
        ];

        for tool_name in expected_tools {
            assert!(router.has_route(tool_name), "Tool '{}' should be registered", tool_name);
        }
    }

    #[tokio::test]
    async fn test_tool_count_is_correct() {
        let router = KreuzbergMcp::tool_router();
        let tools = router.list_all();

        assert_eq!(tools.len(), 7, "Expected 7 tools, found {}", tools.len());
    }

    #[tokio::test]
    async fn test_tools_have_descriptions() {
        let router = KreuzbergMcp::tool_router();
        let tools = router.list_all();

        for tool in tools {
            assert!(
                tool.description.is_some(),
                "Tool '{}' should have a description",
                tool.name
            );
            let desc = tool.description.as_ref().unwrap();
            assert!(!desc.is_empty(), "Tool '{}' description should not be empty", tool.name);
        }
    }

    #[tokio::test]
    async fn test_extract_file_tool_has_correct_schema() {
        let router = KreuzbergMcp::tool_router();
        let tools = router.list_all();

        let extract_file_tool = tools
            .iter()
            .find(|t| t.name == "extract_file")
            .expect("extract_file tool should exist");

        assert!(extract_file_tool.description.is_some());

        assert!(!extract_file_tool.input_schema.is_empty());
    }

    #[tokio::test]
    async fn test_all_tools_have_input_schemas() {
        let router = KreuzbergMcp::tool_router();
        let tools = router.list_all();

        for tool in tools {
            assert!(
                !tool.input_schema.is_empty(),
                "Tool '{}' should have an input schema with fields",
                tool.name
            );
        }
    }

    #[test]
    fn test_server_creation_with_custom_config() {
        let custom_config = ExtractionConfig {
            force_ocr: true,
            use_cache: false,
            ocr: Some(crate::OcrConfig {
                backend: "tesseract".to_string(),
                language: "spa".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let server = KreuzbergMcp::with_config(custom_config.clone());

        assert_eq!(server.default_config.force_ocr, custom_config.force_ocr);
        assert_eq!(server.default_config.use_cache, custom_config.use_cache);
    }

    #[test]
    fn test_server_clone_preserves_config() {
        let custom_config = ExtractionConfig {
            force_ocr: true,
            ..Default::default()
        };

        let server1 = KreuzbergMcp::with_config(custom_config);
        let server2 = server1.clone();

        assert_eq!(server1.default_config.force_ocr, server2.default_config.force_ocr);
    }

    #[tokio::test]
    async fn test_server_is_thread_safe() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let server1 = server.clone();
        let server2 = server.clone();

        let handle1 = tokio::spawn(async move { server1.get_info() });

        let handle2 = tokio::spawn(async move { server2.get_info() });

        let info1 = handle1.await.unwrap();
        let info2 = handle2.await.unwrap();

        assert_eq!(info1.server_info.name, info2.server_info.name);
    }
}
