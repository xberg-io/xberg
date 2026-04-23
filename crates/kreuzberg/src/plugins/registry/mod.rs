//! Plugin registration and discovery.
//!
//! This module provides registries for managing plugins of different types.
//! Each plugin type (OcrBackend, DocumentExtractor, etc.) has its own registry
//! with type-safe registration and lookup.

mod embedding;
mod extractor;
mod ocr;
mod processor;
mod renderer;
mod validator;

pub use embedding::EmbeddingBackendRegistry;
pub use extractor::DocumentExtractorRegistry;
pub use ocr::OcrBackendRegistry;
pub use processor::PostProcessorRegistry;
pub use renderer::RendererRegistry;
pub use validator::ValidatorRegistry;

use crate::{KreuzbergError, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::LazyLock;

/// Validate a plugin name before registration.
///
/// # Rules
///
/// - Name cannot be empty
/// - Name cannot contain whitespace
/// - Name should follow kebab-case convention (lowercase with hyphens)
///
/// # Errors
///
/// Returns `ValidationError` if the name is invalid.
pub(super) fn validate_plugin_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(KreuzbergError::Validation {
            message: "Plugin name cannot be empty".to_string(),
            source: None,
        });
    }

    if name.contains(char::is_whitespace) {
        return Err(KreuzbergError::Validation {
            message: format!("Plugin name '{}' cannot contain whitespace", name),
            source: None,
        });
    }

    Ok(())
}

/// Global OCR backend registry singleton.
pub static OCR_BACKEND_REGISTRY: LazyLock<Arc<RwLock<OcrBackendRegistry>>> =
    LazyLock::new(|| Arc::new(RwLock::new(OcrBackendRegistry::new())));

/// Global embedding backend registry singleton.
pub static EMBEDDING_BACKEND_REGISTRY: LazyLock<Arc<RwLock<EmbeddingBackendRegistry>>> =
    LazyLock::new(|| Arc::new(RwLock::new(EmbeddingBackendRegistry::new())));

/// Global document extractor registry singleton.
pub static DOCUMENT_EXTRACTOR_REGISTRY: LazyLock<Arc<RwLock<DocumentExtractorRegistry>>> =
    LazyLock::new(|| Arc::new(RwLock::new(DocumentExtractorRegistry::new())));

/// Global post-processor registry singleton.
pub static POST_PROCESSOR_REGISTRY: LazyLock<Arc<RwLock<PostProcessorRegistry>>> =
    LazyLock::new(|| Arc::new(RwLock::new(PostProcessorRegistry::new())));

/// Global validator registry singleton.
pub static VALIDATOR_REGISTRY: LazyLock<Arc<RwLock<ValidatorRegistry>>> =
    LazyLock::new(|| Arc::new(RwLock::new(ValidatorRegistry::new())));

/// Global renderer registry singleton.
pub static RENDERER_REGISTRY: LazyLock<Arc<RwLock<RendererRegistry>>> =
    LazyLock::new(|| Arc::new(RwLock::new(RendererRegistry::new())));

/// Get the global OCR backend registry.
pub fn get_ocr_backend_registry() -> Arc<RwLock<OcrBackendRegistry>> {
    OCR_BACKEND_REGISTRY.clone()
}

/// Get the global embedding backend registry.
pub fn get_embedding_backend_registry() -> Arc<RwLock<EmbeddingBackendRegistry>> {
    EMBEDDING_BACKEND_REGISTRY.clone()
}

/// Get the global document extractor registry.
pub fn get_document_extractor_registry() -> Arc<RwLock<DocumentExtractorRegistry>> {
    DOCUMENT_EXTRACTOR_REGISTRY.clone()
}

/// Get the global post-processor registry.
pub fn get_post_processor_registry() -> Arc<RwLock<PostProcessorRegistry>> {
    POST_PROCESSOR_REGISTRY.clone()
}

/// Get the global validator registry.
pub fn get_validator_registry() -> Arc<RwLock<ValidatorRegistry>> {
    VALIDATOR_REGISTRY.clone()
}

/// Get the global renderer registry.
pub fn get_renderer_registry() -> Arc<RwLock<RendererRegistry>> {
    RENDERER_REGISTRY.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_registry_access() {
        let ocr_registry = get_ocr_backend_registry();
        let _ = ocr_registry.read().list();

        let extractor_registry = get_document_extractor_registry();
        let _ = extractor_registry.read().list();

        let processor_registry = get_post_processor_registry();
        let _ = processor_registry.read().list();

        let validator_registry = get_validator_registry();
        let _ = validator_registry.read().list();

        let renderer_registry = get_renderer_registry();
        let _ = renderer_registry.read().list();
    }
}
