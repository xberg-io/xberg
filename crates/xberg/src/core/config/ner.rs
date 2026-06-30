//! NER (named-entity recognition) configuration.
//!
//! When `ExtractionConfig::ner` is `Some`, the NER post-processor runs after
//! extraction and populates [`ExtractedDocument::entities`](crate::types::ExtractedDocument::entities).

use crate::types::entity::EntityCategory;
use serde::{Deserialize, Serialize};

/// Configuration for the NER post-processor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "alef-meta", alef(since = "5.0.0"))]
#[derive(Default)]
pub struct NerConfig {
    /// Backend that runs the entity detection.
    #[serde(default)]
    pub backend: NerBackendKind,
    /// Entity categories to detect. Defaults to a sensible PERSON/ORG/LOCATION/EMAIL set
    /// when empty.
    #[serde(default)]
    pub categories: Vec<EntityCategory>,
    /// Override the default model — only used by [`NerBackendKind::Onnx`].
    /// `None` lets the backend pick its pinned default xberg GLiNER model alias.
    /// Ignored when `hf_repo` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Custom Hugging Face repository to load a GLiNER ONNX export from, bypassing
    /// the pinned `xberg-io/gliner-models` catalog — only used by [`NerBackendKind::Onnx`].
    /// Must be set together with `hf_model_file` and `hf_tokenizer_file`, or left unset.
    /// Files downloaded from a custom repo are **not** checksum-verified, unlike the
    /// pinned catalog models.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.1.0"))]
    pub hf_repo: Option<String>,
    /// Path to the ONNX model file within `hf_repo` (e.g. `"onnx/model.onnx"`).
    /// Required when `hf_repo` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.1.0"))]
    pub hf_model_file: Option<String>,
    /// Path to the tokenizer file within `hf_repo` (e.g. `"tokenizer.json"`).
    /// Required when `hf_repo` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.1.0"))]
    pub hf_tokenizer_file: Option<String>,
    /// GLiNER architecture family for `hf_repo`. Ignored when `hf_repo` is unset.
    /// Defaults to [`GlinerArchitecture::Gliner1`] when `hf_repo` is set and this is `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.2.0"))]
    pub hf_architecture: Option<GlinerArchitecture>,
    /// Optional LLM configuration — only used by [`NerBackendKind::Llm`]. Token usage
    /// for LLM backends is recorded in `ExtractedDocument::llm_usage`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm: Option<super::llm::LlmConfig>,
    /// Arbitrary user-supplied entity labels for zero-shot detection.
    ///
    /// `xberg-gliner` natively supports zero-shot inference over caller-supplied
    /// labels. The LLM backend also honours these
    /// labels by including them in the structured-output schema. Custom labels
    /// surface as [`EntityCategory::Custom`] in the resulting `Entity` stream.
    ///
    /// Use this when you need domain-specific entity types (e.g. `"Treatment"`,
    /// `"Product"`, `"Vessel"`) without forking GLiNER's taxonomy.
    #[serde(default)]
    pub custom_labels: Vec<String>,
}

/// NER backend selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum NerBackendKind {
    /// `xberg-gliner` ONNX inference. Requires `ner-onnx` feature. Models
    /// download lazily from `xberg-io/gliner-models`.
    #[default]
    Onnx,
    /// liter-llm zero-shot NER via structured-output prompts. Requires `ner-llm`
    /// feature. Useful when domain-specific categories outstrip the ONNX taxonomy.
    Llm,
}

/// GLiNER ONNX architecture family. Determines which tensor I/O contract and
/// preprocessing pipeline xberg uses — only relevant when `hf_repo` is set,
/// since the pinned `xberg-io/gliner-models` catalog is always [`GlinerArchitecture::Gliner1`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum GlinerArchitecture {
    /// Span-mode GLiNER (`gliner-community`/`urchade` lineage, the pinned
    /// `xberg-io/gliner-models` catalog, and most GLiNER fine-tunes including
    /// the `knowledgator/gliner-pii-*` family).
    #[default]
    Gliner1,
    /// Schema-prompt GLiNER2 (`fastino/gliner2` lineage). Requires an ONNX export
    /// with `[P]`/`[E]`/`[SEP_TEXT]` special tokens in its tokenizer and the
    /// `input_ids`/`attention_mask`/`text_positions`/`schema_positions`/`span_idx`
    /// tensor contract. Most GLiNER2 model cards ship safetensors only and have
    /// no ONNX export — check the repo's file list for a `.onnx` file before
    /// pointing `hf_repo` at it.
    Gliner2,
}
