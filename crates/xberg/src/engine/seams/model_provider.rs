//! The [`ModelProvider`] seam: on-demand resolution of model weights to a path.
//!
//! The in-core default is [`DefaultModelProvider`], which delegates to
//! [`LayoutModelManager`](crate::layout::LayoutModelManager) — the on-demand
//! download/cache path xberg uses today. Alternative providers (a pre-warmed
//! mirror, an air-gapped bundle) implement this trait and are injected via
//! [`EngineBuilder::with_model_provider`](super::super::EngineBuilder::with_model_provider).
//!
//! Gated behind `layout-detection`: that is the feature under which the model
//! manager (and the ORT/HF download stack it drives) exists.

use std::path::PathBuf;

use async_trait::async_trait;

use crate::Result;
use crate::XbergError;
use crate::layout::{LayoutError, LayoutModelManager};

/// Minimal identifier for a downloadable model.
///
/// The `kind` is the canonical model-type key understood by the underlying
/// model manager (e.g. `"rtdetr"`, `"tatr"`, `"pp_doclayout_v3"`,
/// `"table_classifier"`, `"slanet_wired"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelId {
    /// Canonical model-type key.
    pub kind: String,
}

impl ModelId {
    /// Construct a [`ModelId`] from a model-type key.
    pub fn new(kind: impl Into<String>) -> Self {
        Self { kind: kind.into() }
    }
}

/// Resolves a [`ModelId`] to a local filesystem path, downloading on demand.
///
/// # Thread safety
///
/// Implementations are `Send + Sync + 'static` and held behind
/// `Arc<dyn ModelProvider>`; they may be called concurrently.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait ModelProvider: Send + Sync + 'static {
    /// Ensure the model exists locally, downloading if needed, and return its path.
    ///
    /// # Errors
    ///
    /// Returns an error if the model type is unknown or the download/verify
    /// step fails.
    async fn ensure_model(&self, model: &ModelId) -> Result<PathBuf>;
}

/// In-core default: resolution via [`LayoutModelManager`]'s on-demand path.
///
/// Wraps the same manager the extraction pipeline uses, so a resolved path is
/// identical to the manager's existing on-demand download/cache behavior. The
/// blocking download runs on a blocking thread to keep the async call non-blocking.
#[derive(Debug, Clone)]
pub struct DefaultModelProvider {
    manager: LayoutModelManager,
}

impl Default for DefaultModelProvider {
    fn default() -> Self {
        Self {
            manager: LayoutModelManager::new(None),
        }
    }
}

/// Route a model-type key through the matching public manager entry point,
/// re-categorizing the manager's [`LayoutError`] into an [`XbergError`].
///
/// `ensure_slanet_model` is the public by-variant entry point; it delegates to
/// the same private `ensure_model(model_type)` lookup as the named helpers, so
/// it correctly handles the `slanet_*` variants and rejects any unrecognized
/// key. The resulting [`LayoutError`] is not preserved verbatim: see
/// [`map_layout_error`] for how an unknown key becomes a validation error while
/// download/verify failures become a dependency error.
fn ensure_by_kind(manager: &LayoutModelManager, kind: &str) -> Result<PathBuf> {
    let resolved = match kind {
        "rtdetr" => manager.ensure_rtdetr_model(),
        "tatr" => manager.ensure_tatr_model(),
        "table_classifier" => manager.ensure_table_classifier(),
        "pp_doclayout_v3" => manager.ensure_pp_doclayout_v3_model(),
        variant => manager.ensure_slanet_model(variant),
    };
    resolved.map_err(|error| map_layout_error(kind, error))
}

/// Re-categorize a model-manager [`LayoutError`] into the closest [`XbergError`]
/// kind so a caller can branch on the failure mode.
///
/// The manager reports every failure as [`LayoutError::ModelDownload`] and
/// distinguishes the unknown-model-type case only by a message prefix. An
/// unrecognized key is genuine bad input, so it maps to
/// [`XbergError::Validation`]. Every other failure — HuggingFace download,
/// SHA-256 verification, cache-directory I/O — is an operational failure to
/// obtain a required dependency, so it maps to [`XbergError::MissingDependency`]
/// rather than overloading `Validation` (which implies a client-input error).
fn map_layout_error(kind: &str, error: LayoutError) -> XbergError {
    let message = format!("layout model '{kind}' unavailable: {error}");
    match &error {
        LayoutError::ModelDownload(detail) if detail.starts_with("Unknown model type:") => {
            XbergError::validation(message)
        }
        _ => XbergError::MissingDependency(message),
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl ModelProvider for DefaultModelProvider {
    async fn ensure_model(&self, model: &ModelId) -> Result<PathBuf> {
        let manager = self.manager.clone();
        let kind = model.kind.clone();
        tokio::task::spawn_blocking(move || ensure_by_kind(&manager, &kind))
            .await
            .map_err(|error| XbergError::Other(format!("layout model task failed to join: {error}")))?
    }
}
