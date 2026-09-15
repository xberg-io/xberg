//! OCR backend plugin trait.
//!
//! This module defines the trait for implementing custom OCR backends.

use crate::core::config::OcrConfig;
use crate::plugins::Plugin;
use crate::types::ExtractedDocument;
use crate::{Result, XbergError};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// OCR backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub enum OcrBackendType {
    /// Tesseract OCR (native Rust binding)
    #[default]
    Tesseract,
    /// PaddleOCR (Python-based, via FFI)
    PaddleOCR,
    /// Candle-based VLM OCR (TrOCR, PaddleOCR-VL).
    Candle,
    /// Name-selected built-in or third-party OCR backend.
    Custom,
}

/// How a backend's reported page-level confidence must be interpreted.
///
/// Backend confidence scores are not interchangeable. Tesseract's mean word confidence is a
/// classifier score validated to track legibility on a 0-100 scale. Sceptre (EasyOCR-based)
/// reports a length-penalised `custom_mean` that is rescaled into the same 0-100 range but is
/// *not* comparable — its ordering can be inverted relative to legibility (a dense prose page
/// can score lower than a nearly-blank one). A page-rejection gate calibrated on Tesseract's
/// scale was once applied unconditionally to sceptre's output and rejected every page of a
/// 16-page document, emptying it. This descriptor exists so gating code can ask a backend what
/// its number means instead of assuming.
// `Default` is `Uncalibrated`, matching `OcrBackend::confidence_semantics`'s trait default
// exactly. It reinforces that invariant rather than competing with it: the one value it is
// never safe to fall back to is `Legibility`, which would let an undeclared backend inherit
// Tesseract's gate threshold. `serde` and `Default` are required because the `Legibility`
// variant carries a payload. The payload is NOT what forces the JSON marshalling, though: the
// generated bridge marshals every trait method's return value that way, which is why the
// unit-only `PageOrientationHandling` below needs exactly the same derives. ~keep
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Deserialize, serde::Serialize)]
pub enum ConfidenceSemantics {
    /// Validated to track legibility on a known scale — usable as an absolute quality gate.
    Legibility {
        /// The upper bound of the reported confidence scale (e.g. `100.0` for Tesseract).
        scale_max: f64,
    },
    /// A number is reported, but it is not validated to correlate with legibility.
    /// Never gate on it.
    #[default]
    Uncalibrated,
    /// No page-level confidence is reported at all.
    None,
}

