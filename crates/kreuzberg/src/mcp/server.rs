//! Kreuzberg MCP server implementation.
//!
//! This module provides the main MCP server struct and startup functions.

use crate::ExtractionConfig;
use crate::service::{ExtractionRequest, ExtractionServiceBuilder};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use tower::util::BoxCloneService;

#[cfg(feature = "mcp-http")]
use rmcp::transport::streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager};

/// Kreuzberg MCP server.
///
/// Provides document extraction capabilities via MCP tools.
///
/// The server loads a default extraction configuration from kreuzberg.toml/yaml/json
/// via discovery. Per-request OCR settings override the defaults.
pub struct KreuzbergMcp {
    tool_router: ToolRouter<KreuzbergMcp>,
    /// Default extraction configuration loaded from config file via discovery
    default_config: std::sync::Arc<ExtractionConfig>,
    /// Tower service for extraction requests with tracing and metrics layers.
    ///
    /// Wrapped in `Mutex` because `BoxCloneService` is `Send` but not `Sync`,
    /// while `KreuzbergMcp` must be `Sync` for the MCP handler trait.
    /// The lock is held only long enough to clone the service.
    extraction_service:
        std::sync::Mutex<BoxCloneService<ExtractionRequest, crate::types::ExtractionResult, crate::KreuzbergError>>,
}

impl Clone for KreuzbergMcp {
    fn clone(&self) -> Self {
        let svc = self
            .extraction_service
            .lock()
            .expect("extraction service lock poisoned")
            .clone();
        Self {
            tool_router: self.tool_router.clone(),
            default_config: self.default_config.clone(),
            extraction_service: std::sync::Mutex::new(svc),
        }
    }
}

#[tool_router]
impl KreuzbergMcp {
    /// Create a new Kreuzberg MCP server instance with default config.
    ///
    /// Uses `ExtractionConfig::discover()` to search for kreuzberg.toml/yaml/json
    /// in current and parent directories. Falls back to default configuration if
    /// no config file is found.
    #[allow(clippy::manual_unwrap_or_default)]
    pub(crate) fn new() -> crate::Result<Self> {
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
    pub(crate) fn with_config(config: ExtractionConfig) -> Self {
        let extraction_service = ExtractionServiceBuilder::new().with_tracing().with_metrics().build();

        Self {
            tool_router: Self::tool_router(),
            default_config: std::sync::Arc::new(config),
            extraction_service: std::sync::Mutex::new(extraction_service),
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
        use super::format::{build_config, format_extraction_result_for_wire};
        use tower::Service;

        let use_toon = params
            .response_format
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case("toon"));

        let config =
            build_config(&self.default_config, params.config).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;

        let request = match params.mime_type {
            Some(ref mime) => ExtractionRequest::file_with_mime(&params.path, mime, config),
            None => ExtractionRequest::file(&params.path, config),
        };

        let mut svc = self
            .extraction_service
            .lock()
            .expect("extraction service lock poisoned")
            .clone();
        let result = svc.call(request).await.map_err(map_kreuzberg_error_to_mcp)?;

        let response = format_extraction_result_for_wire(&result, use_toon);
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
        use super::format::{build_config, format_extraction_result_for_wire};
        use base64::prelude::*;
        use tower::Service;

        let use_toon = params
            .response_format
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case("toon"));

        let bytes = BASE64_STANDARD
            .decode(&params.data)
            .map_err(|e| rmcp::ErrorData::invalid_params(format!("Invalid base64: {}", e), None))?;

        let config =
            build_config(&self.default_config, params.config).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;

        let mime_type = params.mime_type.as_deref().unwrap_or("");

        let request = ExtractionRequest::bytes(bytes, mime_type, config);

        let mut svc = self
            .extraction_service
            .lock()
            .expect("extraction service lock poisoned")
            .clone();
        let result = svc.call(request).await.map_err(map_kreuzberg_error_to_mcp)?;

        let response = format_extraction_result_for_wire(&result, use_toon);
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

        if params.paths.is_empty() {
            return Err(rmcp::ErrorData::invalid_params("paths array must not be empty", None));
        }

        let config =
            build_config(&self.default_config, params.config).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;

        let items: Vec<(std::path::PathBuf, Option<crate::FileExtractionConfig>)> =
            if let Some(file_configs) = params.file_configs {
                if file_configs.len() != params.paths.len() {
                    return Err(rmcp::ErrorData::invalid_params(
                        format!(
                            "file_configs length ({}) must match paths length ({})",
                            file_configs.len(),
                            params.paths.len()
                        ),
                        None,
                    ));
                }

                params
                    .paths
                    .iter()
                    .zip(file_configs.into_iter())
                    .map(|(path, fc)| {
                        let file_config = fc
                            .map(serde_json::from_value::<crate::FileExtractionConfig>)
                            .transpose()
                            .map_err(|e| {
                                rmcp::ErrorData::invalid_params(format!("Failed to parse file config: {}", e), None)
                            })?;
                        Ok((std::path::PathBuf::from(path), file_config))
                    })
                    .collect::<Result<Vec<_>, rmcp::ErrorData>>()?
            } else {
                params
                    .paths
                    .iter()
                    .map(|p| (std::path::PathBuf::from(p), None))
                    .collect()
            };

        let use_toon = params
            .response_format
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case("toon"));

