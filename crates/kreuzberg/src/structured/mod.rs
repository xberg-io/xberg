//! LLM-driven structured extraction orchestrator.
//!
//! This module turns a document plus a [`PresetSpec`] (an extraction schema + prompt) into
//! validated structured JSON. It runs the regular kreuzberg extraction pipeline, decides whether
//! vision (page rasters) is needed via [`crate::heuristics::choose_call_mode`], rasterizes pages
//! lazily, packs them into token-aware batches, calls a vision-capable LLM, schema-validates and
//! merges the responses, optionally fuses OCR bounding boxes as citations, and assembles a
//! [`StructuredOutput`].
//!
//! The mechanism lives here; domain knowledge (preset catalogs, tuned thresholds, prompt bodies,
//! model selection, a distributed cache) is supplied by the caller through [`StructuredOptions`],
//! [`PresetSpec::Inline`], [`crate::presets::Registry::extend_from_dir`], and the
//! [`VisionCallCache`] trait.
//!
//! Requires the `structured` feature and is unavailable on `wasm32` (needs native HTTP and PDF
//! rendering).
//!
//! # Layout
//!
//! - [`rasterize`] — render PDF/image pages to PNG ([`PageImage`]).
//! - [`chunker`] — token-aware batch packing.
//! - [`vision_client`] — vision LLM request/response adapter over `liter-llm`.
//! - [`postprocess`] — JSON Schema validation + multi-batch merge.
//! - [`citations`] — fuse OCR bounding boxes onto extracted fields.
//! - [`prompt`] — build system/user prompts from a resolved preset.
//! - [`cache`] — the [`VisionCallCache`] trait and an in-process Moka implementation.

pub mod bindings;
pub mod cache;
pub mod chunker;
pub mod citations;
pub mod postprocess;
pub mod prompt;
pub mod rasterize;
pub mod vision_client;

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio::sync::Semaphore;

use crate::core::config::{ExtractionConfig, LlmConfig, PageConfig};
use crate::heuristics::{
    ConfidenceSignals, ConfidenceWeights, ExtractionConfidence, MultidocThresholds,
    StructuredCallMode, StructuredInput, boundaries_from_extraction_result, choose_call_mode,
    score_confidence,
};
pub use crate::heuristics::StructuredThresholds;
use crate::llm::client::create_client;
use crate::presets::{Registry, resolve};
use crate::types::LlmUsage;

use chunker::{Batch, ChunkerConfig, batch_pages};
use citations::fuse;
use postprocess::validate_and_merge;
use prompt::{BuiltPrompt, build, build_vision_fallback};
use vision_client::{VisionRequest, call};

pub use cache::{CacheKey, MokaVisionCache, VisionCallCache};

/// Default average tokens-per-image estimate for the chunker.
const DEFAULT_AVG_TOKENS_PER_IMAGE: u32 = 1_500;

/// A single rendered document page, ready to send to a vision model.
#[derive(Debug, Clone)]
pub struct PageImage {
    /// 1-indexed page number within the source document.
    pub page_number: u32,
    /// PNG-encoded page raster.
    pub png_bytes: Vec<u8>,
}

/// How the extraction schema + prompt are supplied.
///
/// JSON shape (for the bindings layer):
/// - `{"named": "invoice"}` → [`PresetSpec::Named`]
/// - `{"inline": { ...full Preset JSON... }}` → [`PresetSpec::Inline`]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetSpec {
    /// Look the preset up by id in the global [`crate::presets::Registry`].
    Named(String),
    /// Use a caller-provided preset directly (boxed; presets carry an embedded JSON schema).
    Inline(Box<crate::presets::Preset>),
}

