//! GLM-OCR backend plugin for the Kreuzberg OCR pipeline.
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
use std::path::Path;
use std::sync::{Arc, LazyLock};

use ahash::AHashMap;
use parking_lot::RwLock;

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractionResult;
use kreuzberg_candle_ocr::DType;
use kreuzberg_candle_ocr::DevicePreference;
use kreuzberg_candle_ocr::models::GlmOcrEngine;
use kreuzberg_candle_ocr::models::GlmOcrTask;

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

impl Default for LayoutMode {
    fn default() -> Self {
        // When layout-detection is compiled in, default to Paired so per-region
        // task dispatch is used without explicit configuration. Falls back to
        // WholePage when the feature is absent.
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

/// Process-wide engine pool keyed by `(DevicePreference, DType)`.
///
/// A single engine instance handles all tasks (OCR / Table / Formula / Chart /
/// Caption) via `process_image_with_task`, so the pool key does not include the
/// task. Two callers requesting the same device+dtype but different tasks will
/// share one engine and avoid loading weights twice.
static ENGINE_POOL: LazyLock<RwLock<AHashMap<(DevicePreference, DType), Arc<GlmOcrEngine>>>> =
    LazyLock::new(|| RwLock::new(AHashMap::new()));

/// Return a cached engine for `(preference, dtype)`, initialising one on first use.
///
/// Uses a read → miss → write → double-check pattern so two racing callers do
/// not both pay the initialisation cost.
fn get_or_init_engine(preference: DevicePreference, dtype: DType) -> crate::Result<Arc<GlmOcrEngine>> {
    let key = (preference, dtype);

    // Fast path: engine already in pool.
    {
        let pool = ENGINE_POOL.read();
        if let Some(engine) = pool.get(&key) {
            return Ok(Arc::clone(engine));
        }
    }

    // Slow path: select the device and build the engine, then insert under write lock.
    let device = preference.select().map_err(|e| crate::KreuzbergError::Ocr {
        message: format!("Failed to select compute device: {e}"),
        source: Some(Box::new(e)),
    })?;

    tracing::info!(
        preference = ?preference,
        ?dtype,
        "Initialising GLM-OCR engine (cold start)"
    );
    // Default task passed here is irrelevant to weight loading; the backend
    // always calls `process_image_with_task` with the per-call task.
    let new_engine =
        GlmOcrEngine::new(GlmOcrTask::default(), device, dtype).map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("GLM-OCR engine initialisation failed: {e}"),
            source: Some(Box::new(e)),
        })?;
    let new_engine = Arc::new(new_engine);

    let mut pool = ENGINE_POOL.write();
    // Double-check: another thread may have inserted while we were building.
    if let Some(existing) = pool.get(&key) {
        return Ok(Arc::clone(existing));
    }
    pool.insert(key, Arc::clone(&new_engine));
    Ok(new_engine)
}

