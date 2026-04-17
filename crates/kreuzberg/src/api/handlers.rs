//! API request handlers.

use axum::http::HeaderMap;
use axum::{Json, extract::State, response::IntoResponse};

use tower::Service;

use crate::{batch_extract_bytes, cache, service::ExtractionRequest};

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

/// Check whether TOON wire format was requested via the `Accept` header.
fn wants_toon(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/toon"))
}

/// Serialize extraction results as a TOON response.
fn toon_response(results: &ExtractResponse) -> Result<axum::response::Response<axum::body::Body>, ApiError> {
    let body = serde_toon::to_string(results).map_err(|e| {
        ApiError::internal(crate::error::KreuzbergError::Other(format!(
            "Failed to serialize response to TOON: {}",
            e
        )))
    })?;
    Ok(axum::response::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "application/toon")
        .body(axum::body::Body::from(body))
        .expect("valid response"))
}

/// Extract endpoint handler.
///
/// POST /extract
///
/// Accepts multipart form data with:
/// - `files`: One or more files to extract
/// - `config` (optional): JSON extraction configuration (overrides server defaults)
/// - `format` (optional): Wire format for the response (`json` or `toon`, default: `json`).
///   Alternatively, set the `Accept: application/toon` header.
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
        skip(state, headers, multipart),
        fields(files_count = tracing::field::Empty)
    )
)]
pub async fn extract_handler(
    State(state): State<ApiState>,
    headers: HeaderMap,
    MultipartApi(mut multipart): MultipartApi,
) -> Result<axum::response::Response<axum::body::Body>, ApiError> {
    let mut use_toon = wants_toon(&headers);
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
            "format" => {
                let format_str = field
                    .text()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;
                if format_str.eq_ignore_ascii_case("toon") {
                    use_toon = true;
                }
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

    let results = if files.len() == 1 {
        let (data, mime_type, _file_name) = files
            .into_iter()
            .next()
            .expect("files.len() == 1 guarantees one element exists");
        let request = ExtractionRequest::bytes(data, mime_type, final_config.clone());
        let mut svc = state
            .extraction_service
            .lock()
            .expect("extraction service lock poisoned")
            .clone();
        let result = svc.call(request).await?;
        vec![result]
    } else {
        let files_data: Vec<(Vec<u8>, String, Option<crate::FileExtractionConfig>)> = files
            .into_iter()
            .map(|(data, mime, _name)| (data, mime, None))
            .collect();

        #[cfg(feature = "otel")]
        let batch_span = tracing::info_span!(
            "kreuzberg.service",
            { crate::telemetry::conventions::OPERATION } = crate::telemetry::conventions::operations::BATCH_EXTRACT,
            { crate::telemetry::conventions::BATCH_SIZE } = files_data.len(),
        );
        #[cfg(not(feature = "otel"))]
        let batch_span = tracing::Span::none();

        {
            use tracing::Instrument;
            batch_extract_bytes(files_data, final_config)
                .instrument(batch_span)
                .await?
        }
    };

    if use_toon {
        toon_response(&results)
    } else {
        Ok(Json(results).into_response())
    }
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
    let cache_dir = crate::cache_dir::resolve_cache_base();

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
    let cache_dir = crate::cache_dir::resolve_cache_base();

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

    // Validate preset name if model type is Preset
    if let crate::core::config::EmbeddingModelType::Preset { ref name } = config.model
        && crate::get_preset(name).is_none()
    {
        let available: Vec<&str> = crate::list_presets();
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(format!(
            "Unknown embedding preset '{}'. Available: {}",
            name,
            available.join(", ")
        ))));
    }

    // Generate embeddings directly
    let text_count = request.texts.len();
    let embeddings = crate::embed_texts_async(request.texts, &config)
        .await
        .map_err(ApiError::internal)?;

    let dimensions = embeddings.first().map(|e| e.len()).unwrap_or(0);

    // Get model name from config
    let model_name = match &config.model {
        crate::core::config::EmbeddingModelType::Preset { name } => name.clone(),
        crate::core::config::EmbeddingModelType::Custom { model_id, .. } => model_id.clone(),
        crate::core::config::EmbeddingModelType::Llm { llm } => llm.model.clone(),
    };

    #[cfg(feature = "otel")]
    tracing::Span::current().record("model", &model_name);

    Ok(Json(EmbedResponse {
        embeddings,
        model: model_name,
        dimensions,
        count: text_count,
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

/// Structured extraction endpoint handler.
///
/// POST /extract-structured
///
/// Accepts multipart form data with a file and structured extraction configuration.
/// Extracts document content then runs LLM-based structured extraction using a JSON schema.
///
/// # Fields
///
/// - `file`: The document file (required)
/// - `config`: JSON extraction configuration (optional)
/// - `schema`: JSON schema for structured output (required)
/// - `schema_name`: Schema name (optional, default "extraction")
/// - `model`: LLM model string e.g. "openai/gpt-4o" (required)
/// - `api_key`: API key for the LLM provider (optional)
/// - `prompt`: Custom Jinja2 prompt template (optional)
/// - `strict`: "true"/"false" for strict mode (optional)
#[utoipa::path(
    post,
    path = "/extract-structured",
    tag = "extraction",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Structured extraction successful", body = crate::api::types::StructuredExtractionResponse),
        (status = 400, description = "Bad request", body = crate::api::types::ErrorResponse),
        (status = 413, description = "Payload too large", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg(feature = "liter-llm")]
#[cfg_attr(
    feature = "otel",
    tracing::instrument(name = "api.extract_structured", skip(state, multipart),)
)]
pub async fn extract_structured_handler(
    State(state): State<ApiState>,
    MultipartApi(mut multipart): MultipartApi,
) -> Result<Json<super::types::StructuredExtractionResponse>, ApiError> {
    let mut file_data: Option<(Vec<u8>, String, Option<String>)> = None;
    let mut config: Option<crate::core::config::ExtractionConfig> = None;
    let mut schema: Option<serde_json::Value> = None;
    let mut schema_name = "extraction".to_string();
    let mut model: Option<String> = None;
    let mut api_key: Option<String> = None;
    let mut prompt: Option<String> = None;
    let mut strict = false;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" | "files" => {
                let file_name = field.file_name().map(|s| s.to_string());
                let content_type = field.content_type().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;

                let mut mime_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

                if mime_type == "application/octet-stream"
                    && let Some(ref name) = file_name
                    && let Ok(detected) = crate::core::mime::detect_mime_type(name, false)
                {
                    mime_type = detected;
                }

                file_data = Some((data.to_vec(), mime_type, file_name));
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
            "schema" => {
                let schema_str = field
                    .text()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;
                schema = Some(serde_json::from_str(&schema_str).map_err(|e| {
                    ApiError::validation(crate::error::KreuzbergError::validation(format!(
                        "Invalid JSON schema: {}",
                        e
                    )))
                })?);
            }
            "schema_name" => {
                schema_name = field
                    .text()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;
            }
            "model" => {
                model = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?,
                );
            }
            "api_key" => {
                api_key = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?,
                );
            }
            "prompt" => {
                prompt = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?,
                );
            }
            "strict" => {
                let val = field
                    .text()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;
                strict = val.eq_ignore_ascii_case("true");
            }
            _ => {}
        }
    }

    let (data, mime_type, _file_name) = file_data.ok_or_else(|| {
        ApiError::validation(crate::error::KreuzbergError::validation(
            "No file provided for extraction. Upload a file with field name 'file'.",
        ))
    })?;

    let schema_val = schema.ok_or_else(|| {
        ApiError::validation(crate::error::KreuzbergError::validation(
            "Missing required field 'schema'. Provide a JSON schema string.",
        ))
    })?;

    let model_str = model.ok_or_else(|| {
        ApiError::validation(crate::error::KreuzbergError::validation(
            "Missing required field 'model'. Provide an LLM model string (e.g., 'openai/gpt-4o').",
        ))
    })?;

    // Extract document content
    let final_config = config.as_ref().unwrap_or(&state.default_config);
    let request = ExtractionRequest::bytes(data, &mime_type, final_config.clone());
    let mut svc = state
        .extraction_service
        .lock()
        .expect("extraction service lock poisoned")
        .clone();
    let result = svc.call(request).await?;

    // Build structured extraction config
    let structured_config = crate::core::config::llm::StructuredExtractionConfig {
        schema: schema_val,
        schema_name,
        schema_description: None,
        strict,
        prompt,
        llm: crate::core::config::llm::LlmConfig {
            model: model_str,
            api_key,
            base_url: None,
            timeout_secs: None,
            max_retries: None,
            temperature: None,
            max_tokens: None,
        },
    };

    // Run structured extraction on the extracted content
    let (structured_output, _usage) = crate::llm::structured::extract_structured(&result.content, &structured_config)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(super::types::StructuredExtractionResponse {
        structured_output,
        content: result.content,
        mime_type,
    }))
}

