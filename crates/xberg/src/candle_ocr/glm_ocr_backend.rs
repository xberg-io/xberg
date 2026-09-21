//! GLM-OCR backend plugin for the Xberg OCR pipeline.
//!
//! This module wraps the candle-based GLM-OCR engine in the `OcrBackend` trait,
//! making it available to the extraction pipeline.
//!
//! # Engine pool design
//!
//! The pool key is `(DevicePreference, DType)` — NOT including the task.
//! All five GLM-OCR tasks (`Ocr`, `Table`, `Formula`, `Chart`, `Caption`) differ
//! only in the prompt prefix fed to the decoder. The model weights are identical,
//! so a single engine instance handles every task via `process_image_with_task`.
//! This avoids loading the ~900 MB safetensors five times in paired mode.

use async_trait::async_trait;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use parking_lot::Mutex;

use crate::Result;
use crate::candle_ocr::config::{
    GlmOcrBackendOptions, GlmOcrLayoutMode, GlmOcrTaskKind, parse_backend_options, validate_optional_non_empty,
};
use crate::core::config::OcrConfig;
use crate::engine_cache::EngineCache;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use xberg_candle_ocr::CandleOcrError;
use xberg_candle_ocr::DType;
use xberg_candle_ocr::DevicePreference;
use xberg_candle_ocr::models::GlmOcrEngine;
use xberg_candle_ocr::models::GlmOcrTask;

/// How the backend dispatches inference across a page image.
///
/// `WholePage` passes the raw page bytes to the engine as a single call — fast
/// and simple. `Paired` (compiled only when `layout-detection` is enabled) runs
/// PP-DocLayout-V3 first, crops individual regions, and dispatches each crop to
/// the task that best matches the detected layout class, merging results in
/// reading order.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Feed the whole page to the model with the backend's default task.
    WholePage,
    /// Detect layout regions first; dispatch each crop to the matching task.
    ///
    /// Only available when the `layout-detection` feature is enabled.
    #[cfg(feature = "layout-detection")]
    Paired,
}

// compiled in, WholePage otherwise. #[derive(Default)] cannot express this.
#[allow(clippy::derivable_impls)]
impl Default for LayoutMode {
    fn default() -> Self {
        #[cfg(feature = "layout-detection")]
        {
            LayoutMode::Paired
        }
        #[cfg(not(feature = "layout-detection"))]
        {
            LayoutMode::WholePage
        }
    }
}

/// Key for the GLM-OCR engine cache: `(DevicePreference, DType, cache_dir, revision)`.
type EngineKey = (DevicePreference, DType, PathBuf, String);

/// Process-wide engine cache keyed by `(DevicePreference, DType, cache_dir, revision)`.
///
/// A single engine instance handles all tasks (OCR / Table / Formula / Chart /
/// Caption) via `process_image_with_task`, so the key does not include the
/// task. Two callers requesting the same device+dtype but different tasks will
/// share one engine and avoid loading weights twice.
static ENGINE_POOL: LazyLock<EngineCache<EngineKey, GlmOcrEngine>> = LazyLock::new(EngineCache::unbounded);

/// Key for the layout model cache: `(model_path, device_preference)`.
#[cfg(feature = "layout-detection")]
type LayoutKey = (String, DevicePreference);

/// Process-wide layout model cache keyed by `(model_path, device_preference)`.
///
/// Caches loaded `PpDocLayoutV3Model` instances by their file path and device preference
/// to avoid reloading the expensive ONNX model on each `process_paired` invocation.
/// Two callers requesting the same model path with the same device will share one instance.
/// Different devices get separate model instances due to device-specific optimizations.
/// The model is wrapped in Mutex (not RwLock) since `detect` takes `&mut self`.
///
/// Only available when `layout-detection` is enabled.
#[cfg(feature = "layout-detection")]
static LAYOUT_POOL: LazyLock<
    EngineCache<LayoutKey, Mutex<crate::layout::models::pp_doclayout_v3::PpDocLayoutV3Model>>,
> = LazyLock::new(EngineCache::unbounded);

/// Process-wide cache of the resolved, verified PP-DocLayout-V3 model path.
///
/// Resolving the model runs a SHA-256 over all 131,731,131 bytes of it. Paired
/// mode resolves the model to feed [`LAYOUT_POOL`] its key, and paired mode runs
/// once per page, so the hash was charged to every page of a document: a 22-page
/// document hashed about 2.9 GB and a 731-page document about 96 GB. The path and
/// its verdict are the same for every page, so both are computed once per process
/// (GH#1718).
///
/// Keyed by the cache directory, so a process that changes where Hugging Face
/// caches models resolves again. The verification itself is unchanged — only how
/// often it runs.
///
/// Only available when `layout-detection` is enabled.
#[cfg(feature = "layout-detection")]
static LAYOUT_MODEL_PATH_POOL: LazyLock<EngineCache<PathBuf, PathBuf>> = LazyLock::new(EngineCache::unbounded);

/// Get or build a cached value, delegating to [`EngineCache::get_or_try_init`].
///
/// The cache stays locked for the whole call, so a second caller for the same
/// key waits for the first load instead of building a duplicate engine. That
/// lock covers the WHOLE cache, not only `key`, so it also serialises loads of
/// different keys while either is in flight; that is coarser than a per-key
/// single flight, and it is the convention `EngineCache` already uses for
/// embeddings, reranking, late interaction and sparse embeddings.
///
/// # Errors
/// Propagates errors from the `init` closure. A failed `init` does not insert
/// anything, so the next caller for the same key retries rather than reusing
/// a poisoned entry.
#[inline]
fn pool_get_or_init<K, V, E>(
    cache: &EngineCache<K, V>,
    key: K,
    init: impl FnOnce() -> std::result::Result<V, E>,
) -> std::result::Result<Arc<V>, E>
where
    K: std::hash::Hash + Eq + Clone,
{
    cache.get_or_try_init(key, init)
}