/// Vision-call tuning. Defaults are conservative starting points; production callers override them.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct VisionConfig {
    /// Rasterization DPI for PDF pages.
    pub dpi: u32,
    /// Maximum output tokens requested per vision call.
    pub max_output_tokens: u32,
    /// Confidence below which `TextOnlyWithVisionFallback` escalates to a vision call.
    pub fallback_threshold: f32,
    /// Token ceiling for a single batched call (input side).
    pub max_input_tokens: u32,
    /// Maximum characters of extracted text included as a prompt excerpt.
    pub max_excerpt_chars: usize,
    /// Optional model override; when `None` the model comes from [`StructuredOptions::llm`].
    pub model: Option<String>,
    /// Sampling temperature.
    pub temperature: f32,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            dpi: 200,
            max_output_tokens: 8000,
            fallback_threshold: 0.6,
            max_input_tokens: 800_000,
            max_excerpt_chars: 200_000,
            model: None,
            temperature: 0.0,
        }
    }
}

/// Runtime options for a structured-extraction call.
///
/// This is a runtime call type, not a serializable config: it can carry a non-serializable
/// [`VisionCallCache`] trait object. Config-file users construct it from the serializable
/// [`crate::core::config::StructuredExtractionConfig`].
#[derive(Debug, Clone)]
pub struct StructuredOptions {
    /// LLM connection config (model, key, base URL). Reused via [`crate::llm::client`].
    pub llm: LlmConfig,
    /// Call-mode decision thresholds.
    pub thresholds: StructuredThresholds,
    /// Force a specific call mode, bypassing the heuristic.
    pub force_call_mode: Option<StructuredCallMode>,
    /// Context variables substituted into the preset's `context_template`.
    pub context: BTreeMap<String, String>,
    /// Optional vision-call cache. Excluded from the FFI/binding surface.
    pub cache: Option<Arc<dyn VisionCallCache>>,
    /// Maximum concurrent vision calls.
    pub max_parallel_calls: usize,
    /// Vision-call tuning.
    pub vision: VisionConfig,
    /// Override the preset's citation setting; `None` defers to `preset.emit_citations`.
    pub emit_citations: Option<bool>,
}

impl Default for StructuredOptions {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            thresholds: StructuredThresholds::default(),
            force_call_mode: None,
            context: BTreeMap::new(),
            cache: None,
            max_parallel_calls: 4,
            vision: VisionConfig::default(),
            emit_citations: None,
        }
    }
}

/// Where a cited field's value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationSource {
    /// Value taken from the LLM response only.
    Llm,
    /// Value taken from extracted (OCR) text only.
    Extracted,
    /// Value present in both and reconciled.
    Fused,
    /// No citation could be attached.
    None,
}

/// A single extracted field with optional provenance (page + bounding box + confidence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitedField {
    /// The field value.
    pub value: serde_json::Value,
    /// 1-indexed source page, when known.
    pub page: Option<u32>,
    /// Bounding box `[x, y, width, height]` in page pixels, when known.
    pub bbox: Option<[f64; 4]>,
    /// Citation confidence in `0.0..=1.0`, when known.
    pub confidence: Option<f64>,
    /// Provenance of the value.
    pub source: CitationSource,
}

/// The structured result in both cited (nested) and flattened (value-only) shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationEnvelope {
    /// Citation-annotated structured output (fields wrapped as [`CitedField`] when citations emit).
    pub structured_output: serde_json::Value,
    /// Flat value-only structured output.
    pub flat: serde_json::Value,
}

/// The result of a structured-extraction run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StructuredOutput {
    /// The extracted document text (excerpt used for the prompt / text-only mode).
    pub content: String,
    /// Citation-annotated structured output.
    pub structured_output: CitationEnvelope,
    /// Flattened value-only structured output (mirrors `structured_output.flat`).
    pub structured_output_flat: serde_json::Value,
    /// Confidence scoring for this run.
    pub confidence: ExtractionConfidence,
    /// Per-call LLM token usage.
    pub llm_usage: Vec<LlmUsage>,
    /// The call mode actually used.
    pub call_mode_used: StructuredCallMode,
    /// Whether a vision fallback fired after a low-confidence text-only pass.
    pub fallback_used: bool,
    /// Resolved preset id.
    pub preset_id: String,
    /// Resolved preset version.
    pub preset_version: String,
}

