//! Preset format, registry, loader, and resolver.
//!
//! Ships the preset *format* + registry + loader + resolver. The curated
//! catalog is downstream (kreuzberg-cloud) and injects additional presets via
//! [`Registry::extend_from_dir`].
//!
//! The embedded OSS library contains a single synthetic toy preset
//! (`generic_document`) that exercises the full pipeline without shipping any
//! domain-specific extraction knowledge.
//!
//! # Quick start
//!
//! ```rust
//! use kreuzberg::presets::{Registry, resolve};
//! use std::collections::BTreeMap;
//!
//! let registry = Registry::load_embedded().expect("embedded presets are valid");
//! let preset = registry.get("generic_document").expect("always present");
//! let resolved = resolve(preset, None, &BTreeMap::new()).expect("resolve succeeds");
//! assert_eq!(resolved.id, "generic_document");
//! ```

pub mod loader;
pub mod registry;
pub mod resolve;
pub mod types;

pub use loader::{LoadError, MetaSchema};
pub use registry::Registry;
pub use resolve::{ResolveError, ResolvedPreset, resolve};
pub use types::{CallMode, MergeMode, Preset, PresetCategory, PresetSample, PresetSummary};

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    // ── Embedded registry ──────────────────────────────────────────────────

    #[test]
    fn load_embedded_succeeds_and_contains_generic_document() {
        let registry = Registry::load_embedded().expect("embedded presets must load");
        assert!(
            !registry.is_empty(),
            "embedded registry must contain at least one preset"
        );
        let preset = registry
            .get("generic_document")
            .expect("generic_document preset must be present");
        assert_eq!(preset.version, "v1");
        assert_eq!(preset.category, PresetCategory::Other);
        assert!(!preset.fingerprint.is_empty(), "fingerprint must be stamped");
        assert!(
            preset.fingerprint.starts_with("sha256:"),
            "fingerprint must start with 'sha256:'"
        );
    }

    #[test]
    fn global_returns_registry_with_generic_document() {
        let registry = Registry::global();
        assert!(registry.get("generic_document").is_some());
    }

    #[test]
    fn summaries_contains_generic_document() {
        let registry = Registry::load_embedded().expect("load");
        let summaries = registry.summaries();
        assert!(summaries.iter().any(|s| s.id == "generic_document"));
    }

    // ── extend_from_dir ────────────────────────────────────────────────────

    #[test]
    fn extend_from_dir_loads_valid_preset() {
        let dir = tempfile::tempdir().expect("tempdir");
        let preset_json = serde_json::json!({
            "id": "test_preset",
            "version": "v1",
            "schema_name": "test_preset",
            "description": "A valid test preset for extend_from_dir.",
            "category": "other",
            "tags": [],
            "schema": {
                "type": "object",
                "properties": {
                    "field_one": { "type": "string" }
                }
            },
            "system_prompt": "Extract field_one from the document.",
            "merge_mode": "object_merge",
            "preferred_call_mode": "text_only",
            "emit_citations": false
        });
        let path = dir.path().join("test_preset.json");
        std::fs::write(&path, serde_json::to_vec(&preset_json).unwrap()).expect("write");

        let mut registry = Registry::load_embedded().expect("load embedded");
        let count = registry
            .extend_from_dir(dir.path())
            .expect("extend_from_dir should succeed");
        assert_eq!(count, 1, "should have loaded exactly one additional preset");
        assert!(
            registry.get("test_preset").is_some(),
            "test_preset must be retrievable after extend"
        );
    }

    #[test]
    fn extend_from_dir_rejects_malformed_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("bad.json");
        std::fs::write(&path, b"not valid json {{{").expect("write");

        let mut registry = Registry::load_embedded().expect("load embedded");
        let err = registry
            .extend_from_dir(dir.path())
            .expect_err("malformed JSON must be rejected");
        assert!(
            matches!(err, LoadError::Parse { .. }),
            "expected LoadError::Parse, got: {err}"
        );
    }

    #[test]
    fn extend_from_dir_rejects_schema_invalid_preset() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Valid JSON but missing required fields (no `schema`, no `system_prompt`).
        let invalid = serde_json::json!({
            "id": "bad_preset",
            "version": "v1",
            "schema_name": "bad_preset",
            "description": "Missing required fields.",
            "category": "other",
            "merge_mode": "object_merge",
            "preferred_call_mode": "text_only",
            "emit_citations": false
        });
        let path = dir.path().join("bad_preset.json");
        std::fs::write(&path, serde_json::to_vec(&invalid).unwrap()).expect("write");

        let mut registry = Registry::load_embedded().expect("load embedded");
        let err = registry
            .extend_from_dir(dir.path())
            .expect_err("invalid preset must be rejected");
        assert!(
            matches!(err, LoadError::SchemaValidation { .. }),
            "expected LoadError::SchemaValidation, got: {err}"
        );
    }

    // ── resolve ────────────────────────────────────────────────────────────

    #[test]
    fn resolve_generic_document_without_overrides() {
        let registry = Registry::load_embedded().expect("load");
        let preset = registry.get("generic_document").expect("present");
        let resolved = resolve(preset, None, &BTreeMap::new()).expect("resolve must succeed");
        assert_eq!(resolved.id, "generic_document");
        assert_eq!(resolved.schema, preset.schema);
        assert_eq!(resolved.system_prompt, preset.system_prompt);
    }
}