        let results = batch_extract_file(items, &config)
            .await
            .map_err(map_kreuzberg_error_to_mcp)?;

        let response = if use_toon {
            serde_toon::to_string(&results).unwrap_or_default()
        } else {
            serde_json::to_string_pretty(&results).unwrap_or_default()
        };
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

        let cache_dir = crate::cache_dir::resolve_cache_base();

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

        let cache_dir = crate::cache_dir::resolve_cache_base();

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

    /// Get Kreuzberg version information.
    ///
    /// Returns the current version of the Kreuzberg library.
    #[tool(
        description = "Get the current Kreuzberg library version.",
        annotations(title = "Get Version", read_only_hint = true, idempotent_hint = true)
    )]
    fn get_version(
        &self,
        Parameters(_): Parameters<super::params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let response = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_default(),
        )]))
    }

    /// Get model manifest with expected model files and checksums.
    ///
    /// Returns a manifest of all model files Kreuzberg expects, including
    /// their sizes and SHA256 checksums.
    #[tool(
        description = "Get model manifest listing expected model files, sizes, and SHA256 checksums.",
        annotations(title = "Cache Manifest", read_only_hint = true, idempotent_hint = true)
    )]
    fn cache_manifest(
        &self,
        Parameters(_): Parameters<super::params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        #[allow(unused_mut)]
        let mut entries: Vec<serde_json::Value> = Vec::new();

        #[cfg(feature = "paddle-ocr")]
        {
            let manifest = crate::paddle_ocr::ModelManager::manifest();
            for entry in manifest {
                entries.push(serde_json::to_value(&entry).unwrap_or_default());
            }
        }

        #[cfg(feature = "layout-detection")]
        {
            let manifest = crate::layout::LayoutModelManager::manifest();
            for entry in manifest {
                entries.push(serde_json::to_value(&entry).unwrap_or_default());
            }
        }

        let total_size_bytes: u64 = entries
            .iter()
            .filter_map(|e| e.get("size_bytes").and_then(|v| v.as_u64()))
            .sum();
        let version = env!("CARGO_PKG_VERSION");

        let response = serde_json::json!({
            "kreuzberg_version": version,
            "total_size_bytes": total_size_bytes,
            "model_count": entries.len(),
            "models": entries,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_default(),
        )]))
    }

    /// Download and cache model files.
    ///
    /// Eagerly downloads model files (OCR, layout detection, embeddings)
    /// so they are available for offline use.
    #[tool(
        description = "Download and cache model files for offline use. Optionally download embedding models.",
        annotations(title = "Cache Warm", destructive_hint = false)
    )]
    #[allow(unused_mut)]
    fn cache_warm(
        &self,
        Parameters(params): Parameters<super::params::CacheWarmParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // Validate embedding_model is not an empty string
        if let Some(ref name) = params.embedding_model
            && name.trim().is_empty()
        {
            return Err(rmcp::ErrorData::invalid_params(
                "Field 'embedding_model' must not be empty. Omit the field or provide a valid preset name.".to_string(),
                None,
            ));
        }

        let cache_base = resolve_cache_base();

        let mut downloaded: Vec<String> = Vec::new();
        let mut already_cached: Vec<String> = Vec::new();

        #[cfg(feature = "paddle-ocr")]
        {
            let paddle_dir = cache_base.join("paddle-ocr");
            let manager = crate::paddle_ocr::ModelManager::new(paddle_dir);
            manager.ensure_all_models().map_err(|e| {
                rmcp::ErrorData::internal_error(format!("Failed to download PaddleOCR models: {}", e), None)
            })?;
            downloaded.push("paddle-ocr v2 (server+mobile det, cls, doc_ori, unified+per-script rec)".to_string());
        }

        #[cfg(feature = "layout-detection")]
        {
            let layout_dir = cache_base.join("layout");
            let manager = crate::layout::LayoutModelManager::new(Some(layout_dir));
            let was_cached = manager.is_rtdetr_cached() && manager.is_tatr_cached();
            if was_cached {
                already_cached.push("layout (rtdetr, tatr)".to_string());
            } else {
                manager.ensure_all_models().map_err(|e| {
                    rmcp::ErrorData::internal_error(format!("Failed to download layout models: {}", e), None)
                })?;
                downloaded.push("layout (rtdetr, tatr)".to_string());
            }
        }

        #[cfg(feature = "embeddings")]
        {
            let embeddings_dir = cache_base.join("embeddings");
            let presets_to_warm: Vec<&crate::EmbeddingPreset> = if params.all_embeddings {
                crate::EMBEDDING_PRESETS.iter().collect()
            } else if let Some(ref name) = params.embedding_model {
                match crate::get_preset(name) {
                    Some(preset) => vec![preset],
                    None => {
                        let available: Vec<&str> = crate::list_presets();
                        return Err(rmcp::ErrorData::invalid_params(
                            format!(
                                "Unknown embedding preset '{}'. Available: {}",
                                name,
                                available.join(", ")
                            ),
                            None,
                        ));
                    }
                }
            } else {
                vec![]
            };

            for preset in &presets_to_warm {
                let label = format!("embedding ({})", preset.name);
                crate::warm_model(
                    &crate::core::config::EmbeddingModelType::Preset {
                        name: preset.name.to_string(),
                    },
                    Some(embeddings_dir.clone()),
                )
                .map_err(|e| {
                    rmcp::ErrorData::internal_error(
                        format!("Failed to download embedding model '{}': {}", preset.name, e),
                        None,
                    )
                })?;
                downloaded.push(label);
            }
        }

        #[cfg(not(feature = "embeddings"))]
        {
            if params.all_embeddings || params.embedding_model.is_some() {
                return Err(rmcp::ErrorData::invalid_params(
                    "Embedding model warming requires the 'embeddings' feature to be enabled".to_string(),
                    None,
                ));
            }
        }

        let response = serde_json::json!({
            "cache_dir": cache_base.to_string_lossy(),
            "downloaded": downloaded,
            "already_cached": already_cached,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_default(),
        )]))
    }

    /// Generate vector embeddings for text strings.
    ///
    /// Uses the specified preset model (or "balanced" by default) to generate
    /// vector embeddings for the provided texts.
    /// Requires the `embeddings` feature to be enabled.
    #[tool(
        description = "Generate vector embeddings for text strings. Use preset: 'speed', 'balanced', or 'quality'.",
        annotations(title = "Embed Text", read_only_hint = true, idempotent_hint = true)
    )]
    fn embed_text(
        &self,
        Parameters(params): Parameters<super::params::EmbedTextParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        embed_text_impl(params)
    }

    /// Extract structured data from a document using an LLM with a JSON schema.
    ///
    /// Extracts content from a file, then sends it to a specified LLM with a JSON
    /// schema constraint to produce structured output conforming to the schema.
    /// Requires the `liter-llm` feature to be enabled.
    #[tool(
        description = "Extract structured data from a document using an LLM with a JSON schema. Requires 'liter-llm' feature.",
        annotations(title = "Extract Structured", read_only_hint = true, idempotent_hint = true)
    )]
    async fn extract_structured(
        &self,
        Parameters(params): Parameters<super::params::ExtractStructuredParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        extract_structured_impl(self, params).await
    }

    /// Split text into chunks with configurable size and overlap.
    ///
    /// Supports text, markdown, yaml, and semantic chunking modes. Useful for preparing
    /// text for embedding generation or other downstream processing.
    /// Requires the `chunking` feature to be enabled.
    #[tool(
        description = "Split text into chunks with configurable size and overlap. Supports 'text', 'markdown', 'yaml', and 'semantic' chunker types.",
        annotations(title = "Chunk Text", read_only_hint = true, idempotent_hint = true)
    )]
    fn chunk_text(
        &self,
        Parameters(params): Parameters<super::params::ChunkTextParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        chunk_text_impl(params)
    }
}