/// How a backend copes with a page raster whose text is not upright.
///
/// Rotated-page handling is a backend capability, not a universal guarantee. An A/B run this
/// session against `/Rotate 270` scanned pages showed the three handled cases genuinely differ:
/// Tesseract reconstructs correct reading order on a sideways raster outright; PaddleOCR
/// recognises the rotated text correctly (it warps each detected quad upright before running
/// recognition) but leaves its block list in raw raster `(y, x)` order, so the caller must
/// reorder; sceptre produces character garbage on the same sideways raster and only reads
/// correctly once the page is rendered upright first. A caller that skips an upright-render step
/// for a backend that actually needs one gets silent garbage, not an error.
///
/// # Only one variant is discriminated (#657)
///
/// Read this before "simplifying" the type. There is exactly one decision point in the
/// codebase that inspects this value: `upright_raster_for_backend`
/// (`crate::extractors::pdf::ocr`), which tests `orientation_handling != RequiresUpright` and
/// otherwise does nothing. Every other mention forwards the value to that test. So, *to that
/// codepath*, `SelfCorrecting` and `RecognisesRotatedText` are behaviourally identical — the
/// enum is a boolean at the point of use, and the three variants describe measured backend
/// behaviour rather than three dispatch paths.
///
/// `RecognisesRotatedText`'s actual remedy is not this enum. The block-order fix is the
/// `backend_options["page_rotation_degrees"]` hint injected by
/// `ocr_config_with_page_rotation_hint` (`crate::extractors::pdf::ocr`) **unconditionally, for
/// every backend**, which `PaddleOcrBackend::process_image` reads back
/// (`page_rotation_degrees_from_backend_options` -> `residual_rotation_for_reorder` ->
/// `reorder_blocks_for_page_rotation`, `crate::paddle_ocr::backend`) and applies internally.
/// Declaring `RecognisesRotatedText` therefore changes nothing on its own; a backend in that
/// class must also read the hint. Conversely, gating that hint on this enum would remove a
/// field from Tesseract's `OcrConfig` and hence from the OCR cache key
/// (`hash(image + language + config)`), invalidating every cached page — do not do it without
/// its own A/B.
///
/// # PDF-route-only
///
/// Only the PDF OCR routes call `OcrBackend::page_orientation_handling` (the `--force-ocr`
/// route via `extract_with_ocr` and the scanned-pages route via `extract_mixed_ocr_native`).
/// The raw-image route (`crate::extractors::image`) never calls it: there is no `/Rotate` to
/// consult, and orientation there is handled by the PP-LCNet document-orientation classifier
/// (`crate::doc_orientation`) gated on `OcrConfig::auto_rotate`.
///
/// # Cost of the default
///
/// The trait default is `RequiresUpright` (deliberately the least capable option, see
/// [`OcrBackend::page_orientation_handling`]). A backend that does not declare therefore pays,
/// on every page with `/Rotate != 0`, a re-encode plus rotation of the raster in
/// `upright_raster_for_backend` and a bounding-box round-trip back through
/// `undo_upright_raster_correction`. That is the safe direction to be wrong in, but it is not
/// free, and for a backend that never got measured it is not known to be necessary either.
// `Default` and `serde` are required by the generated FFI bridge, which marshals the return value
// of EVERY `OcrBackend` trait method through JSON -- see `XbergOcrBackendBridge` in
// `crates/xberg-ffi/src/lib.rs`, where `page_orientation_handling` does
// `serde_json::from_str::<PageOrientationHandling>(&json)` and falls back to `Default::default()`
// on an uninitialised vtable slot, a failing host callback, or a null result. Whether a variant
// carries a payload makes no difference to that. `RequiresUpright` is the default so it matches
// `OcrBackend::page_orientation_handling`'s trait default exactly. ~keep
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub enum PageOrientationHandling {
    /// Reconstructs reading order regardless of page rotation — safe to hand a
    /// raster in any orientation.
    SelfCorrecting,
    /// Recognises rotated text correctly but emits blocks in raw raster order,
    /// so the caller must reorder.
    RecognisesRotatedText,
    /// Requires an upright raster; rotated text produces garbage.
    #[default]
    RequiresUpright,
}