/// Return a cached engine for `(preference, dtype)`, initialising one on first use.
///
/// Uses the generic [`pool_get_or_init`] helper to ensure two callers with the same
/// `(preference, dtype)` receive the same Arc instance.
fn get_or_init_engine(
    preference: DevicePreference,
    dtype: DType,
    cache_dir: PathBuf,
    revision: String,
) -> crate::Result<Arc<GlmOcrEngine>> {
    let key = (preference, dtype, cache_dir.clone(), revision.clone());

    pool_get_or_init(&ENGINE_POOL, key, || {
        let device = preference.select().map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to select compute device: {e}"),
            source: Some(Box::new(e)),
        })?;

        tracing::info!(
            preference = ?preference,
            ?dtype,
            "Initialising GLM-OCR engine (cold start)"
        );
        GlmOcrEngine::new_with_hf(GlmOcrTask::default(), device, dtype, Some(&cache_dir), Some(&revision)).map_err(
            |e| crate::XbergError::Ocr {
                message: format!("GLM-OCR engine initialisation failed: {e}"),
                source: Some(Box::new(e)),
            },
        )
    })
}

/// Return a cached layout model for the given path and device, initialising one on first use.
///
/// Uses the generic [`pool_get_or_init`] helper to ensure two callers with the same
/// path and device preference receive the same Arc instance. The model is wrapped in Mutex
/// (not RwLock) since `detect` takes `&mut self` and requires exclusive access.
///
/// Only available when `layout-detection` is enabled.
#[cfg(feature = "layout-detection")]
fn get_or_init_layout_model(
    model_path: &Path,
    device: DevicePreference,
) -> crate::Result<Arc<Mutex<crate::layout::models::pp_doclayout_v3::PpDocLayoutV3Model>>> {
    use crate::layout::models::pp_doclayout_v3::PpDocLayoutV3Model;

    let model_path_str = model_path
        .to_str()
        .ok_or_else(|| crate::XbergError::Ocr {
            message: format!("Model path contains invalid UTF-8: {}", model_path.display()),
            source: None,
        })?
        .to_string();

    let key = (model_path_str.clone(), device);

    pool_get_or_init(&LAYOUT_POOL, key, || {
        tracing::info!(
            path = model_path_str.as_str(),
            ?device,
            "Initialising PP-DocLayout-V3 model (cold start)"
        );
        PpDocLayoutV3Model::from_file(&model_path_str, None)
            .map_err(|e| crate::XbergError::Ocr {
                message: format!("PP-DocLayout-V3 model initialisation failed: {e}"),
                source: Some(Box::new(e)),
            })
            .map(Mutex::new)
    })
}

/// Resolve and SHA-256 verify PP-DocLayout-V3, once per process per cache directory.
///
/// A failed resolve is not cached, so the next page retries rather than reusing a
/// poisoned entry — see [`pool_get_or_init`].
///
/// Only available when `layout-detection` is enabled.
#[cfg(feature = "layout-detection")]
fn ensure_layout_model_path() -> crate::Result<Arc<PathBuf>> {
    use crate::layout::LayoutModelManager;

    pool_get_or_init(&LAYOUT_MODEL_PATH_POOL, hf_hub::resolve_cache_dir(), || {
        tracing::info!("Resolving and verifying PP-DocLayout-V3 (cold start)");
        LayoutModelManager::new(None)
            .ensure_pp_doclayout_v3_model()
            .map_err(|e| crate::XbergError::Ocr {
                message: format!("GLM-OCR paired: layout model unavailable: {e}"),
                source: Some(Box::new(e)),
            })
    })
}

/// Options parsed from backend-specific configuration.
///
/// Extracted from [`OcrConfig.backend_options`] to make GLM-OCR configuration
/// available to both the constructor and the runtime processing paths.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
struct GlmOcrOptions {
    task: GlmOcrTask,
    device: DevicePreference,
    layout_mode: LayoutMode,
    enable_chart_understanding: bool,
    cache_dir: Option<PathBuf>,
}

/// Map a layout detection class to the GLM-OCR task best suited for that region.
///
/// Picture and Chart both go to `Caption` since the model has no standalone
/// chart-region task; caption elicits a descriptive output from the VLM.
/// However, Chart is routed to the Chart task when chart understanding is enabled;
/// otherwise it falls back to Caption for all images.
/// Header and footer are treated as plain `Ocr`.
#[cfg(feature = "layout-detection")]
fn task_for_label(label: crate::layout::LayoutClass, enable_chart_understanding: bool) -> GlmOcrTask {
    use crate::layout::LayoutClass;
    match label {
        LayoutClass::Table => GlmOcrTask::Table,
        LayoutClass::Formula => GlmOcrTask::Formula,
        LayoutClass::Chart => {
            if enable_chart_understanding {
                GlmOcrTask::Chart
            } else {
                GlmOcrTask::Caption
            }
        }
        LayoutClass::Picture => GlmOcrTask::Caption,
        LayoutClass::Text
        | LayoutClass::Title
        | LayoutClass::SectionHeader
        | LayoutClass::Caption
        | LayoutClass::ListItem
        | LayoutClass::Footnote
        | LayoutClass::PageHeader
        | LayoutClass::PageFooter
        | LayoutClass::DocumentIndex
        | LayoutClass::Code
        | LayoutClass::CheckboxSelected
        | LayoutClass::CheckboxUnselected
        | LayoutClass::Form
        | LayoutClass::KeyValueRegion => GlmOcrTask::Ocr,
    }
}

/// Wrap a raw GLM-OCR model output string according to its task.
///
/// Table output is left as-is (the model already emits GFM table syntax).
/// Formula output is wrapped in display-math `$$ … $$` fences.
/// Chart output is fenced in a JSON code block.
/// All other tasks return the text verbatim.
#[cfg(feature = "layout-detection")]
/// Strip leading and trailing `$$` delimiters and surrounding whitespace from a string.
///
/// Used to normalize formula content that may have been wrapped by the model.
/// Delegates to the shared math-delimiter stripper, which removes one pair of
/// `$$..$$`, `\[..\]`, or `$..$` delimiters and trims the result.
fn strip_formula_delimiters(content: &str) -> String {
    crate::extraction::derive::strip_math_delimiters(content).to_string()
}