/// Resolve the cache base directory.
fn resolve_cache_base() -> std::path::PathBuf {
    crate::cache_dir::resolve_cache_base()
}

/// Structured extraction implementation when liter-llm feature is enabled.
#[cfg(feature = "liter-llm")]
async fn extract_structured_impl(
    mcp: &KreuzbergMcp,
    params: super::params::ExtractStructuredParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    use super::errors::map_kreuzberg_error_to_mcp;
    use super::format::build_config;
    use tower::Service;

    let config = build_config(&mcp.default_config, None).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;

    let request = ExtractionRequest::file(&params.path, config.clone());

    let mut svc = mcp
        .extraction_service
        .lock()
        .expect("extraction service lock poisoned")
        .clone();
    let result = svc.call(request).await.map_err(map_kreuzberg_error_to_mcp)?;

    // Build structured extraction config from params
    let structured_config = crate::core::config::llm::StructuredExtractionConfig {
        schema: params.schema,
        schema_name: params.schema_name,
        schema_description: params.schema_description,
        strict: params.strict,
        prompt: params.prompt,
        llm: crate::core::config::llm::LlmConfig {
            model: params.model,
            api_key: params.api_key,
            base_url: None,
            timeout_secs: None,
            max_retries: None,
            temperature: None,
            max_tokens: None,
        },
    };

    let (structured_output, _usage) = crate::llm::structured::extract_structured(&result.content, &structured_config)
        .await
        .map_err(map_kreuzberg_error_to_mcp)?;

    let response = serde_json::json!({
        "structured_output": structured_output,
        "content": result.content,
        "mime_type": result.mime_type.as_ref(),
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&response).unwrap_or_default(),
    )]))
}

