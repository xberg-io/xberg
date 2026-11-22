use serde::{Deserialize, Serialize};

pub(super) const MAX_IMAGE_BYTES: u32 = 32 * 1024 * 1024;
pub(super) const MAX_PIXELS: u32 = 16_777_216;
pub(super) const MAX_PROMPT_BYTES: u32 = 1_024;

/// One pinned artifact of a captioning model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningFile {
    /// Path relative to the owned Hugging Face repository root.
    pub path: String,
    /// Lowercase hexadecimal SHA-256 of the file.
    pub sha256: String,
    /// Exact size of the file in bytes.
    pub size_bytes: u64,
}

/// Export family the pinned graphs belong to. The two families use different preprocessing,
/// prompt construction and decoding, so an adapter never substitutes one for the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(alef, alef(skip))]
pub enum LocalCaptioningFormat {
    /// SmolVLM / Idefics3: vision encoder, embedding graph and a merged decoder reading
    /// `inputs_embeds`.
    SmolVlm,
    /// Florence-2: vision encoder, text encoder and a merged encoder-decoder. The inventory pins
    /// this family and the descriptor validates it, but the adapter refuses it explicitly because
    /// its encoder-decoder, task-token and per-task post-processing pipeline is not implemented;
    /// the pinned graphs themselves run on the bundled ONNX Runtime (see the module docs).
    Florence2,
}

/// Graph quantization selected from the canonical candidate inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(alef, alef(skip))]
pub enum LocalCaptioningPrecision {
    /// Full-precision upstream export.
    Fp32,
    /// Int8 dynamic-quantization upstream export.
    Quantized,
}

/// Complete caller-selected captioning model identity; no model preset is implied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningModel {
    /// Hugging Face repository in `owner/name` form.
    pub repository: String,
    /// Immutable forty-digit hexadecimal Git revision.
    pub revision: String,
    /// Export family that determines preprocessing, prompting and decoding.
    pub format: LocalCaptioningFormat,
    /// Repository-relative vision-encoder graph.
    pub vision_encoder_file: String,
    /// Repository-relative token-embedding graph.
    pub embed_tokens_file: String,
    /// Repository-relative text-encoder graph; Florence-2 only.
    pub text_encoder_file: Option<String>,
    /// Repository-relative merged decoder graph with a key/value cache.
    pub decoder_file: String,
    /// Every artifact preparation must resolve, including the graphs and their companions.
    pub files: Vec<LocalCaptioningFile>,
}

/// Caller-selected limits may lower, but cannot exceed, the experimental adapter's ceilings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningLimits {
    /// Maximum encoded image bytes accepted; default 33,554,432 (32 MiB).
    pub max_image_bytes: u32,
    /// Maximum decoded pixel count accepted before resizing; default 16,777,216.
    pub max_pixels: u32,
    /// Maximum generated tokens, including the stop token; default 64.
    pub max_new_tokens: u32,
    /// Maximum UTF-8 bytes in a caller prompt; default 1,024.
    pub max_prompt_bytes: u32,
}

impl Default for LocalCaptioningLimits {
    fn default() -> Self {
        Self {
            max_image_bytes: MAX_IMAGE_BYTES,
            max_pixels: MAX_PIXELS,
            max_new_tokens: 64,
            max_prompt_bytes: MAX_PROMPT_BYTES,
        }
    }
}

/// Configuration of an explicitly experimental, CPU-only local image captioner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningConfig {
    /// Caller-pinned artifacts and the export family.
    pub model: LocalCaptioningModel,
    /// Bounds checked before decoding an image or generating tokens.
    #[serde(default)]
    pub limits: LocalCaptioningLimits,
    /// Optional caption instruction for SmolVLM. `None` uses the pinned default question.
    /// Florence-2 is refused before prompting, so this field never applies to that family.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Optional cache directory used only by explicit preparation and staging.
    #[serde(default)]
    pub cache_directory: Option<String>,
}

