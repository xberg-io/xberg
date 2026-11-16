//! API error handling.

use axum::{
    Json,
    body::to_bytes,
    extract::{FromRequest, Multipart, Request, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;

use crate::core::config::remote_errors::public_error_message;
use crate::error::{ApiStatusCategory, XbergError};

use super::types::ErrorResponse;

/// Custom JSON extractor that returns JSON error responses instead of plain text.
///
/// This wraps axum's `Json` extractor but uses `ApiError` as the rejection type,
/// ensuring that all JSON parsing errors are returned as JSON with proper content type.
///
/// Additionally, this extractor validates that the root JSON value is an object (not an array),
/// which prevents serde from incorrectly deserializing JSON arrays into struct fields.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(alef, alef(skip))]
pub struct JsonApi<T>(pub T);

impl<T, S> FromRequest<S> for JsonApi<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let bytes = to_bytes(body, usize::MAX).await.map_err(|_| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                XbergError::Other("Failed to read request body".to_string()),
            )
        })?;

        if !bytes.is_empty() {
            let trimmed = std::str::from_utf8(&bytes).unwrap_or("").trim_start();
            if trimmed.starts_with('[') {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    XbergError::validation(
                        "Expected JSON object, but received JSON array. \
                         Please wrap your data in an object with appropriate fields.",
                    ),
                ));
            }
        }

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
#[cfg_attr(alef, alef(skip))]
#[derive(Debug)]
pub struct ApiError {
    /// HTTP status code
    pub status: StatusCode,
    /// Error response body
    pub body: ErrorResponse,
}

impl ApiError {
    /// Create a new API error.
    ///
    /// The client gets the error kind and its public message: a fixed one for filesystem-backed
    /// errors, otherwise the text with server paths replaced by `<path>`. The full message is
    /// logged for server errors and for client errors whose message changed.
    pub(crate) fn new(status: StatusCode, error: XbergError) -> Self {
        let error_type = error.api_error_type();
        let full_message = error.to_string();
        let message = public_error_message(&error, &full_message);
        if status.is_server_error() {
            tracing::error!(status = status.as_u16(), error_type, error = %full_message, "API request failed");
        } else if message != full_message {
            tracing::warn!(status = status.as_u16(), error_type, error = %full_message, "API request rejected");
        }

        Self::with_message(status, error_type, message)
    }

    fn with_message(status: StatusCode, error_type: &str, message: String) -> Self {
        Self {
            status,
            body: ErrorResponse {
                error_type: error_type.to_string(),
                message,
                traceback: None,
                status_code: status.as_u16(),
            },
        }
    }

    /// Create a validation error (400) for a caller config rejected at the trust boundary.
    ///
    /// Not path-redacted: these messages name config fields, some as JSON pointers such as
    /// `/browser/proxy`, and never carry a server value.
    pub(crate) fn caller_config(message: String) -> Self {
        let error = XbergError::validation(message);
        Self::with_message(StatusCode::BAD_REQUEST, error.api_error_type(), error.to_string())
    }

