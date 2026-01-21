//! Plugin management for Kreuzberg
//!
//! Handles registration and management of custom plugins including post-processors,
//! validators, and OCR backends.

pub mod post_processor;
pub mod validator;
pub mod ocr_backend;

pub use post_processor::register_post_processor;
pub use validator::register_validator;
pub use ocr_backend::{register_ocr_backend, unregister_ocr_backend, list_ocr_backends, clear_ocr_backends};

// Plugin registry functions
pub use kreuzberg::get_post_processor_registry;

use magnus::Error;
use kreuzberg::plugins::{
    unregister_validator as kz_unregister_validator,
    clear_validators as kz_clear_validators,
    list_validators as kz_list_validators,
    list_post_processors as kz_list_post_processors,
    list_extractors as kz_list_extractors,
    unregister_extractor as kz_unregister_extractor,
    clear_extractors as kz_clear_extractors,
};

/// Unregister a post-processor plugin by name
pub fn unregister_post_processor(name: String) -> Result<(), Error> {
    let registry = get_post_processor_registry();
    registry
        .write()
        .map_err(|e| crate::error_handling::runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .remove(&name)
        .map_err(crate::error_handling::kreuzberg_error)?;

    Ok(())
}

/// Unregister a validator plugin by name
pub fn unregister_validator(name: String) -> Result<(), Error> {
    kz_unregister_validator(&name)
        .map_err(crate::error_handling::kreuzberg_error)
}

/// Clear all post-processors
pub fn clear_post_processors() -> Result<(), Error> {
    let registry = get_post_processor_registry();
    registry
        .write()
        .map_err(|e| crate::error_handling::runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .shutdown_all()
        .map_err(crate::error_handling::kreuzberg_error)?;

    Ok(())
}

/// Clear all validators
pub fn clear_validators() -> Result<(), Error> {
    kz_clear_validators()
        .map_err(crate::error_handling::kreuzberg_error)
}

/// List registered post-processors
pub fn list_post_processors() -> Result<Vec<String>, Error> {
    kz_list_post_processors()
        .map_err(crate::error_handling::kreuzberg_error)
}

/// List registered validators
pub fn list_validators() -> Result<Vec<String>, Error> {
    kz_list_validators()
        .map_err(crate::error_handling::kreuzberg_error)
}

/// List registered document extractors
pub fn list_document_extractors() -> Result<Vec<String>, Error> {
    kz_list_extractors()
        .map_err(crate::error_handling::kreuzberg_error)
}

/// Unregister a document extractor
pub fn unregister_document_extractor(name: String) -> Result<(), Error> {
    kz_unregister_extractor(&name)
        .map_err(crate::error_handling::kreuzberg_error)
}

/// Clear all document extractors
pub fn clear_document_extractors() -> Result<(), Error> {
    kz_clear_extractors()
        .map_err(crate::error_handling::kreuzberg_error)
}
