//! Read a standalone image again with the built-in PaddleOCR backend when the default
//! Tesseract read of it is unreliable (`OcrConfig::image_paddle_fallback`).
//!
//! Camera photos (receipts, scene text) defeat Tesseract at every PSM and threshold the image
//! route tries, while PaddleOCR's detector reads them. The two backends' confidences are on
//! different scales, so the selection never compares them: Tesseract's confidence only decides
//! which of its own words count as read, and the PaddleOCR read is judged by its text.

use super::{has_robust_word_confidence_distribution, is_implicit_horizontal_tesseract};
use crate::cancellation::{CancellationToken, check_cancelled};
use crate::core::config::{ExtractionConfig, OcrConfig, OcrQualityThresholds, VlmFallbackPolicy};
use crate::plugins::OcrBackend;
use crate::plugins::registry::OcrBackendRegistry;
use crate::types::internal::InternalDocument;
use crate::types::{ExtractedDocument, OcrElementLevel};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

const PADDLE_BACKEND: &str = "paddle-ocr";

/// Tesseract word confidence (0-1) at or above which a word of the primary read counts as read.
/// The same bar `image_color_threshold` uses for a confident word.
const TRUSTED_TESSERACT_WORD: f64 = 0.70;

/// Model sets the PaddleOCR probe has verified in this process. A verified, cached model stays
/// cached, so only a positive answer is kept; it spares hashing the model files on every photo.
static VERIFIED_MODEL_SETS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

/// The built-in PaddleOCR backend when `primary` is the built-in Tesseract backend and the
/// request is [`eligible`]; a backend registered in place of either keeps the request as is.
pub(super) fn backend(
    registry: &OcrBackendRegistry,
    primary: &Arc<dyn OcrBackend>,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    mime_type: &str,
) -> Option<Arc<dyn OcrBackend>> {
    (registry.is_builtin_tesseract(primary) && registry.has_builtin_paddle() && eligible(config, ocr, mime_type))
        .then(|| registry.get("paddleocr").ok())
        .flatten()
}

/// Whether the request leaves the backend choice to Xberg: the default Tesseract read with no
/// caller tuning, no multi-backend policy, and one language PaddleOCR reads with one model.
fn eligible(config: &ExtractionConfig, ocr: &OcrConfig, mime_type: &str) -> bool {
    #[cfg(feature = "layout-types")]
    let layout = config.layout.is_some();
    #[cfg(not(feature = "layout-types"))]
    let layout = {
        let _ = config;
        false
    };
    ocr.image_paddle_fallback
        && !layout
        && is_implicit_horizontal_tesseract(ocr)
        && ocr
            .tesseract_config
            .as_ref()
            .is_none_or(|tesseract| tesseract.preprocessing.is_none())
        && !ocr.has_tuning_backend_options()
        && ocr.pipeline.is_none()
        && ocr.vlm_fallback == VlmFallbackPolicy::Disabled
        && !mime_type.to_ascii_lowercase().contains("tiff")
        && reads_one_paddle_language(ocr)
}

fn reads_one_paddle_language(ocr: &OcrConfig) -> bool {
    let languages = ocr.effective_languages();
    matches!(languages.as_slice(), [language] if crate::paddle_ocr::map_language_code(language).is_some())
        && ocr.effective_tesseract_language() == languages
}

/// Whether `doc` was built from a PaddleOCR read this fallback selected, so a later Tesseract
/// re-read of the first pass does not replace it.
pub(super) fn read_by_paddle(doc: &InternalDocument) -> bool {
    doc.prebuilt_pages
        .iter()
        .flatten()
        .filter_map(|page| page.ocr_confidence.as_ref())
        .any(|confidence| confidence.backend == PADDLE_BACKEND)
}

fn paddle_read_config(effective: &OcrConfig) -> OcrConfig {
    let mut config = effective.clone();
    config.backend = PADDLE_BACKEND.to_string();
    config.tesseract_config = None;
    config
}

/// Whether every model the PaddleOCR read needs is already in the local cache, so the read
/// cannot start a download in the middle of an image extraction.
fn models_cached(paddle: &dyn OcrBackend, config: &OcrConfig) -> bool {
    #[allow(deprecated)]
    let key = format!(
        "{}|{:?}|{:?}|{:?}|{:?}",
        paddle.name(),
        config.paddle_ocr_settings,
        config.paddle_ocr_config,
        config.language,
        config.auto_rotate
    );
    let verified = || VERIFIED_MODEL_SETS.lock().unwrap_or_else(PoisonError::into_inner);
    if verified().contains(&key) {
        return true;
    }
    let cached = paddle.probe(config).status == crate::doctor::ProbeStatus::Pass;
    if cached {
        verified().insert(key);
    }
    cached
}

