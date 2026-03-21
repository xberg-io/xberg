//! API request handlers.

use axum::{Json, extract::State};

use crate::{batch_extract_bytes, cache, extract_bytes};

use super::{
    error::{ApiError, JsonApi, MultipartApi},
    types::{
        ApiState, CacheClearResponse, CacheStatsResponse, ChunkRequest, ChunkResponse, DetectResponse, EmbedRequest,
        EmbedResponse, ExtractResponse, HealthResponse, InfoResponse, ManifestEntryResponse, ManifestResponse,
        VersionResponse, WarmRequest, WarmResponse,
    },
};

/// Health check endpoint handler.
///
/// GET /health
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.health"))]
pub async fn health_handler() -> Json<HealthResponse> {
    // Get plugin status
    let plugin_status = crate::plugins::startup_validation::PluginHealthStatus::check();

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        plugins: Some(super::types::PluginStatus {
            ocr_backends_count: plugin_status.ocr_backends_count,
            ocr_backends: plugin_status.ocr_backends,
            extractors_count: plugin_status.extractors_count,
            post_processors_count: plugin_status.post_processors_count,
        }),
    })
}

/// Server info endpoint handler.
///
/// GET /info
#[utoipa::path(
    get,
    path = "/info",
    tag = "health",
    responses(
        (status = 200, description = "Server information", body = InfoResponse),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.info"))]
pub async fn info_handler() -> Json<InfoResponse> {
    Json(InfoResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_backend: true,
    })
}

/// Extract endpoint handler.
///
/// POST /extract
///
/// Accepts multipart form data with:
/// - `files`: One or more files to extract
/// - `config` (optional): JSON extraction configuration (overrides server defaults)
///
/// Returns a list of extraction results, one per file.
///
/// # Size Limits
///
/// Request body size limits are enforced at the router layer via `DefaultBodyLimit` and `RequestBodyLimitLayer`.
/// Default limits:
/// - Total request body: 100 MB (all files + form data combined)
/// - Individual multipart fields: 100 MB (controlled by Axum's `DefaultBodyLimit`)
///
/// Limits can be configured via environment variables or programmatically when creating the router.
/// If a request exceeds the size limit, it will be rejected with HTTP 413 (Payload Too Large).
///
/// The server's default config (loaded from kreuzberg.toml/yaml/json via discovery)
/// is used as the base, and any per-request config overrides those defaults.
#[utoipa::path(
    post,
    path = "/extract",
    tag = "extraction",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Extraction successful", body = ExtractResponse),
        (status = 400, description = "Bad request", body = crate::api::types::ErrorResponse),
        (status = 413, description = "Payload too large", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg_attr(
    feature = "otel",
    tracing::instrument(
        name = "api.extract",
        skip(state, multipart),
        fields(files_count = tracing::field::Empty)
    )
)]
pub async fn extract_handler(
    State(state): State<ApiState>,
    MultipartApi(mut multipart): MultipartApi,
) -> Result<Json<ExtractResponse>, ApiError> {
    let mut files = Vec::new();
    let mut config: Option<crate::core::config::ExtractionConfig> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "files" => {
                let file_name = field.file_name().map(|s| s.to_string());
                let content_type = field.content_type().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;

                let mut mime_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

                // When the client sends a generic content type, try to detect from the filename
                if mime_type == "application/octet-stream"
                    && let Some(ref name) = file_name
                    && let Ok(detected) = crate::core::mime::detect_mime_type(name, false)
                {
                    mime_type = detected;
                }

                files.push((data.to_vec(), mime_type, file_name));
            }
            "config" => {
                let config_str = field
                    .text()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;

                config = Some(serde_json::from_str(&config_str).map_err(|e| {
                    ApiError::validation(crate::error::KreuzbergError::validation(format!(
                        "Invalid extraction configuration: {}",
                        e
                    )))
                })?);
            }
            "output_format" => {
                let format_str = field
                    .text()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;

                // Ensure config exists before modifying output_format
                let cfg = config.get_or_insert_with(|| (*state.default_config).clone());
                cfg.output_format = match format_str.to_lowercase().as_str() {
                    "plain" => crate::core::config::OutputFormat::Plain,
                    "markdown" => crate::core::config::OutputFormat::Markdown,
                    "djot" => crate::core::config::OutputFormat::Djot,
                    "html" => crate::core::config::OutputFormat::Html,
                    _ => {
                        return Err(ApiError::validation(crate::error::KreuzbergError::validation(format!(
                            "Invalid output_format: '{}'. Valid values: 'plain', 'markdown', 'djot', 'html'",
                            format_str
                        ))));
                    }
                };
            }
            "pdf_password" => {
                let pwd = field
                    .text()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;
                let cfg = config.get_or_insert_with(|| (*state.default_config).clone());
                let pdf_opts = cfg.pdf_options.get_or_insert_with(Default::default);
                pdf_opts.passwords.get_or_insert_with(Vec::new).push(pwd);
            }
            _ => {}
        }
    }

    if files.is_empty() {
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(
            "No files provided for extraction",
        )));
    }

    #[cfg(feature = "otel")]
    tracing::Span::current().record("files_count", files.len());

    // Use provided config or fall back to default from state
    let final_config = config.as_ref().unwrap_or(&state.default_config);

    if files.len() == 1 {
        let (data, mime_type, _file_name) = files
            .into_iter()
            .next()
            .expect("files.len() == 1 guarantees one element exists");
        let result = extract_bytes(&data, mime_type.as_str(), final_config).await?;
        return Ok(Json(vec![result]));
    }

    let files_data: Vec<(Vec<u8>, String, Option<crate::FileExtractionConfig>)> = files
        .into_iter()
        .map(|(data, mime, _name)| (data, mime, None))
        .collect();

    let results = batch_extract_bytes(files_data, final_config).await?;
    Ok(Json(results))
}