/// Structured extraction implementation when liter-llm feature is disabled.
#[cfg(not(feature = "liter-llm"))]
async fn extract_structured_impl(
    _mcp: &KreuzbergMcp,
    _params: super::params::ExtractStructuredParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    Err(rmcp::ErrorData::invalid_params(
        "Structured extraction requires the 'liter-llm' feature to be enabled. Rebuild with --features liter-llm"
            .to_string(),
        None,
    ))
}

/// Embed text implementation when embeddings feature is enabled.
#[cfg(feature = "embeddings")]
fn embed_text_impl(params: super::params::EmbedTextParams) -> Result<CallToolResult, rmcp::ErrorData> {
    if params.texts.is_empty() {
        return Err(rmcp::ErrorData::invalid_params(
            "No texts provided for embedding generation",
            None,
        ));
    }

    if params.texts.iter().any(|t| t.is_empty()) {
        return Err(rmcp::ErrorData::invalid_params(
            "All text entries must be non-empty strings",
            None,
        ));
    }

    // Resolution order: embedding_plugin → model → preset.
    let (config, model_name) = if let Some(ref plugin_name) = params.embedding_plugin {
        if plugin_name.is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "embedding_plugin must not be empty when set".to_string(),
                None,
            ));
        }
        // Pre-flight: surface unknown backends with a list of registered names
        // (parity with the REST `embed_handler` and CLI `embed --provider plugin`).
        let registry = crate::plugins::get_embedding_backend_registry();
        let guard = registry.read();
        if guard.get(plugin_name).is_err() {
            let available = guard.list();
            return Err(rmcp::ErrorData::invalid_params(
                format!(
                    "Embedding backend '{}' is not registered. Available backends: {}",
                    plugin_name,
                    if available.is_empty() {
                        "(none registered)".to_string()
                    } else {
                        available.join(", ")
                    }
                ),
                None,
            ));
        }
        drop(guard);
        let config = crate::core::config::EmbeddingConfig {
            model: crate::core::config::EmbeddingModelType::Plugin {
                name: plugin_name.clone(),
            },
            ..Default::default()
        };
        (config, plugin_name.clone())
    } else if let Some(ref model) = params.model {
        let llm_config = crate::core::config::llm::LlmConfig {
            model: model.clone(),
            api_key: params.api_key.clone(),
            base_url: None,
            timeout_secs: None,
            max_retries: None,
            temperature: None,
            max_tokens: None,
        };
        let config = crate::core::config::EmbeddingConfig {
            model: crate::core::config::EmbeddingModelType::Llm { llm: llm_config },
            ..Default::default()
        };
        (config, model.clone())
    } else {
        let preset_name = params.preset.as_deref().unwrap_or("balanced");

        if crate::get_preset(preset_name).is_none() {
            let available: Vec<&str> = crate::list_presets();
            return Err(rmcp::ErrorData::invalid_params(
                format!(
                    "Unknown embedding preset '{}'. Available: {}",
                    preset_name,
                    available.join(", ")
                ),
                None,
            ));
        }

        let config = crate::core::config::EmbeddingConfig {
            model: crate::core::config::EmbeddingModelType::Preset {
                name: preset_name.to_string(),
            },
            ..Default::default()
        };
        (config, preset_name.to_string())
    };

    let embeddings = crate::embed_texts(&params.texts, &config).map_err(super::errors::map_kreuzberg_error_to_mcp)?;

    let dimensions = embeddings.first().map(|e| e.len()).unwrap_or(0);

    let response = serde_json::json!({
        "embeddings": embeddings,
        "model": model_name,
        "dimensions": dimensions,
        "count": params.texts.len(),
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&response).unwrap_or_default(),
    )]))
}