/// Trait for OCR backend plugins.
///
/// Implement this trait to add custom OCR capabilities. OCR backends can be:
/// - Native Rust implementations (like Tesseract)
/// - FFI bridges to external libraries (like PaddleOCR)
/// - Cloud-based OCR services (Google Vision, AWS Textract, etc.)
///
/// # Thread Safety
///
/// OCR backends must be thread-safe (`Send + Sync`) to support concurrent processing.
///
/// # Example
///
/// ```rust
/// use xberg::plugins::{Plugin, OcrBackend, OcrBackendType};
/// use xberg::{Result, OcrConfig};
/// use async_trait::async_trait;
/// use std::borrow::Cow;
/// use std::path::Path;
/// use xberg::types::{ExtractedDocument, Metadata};
///
/// struct CustomOcrBackend;
///
/// impl Plugin for CustomOcrBackend {
///     fn name(&self) -> &str { "custom-ocr" }
///     fn version(&self) -> String { "1.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl OcrBackend for CustomOcrBackend {
///     async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
///         // Implement OCR logic here
///         let mut document = ExtractedDocument::default();
///         document.content = "Extracted text".to_string();
///         document.mime_type = Cow::Borrowed("text/plain");
///         Ok(document)
///     }
///
///     async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
///         let bytes = std::fs::read(path)?;
///         self.process_image(&bytes, config).await
///     }
///
///     fn supports_language(&self, lang: &str) -> bool {
///         matches!(lang, "eng" | "deu" | "fra")
///     }
///
///     fn backend_type(&self) -> OcrBackendType {
///         OcrBackendType::Custom
///     }
/// }
/// ```
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait OcrBackend: Plugin {
    /// Process an image and extract text via OCR.
    ///
    /// # Arguments
    ///
    /// * `image_bytes` - Raw image data (JPEG, PNG, TIFF, etc.)
    /// * `config` - OCR configuration (language, PSM mode, etc.)
    ///
    /// # Returns
    ///
    /// An `ExtractedDocument` containing the extracted text and metadata.
    ///
    /// # Errors
    ///
    /// - `XbergError::Ocr` - OCR processing failed
    /// - `XbergError::Validation` - Invalid image format or configuration
    /// - `XbergError::Io` - I/O errors (these always bubble up)
    ///
    /// # Reading `backend_options`
    ///
    /// Backends that support runtime tuning can read `config.backend_options` and
    /// deserialize only the keys they care about. Unknown keys are silently ignored,
    /// so multiple backends can coexist in a pipeline without key conflicts.
    ///
    /// ```rust
    /// # use xberg::plugins::{Plugin, OcrBackend};
    /// # use xberg::{Result, OcrConfig};
    /// # use async_trait::async_trait;
    /// # use std::borrow::Cow;
    /// # use std::path::Path;
    /// # use xberg::types::{ExtractedDocument, Metadata};
    /// # struct MyOcr;
    /// # impl Plugin for MyOcr {
    /// #     fn name(&self) -> &str { "my-ocr" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use xberg::plugins::OcrBackendType;
    /// # #[async_trait]
    /// # impl OcrBackend for MyOcr {
    /// #     fn supports_language(&self, _: &str) -> bool { true }
    /// #     fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
    /// #     async fn process_image_file(&self, _: &Path, _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
    ///     // Read backend-specific options; unknown keys are silently ignored.
    ///     let fast_mode = config.backend_options
    ///         .as_ref()
    ///         .and_then(|v| v.get("mode"))
    ///         .and_then(|v| v.as_str())
    ///         .map(|s| s == "fast")
    ///         .unwrap_or(false);
    ///
    ///     if image_bytes.is_empty() {
    ///         return Err(xberg::XbergError::Validation {
    ///             message: "Empty image data".to_string(),
    ///             source: None,
    ///         });
    ///     }
    ///
    ///     let text = if fast_mode {
    ///         "Fast OCR result".to_string()
    ///     } else {
    ///         format!("Extracted text in language: {:?}", config.language)
    ///     };
    ///
    ///     let mut document = ExtractedDocument::default();
    ///     document.content = text;
    ///     document.mime_type = Cow::Borrowed("text/plain");
    ///     Ok(document)
    /// }
    /// # }
    /// ```
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument>;

    /// Process an owned image buffer and extract text via OCR.
    ///
    /// The default implementation delegates to [`Self::process_image`]. Backends
    /// that hand work to an owned blocking task can override this method to avoid
    /// copying the image buffer.
    ///
    /// Excluded from the polyglot binding surface: it is an owned-buffer perf
    /// override whose `Arc<Vec<u8>>` parameter has no binding representation, and
    /// foreign backends satisfy the trait through [`Self::process_image`] via this
    /// default delegation.
    #[cfg_attr(alef, alef(skip))]
    async fn process_image_owned(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        self.process_image(image_bytes.as_slice(), config).await
    }

    /// Process a file and extract text via OCR.
    ///
    /// Default implementation reads the file and calls `process_image`.
    /// Override for custom file handling or optimizations.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the image file
    /// * `config` - OCR configuration
    ///
    /// # Errors
    ///
    /// Same as `process_image`, plus file I/O errors.
    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        #[cfg(feature = "tokio-runtime")]
        {
            use crate::core::io;
            let bytes = io::read_file_async(path).await?;
            self.process_image(&bytes, config).await
        }
        #[cfg(not(feature = "tokio-runtime"))]
        {
            let _ = (path, config);
            Err(XbergError::Other(
                "File-based OCR processing requires the tokio-runtime feature".to_string(),
            ))
        }
    }

    /// Check if this backend supports a given language code.
    ///
    /// # Arguments
    ///
    /// * `lang` - ISO 639-2/3 language code (e.g., "eng", "deu", "fra")
    ///
    /// # Returns
    ///
    /// `true` if the language is supported, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use xberg::plugins::{Plugin, OcrBackend};
    /// # use xberg::Result;
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct MyOcr { languages: Vec<String> }
    /// # impl Plugin for MyOcr {
    /// #     fn name(&self) -> &str { "my-ocr" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use xberg::plugins::OcrBackendType;
    /// # use xberg::{ExtractedDocument, OcrConfig};
    /// # #[async_trait]
    /// # impl OcrBackend for MyOcr {
    /// #     fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
    /// #     async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// #     async fn process_image_file(&self, _: &Path, _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// fn supports_language(&self, lang: &str) -> bool {
    ///     self.languages.contains(&lang.to_string())
    /// }
    /// # }
    /// ```
    fn supports_language(&self, lang: &str) -> bool;

    /// Get the backend type identifier.
    ///
    /// # Returns
    ///
    /// The backend type enum value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use xberg::plugins::{Plugin, OcrBackend, OcrBackendType};
    /// # use xberg::Result;
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct TesseractBackend;
    /// # impl Plugin for TesseractBackend {
    /// #     fn name(&self) -> &str { "tesseract" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use xberg::{ExtractedDocument, OcrConfig};
    /// # #[async_trait]
    /// # impl OcrBackend for TesseractBackend {
    /// #     fn supports_language(&self, _: &str) -> bool { true }
    /// #     async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// #     async fn process_image_file(&self, _: &Path, _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// fn backend_type(&self) -> OcrBackendType {
    ///     OcrBackendType::Tesseract
    /// }
    /// # }
    /// ```
    fn backend_type(&self) -> OcrBackendType;

    /// Optional: Get a list of all supported languages.
    ///
    /// Defaults to empty list. Override to provide comprehensive language support info.
    fn supported_languages(&self) -> Vec<String> {
        vec![]
    }

    /// Optional: Check if the backend supports table detection.
    ///
    /// Defaults to `false`. Override if your backend can detect and extract tables.
    fn supports_table_detection(&self) -> bool {
        false
    }

    /// Check if the backend supports direct document-level processing (e.g. for PDFs).
    ///
    /// Defaults to `false`. Override if the backend has optimized document processing.
    /// PDF extraction uses this optimized path only when both effective page margins
    /// are zero; nonzero margins require per-page image processing so geometry can be
    /// filtered correctly.
    fn supports_document_processing(&self) -> bool {
        false
    }

    /// Declare that this backend emits structured markdown directly (tables, headings, lists)
    /// and downstream layout reconstruction should be skipped.
    ///
    /// Defaults to `false` — classical OCR backends (Tesseract, PaddleOCR classical) return
    /// plain text per detected region. End-to-end VLM backends (PaddleOCR-VL, GOT-OCR 2.0)
    /// emit markdown in one forward pass and should override this to `true`.
    fn emits_structured_markdown(&self) -> bool {
        false
    }

    /// Declare how this backend's reported page-level confidence must be interpreted.
    ///
    /// Defaults to [`ConfidenceSemantics::Uncalibrated`]. This default is deliberately the
    /// least trusting option, not [`ConfidenceSemantics::Legibility`]: a new backend that
    /// reports *some* confidence number is not thereby safe to gate on, and defaulting to
    /// `Legibility` would let the next backend silently inherit a threshold calibrated for a
    /// different backend's scale — exactly the failure this type exists to prevent (see the
    /// type's doc comment). Override this only after validating that the reported number
    /// tracks legibility on a known scale.
    fn confidence_semantics(&self) -> ConfidenceSemantics {
        ConfidenceSemantics::Uncalibrated
    }

    /// Declare how this backend copes with a page raster whose text is not upright.
    ///
    /// Defaults to [`PageOrientationHandling::RequiresUpright`]. This default is deliberately
    /// the least capable option, not [`PageOrientationHandling::SelfCorrecting`]: a new backend
    /// must not silently inherit Tesseract's ability to reconstruct reading order on a rotated
    /// raster and then quietly emit garbage the first time it faces one (see the type's doc
    /// comment for the measured A/B behind this). Override this only after validating the
    /// backend's actual behaviour on a rotated page.
    fn page_orientation_handling(&self) -> PageOrientationHandling {
        PageOrientationHandling::RequiresUpright
    }

    /// Process a document file directly via OCR.
    ///
    /// Only called if `supports_document_processing` returns `true`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the document file (e.g. .pdf)
    /// * `config` - OCR configuration
    async fn process_document(&self, _path: &Path, _config: &OcrConfig) -> Result<ExtractedDocument> {
        Err(crate::XbergError::Other(
            "Document-level OCR processing not supported by this backend".to_string(),
        ))
    }

    /// Optional: Probe whether this backend will actually execute on this host
    /// with the given configuration.
    ///
    /// Used by `doctor` diagnostics. Implementations must not download models
    /// or make billable API calls; anything not yet local is reported as
    /// `ProbeStatus::Skip`. Defaults to `Skip` so custom backends need no
    /// changes; implement it to give users real environment diagnostics.
    ///
    /// Excluded from the polyglot binding surface: doctor results are produced
    /// by the generated `doctor()` function, not per-backend calls.
    #[cfg_attr(alef, alef(skip))]
    fn probe(&self, _config: &OcrConfig) -> crate::doctor::DoctorCheck {
        crate::doctor::DoctorCheck::skip(self.name(), "no probe implemented for this backend")
    }
}