/// Errors returned by the structured-extraction entry points.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StructuredError {
    /// Named preset not found in the registry.
    #[error("preset not found: {0}")]
    PresetNotFound(String),
    /// Preset resolution (schema/prompt templating) failed.
    #[error("preset resolution failed: {0}")]
    Resolve(String),
    /// Underlying document extraction failed.
    #[error("document extraction failed: {0}")]
    Extraction(String),
    /// Page rasterization failed.
    #[error("page rasterization failed: {0}")]
    Rasterize(String),
    /// A vision LLM call failed.
    #[error("vision call failed: {0}")]
    Vision(String),
    /// Schema validation failed.
    #[error("schema validation failed: {0}")]
    Schema(String),
    /// Every vision batch failed; no partial result could be assembled.
    #[error("all vision batches failed: {0}")]
    AllBatchesFailed(String),
    /// The document mime type cannot be structurally extracted.
    #[error("structured extraction is not supported for mime type: {0}")]
    UnsupportedMime(String),
    /// A JSON argument could not be parsed (bindings layer).
    #[error("invalid JSON argument: {0}")]
    InvalidJson(String),
}

// ── Entry points ─────────────────────────────────────────────────────────────

/// Extract structured JSON from a document using the given preset and options.
///
/// # Errors
///
/// Returns [`StructuredError`] on preset resolution failure, extraction failure,
/// rasterization failure, or if all vision batches fail.
pub async fn extract_structured(
    bytes: &[u8],
    mime: &str,
    spec: PresetSpec,
    options: StructuredOptions,
) -> Result<StructuredOutput, StructuredError> {
    orchestrate(bytes, mime, spec, options).await
}

/// Synchronous wrapper for [`extract_structured`].
///
/// Blocks the calling thread using the global Tokio runtime. Suitable for FFI
/// and binding-facing call paths where async is unavailable.
///
/// # Errors
///
/// Returns [`StructuredError`] for the same reasons as [`extract_structured`].
pub fn extract_structured_sync(
    bytes: &[u8],
    mime: &str,
    spec: PresetSpec,
    options: StructuredOptions,
) -> Result<StructuredOutput, StructuredError> {
    global_runtime()
        .map_err(|e| StructuredError::Extraction(format!("runtime init failed: {e}")))?
        .block_on(extract_structured(bytes, mime, spec, options))
}

/// Split a multi-document PDF by detected boundaries and extract structured
/// JSON from each segment independently.
///
/// Non-PDF MIME types are passed through as a single-element result equivalent
/// to calling [`extract_structured`] directly.
///
/// # Errors
///
/// Returns [`StructuredError`] if any individual segment fails.
pub async fn split_and_extract(
    bytes: &[u8],
    mime: &str,
    spec: PresetSpec,
    options: StructuredOptions,
) -> Result<Vec<StructuredOutput>, StructuredError> {
    split_and_orchestrate(bytes, mime, spec, options).await
}

/// Synchronous wrapper for [`split_and_extract`].
///
/// # Errors
///
/// Returns [`StructuredError`] for the same reasons as [`split_and_extract`].
pub fn split_and_extract_sync(
    bytes: &[u8],
    mime: &str,
    spec: PresetSpec,
    options: StructuredOptions,
) -> Result<Vec<StructuredOutput>, StructuredError> {
    global_runtime()
        .map_err(|e| StructuredError::Extraction(format!("runtime init failed: {e}")))?
        .block_on(split_and_extract(bytes, mime, spec, options))
}

// ── Global runtime (mirrors core/extractor/sync.rs pattern) ─────────────────

static GLOBAL_RUNTIME: once_cell::sync::OnceCell<tokio::runtime::Runtime> =
    once_cell::sync::OnceCell::new();

fn global_runtime() -> crate::Result<&'static tokio::runtime::Runtime> {
    GLOBAL_RUNTIME.get_or_try_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("failed to create global Tokio runtime: {e}"),
                plugin_name: "structured_runtime".to_string(),
            })
    })
}

