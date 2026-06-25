//! TrOCR backend plugin for the Xberg OCR pipeline.
//!
//! This module wraps the candle-based TrOCR engine in the `OcrBackend` trait,
//! making it available to the extraction pipeline.

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
use xberg_candle_ocr::DevicePreference;
use xberg_candle_ocr::models::{TrocrEngine, TrocrVariant};

/// `TrocrVariant` is `PartialEq + Eq + Copy` but does not derive `Hash`.
/// We map it to a `u8` discriminant to form a hashable pool key.
fn variant_discriminant(v: TrocrVariant) -> u8 {
    match v {
        TrocrVariant::BasePrinted => 0,
        TrocrVariant::LargePrinted => 1,
        TrocrVariant::BaseHandwritten => 2,
        TrocrVariant::LargeHandwritten => 3,
    }
}

/// Type alias for the engine pool mapping.
type EnginePoolMap = AHashMap<(u8, DevicePreference), Arc<TrocrEngine>>;

/// Process-wide engine pool keyed by `(variant_discriminant, device_preference)`.
///
/// `TrocrEngine` initialisation downloads and parses safetensors weights from HF Hub
/// and is expensive (~400 MB per variant). The pool ensures each
/// `(variant, device)` combination is loaded at most once per process.
static ENGINE_POOL: LazyLock<RwLock<EnginePoolMap>> = LazyLock::new(|| RwLock::new(AHashMap::default()));