/// Runtime capabilities reported by a registered OCR backend.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrBackendCapability {
    /// The backend's registered name.
    name: String,
    /// The exhaustive supported language set, or `None` when the backend does not declare one.
    supported_languages: Option<Vec<String>>,
}

impl OcrBackendCapability {
    /// Return the backend's registered name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the exhaustive language set, or `None` when the backend does not declare one.
    pub fn supported_languages(&self) -> Option<&[String]> {
        self.supported_languages.as_deref()
    }
}

/// Register an OCR backend with the global registry.
///
/// The OCR backend will be registered with its name from the `name()` method
/// and can be used for OCR processing via the extraction pipeline.
///
/// # Arguments
///
/// * `backend` - The OCR backend implementation wrapped in Arc
///
/// # Returns
///
/// - `Ok(())` if registration succeeded
/// - `Err(...)` if validation failed or initialization failed
///
/// # Errors
///
/// - `XbergError::Validation` - Invalid backend name (empty or contains whitespace)
/// - Any error from the backend's `initialize()` method
///
/// # Example
///
/// ```rust
/// use xberg::plugins::{Plugin, OcrBackend, register_ocr_backend, OcrBackendType};
/// use xberg::{Result, OcrConfig};
/// use xberg::types::{ExtractedDocument, Metadata};
/// use async_trait::async_trait;
/// use std::borrow::Cow;
/// use std::sync::Arc;
/// use std::path::Path;
///
/// struct CustomOcr;
///
/// impl Plugin for CustomOcr {
///     fn name(&self) -> &str { "custom-ocr" }
///     fn version(&self) -> String { "1.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl OcrBackend for CustomOcr {
///     async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractedDocument> {
///         let mut document = ExtractedDocument::default();
///         document.content = "text".to_string();
///         document.mime_type = Cow::Borrowed("text/plain");
///         Ok(document)
///     }
///     fn supports_language(&self, _: &str) -> bool { true }
///     fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
/// }
///
/// # tokio_test::block_on(async {
/// let backend = Arc::new(CustomOcr);
/// register_ocr_backend(backend)?;
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn register_ocr_backend(backend: Arc<dyn OcrBackend>) -> crate::Result<()> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let mut registry = registry.write();

    registry.register(backend)
}