// ── Core orchestration ────────────────────────────────────────────────────────

async fn orchestrate(
    bytes: &[u8],
    mime: &str,
    spec: PresetSpec,
    options: StructuredOptions,
) -> Result<StructuredOutput, StructuredError> {
    // ── 1. Resolve preset ────────────────────────────────────────────────────
    let (preset, resolved) = resolve_spec(spec, &options)?;

    // ── 2. Base extraction ───────────────────────────────────────────────────
    let extraction_config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            ..PageConfig::default()
        }),
        ..ExtractionConfig::default()
    };

    let result = crate::core::extractor::extract_bytes(bytes, mime, &extraction_config)
        .await
        .map_err(|e| StructuredError::Extraction(e.to_string()))?;

    // ── 3. Derive StructuredInput ────────────────────────────────────────────
    let page_count = result.pages.as_deref().map_or(1, |p| p.len().max(1)) as u32;
    let pages_with_text = result.pages.as_deref().map_or(1usize, |pages| {
        pages.iter().filter(|p| !p.content.trim().is_empty()).count()
    });
    let text_coverage = pages_with_text as f64 / page_count as f64;
    let avg_chars_per_page = if result.content.is_empty() {
        0.0
    } else {
        result.content.len() as f64 / page_count as f64
    };
    let embedded_image_count =
        result.images.as_deref().map_or(0, |i| i.len()) as u32;
    let user_force_vision = matches!(
        options.force_call_mode,
        Some(StructuredCallMode::VisionOnly | StructuredCallMode::TextPlusVision)
    );

    let input = StructuredInput {
        mime_type: mime.to_string(),
        page_count,
        text_coverage,
        avg_chars_per_page,
        embedded_image_count,
        user_force_vision,
    };

    // ── 4. Choose call mode ──────────────────────────────────────────────────
    let call_mode =
        options.force_call_mode.unwrap_or_else(|| choose_call_mode(&input, &options.thresholds));

    // ── 5. Skip? ─────────────────────────────────────────────────────────────
    if call_mode == StructuredCallMode::Skip {
        return Err(StructuredError::UnsupportedMime(mime.to_string()));
    }

    // ── 6. Citations flag ────────────────────────────────────────────────────
    let emit_citations = options.emit_citations.unwrap_or(resolved.emit_citations);

    // ── 7. Build prompt ──────────────────────────────────────────────────────
    let prompt: BuiltPrompt = build(
        &preset,
        &options.context,
        &result.content,
        call_mode,
        options.vision.max_excerpt_chars,
    );

    // ── 8. Model ─────────────────────────────────────────────────────────────
    let model = options
        .vision
        .model
        .clone()
        .unwrap_or_else(|| options.llm.model.clone());

    // ── 9. Create client ─────────────────────────────────────────────────────
    let client = Arc::new(
        create_client(&options.llm)
            .map_err(|e| StructuredError::Vision(format!("failed to build LLM client: {e}")))?,
    );

    // ── 10. Rasterize lazily ─────────────────────────────────────────────────
    let pages = rasterize::pages_for_call(bytes, mime, call_mode, &options.vision).await?;

    // ── 11. Batch ─────────────────────────────────────────────────────────────
    let chunker_config = ChunkerConfig {
        max_input_tokens: options.vision.max_input_tokens,
        avg_tokens_per_image: DEFAULT_AVG_TOKENS_PER_IMAGE,
    };
    let batches = batch_pages(pages, prompt.user_text.clone(), &chunker_config);

    // ── 12. Run batches ──────────────────────────────────────────────────────
    let (responses, usages) = run_batches(
        batches,
        &client,
        &model,
        &prompt,
        &resolved,
        &options,
    )
    .await?;

    // ── 13. Merge ────────────────────────────────────────────────────────────
    let merged = validate_and_merge(responses, &resolved.schema, resolved.merge_mode);

    // ── 14. Confidence ───────────────────────────────────────────────────────
    let ocr_aggregate = result.ocr_elements.as_deref().and_then(compute_ocr_aggregate);
    let signals = ConfidenceSignals {
        text_coverage: text_coverage as f32,
        ocr_aggregate,
        schema_compliance: merged.schema_compliance,
    };
    let mut confidence = score_confidence(signals, ConfidenceWeights::default());
    let mut fallback_used = false;
    let mut final_merged = merged;
    let mut all_usages = usages;

    // ── 15. Vision fallback ──────────────────────────────────────────────────
    if call_mode == StructuredCallMode::TextOnlyWithVisionFallback
        && confidence.combined < options.vision.fallback_threshold
    {
        let fallback_pages =
            rasterize::pages_for_call(bytes, mime, StructuredCallMode::VisionOnly, &options.vision)
                .await?;

        if !fallback_pages.is_empty() {
            let fallback_prompt = build_vision_fallback(
                &preset,
                &options.context,
                &result.content,
                &final_merged.merged,
                &confidence,
                options.vision.max_excerpt_chars,
            );

            let fallback_batches =
                batch_pages(fallback_pages, fallback_prompt.user_text.clone(), &chunker_config);

            let (fb_responses, fb_usages) = run_batches(
                fallback_batches,
                &client,
                &model,
                &fallback_prompt,
                &resolved,
                &options,
            )
            .await?;

            let fb_merged =
                validate_and_merge(fb_responses, &resolved.schema, resolved.merge_mode);

            let fb_ocr_agg =
                result.ocr_elements.as_deref().and_then(compute_ocr_aggregate);
            let fb_signals = ConfidenceSignals {
                text_coverage: text_coverage as f32,
                ocr_aggregate: fb_ocr_agg,
                schema_compliance: fb_merged.schema_compliance,
            };

            confidence = score_confidence(fb_signals, ConfidenceWeights::default());
            all_usages.extend(fb_usages);
            final_merged = fb_merged;
            fallback_used = true;
        }
    }

    // ── 16. Citations ─────────────────────────────────────────────────────────
    let envelope = fuse(
        final_merged.merged,
        result.ocr_elements.as_deref().unwrap_or(&[]),
        emit_citations,
    );

    // ── 17. Assemble output ──────────────────────────────────────────────────
    Ok(StructuredOutput {
        content: result.content,
        structured_output: envelope.clone(),
        structured_output_flat: envelope.flat.clone(),
        confidence,
        llm_usage: all_usages,
        call_mode_used: call_mode,
        fallback_used,
        preset_id: resolved.id,
        preset_version: resolved.version,
    })
}

