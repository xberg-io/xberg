//! `--ocr*` and `--vlm-*` overrides.

#[cfg(feature = "ocr-surface")]
use anyhow::Context as _;
#[cfg(feature = "ocr-surface")]
use anyhow::{Result, bail};
#[cfg(feature = "ocr-surface")]
use xberg::{ExtractionConfig, LlmConfig, OcrConfig};

use super::ExtractionOverrides;

/// Accepted values for `--ocr-backend`.
#[cfg(feature = "ocr-surface")]
const VALID_OCR_BACKENDS: &[&str] = &[
    "tesseract",
    "paddle-ocr",
    "sceptre",
    "vlm",
    "candle-trocr",
    "candle-paddleocr-vl",
    "candle-glm-ocr",
    "candle-deepseek-ocr",
];

/// Language code used when neither the config nor `--ocr-language` names one.
#[cfg(feature = "ocr-surface")]
pub(super) const DEFAULT_OCR_LANGUAGE: &str = "eng";

/// Language code the PaddleOCR-family backends expect instead of [`DEFAULT_OCR_LANGUAGE`].
#[cfg(feature = "ocr-surface")]
pub(super) const DEFAULT_PADDLE_OCR_LANGUAGE: &str = "en";

/// Backends that use short ISO 639-1 style language codes rather than ISO 639-3.
///
/// `candle-deepseek-ocr` belongs here with the other two candle VLM backends: its
/// `supported_languages()` body is byte-identical to theirs
/// (`crates/xberg/src/candle_ocr/{deepseek,glm}_ocr_backend.rs`), accepting both the
/// ISO 639-3 and ISO 639-1 form of every language. Omitting it made the default reported
/// as `"eng"` where its two siblings report `"en"`, for no reason either backend can see:
/// none of the three consumes `config.language` for inference, so the difference was
/// purely in emitted metadata.
#[cfg(feature = "ocr-surface")]
const PADDLE_LANGUAGE_BACKENDS: &[&str] = &[
    "paddle-ocr",
    "candle-paddleocr-vl",
    "candle-glm-ocr",
    "candle-deepseek-ocr",
];

impl ExtractionOverrides {
    /// Reject invalid or contradictory `--ocr*`/`--vlm-*` flag combinations.
    #[cfg(feature = "ocr-surface")]
    pub(super) fn validate_ocr(&self) -> Result<()> {
        if self.ocr == Some(false) && self.ocr_scanned_pages {
            bail!("--ocr false cannot be combined with --ocr-scanned-pages");
        }
        if self.ocr == Some(false) && self.force_ocr == Some(true) {
            bail!("--ocr false cannot be combined with --force-ocr true");
        }
        if let (Some(ocr), Some(disable_ocr)) = (self.ocr, self.disable_ocr)
            && ocr == disable_ocr
        {
            bail!("--ocr and --disable-ocr specify contradictory values");
        }
        if self.ocr_scanned_pages && self.disable_ocr == Some(true) {
            bail!("--ocr-scanned-pages cannot be combined with --disable-ocr");
        }
        if let Some(confidence) = self.scanned_min_confidence
            && !(0.0..=1.0).contains(&confidence)
        {
            bail!("Invalid scan confidence: {confidence}. Value must be between 0.0 and 1.0.");
        }
        if self.force_ocr == Some(true) && self.disable_ocr == Some(true) {
            bail!("--force-ocr and --disable-ocr cannot both be true");
        }

        if let Some(ref backend) = self.ocr_backend
            && !VALID_OCR_BACKENDS.contains(&backend.as_str())
        {
            bail!(
                "Invalid OCR backend '{}'. Valid backends: {}",
                backend,
                VALID_OCR_BACKENDS.join(", ")
            );
        }

        self.parsed_backend_options()?;

        self.validate_vlm_model_required()?;
        Ok(())
    }