/// Return a cached engine for `(variant, preference)`, initialising one on first use.
///
/// Uses a read → miss → write → double-check pattern so that two racing callers
/// do not both pay the initialisation cost.
fn get_or_init_engine(variant: TrocrVariant, preference: DevicePreference) -> crate::Result<Arc<TrocrEngine>> {
    let key = (variant_discriminant(variant), preference);

    // Fast path: engine already in pool.
    {
        let pool = ENGINE_POOL.read();
        if let Some(engine) = pool.get(&key) {
            return Ok(Arc::clone(engine));
        }
    }

    // Slow path: select the device and build the engine, then insert under write lock.
    let device = preference.select().map_err(|e| crate::XbergError::Ocr {
        message: format!("Failed to select compute device: {e}"),
        source: Some(Box::new(e)),
    })?;

    tracing::info!(variant = ?variant, preference = ?preference, "Initialising TrOCR engine (cold start)");
    let new_engine = TrocrEngine::new(variant, device).map_err(|e| crate::XbergError::Ocr {
        message: format!("TrOCR engine initialisation failed: {e}"),
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

/// TrOCR backend using candle transformers.
///
/// Recognizes text in images via Microsoft's TrOCR model. Supports printed
/// and handwritten text with four model variants (base/large × printed/handwritten).
///
/// # Important: line-level only
///
/// TrOCR is trained to recognise a single line of text per image. When given a
/// full-page document the model will typically decode only the most prominent
/// region and produce nearly-empty output. Use TrOCR for cropped text regions —
/// upstream stages must detect text regions (e.g. PaddleOCR text detector or a
/// layout model) and hand each crop to this backend individually. For full-page
/// VLM OCR use [`PaddleOcrVlBackend`](super::PaddleOcrVlBackend) instead.
///
/// # Configuration
///
/// TrOCR accepts backend options for runtime tuning:
/// ```json
/// {
///   "variant": "base-printed",
///   "device": "auto"
/// }
/// ```
///
/// - `variant` (string): `"base-printed"` (default), `"large-printed"`, `"base-handwritten"`, `"large-handwritten"`
/// - `device` (string): `"auto"`, `"cpu"`, `"cuda"`, `"metal"`
#[cfg_attr(alef, alef(skip))]
pub struct TrocrBackend {
    variant: TrocrVariant,
}

impl TrocrBackend {
    /// Create a new TrOCR backend with the specified variant.
    pub fn new(variant: TrocrVariant) -> Self {
        Self { variant }
    }

    /// Create a TrOCR backend with the default variant (base-printed).
    pub fn default_variant() -> Self {
        Self::new(TrocrVariant::default())
    }

    /// Parse backend options to extract TrOCR-specific configuration.
    ///
    /// Returns `(Some(variant), device)` only when `backend_options` contains an explicit
    /// `"variant"` key. Returns `None` for the variant when the key is absent, so the
    /// caller can fall back to the constructor-time default stored in `self.variant`.
    ///
    /// Device selection is delegated to [`crate::candle_ocr::resolve_device_preference`]
    /// so the central `AccelerationConfig` is honoured.
    fn parse_options(config: &OcrConfig) -> (Option<TrocrVariant>, DevicePreference) {
        let mut variant: Option<TrocrVariant> = None;

        if let Some(opts) = &config.backend_options
            && let Some(v) = opts.get("variant").and_then(|v| v.as_str())
        {
            variant = Some(match v {
                "large-printed" => TrocrVariant::LargePrinted,
                "base-handwritten" => TrocrVariant::BaseHandwritten,
                "large-handwritten" => TrocrVariant::LargeHandwritten,
                _ => TrocrVariant::BasePrinted, // default on unknown
            });
        }

        let device = super::resolve_device_preference(config);
        (variant, device)
    }
}

impl Plugin for TrocrBackend {
    fn name(&self) -> &str {
        "candle-trocr"
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        tracing::debug!("Initializing TrOCR backend: {}", self.variant.description());
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for TrocrBackend {
    /// Recognize text in `image_bytes` via the configured TrOCR variant and device.
    ///
    /// The variant is resolved by taking any explicit `"variant"` key from
    /// `config.backend_options`, falling back to the constructor-time variant stored
    /// in `self.variant`. Inference runs inside `tokio::task::spawn_blocking` so the
    /// async runtime is never blocked.
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        // Parse configuration
        let (parsed_variant, device) = Self::parse_options(config);
        let variant = parsed_variant.unwrap_or(self.variant);

        // Validate image data
        if image_bytes.is_empty() {
            return Err(crate::XbergError::Validation {
                message: "Empty image data provided to TrOCR".to_string(),
                source: None,
            });
        }

        // Clone image bytes for the blocking task
        let image_bytes = image_bytes.to_vec();

        // Run inference in a blocking task to avoid blocking the async runtime
        let content = tokio::task::spawn_blocking(move || {
            // Retrieve a cached engine or initialise one on first use.
            // Device selection happens inside get_or_init_engine on first call;
            // subsequent calls for the same (variant, device) reuse the pooled engine.
            let engine = get_or_init_engine(variant, device)?;

            // Process image through encoder-decoder pipeline
            let output = engine.process_image(&image_bytes).map_err(|e| crate::XbergError::Ocr {
                message: format!("TrOCR inference failed: {}", e),
                source: Some(Box::new(e)),
            })?;

            Ok::<String, crate::XbergError>(output.content)
        })
        .await
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("TrOCR task execution failed: {}", e),
            source: None,
        })??;

        Ok(ExtractionResult {
            content,
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        })
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, lang: &str) -> bool {
        // TrOCR base-printed is trained primarily on English and works best
        // on English text. Other variants may support other languages but
        // comprehensive support requires additional fine-tuning.
        matches!(lang, "eng" | "en")
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["eng".to_string(), "en".to_string()]
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Candle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trocr_backend_creation() {
        let backend = TrocrBackend::default_variant();
        assert_eq!(backend.name(), "candle-trocr");
        assert_eq!(backend.backend_type(), OcrBackendType::Candle);
    }

    #[test]
    fn test_trocr_language_support() {
        let backend = TrocrBackend::default_variant();
        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("en"));
        assert!(!backend.supports_language("deu"));
        assert!(!backend.supports_language("fra"));
    }

    #[test]
    fn test_trocr_supported_languages() {
        let backend = TrocrBackend::default_variant();
        let langs = backend.supported_languages();
        assert!(langs.contains(&"eng".to_string()));
        assert!(langs.contains(&"en".to_string()));
    }

    #[test]
    fn test_parse_options_defaults() {
        let config = OcrConfig::default();
        let (variant, device) = TrocrBackend::parse_options(&config);
        // No "variant" key in options → None, caller falls back to self.variant
        assert_eq!(variant, None);
        assert_eq!(device, DevicePreference::Auto);
    }

    #[test]
    fn test_parse_options_custom_variant() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({
            "variant": "large-printed"
        }));
        let (variant, _device) = TrocrBackend::parse_options(&config);
        assert_eq!(variant, Some(TrocrVariant::LargePrinted));
    }

    #[test]
    fn test_parse_options_custom_device() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({
            "device": "cpu"
        }));
        let (_variant, device) = TrocrBackend::parse_options(&config);
        assert_eq!(device, DevicePreference::Cpu);
    }

    #[test]
    fn test_initialize_and_shutdown() {
        let backend = TrocrBackend::default_variant();
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    #[test]
    fn test_engine_pool_key_mapping() {
        // Verify that variant_discriminant correctly maps variants to unique keys.
        // This ensures that the pool's key function does not accidentally conflate
        // different variants.
        let base_printed = variant_discriminant(TrocrVariant::BasePrinted);
        let large_printed = variant_discriminant(TrocrVariant::LargePrinted);
        let base_handwritten = variant_discriminant(TrocrVariant::BaseHandwritten);
        let large_handwritten = variant_discriminant(TrocrVariant::LargeHandwritten);

        // All variants must have distinct discriminants
        assert_eq!(base_printed, 0);
        assert_eq!(large_printed, 1);
        assert_eq!(base_handwritten, 2);
        assert_eq!(large_handwritten, 3);

        // Verify they are all unique
        let discriminants = [base_printed, large_printed, base_handwritten, large_handwritten];
        for (i, &d1) in discriminants.iter().enumerate() {
            for (j, &d2) in discriminants.iter().enumerate() {
                if i != j {
                    assert_ne!(d1, d2, "Discriminants for variants {} and {} must be unique", i, j);
                }
            }
        }
    }
}