fn wrap_output(task: GlmOcrTask, content: &str) -> String {
    match task {
        GlmOcrTask::Table => content.to_string(),
        GlmOcrTask::Formula => format!("$$\n{}\n$$", content.trim()),
        GlmOcrTask::Chart => format!("```json\n{}\n```", content.trim()),
        GlmOcrTask::Ocr | GlmOcrTask::Caption => content.to_string(),
    }
}

/// Whether a detected layout region is a table, i.e. its bounding box must be
/// carried through to the corresponding `Table` entry in
/// `ExtractedDocument::tables` (issue #187).
#[cfg(feature = "layout-detection")]
fn is_table_region(class: crate::layout::LayoutClass) -> bool {
    class == crate::layout::LayoutClass::Table
}

/// Attach detection-time bounding boxes to the `Table` entries `extract_gfm_tables`
/// parsed out of the assembled markdown, correlating purely by position: the Nth
/// `Table`-class detection with non-empty OCR output corresponds to the Nth GFM
/// table block that appears in `content`, since `process_paired` appends region
/// output to `content` in the same reading order it iterates detections. Extra
/// entries on either side (a table detection whose output failed to parse as GFM,
/// or vice versa) are left without a match rather than mis-attributed.
#[cfg(feature = "layout-detection")]
fn merge_table_bounding_boxes(
    tables: &mut [crate::types::Table],
    detection_bboxes: &[crate::types::extraction::BoundingBox],
) {
    for (table, bbox) in tables.iter_mut().zip(detection_bboxes.iter()) {
        table.bounding_box = Some(*bbox);
    }
}

/// Canonical markdown checkbox marker for a checkbox-class layout detection.
///
/// OCR cannot reliably recover checked-vs-unchecked state from glyph
/// recognition on a tiny checkbox crop (issue #190) — the layout detector has
/// already computed the state, so it is emitted directly instead of routing
/// the crop through the model. Returns `None` for every other layout class.
#[cfg(feature = "layout-detection")]
fn checkbox_marker_for_class(class: crate::layout::LayoutClass) -> Option<&'static str> {
    use crate::layout::LayoutClass;
    match class {
        LayoutClass::CheckboxSelected => Some("[x]"),
        LayoutClass::CheckboxUnselected => Some("[ ]"),
        _ => None,
    }
}

/// GLM-OCR backend using candle transformers.
///
/// A compact vision-language model (0.9 B) for full-page document parsing.
/// Supports text recognition, tables, formulas, charts, and image captioning
/// through a unified interface with markdown output.
///
/// # Constructor notes
///
/// `GlmOcrBackend::new(task, layout_mode)` stores the default task and layout
/// mode. `dtype` defaults to `F32` — the only dtype validated during the smoke
/// test. Use `GlmOcrBackend::with_dtype` to override.
///
/// # Configuration
///
/// GLM-OCR accepts backend options for task, device, and layout mode selection:
/// ```json
/// {
///   "task": "ocr",
///   "device": "auto",
///   "layout_mode": "whole_page",
///   "enable_chart_understanding": false,
///   "cache_dir": "/path/to/huggingface/cache"
/// }
/// ```
///
/// - `task` (string): `"ocr"` (default), `"table"`, `"formula"`, `"chart"`, `"caption"`
/// - `device` (string): `"auto"`, `"cpu"`, `"cuda"`, `"metal"`
/// - `layout_mode` (string): `"whole_page"` (default), `"paired"` (requires `layout-detection` feature)
/// - `enable_chart_understanding` (boolean): route chart regions to chart understanding
/// - `cache_dir` (string, optional): explicit Hugging Face Hub cache root
#[cfg_attr(alef, alef(skip))]
pub struct GlmOcrBackend {
    default_task: GlmOcrTask,
    layout_mode: LayoutMode,
    dtype: DType,
}

impl GlmOcrBackend {
    /// Create a new GLM-OCR backend.
    ///
    /// `dtype` defaults to `F32`. Use [`GlmOcrBackend::with_dtype`] to change it.
    pub fn new(default_task: GlmOcrTask, layout_mode: LayoutMode) -> Self {
        Self {
            default_task,
            layout_mode,
            dtype: DType::F32,
        }
    }

    /// Override the floating-point precision used by the candle engine.
    pub fn with_dtype(mut self, dtype: DType) -> Self {
        self.dtype = dtype;
        self
    }

    /// Parse backend options to extract GLM-OCR-specific configuration.
    ///
    /// Device selection is delegated to [`crate::candle_ocr::resolve_device_preference`]
    /// so the central `AccelerationConfig` is honoured.
    ///
    /// Supports the following backend options (as serde_json values):
    /// - `task` (string): `"ocr"`, `"table"`, `"formula"`, `"chart"`, `"caption"` (default: `"ocr"`)
    /// - `layout_mode` (string): `"whole_page"`, `"paired"` (default: platform-dependent)
    /// - `enable_chart_understanding` (bool): route detected charts to chart task (default: `false`)
    /// - `cache_dir` (string, optional): explicit Hugging Face Hub cache root.
    ///
    /// Unlike [`crate::candle_ocr::PaddleOcrVlBackend`] and
    /// [`crate::candle_ocr::TrocrBackend`], GLM-OCR has no `model_id` option — weights
    /// always come from the single, checksum-pinned `zai-org/GLM-OCR` repository, so an
    /// `hf_revision`/`revision` override could never resolve to different, still-valid
    /// content. Deliberately not exposed: every value other than the internal pin would
    /// fail `GlmOcrEngine::new_with_hf`'s revision check, and the pin itself is already the
    /// default. See the engine-level pinning check for the (retained) defensive guard.
    fn parse_options(&self, config: &OcrConfig) -> Result<GlmOcrOptions> {
        let options: GlmOcrBackendOptions = parse_backend_options(config.backend_options.as_ref(), "candle-glm-ocr")?;
        validate_optional_non_empty(options.cache_dir.as_deref(), "candle-glm-ocr", "cache_dir")?;
        let task = match options.task {
            None => self.default_task,
            Some(GlmOcrTaskKind::Ocr) => GlmOcrTask::Ocr,
            Some(GlmOcrTaskKind::Table) => GlmOcrTask::Table,
            Some(GlmOcrTaskKind::Formula) => GlmOcrTask::Formula,
            Some(GlmOcrTaskKind::Chart) => GlmOcrTask::Chart,
            Some(GlmOcrTaskKind::Caption) => GlmOcrTask::Caption,
        };
        let layout_mode = match options.layout_mode {
            None => self.layout_mode,
            Some(GlmOcrLayoutMode::WholePage) => LayoutMode::WholePage,
            #[cfg(feature = "layout-detection")]
            Some(GlmOcrLayoutMode::Paired) => LayoutMode::Paired,
            #[cfg(not(feature = "layout-detection"))]
            Some(GlmOcrLayoutMode::Paired) => {
                return Err(crate::XbergError::validation(
                    "invalid candle-glm-ocr backend_options.layout_mode: paired requires the layout-detection feature"
                        .to_string(),
                ));
            }
        };
        Ok(GlmOcrOptions {
            task,
            device: super::resolve_device_preference(config, options.device),
            layout_mode,
            enable_chart_understanding: options.enable_chart_understanding.unwrap_or(false),
            cache_dir: options.cache_dir.map(PathBuf::from),
        })
    }
}

