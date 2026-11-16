//! TrOCR backend plugin for the Xberg OCR pipeline.
//!
//! This module wraps the candle-based TrOCR engine in the `OcrBackend` trait,
//! making it available to the extraction pipeline.

use async_trait::async_trait;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use crate::Result;
use crate::candle_ocr::config::{
    CandleTrocrVariant, TrocrBackendOptions, parse_backend_options, validate_optional_non_empty,
};
use crate::core::config::OcrConfig;
use crate::engine_cache::EngineCache;
use crate::extraction::image_decode::{ImageDecodeBudget, PreparedCandleImageDecode};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use xberg_candle_ocr::DevicePreference;
use xberg_candle_ocr::models::{TrocrEngine, TrocrVariant};

/// Parse and check the TrOCR `backend_options`.
///
/// Configuration validation and every page both call this, so an invalid option
/// fails before any page runs. ~keep
pub(crate) fn check_backend_options(value: Option<&serde_json::Value>) -> Result<TrocrBackendOptions> {
    let options: TrocrBackendOptions = parse_backend_options(value, "candle-trocr")?;
    validate_optional_non_empty(options.cache_dir.as_deref(), "candle-trocr", "cache_dir")?;
    validate_optional_non_empty(options.hf_revision.as_deref(), "candle-trocr", "hf_revision")?;
    Ok(options)
}

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

/// Key for the engine cache: `(variant_discriminant, device_preference, cache_dir, revision)`.
type EngineKey = (u8, DevicePreference, PathBuf, String);

/// Tallest a single cropped text-line image is expected to be, in pixels.
///
/// TrOCR is trained on single-line crops (see `TrocrBackend` docs). A raster this
/// tall is far more likely to be a whole page handed to the backend by mistake than
/// an unusually large line crop: at the pipeline's 150 DPI page render, even a large
/// heading line is well under this, while a full US Letter or A4 page renders to
/// roughly 1650-2550px tall. Chosen with headroom above the `test_hello_world.png`
/// fixture (200px) used in `xberg-candle-ocr`'s own integration test.
const MAX_LINE_CROP_HEIGHT_PX: u32 = 300;

/// Reject image dimensions that look like a full page rather than a single
/// cropped text line, before it is handed to a model trained only on the latter.
///
/// TrOCR's [`ImageProcessor`](xberg_candle_ocr::models::image_processor::ImageProcessor)
/// force-resizes any input to a fixed 384x384 square, discarding the original aspect
/// ratio. Handed a whole-page raster, it does not error — it decodes, resizes, and runs
/// inference to completion, producing confident-looking but fabricated text (the model
/// decodes whatever line-shaped pattern the squashed page most resembles). Returning
/// `Ok` with hallucinated content is worse than failing loudly here: a caller checking
/// only the exit code has no way to tell the two apart otherwise.
fn reject_whole_page_input((width, height): (u32, u32)) -> Result<()> {
    if height > MAX_LINE_CROP_HEIGHT_PX {
        return Err(crate::XbergError::Validation {
            message: format!(
                "candle-trocr received a {width}x{height} image that looks like a full page, not a \
                 single cropped text line. TrOCR is trained on line-level crops and will silently \
                 hallucinate text on whole-page input instead of failing (the pipeline force-resizes \
                 any input to a 384x384 square, destroying page layout). Either: (1) crop the page \
                 into individual text lines/regions before calling this backend (e.g. via a text \
                 detector or layout model), or (2) use a full-page backend instead, such as \
                 `candle-paddleocr-vl`, `candle-glm-ocr`, `candle-deepseek-ocr`, `tesseract`, \
                 `paddle-ocr`, or `sceptre`."
            ),
            source: None,
        });
    }

    Ok(())
}