/// Formats endpoint handler.
///
/// GET /formats
///
/// Returns all supported file extensions and their corresponding MIME types.
#[utoipa::path(
    get,
    path = "/formats",
    tag = "health",
    responses(
        (status = 200, description = "Supported formats", body = Vec<crate::SupportedFormat>),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.formats"))]
pub async fn formats_handler() -> Json<Vec<crate::SupportedFormat>> {
    Json(crate::list_supported_formats())
}

/// Cache stats endpoint handler.
///
/// GET /cache/stats
///
/// # Errors
///
/// Returns `ApiError::Internal` if:
/// - Current directory cannot be determined
/// - Cache directory path contains non-UTF8 characters
/// - Cache metadata retrieval fails
#[utoipa::path(
    get,
    path = "/cache/stats",
    tag = "cache",
    responses(
        (status = 200, description = "Cache statistics", body = CacheStatsResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.cache_stats"))]
pub async fn cache_stats_handler() -> Result<Json<CacheStatsResponse>, ApiError> {
    let cache_dir = std::env::current_dir()
        .map_err(|e| {
            ApiError::internal(crate::error::KreuzbergError::Other(format!(
                "Failed to get current directory: {}",
                e
            )))
        })?
        .join(".kreuzberg");

    let cache_dir_str = cache_dir.to_str().ok_or_else(|| {
        ApiError::internal(crate::error::KreuzbergError::Other(format!(
            "Cache directory path contains non-UTF8 characters: {}",
            cache_dir.display()
        )))
    })?;

    let stats = cache::get_cache_metadata(cache_dir_str).map_err(ApiError::internal)?;

    Ok(Json(CacheStatsResponse {
        directory: cache_dir.to_string_lossy().to_string(),
        total_files: stats.total_files,
        total_size_mb: stats.total_size_mb,
        available_space_mb: stats.available_space_mb,
        oldest_file_age_days: stats.oldest_file_age_days,
        newest_file_age_days: stats.newest_file_age_days,
    }))
}

/// Cache clear endpoint handler.
///
/// DELETE /cache/clear
///
/// # Errors
///
/// Returns `ApiError::Internal` if:
/// - Current directory cannot be determined
/// - Cache directory path contains non-UTF8 characters
/// - Cache clearing operation fails
#[utoipa::path(
    delete,
    path = "/cache/clear",
    tag = "cache",
    responses(
        (status = 200, description = "Cache cleared", body = CacheClearResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.cache_clear"))]
pub async fn cache_clear_handler() -> Result<Json<CacheClearResponse>, ApiError> {
    let cache_dir = std::env::current_dir()
        .map_err(|e| {
            ApiError::internal(crate::error::KreuzbergError::Other(format!(
                "Failed to get current directory: {}",
                e
            )))
        })?
        .join(".kreuzberg");

    let cache_dir_str = cache_dir.to_str().ok_or_else(|| {
        ApiError::internal(crate::error::KreuzbergError::Other(format!(
            "Cache directory path contains non-UTF8 characters: {}",
            cache_dir.display()
        )))
    })?;

    let (removed_files, freed_mb) = cache::clear_cache_directory(cache_dir_str).map_err(ApiError::internal)?;

    Ok(Json(CacheClearResponse {
        directory: cache_dir.to_string_lossy().to_string(),
        removed_files,
        freed_mb,
    }))
}

/// Embedding endpoint handler.
///
/// POST /embed
///
/// Accepts JSON body with:
/// - `texts`: Array of strings to generate embeddings for
/// - `config` (optional): Embedding configuration (model, batch size, cache_dir)
///
/// Returns embeddings for each input text.
///
/// # Errors
///
/// Returns `ApiError::Internal` if:
/// - Embeddings feature is not enabled
/// - ONNX Runtime is not available
/// - Model initialization fails
/// - Embedding generation fails
#[utoipa::path(
    post,
    path = "/embed",
    tag = "embeddings",
    request_body = EmbedRequest,
    responses(
        (status = 200, description = "Embeddings generated", body = EmbedResponse),
        (status = 400, description = "Bad request - validation failed (e.g., empty texts array)", body = crate::api::types::ErrorResponse),
        (status = 422, description = "Unprocessable entity - invalid JSON body", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg(feature = "embeddings")]
#[cfg_attr(
    feature = "otel",
    tracing::instrument(
        name = "api.embed",
        skip(request),
        fields(
            texts_count = request.texts.len(),
            model = tracing::field::Empty
        )
    )
)]
pub async fn embed_handler(JsonApi(request): JsonApi<EmbedRequest>) -> Result<Json<EmbedResponse>, ApiError> {
    use crate::types::{Chunk, ChunkMetadata};

    if request.texts.is_empty() {
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(
            "No texts provided for embedding generation",
        )));
    }

    // Validate that no texts are empty
    if request.texts.iter().any(|t| t.is_empty()) {
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(
            "All text entries must be non-empty strings",
        )));
    }

    // Use default config if none provided
    let config = request.config.unwrap_or_default();

    // Create chunks from input texts
    let mut chunks: Vec<Chunk> = request
        .texts
        .iter()
        .enumerate()
        .map(|(idx, text)| Chunk {
            content: text.clone(),
            embedding: None,
            metadata: ChunkMetadata {
                byte_start: 0,
                byte_end: text.len(),
                token_count: None,
                chunk_index: idx,
                total_chunks: request.texts.len(),
                first_page: None,
                last_page: None,
                heading_context: None,
            },
        })
        .collect();

    // Generate embeddings
    crate::embeddings::generate_embeddings_for_chunks(&mut chunks, &config).map_err(ApiError::internal)?;

    // Extract embeddings from chunks
    let embeddings: Vec<Vec<f32>> = chunks
        .into_iter()
        .map(|chunk| {
            chunk.embedding.ok_or_else(|| {
                ApiError::internal(crate::error::KreuzbergError::Other(
                    "Failed to generate embedding for text".to_string(),
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let dimensions = embeddings.first().map(|e| e.len()).unwrap_or(0);

    // Get model name from config
    let model_name = match &config.model {
        crate::core::config::EmbeddingModelType::Preset { name } => name.clone(),
        crate::core::config::EmbeddingModelType::Custom { model_id, .. } => model_id.clone(),
    };

    #[cfg(feature = "otel")]
    tracing::Span::current().record("model", &model_name);

    Ok(Json(EmbedResponse {
        embeddings,
        model: model_name,
        dimensions,
        count: request.texts.len(),
    }))
}

/// Embedding endpoint handler (when embeddings feature is disabled).
///
/// Returns an error indicating embeddings feature is not enabled.
#[utoipa::path(
    post,
    path = "/embed",
    tag = "embeddings",
    request_body = EmbedRequest,
    responses(
        (status = 200, description = "Embeddings generated", body = EmbedResponse),
        (status = 400, description = "Bad request - validation failed (e.g., empty texts array)", body = crate::api::types::ErrorResponse),
        (status = 422, description = "Unprocessable entity - invalid JSON body", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg(not(feature = "embeddings"))]
pub async fn embed_handler(JsonApi(_request): JsonApi<EmbedRequest>) -> Result<Json<EmbedResponse>, ApiError> {
    Err(ApiError::internal(crate::error::KreuzbergError::MissingDependency(
        "Embeddings feature is not enabled. Rebuild with --features embeddings".to_string(),
    )))
}

/// Chunk text endpoint handler.
///
/// POST /chunk
///
/// Accepts JSON body with text and optional configuration.
/// Returns chunks with metadata.
#[utoipa::path(
    post,
    path = "/chunk",
    tag = "chunking",
    request_body = ChunkRequest,
    responses(
        (status = 200, description = "Text chunked successfully", body = ChunkResponse),
        (status = 400, description = "Bad request - validation failed (e.g., empty text)", body = crate::api::types::ErrorResponse),
        (status = 422, description = "Unprocessable entity - invalid JSON body", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg_attr(
    feature = "otel",
    tracing::instrument(
        name = "api.chunk",
        skip(request),
        fields(text_length = request.text.len(), chunker_type = request.chunker_type.as_str())
    )
)]
pub async fn chunk_handler(JsonApi(request): JsonApi<ChunkRequest>) -> Result<Json<ChunkResponse>, ApiError> {
    use super::types::{ChunkItem, ChunkingConfigResponse};
    use crate::chunking::{ChunkerType, ChunkingConfig, chunk_text};

    // Validate input
    if request.text.is_empty() {
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(
            "Text cannot be empty",
        )));
    }

    // Parse chunker_type (empty string is invalid, use default by omitting the field)
    let chunker_type = match request.chunker_type.to_lowercase().as_str() {
        "text" => ChunkerType::Text,
        "markdown" => ChunkerType::Markdown,
        other => {
            return Err(ApiError::validation(crate::error::KreuzbergError::validation(format!(
                "Invalid chunker_type: '{}'. Valid values: 'text', 'markdown'",
                other
            ))));
        }
    };

    // Build config with defaults
    let cfg = request.config.unwrap_or_default();
    let max_characters = cfg.max_characters.unwrap_or(2000);
    let overlap = cfg.overlap.unwrap_or(100);

    // Validate chunking configuration
    if overlap >= max_characters {
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(format!(
            "Invalid chunking configuration: overlap ({}) must be less than max_characters ({})",
            overlap, max_characters
        ))));
    }

    let config = ChunkingConfig {
        max_characters,
        overlap,
        trim: cfg.trim.unwrap_or(true),
        chunker_type,
        ..Default::default()
    };

    // Perform chunking - convert any remaining errors to validation errors since they're likely config issues
    let result = chunk_text(&request.text, &config, None).map_err(|e| {
        // Check if error message indicates a configuration issue
        let msg = e.to_string();
        if msg.contains("configuration") || msg.contains("overlap") || msg.contains("capacity") {
            ApiError::validation(crate::error::KreuzbergError::validation(format!(
                "Invalid chunking configuration: {}",
                msg
            )))
        } else {
            ApiError::internal(e)
        }
    })?;

    // Transform to response
    let chunks = result
        .chunks
        .into_iter()
        .map(|chunk| ChunkItem {
            content: chunk.content,
            byte_start: chunk.metadata.byte_start,
            byte_end: chunk.metadata.byte_end,
            chunk_index: chunk.metadata.chunk_index,
            total_chunks: chunk.metadata.total_chunks,
            first_page: chunk.metadata.first_page,
            last_page: chunk.metadata.last_page,
        })
        .collect();

    Ok(Json(ChunkResponse {
        chunks,
        chunk_count: result.chunk_count,
        config: ChunkingConfigResponse {
            max_characters: config.max_characters,
            overlap: config.overlap,
            trim: config.trim,
            chunker_type: format!("{:?}", config.chunker_type).to_lowercase(),
        },
        input_size_bytes: request.text.len(),
        chunker_type: request.chunker_type.to_lowercase(),
    }))
}

/// Version endpoint handler.
///
/// GET /version
///
/// Returns the current kreuzberg version.
#[utoipa::path(
    get,
    path = "/version",
    tag = "health",
    responses(
        (status = 200, description = "Version information", body = VersionResponse),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.version"))]
pub async fn version_handler() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// MIME type detection endpoint handler.
///
/// POST /detect
///
/// Accepts multipart form data with a single file and returns its detected MIME type.
///
/// # Errors
///
/// Returns `ApiError::Validation` if no file is provided.
/// Returns `ApiError::Internal` if MIME type detection fails.
#[utoipa::path(
    post,
    path = "/detect",
    tag = "extraction",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "MIME type detected", body = DetectResponse),
        (status = 400, description = "Bad request - no file provided", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.detect", skip(multipart)))]
pub async fn detect_handler(MultipartApi(mut multipart): MultipartApi) -> Result<Json<DetectResponse>, ApiError> {
    let mut file_data: Option<(Vec<u8>, Option<String>)> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" || field_name == "files" {
            let file_name = field.file_name().map(|s| s.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;
            file_data = Some((data.to_vec(), file_name));
            break;
        }
    }

    let (data, file_name) = file_data.ok_or_else(|| {
        ApiError::validation(crate::error::KreuzbergError::validation(
            "No file provided for MIME type detection. Upload a file with field name 'file' or 'files'.",
        ))
    })?;

    // Try detection from bytes first, fall back to extension-based detection
    let mime_type = crate::core::mime::detect_mime_type_from_bytes(&data).or_else(|_| {
        if let Some(ref name) = file_name {
            crate::core::mime::detect_mime_type(name, false)
        } else {
            Err(crate::error::KreuzbergError::Other(
                "Could not detect MIME type from file content or filename".to_string(),
            ))
        }
    })?;

    Ok(Json(DetectResponse {
        mime_type,
        filename: file_name,
    }))
}

/// Model manifest endpoint handler.
///
/// GET /cache/manifest
///
/// Returns the expected model files with checksums and sizes.
#[utoipa::path(
    get,
    path = "/cache/manifest",
    tag = "cache",
    responses(
        (status = 200, description = "Model manifest", body = ManifestResponse),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.cache_manifest"))]
pub async fn cache_manifest_handler() -> Json<ManifestResponse> {
    #[allow(unused_mut)]
    let mut models: Vec<ManifestEntryResponse> = Vec::new();

    #[cfg(feature = "paddle-ocr")]
    {
        models.extend(
            crate::paddle_ocr::ModelManager::manifest()
                .into_iter()
                .map(|e| ManifestEntryResponse {
                    relative_path: e.relative_path,
                    sha256: e.sha256,
                    size_bytes: e.size_bytes,
                    source_url: e.source_url,
                }),
        );
    }

    #[cfg(feature = "layout-detection")]
    {
        models.extend(
            crate::layout::LayoutModelManager::manifest()
                .into_iter()
                .map(|e| ManifestEntryResponse {
                    relative_path: e.relative_path,
                    sha256: e.sha256,
                    size_bytes: e.size_bytes,
                    source_url: e.source_url,
                }),
        );
    }

    let total_size_bytes: u64 = models.iter().map(|e| e.size_bytes).sum();
    let model_count = models.len();

    Json(ManifestResponse {
        kreuzberg_version: env!("CARGO_PKG_VERSION").to_string(),
        total_size_bytes,
        model_count,
        models,
    })
}

/// Cache warm endpoint handler.
///
/// POST /cache/warm
///
/// Eagerly downloads all required models to the cache directory.
/// Optionally downloads embedding models when the `embeddings` feature is enabled.
///
/// # Errors
///
/// Returns `ApiError::Internal` if model downloading fails.
/// Returns `ApiError::Validation` if an unknown embedding preset is requested.
#[utoipa::path(
    post,
    path = "/cache/warm",
    tag = "cache",
    request_body = WarmRequest,
    responses(
        (status = 200, description = "Models warmed", body = WarmResponse),
        (status = 400, description = "Bad request - unknown embedding model", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.cache_warm", skip(request)))]
pub async fn cache_warm_handler(JsonApi(request): JsonApi<WarmRequest>) -> Result<Json<WarmResponse>, ApiError> {
    let cache_base = resolve_cache_base();

    #[allow(unused_mut)]
    let mut downloaded: Vec<String> = Vec::new();
    #[allow(unused_mut)]
    let mut already_cached: Vec<String> = Vec::new();

    #[cfg(feature = "paddle-ocr")]
    {
        let paddle_dir = cache_base.join("paddle-ocr");
        let manager = crate::paddle_ocr::ModelManager::new(paddle_dir);

        manager.ensure_all_models().map_err(ApiError::internal)?;
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
                ApiError::internal(crate::error::KreuzbergError::Other(format!(
                    "Failed to download layout models: {}",
                    e
                )))
            })?;
            downloaded.push("layout (rtdetr, tatr)".to_string());
        }
    }

    #[cfg(feature = "embeddings")]
    {
        let embeddings_dir = cache_base.join("embeddings");
        let presets_to_warm: Vec<&crate::EmbeddingPreset> = if request.all_embeddings {
            crate::EMBEDDING_PRESETS.iter().collect()
        } else if let Some(ref name) = request.embedding_model {
            match crate::get_preset(name) {
                Some(preset) => vec![preset],
                None => {
                    let available: Vec<&str> = crate::list_presets();
                    return Err(ApiError::validation(crate::error::KreuzbergError::validation(format!(
                        "Unknown embedding preset '{}'. Available: {}",
                        name,
                        available.join(", ")
                    ))));
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
                ApiError::internal(crate::error::KreuzbergError::Other(format!(
                    "Failed to download embedding model '{}': {}",
                    preset.name, e
                )))
            })?;
            downloaded.push(label);
        }
    }

    #[cfg(not(feature = "embeddings"))]
    {
        if request.all_embeddings || request.embedding_model.is_some() {
            return Err(ApiError::validation(crate::error::KreuzbergError::validation(
                "Embedding model warming requires the 'embeddings' feature to be enabled",
            )));
        }
    }

    Ok(Json(WarmResponse {
        cache_dir: cache_base.to_string_lossy().to_string(),
        downloaded,
        already_cached,
    }))
}

/// Resolve the cache base directory.
fn resolve_cache_base() -> std::path::PathBuf {
    if let Ok(env_path) = std::env::var("KREUZBERG_CACHE_DIR") {
        return std::path::PathBuf::from(env_path);
    }
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".kreuzberg")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::{get, post},
    };
    use tower::ServiceExt;

    fn test_router() -> Router {
        let state = ApiState {
            default_config: std::sync::Arc::new(crate::ExtractionConfig::default()),
        };
        Router::new()
            .route("/version", get(version_handler))
            .route("/detect", post(detect_handler))
            .route("/cache/manifest", get(cache_manifest_handler))
            .route("/cache/warm", post(cache_warm_handler))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_version_handler_returns_200() {
        let app = test_router();
        let response = app
            .oneshot(Request::builder().uri("/version").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["version"].is_string());
        assert!(!json["version"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_cache_manifest_handler_returns_200() {
        let app = test_router();
        let response = app
            .oneshot(Request::builder().uri("/cache/manifest").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["kreuzberg_version"].is_string());
        assert!(json["total_size_bytes"].is_number());
        assert!(json["model_count"].is_number());
        assert!(json["models"].is_array());
    }

    #[tokio::test]
    async fn test_detect_handler_no_file_returns_400() {
        let app = test_router();

        // Send a request without multipart content type - should get an error
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/detect")
                    .header("content-type", "multipart/form-data; boundary=testboundary")
                    .body(Body::from("--testboundary--\r\n"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should fail because no file field is provided
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_cache_warm_handler_empty_request_returns_200() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/cache/warm")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // With no features requesting downloads, should succeed
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["cache_dir"].is_string());
        assert!(json["downloaded"].is_array());
        assert!(json["already_cached"].is_array());
    }
}