/// Unregister an OCR backend by name.
///
/// Removes the OCR backend from the global registry and calls its `shutdown()` method.
///
/// # Arguments
///
/// * `name` - Name of the OCR backend to unregister
///
/// # Returns
///
/// - `Ok(())` if the backend was unregistered or didn't exist
/// - `Err(...)` if the shutdown method failed
///
/// # Example
///
/// ```rust
/// use xberg::plugins::unregister_ocr_backend;
///
/// # tokio_test::block_on(async {
/// unregister_ocr_backend("custom-ocr")?;
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn unregister_ocr_backend(name: &str) -> crate::Result<()> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let mut registry = registry.write();

    registry.remove(name)
}

/// List all registered OCR backends.
///
/// Returns the names of all OCR backends currently registered in the global registry.
///
/// # Returns
///
/// A vector of OCR backend names.
///
/// # Example
///
/// ```rust
/// use xberg::plugins::list_ocr_backends;
///
/// # tokio_test::block_on(async {
/// let backends = list_ocr_backends()?;
/// for name in backends {
///     println!("Registered OCR backend: {}", name);
/// }
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
pub fn list_ocr_backends() -> crate::Result<Vec<String>> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let registry = registry.read();

    Ok(registry.list())
}

/// List the runtime capabilities of all registered OCR backends.
///
/// The result is sorted by backend name. A `None` language set means the backend does not provide
/// an exhaustive enumeration, so callers must not interpret it as supporting no languages.
#[cfg_attr(alef, alef(skip))]
pub fn list_ocr_backend_capabilities() -> crate::Result<Vec<OcrBackendCapability>> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let backends = get_ocr_backend_registry().read().registered_snapshot();
    Ok(backends
        .into_iter()
        .map(|(name, backend)| capability_from(name, backend))
        .collect())
}