/// Process-wide engine cache keyed by `(variant_discriminant, device_preference, cache_dir, revision)`.
///
/// `TrocrEngine` initialisation downloads and parses safetensors weights from HF Hub
/// and is expensive (~400 MB per variant). The cache ensures each
/// `(variant, device, cache_dir, revision)` combination is loaded at most once per process.
static ENGINE_POOL: LazyLock<EngineCache<EngineKey, TrocrEngine>> = LazyLock::new(EngineCache::unbounded);

/// Return a cached engine for `(variant, preference)`, initialising one on first use.
///
/// Delegates to [`EngineCache::get_or_try_init`], which stays locked across the
/// load, so a second caller for the same key waits for the first engine
/// instead of building another.
fn get_or_init_engine(
    variant: TrocrVariant,
    preference: DevicePreference,
    cache_dir: PathBuf,
    revision: String,
) -> crate::Result<Arc<TrocrEngine>> {
    let key = (
        variant_discriminant(variant),
        preference,
        cache_dir.clone(),
        revision.clone(),
    );

    ENGINE_POOL.get_or_try_init(key, || {
        let device = preference.select().map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to select compute device: {e}"),
            source: Some(Box::new(e)),
        })?;

        tracing::info!(variant = ?variant, preference = ?preference, "Initialising TrOCR engine (cold start)");
        TrocrEngine::new_with_hf(variant, device, Some(&cache_dir), Some(&revision)).map_err(|e| {
            crate::XbergError::Ocr {
                message: format!("TrOCR engine initialisation failed: {e}"),
                source: Some(Box::new(e)),
            }
        })
    })
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
///   "device": "auto",
///   "cache_dir": "/path/to/huggingface/cache",
///   "hf_revision": "immutable-commit"
/// }
/// ```
///
/// - `variant` (string): `"base-printed"` (default), `"large-printed"`, `"base-handwritten"`, `"large-handwritten"`
/// - `device` (string): `"auto"`, `"cpu"`, `"cuda"`, `"metal"`
/// - `cache_dir` (string, optional): explicit Hugging Face Hub cache root
/// - `hf_revision` (string, optional): immutable model revision
#[cfg_attr(alef, alef(skip))]
pub struct TrocrBackend {
    variant: TrocrVariant,
}

struct TrocrOptions {
    variant: Option<TrocrVariant>,
    device: DevicePreference,
    cache_dir: Option<PathBuf>,
    hf_revision: Option<String>,
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
    fn parse_options(config: &OcrConfig) -> Result<TrocrOptions> {
        let options = check_backend_options(config.backend_options.as_ref())?;
        let variant = options.variant.map(|variant| match variant {
            CandleTrocrVariant::BasePrinted => TrocrVariant::BasePrinted,
            CandleTrocrVariant::LargePrinted => TrocrVariant::LargePrinted,
            CandleTrocrVariant::BaseHandwritten => TrocrVariant::BaseHandwritten,
            CandleTrocrVariant::LargeHandwritten => TrocrVariant::LargeHandwritten,
        });
        Ok(TrocrOptions {
            variant,
            device: super::resolve_device_preference(config, options.device),
            cache_dir: options.cache_dir.map(PathBuf::from),
            hf_revision: options.hf_revision,
        })
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

/// Inherits the `RequiresUpright` default for `page_orientation_handling` — unmeasured, not validated (#657).
#[async_trait]
impl OcrBackend for TrocrBackend {
    /// Recognize text in `image_bytes` via the configured TrOCR variant and device.
    ///
    /// The variant is resolved by taking any explicit `"variant"` key from
    /// `config.backend_options`, falling back to the constructor-time variant stored
    /// in `self.variant`. Inference runs inside `tokio::task::spawn_blocking` so the
    /// async runtime is never blocked.
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        ImageDecodeBudget::from_security_limits(&limits).validate(
            1,
            1,
            u64::try_from(image_bytes.len()).unwrap_or(u64::MAX),
        )?;
        self.process_image_owned(Arc::new(image_bytes.to_vec()), config).await
    }

    async fn process_image_owned(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let options = Self::parse_options(config)?;
        let variant = options.variant.unwrap_or(self.variant);
        let cache_dir = options.cache_dir.unwrap_or_else(hf_hub::resolve_cache_dir);
        let revision = options.hf_revision.unwrap_or_else(|| variant.revision().to_string());

        if image_bytes.is_empty() {
            return Err(crate::XbergError::Validation {
                message: "Empty image data provided to TrOCR".to_string(),
                source: None,
            });
        }

        let decode_error = |error| match error {
            error @ crate::XbergError::Validation { .. } => error,
            error => crate::XbergError::Ocr {
                message: format!("TrOCR image decode failed: {error}"),
                source: Some(Box::new(error)),
            },
        };
        let (content, image_bytes) = tokio::task::spawn_blocking(move || {
            let decode = PreparedCandleImageDecode::new(&image_bytes, &limits, false).map_err(decode_error)?;
            reject_whole_page_input(decode.dimensions())?;
            let img = decode.decode().map_err(decode_error)?;
            let engine = get_or_init_engine(variant, options.device, cache_dir, revision)?;

            let output = engine.process_decoded_image(&img).map_err(|e| crate::XbergError::Ocr {
                message: format!("TrOCR inference failed: {}", e),
                source: Some(Box::new(e)),
            })?;

            Ok::<_, crate::XbergError>((output.content, image_bytes))
        })
        .await
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("TrOCR task execution failed: {}", e),
            source: None,
        })??;

