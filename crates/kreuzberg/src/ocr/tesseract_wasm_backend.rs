//! WASM-compatible Tesseract OCR backend.
//!
//! Drives the Tesseract+Leptonica WASI build (provided by `kreuzberg-tesseract`'s
//! `build-tesseract-wasm` feature) via in-memory tessdata, so OCR works on
//! `wasm32-unknown-unknown` with no filesystem and no JavaScript dependencies.
//!
//! Tessdata bytes can come from two sources, in priority order:
//! 1. `OcrConfig::tessdata_bytes` — caller-supplied per-language map.
//! 2. The `bundle-tessdata-eng` feature on `kreuzberg-tesseract`, which embeds
//!    the English `eng.traineddata` (~4 MB, tessdata_fast) into the WASM
//!    binary at compile time.
//!
//! Without either, this backend returns a `MissingDependency` error explaining
//! how to provide tessdata.

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractionResult, FormatMetadata, Metadata, OcrMetadata};
use async_trait::async_trait;
use kreuzberg_tesseract::{Pix, TesseractAPI};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

/// Default OCR engine mode: LSTM only (mode 1). Matches the `OEM_LSTM_ONLY`
/// constant from Tesseract's `tesseract/publictypes.h`. LSTM is the only
/// recognition engine compiled into our WASI Tesseract build.
const OEM_LSTM_ONLY: i32 = 1;

/// WASM-compatible Tesseract OCR backend.
pub struct TesseractWasmBackend {
    /// Process-local tessdata cache, keyed by language code.
    tessdata_cache: Mutex<HashMap<String, Vec<u8>>>,
}

impl TesseractWasmBackend {
    /// Create a new Tesseract WASM backend.
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            tessdata_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Resolve tessdata bytes for a language, consulting the cache, the
    /// supplied OcrConfig, and the optional bundled-eng compile-time blob.
    fn resolve_tessdata(&self, language: &str, config: &OcrConfig) -> Result<Vec<u8>> {
        if let Ok(cache) = self.tessdata_cache.lock()
            && let Some(cached) = cache.get(language)
        {
            return Ok(cached.clone());
        }

        if let Some(ref user_supplied) = config.tessdata_bytes
            && let Some(bytes) = user_supplied.get(language)
        {
            self.cache_tessdata(language, bytes.clone());
            return Ok(bytes.clone());
        }

        if language == "eng"
            && let Some(bundled) = bundled_eng_traineddata()
        {
            self.cache_tessdata(language, bundled.to_vec());
            return Ok(bundled.to_vec());
        }

        Err(crate::KreuzbergError::MissingDependency(format!(
            "Tesseract tessdata for language '{language}' not available on WASM. \
             Provide bytes via OcrConfig::tessdata_bytes, or build with the \
             'bundle-tessdata-eng' feature for English."
        )))
    }

    fn cache_tessdata(&self, language: &str, bytes: Vec<u8>) {
        if let Ok(mut cache) = self.tessdata_cache.lock() {
            cache.insert(language.to_string(), bytes);
        }
    }
}

impl Plugin for TesseractWasmBackend {
    fn name(&self) -> &str {
        "tesseract"
    }

    fn version(&self) -> String {
        TesseractAPI::version()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl OcrBackend for TesseractWasmBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        if image_bytes.is_empty() {
            return Err(crate::KreuzbergError::Validation {
                message: "OCR input image is empty".to_string(),
                source: None,
            });
        }

        let language = if config.language.is_empty() { "eng".to_string() } else { config.language.clone() };
        let tessdata = self.resolve_tessdata(&language, config)?;

        let img = image::load_from_memory(image_bytes).map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Failed to decode image for OCR: {e}"),
            source: Some(Box::new(e)),
        })?;
        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();
        let pix = Pix::from_raw_rgb(rgb.as_raw(), width, height).map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Failed to create Leptonica Pix from image: {e}"),
            source: Some(Box::new(e)),
        })?;

        let api = TesseractAPI::new().map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Failed to create Tesseract API handle: {e}"),
            source: Some(Box::new(e)),
        })?;

        api.init_5(&tessdata, tessdata.len() as i32, &language, OEM_LSTM_ONLY, &[])
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("Failed to init Tesseract with bundled tessdata: {e}"),
                source: Some(Box::new(e)),
            })?;

        api.set_image_2(pix.as_ptr()).map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Failed to set image on Tesseract API: {e}"),
            source: Some(Box::new(e)),
        })?;
        api.recognize().map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Tesseract recognition failed: {e}"),
            source: Some(Box::new(e)),
        })?;
        let text = api.get_utf8_text().map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Failed to read Tesseract text output: {e}"),
            source: Some(Box::new(e)),
        })?;

        let metadata = Metadata {
            format: Some(FormatMetadata::Ocr(OcrMetadata {
                language: language.clone(),
                psm: config.tesseract_config.as_ref().map(|c| c.psm).unwrap_or(3),
                output_format: "text".to_string(),
                table_count: 0,
                table_rows: None,
                table_cols: None,
            })),
            ..Default::default()
        };

        Ok(ExtractionResult {
            content: text,
            mime_type: Cow::Borrowed("text/plain"),
            metadata,
            ..Default::default()
        })
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let bytes = std::fs::read(path).map_err(crate::KreuzbergError::Io)?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Tesseract
    }
}

/// Returns the compile-time-bundled English tessdata when the
/// `kreuzberg-tesseract/bundle-tessdata-eng` feature is on, otherwise `None`.
fn bundled_eng_traineddata() -> Option<&'static [u8]> {
    kreuzberg_tesseract::bundled_eng_traineddata()
}