/// Map a layout detection class to the GLM-OCR task best suited for that region.
///
/// Chart and Picture both go to `Caption` since the model has no standalone
/// chart-region task; caption elicits a descriptive output from the VLM.
/// Header and footer are treated as plain `Ocr`.
#[cfg(feature = "layout-detection")]
fn task_for_label(label: crate::layout::LayoutClass) -> GlmOcrTask {
    use crate::layout::LayoutClass;
    match label {
        LayoutClass::Table => GlmOcrTask::Table,
        LayoutClass::Formula => GlmOcrTask::Formula,
        LayoutClass::Picture => GlmOcrTask::Caption,
        // Text-like regions
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
fn wrap_output(task: GlmOcrTask, content: &str) -> String {
    match task {
        GlmOcrTask::Table => content.to_string(),
        GlmOcrTask::Formula => format!("$$\n{}\n$$", content.trim()),
        GlmOcrTask::Chart => format!("```json\n{}\n```", content.trim()),
        GlmOcrTask::Ocr | GlmOcrTask::Caption => content.to_string(),
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
///   "layout_mode": "whole_page"
/// }
/// ```
///
/// - `task` (string): `"ocr"` (default), `"table"`, `"formula"`, `"chart"`, `"caption"`
/// - `device` (string): `"auto"`, `"cpu"`, `"cuda"`, `"metal"`
/// - `layout_mode` (string): `"whole_page"` (default), `"paired"` (requires `layout-detection` feature)
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
    fn parse_options(config: &OcrConfig) -> (GlmOcrTask, DevicePreference, LayoutMode) {
        let mut task = GlmOcrTask::default();
        let mut layout_mode = LayoutMode::default();

        if let Some(opts) = &config.backend_options {
            if let Some(t) = opts.get("task").and_then(|v| v.as_str()) {
                task = match t {
                    "table" => GlmOcrTask::Table,
                    "formula" => GlmOcrTask::Formula,
                    "chart" => GlmOcrTask::Chart,
                    "caption" => GlmOcrTask::Caption,
                    _ => GlmOcrTask::Ocr, // default on unknown
                };
            }

            if let Some(m) = opts.get("layout_mode").and_then(|v| v.as_str()) {
                layout_mode = match m {
                    #[cfg(feature = "layout-detection")]
                    "paired" => LayoutMode::Paired,
                    _ => LayoutMode::WholePage,
                };
            }
        }

        let device = super::resolve_device_preference(config);
        (task, device, layout_mode)
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

#[async_trait]
impl OcrBackend for GlmOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        // Parse configuration
        let (task, device, layout_mode) = Self::parse_options(config);

        // Validate image data
        if image_bytes.is_empty() {
            return Err(crate::KreuzbergError::Validation {
                message: "Empty image data provided to GLM-OCR".to_string(),
                source: None,
            });
        }

        let image_bytes = image_bytes.to_vec();
        let dtype = self.dtype;

        let content = match layout_mode {
            LayoutMode::WholePage => {
                // Run whole-page inference in a blocking task.
                tokio::task::spawn_blocking(move || {
                    let engine = get_or_init_engine(device, dtype)?;
                    let output =
                        engine
                            .process_image_with_task(&image_bytes, task)
                            .map_err(|e| crate::KreuzbergError::Ocr {
                                message: format!("GLM-OCR inference failed: {e}"),
                                source: Some(Box::new(e)),
                            })?;
                    Ok::<String, crate::KreuzbergError>(output.content)
                })
                .await
                .map_err(|e| crate::KreuzbergError::Ocr {
                    message: format!("GLM-OCR task execution failed: {e}"),
                    source: None,
                })??
            }

            #[cfg(feature = "layout-detection")]
            LayoutMode::Paired => process_paired(image_bytes, device, dtype).await?,
        };

        Ok(ExtractionResult {
            content,
            mime_type: Cow::Borrowed("text/markdown"),
            ..Default::default()
        })
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        // GLM-OCR is trained on multilingual data and supports a broad range of
        // scripts. Accept all language codes.
        true
    }

    fn supported_languages(&self) -> Vec<String> {
        // Major language codes supported by GLM-OCR
        vec![
            "eng", "en", // English
            "zho", "zh", // Chinese (simplified and traditional)
            "jpn", "ja", // Japanese
            "kor", "ko", // Korean
            "fra", "fr", // French
            "deu", "de", // German
            "spa", "es", // Spanish
            "ita", "it", // Italian
            "por", "pt", // Portuguese
            "rus", "ru", // Russian
            "ara", "ar", // Arabic
            "hin", "hi", // Hindi
            "tha", "th", // Thai
            "vie", "vi", // Vietnamese
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Candle
    }

    fn emits_structured_markdown(&self) -> bool {
        // GLM-OCR emits markdown output directly from the VLM,
        // so the extraction pipeline should skip layout reconstruction stages.
        true
    }
}

/// Paired-mode dispatch: run PP-DocLayout-V3, crop regions, dispatch per-region task.
///
/// Only compiled when `layout-detection` feature is enabled.
#[cfg(feature = "layout-detection")]
async fn process_paired(image_bytes: Vec<u8>, device: DevicePreference, dtype: DType) -> crate::Result<String> {
    use crate::layout::LayoutModelManager;
    use crate::layout::models::LayoutModel;
    use crate::layout::models::pp_doclayout_v3::PpDocLayoutV3Model;

    tokio::task::spawn_blocking(move || {
        // Decode image once; all region crops reuse the same decoded pixels.
        let img = image::load_from_memory(&image_bytes)
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("GLM-OCR paired: image decode failed: {e}"),
                source: Some(Box::new(e)),
            })?
            .to_rgb8();

        // Resolve the PP-DocLayout-V3 model path via the layout model manager.
        let manager = LayoutModelManager::new(None);
        let model_path = manager
            .ensure_pp_doclayout_v3_model()
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("GLM-OCR paired: layout model unavailable: {e}"),
                source: Some(Box::new(e)),
            })?;

        let mut layout_model = PpDocLayoutV3Model::from_file(&model_path.to_string_lossy(), None).map_err(|e| {
            crate::KreuzbergError::Ocr {
                message: format!("GLM-OCR paired: layout detection init failed: {e}"),
                source: Some(Box::new(e)),
            }
        })?;

        let detections = layout_model.detect(&img).map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("GLM-OCR paired: layout detection failed: {e}"),
            source: Some(Box::new(e)),
        })?;

        // Sort detections in reading order (top-to-bottom, left-to-right).
        let mut sorted = detections;
        sorted.sort_by(|a, b| a.bbox.y1.total_cmp(&b.bbox.y1).then(a.bbox.x1.total_cmp(&b.bbox.x1)));

        let engine = get_or_init_engine(device, dtype)?;
        let img_width = img.width();
        let img_height = img.height();

        let mut parts: Vec<String> = Vec::with_capacity(sorted.len());

        for detection in &sorted {
            let bbox = &detection.bbox;

            // Clamp to image bounds (model coordinates are in pixel space).
            let x = (bbox.x1.max(0.0) as u32).min(img_width.saturating_sub(1));
            let y = (bbox.y1.max(0.0) as u32).min(img_height.saturating_sub(1));
            let w = ((bbox.x2 - bbox.x1).max(1.0) as u32).min(img_width - x);
            let h = ((bbox.y2 - bbox.y1).max(1.0) as u32).min(img_height - y);

            // Crop the region using image::imageops.
            let crop = image::imageops::crop_imm(&img, x, y, w, h).to_image();

            // Encode crop as PNG bytes.
            let mut crop_bytes: Vec<u8> = Vec::new();
            crop.write_to(&mut std::io::Cursor::new(&mut crop_bytes), image::ImageFormat::Png)
                .map_err(|e| crate::KreuzbergError::Ocr {
                    message: format!("GLM-OCR paired: crop encode failed: {e}"),
                    source: Some(Box::new(e)),
                })?;

            let region_task = task_for_label(detection.class_name);

            let output =
                engine
                    .process_image_with_task(&crop_bytes, region_task)
                    .map_err(|e| crate::KreuzbergError::Ocr {
                        message: format!("GLM-OCR paired: region inference failed: {e}"),
                        source: Some(Box::new(e)),
                    })?;

            parts.push(wrap_output(region_task, &output.content));
        }

        Ok::<String, crate::KreuzbergError>(parts.join("\n\n"))
    })
    .await
    .map_err(|e| crate::KreuzbergError::Ocr {
        message: format!("GLM-OCR paired task execution failed: {e}"),
        source: Some(Box::new(e)),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_parse_options_defaults() {
        let config = OcrConfig::default();
        let (task, device, _layout_mode) = GlmOcrBackend::parse_options(&config);
        assert_eq!(task, GlmOcrTask::Ocr);
        assert_eq!(device, DevicePreference::Auto);
    }

    #[test]
    fn test_parse_options_custom_task() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({"task": "table"}));
        let (task, _device, _layout_mode) = GlmOcrBackend::parse_options(&config);
        assert_eq!(task, GlmOcrTask::Table);
    }

    #[test]
    fn test_parse_options_formula_task() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({"task": "formula"}));
        let (task, _device, _layout_mode) = GlmOcrBackend::parse_options(&config);
        assert_eq!(task, GlmOcrTask::Formula);
    }

    #[test]
    fn test_parse_options_custom_device() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({"device": "cpu"}));
        let (_task, device, _layout_mode) = GlmOcrBackend::parse_options(&config);
        assert_eq!(device, DevicePreference::Cpu);
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
        assert_eq!(task_for_label(LayoutClass::Table), GlmOcrTask::Table);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_task_for_label_formula() {
        use crate::layout::LayoutClass;
        assert_eq!(task_for_label(LayoutClass::Formula), GlmOcrTask::Formula);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_task_for_label_text() {
        use crate::layout::LayoutClass;
        assert_eq!(task_for_label(LayoutClass::Text), GlmOcrTask::Ocr);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_wrap_output_formula() {
        let wrapped = wrap_output(GlmOcrTask::Formula, "x^2 + y^2 = r^2");
        assert!(wrapped.starts_with("$$\n"));
        assert!(wrapped.ends_with("\n$$"));
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
}
