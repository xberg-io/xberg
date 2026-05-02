//! Pipeline initialization and setup logic.
//!
//! This module handles the initialization of features and processor cache
//! required for pipeline execution.

use crate::Result;
#[cfg(feature = "quality")]
use std::sync::OnceLock;

use super::cache::{PROCESSOR_CACHE, ProcessorCache};

/// Type alias for processor stages tuple (Early, Middle, Late).
type ProcessorStages = (
    std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
    std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
    std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
);

/// Initialize feature-specific systems that may be needed during pipeline execution.
pub(super) fn initialize_features() {
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    {
        let _ = crate::keywords::ensure_initialized();
    }

    #[cfg(feature = "language-detection")]
    {
        let _ = crate::language_detection::ensure_initialized();
    }

    #[cfg(feature = "chunking")]
    {
        let _ = crate::chunking::ensure_initialized();
    }

    #[cfg(feature = "quality")]
    {
        static QUALITY_INIT: OnceLock<()> = OnceLock::new();
        QUALITY_INIT.get_or_init(|| {
            let registry = crate::plugins::registry::get_post_processor_registry();
            let mut reg = registry.write();
            let _ = reg.register(std::sync::Arc::new(crate::text::QualityProcessor));
        });
    }
}

/// Initialize the processor cache if not already initialized.
pub(super) fn initialize_processor_cache() -> Result<()> {
    let mut cache_lock = PROCESSOR_CACHE.write();
    if cache_lock.is_none() {
        *cache_lock = Some(ProcessorCache::new()?);
    }
    Ok(())
}

/// Get processors from the cache, organized by stage.
pub(super) fn get_processors_from_cache() -> Result<ProcessorStages> {
    let cache_lock = PROCESSOR_CACHE.read();
    let cache = cache_lock
        .as_ref()
        .ok_or_else(|| crate::KreuzbergError::Other("Processor cache not initialized".to_string()))?;
    Ok((
        std::sync::Arc::clone(&cache.early),
        std::sync::Arc::clone(&cache.middle),
        std::sync::Arc::clone(&cache.late),
    ))
}