/// Backend used by the experimental local captioner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(alef, alef(skip))]
pub enum LocalCaptioningBackend {
    /// Native ONNX Runtime with only the CPU execution provider.
    OnnxCpu,
}

/// Role of a graph in the captioning pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(alef, alef(skip))]
pub enum LocalCaptioningGraphRole {
    /// Image encoder.
    VisionEncoder,
    /// Token-embedding lookup.
    EmbedTokens,
    /// Florence-2 text encoder.
    TextEncoder,
    /// Merged autoregressive decoder.
    Decoder,
}

/// One graph actually loaded by the prepared session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningGraph {
    /// Pipeline role.
    pub role: LocalCaptioningGraphRole,
    /// Repository-relative graph path.
    pub path: String,
    /// SHA-256 of the graph bytes.
    pub sha256: String,
}

/// Identity of an explicitly prepared local model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningModelInfo {
    /// Requested backend.
    pub selected_backend: LocalCaptioningBackend,
    /// Backend accepted by runtime/provider checks.
    pub effective_backend: LocalCaptioningBackend,
    /// Export family in use.
    pub format: LocalCaptioningFormat,
    /// Requested graph quantization.
    pub precision: LocalCaptioningPrecision,
    /// Model repository.
    pub repository: String,
    /// Exact revision used during preparation.
    pub revision: String,
    /// Graphs the session actually loaded, with their pinned digests.
    pub graphs: Vec<LocalCaptioningGraph>,
    /// SHA-256 binding the complete artifact descriptor.
    pub manifest_sha256: String,
    /// Always true until a separate model qualification approves a supported preset.
    pub experimental: bool,
}

/// A generated caption and the precise local model that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningResult {
    /// Decoded caption text.
    pub caption: String,
    /// Prepared model identity and backend.
    pub model: LocalCaptioningModelInfo,
}

/// Rust-only request controls; these are not serialized or sent to a provider.
#[derive(Debug, Clone, Default)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningOptions {
    /// Relative cancellation deadline from the operation start. A blocking worker may return later.
    pub deadline_ms: Option<u64>,
    /// Cooperative cancellation, including an ORT termination request during active inference.
    pub cancel_token: Option<crate::cancellation::CancellationToken>,
}

#[cfg(feature = "captioning-onnx")]
impl LocalCaptioningModel {
    /// Descriptor for a staged evaluation candidate, selected from the canonical inventory at
    /// `precision`. The artifacts are not a supported preset and are not qualified.
    ///
    /// # Errors
    ///
    /// Fails when `id` is not an inventory candidate, names another task, or when the selected
    /// file set does not describe a complete graph family.
    pub fn staged(id: &str, precision: LocalCaptioningPrecision) -> crate::Result<Self> {
        use crate::candidate_models::{CandidateModel, CandidatePrecision, CandidateTask};
        let model = CandidateModel::find(id)
            .ok_or_else(|| super::policy::invalid(format!("{id} is not a candidate in the model inventory")))?;
        if model.task != CandidateTask::ImageCaptioning {
            return Err(super::policy::invalid(format!(
                "{id} is a {:?} candidate, not an image-captioning candidate",
                model.task
            )));
        }
        let precision = match precision {
            LocalCaptioningPrecision::Fp32 => CandidatePrecision::Fp32,
            LocalCaptioningPrecision::Quantized => CandidatePrecision::Quantized,
        };
        crate::candidate_models::captioning_model(model, precision).map_err(super::policy::invalid)
    }

    /// Identifiers of the staged image-captioning candidates. Empty when the inventory has none;
    /// the list is experimental assessment data, never a supported preset list.
    pub fn staged_candidates() -> Vec<&'static str> {
        use crate::candidate_models::{CandidateModel, CandidateTask};
        CandidateTask::ImageCaptioning
            .models()
            .into_iter()
            .map(|model: &'static CandidateModel| model.id)
            .collect()
    }
}
