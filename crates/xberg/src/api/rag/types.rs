//! Request/response types for the fork-only `/v1/process` + rehydrate API surface.
//!
//! Fenced here (rather than in the upstream-shaped `api/types.rs`) so that
//! file stays byte-close to upstream `xberg-io/xberg`. Gated on the
//! `process-api` feature.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// POST /v1/process types
// ---------------------------------------------------------------------------

/// Request body for `POST /v1/process`. JSON only (text or URL input).
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessRequest {
    /// Inline text to process (mutually exclusive with `url`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// URL to fetch and process (mutually exclusive with `text`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Pipeline operations to run after extraction.
    #[serde(default)]
    pub operations: ProcessOperations,
}

/// Operations to apply in the process pipeline.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessOperations {
    /// Named-entity recognition configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ner: Option<crate::core::config::ner::NerConfig>,
    /// Redaction configuration and options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redact: Option<ProcessRedactOperation>,
}

/// Redaction operation within `POST /v1/process`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessRedactOperation {
    /// Redaction configuration (strategy, categories, etc.).
    #[serde(flatten)]
    pub config: crate::core::config::redaction::RedactionConfig,
    /// When `true`, capture a rehydration map so PII can be restored later.
    /// Requires `passphrase`.
    #[serde(default)]
    pub rehydrate: bool,
    /// Passphrase used to encrypt the rehydration map.
    /// Required when `rehydrate` is `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub passphrase: Option<String>,
}

/// Response from `POST /v1/process`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessResponse {
    /// The extracted (and optionally redacted) document.
    pub document: crate::types::ExtractedDocument,
    /// Key to pass to `POST /v1/documents/{rehydration_key}/rehydrate`.
    /// Only present when `operations.redact.rehydrate` was `true` and
    /// the `redaction-rehydrate` feature is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rehydration_key: Option<String>,
}

// ---------------------------------------------------------------------------
// POST /v1/documents/{id}/rehydrate types
// ---------------------------------------------------------------------------

/// Request body for `POST /v1/documents/{rehydration_key}/rehydrate`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RehydrateRequest {
    /// Passphrase the map was encrypted with (`operations.redact.passphrase`
    /// from the originating `/v1/process` call). Never logged or cached
    /// beyond this request.
    pub passphrase: String,
}

/// Response body for `POST /v1/documents/{rehydration_key}/rehydrate`:
/// the decrypted token → original-text map. Callers substitute tokens back
/// into their own copy of the redacted document.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RehydrateResponse {
    pub restored: std::collections::HashMap<String, String>,
}
