//! DeepSeek-OCR backend plugin for the Xberg OCR pipeline.
//!
//! This module wraps the candle-based DeepSeek-OCR engine in the `OcrBackend`
//! trait, making it available to the extraction pipeline.
//!
//! # Engine pool design
//!
//! Calls with identical engine configuration share an engine instance to avoid
//! redundant weight loading.

use async_trait::async_trait;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use crate::Result;
use crate::candle_ocr::config::{
    CandleDeepseekOcrDtype, DeepseekOcrBackendOptions, parse_backend_options, validate_optional_non_empty,
};
use crate::core::config::OcrConfig;
use crate::engine_cache::EngineCache;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use xberg_candle_ocr::DType;
use xberg_candle_ocr::Device;
use xberg_candle_ocr::DevicePreference;
use xberg_candle_ocr::models::DeepseekOCREngine;

/// Pick the floating-point precision that matches the actual compute device.
///
/// A BF16 checkpoint loaded as F32 doubles the weight footprint for no accuracy benefit -- on
/// an L4 (24 GB) this was the 16.5 GB reported in #1674 for a 3B-parameter model whose weights
/// are 6.67 GB in their native BF16 form. Metal's BF16 kernel coverage is incomplete, so F16 is
/// the safe default there instead; CPU inference stays F32 for broad portability.
fn default_dtype_for(device: &Device) -> DType {
    match device {
        Device::Cuda(_) => DType::BF16,
        Device::Metal(_) => DType::F16,
        Device::Cpu => DType::F32,
    }
}

/// Map a parsed `backend_options.dtype` request to a concrete [`DType`]. `Auto` defers to
/// [`default_dtype_for`], so it returns `None` here.
fn requested_dtype(value: Option<CandleDeepseekOcrDtype>) -> Option<DType> {
    match value {
        None | Some(CandleDeepseekOcrDtype::Auto) => None,
        Some(CandleDeepseekOcrDtype::F32) => Some(DType::F32),
        Some(CandleDeepseekOcrDtype::F16) => Some(DType::F16),
        Some(CandleDeepseekOcrDtype::Bf16) => Some(DType::BF16),
    }
}

/// Engine pool key, unchanged from before GH#1722: keyed by the CONCRETE `dtype`, not the raw
/// request. Keying on the raw request instead (an earlier version of this fix did that) made
/// the key finer: a caller who leaves `dtype` unset and a caller who asks for the device's own
/// default dtype explicitly (e.g. `"bf16"` on CUDA) stopped sharing one engine and each paid
/// for a full cold start, a second multi-GB model resident for the life of the process since
/// the pool never evicts. Resolving `dtype` before the key is built (see [`resolve_device_once`]
/// below) avoids that split without touching this key at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EnginePoolKey {
    preference: DevicePreference,
    dtype: DType,
    model_path: std::path::PathBuf,
    version: usize,
}

impl EnginePoolKey {
    fn new(preference: DevicePreference, dtype: DType, model_path: &str, version: usize) -> Self {
        Self {
            preference,
            dtype,
            model_path: model_path.into(),
            version,
        }
    }
}

/// Pooled engine value: shared reference with interior mutability for the engine.
type PooledEngine = Arc<parking_lot::Mutex<DeepseekOCREngine>>;

static ENGINE_POOL: LazyLock<EngineCache<EnginePoolKey, parking_lot::Mutex<DeepseekOCREngine>>> =
    LazyLock::new(EngineCache::unbounded);

/// Resolved devices keyed by the preference that produced them.
///
/// This is the same [`EngineCache`] the engine pool above uses, at a different key and value.
/// It is a separate instance rather than a second use of `ENGINE_POOL`: the device is resolved
/// before the dtype is known, and folding it into the pool would re-split the pool key on dtype
/// (see [`EnginePoolKey`]) and hold two identical engines resident.
static DEVICE_CACHE: LazyLock<EngineCache<DevicePreference, Device>> = LazyLock::new(EngineCache::unbounded);