    /// Create a validation error (400).
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn validation(error: XbergError) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error)
    }

    /// Create an unprocessable entity error (422).
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn unprocessable(error: XbergError) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, error)
    }

    /// Create an internal server error (500).
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn internal(error: XbergError) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error)
    }

    /// Create a bad gateway error (502).
    ///
    /// Use when an upstream service (e.g., model download from HuggingFace) fails.
    #[cfg(any(
        paddle_ocr,
        feature = "layout-detection",
        feature = "embeddings",
        feature = "ner-onnx"
    ))]
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn bad_gateway(error: XbergError) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<XbergError> for ApiError {
    fn from(error: XbergError) -> Self {
        match error.api_status_category() {
            ApiStatusCategory::Validation => Self::validation(error),
            ApiStatusCategory::Unprocessable => Self::unprocessable(error),
            ApiStatusCategory::Internal => Self::internal(error),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_use_bad_request_for_validation_and_unsupported_format_errors() {
        let validation = ApiError::from(XbergError::validation("bad input"));
        let unsupported = ApiError::from(XbergError::UnsupportedFormat("application/unknown".to_string()));

        assert_eq!(validation.status, StatusCode::BAD_REQUEST);
        assert_eq!(validation.body.error_type, "ValidationError");
        assert_eq!(validation.body.status_code, StatusCode::BAD_REQUEST.as_u16());

        assert_eq!(unsupported.status, StatusCode::BAD_REQUEST);
        assert_eq!(unsupported.body.error_type, "UnsupportedFormatError");
    }

    #[test]
    fn should_use_unprocessable_entity_for_parsing_and_ocr_errors() {
        let parsing = ApiError::from(XbergError::parsing("corrupt file"));
        let ocr = ApiError::from(XbergError::ocr("ocr failed"));

        assert_eq!(parsing.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(parsing.body.error_type, "ParsingError");

        assert_eq!(ocr.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(ocr.body.error_type, "OCRError");
    }

    #[test]
    fn should_use_internal_server_error_for_every_other_variant() {
        let io = ApiError::from(XbergError::Io(std::io::Error::other("disk failure")));
        let cancelled = ApiError::from(XbergError::Cancelled);
        let other = ApiError::from(XbergError::Other("unexpected".to_string()));

        assert_eq!(io.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(io.body.error_type, "IOError");

        assert_eq!(cancelled.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(cancelled.body.error_type, "CancelledError");

        assert_eq!(other.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(other.body.error_type, "Error");
    }

    #[test]
    fn should_redact_server_paths_but_keep_error_kind_and_message() {
        let cases = [
            (
                XbergError::parsing("Failed to open /srv/xberg/models/layout.onnx: permission denied"),
                StatusCode::UNPROCESSABLE_ENTITY,
                "ParsingError",
                "Parsing error: Failed to open <path>: permission denied",
                "/srv/xberg/models",
            ),
            (
                XbergError::Io(std::io::Error::other(
                    "cannot read '/home/operator/.cache/xberg/ocr.bin'",
                )),
                StatusCode::INTERNAL_SERVER_ERROR,
                "IOError",
                "IO error: a file could not be read or written",
                "/home/operator",
            ),
            (
                XbergError::ocr("tessdata missing at C:\\xberg\\tessdata."),
                StatusCode::UNPROCESSABLE_ENTITY,
                "OCRError",
                "OCR error: tessdata missing at <path>.",
                "xberg\\tessdata",
            ),
            (
                XbergError::Other("cache_dir=/var/cache/xberg is full".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error",
                "cache_dir=<path> is full",
                "/var/cache",
            ),
            (
                XbergError::validation("Cannot read file:///etc/passwd or ~/secrets.txt (\\\\fileserver\\share)"),
                StatusCode::BAD_REQUEST,
                "ValidationError",
                "Validation error: Cannot read <path> or <path> (<path>)",
                "passwd",
            ),
        ];

        for (error, status, error_type, message, leaked) in cases {
            let api_error = ApiError::from(error);
            assert_eq!(api_error.status, status, "{message}");
            assert_eq!(api_error.body.error_type, error_type, "{message}");
            assert_eq!(api_error.body.message, message);
            assert!(!api_error.body.message.contains(leaked), "{leaked} leaked");
        }
    }

    #[test]
    fn should_give_filesystem_errors_a_fixed_public_message() {
        let cases = [
            (
                XbergError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "File does not exist: \"/srv/my docs/a.pdf\"",
                )),
                StatusCode::INTERNAL_SERVER_ERROR,
                "IOError",
                "IO error: file not found",
            ),
            (
                XbergError::cache("cannot write 'C:\\Program Files\\x\\a.pdf'"),
                StatusCode::INTERNAL_SERVER_ERROR,
                "CacheError",
                "Cache error: the cache could not be read or written",
            ),
        ];

        for (error, status, error_type, message) in cases {
            let api_error = ApiError::from(error);
            assert_eq!(api_error.status, status);
            assert_eq!(api_error.body.error_type, error_type);
            assert_eq!(api_error.body.message, message);
            for fragment in ["srv", "my docs", "a.pdf", "Program", "Files"] {
                assert!(!api_error.body.message.contains(fragment), "{fragment} survived");
            }
        }
    }

    #[test]
    fn should_keep_urls_mime_types_and_whitespace_when_redacting() {
        for message in [
            "Validation error: Local path and file:// URI extraction are disabled for the HTTP API.",
            "Failed to fetch https://example.com/docs/report.pdf: HTTP 404",
            "Unsupported format: application/x-unsupported-format",
            "no paths here",
        ] {
            assert_eq!(crate::core::config::remote_errors::redact_paths(message), message);
        }
        assert_eq!(
            crate::core::config::remote_errors::redact_paths("a  /srv/x\tb\n"),
            "a  <path>\tb\n"
        );
    }

    #[test]
    fn should_keep_json_pointers_in_caller_config_rejections() {
        let message = "Caller extraction config contains forbidden url.crawl setting: \
                       invalid_config: untrusted caller may not set /browser/proxy";

        let api_error = ApiError::caller_config(message.to_string());

        assert_eq!(api_error.status, StatusCode::BAD_REQUEST);
        assert_eq!(api_error.body.error_type, "ValidationError");
        assert_eq!(api_error.body.message, format!("Validation error: {message}"));
    }

    #[test]
    fn should_report_every_variant_error_type_via_api_error_new() {
        let cases: [(XbergError, &str); 18] = [
            (XbergError::validation("t"), "ValidationError"),
            (XbergError::parsing("t"), "ParsingError"),
            (XbergError::ocr("t"), "OCRError"),
            (XbergError::Io(std::io::Error::other("t")), "IOError"),
            (XbergError::cache("t"), "CacheError"),
            (XbergError::image_processing("t"), "ImageProcessingError"),
            (XbergError::serialization("t"), "SerializationError"),
            (XbergError::MissingDependency("t".to_string()), "MissingDependencyError"),
            (
                XbergError::Plugin {
                    message: "t".to_string(),
                    plugin_name: "p".to_string(),
                },
                "PluginError",
            ),
            (XbergError::LockPoisoned("t".to_string()), "LockPoisonedError"),
            (
                XbergError::UnsupportedFormat("t/mime".to_string()),
                "UnsupportedFormatError",
            ),
            (XbergError::embedding("t"), "EmbeddingError"),
            (
                XbergError::Timeout {
                    elapsed_ms: 1,
                    limit_ms: 1,
                },
                "TimeoutError",
            ),
            (XbergError::Other("t".to_string()), "Error"),
            (XbergError::Cancelled, "CancelledError"),
            (XbergError::security("t"), "SecurityError"),
            (XbergError::transcription("t"), "TranscriptionError"),
            (XbergError::reranking("t"), "RerankingError"),
        ];

        for (error, expected_type) in cases {
            let api_error = ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, error);
            assert_eq!(api_error.body.error_type, expected_type);
        }
    }
}
