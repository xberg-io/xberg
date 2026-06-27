//! Public types for the preset format.
//!
//! [`CallMode`] and [`MergeMode`] are defined in [`crate::core::config`] and
//! re-exported here so callers working exclusively with presets can import them
//! from a single module.

use serde::{Deserialize, Serialize};

pub use crate::core::config::{CallMode, MergeMode};

/// High-level category used to group presets in the registry UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum PresetCategory {
    /// Invoices, receipts, statements, purchase orders, W-9.
    Finance,
    /// Passports, drivers licenses, insurance cards.
    Identity,
    /// Contracts, NDAs, agreements.
    Legal,
    /// Bills of lading, customs declarations, packing lists.
    Logistics,
    /// Clinical records, lab reports.
    Medical,
    /// Pay stubs, resumes, employment offers.
    Hr,
    /// Catch-all for documents that don't fit the other categories.
    Other,
}

/// Pointer to a sample input + its reference output bundled with the preset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PresetSample {
    /// Path to the sample input file, relative to the preset directory.
    pub input_path: String,
    /// Path to the reference structured output, relative to the preset directory.
    pub output_path: String,
}

/// A curated structured-extraction preset loaded from the embedded library.
///
/// Each preset is a JSON file under `src/presets/library/<id>/v1.json` that
/// validates against the meta-schema in `src/presets/preset.schema.json`.
///
/// Downstream catalog consumers can inject presets via
/// [`super::registry::Registry::extend_from_dir`]. The embedded OSS library
/// ships only the `generic_document` toy preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct Preset {
    /// Stable, URL-safe preset identifier (lowercase snake_case).
    pub id: String,
    /// Monotonic version string (e.g. `v1`).
    pub version: String,
    /// Human-readable schema name forwarded to the LLM as the response/tool name.
    pub schema_name: String,
    /// One-line preset description shown in the registry UI.
    pub description: String,
    /// Top-level category for grouping in the playground.
    pub category: PresetCategory,
    /// Free-form tags used for search/filtering. May be empty.
    #[serde(default)]
    pub tags: Vec<String>,
    /// JSON Schema (Draft 2020-12) describing the structured output shape.
    pub schema: serde_json::Value,
    /// Instruction primer sent to the model.
    pub system_prompt: String,
    /// Optional mustache-style template merged with caller-supplied context.
    #[serde(default)]
    pub context_template: Option<String>,
    /// Strategy for merging per-batch outputs across paginated calls.
    pub merge_mode: MergeMode,
    /// Default call mode suggested for this preset; heuristics may override.
    pub preferred_call_mode: CallMode,
    /// When true, the prompt asks the model to wrap each field as
    /// `{value, page, bbox, confidence}` for downstream citation overlays.
    pub emit_citations: bool,
    /// Optional bundled sample (input file + reference output) for preview.
    #[serde(default)]
    pub sample: Option<PresetSample>,
    /// Stable sha256 fingerprint of the canonical preset file contents.
    ///
    /// Populated at registry load — not present in the on-disk JSON files.
    /// Used as a cache-invalidation token by the worker pipeline.
    #[serde(default, skip_deserializing)]
    pub fingerprint: String,
}

/// Lightweight projection of [`Preset`] used by the registry list endpoint
/// (omits the full schema and prompt to keep the payload small).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PresetSummary {
    /// Preset identifier matching [`Preset::id`].
    pub id: String,
    /// Preset version matching [`Preset::version`].
    pub version: String,
    /// Schema name matching [`Preset::schema_name`].
    pub schema_name: String,
    /// One-line preset description.
    pub description: String,
    /// Top-level category.
    pub category: PresetCategory,
    /// Free-form tags.
    pub tags: Vec<String>,
    /// Default call mode.
    pub preferred_call_mode: CallMode,
    /// Whether the preset prompts the model for citations.
    pub emit_citations: bool,
    /// Stable fingerprint matching [`Preset::fingerprint`].
    pub fingerprint: String,
}

impl From<&Preset> for PresetSummary {
    fn from(p: &Preset) -> Self {
        Self {
            id: p.id.clone(),
            version: p.version.clone(),
            schema_name: p.schema_name.clone(),
            description: p.description.clone(),
            category: p.category,
            tags: p.tags.clone(),
            preferred_call_mode: p.preferred_call_mode,
            emit_citations: p.emit_citations,
            fingerprint: p.fingerprint.clone(),
        }
    }
}
