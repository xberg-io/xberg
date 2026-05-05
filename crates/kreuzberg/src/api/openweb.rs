//! OpenWebUI compatibility handlers.
//!
//! Provides endpoints compatible with OpenWebUI's Content Extraction Engine:
//!
//! - `PUT /process` — "External" engine: raw binary body, returns `{page_content, metadata}`
//! - `POST /v1/convert/file` — "Docling" engine: multipart form-data, returns `{document: {md_content}, status}`

use axum::{Json, body::Bytes, extract::State, http::HeaderMap};
use tower::Service;

use crate::service::ExtractionRequest;

use super::{
    error::{ApiError, MultipartApi},
    types::{ApiState, DoclingCompatDocument, DoclingCompatResponse, OpenWebDocumentMetadata, OpenWebDocumentResponse},
};

/// OpenWebUI "External" engine handler.
///
/// PUT /process
///
/// Accepts raw binary file content in the request body.
/// Uses `Content-Type` header for MIME type and `X-Filename` header for the filename.
///
/// Returns a JSON document matching OpenWebUI's external document loader contract.
#[utoipa::path(
    put,
    path = "/process",
    tag = "openweb",
    request_body(content_type = "application/octet-stream", content = Vec<u8>),
    responses(
        (status = 200, description = "Document extracted", body = OpenWebDocumentResponse),
        (status = 400, description = "Bad request", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg_attr(
    feature = "otel",
    tracing::instrument(name = "api.openweb_process", skip(state, headers, body))
)]
pub(crate) async fn openweb_external_handler(
    State(state): State<ApiState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<OpenWebDocumentResponse>, ApiError> {
    if body.is_empty() {
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(
            "Empty request body — upload a file as the raw request body",
        )));
    }

    // Extract MIME type from Content-Type header, stripping any parameters (e.g. charset)
    let mime_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or(v).trim())
        .unwrap_or(crate::core::mime::OCTET_STREAM_MIME_TYPE)
        .to_string();

    // Extract filename from X-Filename header (URL-encoded by OpenWebUI)
    let filename = headers
        .get("X-Filename")
        .and_then(|v| v.to_str().ok())
        .map(|v| urlencoding::decode(v).unwrap_or_else(|_| v.into()).into_owned())
        .unwrap_or_else(|| "unknown".to_string());

    // Detect MIME type from filename if the header was generic
    let mime_type = if mime_type == crate::core::mime::OCTET_STREAM_MIME_TYPE {
        crate::core::mime::detect_mime_type(&filename, false).unwrap_or(mime_type)
    } else {
        mime_type
    };

    // Build extraction config with markdown output
    let mut config = (*state.default_config).clone();
    config.output_format = crate::core::config::OutputFormat::Markdown;

    let request = ExtractionRequest::bytes(body.to_vec(), mime_type, config);
    let mut svc = state
        .extraction_service
        .lock()
        .expect("extraction service lock poisoned")
        .clone();
    let result = svc.call(request).await?;

    Ok(Json(OpenWebDocumentResponse {
        page_content: result.content,
        metadata: OpenWebDocumentMetadata { source: filename },
    }))
}

/// OpenWebUI "Docling" engine handler (docling-serve compatible).
///
/// POST /v1/convert/file
///
/// Accepts multipart form-data with a `files` field containing the document.
/// Returns a JSON response matching docling-serve's `/v1/convert/file` contract.
///
/// OpenWebUI reads only `document.md_content` from the response.
#[utoipa::path(
    post,
    path = "/v1/convert/file",
    tag = "openweb",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Document converted", body = DoclingCompatResponse),
        (status = 400, description = "Bad request", body = crate::api::types::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::api::types::ErrorResponse),
    )
)]
#[cfg_attr(
    feature = "otel",
    tracing::instrument(name = "api.openweb_docling", skip(state, multipart))
)]
pub(crate) async fn openweb_docling_handler(
    State(state): State<ApiState>,
    MultipartApi(mut multipart): MultipartApi,
) -> Result<Json<DoclingCompatResponse>, ApiError> {
    let mut file_data: Option<(Vec<u8>, String)> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "files" || field_name == "file" {
            let file_name = field.file_name().map(|s| s.to_string());
            let content_type = field.content_type().map(|s| s.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;

            let mut mime_type = content_type.unwrap_or_else(|| crate::core::mime::OCTET_STREAM_MIME_TYPE.to_string());

            // Detect from filename if generic
            if mime_type == crate::core::mime::OCTET_STREAM_MIME_TYPE
                && let Some(ref name) = file_name
                && let Ok(detected) = crate::core::mime::detect_mime_type(name, false)
            {
                mime_type = detected;
            }

            file_data = Some((data.to_vec(), mime_type));
            break;
        }
    }

    let (data, mime_type) = file_data.ok_or_else(|| {
        ApiError::validation(crate::error::KreuzbergError::validation(
            "No file provided. Upload a file with field name 'files'.",
        ))
    })?;

    // Build extraction config with markdown output
    let mut config = (*state.default_config).clone();
    config.output_format = crate::core::config::OutputFormat::Markdown;

    let request = ExtractionRequest::bytes(data, mime_type, config);
    let mut svc = state
        .extraction_service
        .lock()
        .expect("extraction service lock poisoned")
        .clone();
    let result = svc.call(request).await?;

    Ok(Json(DoclingCompatResponse {
        document: DoclingCompatDocument {
            md_content: result.content,
        },
        status: "success".to_string(),
    }))
}