async fn split_and_orchestrate(
    bytes: &[u8],
    mime: &str,
    spec: PresetSpec,
    options: StructuredOptions,
) -> Result<Vec<StructuredOutput>, StructuredError> {
    let mime_lc = mime.to_ascii_lowercase();

    if mime_lc != "application/pdf" {
        // Non-PDF: treat as a single document.
        return Ok(vec![extract_structured(bytes, mime, spec, options).await?]);
    }

    // Build extraction config with page data to derive boundaries.
    let extraction_config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            ..PageConfig::default()
        }),
        ..ExtractionConfig::default()
    };

    let result = crate::core::extractor::extract_bytes(bytes, mime, &extraction_config)
        .await
        .map_err(|e| StructuredError::Extraction(e.to_string()))?;

    let boundaries =
        boundaries_from_extraction_result(&result, &MultidocThresholds::default());

    // If there is only one boundary (or no boundaries), fall back to single extraction.
    if boundaries.len() <= 1 {
        return Ok(vec![extract_structured(bytes, mime, spec, options).await?]);
    }

    // Slice the PDF into per-boundary segments concurrently.
    let semaphore = Arc::new(Semaphore::new(options.max_parallel_calls));
    let bytes_arc = Arc::new(bytes.to_vec());
    let spec_arc = Arc::new(spec);
    let options_arc = Arc::new(options);

    let mut handles = Vec::with_capacity(boundaries.len());

    for boundary in &boundaries {
        let start = boundary.start_page;
        let end = boundary.end_page;
        let bytes_clone = Arc::clone(&bytes_arc);
        let spec_clone = Arc::clone(&spec_arc);
        let options_clone = Arc::clone(&options_arc);
        let permit = Arc::clone(&semaphore);

        handles.push(tokio::spawn(async move {
            let _permit = permit
                .acquire_owned()
                .await
                .map_err(|e| StructuredError::Extraction(format!("semaphore closed: {e}")))?;

            let slice_bytes = tokio::task::spawn_blocking(move || {
                slice_pdf_pages(&bytes_clone, start, end)
            })
            .await
            .map_err(|e| StructuredError::Extraction(format!("spawn_blocking panicked: {e}")))?
            .map_err(|e| {
                StructuredError::Extraction(format!(
                    "PDF slice failed (pages {start}..{end}): {e}"
                ))
            })?;

            extract_structured(
                &slice_bytes,
                "application/pdf",
                (*spec_clone).clone(),
                (*options_clone).clone(),
            )
            .await
        }));
    }

    let mut outputs = Vec::with_capacity(handles.len());
    for handle in handles {
        let output = handle
            .await
            .map_err(|e| StructuredError::Extraction(format!("task panicked: {e}")))?;
        outputs.push(output?);
    }

    Ok(outputs)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Resolve a [`PresetSpec`] into a `(Preset, ResolvedPreset)` pair.
