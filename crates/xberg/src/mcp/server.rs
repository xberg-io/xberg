//! Xberg MCP server implementation.
//!
//! This module provides the main MCP server struct and startup functions.

use super::format::build_config;
use crate::ExtractionConfig;
use crate::service::{ExtractionRequest, ExtractionServiceBuilder};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{
        prompt::PromptContext,
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use tower::util::BoxCloneService;

#[cfg(feature = "mcp-http")]
use rmcp::transport::streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager};

/// Xberg MCP server.
///
/// Provides document extraction capabilities via MCP tools.
///
/// The server loads a default extraction configuration from xberg.toml/yaml/json
/// via discovery. Per-request OCR settings override the defaults.
#[cfg_attr(alef, alef(skip))]
pub struct XbergMcp {
    tool_router: ToolRouter<XbergMcp>,
    /// Prompt router for the three guided-workflow prompts.
    prompt_router: PromptRouter<XbergMcp>,
    /// Default extraction configuration loaded from config file via discovery
    default_config: std::sync::Arc<ExtractionConfig>,
    /// Tower service for extraction requests with tracing and metrics layers.
    ///
    /// Wrapped in `Mutex` because `BoxCloneService` is `Send` but not `Sync`,
    /// while `XbergMcp` must be `Sync` for the MCP handler trait.
    /// The lock is held only long enough to clone the service.
    extraction_service:
        std::sync::Mutex<BoxCloneService<ExtractionRequest, crate::types::ExtractionResult, crate::XbergError>>,
}

impl Clone for XbergMcp {
    fn clone(&self) -> Self {
        let svc = self
            .extraction_service
            .lock()
            .expect("extraction service lock poisoned")
            .clone();
        Self {
            tool_router: self.tool_router.clone(),
            prompt_router: self.prompt_router.clone(),
            default_config: self.default_config.clone(),
            extraction_service: std::sync::Mutex::new(svc),
        }
    }
}

#[tool_router]
impl XbergMcp {
    /// Create a new Xberg MCP server instance with default config.
    ///
    /// Uses `ExtractionConfig::discover()` to search for xberg.toml/yaml/json
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

    /// Create a new Xberg MCP server instance with explicit config.
    ///
    /// # Arguments
    ///
    /// * `config` - Default extraction configuration for all tool calls
    pub(crate) fn with_config(config: ExtractionConfig) -> Self {
        let extraction_service = ExtractionServiceBuilder::new().with_tracing().with_metrics().build();

        Self {
            tool_router: Self::tool_router(),
            prompt_router: super::prompts::build_prompt_router(),
            default_config: std::sync::Arc::new(config),
            extraction_service: std::sync::Mutex::new(extraction_service),
        }
    }

