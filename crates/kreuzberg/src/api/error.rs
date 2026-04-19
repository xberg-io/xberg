//! API error handling.

use axum::{
    Json,
    body::to_bytes,
    extract::{FromRequest, Multipart, Request, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;

use crate::error::KreuzbergError;

use super::types::ErrorResponse;

/// Custom JSON extractor that returns JSON error responses instead of plain text.
///
/// This wraps axum's `Json` extractor but uses `ApiError` as the rejection type,
/// ensuring that all JSON parsing errors are returned as JSON with proper content type.
///
/// Additionally, this extractor validates that the root JSON value is an object (not an array),
/// which prevents serde from incorrectly deserializing JSON arrays into struct fields.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonApi<T>(pub T);

impl<T, S> FromRequest<S> for JsonApi<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // First, extract the body to check if it's a valid JSON object (not array)
        let (parts, body) = req.into_parts();
        let bytes = to_bytes(body, usize::MAX).await.map_err(|_| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                KreuzbergError::Other("Failed to read request body".to_string()),
            )
        })?;

        // Validate that the root JSON is an object, not an array
        if !bytes.is_empty() {
            let trimmed = std::str::from_utf8(&bytes).unwrap_or("").trim_start();
            if trimmed.starts_with('[') {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    KreuzbergError::validation(
                        "Expected JSON object, but received JSON array. \
                         Please wrap your data in an object with appropriate fields.",
                    ),
                ));
            }
        }

        // Reconstruct the request and use the standard Json extractor
        let req = Request::from_parts(parts, axum::body::Body::from(bytes));
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(JsonApi(value)),
            Err(rejection) => Err(ApiError::from(rejection)),
        }
    }
}

/// Custom Multipart extractor that returns JSON error responses instead of plain text.
///
/// This wraps axum's `Multipart` extractor but uses `ApiError` as the rejection type,
/// ensuring that multipart parsing errors are returned as JSON with proper content type.
pub struct MultipartApi(pub Multipart);

impl<S> FromRequest<S> for MultipartApi
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Multipart::from_request(req, state).await {
            Ok(multipart) => Ok(MultipartApi(multipart)),
            Err(rejection) => Err(ApiError {
                status: StatusCode::BAD_REQUEST,
                body: ErrorResponse {
                    error_type: "MultipartError".to_string(),
                    message: rejection.body_text(),
                    traceback: None,
                    status_code: StatusCode::BAD_REQUEST.as_u16(),
                },
            }),
        }
    }
}

/// API-specific error wrapper.
#[derive(Debug)]
pub struct ApiError {
    /// HTTP status code
    pub status: StatusCode,
    /// Error response body
    pub body: ErrorResponse,
}

impl ApiError {
    /// Create a new API error.
    pub fn new(status: StatusCode, error: KreuzbergError) -> Self {
        let error_type = match &error {
            KreuzbergError::Validation { .. } => "ValidationError",
            KreuzbergError::Parsing { .. } => "ParsingError",
            KreuzbergError::Ocr { .. } => "OCRError",
            KreuzbergError::Io(_) => "IOError",
            KreuzbergError::Cache { .. } => "CacheError",
            KreuzbergError::ImageProcessing { .. } => "ImageProcessingError",
            KreuzbergError::Serialization { .. } => "SerializationError",
            KreuzbergError::MissingDependency(_) => "MissingDependencyError",
            KreuzbergError::Plugin { .. } => "PluginError",
            KreuzbergError::LockPoisoned(_) => "LockPoisonedError",
            KreuzbergError::UnsupportedFormat(_) => "UnsupportedFormatError",
            KreuzbergError::Embedding { .. } => "EmbeddingError",
            KreuzbergError::Timeout { .. } => "TimeoutError",
            KreuzbergError::Other(_) => "Error",
            KreuzbergError::Cancelled => "CancelledError",
        };

        Self {
            status,
            body: ErrorResponse {
                error_type: error_type.to_string(),
                message: error.to_string(),
                traceback: None,
                status_code: status.as_u16(),
            },
        }
    }

    /// Create a validation error (400).
    pub fn validation(error: KreuzbergError) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error)
    }

    /// Create an unprocessable entity error (422).
    pub fn unprocessable(error: KreuzbergError) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, error)
    }

    /// Create an internal server error (500).
    pub fn internal(error: KreuzbergError) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error)
    }

    /// Create a bad gateway error (502).
    ///
    /// Use when an upstream service (e.g., model download from HuggingFace) fails.
    pub fn bad_gateway(error: KreuzbergError) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<KreuzbergError> for ApiError {
    fn from(error: KreuzbergError) -> Self {
        match &error {
            KreuzbergError::Validation { .. } | KreuzbergError::UnsupportedFormat(_) => Self::validation(error),
            KreuzbergError::Parsing { .. } | KreuzbergError::Ocr { .. } => Self::unprocessable(error),
            _ => Self::internal(error),
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        let (status, message) = match rejection {
            JsonRejection::JsonDataError(err) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!(
                    "Failed to deserialize the JSON body into the target type: {}",
                    err.body_text()
                ),
            ),
            JsonRejection::JsonSyntaxError(err) => (
                StatusCode::BAD_REQUEST,
                format!("Failed to parse the request body as JSON: {}", err.body_text()),
            ),
            JsonRejection::MissingJsonContentType(_) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected request with `Content-Type: application/json`".to_string(),
            ),
            JsonRejection::BytesRejection(err) => {
                (StatusCode::BAD_REQUEST, format!("Failed to read request body: {}", err))
            }
            _ => (StatusCode::BAD_REQUEST, "Unknown JSON parsing error".to_string()),
        };

        Self {
            status,
            body: ErrorResponse {
                error_type: "JsonParsingError".to_string(),
                message,
                traceback: None,
                status_code: status.as_u16(),
            },
        }
    }
}