fn resolve_spec(
    spec: PresetSpec,
    options: &StructuredOptions,
) -> Result<(crate::presets::Preset, crate::presets::ResolvedPreset), StructuredError> {
    match spec {
        PresetSpec::Named(id) => {
            let preset = Registry::global()
                .get(&id)
                .ok_or_else(|| StructuredError::PresetNotFound(id.clone()))?
                .clone();
            let resolved = resolve(&preset, None, &options.context)
                .map_err(|e| StructuredError::Resolve(e.to_string()))?;
            Ok((preset, resolved))
        }
        PresetSpec::Inline(boxed) => {
            let preset = *boxed;
            let resolved = resolve(&preset, None, &options.context)
                .map_err(|e| StructuredError::Resolve(e.to_string()))?;
            Ok((preset, resolved))
        }
    }
}

/// Run batches concurrently up to `options.max_parallel_calls`.
///
/// Returns `(responses, usages)`. When every batch fails, returns
/// [`StructuredError::AllBatchesFailed`].
async fn run_batches(
    batches: Vec<Batch>,
    client: &Arc<liter_llm::client::DefaultClient>,
    model: &str,
    prompt: &BuiltPrompt,
    resolved: &crate::presets::ResolvedPreset,
    options: &StructuredOptions,
) -> Result<(Vec<serde_json::Value>, Vec<LlmUsage>), StructuredError> {
    let semaphore = Arc::new(Semaphore::new(options.max_parallel_calls));
    let mut handles = Vec::with_capacity(batches.len());

    for batch in batches {
        let client_clone = Arc::clone(client);
        let model_owned = model.to_string();
        let system_prompt = prompt.system.clone();
        let schema = resolved.schema.clone();
        let schema_name = resolved.schema_name.clone();
        let max_output_tokens = options.vision.max_output_tokens;
        let temperature = options.vision.temperature;
        let cache = options.cache.clone();
        let preset_fingerprint = resolved.fingerprint.clone();
        let permit = Arc::clone(&semaphore);

        handles.push(tokio::spawn(async move {
            let _permit = permit
                .acquire_owned()
                .await
                .map_err(|e| StructuredError::Vision(format!("semaphore closed: {e}")))?;

            // Build cache key.
            let content_hash = hash_batch_pages(&batch);
            let prompt_hash = hash_strings(&[&system_prompt, batch.user_text.as_deref().unwrap_or("")]);
            let page_range = batch_page_range(&batch);
            let key = CacheKey {
                content_hash,
                page_range,
                preset_fingerprint,
                prompt_hash,
                model: model_owned.clone(),
            };

            // Cache hit path.
            if let Some(cache_ref) = &cache
                && let Some(cached) = cache_ref.get(&key)
            {
                return Ok::<(serde_json::Value, LlmUsage), StructuredError>((
                    cached,
                    LlmUsage {
                        model: model_owned.clone(),
                        source: "structured_extraction_cache".to_string(),
                        input_tokens: None,
                        output_tokens: None,
                        total_tokens: None,
                        estimated_cost: None,
                        finish_reason: None,
                    },
                ));
            }

            // Cache miss — call the vision model.
            let request = VisionRequest {
                system_prompt: system_prompt.clone(),
                user_text: batch.user_text,
                images: batch.pages,
                response_schema: schema,
                response_schema_name: schema_name,
                max_output_tokens,
                temperature,
                model: model_owned.clone(),
            };

            let response = call(&client_clone, request).await?;

            if let Some(cache_ref) = &cache {
                cache_ref.put(key, response.content.clone());
            }

            Ok((response.content, response.usage))
        }));
    }

    let mut responses: Vec<serde_json::Value> = Vec::new();
    let mut usages: Vec<LlmUsage> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for handle in handles {
        match handle.await {
            Ok(Ok((val, usage))) => {
                responses.push(val);
                usages.push(usage);
            }
            Ok(Err(e)) => {
                errors.push(e.to_string());
            }
            Err(join_err) => {
                errors.push(format!("batch task panicked: {join_err}"));
            }
        }
    }

    if responses.is_empty() {
        return Err(StructuredError::AllBatchesFailed(errors.join("; ")));
    }

    Ok((responses, usages))
}