    /// Extract content from bytes or a URI.
    #[tool(
        description = "Extract content from bytes, a local path, file:// URI, remote document URL, or website URL.",
        annotations(title = "Extract", read_only_hint = true, idempotent_hint = true, open_world_hint = true),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::ExtractionOutput>()
            .expect("ExtractionOutput schema must be valid")
    )]
    async fn extract(
        &self,
        Parameters(params): Parameters<super::params::ExtractParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_xberg_error_to_mcp;

        let use_toon = params
            .response_format
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case("toon"));

        let mut config =
            build_config(&self.default_config, params.config).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;
        apply_pdf_password(&mut config, params.pdf_password)?;
        let input = parse_extract_input(params.input)?;

        let output = crate::extract(input, &config).await.map_err(map_xberg_error_to_mcp)?;
        let response = format_extraction_output_for_wire(&output, use_toon);
        let mut tool_result = CallToolResult::success(vec![Content::text(response)]);
        tool_result.structured_content = serde_json::to_value(&output).ok();
        Ok(tool_result)
    }

    /// Extract content from multiple bytes or URI inputs.
    #[tool(
        description = "Extract content from multiple bytes, local paths, file:// URIs, remote document URLs, or website URLs.",
        annotations(title = "Extract Batch", read_only_hint = true, idempotent_hint = true, open_world_hint = true),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::ExtractionOutput>()
            .expect("ExtractionOutput schema must be valid")
    )]
    async fn extract_batch(
        &self,
        Parameters(params): Parameters<super::params::ExtractBatchParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_xberg_error_to_mcp;

        let use_toon = params
            .response_format
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case("toon"));

        let mut config =
            build_config(&self.default_config, params.config).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;
        apply_pdf_password(&mut config, params.pdf_password)?;
        let inputs = params
            .inputs
            .into_iter()
            .map(parse_extract_input)
            .collect::<Result<Vec<_>, _>>()?;

        let output = crate::extract_batch(inputs, &config)
            .await
            .map_err(map_xberg_error_to_mcp)?;
        let response = format_extraction_output_for_wire(&output, use_toon);
        let mut tool_result = CallToolResult::success(vec![Content::text(response)]);
        tool_result.structured_content = serde_json::to_value(&output).ok();
        Ok(tool_result)
    }

    /// Detect the MIME type of a file.
    ///
    /// This tool identifies the file format, useful for determining which extractor to use.
    #[tool(
        description = "Detect the MIME type of a file. Returns the detected MIME type string.",
        annotations(title = "Detect MIME Type", read_only_hint = true, idempotent_hint = true),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::DetectMimeTypeOutput>()
            .expect("DetectMimeTypeOutput schema must be valid")
    )]
    fn detect_mime_type(
        &self,
        Parameters(params): Parameters<super::params::DetectMimeTypeParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_xberg_error_to_mcp;
        use crate::detect_mime_type;

        let mime_type = detect_mime_type(params.path.clone(), params.use_content).map_err(map_xberg_error_to_mcp)?;

        let dto = super::schema::DetectMimeTypeOutput {
            mime_type: mime_type.clone(),
        };
        let mut tool_result = CallToolResult::success(vec![Content::text(mime_type)]);
        tool_result.structured_content = serde_json::to_value(&dto).ok();
        Ok(tool_result)
    }

    /// Get cache statistics.
    ///
    /// This tool returns statistics about the cache including total files, size, and disk space.
    #[tool(
        description = "Get cache statistics including total files, size, and available disk space.",
        annotations(title = "Cache Stats", read_only_hint = true, idempotent_hint = true),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::CacheStatsOutput>()
            .expect("CacheStatsOutput schema must be valid")
    )]
    fn cache_stats(
        &self,
        Parameters(_): Parameters<super::params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_xberg_error_to_mcp;
        use crate::cache;

        let cache_dir = crate::cache_dir::resolve_cache_base();

        let stats = cache::get_cache_metadata(cache_dir.to_str().unwrap_or(".")).map_err(map_xberg_error_to_mcp)?;

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

        let dto = super::schema::CacheStatsOutput {
            directory: cache_dir.to_string_lossy().into_owned(),
            total_files: stats.total_files as u64,
            total_size_mb: stats.total_size_mb,
            available_space_mb: stats.available_space_mb,
        };
        let mut tool_result = CallToolResult::success(vec![Content::text(response)]);
        tool_result.structured_content = serde_json::to_value(&dto).ok();
        Ok(tool_result)
    }

    /// List all supported document formats.
    ///
    /// This tool returns all file extensions and MIME types that Xberg can process.
    #[tool(
        description = "List all supported document formats with their file extensions and MIME types.",
        annotations(title = "List Formats", read_only_hint = true, idempotent_hint = true),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::ListFormatsOutput>()
            .expect("ListFormatsOutput schema must be valid")
    )]
    fn list_formats(
        &self,
        Parameters(_): Parameters<super::params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let formats = crate::core::mime::list_supported_formats();
        let response = serde_json::to_string_pretty(&formats).unwrap_or_default();
        let dto = super::schema::ListFormatsOutput {
            formats: formats
                .into_iter()
                .map(|f| serde_json::to_value(f).unwrap_or_default())
                .collect(),
        };
        let mut tool_result = CallToolResult::success(vec![Content::text(response)]);
        tool_result.structured_content = serde_json::to_value(&dto).ok();
        Ok(tool_result)
    }

    /// Clear the cache.
    ///
    /// This tool removes all cached files and returns the number of files removed and space freed.
    #[tool(
        description = "Clear all cached files. Returns the number of files removed and space freed in MB.",
        annotations(title = "Clear Cache", read_only_hint = false, destructive_hint = true)
    )]
    fn cache_clear(
        &self,
        Parameters(_): Parameters<super::params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        use super::errors::map_xberg_error_to_mcp;
        use crate::cache;

        let cache_dir = crate::cache_dir::resolve_cache_base();

        let (removed_files, freed_mb) =
            cache::clear_cache_directory(cache_dir.to_str().unwrap_or(".")).map_err(map_xberg_error_to_mcp)?;

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

    /// Get Xberg version information.
    ///
    /// Returns the current version of the Xberg library.
    #[tool(
        description = "Get the current Xberg library version.",
        annotations(title = "Get Version", read_only_hint = true, idempotent_hint = true),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::VersionOutput>()
            .expect("VersionOutput schema must be valid")
    )]
    fn get_version(
        &self,
        Parameters(_): Parameters<super::params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let version = env!("CARGO_PKG_VERSION");
        let dto = super::schema::VersionOutput {
            version: version.to_string(),
        };
        let response = serde_json::to_string_pretty(&dto).unwrap_or_default();
        let mut tool_result = CallToolResult::success(vec![Content::text(response)]);
        tool_result.structured_content = serde_json::to_value(&dto).ok();
        Ok(tool_result)
    }

    /// Get model manifest with expected model files and checksums.
    ///
    /// Returns a manifest of all model files Xberg expects, including
    /// their sizes and SHA256 checksums.
    #[tool(
        description = "Get model manifest listing expected model files, sizes, and SHA256 checksums.",
        annotations(title = "Cache Manifest", read_only_hint = true, idempotent_hint = true),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::CacheManifestOutput>()
            .expect("CacheManifestOutput schema must be valid")
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

        #[cfg(feature = "ner-onnx")]
        {
            let manifest = crate::text::ner::manifest();
            for entry in manifest {
                entries.push(serde_json::to_value(&entry).unwrap_or_default());
            }
        }

        let total_size_bytes: u64 = entries
            .iter()
            .filter_map(|e| e.get("size_bytes").and_then(|v| v.as_u64()))
            .sum();
        let version = env!("CARGO_PKG_VERSION");

        let dto = super::schema::CacheManifestOutput {
            xberg_version: version.to_string(),
            model_count: entries.len(),
            total_size_bytes,
            models: entries,
        };
        let response = serde_json::to_string_pretty(&dto).unwrap_or_default();
        let mut tool_result = CallToolResult::success(vec![Content::text(response)]);
        tool_result.structured_content = serde_json::to_value(&dto).ok();
        Ok(tool_result)
    }

    /// Download and cache model files.
    ///
    /// Eagerly downloads model files (OCR, layout detection, embeddings, NER)
    /// so they are available for offline use.
    #[tool(
        description = "Download and cache model files for offline use. Optionally download embedding and GLiNER NER models.",
        annotations(
            title = "Cache Warm",
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = true
        )
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
        if let Some(ref name) = params.ner_model
            && name.trim().is_empty()
        {
            return Err(rmcp::ErrorData::invalid_params(
                "Field 'ner_model' must not be empty. Omit the field or provide a valid model name.".to_string(),
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
            let presets_to_warm: Vec<crate::EmbeddingPreset> = if params.all_embeddings {
                crate::embeddings::EMBEDDING_PRESETS.clone()
            } else if let Some(ref name) = params.embedding_model {
                match crate::embeddings::get_preset(name) {
                    Some(preset) => vec![preset],
                    None => {
                        let available: Vec<String> = crate::embeddings::list_presets();
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
                crate::embeddings::warm_model(
                    &crate::core::config::EmbeddingModelType::Preset {
                        name: preset.name.clone(),
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

        #[cfg(feature = "ner-onnx")]
        {
            if params.ner || params.all_ner_models || params.ner_model.is_some() {
                let models_to_warm: Vec<String> = if params.all_ner_models {
                    crate::text::ner::known_models().iter().map(|s| s.to_string()).collect()
                } else if let Some(ref name) = params.ner_model {
                    vec![name.clone()]
                } else {
                    vec![crate::text::ner::default_model_name().to_string()]
                };

                let ner_dir = cache_base.join("ner");
                for model in &models_to_warm {
                    let path = crate::text::ner::download_model(model, Some(ner_dir.clone())).map_err(|e| {
                        rmcp::ErrorData::internal_error(
                            format!("Failed to download NER model '{}': {}", model, e),
                            None,
                        )
                    })?;
                    downloaded.push(format!("ner gliner ({model}) -> {}", path.display()));
                }
            }
        }

        #[cfg(not(feature = "ner-onnx"))]
        {
            if params.ner || params.all_ner_models || params.ner_model.is_some() {
                return Err(rmcp::ErrorData::invalid_params(
                    "NER model warming requires the 'ner-onnx' feature to be enabled".to_string(),
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
        annotations(
            title = "Embed Text",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        ),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::EmbedTextOutput>()
            .expect("EmbedTextOutput schema must be valid")
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
        annotations(
            title = "Extract Structured",
            read_only_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        ),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::ExtractStructuredOutput>()
            .expect("ExtractStructuredOutput schema must be valid")
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
        annotations(title = "Chunk Text", read_only_hint = true, idempotent_hint = true),
        output_schema = rmcp::handler::server::common::schema_for_output::<super::schema::ChunkTextOutput>()
            .expect("ChunkTextOutput schema must be valid")
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

fn parse_extract_input(value: serde_json::Value) -> Result<crate::ExtractInput, rmcp::ErrorData> {
    serde_json::from_value::<crate::ExtractInput>(value)
        .map_err(|error| rmcp::ErrorData::invalid_params(format!("Invalid ExtractInput: {error}"), None))
}

fn format_extraction_output_for_wire(output: &crate::ExtractionOutput, use_toon: bool) -> String {
    if use_toon {
        serde_toon::to_string(output).unwrap_or_else(|error| {
            tracing::error!(%error, "Failed to serialize extraction output to TOON, falling back to JSON");
            serde_json::to_string_pretty(output).unwrap_or_default()
        })
    } else {
        serde_json::to_string_pretty(output).unwrap_or_default()
    }
}

fn apply_pdf_password(config: &mut ExtractionConfig, password: Option<String>) -> Result<(), rmcp::ErrorData> {
    let Some(password) = password else {
        return Ok(());
    };
    if password.is_empty() {
        return Err(rmcp::ErrorData::invalid_params(
            "pdf_password must not be empty when set".to_string(),
            None,
        ));
    }

    #[cfg(feature = "pdf")]
    {
        let pdf_options = config
            .pdf_options
            .get_or_insert_with(crate::core::config::pdf::PdfConfig::default);
        pdf_options.passwords.get_or_insert_with(Vec::new).push(password);
        Ok(())
    }

    #[cfg(not(feature = "pdf"))]
    {
        let _ = config;
        Err(rmcp::ErrorData::invalid_params(
            "pdf_password requires the 'pdf' feature to be enabled".to_string(),
            None,
        ))
    }
}

/// Handle completion requests for prompt arguments and resource URIs.
fn complete_impl(request: CompleteRequestParams) -> Result<CompleteResult, rmcp::ErrorData> {
    use rmcp::model::{CompletionInfo, Reference};

    let arg_name = &request.argument.name;
    let arg_value = &request.argument.value;

    let candidates: Vec<String> = match &request.r#ref {
        Reference::Prompt(prompt_ref) => {
            match (prompt_ref.name.as_str(), arg_name.as_str()) {
                // OCR language completions for extract_with_ocr
                (_, "languages") => complete_ocr_languages(arg_value),
                // Embedding preset completions for semantic_search
                (_, "preset") => complete_embedding_presets(arg_value),
                // Chunker type completions for semantic_search
                (_, "chunker_type") => complete_chunker_types(arg_value),
                // output_format for extract_document
                (_, "output_format") => complete_output_formats(arg_value),
                _ => vec![],
            }
        }
        Reference::Resource(_) => vec![],
    };

    let completion = CompletionInfo::with_all_values(candidates).unwrap_or_else(|_| CompletionInfo {
        values: vec![],
        total: Some(0),
        has_more: Some(false),
    });
    Ok(CompleteResult::new(completion))
}

/// Return OCR language code completions filtered by the given prefix.
fn complete_ocr_languages(prefix: &str) -> Vec<String> {
    let all = [
        "afr", "amh", "ara", "asm", "aze", "bel", "ben", "bod", "bos", "bul", "cat", "ceb", "ces", "chi_sim",
        "chi_tra", "chr", "cos", "cym", "dan", "deu", "div", "dzo", "ell", "eng", "enm", "epo", "est", "eus", "fao",
        "fas", "fil", "fin", "fra", "frm", "gle", "glg", "grc", "guj", "hat", "heb", "hin", "hrv", "hun", "hye", "iku",
        "ind", "isl", "ita", "ita_old", "jav", "jpn", "kan", "kat", "kaz", "khm", "kir", "kor", "kur", "lao", "lat",
        "lav", "lit", "ltz", "mal", "mar", "mkd", "mlt", "mon", "mri", "msa", "mya", "nep", "nor", "oci", "ori", "pan",
        "pol", "por", "pus", "ron", "rus", "san", "sin", "slk", "slv", "snd", "spa", "spa_old", "sqi", "srp", "swa",
        "swe", "syr", "tam", "tat", "tel", "tgk", "tgl", "tha", "tir", "ton", "tur", "uig", "ukr", "urd", "uzb", "vie",
        "yid", "yor",
    ];
    // The value may be a comma-separated list; complete the last segment.
    let last = prefix.split(',').next_back().unwrap_or(prefix).trim();
    all.iter()
        .filter(|lang| lang.starts_with(last))
        .map(|s| s.to_string())
        .take(20)
        .collect()
}

/// Return embedding preset completions filtered by prefix.
fn complete_embedding_presets(prefix: &str) -> Vec<String> {
    let presets = ["speed", "balanced", "quality"];
    presets
        .iter()
        .filter(|p| p.starts_with(prefix))
        .map(|s| s.to_string())
        .collect()
}

/// Return chunker type completions filtered by prefix.
fn complete_chunker_types(prefix: &str) -> Vec<String> {
    let types = ["text", "markdown", "yaml", "semantic"];
    types
        .iter()
        .filter(|t| t.starts_with(prefix))
        .map(|s| s.to_string())
        .collect()
}

/// Return output format completions filtered by prefix.
fn complete_output_formats(prefix: &str) -> Vec<String> {
    let formats = ["json", "toon"];
    formats
        .iter()
        .filter(|f| f.starts_with(prefix))
        .map(|s| s.to_string())
        .collect()
}

/// Structured extraction implementation when liter-llm feature is enabled.
#[cfg(all(feature = "liter-llm", not(target_arch = "wasm32")))]
async fn extract_structured_impl(
    mcp: &XbergMcp,
    params: super::params::ExtractStructuredParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    use super::errors::map_xberg_error_to_mcp;
    use tower::Service;

    let config = build_config(&mcp.default_config, None).map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;

    let request = ExtractionRequest::file(&params.path, config.clone());

    let mut svc = mcp
        .extraction_service
        .lock()
        .expect("extraction service lock poisoned")
        .clone();
    let result = svc.call(request).await.map_err(map_xberg_error_to_mcp)?;

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
        .map_err(map_xberg_error_to_mcp)?;

    let dto = super::schema::ExtractStructuredOutput {
        structured_output: structured_output.clone(),
        content: result.content.clone(),
        mime_type: Some(result.mime_type.as_ref().to_string()),
    };
    let response = serde_json::to_string_pretty(&dto).unwrap_or_default();
    let mut tool_result = CallToolResult::success(vec![Content::text(response)]);
    tool_result.structured_content = serde_json::to_value(&dto).ok();
    Ok(tool_result)
}

/// Structured extraction implementation when liter-llm feature is disabled.
#[cfg(any(not(feature = "liter-llm"), target_arch = "wasm32"))]
async fn extract_structured_impl(
    _mcp: &XbergMcp,
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

        if crate::embeddings::get_preset(preset_name).is_none() {
            let available: Vec<String> = crate::embeddings::list_presets();
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

    let embeddings =
        crate::embeddings::embed_texts(&params.texts, &config).map_err(super::errors::map_xberg_error_to_mcp)?;

    let dimensions = embeddings.first().map(|e| e.len()).unwrap_or(0);
    let count = params.texts.len();

    let dto = super::schema::EmbedTextOutput {
        embeddings: embeddings.clone(),
        model: model_name.clone(),
        dimensions,
        count,
    };
    let response = serde_json::to_string_pretty(&dto).unwrap_or_default();
    let mut tool_result = CallToolResult::success(vec![Content::text(response)]);
    tool_result.structured_content = serde_json::to_value(&dto).ok();
    Ok(tool_result)
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

    let result = chunk_text(&params.text, &config, None).map_err(super::errors::map_xberg_error_to_mcp)?;

    let chunk_items: Vec<super::schema::ChunkItem> = result
        .chunks
        .iter()
        .map(|c| super::schema::ChunkItem {
            content: c.content.clone(),
            chunk_index: c.metadata.chunk_index,
            total_chunks: c.metadata.total_chunks,
        })
        .collect();

    let dto = super::schema::ChunkTextOutput {
        chunk_count: result.chunk_count,
        chunks: chunk_items,
    };
    let response = serde_json::to_string_pretty(&dto).unwrap_or_default();
    let mut tool_result = CallToolResult::success(vec![Content::text(response)]);
    tool_result.structured_content = serde_json::to_value(&dto).ok();
    Ok(tool_result)
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
impl ServerHandler for XbergMcp {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .enable_completions()
            .build();

        let server_info = Implementation::new("xberg-mcp", env!("CARGO_PKG_VERSION"))
            .with_title("Xberg Document Intelligence MCP Server")
            .with_description(
                "Document intelligence library for extracting content from PDFs, images, office documents, and more.",
            )
            .with_website_url("https://docs.xberg.io");

        InitializeResult::new(capabilities)
            .with_server_info(server_info)
            .with_instructions(
                "Extract content from documents in various formats. Supports PDFs, Word documents, \
                 Excel spreadsheets, images (with OCR), HTML, emails, and more. Use enable_ocr=true \
                 for scanned documents, force_ocr=true to always use OCR even if text extraction \
                 succeeds. Use disable_ocr=true to skip OCR entirely (images return metadata only).",
            )
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, rmcp::ErrorData>> + rmcp::service::MaybeSendFuture + '_
    {
        std::future::ready(Ok(super::resources::list_resources()))
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourceTemplatesResult, rmcp::ErrorData>>
    + rmcp::service::MaybeSendFuture
    + '_ {
        std::future::ready(Ok(super::resources::list_resource_templates()))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, rmcp::ErrorData>> + rmcp::service::MaybeSendFuture + '_
    {
        std::future::ready(super::resources::read_resource(&request.uri))
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListPromptsResult, rmcp::ErrorData>> + rmcp::service::MaybeSendFuture + '_
    {
        let prompts = self.prompt_router.list_all();
        std::future::ready(Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
            meta: None,
        }))
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<GetPromptResult, rmcp::ErrorData>> + rmcp::service::MaybeSendFuture + '_
    {
        let pr = self.prompt_router.clone();
        let pc = PromptContext::new(self, request.name, request.arguments, context);
        async move { pr.get_prompt(pc).await }
    }

    fn complete(
        &self,
        request: CompleteRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<CompleteResult, rmcp::ErrorData>> + rmcp::service::MaybeSendFuture + '_
    {
        std::future::ready(complete_impl(request))
    }
}

impl Default for XbergMcp {
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

/// Start the Xberg MCP server.
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
/// use xberg::mcp::start_mcp_server;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     start_mcp_server().await?;
///     Ok(())
/// }
/// ```
#[cfg_attr(alef, alef(skip))]
pub async fn start_mcp_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = XbergMcp::new()?.serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

/// Start MCP server with custom extraction config.
///
/// This variant allows specifying a custom extraction configuration
/// (e.g., loaded from a file) instead of using defaults.
#[cfg_attr(alef, alef(skip))]
pub async fn start_mcp_server_with_config(
    config: ExtractionConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = XbergMcp::with_config(config).serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

/// Wait for a shutdown signal: SIGTERM on Unix platforms or Ctrl-C on all platforms.
///
/// The future resolves as soon as the first signal arrives, allowing axum's
/// `with_graceful_shutdown` to drain in-flight connections before the process exits.
#[cfg(feature = "mcp-http")]
async fn mcp_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to install SIGTERM handler: {}", e);
                // Fall back to Ctrl-C only.
                tokio::signal::ctrl_c()
                    .await
                    .unwrap_or_else(|e| tracing::warn!("Failed to listen for Ctrl-C: {}", e));
                tracing::info!("MCP server shutting down gracefully on signal...");
                return;
            }
        };

        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("MCP server shutting down gracefully on signal...");
            }
            result = tokio::signal::ctrl_c() => {
                if let Err(e) = result {
                    tracing::warn!("Failed to listen for Ctrl-C: {}", e);
                }
                tracing::info!("MCP server shutting down gracefully on signal...");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .unwrap_or_else(|e| tracing::warn!("Failed to listen for Ctrl-C: {}", e));
        tracing::info!("MCP server shutting down gracefully on signal...");
    }
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
/// use xberg::mcp::start_mcp_server_http;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     start_mcp_server_http("127.0.0.1", 8001).await?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "mcp-http")]
#[cfg_attr(alef, alef(skip))]
pub async fn start_mcp_server_http(
    host: impl AsRef<str>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::Router;
    use std::net::SocketAddr;

    let http_service = StreamableHttpService::new(
        || XbergMcp::new().map_err(|e| std::io::Error::other(e.to_string())),
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
    axum::serve(listener, router)
        .with_graceful_shutdown(mcp_shutdown_signal())
        .await?;

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
/// use xberg::mcp::start_mcp_server_http_with_config;
/// use xberg::ExtractionConfig;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let config = ExtractionConfig::default();
///     start_mcp_server_http_with_config("127.0.0.1", 8001, config).await?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "mcp-http")]
#[cfg_attr(alef, alef(skip))]
pub async fn start_mcp_server_http_with_config(
    host: impl AsRef<str>,
    port: u16,
    config: ExtractionConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::Router;
    use std::net::SocketAddr;

    let http_service = StreamableHttpService::new(
        move || Ok(XbergMcp::with_config(config.clone())),
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
    axum::serve(listener, router)
        .with_graceful_shutdown(mcp_shutdown_signal())
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_router_has_routes() {
        let router = XbergMcp::tool_router();
        assert!(router.has_route("extract"));
        assert!(router.has_route("extract_batch"));
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
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.name, "xberg-mcp");
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

        let server = XbergMcp::with_config(custom_config);

        assert!(server.default_config.force_ocr);
        assert!(!server.default_config.use_cache);
    }

    #[test]
    fn test_new_creates_server_with_default_config() {
        let server = XbergMcp::new();
        assert!(server.is_ok());
    }

    #[test]
    fn test_default_creates_server_without_panic() {
        let server = XbergMcp::default();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "xberg-mcp");
    }

    #[test]
    fn test_server_info_has_correct_fields() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.name, "xberg-mcp");
        assert_eq!(
            info.server_info.title,
            Some("Xberg Document Intelligence MCP Server".to_string())
        );
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.server_info.website_url, Some("https://docs.xberg.io".to_string()));
        assert!(info.instructions.is_some());
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_mcp_server_info_protocol_version() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.protocol_version, ProtocolVersion::default());
    }

    #[test]
    fn test_mcp_server_info_has_all_required_fields() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert!(!info.server_info.name.is_empty());
        assert!(!info.server_info.version.is_empty());

        assert!(info.server_info.title.is_some());
        assert!(info.server_info.website_url.is_some());
        assert!(info.instructions.is_some());
    }

    #[test]
    fn test_mcp_server_capabilities_declares_tools() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_mcp_server_name_follows_convention() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.name, "xberg-mcp");
        assert!(!info.server_info.name.contains('_'));
        assert!(!info.server_info.name.contains(' '));
    }

    #[test]
    fn test_mcp_version_matches_cargo_version() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_mcp_instructions_are_helpful() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        let instructions = info.instructions.expect("Instructions should be present");

        assert!(instructions.contains("extract") || instructions.contains("Extract"));
        assert!(instructions.contains("OCR") || instructions.contains("ocr"));
        assert!(instructions.contains("document"));
    }

    #[tokio::test]
    async fn test_all_tools_are_registered() {
        let router = XbergMcp::tool_router();

        let expected_tools = vec![
            "extract",
            "extract_batch",
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
        let router = XbergMcp::tool_router();
        let tools = router.list_all();

        assert_eq!(tools.len(), 12, "Expected 12 tools, found {}", tools.len());
    }

    #[tokio::test]
    async fn test_tools_have_descriptions() {
        let router = XbergMcp::tool_router();
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
    async fn test_tool_annotations_reflect_behavior() {
        let router = XbergMcp::tool_router();
        let tools = router.list_all();

        let annotations_for = |name: &str| {
            tools
                .iter()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("tool '{name}' should exist"))
                .annotations
                .clone()
                .unwrap_or_else(|| panic!("tool '{name}' should have annotations"))
        };

        // Local, side-effect-free info tools: read-only, idempotent, closed-world.
        for name in [
            "detect_mime_type",
            "cache_stats",
            "list_formats",
            "get_version",
            "cache_manifest",
            "chunk_text",
        ] {
            let a = annotations_for(name);
            assert_eq!(a.read_only_hint, Some(true), "{name} should be read-only");
            assert_eq!(a.idempotent_hint, Some(true), "{name} should be idempotent");
            assert_ne!(a.open_world_hint, Some(true), "{name} should be closed-world");
        }

        for name in ["extract", "extract_batch"] {
            let a = annotations_for(name);
            assert_eq!(a.read_only_hint, Some(true), "{name} should be read-only");
            assert_eq!(a.idempotent_hint, Some(true), "{name} should be idempotent");
            assert_eq!(a.open_world_hint, Some(true), "{name} may fetch URLs");
        }

        // Destructive cache deletion: explicitly not read-only.
        let clear = annotations_for("cache_clear");
        assert_eq!(
            clear.read_only_hint,
            Some(false),
            "cache_clear modifies the environment"
        );
        assert_eq!(clear.destructive_hint, Some(true), "cache_clear is destructive");

        // Downloads model files from HuggingFace: writes cache, reaches the open world.
        let warm = annotations_for("cache_warm");
        assert_eq!(warm.read_only_hint, Some(false), "cache_warm writes the cache");
        assert_eq!(
            warm.destructive_hint,
            Some(false),
            "cache_warm is additive, not destructive"
        );
        assert_eq!(warm.open_world_hint, Some(true), "cache_warm fetches from HuggingFace");

        // May fetch an embedding model from HuggingFace.
        let embed = annotations_for("embed_text");
        assert_eq!(embed.read_only_hint, Some(true), "embed_text does not mutate user data");
        assert_eq!(embed.open_world_hint, Some(true), "embed_text may fetch a model");

        // External LLM call: non-deterministic, open-world.
        let structured = annotations_for("extract_structured");
        assert_eq!(
            structured.idempotent_hint,
            Some(false),
            "extract_structured is non-deterministic"
        );
        assert_eq!(
            structured.open_world_hint,
            Some(true),
            "extract_structured calls an external LLM"
        );
    }

    #[tokio::test]
    async fn test_extract_tool_has_correct_schema() {
        let router = XbergMcp::tool_router();
        let tools = router.list_all();

        let extract_tool = tools
            .iter()
            .find(|t| t.name == "extract")
            .expect("extract tool should exist");

        assert!(extract_tool.description.is_some());

        assert!(!extract_tool.input_schema.is_empty());
    }

    #[tokio::test]
    async fn test_all_tools_have_input_schemas() {
        let router = XbergMcp::tool_router();
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
                language: vec!["spa".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };

        let server = XbergMcp::with_config(custom_config.clone());

        assert_eq!(server.default_config.force_ocr, custom_config.force_ocr);
        assert_eq!(server.default_config.use_cache, custom_config.use_cache);
    }

    #[test]
    fn test_server_clone_preserves_config() {
        let custom_config = ExtractionConfig {
            force_ocr: true,
            ..Default::default()
        };

        let server1 = XbergMcp::with_config(custom_config);
        let server2 = server1.clone();

        assert_eq!(server1.default_config.force_ocr, server2.default_config.force_ocr);
    }

    #[tokio::test]
    async fn test_server_is_thread_safe() {
        let server = XbergMcp::with_config(ExtractionConfig::default());

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
        let server = XbergMcp::with_config(ExtractionConfig::default());

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
        // Verify structured_content is also present
        assert!(
            call_result.structured_content.is_some(),
            "get_version should have structured_content"
        );
        let sc = call_result.structured_content.unwrap();
        assert_eq!(sc["version"], env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_cache_manifest_returns_json() {
        let server = XbergMcp::with_config(ExtractionConfig::default());

        let result = server.cache_manifest(rmcp::handler::server::wrapper::Parameters(
            crate::mcp::params::EmptyParams {},
        ));

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    let parsed: serde_json::Value = serde_json::from_str(&text.text).expect("Should be valid JSON");
                    assert!(parsed.get("xberg_version").is_some());
                    assert!(parsed.get("model_count").is_some());
                    assert!(parsed.get("models").is_some());
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
        // Verify structured_content is also present
        assert!(
            call_result.structured_content.is_some(),
            "cache_manifest should have structured_content"
        );
    }

    #[cfg(feature = "chunking")]
    #[test]
    fn test_chunk_text_returns_chunks() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
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
        let server = XbergMcp::with_config(ExtractionConfig::default());
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
        let server = XbergMcp::with_config(ExtractionConfig::default());
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
    async fn test_extract_batch_empty_inputs_returns_empty_envelope() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let params = crate::mcp::params::ExtractBatchParams {
            inputs: vec![],
            config: None,
            pdf_password: None,
            response_format: None,
        };

        let result = server
            .extract_batch(rmcp::handler::server::wrapper::Parameters(params))
            .await;
        assert!(result.is_ok());
        let result = result.unwrap();
        let structured = result.structured_content.expect("structured content should exist");
        assert_eq!(structured["summary"]["inputs"], 0);
        assert_eq!(structured["summary"]["results"], 0);
        assert_eq!(structured["summary"]["errors"], 0);
    }

    #[test]
    fn test_chunk_text_max_characters_zero_returns_error() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
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
        let server = XbergMcp::with_config(ExtractionConfig::default());
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

    // --- New tests for capabilities, resources, prompts, completions, output_schema ---

    #[test]
    fn test_capabilities_declare_resources_prompts_completions() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();
        assert!(
            info.capabilities.resources.is_some(),
            "resources capability should be declared"
        );
        assert!(
            info.capabilities.prompts.is_some(),
            "prompts capability should be declared"
        );
        assert!(
            info.capabilities.completions.is_some(),
            "completions capability should be declared"
        );
        assert!(info.capabilities.tools.is_some(), "tools capability should be declared");
    }

    #[tokio::test]
    async fn test_output_schema_present_on_structured_tools() {
        let router = XbergMcp::tool_router();
        let tools = router.list_all();
        let structured_tools = [
            "extract",
            "extract_batch",
            "detect_mime_type",
            "get_version",
            "list_formats",
            "cache_stats",
            "cache_manifest",
            "chunk_text",
            "embed_text",
        ];
        for name in structured_tools {
            let tool = tools
                .iter()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("tool '{}' not found", name));
            assert!(
                tool.output_schema.is_some(),
                "tool '{}' should have output_schema",
                name
            );
        }
    }

    #[test]
    fn test_list_resources_returns_expected_uris() {
        let result = crate::mcp::resources::list_resources();
        let uris: Vec<&str> = result.resources.iter().map(|r| r.uri.as_str()).collect();
        assert!(uris.contains(&"xberg://formats"), "formats resource missing");
        assert!(uris.contains(&"xberg://models"), "models resource missing");
        assert!(
            uris.contains(&"xberg://languages/ocr"),
            "ocr languages resource missing"
        );
    }

    #[test]
    fn test_read_resource_formats_roundtrip() {
        let result =
            crate::mcp::resources::read_resource("xberg://formats").expect("formats resource should be readable");
        assert!(!result.contents.is_empty());
        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let _: serde_json::Value = serde_json::from_str(text).expect("formats should be valid JSON");
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[test]
    fn test_list_prompts_returns_workflows() {
        let server = XbergMcp::with_config(ExtractionConfig::default());
        let prompts = server.prompt_router.list_all();
        let names: Vec<&str> = prompts.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"extract_document"), "extract_document prompt missing");
        assert!(names.contains(&"extract_with_ocr"), "extract_with_ocr prompt missing");
        assert!(names.contains(&"semantic_search"), "semantic_search prompt missing");
    }

    #[test]
    fn test_complete_ocr_language_by_prefix() {
        // "en" prefix should match "eng"
        let candidates = complete_ocr_languages("en");
        assert!(!candidates.is_empty(), "should return candidates for prefix 'en'");
        assert!(
            candidates.iter().any(|c| c == "eng"),
            "eng should be in completions for prefix 'en'"
        );
    }

    #[test]
    fn test_complete_embedding_presets() {
        let candidates = complete_embedding_presets("b");
        assert_eq!(candidates, vec!["balanced"]);
    }

    #[test]
    fn test_complete_chunker_types_empty_prefix_returns_all() {
        let candidates = complete_chunker_types("");
        assert_eq!(candidates.len(), 4);
    }

    #[test]
    fn test_complete_output_formats() {
        let candidates = complete_output_formats("j");
        assert_eq!(candidates, vec!["json"]);
    }
}