/// Embed text implementation when embeddings feature is disabled.
#[cfg(not(feature = "embeddings"))]
fn embed_text_impl(_params: super::params::EmbedTextParams) -> Result<CallToolResult, rmcp::ErrorData> {
    Err(rmcp::ErrorData::invalid_params(
        "Embeddings feature is not enabled. Rebuild with --features embeddings".to_string(),
        None,
    ))
}

/// Chunk text implementation when chunking feature is enabled.
#[cfg(feature = "chunking")]
fn chunk_text_impl(params: super::params::ChunkTextParams) -> Result<CallToolResult, rmcp::ErrorData> {
    use crate::chunking::{ChunkingConfig, chunk_text};
    use crate::core::config::ChunkerType;

    if params.text.is_empty() {
        return Err(rmcp::ErrorData::invalid_params("Text cannot be empty", None));
    }

    let chunker_type = match params.chunker_type.as_deref().unwrap_or("text") {
        "text" => ChunkerType::Text,
        "markdown" => ChunkerType::Markdown,
        "yaml" => ChunkerType::Yaml,
        "semantic" => ChunkerType::Semantic,
        other => {
            return Err(rmcp::ErrorData::invalid_params(
                format!(
                    "Invalid chunker_type: '{}'. Valid values: 'text', 'markdown', 'yaml', 'semantic'",
                    other
                ),
                None,
            ));
        }
    };

    let max_characters = params.max_characters.unwrap_or(2000);
    let overlap = params.overlap.unwrap_or(100);

    if max_characters == 0 || max_characters > 1_000_000 {
        return Err(rmcp::ErrorData::invalid_params(
            format!("max_characters must be between 1 and 1,000,000, got {}", max_characters),
            None,
        ));
    }

    if overlap >= max_characters {
        return Err(rmcp::ErrorData::invalid_params(
            format!(
                "overlap ({}) must be less than max_characters ({})",
                overlap, max_characters
            ),
            None,
        ));
    }

    let config = ChunkingConfig {
        max_characters,
        overlap,
        trim: true,
        chunker_type,
        topic_threshold: params.topic_threshold,
        ..Default::default()
    };

    let result = chunk_text(&params.text, &config, None).map_err(super::errors::map_kreuzberg_error_to_mcp)?;

    let response = serde_json::json!({
        "chunk_count": result.chunk_count,
        "input_size_bytes": params.text.len(),
        "config": {
            "max_characters": config.max_characters,
            "overlap": config.overlap,
            "chunker_type": format!("{:?}", config.chunker_type).to_lowercase(),
        },
        "chunks": result.chunks.iter().map(|c| {
            serde_json::json!({
                "content": c.content,
                "byte_start": c.metadata.byte_start,
                "byte_end": c.metadata.byte_end,
                "chunk_index": c.metadata.chunk_index,
                "total_chunks": c.metadata.total_chunks,
            })
        }).collect::<Vec<_>>(),
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&response).unwrap_or_default(),
    )]))
}