    /// Reject `--vlm-api-key`, `--vlm-prompt`, or `--ocr-backend vlm` given without
    /// `--vlm-model`. Split out of `validate_ocr` to keep it under the cyclomatic
    /// complexity limit; the three checks always ran as the tail of that function.
    #[cfg(feature = "ocr-surface")]
    fn validate_vlm_model_required(&self) -> Result<()> {
        if self.vlm_api_key.is_some() && self.vlm_model.is_none() {
            bail!("--vlm-api-key requires --vlm-model to be specified");
        }
        if self.vlm_prompt.is_some() && self.vlm_model.is_none() {
            bail!("--vlm-prompt requires --vlm-model to be specified");
        }
        if self.ocr_backend.as_deref() == Some("vlm") && self.vlm_model.is_none() {
            bail!("--ocr-backend vlm requires --vlm-model to be specified");
        }
        Ok(())
    }

    /// Apply the `--ocr*` flags onto `config.ocr`.
    ///
    /// Each flag mutates only the field it names. Fields with no CLI flag
    /// (`quality_thresholds`, `pipeline`, `tesseract_config`, `vlm_config`, …)
    /// keep whatever the config file or `--config-json` set for them.
    ///
    /// Naming any `--ocr-*` field flag (`--ocr-backend`, `--ocr-backend-options`,
    /// `--ocr-auto-rotate`, `--ocr-language`) is on its own enough to materialise
    /// `config.ocr` when it is still `None`, matching the `has_*_flag` pattern used
    /// by the other `apply_*` methods below. This matters because the flags that
    /// make OCR actually run (`--force-ocr`, `--ocr-scanned-pages`) do not require
    /// `config.ocr` to exist first: without this, a named backend/option/language
    /// was silently discarded while OCR still ran, using whatever backend the
    /// default config carries instead of the one requested.
    #[cfg(feature = "ocr-surface")]
    pub(super) fn apply_ocr(&self, config: &mut ExtractionConfig) {
        if self.ocr == Some(false) {
            config.ocr = None;
            config.disable_ocr = true;
            config.force_ocr = false;
            config.ocr_strategy = xberg::OcrStrategy::Auto;
            config.force_ocr_pages = None;
        } else {
            if self.ocr == Some(true) {
                config.ocr.get_or_insert_with(OcrConfig::default).enabled = true;
                config.disable_ocr = false;
            } else if self.has_ocr_field_flag() {
                config.ocr.get_or_insert_with(OcrConfig::default);
            }
            if let Some(ocr) = config.ocr.as_mut() {
                self.apply_ocr_fields(ocr);
            }
        }

        if self.ocr != Some(false)
            && let Some(force_ocr_flag) = self.force_ocr
        {
            config.force_ocr = force_ocr_flag;
        }
        if self.ocr.is_none()
            && let Some(disable_ocr_flag) = self.disable_ocr
        {
            config.disable_ocr = disable_ocr_flag;
        }
        if self.ocr != Some(false) && self.ocr_scanned_pages {
            config.ocr_strategy = xberg::OcrStrategy::ScannedPages {
                min_confidence: self
                    .scanned_min_confidence
                    .unwrap_or(xberg::core::config::DEFAULT_SCANNED_MIN_CONFIDENCE),
            };
            // The mixed OCR route (`extract_mixed_ocr_native`) needs per-page byte boundaries
            // to locate a detected scan within the native text and splice OCR output back in.
            // Boundaries are only tracked by the PDF backend when `config.pages` is
            // materialised (`pdf::native::text::extract_text_from_native_document`'s
            // `page_config` branch); without `--extract-pages`, `config.pages` stayed `None`,
            // so this route always hit its "no page boundaries available" fallback and
            // returned the native text untouched -- empty, for a scan, i.e. a zero-byte,
            // exit-0 result (#656). `PageConfig::default()` keeps `extract_pages: false`, so
            // this turns boundary tracking on without adding the `pages` array to the result
            // unless the caller separately asked for it.
            config.pages.get_or_insert_with(Default::default);
        }
    }

    /// Whether any `--ocr-*` field flag (backend, backend options, auto-rotate,
    /// language) was given, independent of `--ocr`/`--ocr true`.
    ///
    /// Deliberately excludes `--ocr-no-cache`: `apply_ocr_no_cache` only ever mutates an
    /// already-materialised `tesseract_config` (see its doc comment), so naming
    /// `--ocr-no-cache` alone has nothing to do once `config.ocr` exists, and must not be
    /// the thing that materialises `config.ocr` in the first place. Doing so would flip
    /// `config.ocr` from `None` to `Some(OcrConfig::default())` purely as a side effect of
    /// a caching flag, which can itself change behaviour downstream (e.g.
    /// `should_use_layout_ocr` in `crates/xberg/src/extractors/image.rs` branches on
    /// `config.ocr.is_some()`).
    #[cfg(feature = "ocr-surface")]
    fn has_ocr_field_flag(&self) -> bool {
        self.ocr_backend.is_some()
            || self.ocr_backend_options.is_some()
            || self.ocr_auto_rotate.is_some()
            || self.ocr_language.is_some()
    }

