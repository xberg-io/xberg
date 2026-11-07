use serde::{Deserialize, Serialize};

pub(super) const MAX_INPUT_BYTES: u32 = 65_536;
pub(super) const MAX_LABEL_BYTES: u32 = 1_024;
pub(super) const MAX_LABELS: u32 = 64;
pub(super) const MAX_TOKENS: u32 = 512;

/// Complete caller-selected NLI model identity; no model preset is implied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalNliModel {
    /// Hugging Face repository in `owner/name` form.
    pub repository: String,
    /// Immutable forty-digit hexadecimal Git revision.
    pub revision: String,
    /// Repository-relative ONNX graph path.
    pub model_file: String,
    /// Repository-relative external tensor files required by the graph.
    pub external_files: Vec<String>,
    /// SHA-256 manifest covering the graph and all tokenizer/configuration assets.
    pub sha256_manifest: String,
    /// Name of the three-way NLI logits output.
    pub output_name: String,
    /// Index of entailment in the three-way logits.
    pub entailment_index: u32,
    /// Index of contradiction in the three-way logits.
    pub contradiction_index: u32,
}

/// Caller-selected limits may lower, but cannot exceed, the experimental adapter's ceilings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalClassificationLimits {
    /// Maximum UTF-8 bytes in a premise; default 65,536.
    pub max_input_bytes: u32,
    /// Maximum UTF-8 bytes in one label; default 1,024.
    pub max_label_bytes: u32,
    /// Maximum labels per request; default 64.
    pub max_labels: u32,
    /// Maximum token count including both sequences and special tokens; default 512.
    pub max_tokens: u32,
}

impl Default for LocalClassificationLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: MAX_INPUT_BYTES,
            max_label_bytes: MAX_LABEL_BYTES,
            max_labels: MAX_LABELS,
            max_tokens: MAX_TOKENS,
        }
    }
}

/// Configuration of an explicitly experimental, CPU-only NLI classifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalClassificationConfig {
    /// Caller-pinned artifacts and explicit NLI label semantics.
    pub model: LocalNliModel,
    /// Bounds checked before tokenization and inference.
    #[serde(default)]
    pub limits: LocalClassificationLimits,
    /// Template containing exactly one literal `{label}` placeholder.
    pub hypothesis_template: String,
    /// Independent label probabilities when true; otherwise choose at most one label.
    pub multi_label: bool,
    /// Minimum returned probability; this is caller-selected and is not calibrated by the SDK.
    pub confidence_threshold: f32,
    /// Optional cache directory used only by explicit preparation.
    pub cache_directory: Option<String>,
}

/// Backend used by the experimental local classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(alef, alef(skip))]
pub enum LocalClassificationBackend {
    /// Native ONNX Runtime with only the CPU execution provider.
    OnnxCpu,
}

/// Identity of an explicitly prepared local model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalClassificationModelInfo {
    /// Requested backend.
    pub selected_backend: LocalClassificationBackend,
    /// Backend accepted by runtime/provider checks.
    pub effective_backend: LocalClassificationBackend,
    /// Model repository.
    pub repository: String,
    /// Exact revision used during preparation.
    pub revision: String,
    /// Repository-relative ONNX graph path.
    pub model_file: String,
    /// SHA-256 of the selected graph.
    pub model_sha256: String,
    /// SHA-256 binding the complete artifact inventory.
    pub manifest_sha256: String,
    /// Always true until a separate model qualification approves a supported preset.
    pub experimental: bool,
}

/// Classification labels and the precise local model that produced them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalClassificationResult {
    /// Accepted labels in caller order for multi-label mode.
    pub labels: Vec<crate::types::ClassificationLabel>,
    /// Prepared model identity and backend.
    pub model: LocalClassificationModelInfo,
}

/// Rust-only request controls; these are not serialized or sent to a provider.
#[derive(Debug, Clone, Default)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalClassificationOptions {
    /// Relative cancellation deadline from the operation start. A blocking worker may return later.
    pub deadline_ms: Option<u64>,
    /// Cooperative cancellation, including an ORT termination request during active inference.
    pub cancel_token: Option<crate::cancellation::CancellationToken>,
}