/// Chunk text implementation when chunking feature is disabled.
#[cfg(not(feature = "chunking"))]
fn chunk_text_impl(_params: super::params::ChunkTextParams) -> Result<CallToolResult, rmcp::ErrorData> {
    Err(rmcp::ErrorData::invalid_params(
        "Chunking feature is not enabled. Rebuild with --features chunking".to_string(),
        None,
    ))
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
                 succeeds. Use disable_ocr=true to skip OCR entirely (images return metadata only).",
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
        assert!(router.has_route("get_version"));
        assert!(router.has_route("cache_manifest"));
        assert!(router.has_route("cache_warm"));
        assert!(router.has_route("chunk_text"));
        assert!(router.has_route("embed_text"));
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
            "get_version",
            "cache_manifest",
            "cache_warm",
            "chunk_text",
        ];

        let expected_tools_extra = ["embed_text"];

        for tool_name in expected_tools.iter().chain(expected_tools_extra.iter()) {
            assert!(router.has_route(tool_name), "Tool '{}' should be registered", tool_name);
        }
    }

    #[tokio::test]
    async fn test_tool_count_is_correct() {
        let router = KreuzbergMcp::tool_router();
        let tools = router.list_all();

        assert_eq!(tools.len(), 13, "Expected 13 tools, found {}", tools.len());
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

    #[test]
    fn test_get_version_returns_version() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let result = server.get_version(rmcp::handler::server::wrapper::Parameters(
            crate::mcp::params::EmptyParams {},
        ));

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    let parsed: serde_json::Value = serde_json::from_str(&text.text).expect("Should be valid JSON");
                    assert_eq!(parsed["version"], env!("CARGO_PKG_VERSION"));
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[test]
    fn test_cache_manifest_returns_json() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let result = server.cache_manifest(rmcp::handler::server::wrapper::Parameters(
            crate::mcp::params::EmptyParams {},
        ));

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    let parsed: serde_json::Value = serde_json::from_str(&text.text).expect("Should be valid JSON");
                    assert!(parsed.get("kreuzberg_version").is_some());
                    assert!(parsed.get("model_count").is_some());
                    assert!(parsed.get("models").is_some());
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[cfg(feature = "chunking")]
    #[test]
    fn test_chunk_text_returns_chunks() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = crate::mcp::params::ChunkTextParams {
            text: "Hello world. This is a test.".to_string(),
            max_characters: None,
            overlap: None,
            chunker_type: None,
            topic_threshold: None,
        };

        let result = server.chunk_text(rmcp::handler::server::wrapper::Parameters(params));

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    let parsed: serde_json::Value = serde_json::from_str(&text.text).expect("Should be valid JSON");
                    assert!(parsed.get("chunk_count").is_some());
                    assert!(parsed.get("chunks").is_some());
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[test]
    fn test_chunk_text_rejects_empty_input() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = crate::mcp::params::ChunkTextParams {
            text: String::new(),
            max_characters: None,
            overlap: None,
            chunker_type: None,
            topic_threshold: None,
        };

        let result = server.chunk_text(rmcp::handler::server::wrapper::Parameters(params));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.0, -32602);
    }

    #[test]
    fn test_chunk_text_rejects_invalid_chunker_type() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = crate::mcp::params::ChunkTextParams {
            text: "Some text".to_string(),
            max_characters: None,
            overlap: None,
            chunker_type: Some("invalid".to_string()),
            topic_threshold: None,
        };

        let result = server.chunk_text(rmcp::handler::server::wrapper::Parameters(params));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.0, -32602);
    }

    #[tokio::test]
    async fn test_batch_extract_files_empty_paths_returns_error() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = crate::mcp::params::BatchExtractFilesParams {
            paths: vec![],
            config: None,
            pdf_password: None,
            file_configs: None,
            response_format: None,
        };

        let result = server
            .batch_extract_files(rmcp::handler::server::wrapper::Parameters(params))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code.0, -32602);
        assert!(
            err.message.contains("paths array must not be empty"),
            "Expected empty paths error, got: {}",
            err.message
        );
    }

    #[test]
    fn test_chunk_text_max_characters_zero_returns_error() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = crate::mcp::params::ChunkTextParams {
            text: "Some text to chunk".to_string(),
            max_characters: Some(0),
            overlap: None,
            chunker_type: None,
            topic_threshold: None,
        };

        let result = server.chunk_text(rmcp::handler::server::wrapper::Parameters(params));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code.0, -32602);
        assert!(
            err.message.contains("max_characters must be between"),
            "Expected bounds error, got: {}",
            err.message
        );
    }

    #[test]
    fn test_chunk_text_max_characters_too_large_returns_error() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = crate::mcp::params::ChunkTextParams {
            text: "Some text to chunk".to_string(),
            max_characters: Some(2_000_000),
            overlap: None,
            chunker_type: None,
            topic_threshold: None,
        };

        let result = server.chunk_text(rmcp::handler::server::wrapper::Parameters(params));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code.0, -32602);
        assert!(
            err.message.contains("max_characters must be between"),
            "Expected bounds error, got: {}",
            err.message
        );
    }
}