fn alphanumeric_token_count(text: &str) -> usize {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .count()
}

/// Tokens of the Tesseract read that Tesseract itself is confident about. Without word
/// elements every token of its text counts, which can only make the PaddleOCR read harder to
/// adopt.
fn trusted_primary_tokens(primary: &ExtractedDocument) -> usize {
    let mut words = primary
        .ocr_elements
        .iter()
        .flatten()
        .filter(|element| element.level == OcrElementLevel::Word && !element.text.trim().is_empty())
        .peekable();
    if words.peek().is_none() {
        return alphanumeric_token_count(&primary.content);
    }
    words
        .filter(|element| element.confidence.recognition >= TRUSTED_TESSERACT_WORD)
        .map(|element| alphanumeric_token_count(&element.text))
        .sum()
}

/// The PaddleOCR read replaces the Tesseract read when it reads more letter-or-digit tokens
/// than Tesseract read with confidence, and its text clears the same quality gate a PDF OCR
/// pipeline stage must clear to be accepted (`pipeline_min_quality`).
fn paddle_read_wins(
    primary: &ExtractedDocument,
    candidate: &ExtractedDocument,
    thresholds: &OcrQualityThresholds,
) -> bool {
    alphanumeric_token_count(&candidate.content) > trusted_primary_tokens(primary)
        && crate::extractors::pdf::ocr::compute_quality_score(&candidate.content, thresholds)
            >= thresholds.pipeline_min_quality
}

/// Returns the selected read and, when PaddleOCR produced it, the PaddleOCR backend.
///
/// `paddle` is the built-in PaddleOCR backend, or `None` when it is not registered or the
/// request is not [`eligible`]; then the primary read is returned unchanged.
pub(super) async fn retry(
    paddle: Option<Arc<dyn OcrBackend>>,
    bytes: Arc<Vec<u8>>,
    (original, effective): (&OcrConfig, &OcrConfig),
    cancel: Option<&CancellationToken>,
    primary: ExtractedDocument,
) -> crate::Result<(ExtractedDocument, Option<Arc<dyn OcrBackend>>)> {
    let Some(paddle) = paddle else {
        return Ok((primary, None));
    };
    if has_robust_word_confidence_distribution(&primary) {
        return Ok((primary, None));
    }
    let config = paddle_read_config(effective);
    if !models_cached(paddle.as_ref(), &config) {
        tracing::debug!("PaddleOCR image fallback skipped: models are not cached locally");
        return Ok((primary, None));
    }
    let thresholds = original.effective_thresholds();
    let (selected, adopted) = read_if_better(
        cancel,
        async || paddle.process_image_owned(bytes, &config).await,
        primary,
        &thresholds,
    )
    .await?;
    Ok((selected, adopted.then_some(paddle)))
}

