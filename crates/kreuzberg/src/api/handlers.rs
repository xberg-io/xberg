//! API request handlers.

use axum::{Json, extract::State};

use crate::{batch_extract_bytes, cache, extract_bytes};

use super::{
    error::{ApiError, JsonApi, MultipartApi},
    types::{
        ApiState, CacheClearResponse, CacheStatsResponse, ChunkRequest, ChunkResponse, EmbedRequest, EmbedResponse,
        ExtractResponse, HealthResponse, InfoResponse,
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

    let files_data: Vec<(Vec<u8>, String)> = files.into_iter().map(|(data, mime, _name)| (data, mime)).collect();

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
        #[cfg(feature = "embeddings")]
        crate::core::config::EmbeddingModelType::FastEmbed { model, .. } => model.clone(),
        crate::core::config::EmbeddingModelType::Custom { .. } => "custom".to_string(),
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