/// Structured extraction endpoint stub (when liter-llm feature is disabled).
///
/// POST /extract-structured
#[utoipa::path(
    post,
    path = "/extract-structured",
    tag = "extraction",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Structured extraction successful", body = crate::api::types::StructuredExtractionResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg(not(feature = "liter-llm"))]
pub async fn extract_structured_handler(
    State(_state): State<ApiState>,
    MultipartApi(_multipart): MultipartApi,
) -> Result<Json<super::types::StructuredExtractionResponse>, ApiError> {
    Err(ApiError::new(
        axum::http::StatusCode::NOT_IMPLEMENTED,
        crate::error::KreuzbergError::MissingDependency(
            "Structured extraction requires the 'liter-llm' feature to be enabled. Rebuild with --features liter-llm"
                .to_string(),
        ),
    ))
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
        "yaml" => ChunkerType::Yaml,
        "semantic" => ChunkerType::Semantic,
        other => {
            return Err(ApiError::validation(crate::error::KreuzbergError::validation(format!(
                "Invalid chunker_type: '{}'. Valid values: 'text', 'markdown', 'yaml', 'semantic'",
                other
            ))));
        }
    };

    // Build config with defaults
    let cfg = request.config.unwrap_or_default();
    let max_characters = cfg.max_characters.unwrap_or(2000);
    let overlap = cfg.overlap.unwrap_or(100);

    // Validate max_characters bounds
    if max_characters == 0 || max_characters > 1_000_000 {
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(format!(
            "max_characters must be between 1 and 1,000,000, got {}",
            max_characters
        ))));
    }

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
        topic_threshold: cfg.topic_threshold,
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
            topic_threshold: config.topic_threshold,
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
        (status = 400, description = "Bad request - unknown or empty embedding model", body = crate::api::types::ErrorResponse),
        (status = 422, description = "Unprocessable entity - invalid JSON body", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
        (status = 502, description = "Bad gateway - upstream model download failed", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.cache_warm", skip(request)))]
pub async fn cache_warm_handler(JsonApi(request): JsonApi<WarmRequest>) -> Result<Json<WarmResponse>, ApiError> {
    // Validate embedding_model is not an empty string
    if let Some(ref name) = request.embedding_model
        && name.trim().is_empty()
    {
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(
            "Field 'embedding_model' must not be empty. Omit the field or provide a valid preset name.",
        )));
    }

    let cache_base = resolve_cache_base();

    #[allow(unused_mut)]
    let mut downloaded: Vec<String> = Vec::new();
    #[allow(unused_mut)]
    let mut already_cached: Vec<String> = Vec::new();

    #[cfg(feature = "paddle-ocr")]
    {
        let paddle_dir = cache_base.join("paddle-ocr");
        let manager = crate::paddle_ocr::ModelManager::new(paddle_dir);

        manager.ensure_all_models().map_err(ApiError::bad_gateway)?;
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
                ApiError::bad_gateway(crate::error::KreuzbergError::Other(format!(
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
                ApiError::bad_gateway(crate::error::KreuzbergError::Other(format!(
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
    crate::cache_dir::resolve_cache_base()
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
        let extraction_service = crate::service::ExtractionServiceBuilder::new().build();
        let state = ApiState {
            default_config: std::sync::Arc::new(crate::ExtractionConfig::default()),
            extraction_service: std::sync::Arc::new(std::sync::Mutex::new(extraction_service)),
        };
        Router::new()
            .route("/version", get(version_handler))
            .route("/detect", post(detect_handler))
            .route("/cache/manifest", get(cache_manifest_handler))
            .route("/cache/warm", post(cache_warm_handler))
            .route("/embed", post(embed_handler))
            .route("/chunk", post(chunk_handler))
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

    #[tokio::test]
    async fn test_cache_warm_handler_empty_embedding_model_returns_400() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/cache/warm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"embedding_model": ""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let error_msg = json["message"].as_str().unwrap_or("");
        assert!(
            error_msg.contains("must not be empty"),
            "Expected empty embedding_model validation error, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_cache_warm_handler_whitespace_embedding_model_returns_400() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/cache/warm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"embedding_model": "   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[cfg(feature = "embeddings")]
    #[tokio::test]
    async fn test_embed_handler_invalid_preset_returns_400() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/embed")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"texts": ["hello"], "config": {"model": {"type": "preset", "name": "nonexistent_preset"}}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let error_msg = json["message"].as_str().unwrap_or("");
        assert!(
            error_msg.contains("Unknown embedding preset"),
            "Expected preset validation error, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_chunk_handler_max_characters_zero_returns_400() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chunk")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"text": "hello world", "chunker_type": "text", "config": {"max_characters": 0}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let error_msg = json["message"].as_str().unwrap_or("");
        assert!(
            error_msg.contains("max_characters must be between"),
            "Expected bounds error, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_chunk_handler_max_characters_too_large_returns_400() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chunk")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"text": "hello world", "chunker_type": "text", "config": {"max_characters": 2000000}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let error_msg = json["message"].as_str().unwrap_or("");
        assert!(
            error_msg.contains("max_characters must be between"),
            "Expected bounds error, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_chunk_handler_semantic_chunker_type_accepted() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chunk")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"text": "Hello world. This is a test.", "chunker_type": "semantic"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_chunk_handler_invalid_chunker_type_returns_400() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chunk")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text": "Hello world.", "chunker_type": "invalid"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let error_msg = json["message"].as_str().unwrap_or("");
        assert!(
            error_msg.contains("semantic"),
            "Error should list valid chunker types including semantic, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_chunk_handler_topic_threshold_accepted() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chunk")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"text": "Hello world. This is a test.", "chunker_type": "semantic", "config": {"topic_threshold": 0.5}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["config"]["topic_threshold"], 0.5);
    }
}