impl Plugin for GlmOcrBackend {
    fn name(&self) -> &str {
        "candle-glm-ocr"
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        tracing::debug!(
            task = %self.default_task,
            "Initializing GLM-OCR backend"
        );
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Inherits the `RequiresUpright` default for `page_orientation_handling` — unmeasured, not validated (#657).
#[async_trait]
impl OcrBackend for GlmOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        let opts = self.parse_options(config)?;

        if image_bytes.is_empty() {
            return Err(crate::XbergError::Validation {
                message: "Empty image data provided to GLM-OCR".to_string(),
                source: None,
            });
        }

        let image_bytes_owned = image_bytes.to_vec();
        let dtype = self.dtype;
        let cache_dir = opts.cache_dir.unwrap_or_else(hf_hub::resolve_cache_dir);
        let revision = GlmOcrEngine::revision().to_string();

        let (content, formulas, table_bboxes) = match opts.layout_mode {
            LayoutMode::WholePage => {
                let task = opts.task;
                let device = opts.device;
                crate::extraction::image_decode::validate_standard_image_with_default_security_limits(
                    &image_bytes_owned,
                )?;
                let content = tokio::task::spawn_blocking(move || {
                    let engine = get_or_init_engine(device, dtype, cache_dir, revision)?;
                    let output = engine.process_image_with_task(&image_bytes_owned, task).map_err(|e| {
                        crate::XbergError::Ocr {
                            message: format!("GLM-OCR inference failed: {e}"),
                            source: Some(Box::new(e)),
                        }
                    })?;
                    Ok::<String, crate::XbergError>(output.content)
                })
                .await
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("GLM-OCR task execution failed: {e}"),
                    source: None,
                })??;
                (content, Vec::new(), Vec::new())
            }

            #[cfg(feature = "layout-detection")]
            LayoutMode::Paired => {
                let enable_chart_understanding = opts.enable_chart_understanding;
                process_paired(
                    image_bytes_owned,
                    opts.device,
                    dtype,
                    enable_chart_understanding,
                    cache_dir,
                    revision,
                )
                .await?
            }
        };

        // A `Paired` layout mixes per-region tasks (table/formula/chart regions
        // legitimately emit markup alongside OCR'd text regions), so the bare-LaTeX rule
        // in `filter_implausible_lines` is disabled for it; only a `WholePage` call with
        // the default OCR task is a plain-text task (GH#1676). ~keep
        let plain_text_task = matches!(opts.layout_mode, LayoutMode::WholePage) && opts.task == GlmOcrTask::Ocr;

        let mut document = super::ocr_result::build_ocr_document(
            content,
            formulas,
            image_bytes,
            config,
            super::ocr_result::OcrDocumentContext {
                mime_type: Cow::Borrowed("text/markdown"),
                backend_name: "candle-glm-ocr",
                plain_text_task,
            },
        );
        #[cfg(feature = "layout-detection")]
        merge_table_bounding_boxes(&mut document.tables, &table_bboxes);
        #[cfg(not(feature = "layout-detection"))]
        let _ = table_bboxes;
        Ok(document)
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn supported_languages(&self) -> Vec<String> {
        vec![
            "eng", "en", "zho", "zh", "jpn", "ja", "kor", "ko", "fra", "fr", "deu", "de", "spa", "es", "ita", "it",
            "por", "pt", "rus", "ru", "ara", "ar", "hin", "hi", "tha", "th", "vie", "vi",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Candle
    }

    fn emits_structured_markdown(&self) -> bool {
        true
    }

    /// GLM-OCR reports no page-level confidence.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::None
    }

    // Rotation handling has not been measured for this backend; it stays on the trait's
    // `RequiresUpright` default.
}