    /// Mutate the individual OCR fields that have a CLI flag, in place.
    ///
    /// Every assignment is guarded by the presence of its own flag, so sibling
    /// fields on `ocr` survive untouched.
    #[cfg(feature = "ocr-surface")]
    fn apply_ocr_fields(&self, ocr: &mut OcrConfig) {
        if let Some(ref backend) = self.ocr_backend {
            ocr.backend = backend.clone();
        }
        if let Some(options) = self.parsed_backend_options().ok().flatten() {
            ocr.backend_options = Some(options);
        }
        if let Some(rotate) = self.ocr_auto_rotate {
            ocr.auto_rotate = Some(rotate);
        }
        if let Some(no_cache) = self.ocr_no_cache {
            apply_ocr_no_cache(ocr, no_cache);
        }

        if let Some(ref language) = self.ocr_language {
            set_ocr_language(ocr, vec![language.clone()]);
            return;
        }

        // No `--ocr-language`. When the caller selected a backend (via `--ocr true` or
        // `--ocr-backend`) and the config still carries the untouched default language,
        // substitute the code that backend expects. An explicitly configured language is
        // never rewritten, and a run with no OCR flags at all is a no-op.
        let backend_selected = self.ocr == Some(true) || self.ocr_backend.is_some();
        let backend_default = default_language_for_backend(&ocr.backend);
        if backend_selected && backend_default != DEFAULT_OCR_LANGUAGE && is_default_ocr_language(&ocr.language) {
            ocr.language = vec![backend_default.to_string()];
        }
    }

    #[cfg(feature = "ocr-surface")]
    pub(super) fn apply_vlm_ocr(&self, config: &mut ExtractionConfig) {
        if let Some(ref vlm_model) = self.vlm_model {
            let vlm_llm_config = LlmConfig {
                model: vlm_model.clone(),
                api_key: self.vlm_api_key.clone(),
                ..Default::default()
            };

            let backend_options = self.parsed_backend_options().ok().flatten();
            let ocr = config.ocr.get_or_insert_with(|| OcrConfig {
                enabled: true,
                backend: "vlm".to_string(),
                language: vec!["eng".to_string()],
                tesseract_config: None,
                output_format: None,
                paddle_ocr_config: None,
                paddle_ocr_settings: None,
                element_config: None,
                quality_thresholds: None,
                pipeline: None,
                auto_rotate: None,
                vlm_config: None,
                vlm_fallback: Default::default(),
                vlm_prompt: None,
                acceleration: None,
                security_limits: None,
                tessdata_bytes: None,
                tessdata_path: None,
                numeric_repair: false,
                backend_options,
            });

            ocr.backend = "vlm".to_string();
            ocr.vlm_config = Some(vlm_llm_config);

            if let Some(ref prompt) = self.vlm_prompt {
                ocr.vlm_prompt = Some(prompt.clone());
            }
        }
    }

    /// Parse `--ocr-backend-options` into a `serde_json::Value`, enforcing that it is a JSON object.
    #[cfg(feature = "ocr-surface")]
    fn parsed_backend_options(&self) -> Result<Option<serde_json::Value>> {
        let Some(ref s) = self.ocr_backend_options else {
            return Ok(None);
        };
        let value: serde_json::Value =
            serde_json::from_str(s).with_context(|| format!("invalid --ocr-backend-options JSON: {s}"))?;
        if !value.is_object() {
            bail!("--ocr-backend-options must be a JSON object");
        }
        Ok(Some(value))
    }
}