/// Check whether a registered OCR backend supports one language code.
///
/// This avoids allocating a complete language manifest when a caller only needs to validate one
/// requested code. Backend callbacks run after the registry read lock is released.
#[cfg_attr(alef, alef(skip))]
pub fn ocr_backend_supports_language(backend_name: &str, language: &str) -> crate::Result<bool> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let backend = {
        let registry = get_ocr_backend_registry();
        registry.read().registered(backend_name)
    };
    let backend = backend.ok_or_else(|| XbergError::Plugin {
        message: format!("OCR backend '{backend_name}' not registered"),
        plugin_name: backend_name.to_string(),
    })?;
    Ok(backend.supports_language(language))
}

fn capability_from(name: String, backend: Arc<dyn OcrBackend>) -> OcrBackendCapability {
    let supported_languages = match backend.supported_languages() {
        languages if languages.is_empty() => None,
        languages => Some(languages),
    };
    OcrBackendCapability {
        name,
        supported_languages,
    }
}

/// Clear all OCR backends from the global registry.
///
/// Removes all OCR backends and calls their `shutdown()` methods.
///
/// # Returns
///
/// - `Ok(())` if all backends were cleared successfully
/// - `Err(...)` if any shutdown method failed
///
/// # Example
///
/// ```rust
/// use xberg::plugins::clear_ocr_backends;
///
/// # tokio_test::block_on(async {
/// clear_ocr_backends()?;
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
pub fn clear_ocr_backends() -> crate::Result<()> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let mut registry = registry.write();

    registry.shutdown_all()
}

