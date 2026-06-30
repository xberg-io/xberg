//! The [`PresetResolver`] seam: built-in preset lookup and override merging.
//!
//! The in-core default is [`CorePresetResolver`], which looks presets up in the
//! embedded [`Registry`] and merges overrides via
//! [`presets::resolve`](crate::presets::resolve) — exactly what xberg does
//! today. Alternative resolvers (a custom registry, a preset source loaded at
//! startup) implement this trait and are injected via
//! [`EngineBuilder::with_preset_resolver`](super::super::EngineBuilder::with_preset_resolver).

use std::collections::BTreeMap;

use serde_json::Value;

use crate::presets::{Preset, Registry, ResolveError, ResolvedPreset, resolve};

/// Resolves preset identifiers to [`Preset`]s and merges caller overrides.
///
/// # Thread safety
///
/// Implementations are `Send + Sync + 'static` and held behind
/// `Arc<dyn PresetResolver>`; they may be called concurrently.
pub trait PresetResolver: Send + Sync + 'static {
    /// Look up a preset by identifier, returning an owned copy or `None`.
    fn get(&self, id: &str) -> Option<Preset>;

    /// Merge `preset` with a `custom_schema` override and `context` map.
    ///
    /// # Errors
    ///
    /// [`ResolveError::SchemaNotObject`] if `custom_schema` is not a JSON object.
    fn resolve(
        &self,
        preset: &Preset,
        custom_schema: Option<Value>,
        context: &BTreeMap<String, String>,
    ) -> Result<ResolvedPreset, ResolveError>;
}

/// In-core default: resolution over the embedded built-in preset library.
///
/// [`get`](PresetResolver::get) clones the matching preset out of the global
/// embedded [`Registry`]; [`resolve`](PresetResolver::resolve) delegates
/// straight to [`presets::resolve`](crate::presets::resolve), so the result is
/// identical to calling that free function directly.
#[derive(Debug, Default, Clone, Copy)]
pub struct CorePresetResolver;

impl PresetResolver for CorePresetResolver {
    fn get(&self, id: &str) -> Option<Preset> {
        Registry::global().get(id).cloned()
    }

    fn resolve(
        &self,
        preset: &Preset,
        custom_schema: Option<Value>,
        context: &BTreeMap<String, String>,
    ) -> Result<ResolvedPreset, ResolveError> {
        resolve(preset, custom_schema, context)
    }
}