/// Paired-mode dispatch: run PP-DocLayout-V3, crop regions, dispatch per-region task.
///
/// Returns both the assembled markdown content and a vector of recognized formulas.
/// Each formula captures the LaTeX source (without `$$` delimiters) and its bounding box.
/// The page number must be filled in by the caller.
///
/// Only compiled when `layout-detection` feature is enabled.
#[cfg(feature = "layout-detection")]
async fn process_paired(
    image_bytes: Vec<u8>,
    device: DevicePreference,
    dtype: DType,
    enable_chart_understanding: bool,
    cache_dir: PathBuf,
    revision: String,
) -> crate::Result<(
    String,
    Vec<crate::types::Formula>,
    Vec<crate::types::extraction::BoundingBox>,
)> {
    use crate::layout::models::LayoutModel;

    const CROP_PNG_ENCODE_BYTES_PER_PIXEL: u64 = 4;
    const CROP_PNG_ENCODE_FIXED_BYTES: u64 = 256 * 1024;

    tokio::task::spawn_blocking(move || {
        let img = crate::extraction::image_decode::decode_standard_rgb8_with_default_security_limits(&image_bytes)
            .map_err(|error| crate::XbergError::Ocr {
                message: format!("GLM-OCR paired: image decode failed: {error}"),
                source: Some(Box::new(error)),
            })?;
        let security_limits = crate::extractors::security::SecurityLimits::default();
        crate::layout::engine::validate_layout_batch_peak(&[&img], &security_limits)?;

        let model_path = ensure_layout_model_path()?;

        let layout_model =
            get_or_init_layout_model(model_path.as_path(), device).map_err(|e| crate::XbergError::Ocr {
                message: format!("GLM-OCR paired: layout detection init failed: {e}"),
                source: Some(Box::new(e)),
            })?;

        let detections = layout_model.lock().detect(&img).map_err(|e| crate::XbergError::Ocr {
            message: format!("GLM-OCR paired: layout detection failed: {e}"),
            source: Some(Box::new(e)),
        })?;

        let mut sorted = detections;
        sorted.sort_by(|a, b| a.bbox.y1.total_cmp(&b.bbox.y1).then(a.bbox.x1.total_cmp(&b.bbox.x1)));

        let engine = get_or_init_engine(device, dtype, cache_dir, revision)?;

        if sorted.is_empty() {
            tracing::debug!("GLM-OCR paired: no layout regions detected, falling back to whole-page inference");
            let output = engine
                .process_image_with_task(&image_bytes, GlmOcrTask::Ocr)
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("GLM-OCR paired (fallback): whole-page inference failed: {e}"),
                    source: Some(Box::new(e)),
                })?;
            return Ok::<
                (
                    String,
                    Vec<crate::types::Formula>,
                    Vec<crate::types::extraction::BoundingBox>,
                ),
                crate::XbergError,
            >((output.content, Vec::new(), Vec::new()));
        }

        let img_width = img.width();
        let img_height = img.height();

        let mut parts: Vec<String> = Vec::with_capacity(sorted.len());
        let mut formulas: Vec<crate::types::Formula> = Vec::new();
        let mut table_bboxes: Vec<crate::types::extraction::BoundingBox> = Vec::new();

        for detection in &sorted {
            let bbox = &detection.bbox;
            let region_bbox = crate::types::extraction::BoundingBox {
                x0: bbox.x1 as f64,
                y0: bbox.y1 as f64,
                x1: bbox.x2 as f64,
                y1: bbox.y2 as f64,
            };

            // Checkbox state is decided by the layout detector, not the OCR model —
            // glyph OCR on a tiny checkbox crop cannot reliably recover the state bit
            // (#190). Emit the canonical marker directly and skip the model call.
            if let Some(marker) = checkbox_marker_for_class(detection.class_name) {
                parts.push(marker.to_string());
                continue;
            }

            let x = (bbox.x1.max(0.0) as u32).min(img_width.saturating_sub(1));
            let y = (bbox.y1.max(0.0) as u32).min(img_height.saturating_sub(1));
            let w = ((bbox.x2 - bbox.x1).max(1.0) as u32).min(img_width - x);
            let h = ((bbox.y2 - bbox.y1).max(1.0) as u32).min(img_height - y);

            let current_bytes = u64::try_from(img.as_raw().len()).map_err(|_| {
                crate::extraction::image_decode::image_dimension_error(img_width, img_height, u64::MAX, u64::MAX)
            })?;
            let crop_and_encode_bytes =
                crate::extraction::image_decode::decoded_byte_count(w, h, 3 + CROP_PNG_ENCODE_BYTES_PER_PIXEL)?
                    .checked_add(CROP_PNG_ENCODE_FIXED_BYTES)
                    .ok_or_else(|| {
                        crate::extraction::image_decode::image_dimension_error(
                            img_width,
                            img_height,
                            u64::MAX,
                            u64::MAX,
                        )
                    })?;
            crate::extraction::image_decode::validate_image_live_bytes(
                img_width,
                img_height,
                current_bytes,
                crop_and_encode_bytes,
                &security_limits,
            )?;

            let crop = image::imageops::crop_imm(&img, x, y, w, h).to_image();

            let mut crop_bytes: Vec<u8> = Vec::new();
            crop.write_to(&mut std::io::Cursor::new(&mut crop_bytes), image::ImageFormat::Png)
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("GLM-OCR paired: crop encode failed: {e}"),
                    source: Some(Box::new(e)),
                })?;

            let region_task = task_for_label(detection.class_name, enable_chart_understanding);

            // ~keep: distinguishes region-overlap duplication (two detections OCR'd over the
            // same area) from GH#1675's in-region repetition -- without per-region logging,
            // the two look identical in the assembled output.
            tracing::debug!(
                class = ?detection.class_name,
                bbox = ?bbox,
                "GLM-OCR paired: processing region"
            );

            let output = match engine.process_image_with_task(&crop_bytes, region_task) {
                Ok(out) => out,
                Err(CandleOcrError::UnsupportedConfig(ref msg)) => {
                    tracing::warn!(
                        class = ?detection.class_name,
                        bbox = ?bbox,
                        reason = %msg,
                        "GLM-OCR paired: skipping region (unsupported config)"
                    );
                    continue;
                }
                Err(e) => {
                    return Err(crate::XbergError::Ocr {
                        message: format!("GLM-OCR paired: region inference failed: {e}"),
                        source: Some(Box::new(e)),
                    });
                }
            };

            let latex_clean = if detection.class_name == crate::layout::LayoutClass::Formula {
                strip_formula_delimiters(&output.content)
            } else {
                output.content.clone()
            };

            let wrapped = wrap_output(region_task, &latex_clean);

            if detection.class_name == crate::layout::LayoutClass::Formula && !latex_clean.is_empty() {
                formulas.push(crate::types::Formula {
                    latex: latex_clean,
                    bbox: Some(region_bbox),
                    // The caller renumbers to the real document page.
                    page: Some(1),
                });
            } else if is_table_region(detection.class_name) && !output.content.trim().is_empty() {
                table_bboxes.push(region_bbox);
            }

            parts.push(wrapped);
        }

        Ok::<
            (
                String,
                Vec<crate::types::Formula>,
                Vec<crate::types::extraction::BoundingBox>,
            ),
            crate::XbergError,
        >((parts.join("\n\n"), formulas, table_bboxes))
    })
    .await
    .map_err(|e| crate::XbergError::Ocr {
        message: format!("GLM-OCR paired task execution failed: {e}"),
        source: Some(Box::new(e)),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key no other test in this process shares, so the process-wide pool cannot
    /// leak a resolution between tests.
    #[cfg(feature = "layout-detection")]
    fn unique_cache_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("xberg-gh1718-{tag}-{:?}", std::thread::current().id()))
    }

    /// GH#1718: the layout model was resolved and SHA-256 verified once per page.
    #[cfg(feature = "layout-detection")]
    #[test]
    fn the_layout_model_is_resolved_once_for_every_page_of_a_document() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cache_dir = unique_cache_dir("once");
        let resolved = PathBuf::from("/models/pp_doclayout_v3.onnx");
        let resolves = AtomicUsize::new(0);

        // 22 pages: the document GH#1718 measured at about 2.9 GB of hashing.
        for page in 0..22 {
            let path = pool_get_or_init(&LAYOUT_MODEL_PATH_POOL, cache_dir.clone(), || {
                resolves.fetch_add(1, Ordering::SeqCst);
                Ok::<PathBuf, crate::XbergError>(resolved.clone())
            })
            .expect("the model path must resolve");
            assert_eq!(*path, resolved, "page {page} must see the same model path");
        }

        assert_eq!(
            resolves.load(Ordering::SeqCst),
            1,
            "the model must be resolved and verified once, not once per page"
        );
    }

    /// A download or verification failure must not poison the pool for the rest of
    /// the process; the next page retries.
    #[cfg(feature = "layout-detection")]
    #[test]
    fn a_failed_layout_model_resolution_is_not_cached() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cache_dir = unique_cache_dir("retry");
        let resolved = PathBuf::from("/models/pp_doclayout_v3.onnx");
        let resolves = AtomicUsize::new(0);

        let first = pool_get_or_init(&LAYOUT_MODEL_PATH_POOL, cache_dir.clone(), || {
            resolves.fetch_add(1, Ordering::SeqCst);
            Err::<PathBuf, crate::XbergError>(crate::XbergError::Ocr {
                message: "layout model unavailable".to_string(),
                source: None,
            })
        });
        assert!(first.is_err(), "the first resolve must report the failure");

        let second = pool_get_or_init(&LAYOUT_MODEL_PATH_POOL, cache_dir, || {
            resolves.fetch_add(1, Ordering::SeqCst);
            Ok::<PathBuf, crate::XbergError>(resolved.clone())
        })
        .expect("the retry must resolve");

        assert_eq!(*second, resolved);
        assert_eq!(resolves.load(Ordering::SeqCst), 2, "a failed resolve must be retried");
    }

    #[test]
    fn test_glm_ocr_backend_creation() {
        let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
        assert_eq!(backend.name(), "candle-glm-ocr");
        assert_eq!(backend.backend_type(), OcrBackendType::Candle);
    }

    #[test]
    fn test_glm_ocr_emits_structured_markdown() {
        let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
        assert!(backend.emits_structured_markdown());
    }

    #[test]
    fn test_glm_ocr_language_support() {
        let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("zho"));
        assert!(backend.supports_language("jpn"));
        assert!(backend.supports_language("unknown"));
    }

    #[test]
    fn test_glm_ocr_supported_languages() {
        let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
        let langs = backend.supported_languages();
        assert!(langs.contains(&"eng".to_string()));
        assert!(langs.contains(&"zho".to_string()));
        assert!(langs.contains(&"fra".to_string()));
    }

    #[tokio::test]
    async fn should_reject_oversized_declared_dimensions_before_loading_glm_model() {
        let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::WholePage);
        let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(6000, 6000);

        let error = backend
            .process_image(&bytes, &OcrConfig::default())
            .await
            .expect_err("GLM-OCR must validate the decoded-byte budget before model initialization");

        assert!(matches!(error, crate::XbergError::Validation { .. }));
        assert!(error.to_string().contains("6000x6000"));
        assert!(error.to_string().contains("security_limits.max_content_size"));
    }

    #[test]
    fn test_parse_options_defaults() {
        let config = OcrConfig::default();
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert_eq!(opts.task, GlmOcrTask::Ocr);
        assert_eq!(opts.device, DevicePreference::Auto);
        assert!(!opts.enable_chart_understanding);
    }

    #[test]
    fn test_parse_options_custom_task() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"task": "table"})),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert_eq!(opts.task, GlmOcrTask::Table);
    }

    #[test]
    fn test_parse_options_formula_task() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"task": "formula"})),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert_eq!(opts.task, GlmOcrTask::Formula);
    }

    #[test]
    fn test_parse_options_custom_device() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"device": "cpu"})),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert_eq!(opts.device, DevicePreference::Cpu);
    }

    #[test]
    fn test_parse_options_enable_chart_understanding_true() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"enable_chart_understanding": true})),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert!(opts.enable_chart_understanding);
    }

    #[test]
    fn test_parse_options_enable_chart_understanding_false() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"enable_chart_understanding": false})),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert!(!opts.enable_chart_understanding);
    }

    #[test]
    fn test_parse_options_chart_understanding_default() {
        let config = OcrConfig::default();
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert!(!opts.enable_chart_understanding);
    }

    #[test]
    fn test_parse_options_combined() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "task": "chart",
                "device": "cuda",
                "enable_chart_understanding": true
            })),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert_eq!(opts.task, GlmOcrTask::Chart);
        assert_eq!(opts.device, DevicePreference::Cuda);
        assert!(opts.enable_chart_understanding);
    }

    #[test]
    fn test_parse_options_non_object_json_returns_contextual_errors() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!([1, 2, 3])),
            ..Default::default()
        };
        let error = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap_err()
            .to_string();
        assert!(error.contains("candle-glm-ocr backend_options"));

        let config = OcrConfig {
            backend_options: Some(serde_json::json!("ocr")),
            ..Default::default()
        };
        let error = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap_err()
            .to_string();
        assert!(error.contains("candle-glm-ocr backend_options"));
    }

    #[test]
    fn test_parse_options_empty_object_returns_defaults() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({})),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert_eq!(opts.task, GlmOcrTask::Ocr);
        assert_eq!(opts.device, DevicePreference::Auto);
        assert!(!opts.enable_chart_understanding);
        assert!(opts.cache_dir.is_none());
    }

    #[test]
    fn test_parse_options_layout_mode_and_cache_dir() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "layout_mode": "paired",
                "cache_dir": "/tmp/glm-cache"
            })),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        assert_eq!(opts.layout_mode, LayoutMode::Paired);
        assert_eq!(opts.cache_dir.as_deref(), Some(Path::new("/tmp/glm-cache")));
    }

    #[test]
    fn test_initialize_and_shutdown() {
        let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_task_for_label_table() {
        use crate::layout::LayoutClass;
        assert_eq!(task_for_label(LayoutClass::Table, false), GlmOcrTask::Table);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_task_for_label_formula() {
        use crate::layout::LayoutClass;
        assert_eq!(task_for_label(LayoutClass::Formula, false), GlmOcrTask::Formula);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_task_for_label_text() {
        use crate::layout::LayoutClass;
        assert_eq!(task_for_label(LayoutClass::Text, false), GlmOcrTask::Ocr);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_task_for_label_chart_disabled() {
        use crate::layout::LayoutClass;
        assert_eq!(task_for_label(LayoutClass::Chart, false), GlmOcrTask::Caption);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_task_for_label_chart_enabled() {
        use crate::layout::LayoutClass;
        assert_eq!(task_for_label(LayoutClass::Chart, true), GlmOcrTask::Chart);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_parse_and_route_chart_with_understanding_enabled() {
        use crate::layout::LayoutClass;
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"enable_chart_understanding": true})),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        let routed_task = task_for_label(LayoutClass::Chart, opts.enable_chart_understanding);
        assert_eq!(routed_task, GlmOcrTask::Chart);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_parse_and_route_chart_with_understanding_disabled() {
        use crate::layout::LayoutClass;
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"enable_chart_understanding": false})),
            ..Default::default()
        };
        let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
            .parse_options(&config)
            .unwrap();
        let routed_task = task_for_label(LayoutClass::Chart, opts.enable_chart_understanding);
        assert_eq!(routed_task, GlmOcrTask::Caption);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_wrap_output_formula() {
        let wrapped = wrap_output(GlmOcrTask::Formula, "x^2 + y^2 = r^2");
        assert!(wrapped.starts_with("$$\n"));
        assert!(wrapped.ends_with("\n$$"));
        assert!(wrapped.contains("x^2 + y^2 = r^2"));
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_strip_formula_delimiters_removes_wrapping_dollars() {
        let wrapped = "$$\nE = mc^2\n$$";
        let result = strip_formula_delimiters(wrapped);
        assert_eq!(result, "E = mc^2");
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_strip_formula_delimiters_handles_pre_wrapped_content() {
        let pre_wrapped = "$$x^2 + y^2 = z^2$$";
        let result = strip_formula_delimiters(pre_wrapped);
        assert_eq!(result, "x^2 + y^2 = z^2");
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_strip_formula_delimiters_preserves_undecorated_content() {
        let plain = "a + b = c";
        let result = strip_formula_delimiters(plain);
        assert_eq!(result, "a + b = c");
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_formula_extraction_from_wrapped_output() {
        let task = GlmOcrTask::Formula;
        let raw_latex = "E = mc^2";
        let wrapped = wrap_output(task, raw_latex);
        let stripped = strip_formula_delimiters(&wrapped);
        assert_eq!(stripped, raw_latex);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_wrap_output_chart() {
        let wrapped = wrap_output(GlmOcrTask::Chart, r#"{"type":"bar"}"#);
        assert!(wrapped.starts_with("```json\n"));
        assert!(wrapped.ends_with("\n```"));
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_wrap_output_table_passthrough() {
        let table = "| A | B |\n|---|---|\n| 1 | 2 |";
        let wrapped = wrap_output(GlmOcrTask::Table, table);
        assert_eq!(wrapped, table);
    }

    #[test]
    fn test_pool_get_or_init_caches_on_first_miss() {
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let pool: EngineCache<&str, u32> = EngineCache::unbounded();
        let init_count = StdArc::new(AtomicUsize::new(0));

        let init_count_clone = StdArc::clone(&init_count);
        let result1 = pool_get_or_init(&pool, "test_key", || {
            init_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<u32, String>(42)
        });

        assert!(result1.is_ok());
        assert_eq!(init_count.load(Ordering::SeqCst), 1, "Initializer should run once");

        let init_count_clone = StdArc::clone(&init_count);
        let result2 = pool_get_or_init(&pool, "test_key", || {
            init_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<u32, String>(99)
        });

        assert!(result2.is_ok());
        assert_eq!(
            init_count.load(Ordering::SeqCst),
            1,
            "Initializer should still have run exactly once"
        );

        let v1 = result1.unwrap();
        let v2 = result2.unwrap();
        assert!(Arc::ptr_eq(&v1, &v2), "Cached values should be the same Arc instance");
        assert_eq!(*v1, 42, "First initializer's value should be stored");
    }

    #[test]
    fn test_pool_get_or_init_concurrent_access() {
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::thread;

        let pool: StdArc<EngineCache<&str, u32>> = StdArc::new(EngineCache::unbounded());
        let init_count = StdArc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..5 {
            let pool_clone = StdArc::clone(&pool);
            let init_count_clone = StdArc::clone(&init_count);

            let handle = thread::spawn(move || {
                let result = pool_get_or_init(&pool_clone, "concurrent_key", || {
                    init_count_clone.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    Ok::<u32, String>(42)
                });
                result.unwrap()
            });
            handles.push(handle);
        }

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        for i in 1..results.len() {
            assert!(
                Arc::ptr_eq(&results[0], &results[i]),
                "All concurrent callers should receive the same Arc instance"
            );
        }

        let final_count = init_count.load(Ordering::SeqCst);
        assert_eq!(
            final_count, 1,
            "Initializer must run exactly once, not once per racing caller"
        );
    }

    #[test]
    fn test_pool_get_or_init_failed_load_does_not_poison() {
        let pool: EngineCache<&str, u32> = EngineCache::unbounded();

        let failed = pool_get_or_init(&pool, "key", || Err::<u32, String>("load failed".to_string()));
        assert!(failed.is_err(), "a failing init must not be papered over");

        let recovered = pool_get_or_init(&pool, "key", || Ok::<u32, String>(7));
        assert_eq!(
            *recovered.expect("a later caller retries after a failed load"),
            7,
            "the retry must build a fresh value rather than reuse a poisoned entry"
        );
    }

    #[test]
    fn test_glm_ocr_zero_regions_fallback_guard() {
        assert_eq!(GlmOcrTask::Ocr, GlmOcrTask::default());
    }

    // --- issue #187: paired-mode table bounding boxes must survive re-parsing ---

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_is_table_region_true_for_table_class() {
        use crate::layout::LayoutClass;
        assert!(is_table_region(LayoutClass::Table));
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_is_table_region_false_for_text_class() {
        use crate::layout::LayoutClass;
        assert!(!is_table_region(LayoutClass::Text));
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_merge_table_bounding_boxes_attaches_bbox_matching_detection_order() {
        use crate::types::Table;
        use crate::types::extraction::BoundingBox;

        // Simulates two Table-class GFM tables parsed out of `content` by
        // `extract_gfm_tables`, in the same order the source detections appeared.
        let mut tables = vec![
            Table {
                cells: vec![vec!["A".to_string()]],
                markdown: "| A |\n|---|".to_string(),
                page_number: 1,
                ..Default::default()
            },
            Table {
                cells: vec![vec!["B".to_string()]],
                markdown: "| B |\n|---|".to_string(),
                page_number: 1,
                ..Default::default()
            },
        ];

        let bboxes = vec![
            BoundingBox {
                x0: 10.0,
                y0: 20.0,
                x1: 100.0,
                y1: 200.0,
            },
            BoundingBox {
                x0: 5.0,
                y0: 6.0,
                x1: 7.0,
                y1: 8.0,
            },
        ];

        merge_table_bounding_boxes(&mut tables, &bboxes);

        assert_eq!(tables.len(), 2, "table count must be unchanged by the merge");
        assert_eq!(
            tables[0].bounding_box,
            Some(BoundingBox {
                x0: 10.0,
                y0: 20.0,
                x1: 100.0,
                y1: 200.0
            })
        );
        assert_eq!(
            tables[1].bounding_box,
            Some(BoundingBox {
                x0: 5.0,
                y0: 6.0,
                x1: 7.0,
                y1: 8.0
            })
        );
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_process_paired_table_detection_yields_exactly_one_table_with_source_bbox() {
        use crate::core::config::OcrConfig;
        use crate::layout::LayoutClass;
        use crate::types::extraction::BoundingBox;

        // Synthetic fixture standing in for `sorted` detections in `process_paired`:
        // one Table-class region (should become a structured Table entry with its
        // detection bbox attached) and one Text-class region (must NOT show up in
        // `result.tables`). Mirrors the per-detection loop body without needing the
        // GLM model or PP-DocLayout-V3.
        let table_bbox = BoundingBox {
            x0: 12.0,
            y0: 34.0,
            x1: 512.0,
            y1: 734.0,
        };
        let table_output = "| Name | Age |\n|------|-----|\n| Alice | 30 |";
        let text_output = "Just some plain OCR'd prose.";

        let regions: Vec<(LayoutClass, BoundingBox, &str)> = vec![
            (LayoutClass::Table, table_bbox, table_output),
            (LayoutClass::Text, BoundingBox::default(), text_output),
        ];

        let mut parts: Vec<String> = Vec::with_capacity(regions.len());
        let mut table_bboxes: Vec<BoundingBox> = Vec::new();
        for (class, bbox, output) in &regions {
            if is_table_region(*class) && !output.trim().is_empty() {
                table_bboxes.push(*bbox);
            }
            parts.push(output.to_string());
        }
        let content = parts.join("\n\n");

        let config = OcrConfig::default();
        let mut doc = super::super::ocr_result::build_ocr_document(
            content,
            Vec::new(),
            &[],
            &config,
            super::super::ocr_result::OcrDocumentContext {
                mime_type: std::borrow::Cow::Borrowed("text/markdown"),
                backend_name: "candle-glm-ocr",
                plain_text_task: true,
            },
        );
        merge_table_bounding_boxes(&mut doc.tables, &table_bboxes);

        assert_eq!(
            doc.tables.len(),
            1,
            "the Text-class region must not produce a table entry"
        );
        assert_eq!(doc.tables[0].bounding_box, Some(table_bbox));
    }

    // --- issue #190: checkbox selected/unselected state must not be lost to OCR ---

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_checkbox_marker_for_class_selected_is_x_marker() {
        use crate::layout::LayoutClass;
        assert_eq!(checkbox_marker_for_class(LayoutClass::CheckboxSelected), Some("[x]"));
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_checkbox_marker_for_class_unselected_is_empty_marker() {
        use crate::layout::LayoutClass;
        assert_eq!(checkbox_marker_for_class(LayoutClass::CheckboxUnselected), Some("[ ]"));
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_checkbox_marker_for_class_none_for_non_checkbox_class() {
        use crate::layout::LayoutClass;
        assert_eq!(checkbox_marker_for_class(LayoutClass::Text), None);
        assert_eq!(checkbox_marker_for_class(LayoutClass::Table), None);
    }
}