/// Ensure the global OCR backend registry has its built-in backends registered.
///
/// The global registry is seeded with the built-in backends (Tesseract,
/// PaddleOCR, VLM — gated by feature flags) when it is first constructed.
/// However, [`clear_ocr_backends`] empties the registry, leaving subsequent
/// OCR operations with no backend to dispatch to.
///
/// This function is the self-healing counterpart, mirroring
/// `crate::extractors::ensure_initialized` for the document extractor registry:
/// it re-registers the built-in backends whenever the built-in default is
/// missing so that callers always see a usable registry. It re-seeds not only
/// when the registry is empty but also when it is non-empty yet missing the
/// built-in default (e.g. after [`clear_ocr_backends`] followed by registering
/// a *different* backend) — the plain "empty" check would leave default-config
/// OCR without a backend. Re-seeding is non-destructive (user-registered
/// backends are kept) and cheap to invoke before every OCR dispatch.
#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
pub(crate) fn ensure_ocr_backends_initialized() {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();

    {
        let registry = registry.read();
        if !registry.is_missing_default_backend() {
            return;
        }
    }

    registry.write().ensure_defaults();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    struct MockOcrBackend {
        languages: Vec<String>,
    }

    impl Plugin for MockOcrBackend {
        fn name(&self) -> &str {
            "mock-ocr"
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl OcrBackend for MockOcrBackend {
        async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> Result<ExtractedDocument> {
            Ok(ExtractedDocument {
                content: "Mocked OCR text".to_string(),
                mime_type: Cow::Borrowed("text/plain"),
                ..Default::default()
            })
        }

        fn supports_language(&self, lang: &str) -> bool {
            self.languages.iter().any(|l| l == lang)
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }

        fn supported_languages(&self) -> Vec<String> {
            self.languages.clone()
        }
    }

    #[tokio::test]
    async fn test_ocr_backend_process_image() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string(), "deu".to_string()],
        };

        let config = OcrConfig {
            backend: "mock".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        };

        let result = backend.process_image(b"fake image data", &config).await.unwrap();
        assert_eq!(result.content, "Mocked OCR text");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_ocr_backend_process_image_owned_default_impl_is_object_safe() {
        let backend: Arc<dyn OcrBackend> = Arc::new(MockOcrBackend {
            languages: vec!["eng".to_string()],
        });

        let result = backend
            .process_image_owned(Arc::new(b"fake image data".to_vec()), &OcrConfig::default())
            .await
            .unwrap();

        assert_eq!(result.content, "Mocked OCR text");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[test]
    fn test_ocr_backend_supports_language() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string(), "deu".to_string()],
        };

        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("deu"));
        assert!(!backend.supports_language("fra"));
    }

    #[test]
    fn test_ocr_backend_type() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        assert_eq!(backend.backend_type(), OcrBackendType::Custom);
    }

    #[test]
    fn test_ocr_backend_supported_languages() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string(), "deu".to_string(), "fra".to_string()],
        };

        let supported = backend.supported_languages();
        assert_eq!(supported.len(), 3);
        assert!(supported.contains(&"eng".to_string()));
        assert!(supported.contains(&"deu".to_string()));
        assert!(supported.contains(&"fra".to_string()));
    }

    #[test]
    fn backend_capability_reports_the_registered_backends_declared_languages() {
        let backend: Arc<dyn OcrBackend> = Arc::new(MockOcrBackend {
            languages: vec!["eng".to_string(), "deu".to_string()],
        });

        let capability = capability_from("mock-ocr".to_string(), backend);
        assert_eq!(capability.name(), "mock-ocr");
        assert_eq!(
            capability.supported_languages(),
            Some(["eng".to_string(), "deu".to_string()].as_slice())
        );
    }

    #[test]
    fn backend_capability_distinguishes_an_unspecified_language_set() {
        let backend: Arc<dyn OcrBackend> = Arc::new(MockOcrBackend { languages: Vec::new() });

        let capability = capability_from("mock-ocr".to_string(), backend);
        assert_eq!(capability.name(), "mock-ocr");
        assert_eq!(capability.supported_languages(), None);
    }

    #[test]
    fn test_ocr_backend_type_variants() {
        assert_eq!(OcrBackendType::Tesseract, OcrBackendType::Tesseract);
        assert_ne!(OcrBackendType::Tesseract, OcrBackendType::PaddleOCR);
        assert_ne!(OcrBackendType::PaddleOCR, OcrBackendType::Custom);
    }

    #[test]
    fn test_ocr_backend_type_debug() {
        let backend_type = OcrBackendType::Tesseract;
        let debug_str = format!("{:?}", backend_type);
        assert!(debug_str.contains("Tesseract"));
    }

    #[test]
    fn test_ocr_backend_type_clone() {
        let backend_type = OcrBackendType::PaddleOCR;
        let cloned = backend_type;
        assert_eq!(backend_type, cloned);
    }

    #[test]
    fn test_ocr_backend_default_table_detection() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };
        assert!(!backend.supports_table_detection());
    }

    /// Regression test for the sceptre confidence-gating failure: a backend that reports a
    /// page-level confidence number without declaring `confidence_semantics` must default to
    /// `Uncalibrated`, never to `Legibility`. Defaulting to `Legibility` would let the next
    /// backend added to this codebase silently inherit Tesseract's gate threshold and repeat
    /// the sceptre failure, which rejected all 16 pages of a document and emptied it.
    #[test]
    fn should_default_to_uncalibrated_for_a_backend_that_does_not_declare_semantics() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        assert_eq!(backend.confidence_semantics(), ConfidenceSemantics::Uncalibrated);
    }

    /// Gating code reaches a backend as `&dyn OcrBackend` out of the registry, never as a
    /// concrete type, so the declared semantics must survive dynamic dispatch — including the
    /// `scale_max` payload, which is what a caller divides by instead of a hardcoded 100.
    #[test]
    fn should_report_declared_semantics_through_a_trait_object() {
        struct CalibratedBackend;

        impl Plugin for CalibratedBackend {
            fn name(&self) -> &str {
                "calibrated"
            }

            fn version(&self) -> String {
                "1.0.0".to_string()
            }

            fn initialize(&self) -> Result<()> {
                Ok(())
            }

            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl OcrBackend for CalibratedBackend {
            async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> Result<ExtractedDocument> {
                unreachable!("this backend exists only to declare confidence semantics")
            }

            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }

            fn supports_language(&self, lang: &str) -> bool {
                lang == "eng"
            }

            fn supported_languages(&self) -> Vec<String> {
                vec!["eng".to_string()]
            }

            fn confidence_semantics(&self) -> ConfidenceSemantics {
                ConfidenceSemantics::Legibility { scale_max: 255.0 }
            }
        }

        let backend: &dyn OcrBackend = &CalibratedBackend;

        match backend.confidence_semantics() {
            ConfidenceSemantics::Legibility { scale_max } => assert_eq!(scale_max, 255.0),
            other => panic!("expected the declared Legibility semantics, got {other:?}"),
        }
    }

    /// Regression guard for the rotation-handling capability: a backend that does not declare
    /// `page_orientation_handling` must default to `RequiresUpright`, never to `SelfCorrecting`.
    /// Defaulting to `SelfCorrecting` would let a new backend that cannot self-correct silently
    /// inherit Tesseract's guarantee and emit garbage the first time it is handed a rotated
    /// raster, mirroring the sceptre confidence-gating failure above.
    #[test]
    fn should_default_to_requires_upright_for_a_backend_that_does_not_declare_orientation_handling() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        let dynamic: &dyn OcrBackend = &backend;
        assert_eq!(
            dynamic.page_orientation_handling(),
            PageOrientationHandling::RequiresUpright
        );
    }

    /// `process_image_file`'s default impl returns `Other("File-based OCR processing
    /// requires the tokio-runtime feature")` without that feature, so this test can only
    /// assert the real behaviour in a build that has it.
    #[cfg(feature = "tokio-runtime")]
    #[tokio::test]
    async fn test_ocr_backend_process_image_file_default_impl() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"fake image data").unwrap();
        let path = temp_file.path();

        let config = OcrConfig {
            backend: "mock".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        };

        let result = backend.process_image_file(path, &config).await.unwrap();
        assert_eq!(result.content, "Mocked OCR text");
    }

    #[test]
    fn test_ocr_backend_plugin_interface() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        assert_eq!(backend.name(), "mock-ocr");
        assert_eq!(backend.version(), "1.0.0");
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    #[test]
    fn test_ocr_backend_empty_languages() {
        let backend = MockOcrBackend { languages: vec![] };

        let supported = backend.supported_languages();
        assert_eq!(supported.len(), 0);
        assert!(!backend.supports_language("eng"));
    }

    #[tokio::test]
    async fn test_ocr_backend_with_empty_image() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        let config = OcrConfig {
            backend: "mock".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        };

        let result = backend.process_image(b"", &config).await;
        assert!(result.is_ok());
    }

    struct OptionAwareBackend;

    impl Plugin for OptionAwareBackend {
        fn name(&self) -> &str {
            "option-aware"
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl OcrBackend for OptionAwareBackend {
        async fn process_image(&self, _image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
            let mode = config
                .backend_options
                .as_ref()
                .and_then(|v| v.get("mode"))
                .and_then(|v| v.as_str())
                .unwrap_or("standard");

            Ok(ExtractedDocument {
                content: format!("mode={mode}"),
                mime_type: Cow::Borrowed("text/plain"),
                ..Default::default()
            })
        }

        fn supports_language(&self, _: &str) -> bool {
            true
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
    }

    #[tokio::test]
    async fn test_backend_reads_backend_options() {
        let backend = OptionAwareBackend;

        let config_with_options = OcrConfig {
            backend_options: Some(serde_json::json!({"mode": "fast", "threshold": 0.8})),
            ..Default::default()
        };
        let result = backend.process_image(b"img", &config_with_options).await.unwrap();
        assert_eq!(result.content, "mode=fast");

        let config_without_options = OcrConfig::default();
        let result = backend.process_image(b"img", &config_without_options).await.unwrap();
        assert_eq!(result.content, "mode=standard");
    }

    #[tokio::test]
    async fn test_backend_options_unknown_keys_silently_ignored() {
        let backend = OptionAwareBackend;

        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "unknown_key": "value",
                "another_unknown": 42
            })),
            ..Default::default()
        };
        let result = backend.process_image(b"img", &config).await;
        assert!(result.is_ok(), "unknown backend_options keys must not cause errors");
    }
}
