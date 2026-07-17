//! Handlers for the fork-only `/v1/process` + rehydrate API surface.
//!
//! Fenced here (rather than in the upstream-shaped `api/handlers.rs`) so that
//! file stays byte-close to upstream `xberg-io/xberg`. Gated on the
//! `process-api` feature.

use axum::{Json, extract::State};
use bytes::Bytes;

use super::types::{ProcessRequest, ProcessResponse, RehydrateRequest, RehydrateResponse};
use crate::api::error::ApiError;
use crate::api::handlers::{ApiExtractInput, enforce_api_uri_policy, extract_unified_inputs};
use crate::api::types::ApiState;

/// Rehydration endpoint handler.
///
/// POST /v1/documents/{rehydration_key}/rehydrate
///
/// Retrieves the encrypted rehydration map stored by `POST /v1/process`,
/// decrypts it with the caller-supplied passphrase, and returns the
/// token → original-text map. Returns 404 if the key is absent or expired,
/// 403 if the passphrase is wrong.
#[cfg(feature = "process-api")]
#[cfg_attr(
    feature = "otel",
    tracing::instrument(name = "api.rehydrate", skip(state, request, rehydration_key))
)]
pub(crate) async fn rehydrate_handler(
    State(state): State<ApiState>,
    axum::extract::Path(rehydration_key): axum::extract::Path<String>,
    Json(request): Json<RehydrateRequest>,
) -> Result<Json<RehydrateResponse>, ApiError> {
    let ctx = xberg_doc_store::TenantCtx::default_tenant();
    let doc_id = xberg_doc_store::DocumentId(rehydration_key.clone());
    let encrypted = state
        .rehydration_store
        .get_map(&ctx, &doc_id)
        .await
        .map_err(|e| ApiError::internal(crate::error::XbergError::Other(e.to_string())))?
        .ok_or_else(|| ApiError {
            status: axum::http::StatusCode::NOT_FOUND,
            body: crate::api::types::ErrorResponse {
                error_type: "NotFoundError".to_string(),
                message: format!("Rehydration key '{rehydration_key}' not found or expired"),
                traceback: None,
                status_code: axum::http::StatusCode::NOT_FOUND.as_u16(),
            },
        })?;

    #[cfg(feature = "redaction-rehydrate")]
    let restored = crate::text::redaction::rehydration::decrypt_map(&encrypted, &request.passphrase)
        .map_err(|e| ApiError::new(axum::http::StatusCode::FORBIDDEN, e))?;

    #[cfg(not(feature = "redaction-rehydrate"))]
    {
        let _ = (&encrypted, &request.passphrase);
        Err(ApiError {
            status: axum::http::StatusCode::NOT_IMPLEMENTED,
            body: crate::api::types::ErrorResponse {
                error_type: "NotImplementedError".to_string(),
                message: "Rehydration requires the `redaction-rehydrate` feature".to_string(),
                traceback: None,
                status_code: axum::http::StatusCode::NOT_IMPLEMENTED.as_u16(),
            },
        })
    }

    #[cfg(feature = "redaction-rehydrate")]
    {
        tracing::info!(restored_count = restored.len(), "PII rehydration performed");

        Ok(Json(RehydrateResponse { restored }))
    }
}