async fn read_if_better(
    cancel: Option<&CancellationToken>,
    read: impl AsyncFnOnce() -> crate::Result<ExtractedDocument>,
    primary: ExtractedDocument,
    thresholds: &OcrQualityThresholds,
) -> crate::Result<(ExtractedDocument, bool)> {
    check_cancelled(cancel)?;
    let result = read().await;
    check_cancelled(cancel)?;
    match result {
        Ok(candidate) if paddle_read_wins(&primary, &candidate, thresholds) => {
            tracing::debug!("PaddleOCR image fallback adopted");
            Ok((candidate, true))
        }
        Ok(_) => {
            tracing::debug!("PaddleOCR image fallback rejected; keeping the Tesseract read");
            Ok((primary, false))
        }
        Err(error @ crate::XbergError::Cancelled) => Err(error),
        Err(error) => {
            crate::core::extractor::cacheability::invalidate_current();
            tracing::warn!(%error, "PaddleOCR image fallback failed; keeping the Tesseract read");
            Ok((primary, false))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OcrBoundingGeometry, OcrConfidence, OcrElement};

    fn word(text: &str, recognition: f64) -> OcrElement {
        OcrElement {
            text: text.into(),
            geometry: OcrBoundingGeometry::Rectangle {
                left: 0,
                top: 0,
                width: 10,
                height: 10,
            },
            confidence: OcrConfidence {
                detection: None,
                recognition,
            },
            level: OcrElementLevel::Word,
            rotation: None,
            page_number: 1,
            parent_id: None,
            backend_metadata: Default::default(),
        }
    }

    fn tesseract_read(words: &[(&str, f64)]) -> ExtractedDocument {
        ExtractedDocument {
            content: words.iter().map(|(text, _)| *text).collect::<Vec<_>>().join(" "),
            ocr_elements: Some(words.iter().map(|(text, conf)| word(text, *conf)).collect()),
            ..Default::default()
        }
    }

    fn paddle_read(text: &str) -> ExtractedDocument {
        ExtractedDocument {
            content: text.into(),
            ..Default::default()
        }
    }

    /// The cord_receipt_02 shape: Tesseract trusts two tokens ("CA", "391000"), one of them a
    /// misread, while PaddleOCR reads the receipt.
    const RECEIPT_TESSERACT: &[(&str, f64)] = &[
        ("vs", 0.37),
        ("Ae.", 0.31),
        ("sa)", 0.0),
        ("rOTAL", 0.52),
        ("391000", 0.71),
        ("CA", 0.96),
    ];

    #[test]
    fn paddle_read_wins_when_it_reads_more_tokens_than_tesseract_trusts() {
        let primary = tesseract_read(RECEIPT_TESSERACT);
        assert_eq!(trusted_primary_tokens(&primary), 2);
        let candidate = paddle_read("TOTAL 91000\nCASH 91000");
        assert!(paddle_read_wins(&primary, &candidate, &OcrQualityThresholds::default()));
    }

    #[test]
    fn paddle_read_loses_when_it_reads_no_more_than_tesseract_trusts() {
        let primary = tesseract_read(RECEIPT_TESSERACT);
        assert!(!paddle_read_wins(
            &primary,
            &paddle_read("TOTAL 91000"),
            &OcrQualityThresholds::default()
        ));
    }

    #[test]
    fn paddle_read_loses_below_the_pipeline_quality_gate() {
        let primary = tesseract_read(RECEIPT_TESSERACT);
        let candidate = paddle_read("TOTAL 91000\nCASH 91000");
        let strict = OcrQualityThresholds {
            pipeline_min_quality: 1.0,
            ..Default::default()
        };
        assert!(crate::extractors::pdf::ocr::compute_quality_score(&candidate.content, &strict) < 1.0);
        assert!(!paddle_read_wins(&primary, &candidate, &strict));
    }

    #[test]
    fn an_empty_paddle_read_never_wins() {
        let primary = tesseract_read(&[]);
        assert!(!paddle_read_wins(
            &primary,
            &paddle_read("  "),
            &OcrQualityThresholds::default()
        ));
    }

    #[test]
    fn a_read_without_word_elements_trusts_every_token() {
        let primary = paddle_read("TOTAL 91000 CASH");
        assert_eq!(trusted_primary_tokens(&primary), 3);
        assert!(!paddle_read_wins(
            &primary,
            &paddle_read("TOTAL 91000 CASH"),
            &OcrQualityThresholds::default()
        ));
    }

    fn default_extraction() -> ExtractionConfig {
        ExtractionConfig::default()
    }

    #[test]
    fn the_default_tesseract_request_is_eligible() {
        assert!(eligible(&default_extraction(), &OcrConfig::default(), "image/jpeg"));
    }

    #[test]
    fn an_explicit_other_backend_is_never_replaced() {
        for backend in ["paddle-ocr", "paddleocr", "sceptre", "vlm", "custom-ocr"] {
            let ocr = OcrConfig {
                backend: backend.to_string(),
                ..Default::default()
            };
            assert!(
                !eligible(&default_extraction(), &ocr, "image/jpeg"),
                "backend: {backend}"
            );
        }
    }

    #[test]
    fn the_option_turns_the_fallback_off() {
        let ocr = OcrConfig {
            image_paddle_fallback: false,
            ..Default::default()
        };
        assert!(!eligible(&default_extraction(), &ocr, "image/jpeg"));
    }

    #[test]
    fn tesseract_tuning_and_multi_backend_policies_keep_the_tesseract_read() {
        let psm = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: Some(6),
                ..Default::default()
            }),
            ..Default::default()
        };
        let preprocessing = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                preprocessing: Some(Default::default()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let options = OcrConfig {
            backend_options: Some(serde_json::json!({"mode": "fast"})),
            ..Default::default()
        };
        let pipeline = OcrConfig {
            pipeline: Some(crate::core::config::OcrPipelineConfig {
                stages: vec![],
                quality_thresholds: Default::default(),
            }),
            ..Default::default()
        };
        let two_languages = OcrConfig {
            language: vec!["eng".to_string(), "deu".to_string()],
            ..Default::default()
        };
        for (name, ocr) in [
            ("psm", psm),
            ("preprocessing", preprocessing),
            ("backend_options", options),
            ("pipeline", pipeline),
            ("two languages", two_languages),
        ] {
            assert!(!eligible(&default_extraction(), &ocr, "image/jpeg"), "{name}");
        }
    }

    #[test]
    fn the_cache_switch_alone_keeps_the_request_eligible() {
        let ocr = OcrConfig {
            backend_options: Some(serde_json::json!({"use_cache": false})),
            ..Default::default()
        };
        assert!(eligible(&default_extraction(), &ocr, "image/png"));
    }

    #[test]
    fn tiff_frames_keep_the_tesseract_read() {
        assert!(!eligible(&default_extraction(), &OcrConfig::default(), "image/tiff"));
    }

    #[tokio::test]
    async fn without_a_paddle_backend_the_primary_read_is_returned_unchanged() {
        let primary = tesseract_read(RECEIPT_TESSERACT);
        let config = OcrConfig::default();
        let (selected, paddle) = retry(None, Arc::new(Vec::new()), (&config, &config), None, primary.clone())
            .await
            .unwrap();
        assert_eq!(selected.content, primary.content);
        assert!(paddle.is_none());
    }

    #[tokio::test]
    async fn a_robust_tesseract_read_is_never_read_again() {
        let primary = tesseract_read(&[("TOTAL", 0.95), ("91000", 0.93), ("CASH", 0.91)]);
        assert!(has_robust_word_confidence_distribution(&primary));
        let backend: Arc<dyn OcrBackend> = Arc::new(tests_backend::PanickingBackend);
        let config = OcrConfig::default();
        let (selected, paddle) = retry(
            Some(backend),
            Arc::new(Vec::new()),
            (&config, &config),
            None,
            primary.clone(),
        )
        .await
        .unwrap();
        assert_eq!(selected.content, primary.content);
        assert!(paddle.is_none());
    }

    #[tokio::test]
    async fn the_better_paddle_read_is_selected() {
        let (selected, adopted) = read_if_better(
            None,
            async || Ok(paddle_read("TOTAL 91000\nCASH 91000")),
            tesseract_read(RECEIPT_TESSERACT),
            &OcrQualityThresholds::default(),
        )
        .await
        .unwrap();
        assert!(adopted);
        assert_eq!(selected.content, "TOTAL 91000\nCASH 91000");
    }

    #[tokio::test]
    async fn a_failed_paddle_read_keeps_the_tesseract_read() {
        let primary = tesseract_read(RECEIPT_TESSERACT);
        let (selected, adopted) = read_if_better(
            None,
            async || {
                Err(crate::XbergError::Ocr {
                    message: "model missing".to_string(),
                    source: None,
                })
            },
            primary.clone(),
            &OcrQualityThresholds::default(),
        )
        .await
        .unwrap();
        assert!(!adopted);
        assert_eq!(selected.content, primary.content);
    }

    #[tokio::test]
    async fn cancellation_before_the_read_prevents_it() {
        let token = CancellationToken::new();
        token.cancel();
        let result = read_if_better(
            Some(&token),
            async || panic!("cancelled before the PaddleOCR read"),
            tesseract_read(RECEIPT_TESSERACT),
            &OcrQualityThresholds::default(),
        )
        .await;
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    }

    #[tokio::test]
    async fn cancellation_during_the_read_discards_the_candidate() {
        let token = CancellationToken::new();
        let result = read_if_better(
            Some(&token),
            async || {
                token.cancel();
                Ok(paddle_read("TOTAL 91000\nCASH 91000"))
            },
            tesseract_read(RECEIPT_TESSERACT),
            &OcrQualityThresholds::default(),
        )
        .await;
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    }

    #[test]
    fn a_document_from_the_paddle_read_is_recognised() {
        let page = |backend: &str| crate::types::PageContent {
            page_number: 1,
            content: "TOTAL".to_string(),
            tables: vec![],
            image_indices: vec![],
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: Some(crate::types::page::PageOcrConfidence {
                score: None,
                word_count: 0,
                backend: backend.to_string(),
            }),
            native_content: None,
            geometry: None,
            viewports: None,
        };
        let doc = |backend: &str| InternalDocument {
            prebuilt_pages: Some(vec![page(backend)]),
            ..Default::default()
        };
        assert!(read_by_paddle(&doc(PADDLE_BACKEND)));
        assert!(!read_by_paddle(&doc("tesseract")));
        assert!(!read_by_paddle(&InternalDocument::default()));
    }

    mod tests_backend {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
        use crate::types::ExtractedDocument;

        pub(super) struct PanickingBackend;

        impl Plugin for PanickingBackend {
            fn name(&self) -> &str {
                "paddle-fallback-test"
            }
            fn version(&self) -> String {
                "0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        #[async_trait::async_trait]
        impl OcrBackend for PanickingBackend {
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
                panic!("a robust Tesseract read must not be read again")
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::PaddleOCR
            }
            fn probe(&self, _: &OcrConfig) -> crate::doctor::DoctorCheck {
                panic!("a robust Tesseract read must not probe the PaddleOCR models")
            }
        }
    }
}