/// SHA-256 hex of all page PNG bytes in a batch, concatenated.
fn hash_batch_pages(batch: &Batch) -> String {
    let mut hasher = Sha256::new();
    for page in &batch.pages {
        hasher.update(&page.png_bytes);
    }
    digest_to_hex(hasher.finalize().as_slice())
}

/// SHA-256 hex of an ordered sequence of string slices.
fn hash_strings(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\x00"); // delimiter
    }
    digest_to_hex(hasher.finalize().as_slice())
}

/// Format a raw digest byte slice as lowercase hex.
fn digest_to_hex(digest: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

/// Extract the inclusive 1-indexed page range covered by a batch.
///
/// Returns `(1, 1)` when the batch contains no pages (text-only call).
fn batch_page_range(batch: &Batch) -> (u32, u32) {
    let numbers: Vec<u32> = batch.pages.iter().map(|p| p.page_number).collect();
    let first = numbers.first().copied().unwrap_or(1);
    let last = numbers.last().copied().unwrap_or(1);
    (first, last)
}

/// Compute mean OCR recognition confidence; returns `None` when absent.
fn compute_ocr_aggregate(elements: &[crate::types::OcrElement]) -> Option<f32> {
    if elements.is_empty() {
        return None;
    }
    let sum: f64 = elements.iter().map(|e| e.confidence.recognition).sum();
    Some((sum / elements.len() as f64) as f32)
}

/// Extract pages `start..=end` (1-indexed) from a PDF using `lopdf`.
///
/// All other pages are deleted; the mutated document is serialised back to bytes.
fn slice_pdf_pages(bytes: &[u8], start: u32, end: u32) -> Result<Vec<u8>, String> {
    let mut doc = lopdf::Document::load_mem(bytes)
        .map_err(|e| format!("lopdf load_mem failed: {e}"))?;

    let all_pages: Vec<u32> = doc.get_pages().keys().copied().collect();
    let to_delete: Vec<u32> = all_pages
        .into_iter()
        .filter(|&p| p < start || p > end)
        .collect();

    doc.delete_pages(&to_delete);

    let mut out: Vec<u8> = Vec::new();
    doc.save_to(&mut out).map_err(|e| format!("lopdf save_to failed: {e}"))?;

    Ok(out)
}