        Ok(super::ocr_result::build_ocr_document(
            content,
            Vec::new(),
            &image_bytes,
            config,
            super::ocr_result::OcrDocumentContext {
                mime_type: Cow::Borrowed("text/plain"),
                backend_name: "candle-trocr",
                // TrOCR has no task selection; every call is plain-text OCR. ~keep
                plain_text_task: true,
            },
        ))
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let bytes = super::read_image_file(path, &limits).await?;
        self.process_image_owned(Arc::new(bytes), config).await
    }

    fn supports_language(&self, lang: &str) -> bool {
        matches!(lang, "eng" | "en")
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["eng".to_string(), "en".to_string()]
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Candle
    }

    /// TrOCR reports no page-level confidence.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::None
    }

    // Rotation handling has not been measured for this backend; it stays on the trait's
    // `RequiresUpright` default.
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
        let options = TrocrBackend::parse_options(&config).unwrap();
        assert_eq!(options.variant, None);
        assert_eq!(options.device, DevicePreference::Auto);
    }

    #[test]
    fn test_parse_options_custom_variant() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "variant": "large-printed"
            })),
            ..Default::default()
        };
        let options = TrocrBackend::parse_options(&config).unwrap();
        assert_eq!(options.variant, Some(TrocrVariant::LargePrinted));
    }

    #[test]
    fn test_parse_options_custom_device() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "device": "cpu"
            })),
            ..Default::default()
        };
        let options = TrocrBackend::parse_options(&config).unwrap();
        assert_eq!(options.device, DevicePreference::Cpu);
    }

    #[test]
    fn test_parse_options_hf_cache_and_revision() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "cache_dir": "/tmp/trocr-cache",
                "hf_revision": "trocr-revision"
            })),
            ..Default::default()
        };
        let options = TrocrBackend::parse_options(&config).unwrap();
        assert_eq!(options.cache_dir.as_deref(), Some(Path::new("/tmp/trocr-cache")));
        assert_eq!(options.hf_revision.as_deref(), Some("trocr-revision"));
    }

    #[test]
    fn test_initialize_and_shutdown() {
        let backend = TrocrBackend::default_variant();
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    #[test]
    fn test_engine_pool_key_mapping() {
        let base_printed = variant_discriminant(TrocrVariant::BasePrinted);
        let large_printed = variant_discriminant(TrocrVariant::LargePrinted);
        let base_handwritten = variant_discriminant(TrocrVariant::BaseHandwritten);
        let large_handwritten = variant_discriminant(TrocrVariant::LargeHandwritten);

        assert_eq!(base_printed, 0);
        assert_eq!(large_printed, 1);
        assert_eq!(base_handwritten, 2);
        assert_eq!(large_handwritten, 3);

        let discriminants = [base_printed, large_printed, base_handwritten, large_handwritten];
        for (i, &d1) in discriminants.iter().enumerate() {
            for (j, &d2) in discriminants.iter().enumerate() {
                if i != j {
                    assert_ne!(d1, d2, "Discriminants for variants {} and {} must be unique", i, j);
                }
            }
        }
    }

    #[tokio::test]
    async fn configured_raster_budget_is_checked_before_truncated_pixels() {
        let bmp = crate::extraction::image_decode::bmp_with_declared_dimensions(32, 32);
        let backend = TrocrBackend::new(TrocrVariant::BasePrinted);
        let limit = 1024;
        assert!(bmp.len() < limit);
        let config = OcrConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: limit,
                ..Default::default()
            }),
            ..Default::default()
        };
        let error = backend
            .process_image(&bmp, &config)
            .await
            .expect_err("decoded raster exceeds the budget");
        assert!(
            matches!(&error, crate::XbergError::Validation { message, .. }
            if message.contains("security_limits.max_content_size")),
            "{error}"
        );

        let error = backend
            .process_image(&bmp, &OcrConfig::default())
            .await
            .expect_err("the admitted raster has truncated pixels");
        assert!(
            matches!(&error, crate::XbergError::Ocr { message, source: Some(_) }
            if message.starts_with("TrOCR image decode failed:")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn configured_encoded_budget_rejects_a_file_before_decoding() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), [0; 64]).unwrap();
        let config = OcrConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: 32,
                ..Default::default()
            }),
            ..Default::default()
        };
        let error = TrocrBackend::new(TrocrVariant::BasePrinted)
            .process_image_file(temp.path(), &config)
            .await
            .expect_err("encoded file exceeds the budget");
        assert!(
            matches!(&error, crate::XbergError::Validation { message, .. }
            if message.contains("encoded image file exceeds security_limits.max_content_size")),
            "{error}"
        );
    }

    fn fixture_bytes(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../test_documents/images/{name}"));
        std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()))
    }

    /// A whole scanned page (595x842, an A4-shaped raster) must be rejected before TrOCR
    /// wastes a forward pass hallucinating line-shaped text on it.
    ///
    /// This regression-guards the fix for the bug measured against
    /// `test_documents/pdf_scanned/ordinance_2197_scanned.pdf`: `candle-trocr` exited 0 and
    /// emitted 145 bytes of receipt-shaped hallucination (`AMOUNT NAME`, `CASHIER`, ...) for a
    /// 16-page zoning ordinance, because nothing rejected the full-page raster before it was
    /// force-resized to 384x384 and decoded anyway. Before the fix (no `reject_whole_page_input`
    /// call in `process_image`), this assertion fails: `is_err()` is `false` because no check
    /// exists to reject a full-page input.
    #[test]
    fn test_reject_whole_page_input_rejects_full_page_raster() {
        let bytes = fixture_bytes("ocr_test_original.png");
        let dimensions = xberg_candle_ocr::models::image_processor::dimensions(&bytes).unwrap();
        let result = reject_whole_page_input(dimensions);
        assert!(
            result.is_err(),
            "a 595x842 full-page raster must be rejected as line-level TrOCR input, got Ok"
        );
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("full page"),
            "error message must explain the mismatch, got: {message}"
        );
    }

    /// A genuine single-line crop (800x200, wide and short) must be accepted.
    #[test]
    fn test_reject_whole_page_input_accepts_line_crop() {
        let bytes = fixture_bytes("test_hello_world.png");
        let dimensions = xberg_candle_ocr::models::image_processor::dimensions(&bytes).unwrap();
        let result = reject_whole_page_input(dimensions);
        assert!(
            result.is_ok(),
            "an 800x200 single-line crop must be accepted, got {:?}",
            result.err().map(|e| e.to_string())
        );
    }
}