/// Process endpoint handler.
///
/// POST /v1/process
///
/// Accepts a JSON body with `text` or `url` (mutually exclusive) and an
/// `operations` block that controls NER and redaction. When
/// `operations.redact.rehydrate` is `true` a passphrase must also be supplied;
/// the encrypted rehydration map is stored server-side and its key is returned
/// in the response.
#[cfg(feature = "process-api")]
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.process", skip(state, request)))]
pub(crate) async fn process_handler(
    State(state): State<ApiState>,
    Json(request): Json<ProcessRequest>,
) -> Result<Json<ProcessResponse>, ApiError> {
    let input = match (&request.text, &request.url) {
        (Some(text), None) => ApiExtractInput::bytes(
            Bytes::from(text.clone().into_bytes()),
            "text/plain".to_string(),
            None,
        ),
        (None, Some(url)) => ApiExtractInput::uri(url.clone(), None),
        (Some(_), Some(_)) => {
            return Err(ApiError::validation(crate::error::XbergError::validation(
                "Exactly one of `text` or `url` must be set, not both",
            )));
        }
        (None, None) => {
            return Err(ApiError::validation(crate::error::XbergError::validation(
                "Exactly one of `text` or `url` must be set",
            )));
        }
    };

    enforce_api_uri_policy(std::slice::from_ref(&input))?;

    let rehydrate = request.operations.redact.as_ref().map(|r| r.rehydrate).unwrap_or(false);

    if rehydrate {
        // Fail fast on both gating checks — missing feature or missing passphrase —
        // before running the (potentially expensive) extraction.
        #[cfg(not(feature = "redaction-rehydrate"))]
        {
            Err(ApiError {
                status: axum::http::StatusCode::NOT_IMPLEMENTED,
                body: crate::api::types::ErrorResponse {
                    error_type: "NotImplementedError".to_string(),
                    message: "Rehydration requires the `redaction-rehydrate` feature".to_string(),
                    traceback: None,
                    status_code: axum::http::StatusCode::NOT_IMPLEMENTED.as_u16(),
                },
            })
        }

        #[cfg(feature = "redaction-rehydrate")]
        {
            let redact_op = request.operations.redact.as_ref().expect("checked above");
            let passphrase = redact_op.passphrase.as_deref().ok_or_else(|| {
                ApiError::validation(crate::error::XbergError::validation(
                    "operations.redact.passphrase is required when operations.redact.rehydrate is true",
                ))
            })?;
            if passphrase.trim().is_empty() {
                return Err(ApiError::validation(crate::error::XbergError::validation(
                    "operations.redact.passphrase must not be empty",
                )));
            }

            let mut config = (*state.default_config).clone();
            config.ner = request.operations.ner.clone();
            // Extraction must not auto-redact here: the default_config's own
            // redaction post-processor (if configured) would consume the
            // original text before redact_capturing_rehydration_map below
            // runs, leaving the rehydration map empty or incomplete.
            config.redaction = None;

            let mut results = extract_unified_inputs(vec![input], config).await?;
            let mut document = results.results.pop().ok_or_else(|| {
                ApiError::internal(crate::error::XbergError::Other(
                    "extraction produced no document".into(),
                ))
            })?;

            let outcome = crate::text::redaction::redact_capturing_rehydration_map(&mut document, &redact_op.config)
                .await
                .map_err(ApiError::from)?;
            if !outcome.rejection_counts.is_empty() {
                tracing::debug!(
                    target: "xberg::redaction",
                    rejections = ?outcome.rejection_counts,
                    "post-detection validators rejected candidate PII matches"
                );
            }
            let encrypted = crate::text::redaction::encrypt_map(&outcome.map, passphrase).map_err(ApiError::from)?;
            let doc_id = state
                .rehydration_store
                .put_map(&xberg_doc_store::TenantCtx::default_tenant(), encrypted)
                .await
                .map_err(|e| ApiError::internal(crate::error::XbergError::Other(e.to_string())))?;

            Ok(Json(ProcessResponse {
                document,
                rehydration_key: Some(doc_id.0),
            }))
        }
    } else {
        let mut config = (*state.default_config).clone();
        config.ner = request.operations.ner.clone();
        if let Some(redact_op) = &request.operations.redact {
            config.redaction = Some(redact_op.config.clone());
        }

        let mut results = extract_unified_inputs(vec![input], config).await?;
        let document = results.results.pop().ok_or_else(|| {
            ApiError::internal(crate::error::XbergError::Other(
                "extraction produced no document".into(),
            ))
        })?;
        Ok(Json(ProcessResponse {
            document,
            rehydration_key: None,
        }))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[cfg(feature = "process-api")]
    pub(crate) fn make_api_state() -> ApiState {
        let extraction_service = crate::service::ExtractionServiceBuilder::new().build();
        ApiState {
            default_config: std::sync::Arc::new(crate::ExtractionConfig::default()),
            extraction_service: std::sync::Arc::new(std::sync::Mutex::new(extraction_service)),
            job_store: std::sync::Arc::new(crate::api::jobs::JobStore::new()),
            rehydration_store: std::sync::Arc::new(xberg_doc_store::backends::memory::InMemoryRehydrationStore::new()),
        }
    }

    #[cfg(feature = "process-api")]
    #[cfg(feature = "redaction-rehydrate")]
    #[tokio::test]
    async fn process_handler_rehydrate_map_is_populated_when_default_config_also_redacts() {
        // Regression test: server-side default_config.redaction must not run during
        // extraction when rehydrate=true, or redact_capturing_rehydration_map sees
        // already-redacted text and captures an empty/incomplete map.
        use super::super::types::{ProcessOperations, ProcessRedactOperation, ProcessRequest};
        let mut config = crate::ExtractionConfig::default();
        config.redaction = Some(crate::core::config::redaction::RedactionConfig {
            strategy: crate::types::redaction::RedactionStrategy::Mask,
            ..Default::default()
        });
        let state = ApiState {
            default_config: std::sync::Arc::new(config),
            ..make_api_state()
        };
        let request = ProcessRequest {
            text: Some("Contact Alice at alice@example.com.".to_string()),
            url: None,
            operations: ProcessOperations {
                ner: None,
                redact: Some(ProcessRedactOperation {
                    config: crate::core::config::redaction::RedactionConfig {
                        strategy: crate::types::redaction::RedactionStrategy::TokenReplace,
                        ..Default::default()
                    },
                    rehydrate: true,
                    passphrase: Some("test-passphrase".to_string()),
                }),
            },
        };
        let response = process_handler(axum::extract::State(state.clone()), axum::extract::Json(request))
            .await
            .expect("handler must succeed");
        let rehydration_key = response.0.rehydration_key.clone().expect("rehydrate=true must return a key");
        assert!(
            !response.0.document.content.contains("alice@example.com"),
            "document must be redacted"
        );

        let restored = rehydrate_handler(
            axum::extract::State(state),
            axum::extract::Path(rehydration_key),
            axum::extract::Json(super::super::types::RehydrateRequest {
                passphrase: "test-passphrase".to_string(),
            }),
        )
        .await
        .expect("rehydrate must succeed");
        assert!(
            restored.0.restored.values().any(|v| v == "alice@example.com"),
            "rehydration map must contain the original email, got: {:?}",
            restored.0.restored
        );
    }

    #[cfg(feature = "process-api")]
    #[cfg(feature = "redaction-rehydrate")]
    #[tokio::test]
    async fn process_handler_redacts_email_with_mask_strategy() {
        use super::super::types::{ProcessOperations, ProcessRedactOperation, ProcessRequest};
        let state = make_api_state();
        let request = ProcessRequest {
            text: Some("Contact Alice at alice@example.com.".to_string()),
            url: None,
            operations: ProcessOperations {
                ner: None,
                redact: Some(ProcessRedactOperation {
                    config: crate::core::config::redaction::RedactionConfig {
                        strategy: crate::types::redaction::RedactionStrategy::Mask,
                        ..Default::default()
                    },
                    rehydrate: false,
                    passphrase: None,
                }),
            },
        };
        let response = process_handler(axum::extract::State(state), axum::extract::Json(request))
            .await
            .expect("handler must succeed");
        assert!(response.0.document.content.contains("[REDACTED]"));
        assert!(!response.0.document.content.contains("alice@example.com"));
        assert!(response.0.rehydration_key.is_none());
    }

    #[cfg(feature = "process-api")]
    #[cfg(feature = "redaction-rehydrate")]
    #[tokio::test]
    async fn process_handler_requires_passphrase_when_rehydrate_is_true() {
        use super::super::types::{ProcessOperations, ProcessRedactOperation, ProcessRequest};
        let state = make_api_state();
        let request = ProcessRequest {
            text: Some("Contact Alice at alice@example.com.".to_string()),
            url: None,
            operations: ProcessOperations {
                ner: None,
                redact: Some(ProcessRedactOperation {
                    config: crate::core::config::redaction::RedactionConfig {
                        strategy: crate::types::redaction::RedactionStrategy::TokenReplace,
                        ..Default::default()
                    },
                    rehydrate: true,
                    passphrase: None,
                }),
            },
        };
        let result = process_handler(axum::extract::State(state), axum::extract::Json(request)).await;
        assert!(result.is_err(), "must reject rehydrate=true without a passphrase");
    }

    #[cfg(feature = "process-api")]
    #[tokio::test]
    async fn process_handler_rejects_both_text_and_url() {
        use super::super::types::{ProcessOperations, ProcessRequest};
        let state = make_api_state();
        let request = ProcessRequest {
            text: Some("hello".to_string()),
            url: Some("https://example.com/doc.txt".to_string()),
            operations: ProcessOperations::default(),
        };
        let result = process_handler(axum::extract::State(state), axum::extract::Json(request)).await;
        assert!(result.is_err(), "must reject when both text and url are set");
    }

    #[cfg(feature = "process-api")]
    #[tokio::test]
    async fn rehydrate_handler_returns_404_for_unknown_key() {
        let state = make_api_state();
        let result = rehydrate_handler(
            axum::extract::State(state),
            axum::extract::Path("reh_does_not_exist".to_string()),
            axum::extract::Json(super::super::types::RehydrateRequest {
                passphrase: "anything".to_string(),
            }),
        )
        .await;
        let err = result.expect_err("unknown key must error");
        assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
    }

    #[cfg(all(feature = "process-api", feature = "redaction-rehydrate"))]
    #[tokio::test]
    async fn rehydrate_handler_round_trips_a_stored_map() {
        let state = make_api_state();
        let mut map = std::collections::HashMap::new();
        map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());
        let encrypted = crate::text::redaction::rehydration::encrypt_map(&map, "test-passphrase").expect("encrypt");
        let doc_id = state
            .rehydration_store
            .put_map(&xberg_doc_store::TenantCtx::default_tenant(), encrypted)
            .await
            .expect("put_map");
        let response = rehydrate_handler(
            axum::extract::State(state),
            axum::extract::Path(doc_id.0),
            axum::extract::Json(super::super::types::RehydrateRequest {
                passphrase: "test-passphrase".to_string(),
            }),
        )
        .await
        .expect("rehydrate must succeed");
        assert_eq!(
            response.0.restored.get("[EMAIL_1]"),
            Some(&"alice@example.com".to_string())
        );
    }

    #[cfg(all(feature = "process-api", feature = "redaction-rehydrate"))]
    #[tokio::test]
    async fn rehydrate_handler_rejects_wrong_passphrase() {
        let state = make_api_state();
        let mut map = std::collections::HashMap::new();
        map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());
        let encrypted = crate::text::redaction::rehydration::encrypt_map(&map, "correct").expect("encrypt");
        let doc_id = state
            .rehydration_store
            .put_map(&xberg_doc_store::TenantCtx::default_tenant(), encrypted)
            .await
            .expect("put_map");
        let result = rehydrate_handler(
            axum::extract::State(state),
            axum::extract::Path(doc_id.0),
            axum::extract::Json(super::super::types::RehydrateRequest {
                passphrase: "wrong".to_string(),
            }),
        )
        .await;
        let err = result.expect_err("wrong passphrase must error");
        assert_eq!(err.status, axum::http::StatusCode::FORBIDDEN);
    }
}