/// Resolve `preference` to a [`Device`], memoized per preference for the life of the process.
///
/// `process_image` calls this on every page, but the underlying accelerator probe -- and the
/// 'auto' fallback warning it can log -- only runs on the first call for a given preference
/// instead of once per page (GH#1722): a 731-page document used to log 731 identical warnings.
/// GLM-OCR, PaddleOCR-VL and TrOCR avoid the same problem by resolving inside their engine
/// pool's cold start instead; DeepSeek-OCR cannot do that without re-splitting the engine pool
/// key on dtype (see [`EnginePoolKey`]), so it memoizes the device directly instead.
fn resolve_device_once(preference: DevicePreference) -> crate::Result<Device> {
    resolve_device_in(&DEVICE_CACHE, preference, DevicePreference::select)
}

/// Resolve `preference` through `select` on the first call for that preference and reuse the
/// device afterwards, in `cache`.
///
/// `cache` and `select` are parameters so a test can drive a fresh, non-shared cache with a
/// counting stand-in: the real accelerator probe has no observable call count of its own, so
/// that is the only way to pin "resolved once" rather than "resolved once, probably."
///
/// [`EngineCache::get_or_try_init`] keeps nothing when `init` returns `Err`, so a failed probe
/// is retried on the next page rather than pinning the process to its first answer. An explicit
/// `Cuda` preference whose init fails transiently therefore still recovers, which is what it did
/// before this cache existed and what the peer backends do today. `Auto` is the exception, and
/// deliberately so: its CPU fallback is a *success*, so it is cached like any other, and a host
/// whose accelerator never appears probes once instead of once per page. That is the 731-line
/// log this cache exists to remove.
fn resolve_device_in(
    cache: &EngineCache<DevicePreference, Device>,
    preference: DevicePreference,
    select: impl FnOnce(DevicePreference) -> xberg_candle_ocr::Result<Device>,
) -> crate::Result<Device> {
    cache
        .get_or_try_init(preference, || {
            select(preference).map_err(|e| crate::XbergError::Ocr {
                message: format!("Failed to select compute device: {e}"),
                source: Some(Box::new(e)),
            })
        })
        .map(|device| (*device).clone())
}

fn get_or_init_engine(
    preference: DevicePreference,
    device: Device,
    dtype: DType,
    model_path: &str,
    version: usize,
) -> crate::Result<PooledEngine> {
    let key = EnginePoolKey::new(preference, dtype, model_path, version);

    ENGINE_POOL.get_or_try_init(key, || {
        tracing::info!(
            preference = ?preference,
            ?dtype,
            model_path = %model_path,
            "Initialising DeepSeek-OCR engine (cold start)"
        );

        let new_engine =
            DeepseekOCREngine::init(model_path, device, dtype, version).map_err(|e| crate::XbergError::Ocr {
                message: format!("DeepSeek-OCR engine initialisation failed: {e}"),
                source: Some(Box::new(e)),
            })?;
        Ok(parking_lot::Mutex::new(new_engine))
    })
}

/// Default HuggingFace repo id for DeepSeek-OCR weights: the original
/// `deepseek-ai/DeepSeek-OCR` repository, pinned to an immutable, checksum-verified
/// revision (see `model_stager::DEEPSEEK_OCR`). Used when `backend_options` provides
/// neither `model_path` nor a custom `model_id`.
const DEFAULT_MODEL_ID: &str = "deepseek-ai/DeepSeek-OCR";