/// The default OCR language code for `backend`.
#[cfg(feature = "ocr-surface")]
pub(super) fn default_language_for_backend(backend: &str) -> &'static str {
    if PADDLE_LANGUAGE_BACKENDS.contains(&backend) {
        DEFAULT_PADDLE_OCR_LANGUAGE
    } else {
        DEFAULT_OCR_LANGUAGE
    }
}

/// Whether `language` is still the untouched compiled-in default.
#[cfg(feature = "ocr-surface")]
fn is_default_ocr_language(language: &[String]) -> bool {
    matches!(language, [only] if only == DEFAULT_OCR_LANGUAGE)
}

/// Force the Tesseract OCR result cache on or off for this run, without perturbing any
/// other Tesseract setting.
///
/// `TesseractConfig::use_cache` (`xberg::TesseractConfig`) already exists and already
/// gates the OCR cache end to end (`process_image_resolved` in
/// `crates/xberg/src/ocr/processor/execution.rs`) — the CLI simply had no flag wired
/// to it before `--ocr-no-cache`.
///
/// This only mutates an *already-materialised* `tesseract_config` (e.g. one set by a
/// loaded config file). It deliberately does **not** call `get_or_insert_with` to create
/// one when `ocr.tesseract_config` is still `None` (as a first version of this function
/// did — see #693): `tesseract_config.is_none()` is a load-bearing sentinel downstream.
/// `crates/xberg/src/extractors/image.rs::apply_default_tesseract_psm` (and its siblings
/// `is_implicit_horizontal_tesseract`, `should_retry_sparse_image_ocr`) only install their
/// own whole-image/vertical/sparse-retry PSM and element defaults when `tesseract_config`
/// is still `None` by the time OCR runs; once it is `Some(..)`, those call sites treat it
/// as "the caller already made an explicit choice" and skip their own defaulting,
/// leaving `TesseractConfig::default()`'s `psm = 3` in place instead of e.g.
/// `WHOLE_IMAGE_TESSERACT_PSM = 11`. Measured effect on a real scan: 217 recognised words
/// without the flag vs. 194 with it, from a changed PSM alone — a caching flag must never
/// change what Tesseract recognises.
///
/// Closing this properly for the "no `tesseract_config` yet" case needs a cache-bypass
/// signal that lives outside `TesseractConfig`'s `Option` sentinel — for example a new
/// `OcrConfig::bypass_ocr_cache: bool` field (in `crates/xberg/src/core/config/ocr.rs`)
/// that `TesseractBackend::config_to_tesseract`
/// (`crates/xberg/src/ocr/tesseract_backend.rs`) ORs into the internal
/// `TesseractConfig::use_cache` it builds, independent of whether `tesseract_config` was
/// supplied. That change is outside this file's scope; until it lands, this flag is a
/// no-op (with a warning) unless `tesseract_config` is already set.
#[cfg(feature = "ocr-surface")]
fn apply_ocr_no_cache(ocr: &mut OcrConfig, no_cache: bool) {
    let Some(tesseract_config) = ocr.tesseract_config.as_mut() else {
        tracing::warn!(
            "--ocr-no-cache has no effect: no `tesseract_config` is set yet (e.g. via a \
             config file's `ocr.tesseract_config`). Materialising one just to carry \
             `use_cache: false` would also silently change Tesseract's PSM and other \
             defaults for this run (see issue #693), so this flag is a no-op here instead \
             of risking that. Clear the on-disk OCR cache directory instead, or set \
             `ocr.tesseract_config` explicitly before using --ocr-no-cache."
        );
        return;
    };
    tesseract_config.use_cache = !no_cache;
}

/// Set `language` on an OCR config and on every nested Tesseract-flavoured
/// config that carries its own copy of it.
#[cfg(feature = "ocr-surface")]
fn set_ocr_language(ocr: &mut OcrConfig, language: Vec<String>) {
    ocr.language = language.clone();
    if let Some(tesseract_config) = ocr.tesseract_config.as_mut() {
        tesseract_config.language = language.clone();
    }
    if let Some(pipeline) = ocr.pipeline.as_mut() {
        for stage in &mut pipeline.stages {
            if stage.backend != "tesseract" {
                continue;
            }
            stage.language = Some(language.clone());
            if let Some(tesseract_config) = stage.tesseract_config.as_mut() {
                tesseract_config.language = language.clone();
            }
        }
    }
}
