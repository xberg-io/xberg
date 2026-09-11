//! Release embedding engines that a process no longer needs.

use std::num::NonZeroUsize;

use super::{CUSTOM_MODEL_FILE, get_preset};
use crate::XbergError;
use crate::core::config::EmbeddingModelType;

#[cfg(feature = "embeddings")]
use super::ENGINE_CACHE;
#[cfg(feature = "static-embeddings")]
use super::STATIC_ENGINE_CACHE;

/// Resolve the repository and model file that identify `model` in the engine caches.
fn resolve_model_identity(model: &EmbeddingModelType) -> crate::Result<(String, String)> {
    match model {
        EmbeddingModelType::Preset { name } => {
            let preset =
                get_preset(name).ok_or_else(|| XbergError::embedding(format!("Unknown embedding preset: {name}")))?;
            Ok((preset.model_repo, preset.model_file))
        }
        EmbeddingModelType::Custom { model_id, .. } => Ok((model_id.clone(), CUSTOM_MODEL_FILE.to_string())),
        EmbeddingModelType::Llm { .. } => Err(XbergError::embedding(
            "LLM embeddings keep no local model to evict; the provider serves them over HTTP.",
        )),
        EmbeddingModelType::Plugin { .. } => Err(XbergError::embedding(
            "Plugin embeddings keep no local model to evict; the registered backend owns the model lifecycle.",
        )),
    }
}

/// Drop every resident engine loaded for `model` so its memory can be freed.
///
/// Engines are matched by repository and model file, whatever pooling, sequence
/// length, acceleration or cache directory they were loaded with. A caller that
/// still holds an engine keeps it alive until it drops its handle. Returns the
/// number of engines removed; `Ok(0)` means none was resident.
///
/// # Errors
///
/// - [`crate::XbergError::Embedding`] for an unknown preset, or for an `Llm` or
///   `Plugin` model, which keep no local engine.
#[cfg_attr(alef, alef(skip))]
pub fn evict_model(model: &EmbeddingModelType) -> crate::Result<usize> {
    let (repo_name, model_file) = resolve_model_identity(model)?;
    let matches = |repo: &str, file: &str| repo == repo_name && file == model_file;
    let mut removed = 0;
    #[cfg(feature = "embeddings")]
    {
        removed += ENGINE_CACHE.evict_where(|key| matches(&key.repo_name, &key.model_file));
    }
    #[cfg(feature = "static-embeddings")]
    {
        removed += STATIC_ENGINE_CACHE.evict_where(|key| matches(&key.repo_name, &key.model_file));
    }
    Ok(removed)
}

/// Drop every resident embedding engine, ONNX and static alike. Returns the
/// number of engines removed.
#[cfg_attr(alef, alef(skip))]
pub fn clear_engine_cache() -> usize {
    let mut removed = 0;
    #[cfg(feature = "embeddings")]
    {
        removed += ENGINE_CACHE.clear();
    }
    #[cfg(feature = "static-embeddings")]
    {
        removed += STATIC_ENGINE_CACHE.clear();
    }
    removed
}

/// Bound the number of embedding engines kept resident, or lift the bound with `None`.
///
/// The bound applies to each backend's cache: at most `max_resident` ONNX engines
/// and at most `max_resident` static (model2vec) engines stay loaded. When a new
/// engine would exceed the bound, the least recently used one is dropped first.
/// Lowering the bound drops engines at once. The default is no bound.
#[cfg_attr(alef, alef(skip))]
pub fn set_engine_cache_limit(max_resident: Option<NonZeroUsize>) {
    #[cfg(feature = "embeddings")]
    ENGINE_CACHE.set_limit(max_resident);
    #[cfg(feature = "static-embeddings")]
    STATIC_ENGINE_CACHE.set_limit(max_resident);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evict_model_reports_zero_when_the_model_is_not_resident() {
        let removed = evict_model(&EmbeddingModelType::Preset {
            name: "fast".to_string(),
        })
        .expect("a known preset resolves");
        assert_eq!(removed, 0);
    }

    #[test]
    fn evict_model_rejects_an_unknown_preset() {
        let err = evict_model(&EmbeddingModelType::Preset {
            name: "no-such-preset".to_string(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("Unknown embedding preset"), "{err}");
    }

    #[test]
    fn evict_model_rejects_a_plugin_model_which_keeps_no_local_engine() {
        let err = evict_model(&EmbeddingModelType::Plugin {
            name: "custom".to_string(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("no local model to evict"), "{err}");
    }

    #[test]
    fn set_engine_cache_limit_accepts_a_bound_and_lifts_it_again() {
        set_engine_cache_limit(NonZeroUsize::new(1));
        set_engine_cache_limit(None);
        assert_eq!(clear_engine_cache(), 0);
    }
}