/// DeepSeek-OCR backend using candle transformers.
///
/// A vision-language model combining SAM vision encoder, ViT/Qwen2 vision
/// transformer, CLIP projection, and language decoder for multimodal OCR.
///
/// # Configuration
///
/// DeepSeek-OCR accepts backend options for weight source, device, version, and dtype:
/// ```json
/// {
///   "device": "auto",
///   "model_id": "deepseek-ai/DeepSeek-OCR",
///   "version": 2,
///   "dtype": "auto"
/// }
/// ```
///
/// - `device` (string): `"auto"` (default), `"cpu"`, `"cuda"`, `"metal"`
/// - `model_id` (string): HuggingFace repo id to auto-download weights from. Defaults to
///   `deepseek-ai/DeepSeek-OCR`, pinned to a checksum-verified revision. Ignored when
///   `model_path` is set.
/// - `model_path` (string, optional): path to a local model directory. Takes precedence
///   over `model_id`. When omitted, the weights named by `model_id` are downloaded on
///   first use into the standard Hugging Face cache -- no manual staging required.
/// - `hf_revision` (string, optional): immutable commit for a custom `model_id`. The
///   default model is pinned automatically.
/// - `cache_dir` (string, optional): explicit Hugging Face Hub cache root. When omitted,
///   `HF_HUB_CACHE`, `HUGGINGFACE_HUB_CACHE`, and `HF_HOME` are honored.
/// - `version` (integer): model version `1` or `2` (default: `2`)
/// - `dtype` (string): `"auto"` (default, picks BF16 on CUDA / F16 on Metal / F32 on CPU),
///   `"f32"`, `"f16"`, or `"bf16"`. A dtype with no kernel on the selected device fails the
///   load hard rather than silently falling back -- this is not a tuning knob.
#[cfg_attr(alef, alef(skip))]
pub struct DeepseekOcrBackend {
    dtype: Option<DType>,
}

/// Parsed, defaulted `candle-deepseek-ocr` backend configuration for a single call.
#[derive(Debug)]
struct DeepseekOcrOptions {
    model_path: Option<String>,
    model_id: String,
    hf_revision: Option<String>,
    cache_dir: Option<PathBuf>,
    device: DevicePreference,
    version: usize,
    /// `None` defers to [`default_dtype_for`] once the device is resolved.
    dtype: Option<DType>,
}

impl DeepseekOcrBackend {
    /// Create a new DeepSeek-OCR backend.
    ///
    /// The data type is auto-selected per compute device (see [`default_dtype_for`]) unless
    /// overridden by [`DeepseekOcrBackend::with_dtype`] or a `backend_options.dtype` request,
    /// which takes precedence over both.
    pub fn new() -> Self {
        Self { dtype: None }
    }

    /// Force a specific floating-point precision regardless of device.
    ///
    /// A per-call `backend_options.dtype` request still takes precedence over this
    /// constructor-level override.
    pub fn with_dtype(mut self, dtype: DType) -> Self {
        self.dtype = Some(dtype);
        self
    }

    /// Parse backend options to extract DeepSeek-OCR-specific configuration.
    ///
    /// Device selection is delegated to [`crate::candle_ocr::resolve_device_preference`]
    /// so the central `AccelerationConfig` is honoured. `model_id` defaults to
    /// [`DEFAULT_MODEL_ID`] and is only consulted when `model_path` is absent.
    fn parse_options(&self, config: &OcrConfig) -> Result<DeepseekOcrOptions> {
        let options: DeepseekOcrBackendOptions =
            parse_backend_options(config.backend_options.as_ref(), "candle-deepseek-ocr")?;
        for (field, value) in [
            ("model_path", options.model_path.as_deref()),
            ("model_id", options.model_id.as_deref()),
            ("hf_revision", options.hf_revision.as_deref()),
            ("cache_dir", options.cache_dir.as_deref()),
        ] {
            validate_optional_non_empty(value, "candle-deepseek-ocr", field)?;
        }
        let version = options.version.unwrap_or(2);
        if !matches!(version, 1 | 2) {
            return Err(crate::XbergError::validation(format!(
                "invalid candle-deepseek-ocr backend_options.version: expected 1 or 2, got {version}"
            )));
        }
        Ok(DeepseekOcrOptions {
            model_path: options.model_path,
            model_id: options.model_id.unwrap_or_else(|| DEFAULT_MODEL_ID.to_string()),
            hf_revision: options.hf_revision,
            cache_dir: options.cache_dir.map(PathBuf::from),
            device: super::resolve_device_preference(config, options.device),
            version: version as usize,
            dtype: requested_dtype(options.dtype).or(self.dtype),
        })
    }
}

impl Default for DeepseekOcrBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DeepseekOcrBackend {
    fn name(&self) -> &str {
        "candle-deepseek-ocr"
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        tracing::debug!("Initializing DeepSeek-OCR backend");
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Inherits the `RequiresUpright` default for `page_orientation_handling` — unmeasured, not validated (#657).
#[async_trait]
impl OcrBackend for DeepseekOcrBackend {
    /// Process an image using the DeepSeek-OCR engine.
    ///
    /// # Errors
    ///
    /// Returns [`crate::XbergError::Validation`] if `image_bytes` is empty. Returns
    /// [`crate::XbergError::Ocr`] if weight download, device selection, engine
    /// initialisation, or inference fails.
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        if image_bytes.is_empty() {
            return Err(crate::XbergError::Validation {
                message: "Empty image data provided to DeepSeek-OCR".to_string(),
                source: None,
            });
        }

        let options = self.parse_options(config)?;
        let image_bytes_owned = image_bytes.to_vec();

        let content = tokio::task::spawn_blocking(move || {
            let model_path = match options.model_path {
                Some(path) => PathBuf::from(path),
                None => super::model_stager::ensure_deepseek_ocr(
                    &options.model_id,
                    options.hf_revision.as_deref(),
                    options.cache_dir.as_deref(),
                )
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("DeepSeek-OCR weight download failed: {e}"),
                    source: None,
                })?,
            };
            let model_path = model_path.to_string_lossy().into_owned();

            let device = resolve_device_once(options.device)?;
            let dtype = options.dtype.unwrap_or_else(|| default_dtype_for(&device));
            let engine = get_or_init_engine(options.device, device, dtype, &model_path, options.version)?;
            let mut engine_guard = engine.lock();
            let output = engine_guard
                .process_image(&image_bytes_owned, None)
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("DeepSeek-OCR inference failed: {e}"),
                    source: Some(Box::new(e)),
                })?;
            Ok::<String, crate::XbergError>(output)
        })
        .await
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("DeepSeek-OCR task execution failed: {e}"),
            source: None,
        })??;

        Ok(super::ocr_result::build_ocr_document(
            content,
            Vec::new(),
            image_bytes,
            config,
            super::ocr_result::OcrDocumentContext {
                mime_type: Cow::Borrowed("text/markdown"),
                backend_name: "candle-deepseek-ocr",
                // DeepSeek-OCR has no task selection; every call is plain-text OCR. ~keep
                plain_text_task: true,
            },
        ))
    }

    /// Process an image file using the DeepSeek-OCR engine.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or if inference fails.
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

    /// DeepSeek-OCR reports no page-level confidence.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::None
    }

    // Rotation handling has not been measured for this backend; it stays on the trait's
    // `RequiresUpright` default.
}

#[cfg(test)]
mod tests {
    use ahash::AHashMap;

    use super::*;

    #[test]
    fn test_deepseek_ocr_backend_creation() {
        let backend = DeepseekOcrBackend::new();
        assert_eq!(backend.name(), "candle-deepseek-ocr");
        assert_eq!(backend.backend_type(), OcrBackendType::Candle);
    }

    #[test]
    fn test_deepseek_ocr_emits_structured_markdown() {
        let backend = DeepseekOcrBackend::new();
        assert!(backend.emits_structured_markdown());
    }

    #[test]
    fn test_deepseek_ocr_language_support() {
        let backend = DeepseekOcrBackend::new();
        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("zho"));
        assert!(backend.supports_language("jpn"));
        assert!(backend.supports_language("unknown"));
    }

    #[test]
    fn test_deepseek_ocr_supported_languages() {
        let backend = DeepseekOcrBackend::new();
        let langs = backend.supported_languages();
        assert!(langs.contains(&"eng".to_string()));
        assert!(langs.contains(&"zho".to_string()));
        assert!(langs.contains(&"fra".to_string()));
    }

    #[test]
    fn test_parse_options_defaults() {
        let config = OcrConfig::default();
        let options = DeepseekOcrBackend::new().parse_options(&config).unwrap();
        assert!(options.model_path.is_none());
        assert_eq!(options.device, DevicePreference::Auto);
        assert_eq!(options.version, 2);
        assert_eq!(options.dtype, None);
    }

    #[test]
    fn test_parse_options_defaults_model_id() {
        let config = OcrConfig::default();
        let options = DeepseekOcrBackend::new().parse_options(&config).unwrap();
        assert_eq!(options.model_id, DEFAULT_MODEL_ID);
        assert_eq!(DEFAULT_MODEL_ID, "deepseek-ai/DeepSeek-OCR");
    }

    #[test]
    fn test_parse_options_model_path() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"model_path": "/models/deepseek"})),
            ..Default::default()
        };
        let options = DeepseekOcrBackend::new().parse_options(&config).unwrap();
        assert_eq!(options.model_path.as_deref(), Some("/models/deepseek"));
    }

    #[test]
    fn test_parse_options_custom_model_id_and_revision() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"model_id": "example/deepseek-ocr", "hf_revision": "abc123"})),
            ..Default::default()
        };
        let options = DeepseekOcrBackend::new().parse_options(&config).unwrap();
        assert_eq!(options.model_id, "example/deepseek-ocr");
        assert_eq!(options.hf_revision.as_deref(), Some("abc123"));
        assert!(options.model_path.is_none());
    }

    #[test]
    fn test_parse_options_custom_device() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"device": "cpu"})),
            ..Default::default()
        };
        let options = DeepseekOcrBackend::new().parse_options(&config).unwrap();
        assert_eq!(options.device, DevicePreference::Cpu);
    }

    #[test]
    fn test_parse_options_rejects_unsupported_version() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"version": 3})),
            ..Default::default()
        };
        let error = DeepseekOcrBackend::new()
            .parse_options(&config)
            .unwrap_err()
            .to_string();
        assert!(error.contains("backend_options.version"));
    }

    #[test]
    fn test_parse_options_accepts_supported_version() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"version": 1})),
            ..Default::default()
        };
        let options = DeepseekOcrBackend::new().parse_options(&config).unwrap();
        assert_eq!(options.version, 1);
    }

    #[test]
    fn test_parse_options_non_object_json_returns_contextual_error() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!(null)),
            ..Default::default()
        };
        let error = DeepseekOcrBackend::new()
            .parse_options(&config)
            .unwrap_err()
            .to_string();
        assert!(error.contains("candle-deepseek-ocr backend_options"));
    }

    #[test]
    fn test_parse_options_empty_object_returns_defaults() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({})),
            ..Default::default()
        };
        let options = DeepseekOcrBackend::new().parse_options(&config).unwrap();
        assert!(options.model_path.is_none());
        assert_eq!(options.device, DevicePreference::Auto);
        assert_eq!(options.version, 2);
        assert_eq!(options.dtype, None);
    }

    #[test]
    fn test_parse_options_explicit_dtype() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"dtype": "bf16"})),
            ..Default::default()
        };
        let options = DeepseekOcrBackend::new().parse_options(&config).unwrap();
        assert_eq!(options.dtype, Some(DType::BF16));
    }

    #[test]
    fn test_parse_options_auto_dtype_resolves_to_none() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"dtype": "auto"})),
            ..Default::default()
        };
        let options = DeepseekOcrBackend::new().parse_options(&config).unwrap();
        assert_eq!(options.dtype, None);
    }

    #[test]
    fn test_parse_options_constructor_dtype_used_when_options_dtype_absent() {
        let config = OcrConfig::default();
        let options = DeepseekOcrBackend::new()
            .with_dtype(DType::F16)
            .parse_options(&config)
            .unwrap();
        assert_eq!(options.dtype, Some(DType::F16));
    }

    #[test]
    fn test_parse_options_explicit_dtype_overrides_constructor_dtype() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"dtype": "bf16"})),
            ..Default::default()
        };
        let options = DeepseekOcrBackend::new()
            .with_dtype(DType::F16)
            .parse_options(&config)
            .unwrap();
        assert_eq!(options.dtype, Some(DType::BF16));
    }

    #[test]
    fn default_dtype_for_cpu_is_f32() {
        assert_eq!(default_dtype_for(&Device::Cpu), DType::F32);
    }

    #[test]
    fn requested_dtype_maps_each_explicit_variant() {
        assert_eq!(requested_dtype(None), None);
        assert_eq!(requested_dtype(Some(CandleDeepseekOcrDtype::Auto)), None);
        assert_eq!(requested_dtype(Some(CandleDeepseekOcrDtype::F32)), Some(DType::F32));
        assert_eq!(requested_dtype(Some(CandleDeepseekOcrDtype::F16)), Some(DType::F16));
        assert_eq!(requested_dtype(Some(CandleDeepseekOcrDtype::Bf16)), Some(DType::BF16));
    }

    #[test]
    fn test_initialize_and_shutdown() {
        let backend = DeepseekOcrBackend::new();
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    #[test]
    fn engine_pool_reuses_equal_configs_and_isolates_distinct_configs() {
        let original = EnginePoolKey::new(DevicePreference::Cpu, DType::F32, "/models/v1", 1);
        let equal = EnginePoolKey::new(DevicePreference::Cpu, DType::F32, "/models/v1", 1);
        let mut pool = AHashMap::new();
        pool.insert(original, 7_u8);

        assert_eq!(pool.get(&equal), Some(&7));
        assert_eq!(
            pool.get(&EnginePoolKey::new(DevicePreference::Cpu, DType::F32, "/models/v2", 1)),
            None
        );
        assert_eq!(
            pool.get(&EnginePoolKey::new(DevicePreference::Cpu, DType::F32, "/models/v1", 2)),
            None
        );
        assert_eq!(
            pool.get(&EnginePoolKey::new(DevicePreference::Auto, DType::F32, "/models/v1", 1)),
            None
        );
    }

    /// Exercises [`resolve_device_in`] -- the exact memoization `resolve_device_once` wraps
    /// around the shared process-wide `DEVICE_CACHE` -- against a FRESH, non-shared cache
    /// instance, with a counting stand-in in place of [`DevicePreference::select`]. Two things
    /// forced that shape rather than calling `resolve_device_once` or `process_image` directly:
    ///
    /// - The real `DEVICE_CACHE` is a single process-wide static; Rust's test runner shares one
    ///   process across every test in this binary, so a test against the shared instance would
    ///   see whatever an earlier test (or a later one, run in a different order) already cached
    ///   for the same preference, rather than a clean first call.
    /// - `DevicePreference::select` has no observable call count of its own -- on a CPU-only
    ///   build it is a cheap, deterministic match with no counter to read -- so "resolved once"
    ///   can only be pinned by substituting a selector that counts, not by calling the real one
    ///   and inspecting a side effect it does not have.
    ///
    /// This pins that `resolve_device_in` calls its `select` argument at most once across
    /// repeated calls with the same preference. It does NOT exercise `process_image` or the
    /// shared `resolve_device_once` wrapper end to end -- doing that would need a real model
    /// download -- so it would not by itself catch a regression where a future change calls
    /// `DevicePreference::select` directly again somewhere in `process_image`, bypassing
    /// `resolve_device_once` entirely. What guards against that is `resolve_device_once` being
    /// the only place in this file that calls `DevicePreference::select`.
    #[test]
    fn device_cache_resolves_each_preference_once_across_several_calls() {
        let cache = EngineCache::unbounded();
        let selects = std::sync::atomic::AtomicUsize::new(0);

        for page in 0..5 {
            let device = resolve_device_in(&cache, DevicePreference::Auto, |_preference| {
                selects.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(Device::Cpu)
            });
            assert!(device.is_ok(), "page {page}: {:?}", device.err());
        }

        assert_eq!(
            selects.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the selector must run once per preference, not once per call"
        );
    }

    /// The four `DevicePreference` variants must land under four distinct cache keys: resolving
    /// one preference must not short-circuit a later call for a different one.
    #[test]
    fn device_cache_keeps_each_preference_in_its_own_slot() {
        let cache = EngineCache::unbounded();
        let selects = std::sync::atomic::AtomicUsize::new(0);
        let counting_select = |preference: DevicePreference| {
            selects.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match preference {
                DevicePreference::Cpu | DevicePreference::Auto => Ok(Device::Cpu),
                DevicePreference::Cuda | DevicePreference::Metal => Err(
                    xberg_candle_ocr::CandleOcrError::UnsupportedConfig("no accelerator on this test host".into()),
                ),
            }
        };

        assert!(resolve_device_in(&cache, DevicePreference::Cpu, counting_select).is_ok());
        assert!(resolve_device_in(&cache, DevicePreference::Auto, counting_select).is_ok());
        assert!(resolve_device_in(&cache, DevicePreference::Cuda, counting_select).is_err());
        assert!(resolve_device_in(&cache, DevicePreference::Metal, counting_select).is_err());

        assert_eq!(
            selects.load(std::sync::atomic::Ordering::SeqCst),
            4,
            "four distinct preferences must each resolve once, under their own key"
        );
    }

    /// A failed probe must not be memoized. The peer backends reach the same cache through
    /// [`EngineCache::get_or_try_init`], which keeps nothing when `init` fails (see
    /// `get_or_try_init_keeps_nothing_when_init_fails`), and before this cache existed
    /// DeepSeek-OCR re-selected on every page, so a transient accelerator failure healed by
    /// itself. Caching the `Err` would have pinned the process to its first answer for good.
    ///
    /// The stand-in fails once and then succeeds, which is exactly the transient shape.
    #[test]
    fn a_failed_device_probe_is_retried_rather_than_cached() {
        let cache = EngineCache::unbounded();
        let selects = std::sync::atomic::AtomicUsize::new(0);
        let select_device = |_preference: DevicePreference| -> xberg_candle_ocr::Result<Device> {
            let attempt = selects.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if attempt == 0 {
                return Err(xberg_candle_ocr::CandleOcrError::UnsupportedConfig(
                    "accelerator busy on the first attempt".into(),
                ));
            }
            Ok(Device::Cpu)
        };

        assert!(
            resolve_device_in(&cache, DevicePreference::Cuda, select_device).is_err(),
            "the first probe fails"
        );
        assert!(
            resolve_device_in(&cache, DevicePreference::Cuda, select_device).is_ok(),
            "a cached Err would make the second call fail too, pinning the process to the failure"
        );
        assert_eq!(
            selects.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "the failed probe must be retried, so the selector runs a second time"
        );
    }

    /// The error a failed probe returns keeps the candle error as its `source`. The cache holds
    /// the `XbergError` itself rather than a `String`, so nothing has to be stringified to make
    /// it cacheable and the chain survives.
    #[test]
    fn a_failed_device_probe_keeps_the_underlying_error_as_its_source() {
        let cache = EngineCache::unbounded();
        let error = resolve_device_in(&cache, DevicePreference::Cuda, |_preference| {
            Err(xberg_candle_ocr::CandleOcrError::UnsupportedConfig(
                "no accelerator on this test host".into(),
            ))
        })
        .expect_err("a failed probe must be an error");

        match error {
            crate::XbergError::Ocr { source, .. } => {
                let source = source.expect("the candle error must survive as the source");
                assert!(
                    source.to_string().contains("no accelerator on this test host"),
                    "the source must be the underlying error, got: {source}"
                );
            }
            other => panic!("expected an Ocr error, got {other:?}"),
        }
    }

    /// Deliberately unmemoized baseline for the RED half of the fix: identical to
    /// `resolve_device_in` except it calls `select` on every call instead of caching it
    /// after the first. Run only by the test below, never by production code, to show the same
    /// assertion `device_cache_resolves_each_preference_once_across_several_calls` makes fails
    /// without the cache -- i.e. that the assertion is capable of catching the bug the cache
    /// fixes, not just of passing.
    fn resolve_without_memoizing(
        preference: DevicePreference,
        select: impl Fn(DevicePreference) -> xberg_candle_ocr::Result<Device>,
    ) -> crate::Result<Device> {
        select(preference).map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to select compute device: {e}"),
            source: Some(Box::new(e)),
        })
    }

    #[test]
    fn unmemoized_resolution_would_fail_the_once_per_preference_assertion() {
        let selects = std::sync::atomic::AtomicUsize::new(0);
        let select_device = |_preference: DevicePreference| -> xberg_candle_ocr::Result<Device> {
            selects.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Device::Cpu)
        };

        for page in 0..5 {
            let device = resolve_without_memoizing(DevicePreference::Auto, select_device);
            assert!(device.is_ok(), "page {page}: {:?}", device.err());
        }

        assert_eq!(
            selects.load(std::sync::atomic::Ordering::SeqCst),
            5,
            "the unmemoized baseline resolves once per call -- this is the GH#1722 bug shape, \
             confirming the assertion above is not vacuously true"
        );
    }
}
